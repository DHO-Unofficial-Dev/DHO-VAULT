// SPDX-License-Identifier: MPL-2.0

//! Read-only discovery and inspection of a DHO game client installation.

mod snapshot;

pub use snapshot::{
    ASSET_SNAPSHOT_FORMAT_VERSION, AssetSnapshot, AssetSnapshotChange, AssetSnapshotCompareError,
    AssetSnapshotDiff, AssetSnapshotEntry, AssetSnapshotError, AssetSourceKind,
    inspect_asset_snapshot,
};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use dho_catalog::{CatalogRecordKey, VerificationStatus, assembly_plan, classify_record};
use dho_core::{IndexParseError, IndexedArchive};
use dho_extract::{
    ExtractError, LoadedArchive, LoadedRawImageArchive, RawImageSpec, RawPixelFormat,
    RawResourceKey, ResourceKey,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const INDEXED_ARCHIVE_PREFIXES: [&str; 13] = [
    "im", "sa", "sb", "sc", "sd", "se", "sf", "sg", "sw", "sx", "sy", "sz", "is",
];
pub const SUPPORTED_ARCHIVE_PREFIXES: [&str; 14] = [
    "im", "sa", "sb", "sc", "sd", "se", "sf", "sg", "sh", "sw", "sx", "sy", "sz", "is",
];
pub const VIEWER_CATEGORY_PAGE_SIZE: usize = 32;

#[derive(Debug, Clone, Copy)]
pub(crate) struct RawArchiveDefinition {
    pub prefix: &'static str,
    pub archive_count: u32,
    pub spec: RawImageSpec,
}

pub(crate) const RAW_IMAGE_ARCHIVES: [RawArchiveDefinition; 1] = [RawArchiveDefinition {
    prefix: "sh",
    archive_count: 1,
    spec: RawImageSpec {
        width: 256,
        height: 256,
        pixel_format: RawPixelFormat::Gray8,
    },
}];

/// Resolves the physical subdirectory for an archive while preserving callers that already pass
/// a concrete archive directory.
pub fn resolve_archive_directory(resource_root: impl AsRef<Path>, prefix: &str) -> PathBuf {
    let resource_root = resource_root.as_ref();
    if resource_root.join(format!("{prefix}000000.bin")).is_file()
        || resource_root.join(format!("{prefix}000001.bin")).is_file()
    {
        return resource_root.to_owned();
    }

    let subdirectory = if matches!(
        prefix.to_ascii_lowercase().as_str(),
        "sw" | "sx" | "sy" | "sz"
    ) {
        "0002"
    } else {
        "0001"
    };
    resource_root.join(subdirectory)
}

const THUMBNAIL_MAX_WIDTH: u32 = 160;
const THUMBNAIL_MAX_HEIGHT: u32 = 160;
const DETAIL_MAX_WIDTH: u32 = 1024;
const DETAIL_MAX_HEIGHT: u32 = 1024;
const MAX_IMAGE_DECODE_SIZE: usize = 64 * 1024 * 1024;
const MAX_ASSEMBLED_DECODE_SIZE: usize = 128 * 1024 * 1024;
const MAX_THUMBNAIL_DECODE_SIZE: usize =
    THUMBNAIL_MAX_WIDTH as usize * THUMBNAIL_MAX_HEIGHT as usize * 4;
const MAX_DETAIL_DECODE_SIZE: usize = DETAIL_MAX_WIDTH as usize * DETAIL_MAX_HEIGHT as usize * 4;
const THUMBNAIL_CACHE_MAX_ITEMS: usize = 256;
const THUMBNAIL_CACHE_MAX_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveSummary {
    pub prefix: String,
    pub has_index: bool,
    pub record_count: u32,
    pub group_count: u32,
    pub image_block_count: u32,
    pub archive_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameDirectorySummary {
    pub game_directory: String,
    pub resource_directory: String,
    pub archives: Vec<ArchiveSummary>,
    pub verified_categories: Vec<VerifiedCategorySummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedCategorySummary {
    pub path: Vec<String>,
    pub asset_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedCategoryPage {
    pub path: Vec<String>,
    pub offset: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub items: Vec<VerifiedAssetThumbnail>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedAssetSearchPage {
    pub query: String,
    pub offset: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub items: Vec<VerifiedAssetSearchItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedUpdatePage {
    pub offset: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub detected_record_count: usize,
    pub review_required_count: usize,
    pub items: Vec<VerifiedAssetSearchItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedAssetSearchItem {
    pub path: Vec<String>,
    pub thumbnail: VerifiedAssetThumbnail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedAssetThumbnail {
    pub archive: String,
    pub icon_id: Option<u32>,
    pub block_index: u32,
    pub source_width: u32,
    pub source_height: u32,
    pub thumbnail_width: u32,
    pub thumbnail_height: u32,
    pub assembled: bool,
    pub thumbnail_data_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedAssetDetail {
    pub path: Vec<String>,
    pub archive: String,
    pub icon_id: Option<u32>,
    pub block_index: u32,
    pub source_width: u32,
    pub source_height: u32,
    pub preview_width: u32,
    pub preview_height: u32,
    pub assembled: bool,
    pub preview_data_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedAssetPng {
    pub archive: String,
    pub icon_id: Option<u32>,
    pub block_index: u32,
    pub width: u32,
    pub height: u32,
    pub assembled: bool,
    pub png: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct VerifiedCategoryAsset(VerifiedAssetRef);

impl VerifiedCategoryAsset {
    pub fn archive(&self) -> &str {
        &self.0.prefix
    }

    pub fn icon_id(&self) -> Option<u32> {
        self.0.icon_id()
    }

    pub fn block_index(&self) -> u32 {
        self.0.canonical_block
    }

    pub fn assembled(&self) -> bool {
        self.0.assembled
    }
}

#[derive(Debug, Clone)]
pub struct VerifiedSearchAsset(VerifiedSearchAssetRef);

impl VerifiedSearchAsset {
    pub fn path(&self) -> &[String] {
        &self.0.path
    }

    pub fn archive(&self) -> &str {
        &self.0.asset.prefix
    }

    pub fn icon_id(&self) -> Option<u32> {
        self.0.asset.icon_id()
    }

    pub fn block_index(&self) -> u32 {
        self.0.asset.canonical_block
    }

    pub fn assembled(&self) -> bool {
        self.0.asset.assembled
    }
}

#[derive(Debug, Default)]
pub struct ViewerSession {
    resource_directory: Option<PathBuf>,
    archives: HashMap<String, LoadedArchive>,
    raw_archives: HashMap<String, LoadedRawImageArchive>,
    search_assets: Option<Vec<VerifiedSearchAssetRef>>,
    thumbnail_cache: ThumbnailCache,
}

#[derive(Debug, Clone)]
struct VerifiedAssetRef {
    prefix: String,
    key: ResourceKey,
    raw_key: Option<RawResourceKey>,
    canonical_block: u32,
    assembled: bool,
}

impl VerifiedAssetRef {
    fn icon_id(&self) -> Option<u32> {
        self.raw_key.is_none().then_some(self.key.icon_id)
    }
}

#[derive(Debug, Clone)]
struct VerifiedSearchAssetRef {
    path: Vec<String>,
    asset: VerifiedAssetRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ThumbnailCacheKey {
    prefix: String,
    icon_id: u32,
    canonical_block: u32,
    assembled: bool,
}

impl From<&VerifiedAssetRef> for ThumbnailCacheKey {
    fn from(asset: &VerifiedAssetRef) -> Self {
        Self {
            prefix: asset.prefix.clone(),
            icon_id: asset.key.icon_id,
            canonical_block: asset.canonical_block,
            assembled: asset.assembled,
        }
    }
}

#[derive(Debug)]
struct ThumbnailCacheEntry {
    thumbnail: VerifiedAssetThumbnail,
    size_bytes: usize,
}

#[derive(Debug)]
struct ThumbnailCache {
    entries: HashMap<ThumbnailCacheKey, ThumbnailCacheEntry>,
    recency: VecDeque<ThumbnailCacheKey>,
    total_bytes: usize,
    max_items: usize,
    max_bytes: usize,
}

impl Default for ThumbnailCache {
    fn default() -> Self {
        Self::with_limits(THUMBNAIL_CACHE_MAX_ITEMS, THUMBNAIL_CACHE_MAX_BYTES)
    }
}

impl ThumbnailCache {
    fn with_limits(max_items: usize, max_bytes: usize) -> Self {
        Self {
            entries: HashMap::new(),
            recency: VecDeque::new(),
            total_bytes: 0,
            max_items,
            max_bytes,
        }
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.recency.clear();
        self.total_bytes = 0;
    }

    fn get(&mut self, key: &ThumbnailCacheKey) -> Option<VerifiedAssetThumbnail> {
        let thumbnail = self.entries.get(key)?.thumbnail.clone();
        self.recency.retain(|existing| existing != key);
        self.recency.push_back(key.clone());
        Some(thumbnail)
    }

    fn insert(&mut self, key: ThumbnailCacheKey, thumbnail: VerifiedAssetThumbnail) {
        let size_bytes = thumbnail_cache_size(&thumbnail);
        if self.max_items == 0 || size_bytes > self.max_bytes {
            return;
        }

        if let Some(previous) = self.entries.remove(&key) {
            self.total_bytes = self.total_bytes.saturating_sub(previous.size_bytes);
            self.recency.retain(|existing| existing != &key);
        }

        while !self.entries.is_empty()
            && (self.entries.len() >= self.max_items
                || self.total_bytes.saturating_add(size_bytes) > self.max_bytes)
        {
            let Some(oldest) = self.recency.pop_front() else {
                self.clear();
                break;
            };
            if let Some(removed) = self.entries.remove(&oldest) {
                self.total_bytes = self.total_bytes.saturating_sub(removed.size_bytes);
            }
        }

        self.total_bytes = self.total_bytes.saturating_add(size_bytes);
        self.recency.push_back(key.clone());
        self.entries.insert(
            key,
            ThumbnailCacheEntry {
                thumbnail,
                size_bytes,
            },
        );
    }
}

fn thumbnail_cache_size(thumbnail: &VerifiedAssetThumbnail) -> usize {
    std::mem::size_of::<VerifiedAssetThumbnail>()
        .saturating_add(thumbnail.archive.len())
        .saturating_add(thumbnail.thumbnail_data_url.len())
}

impl ViewerSession {
    pub fn resource_directory(&self) -> Option<&Path> {
        self.resource_directory.as_deref()
    }

    pub fn set_resource_directory(&mut self, path: impl Into<PathBuf>) {
        let path = path.into();
        if self.resource_directory.as_ref() != Some(&path) {
            self.archives.clear();
            self.raw_archives.clear();
            self.search_assets = None;
            self.thumbnail_cache.clear();
            self.resource_directory = Some(path);
        }
    }

    pub fn category_page(
        &mut self,
        path: &[String],
        offset: usize,
        page_size: usize,
    ) -> Result<VerifiedCategoryPage, ViewerSessionError> {
        let (selected, total_count) = self.verified_asset_page(path, offset, page_size)?;
        let mut items = Vec::with_capacity(selected.len());
        for asset in selected {
            items.push(self.asset_thumbnail(asset)?);
        }

        Ok(VerifiedCategoryPage {
            path: path.to_vec(),
            offset,
            page_size,
            total_count,
            items,
        })
    }

    pub fn search_page(
        &mut self,
        query: &str,
        offset: usize,
        page_size: usize,
    ) -> Result<VerifiedAssetSearchPage, ViewerSessionError> {
        if !(1..=VIEWER_CATEGORY_PAGE_SIZE).contains(&page_size) {
            return Err(ViewerSessionError::InvalidPageSize {
                requested: page_size,
                maximum: VIEWER_CATEGORY_PAGE_SIZE,
            });
        }

        let (query, matching) = self.matching_search_assets(query)?;
        let total_count = matching.len();
        if offset > 0 && offset >= total_count {
            return Err(ViewerSessionError::OffsetOutOfRange {
                offset,
                total_count,
            });
        }
        let end = offset.saturating_add(page_size).min(total_count);
        let selected = matching.get(offset..end).unwrap_or_default().to_vec();
        let mut items = Vec::with_capacity(selected.len());
        for selected in selected {
            items.push(VerifiedAssetSearchItem {
                path: selected.path,
                thumbnail: self.asset_thumbnail(selected.asset)?,
            });
        }

        Ok(VerifiedAssetSearchPage {
            query,
            offset,
            page_size,
            total_count,
            items,
        })
    }

    pub fn update_page(
        &mut self,
        added_assets: &[AssetSnapshotEntry],
        offset: usize,
        page_size: usize,
    ) -> Result<VerifiedUpdatePage, ViewerSessionError> {
        if !(1..=VIEWER_CATEGORY_PAGE_SIZE).contains(&page_size) {
            return Err(ViewerSessionError::InvalidPageSize {
                requested: page_size,
                maximum: VIEWER_CATEGORY_PAGE_SIZE,
            });
        }

        let mut review_required_count = 0;
        let mut unique =
            BTreeMap::<(Vec<String>, String, u32), (ResourceKey, Option<RawResourceKey>)>::new();
        for asset in added_assets {
            let raw_key = asset.raw_resource_key();
            if asset.source_kind == AssetSourceKind::RawBlock && raw_key.is_none() {
                review_required_count += 1;
                continue;
            }
            let classification = classify_record(CatalogRecordKey {
                archive: &asset.archive,
                group_code: asset.group_code,
                icon_id: asset.icon_id,
                block_index: asset.block_index,
            });
            if classification.boundary_status != VerificationStatus::HumanVerified
                || classification.meaning_status != VerificationStatus::HumanVerified
            {
                review_required_count += 1;
                continue;
            }
            let Some(category) = classification.category else {
                review_required_count += 1;
                continue;
            };
            let canonical_block = assembly_plan(&asset.archive, asset.block_index)
                .map_or(asset.block_index, |plan| plan.first_block);
            let path = category
                .segments()
                .iter()
                .map(|segment| (*segment).to_owned())
                .collect::<Vec<_>>();
            unique
                .entry((path, asset.archive.to_ascii_lowercase(), canonical_block))
                .or_insert((
                    ResourceKey {
                        group_code: asset.group_code,
                        icon_id: asset.icon_id,
                        block_index: asset.block_index,
                    },
                    raw_key,
                ));
        }

        let assets = unique
            .into_iter()
            .map(|((path, prefix, canonical_block), (key, raw_key))| {
                let assembled = assembly_plan(&prefix, canonical_block).is_some();
                VerifiedSearchAssetRef {
                    path,
                    asset: VerifiedAssetRef {
                        prefix,
                        key,
                        raw_key,
                        canonical_block,
                        assembled,
                    },
                }
            })
            .collect::<Vec<_>>();
        let total_count = assets.len();
        if offset > 0 && offset >= total_count {
            return Err(ViewerSessionError::OffsetOutOfRange {
                offset,
                total_count,
            });
        }
        let end = offset.saturating_add(page_size).min(total_count);
        let selected = assets.get(offset..end).unwrap_or_default().to_vec();
        let mut items = Vec::with_capacity(selected.len());
        for selected in selected {
            items.push(VerifiedAssetSearchItem {
                path: selected.path,
                thumbnail: self.asset_thumbnail(selected.asset)?,
            });
        }

        Ok(VerifiedUpdatePage {
            offset,
            page_size,
            total_count,
            detected_record_count: added_assets.len(),
            review_required_count,
            items,
        })
    }

    pub fn category_assets(
        &mut self,
        path: &[String],
    ) -> Result<Vec<VerifiedCategoryAsset>, ViewerSessionError> {
        if path.is_empty() {
            return Err(ViewerSessionError::EmptyCategoryPath);
        }
        let assets = self.verified_assets(path)?;
        if assets.is_empty() {
            return Err(ViewerSessionError::CategoryNotFound {
                path: path.to_vec(),
            });
        }
        Ok(assets.into_iter().map(VerifiedCategoryAsset).collect())
    }

    pub fn search_assets(
        &mut self,
        query: &str,
    ) -> Result<Vec<VerifiedSearchAsset>, ViewerSessionError> {
        let (_, assets) = self.matching_search_assets(query)?;
        Ok(assets.into_iter().map(VerifiedSearchAsset).collect())
    }

    pub fn category_asset_png(
        &mut self,
        asset: &VerifiedCategoryAsset,
    ) -> Result<VerifiedAssetPng, ViewerSessionError> {
        self.extract_asset_png(asset.0.clone())
    }

    pub fn search_asset_png(
        &mut self,
        asset: &VerifiedSearchAsset,
    ) -> Result<VerifiedAssetPng, ViewerSessionError> {
        self.extract_asset_png(asset.0.asset.clone())
    }

    pub fn asset_detail(
        &mut self,
        path: &[String],
        prefix: &str,
        block_index: u32,
    ) -> Result<VerifiedAssetDetail, ViewerSessionError> {
        let asset = self.verified_asset(path, prefix, block_index)?;
        if let Some(raw_key) = asset.raw_key {
            let extracted = self
                .raw_archive(&asset.prefix)?
                .extract_thumbnail_png(
                    raw_key,
                    MAX_IMAGE_DECODE_SIZE,
                    DETAIL_MAX_WIDTH,
                    DETAIL_MAX_HEIGHT,
                    MAX_DETAIL_DECODE_SIZE,
                )
                .map_err(|source| ViewerSessionError::Extract {
                    prefix: asset.prefix.clone(),
                    block_index: asset.canonical_block,
                    source,
                })?;
            return Ok(VerifiedAssetDetail {
                path: path.to_vec(),
                archive: asset.prefix,
                icon_id: None,
                block_index: asset.canonical_block,
                source_width: extracted.source_width,
                source_height: extracted.source_height,
                preview_width: extracted.width,
                preview_height: extracted.height,
                assembled: false,
                preview_data_url: png_data_url(&extracted.png),
            });
        }
        let archive = self.archive(&asset.prefix)?;

        if asset.assembled {
            let extracted = archive
                .extract_verified_assembly_thumbnail(
                    asset.canonical_block,
                    MAX_IMAGE_DECODE_SIZE,
                    MAX_ASSEMBLED_DECODE_SIZE,
                    DETAIL_MAX_WIDTH,
                    DETAIL_MAX_HEIGHT,
                    MAX_DETAIL_DECODE_SIZE,
                )
                .map_err(|source| ViewerSessionError::Extract {
                    prefix: asset.prefix.clone(),
                    block_index: asset.canonical_block,
                    source,
                })?
                .ok_or_else(|| ViewerSessionError::AssemblyRuleMissing {
                    prefix: asset.prefix.clone(),
                    block_index: asset.canonical_block,
                })?;
            Ok(VerifiedAssetDetail {
                path: path.to_vec(),
                archive: asset.prefix,
                icon_id: asset.raw_key.is_none().then_some(asset.key.icon_id),
                block_index: extracted.first_block,
                source_width: extracted.source_width,
                source_height: extracted.source_height,
                preview_width: extracted.width,
                preview_height: extracted.height,
                assembled: true,
                preview_data_url: png_data_url(&extracted.png),
            })
        } else {
            let extracted = archive
                .extract_thumbnail_png(
                    asset.key,
                    MAX_IMAGE_DECODE_SIZE,
                    DETAIL_MAX_WIDTH,
                    DETAIL_MAX_HEIGHT,
                    MAX_DETAIL_DECODE_SIZE,
                )
                .map_err(|source| ViewerSessionError::Extract {
                    prefix: asset.prefix.clone(),
                    block_index: asset.canonical_block,
                    source,
                })?;
            Ok(VerifiedAssetDetail {
                path: path.to_vec(),
                archive: asset.prefix,
                icon_id: asset.raw_key.is_none().then_some(asset.key.icon_id),
                block_index: asset.canonical_block,
                source_width: extracted.source_width,
                source_height: extracted.source_height,
                preview_width: extracted.width,
                preview_height: extracted.height,
                assembled: false,
                preview_data_url: png_data_url(&extracted.png),
            })
        }
    }

    pub fn asset_png(
        &mut self,
        path: &[String],
        prefix: &str,
        block_index: u32,
    ) -> Result<VerifiedAssetPng, ViewerSessionError> {
        let asset = self.verified_asset(path, prefix, block_index)?;
        self.extract_asset_png(asset)
    }

    fn extract_asset_png(
        &mut self,
        asset: VerifiedAssetRef,
    ) -> Result<VerifiedAssetPng, ViewerSessionError> {
        if let Some(raw_key) = asset.raw_key {
            let extracted = self
                .raw_archive(&asset.prefix)?
                .extract_png(raw_key, MAX_IMAGE_DECODE_SIZE)
                .map_err(|source| ViewerSessionError::Extract {
                    prefix: asset.prefix.clone(),
                    block_index: asset.canonical_block,
                    source,
                })?;
            return Ok(VerifiedAssetPng {
                archive: asset.prefix,
                icon_id: None,
                block_index: asset.canonical_block,
                width: extracted.width,
                height: extracted.height,
                assembled: false,
                png: extracted.png,
            });
        }
        let archive = self.archive(&asset.prefix)?;

        if asset.assembled {
            let extracted = archive
                .extract_verified_assembly(
                    asset.canonical_block,
                    MAX_IMAGE_DECODE_SIZE,
                    MAX_ASSEMBLED_DECODE_SIZE,
                )
                .map_err(|source| ViewerSessionError::Extract {
                    prefix: asset.prefix.clone(),
                    block_index: asset.canonical_block,
                    source,
                })?
                .ok_or_else(|| ViewerSessionError::AssemblyRuleMissing {
                    prefix: asset.prefix.clone(),
                    block_index: asset.canonical_block,
                })?;
            Ok(VerifiedAssetPng {
                archive: asset.prefix,
                icon_id: asset.raw_key.is_none().then_some(asset.key.icon_id),
                block_index: extracted.first_block,
                width: extracted.width,
                height: extracted.height,
                assembled: true,
                png: extracted.png,
            })
        } else {
            let extracted = archive
                .extract_png(asset.key, MAX_IMAGE_DECODE_SIZE)
                .map_err(|source| ViewerSessionError::Extract {
                    prefix: asset.prefix.clone(),
                    block_index: asset.canonical_block,
                    source,
                })?;
            Ok(VerifiedAssetPng {
                archive: asset.prefix,
                icon_id: asset.raw_key.is_none().then_some(asset.key.icon_id),
                block_index: asset.canonical_block,
                width: extracted.width,
                height: extracted.height,
                assembled: false,
                png: extracted.png,
            })
        }
    }

    fn asset_thumbnail(
        &mut self,
        asset: VerifiedAssetRef,
    ) -> Result<VerifiedAssetThumbnail, ViewerSessionError> {
        let cache_key = ThumbnailCacheKey::from(&asset);
        if let Some(thumbnail) = self.thumbnail_cache.get(&cache_key) {
            return Ok(thumbnail);
        }

        if let Some(raw_key) = asset.raw_key {
            let extracted = self
                .raw_archive(&asset.prefix)?
                .extract_thumbnail_png(
                    raw_key,
                    MAX_IMAGE_DECODE_SIZE,
                    THUMBNAIL_MAX_WIDTH,
                    THUMBNAIL_MAX_HEIGHT,
                    MAX_THUMBNAIL_DECODE_SIZE,
                )
                .map_err(|source| ViewerSessionError::Extract {
                    prefix: asset.prefix.clone(),
                    block_index: asset.canonical_block,
                    source,
                })?;
            let thumbnail = VerifiedAssetThumbnail {
                archive: asset.prefix,
                icon_id: None,
                block_index: asset.canonical_block,
                source_width: extracted.source_width,
                source_height: extracted.source_height,
                thumbnail_width: extracted.width,
                thumbnail_height: extracted.height,
                assembled: false,
                thumbnail_data_url: png_data_url(&extracted.png),
            };
            self.thumbnail_cache.insert(cache_key, thumbnail.clone());
            return Ok(thumbnail);
        }

        let archive = self.archive(&asset.prefix)?;
        let thumbnail = if asset.assembled {
            let extracted = archive
                .extract_verified_assembly_thumbnail(
                    asset.canonical_block,
                    MAX_IMAGE_DECODE_SIZE,
                    MAX_ASSEMBLED_DECODE_SIZE,
                    THUMBNAIL_MAX_WIDTH,
                    THUMBNAIL_MAX_HEIGHT,
                    MAX_THUMBNAIL_DECODE_SIZE,
                )
                .map_err(|source| ViewerSessionError::Extract {
                    prefix: asset.prefix.clone(),
                    block_index: asset.canonical_block,
                    source,
                })?
                .ok_or_else(|| ViewerSessionError::AssemblyRuleMissing {
                    prefix: asset.prefix.clone(),
                    block_index: asset.canonical_block,
                })?;
            VerifiedAssetThumbnail {
                archive: asset.prefix,
                icon_id: asset.raw_key.is_none().then_some(asset.key.icon_id),
                block_index: extracted.first_block,
                source_width: extracted.source_width,
                source_height: extracted.source_height,
                thumbnail_width: extracted.width,
                thumbnail_height: extracted.height,
                assembled: true,
                thumbnail_data_url: png_data_url(&extracted.png),
            }
        } else {
            let extracted = archive
                .extract_thumbnail_png(
                    asset.key,
                    MAX_IMAGE_DECODE_SIZE,
                    THUMBNAIL_MAX_WIDTH,
                    THUMBNAIL_MAX_HEIGHT,
                    MAX_THUMBNAIL_DECODE_SIZE,
                )
                .map_err(|source| ViewerSessionError::Extract {
                    prefix: asset.prefix.clone(),
                    block_index: asset.canonical_block,
                    source,
                })?;
            VerifiedAssetThumbnail {
                archive: asset.prefix,
                icon_id: asset.raw_key.is_none().then_some(asset.key.icon_id),
                block_index: asset.canonical_block,
                source_width: extracted.source_width,
                source_height: extracted.source_height,
                thumbnail_width: extracted.width,
                thumbnail_height: extracted.height,
                assembled: false,
                thumbnail_data_url: png_data_url(&extracted.png),
            }
        };
        self.thumbnail_cache.insert(cache_key, thumbnail.clone());
        Ok(thumbnail)
    }

    fn verified_asset_page(
        &mut self,
        path: &[String],
        offset: usize,
        page_size: usize,
    ) -> Result<(Vec<VerifiedAssetRef>, usize), ViewerSessionError> {
        if path.is_empty() {
            return Err(ViewerSessionError::EmptyCategoryPath);
        }
        if !(1..=VIEWER_CATEGORY_PAGE_SIZE).contains(&page_size) {
            return Err(ViewerSessionError::InvalidPageSize {
                requested: page_size,
                maximum: VIEWER_CATEGORY_PAGE_SIZE,
            });
        }

        let assets = self.verified_assets(path)?;
        if assets.is_empty() {
            return Err(ViewerSessionError::CategoryNotFound {
                path: path.to_vec(),
            });
        }
        if offset >= assets.len() {
            return Err(ViewerSessionError::OffsetOutOfRange {
                offset,
                total_count: assets.len(),
            });
        }

        let total_count = assets.len();
        let end = offset.saturating_add(page_size).min(total_count);
        Ok((assets[offset..end].to_vec(), total_count))
    }

    fn verified_asset(
        &mut self,
        path: &[String],
        prefix: &str,
        block_index: u32,
    ) -> Result<VerifiedAssetRef, ViewerSessionError> {
        if path.is_empty() {
            return Err(ViewerSessionError::EmptyCategoryPath);
        }
        let normalized_prefix = prefix.to_ascii_lowercase();
        self.verified_assets(path)?
            .into_iter()
            .find(|asset| asset.prefix == normalized_prefix && asset.canonical_block == block_index)
            .ok_or_else(|| ViewerSessionError::AssetNotFound {
                path: path.to_vec(),
                prefix: normalized_prefix,
                block_index,
            })
    }

    fn verified_assets(
        &mut self,
        path: &[String],
    ) -> Result<Vec<VerifiedAssetRef>, ViewerSessionError> {
        let resource_directory = self
            .resource_directory
            .clone()
            .ok_or(ViewerSessionError::ResourceDirectoryNotSelected)?;
        let mut assets = Vec::new();

        for prefix in INDEXED_ARCHIVE_PREFIXES {
            if !resolve_archive_directory(&resource_directory, prefix)
                .join(format!("{prefix}000000.bin"))
                .is_file()
            {
                continue;
            }
            let archive = self.archive(prefix)?;
            let mut unique = BTreeMap::<u32, ResourceKey>::new();
            for record in archive.records() {
                let classification = classify_record(CatalogRecordKey {
                    archive: prefix,
                    group_code: record.group_code,
                    icon_id: record.icon_id,
                    block_index: record.block_index,
                });
                if classification.boundary_status != VerificationStatus::HumanVerified
                    || classification.meaning_status != VerificationStatus::HumanVerified
                    || !classification
                        .category
                        .is_some_and(|category| category_matches(category.segments(), path))
                {
                    continue;
                }
                let canonical_block = assembly_plan(prefix, record.block_index)
                    .map_or(record.block_index, |plan| plan.first_block);
                unique.entry(canonical_block).or_insert(ResourceKey {
                    group_code: record.group_code,
                    icon_id: record.icon_id,
                    block_index: record.block_index,
                });
            }
            assets.extend(
                unique
                    .into_iter()
                    .map(|(canonical_block, key)| VerifiedAssetRef {
                        prefix: prefix.to_owned(),
                        key,
                        raw_key: None,
                        canonical_block,
                        assembled: assembly_plan(prefix, canonical_block).is_some(),
                    }),
            );
        }

        for definition in RAW_IMAGE_ARCHIVES {
            if !resolve_archive_directory(&resource_directory, definition.prefix)
                .join(format!("{}000001.bin", definition.prefix))
                .is_file()
            {
                continue;
            }
            let archive = self.raw_archive(definition.prefix)?;
            let matching = archive
                .records()
                .filter_map(|record| {
                    let classification = classify_record(CatalogRecordKey {
                        archive: definition.prefix,
                        group_code: 0,
                        icon_id: record.key.block_index,
                        block_index: record.key.block_index,
                    });
                    (classification.boundary_status == VerificationStatus::HumanVerified
                        && classification.meaning_status == VerificationStatus::HumanVerified
                        && classification
                            .category
                            .is_some_and(|category| category_matches(category.segments(), path)))
                    .then_some(VerifiedAssetRef {
                        prefix: definition.prefix.to_owned(),
                        key: ResourceKey {
                            group_code: 0,
                            icon_id: record.key.block_index,
                            block_index: record.key.block_index,
                        },
                        raw_key: Some(record.key),
                        canonical_block: record.key.block_index,
                        assembled: false,
                    })
                })
                .collect::<Vec<_>>();
            assets.extend(matching);
        }

        Ok(assets)
    }

    fn matching_search_assets(
        &mut self,
        query: &str,
    ) -> Result<(String, Vec<VerifiedSearchAssetRef>), ViewerSessionError> {
        let query = query.trim();
        if query.is_empty() {
            return Err(ViewerSessionError::EmptySearchQuery);
        }
        let terms = query
            .split_whitespace()
            .map(str::to_lowercase)
            .collect::<Vec<_>>();
        let assets = self
            .verified_search_assets()?
            .iter()
            .filter(|asset| search_asset_matches(asset, &terms))
            .cloned()
            .collect();
        Ok((query.to_owned(), assets))
    }

    fn verified_search_assets(&mut self) -> Result<&[VerifiedSearchAssetRef], ViewerSessionError> {
        if self.search_assets.is_none() {
            let resource_directory = self
                .resource_directory
                .clone()
                .ok_or(ViewerSessionError::ResourceDirectoryNotSelected)?;
            let mut assets = Vec::new();

            for prefix in INDEXED_ARCHIVE_PREFIXES {
                if !resolve_archive_directory(&resource_directory, prefix)
                    .join(format!("{prefix}000000.bin"))
                    .is_file()
                {
                    continue;
                }
                let archive = self.archive(prefix)?;
                let mut unique = BTreeMap::<(Vec<String>, u32), ResourceKey>::new();
                for record in archive.records() {
                    let classification = classify_record(CatalogRecordKey {
                        archive: prefix,
                        group_code: record.group_code,
                        icon_id: record.icon_id,
                        block_index: record.block_index,
                    });
                    if classification.boundary_status != VerificationStatus::HumanVerified
                        || classification.meaning_status != VerificationStatus::HumanVerified
                    {
                        continue;
                    }
                    let Some(category) = classification.category else {
                        continue;
                    };
                    let canonical_block = assembly_plan(prefix, record.block_index)
                        .map_or(record.block_index, |plan| plan.first_block);
                    let path = category
                        .segments()
                        .iter()
                        .map(|segment| (*segment).to_owned())
                        .collect();
                    unique
                        .entry((path, canonical_block))
                        .or_insert(ResourceKey {
                            group_code: record.group_code,
                            icon_id: record.icon_id,
                            block_index: record.block_index,
                        });
                }
                assets.extend(unique.into_iter().map(|((path, canonical_block), key)| {
                    VerifiedSearchAssetRef {
                        path,
                        asset: VerifiedAssetRef {
                            prefix: prefix.to_owned(),
                            key,
                            raw_key: None,
                            canonical_block,
                            assembled: assembly_plan(prefix, canonical_block).is_some(),
                        },
                    }
                }));
            }
            for definition in RAW_IMAGE_ARCHIVES {
                if !resolve_archive_directory(&resource_directory, definition.prefix)
                    .join(format!("{}000001.bin", definition.prefix))
                    .is_file()
                {
                    continue;
                }
                let archive = self.raw_archive(definition.prefix)?;
                let raw_assets = archive
                    .records()
                    .filter_map(|record| {
                        let classification = classify_record(CatalogRecordKey {
                            archive: definition.prefix,
                            group_code: 0,
                            icon_id: record.key.block_index,
                            block_index: record.key.block_index,
                        });
                        if classification.boundary_status != VerificationStatus::HumanVerified
                            || classification.meaning_status != VerificationStatus::HumanVerified
                        {
                            return None;
                        }
                        let path = classification
                            .category?
                            .segments()
                            .iter()
                            .map(|segment| (*segment).to_owned())
                            .collect();
                        Some(VerifiedSearchAssetRef {
                            path,
                            asset: VerifiedAssetRef {
                                prefix: definition.prefix.to_owned(),
                                key: ResourceKey {
                                    group_code: 0,
                                    icon_id: record.key.block_index,
                                    block_index: record.key.block_index,
                                },
                                raw_key: Some(record.key),
                                canonical_block: record.key.block_index,
                                assembled: false,
                            },
                        })
                    })
                    .collect::<Vec<_>>();
                assets.extend(raw_assets);
            }
            assets.sort_by(|left, right| {
                left.path
                    .cmp(&right.path)
                    .then_with(|| left.asset.prefix.cmp(&right.asset.prefix))
                    .then_with(|| left.asset.canonical_block.cmp(&right.asset.canonical_block))
            });

            self.search_assets = Some(assets);
        }
        Ok(self.search_assets.as_deref().unwrap_or_default())
    }

    fn archive(&mut self, prefix: &str) -> Result<&LoadedArchive, ViewerSessionError> {
        let resource_directory = self
            .resource_directory
            .clone()
            .ok_or(ViewerSessionError::ResourceDirectoryNotSelected)?;
        if !self.archives.contains_key(prefix) {
            let archive_directory = resolve_archive_directory(&resource_directory, prefix);
            let archive = LoadedArchive::open(archive_directory, prefix).map_err(|source| {
                ViewerSessionError::OpenArchive {
                    prefix: prefix.to_owned(),
                    source,
                }
            })?;
            self.archives.insert(prefix.to_owned(), archive);
        }
        Ok(self
            .archives
            .get(prefix)
            .expect("archive was inserted before lookup"))
    }

    fn raw_archive(&mut self, prefix: &str) -> Result<&LoadedRawImageArchive, ViewerSessionError> {
        let resource_directory = self
            .resource_directory
            .clone()
            .ok_or(ViewerSessionError::ResourceDirectoryNotSelected)?;
        let normalized = prefix.to_ascii_lowercase();
        let definition = RAW_IMAGE_ARCHIVES
            .iter()
            .find(|definition| definition.prefix == normalized)
            .copied()
            .ok_or_else(|| ViewerSessionError::RawArchiveDefinitionMissing {
                prefix: normalized.clone(),
            })?;
        if !self.raw_archives.contains_key(&normalized) {
            let archive_directory = resolve_archive_directory(&resource_directory, &normalized);
            let archive = LoadedRawImageArchive::open(
                archive_directory,
                &normalized,
                definition.archive_count,
                definition.spec,
            )
            .map_err(|source| ViewerSessionError::OpenArchive {
                prefix: normalized.clone(),
                source,
            })?;
            self.raw_archives.insert(normalized.clone(), archive);
        }
        Ok(self
            .raw_archives
            .get(&normalized)
            .expect("raw archive was inserted before lookup"))
    }
}

fn category_matches(segments: &[&str], path: &[String]) -> bool {
    segments.len() == path.len()
        && segments
            .iter()
            .zip(path)
            .all(|(segment, expected)| *segment == expected)
}

fn search_asset_matches(asset: &VerifiedSearchAssetRef, terms: &[String]) -> bool {
    let text = format!("{} {}", asset.path.join(" "), asset.asset.prefix).to_lowercase();
    let icon_id = asset.asset.key.icon_id.to_string();
    let block_index = asset.asset.canonical_block.to_string();
    terms.iter().all(|term| {
        if term.chars().all(|character| character.is_ascii_digit()) {
            term == &icon_id || term == &block_index
        } else {
            text.contains(term)
        }
    })
}

fn png_data_url(png: &[u8]) -> String {
    format!("data:image/png;base64,{}", BASE64_STANDARD.encode(png))
}

pub fn inspect_game_directory(
    game_directory: impl AsRef<Path>,
) -> Result<GameDirectorySummary, GameDirectoryError> {
    let game_directory = game_directory.as_ref();
    if !game_directory.is_dir() {
        return Err(GameDirectoryError::NotDirectory {
            path: game_directory.to_owned(),
        });
    }

    let executable = game_directory.join("GVOnline.exe");
    if !executable.is_file() {
        return Err(GameDirectoryError::MissingExecutable { path: executable });
    }

    let resource_directory = game_directory.join("0010");
    if !resource_directory.is_dir() {
        return Err(GameDirectoryError::MissingResourceDirectory {
            path: resource_directory,
        });
    }

    let mut archives = Vec::new();
    let mut verified_assets = BTreeMap::<Vec<&'static str>, BTreeSet<(String, u32)>>::new();
    for prefix in INDEXED_ARCHIVE_PREFIXES {
        let path = resolve_archive_directory(&resource_directory, prefix)
            .join(format!("{prefix}000000.bin"));
        if !path.is_file() {
            continue;
        }

        let bytes = fs::read(&path).map_err(|source| GameDirectoryError::ReadIndex {
            path: path.clone(),
            source,
        })?;
        let index =
            IndexedArchive::parse(&bytes).map_err(|source| GameDirectoryError::ParseIndex {
                prefix: prefix.to_owned(),
                path,
                source,
            })?;
        let header = index.header;
        for record in &index.records {
            let classification = classify_record(CatalogRecordKey {
                archive: prefix,
                group_code: record.group_code,
                icon_id: record.icon_id,
                block_index: record.block_index,
            });
            if classification.boundary_status != VerificationStatus::HumanVerified
                || classification.meaning_status != VerificationStatus::HumanVerified
            {
                continue;
            }
            let Some(category) = classification.category else {
                continue;
            };
            let canonical_block = assembly_plan(prefix, record.block_index)
                .map_or(record.block_index, |plan| plan.first_block);
            verified_assets
                .entry(category.segments().to_vec())
                .or_default()
                .insert((prefix.to_owned(), canonical_block));
        }
        archives.push(ArchiveSummary {
            prefix: prefix.to_owned(),
            has_index: true,
            record_count: header.record_count,
            group_count: header.group_count,
            image_block_count: header.image_block_count,
            archive_count: header.archive_count,
        });
    }

    for definition in RAW_IMAGE_ARCHIVES {
        let directory = resolve_archive_directory(&resource_directory, definition.prefix);
        if !directory
            .join(format!("{}000001.bin", definition.prefix))
            .is_file()
        {
            continue;
        }
        let archive = LoadedRawImageArchive::open(
            directory,
            definition.prefix,
            definition.archive_count,
            definition.spec,
        )
        .map_err(|source| GameDirectoryError::OpenArchive {
            prefix: definition.prefix.to_owned(),
            source,
        })?;
        let records = archive.records().collect::<Vec<_>>();
        for record in &records {
            let classification = classify_record(CatalogRecordKey {
                archive: definition.prefix,
                group_code: 0,
                icon_id: record.key.block_index,
                block_index: record.key.block_index,
            });
            if classification.boundary_status != VerificationStatus::HumanVerified
                || classification.meaning_status != VerificationStatus::HumanVerified
            {
                continue;
            }
            if let Some(category) = classification.category {
                verified_assets
                    .entry(category.segments().to_vec())
                    .or_default()
                    .insert((definition.prefix.to_owned(), record.key.block_index));
            }
        }
        archives.push(ArchiveSummary {
            prefix: definition.prefix.to_owned(),
            has_index: false,
            record_count: u32::try_from(records.len()).unwrap_or(u32::MAX),
            group_count: 0,
            image_block_count: u32::try_from(records.len()).unwrap_or(u32::MAX),
            archive_count: archive.archive_count(),
        });
    }
    archives.sort_by_key(|archive| {
        SUPPORTED_ARCHIVE_PREFIXES
            .iter()
            .position(|prefix| *prefix == archive.prefix)
            .unwrap_or(usize::MAX)
    });

    if archives.is_empty() {
        return Err(GameDirectoryError::NoSupportedArchives {
            path: resource_directory,
        });
    }

    Ok(GameDirectorySummary {
        game_directory: game_directory.to_string_lossy().into_owned(),
        resource_directory: resource_directory.to_string_lossy().into_owned(),
        archives,
        verified_categories: verified_assets
            .into_iter()
            .map(|(path, assets)| VerifiedCategorySummary {
                path: path.into_iter().map(str::to_owned).collect(),
                asset_count: assets.len(),
            })
            .collect(),
    })
}

#[derive(Debug)]
pub enum ViewerSessionError {
    ResourceDirectoryNotSelected,
    EmptyCategoryPath,
    EmptySearchQuery,
    InvalidPageSize {
        requested: usize,
        maximum: usize,
    },
    CategoryNotFound {
        path: Vec<String>,
    },
    AssetNotFound {
        path: Vec<String>,
        prefix: String,
        block_index: u32,
    },
    OffsetOutOfRange {
        offset: usize,
        total_count: usize,
    },
    OpenArchive {
        prefix: String,
        source: ExtractError,
    },
    RawArchiveDefinitionMissing {
        prefix: String,
    },
    Extract {
        prefix: String,
        block_index: u32,
        source: ExtractError,
    },
    AssemblyRuleMissing {
        prefix: String,
        block_index: u32,
    },
}

impl fmt::Display for ViewerSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResourceDirectoryNotSelected => {
                write!(formatter, "먼저 게임 폴더를 선택해 주세요.")
            }
            Self::EmptyCategoryPath => write!(formatter, "카테고리 경로가 비어 있습니다."),
            Self::EmptySearchQuery => write!(formatter, "검색어를 입력해 주세요."),
            Self::InvalidPageSize { requested, maximum } => write!(
                formatter,
                "한 번에 불러올 이미지 수는 1개부터 {maximum}개까지입니다: {requested}"
            ),
            Self::CategoryNotFound { path } => {
                write!(
                    formatter,
                    "확인된 카테고리를 찾지 못했습니다: {}",
                    path.join(" > ")
                )
            }
            Self::AssetNotFound {
                path,
                prefix,
                block_index,
            } => write!(
                formatter,
                "카테고리에 속한 확인된 이미지를 찾지 못했습니다: {} / {prefix} {block_index}",
                path.join(" > ")
            ),
            Self::OffsetOutOfRange {
                offset,
                total_count,
            } => write!(
                formatter,
                "이미지 시작 위치가 카테고리 범위를 벗어났습니다: {offset}/{total_count}"
            ),
            Self::OpenArchive { prefix, source } => {
                write!(
                    formatter,
                    "{prefix} 이미지 묶음을 열지 못했습니다: {source}"
                )
            }
            Self::RawArchiveDefinitionMissing { prefix } => write!(
                formatter,
                "{prefix} 원시 이미지 아카이브 명세를 찾지 못했습니다."
            ),
            Self::Extract {
                prefix,
                block_index,
                source,
            } => write!(
                formatter,
                "{prefix} 이미지 {block_index}를 만들지 못했습니다: {source}"
            ),
            Self::AssemblyRuleMissing {
                prefix,
                block_index,
            } => write!(
                formatter,
                "{prefix} 이미지 {block_index}의 검증된 조립 규칙을 찾지 못했습니다."
            ),
        }
    }
}

impl Error for ViewerSessionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::OpenArchive { source, .. } | Self::Extract { source, .. } => Some(source),
            Self::ResourceDirectoryNotSelected
            | Self::EmptyCategoryPath
            | Self::EmptySearchQuery
            | Self::InvalidPageSize { .. }
            | Self::RawArchiveDefinitionMissing { .. }
            | Self::CategoryNotFound { .. }
            | Self::AssetNotFound { .. }
            | Self::OffsetOutOfRange { .. }
            | Self::AssemblyRuleMissing { .. } => None,
        }
    }
}

#[derive(Debug)]
pub enum GameDirectoryError {
    NotDirectory {
        path: PathBuf,
    },
    MissingExecutable {
        path: PathBuf,
    },
    MissingResourceDirectory {
        path: PathBuf,
    },
    ReadIndex {
        path: PathBuf,
        source: io::Error,
    },
    ParseIndex {
        prefix: String,
        path: PathBuf,
        source: IndexParseError,
    },
    OpenArchive {
        prefix: String,
        source: ExtractError,
    },
    NoSupportedArchives {
        path: PathBuf,
    },
}

impl fmt::Display for GameDirectoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotDirectory { path } => {
                write!(
                    formatter,
                    "선택한 경로가 폴더가 아닙니다: {}",
                    path.display()
                )
            }
            Self::MissingExecutable { path } => write!(
                formatter,
                "선택한 폴더에서 GVOnline.exe를 찾지 못했습니다: {}",
                path.display()
            ),
            Self::MissingResourceDirectory { path } => write!(
                formatter,
                "게임 리소스 폴더를 찾지 못했습니다: {}",
                path.display()
            ),
            Self::ReadIndex { path, source } => write!(
                formatter,
                "MWC 인덱스를 읽지 못했습니다 ({}): {source}",
                path.display()
            ),
            Self::ParseIndex {
                prefix,
                path,
                source,
            } => write!(
                formatter,
                "{prefix} MWC 인덱스를 해석하지 못했습니다 ({}): {source}",
                path.display()
            ),
            Self::OpenArchive { prefix, source } => write!(
                formatter,
                "{prefix} 원시 이미지 묶음을 열지 못했습니다: {source}"
            ),
            Self::NoSupportedArchives { path } => write!(
                formatter,
                "지원하는 이미지 리소스(im, sa, sb, sc, sd, se, sf, sg, sh, sw, sx, sy, sz, is)를 찾지 못했습니다: {}",
                path.display()
            ),
        }
    }
}

impl Error for GameDirectoryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadIndex { source, .. } => Some(source),
            Self::ParseIndex { source, .. } => Some(source),
            Self::OpenArchive { source, .. } => Some(source),
            Self::NotDirectory { .. }
            | Self::MissingExecutable { .. }
            | Self::MissingResourceDirectory { .. }
            | Self::NoSupportedArchives { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let number = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "dho-vault-game-directory-test-{}-{number}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("create test directory");
            Self(path)
        }

        fn prepare_game(&self) -> PathBuf {
            fs::write(self.0.join("GVOnline.exe"), []).expect("write test executable");
            let resources = self.0.join("0010").join("0001");
            fs::create_dir_all(&resources).expect("create resource directory");
            resources
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn push_u32(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_index(path: &Path, group_code: u32) {
        write_index_records(path, &[[7, 0, 48, 48, group_code]], 1);
    }

    fn write_index_records(path: &Path, records: &[[u32; 5]], image_block_count: u32) {
        let mut bytes = Vec::new();
        let group_count = records
            .iter()
            .map(|record| record[4])
            .collect::<BTreeSet<_>>()
            .len() as u32;
        for value in [
            records.len() as u32,
            group_count,
            48,
            48,
            image_block_count,
            1,
            0,
        ] {
            push_u32(&mut bytes, value);
        }
        for record in records {
            for value in record {
                push_u32(&mut bytes, *value);
            }
        }
        fs::write(path, bytes).expect("write test index");
    }

    fn zlib_block(raw: &[u8]) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(raw).expect("write zlib input");
        let compressed = encoder.finish().expect("finish zlib stream");
        let mut block = b"MWC\x1a".to_vec();
        push_u32(&mut block, raw.len() as u32);
        push_u32(&mut block, compressed.len() as u32);
        block.extend_from_slice(&compressed);
        block
    }

    fn write_data_file(path: &Path, blocks: &[Vec<u8>]) {
        let data = blocks
            .iter()
            .flat_map(|raw| zlib_block(raw))
            .collect::<Vec<_>>();
        fs::write(path, data).expect("write test data file");
    }

    fn test_thumbnail(icon_id: u32, data_url_bytes: usize) -> VerifiedAssetThumbnail {
        VerifiedAssetThumbnail {
            archive: "sb".to_owned(),
            icon_id: Some(icon_id),
            block_index: icon_id,
            source_width: 1,
            source_height: 1,
            thumbnail_width: 1,
            thumbnail_height: 1,
            assembled: false,
            thumbnail_data_url: "x".repeat(data_url_bytes),
        }
    }

    fn test_thumbnail_key(icon_id: u32) -> ThumbnailCacheKey {
        ThumbnailCacheKey {
            prefix: "sb".to_owned(),
            icon_id,
            canonical_block: icon_id,
            assembled: false,
        }
    }

    #[test]
    fn thumbnail_cache_evicts_the_least_recently_used_item() {
        let mut cache = ThumbnailCache::with_limits(2, usize::MAX);
        let first = test_thumbnail_key(1);
        let second = test_thumbnail_key(2);
        let third = test_thumbnail_key(3);
        cache.insert(first.clone(), test_thumbnail(1, 1));
        cache.insert(second.clone(), test_thumbnail(2, 1));

        assert_eq!(cache.get(&first).expect("first thumbnail").icon_id, Some(1));
        cache.insert(third.clone(), test_thumbnail(3, 1));

        assert!(cache.get(&second).is_none());
        assert!(cache.get(&first).is_some());
        assert!(cache.get(&third).is_some());
        assert_eq!(cache.entries.len(), 2);
    }

    #[test]
    fn thumbnail_cache_enforces_its_byte_limit() {
        let item_size = thumbnail_cache_size(&test_thumbnail(1, 4));
        let mut cache = ThumbnailCache::with_limits(10, item_size * 2);
        let first = test_thumbnail_key(1);
        let second = test_thumbnail_key(2);
        let third = test_thumbnail_key(3);
        cache.insert(first.clone(), test_thumbnail(1, 4));
        cache.insert(second.clone(), test_thumbnail(2, 4));
        assert_eq!(cache.total_bytes, item_size * 2);

        cache.insert(third.clone(), test_thumbnail(3, 4));

        assert!(cache.get(&first).is_none());
        assert!(cache.get(&second).is_some());
        assert!(cache.get(&third).is_some());
        assert_eq!(cache.total_bytes, item_size * 2);
    }

    #[test]
    fn thumbnail_cache_does_not_store_an_oversized_item() {
        let thumbnail = test_thumbnail(1, 4);
        let mut cache = ThumbnailCache::with_limits(10, thumbnail_cache_size(&thumbnail) - 1);
        let key = test_thumbnail_key(1);

        cache.insert(key.clone(), thumbnail);

        assert!(cache.get(&key).is_none());
        assert_eq!(cache.total_bytes, 0);
    }

    #[test]
    fn viewer_session_clears_thumbnails_only_when_the_directory_changes() {
        let mut session = ViewerSession::default();
        session.set_resource_directory("first");
        let key = test_thumbnail_key(1);
        session
            .thumbnail_cache
            .insert(key.clone(), test_thumbnail(1, 1));

        session.set_resource_directory("first");
        assert!(session.thumbnail_cache.get(&key).is_some());

        session.set_resource_directory("second");
        assert!(session.thumbnail_cache.get(&key).is_none());
        assert_eq!(session.thumbnail_cache.total_bytes, 0);
    }

    #[test]
    fn reports_supported_archive_headers() {
        let directory = TestDirectory::new();
        let resources = directory.prepare_game();
        let secondary_resources = resources.parent().expect("0010 directory").join("0002");
        fs::create_dir(&secondary_resources).expect("create secondary resource directory");
        write_index_records(&resources.join("im000000.bin"), &[[0, 0, 128, 128, 0]], 1);
        write_index_records(&resources.join("sa000000.bin"), &[[0, 0, 48, 48, 0]], 1);
        write_index(&resources.join("sb000000.bin"), 10);
        write_index_records(&resources.join("se000000.bin"), &[[0, 0, 120, 24, 0]], 1);
        write_index_records(
            &resources.join("sf000000.bin"),
            &[[1, 0, 120, 24, 0], [1, 1_136, 72, 72, 1]],
            1_137,
        );
        write_index_records(
            &resources.join("sg000000.bin"),
            &[[1, 0, 40, 24, 0], [2_000, 0, 40, 24, 0]],
            1,
        );
        write_index_records(
            &secondary_resources.join("sw000000.bin"),
            &[[0, 0, 80, 80, 0]],
            1,
        );
        write_index_records(
            &secondary_resources.join("sx000000.bin"),
            &[[0, 0, 256, 384, 0]],
            1,
        );
        write_index_records(
            &secondary_resources.join("sy000000.bin"),
            &[[0, 0, 512, 256, 0]],
            1,
        );
        write_index_records(
            &secondary_resources.join("sz000000.bin"),
            &[[0, 0, 256, 384, 0]],
            1,
        );
        write_index(&resources.join("is000000.bin"), 20);

        let summary = inspect_game_directory(&directory.0).expect("inspect game directory");

        assert_eq!(
            summary
                .archives
                .iter()
                .map(|archive| archive.prefix.as_str())
                .collect::<Vec<_>>(),
            [
                "im", "sa", "sb", "se", "sf", "sg", "sw", "sx", "sy", "sz", "is",
            ]
        );
        assert_eq!(
            PathBuf::from(&summary.resource_directory),
            directory.0.join("0010")
        );
        assert_eq!(summary.archives[0].record_count, 1);
        assert_eq!(summary.archives[0].group_count, 1);
        assert_eq!(summary.archives[0].image_block_count, 1);
        assert_eq!(summary.archives[0].archive_count, 1);
        assert_eq!(
            summary.verified_categories,
            [
                VerifiedCategorySummary {
                    path: ["UI 아이콘", "원형 아이콘"].map(str::to_owned).to_vec(),
                    asset_count: 1,
                },
                VerifiedCategorySummary {
                    path: ["UI 이미지", "버튼"].map(str::to_owned).to_vec(),
                    asset_count: 2,
                },
                VerifiedCategorySummary {
                    path: ["UI 이미지", "텍스트 라벨"].map(str::to_owned).to_vec(),
                    asset_count: 1,
                },
                VerifiedCategorySummary {
                    path: ["이벤트", "삽화"].map(str::to_owned).to_vec(),
                    asset_count: 1,
                },
                VerifiedCategorySummary {
                    path: ["이벤트", "포트레잇"].map(str::to_owned).to_vec(),
                    asset_count: 2,
                },
                VerifiedCategorySummary {
                    path: ["인물", "부관 스킬"].map(str::to_owned).to_vec(),
                    asset_count: 1,
                },
                VerifiedCategorySummary {
                    path: ["인물", "캐릭터 얼굴"].map(str::to_owned).to_vec(),
                    asset_count: 1,
                },
                VerifiedCategorySummary {
                    path: ["장비", "방어구", "몸"].map(str::to_owned).to_vec(),
                    asset_count: 1,
                },
                VerifiedCategorySummary {
                    path: ["지도", "국가 선택 지도"].map(str::to_owned).to_vec(),
                    asset_count: 1,
                },
                VerifiedCategorySummary {
                    path: ["클라이언트", "로딩·스플래시 이미지"]
                        .map(str::to_owned)
                        .to_vec(),
                    asset_count: 1,
                },
            ]
        );
    }

    #[test]
    fn loads_verified_thumbnails_from_the_secondary_resource_directory() {
        let directory = TestDirectory::new();
        let primary_resources = directory.prepare_game();
        let resource_root = primary_resources.parent().expect("0010 directory");
        let secondary_resources = resource_root.join("0002");
        fs::create_dir(&secondary_resources).expect("create secondary resource directory");
        write_index_records(
            &secondary_resources.join("sw000000.bin"),
            &[[0, 0, 1, 1, 0]],
            1,
        );
        write_data_file(
            &secondary_resources.join("sw000001.bin"),
            &[vec![0, 0, 255, 255]],
        );

        let mut session = ViewerSession::default();
        session.set_resource_directory(resource_root);
        let category = ["인물", "캐릭터 얼굴"].map(str::to_owned);
        let page = session
            .category_page(&category, 0, 1)
            .expect("load SW thumbnail page");

        assert_eq!(page.total_count, 1);
        assert_eq!(page.items[0].archive, "sw");
        assert_eq!(
            (page.items[0].source_width, page.items[0].source_height),
            (1, 1)
        );
        assert!(
            page.items[0]
                .thumbnail_data_url
                .starts_with("data:image/png;base64,")
        );
    }

    #[test]
    fn summarizes_only_verified_unique_and_assembled_assets() {
        let directory = TestDirectory::new();
        let resources = directory.prepare_game();
        write_index_records(
            &resources.join("sb000000.bin"),
            &[
                [100_100, 0, 48, 48, 1],
                [100_101, 0, 48, 48, 2],
                [100_102, 1, 48, 48, 1],
                [1_200_002, 2, 48, 48, 1],
            ],
            3,
        );
        let sd_records = (0..28)
            .map(|offset| [offset + 1, 10_368 + offset, 128, 128, 33])
            .collect::<Vec<_>>();
        write_index_records(&resources.join("sd000000.bin"), &sd_records, 10_396);

        let summary = inspect_game_directory(&directory.0).expect("inspect categorized game");
        let head = summary
            .verified_categories
            .iter()
            .find(|category| category.path == ["장비", "방어구", "머리"])
            .expect("head equipment category");
        let book = summary
            .verified_categories
            .iter()
            .find(|category| category.path == ["UI 이미지", "예지의 서", "표지"])
            .expect("Book of Wisdom category");

        assert_eq!(head.asset_count, 2);
        assert_eq!(book.asset_count, 1);
        assert_eq!(
            summary
                .verified_categories
                .iter()
                .map(|category| category.asset_count)
                .sum::<usize>(),
            3
        );
    }

    #[test]
    fn pages_verified_category_thumbnails_without_duplicate_blocks() {
        let directory = TestDirectory::new();
        let resources = directory.prepare_game();
        write_index_records(
            &resources.join("sb000000.bin"),
            &[
                [100_100, 0, 2, 1, 1],
                [100_101, 0, 2, 1, 2],
                [100_102, 1, 1, 2, 1],
                [1_200_002, 2, 1, 1, 1],
            ],
            3,
        );
        write_data_file(
            &resources.join("sb000001.bin"),
            &[
                vec![0, 0, 255, 255, 0, 0, 255, 255],
                vec![0, 255, 0, 255, 0, 255, 0, 255],
                vec![255, 0, 0, 255],
            ],
        );
        let mut session = ViewerSession::default();
        session.set_resource_directory(&resources);
        let category = ["장비", "방어구", "머리"].map(str::to_owned);

        let first = session
            .category_page(&category, 0, 1)
            .expect("load first thumbnail page");
        let second = session
            .category_page(&category, 1, 1)
            .expect("load second thumbnail page");

        assert_eq!(first.total_count, 2);
        assert_eq!(first.items.len(), 1);
        assert_eq!(first.items[0].block_index, 0);
        assert_eq!(
            (first.items[0].source_width, first.items[0].source_height),
            (2, 1)
        );
        assert!(!first.items[0].assembled);
        assert!(
            first.items[0]
                .thumbnail_data_url
                .starts_with("data:image/png;base64,")
        );
        assert_eq!(second.items[0].block_index, 1);

        let detail = session
            .asset_detail(&category, "SB", 0)
            .expect("load verified asset detail");
        assert_eq!((detail.source_width, detail.source_height), (2, 1));
        assert_eq!((detail.preview_width, detail.preview_height), (2, 1));
        assert!(!detail.assembled);
        assert!(
            detail
                .preview_data_url
                .starts_with("data:image/png;base64,")
        );

        let png = session
            .asset_png(&category, "SB", 0)
            .expect("extract verified asset PNG");
        assert_eq!((png.width, png.height), (2, 1));
        assert_eq!(png.block_index, 0);
        assert!(!png.assembled);
        assert_eq!(&png.png[..8], b"\x89PNG\r\n\x1a\n");

        let category_assets = session
            .category_assets(&category)
            .expect("load verified category assets");
        assert_eq!(category_assets.len(), 2);
        assert_eq!(category_assets[0].archive(), "sb");
        assert_eq!(category_assets[0].icon_id(), Some(100_100));
        assert_eq!(category_assets[0].block_index(), 0);
        assert!(!category_assets[0].assembled());
        let category_png = session
            .category_asset_png(&category_assets[1])
            .expect("extract category asset directly");
        assert_eq!(category_png.block_index, 1);
        assert_eq!(&category_png.png[..8], b"\x89PNG\r\n\x1a\n");

        let detail_error = session.asset_detail(&category, "sb", 2).unwrap_err();
        assert!(matches!(
            detail_error,
            ViewerSessionError::AssetNotFound {
                ref path,
                ref prefix,
                block_index: 2,
            } if path == &category && prefix == "sb"
        ));

        let png_error = session.asset_png(&category, "sb", 2).unwrap_err();
        assert!(matches!(
            png_error,
            ViewerSessionError::AssetNotFound { block_index: 2, .. }
        ));

        let error = session.category_page(&category, 2, 1).unwrap_err();
        assert!(matches!(
            error,
            ViewerSessionError::OffsetOutOfRange {
                offset: 2,
                total_count: 2,
            }
        ));
    }

    #[test]
    fn searches_only_verified_assets_and_pages_thumbnails() {
        let directory = TestDirectory::new();
        let resources = directory.prepare_game();
        write_index_records(
            &resources.join("sb000000.bin"),
            &[
                [100_100, 0, 1, 1, 1],
                [100_101, 1, 1, 1, 1],
                [1_200_002, 2, 1, 1, 1],
            ],
            3,
        );
        write_data_file(
            &resources.join("sb000001.bin"),
            &[
                vec![0, 0, 255, 255],
                vec![255, 0, 0, 255],
                vec![0, 255, 0, 255],
            ],
        );
        let mut session = ViewerSession::default();
        session.set_resource_directory(&resources);

        let category_page = session
            .search_page("SB 방어구 머리", 0, 1)
            .expect("search verified category");
        assert_eq!(category_page.total_count, 2);
        assert_eq!(category_page.items.len(), 1);
        assert_eq!(category_page.items[0].path, ["장비", "방어구", "머리"]);
        assert_eq!(category_page.items[0].thumbnail.icon_id, Some(100_100));
        assert!(session.search_assets.is_some());

        let second_page = session
            .search_page("머리", 1, 1)
            .expect("reuse the search index for the next page");
        assert_eq!(second_page.items[0].thumbnail.icon_id, Some(100_101));

        let id_page = session
            .search_page("100100", 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("search exact icon ID");
        assert_eq!(id_page.total_count, 1);
        assert_eq!(id_page.items[0].thumbnail.block_index, 0);

        let search_assets = session
            .search_assets("머리")
            .expect("list matching search assets without thumbnails");
        assert_eq!(search_assets.len(), 2);
        assert_eq!(search_assets[0].path(), ["장비", "방어구", "머리"]);
        assert_eq!(search_assets[1].icon_id(), Some(100_101));
        let search_png = session
            .search_asset_png(&search_assets[1])
            .expect("extract search asset PNG");
        assert_eq!(search_png.block_index, 1);
        assert_eq!(&search_png.png[..8], b"\x89PNG\r\n\x1a\n");

        let added_assets = vec![
            AssetSnapshotEntry::new("sb", 1, 100_100, 0, 1, 1),
            AssetSnapshotEntry::new("sb", 1, 100_101, 1, 1, 1),
            AssetSnapshotEntry::new("sb", 1, 1_200_002, 2, 1, 1),
            AssetSnapshotEntry::new("SB", 1, 100_100, 0, 1, 1),
        ];
        let update_page = session
            .update_page(&added_assets, 0, 1)
            .expect("page verified newly added assets");
        assert_eq!(update_page.detected_record_count, 4);
        assert_eq!(update_page.review_required_count, 1);
        assert_eq!(update_page.total_count, 2);
        assert_eq!(update_page.items.len(), 1);
        assert_eq!(update_page.items[0].path, ["장비", "방어구", "머리"]);
        assert_eq!(update_page.items[0].thumbnail.icon_id, Some(100_100));
        let second_update_page = session
            .update_page(&added_assets, 1, 1)
            .expect("page the next newly added asset");
        assert_eq!(second_update_page.items[0].thumbnail.icon_id, Some(100_101));

        let empty_page = session
            .search_page("없는 검색어", 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("return an empty search page");
        assert_eq!(empty_page.total_count, 0);
        assert!(empty_page.items.is_empty());

        let empty_query = session.search_page("  ", 0, 1).unwrap_err();
        assert!(matches!(empty_query, ViewerSessionError::EmptySearchQuery));
        let offset_error = session.search_page("머리", 2, 1).unwrap_err();
        assert!(matches!(
            offset_error,
            ViewerSessionError::OffsetOutOfRange {
                offset: 2,
                total_count: 2,
            }
        ));

        session.set_resource_directory(resources.join("other"));
        assert!(session.search_assets.is_none());
    }

    #[test]
    fn displays_and_extracts_verified_sh_raw_blocks_without_an_icon_id() {
        let directory = TestDirectory::new();
        let resources = directory.prepare_game();
        write_data_file(
            &resources.join("sh000001.bin"),
            &[vec![0x11; 256 * 256], vec![0xcc; 256 * 256]],
        );
        let mut session = ViewerSession::default();
        session.set_resource_directory(&resources);
        let category = ["UI 이미지", "별자리 조사", "별자리 선화 (256×256)"].map(str::to_owned);

        let page = session
            .category_page(&category, 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("load SH raw image page");
        assert_eq!(page.total_count, 2);
        assert_eq!(page.items[0].archive, "sh");
        assert_eq!(page.items[0].icon_id, None);
        assert_eq!(page.items[0].block_index, 0);
        assert_eq!(
            (page.items[0].source_width, page.items[0].source_height),
            (256, 256)
        );

        let detail = session
            .asset_detail(&category, "SH", 1)
            .expect("load SH raw image detail");
        assert_eq!(detail.icon_id, None);
        assert_eq!(detail.block_index, 1);
        assert_eq!((detail.preview_width, detail.preview_height), (256, 256));

        let png = session
            .asset_png(&category, "sh", 1)
            .expect("extract SH raw image PNG");
        assert_eq!(png.icon_id, None);
        assert_eq!((png.width, png.height), (256, 256));
        assert_eq!(&png.png[..8], b"\x89PNG\r\n\x1a\n");

        let search = session
            .search_page("SH 별자리 1", 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("search SH raw block");
        assert_eq!(search.total_count, 1);
        assert_eq!(search.items[0].thumbnail.block_index, 1);

        let added = AssetSnapshotEntry::new_raw(
            "sh",
            RawResourceKey {
                block_index: 1,
                file_number: 1,
                file_block_index: 1,
            },
            256,
            256,
        );
        let update = session
            .update_page(&[added], 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("show added SH raw block");
        assert_eq!(update.total_count, 1);
        assert_eq!(update.review_required_count, 0);
        assert_eq!(update.items[0].thumbnail.icon_id, None);
    }

    #[test]
    fn summarizes_sh_without_claiming_an_index_group() {
        let directory = TestDirectory::new();
        let resources = directory.prepare_game();
        write_data_file(
            &resources.join("sh000001.bin"),
            &[vec![0x11; 256 * 256], vec![0xcc; 256 * 256]],
        );

        let summary = inspect_game_directory(&directory.0).expect("inspect SH-only game fixture");

        assert_eq!(summary.archives.len(), 1);
        assert_eq!(summary.archives[0].prefix, "sh");
        assert!(!summary.archives[0].has_index);
        assert_eq!(summary.archives[0].record_count, 2);
        assert_eq!(summary.archives[0].group_count, 0);
        assert_eq!(summary.archives[0].image_block_count, 2);
        assert_eq!(summary.archives[0].archive_count, 1);
        assert_eq!(summary.verified_categories.len(), 1);
        assert_eq!(
            summary.verified_categories[0].path,
            ["UI 이미지", "별자리 조사", "별자리 선화 (256×256)"]
        );
        assert_eq!(summary.verified_categories[0].asset_count, 2);
    }

    #[test]
    fn exports_a_verified_assembly_as_one_png() {
        let directory = TestDirectory::new();
        let resources = directory.prepare_game();
        let records = (0..28)
            .map(|offset| {
                let width = if offset % 7 == 6 { 14 } else { 128 };
                let height = if offset / 7 == 3 { 20 } else { 128 };
                [offset + 1, 10_368 + offset, width, height, 33]
            })
            .collect::<Vec<_>>();
        write_index_records(&resources.join("sd000000.bin"), &records, 10_396);
        let mut blocks = vec![Vec::new(); 10_368];
        blocks.extend(
            records
                .iter()
                .map(|record| [16, 32, 64, 255].repeat((record[2] * record[3]) as usize)),
        );
        write_data_file(&resources.join("sd000001.bin"), &blocks);
        let mut session = ViewerSession::default();
        session.set_resource_directory(&resources);
        let category = ["UI 이미지", "예지의 서", "표지"].map(str::to_owned);

        let png = session
            .asset_png(&category, "sd", 10_368)
            .expect("extract verified assembly PNG");

        assert_eq!(png.archive, "sd");
        assert_eq!(png.block_index, 10_368);
        assert_eq!((png.width, png.height), (782, 404));
        assert!(png.assembled);
        assert_eq!(&png.png[..8], b"\x89PNG\r\n\x1a\n");

        let category_assets = session
            .category_assets(&category)
            .expect("load verified assembly category");
        assert_eq!(category_assets.len(), 1);
        assert!(category_assets[0].assembled());
        let category_png = session
            .category_asset_png(&category_assets[0])
            .expect("extract verified assembly directly");
        assert_eq!((category_png.width, category_png.height), (782, 404));
    }

    #[test]
    fn enforces_the_viewer_category_page_size() {
        let mut session = ViewerSession::default();
        let category = ["장비".to_owned()];

        let error = session
            .category_page(&category, 0, VIEWER_CATEGORY_PAGE_SIZE + 1)
            .unwrap_err();

        assert!(matches!(
            error,
            ViewerSessionError::InvalidPageSize {
                requested,
                maximum: VIEWER_CATEGORY_PAGE_SIZE,
            } if requested == VIEWER_CATEGORY_PAGE_SIZE + 1
        ));
    }

    #[test]
    fn rejects_a_folder_without_the_game_executable() {
        let directory = TestDirectory::new();

        let error = inspect_game_directory(&directory.0).unwrap_err();

        assert!(matches!(
            error,
            GameDirectoryError::MissingExecutable { .. }
        ));
    }

    #[test]
    fn rejects_a_folder_without_supported_archives() {
        let directory = TestDirectory::new();
        directory.prepare_game();

        let error = inspect_game_directory(&directory.0).unwrap_err();

        assert!(matches!(
            error,
            GameDirectoryError::NoSupportedArchives { .. }
        ));
    }

    #[test]
    fn reports_the_prefix_and_path_of_a_malformed_index() {
        let directory = TestDirectory::new();
        let resources = directory.prepare_game();
        fs::write(resources.join("sd000000.bin"), [1, 2, 3]).expect("write malformed index");

        let error = inspect_game_directory(&directory.0).unwrap_err();

        assert!(matches!(
            error,
            GameDirectoryError::ParseIndex { ref prefix, ref path, .. }
                if prefix == "sd" && path.ends_with("sd000000.bin")
        ));
    }
}
