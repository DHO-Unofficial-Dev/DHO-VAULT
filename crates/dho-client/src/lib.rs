// SPDX-License-Identifier: MPL-2.0

//! Read-only discovery and inspection of a DHO game client installation.

mod audio_catalog;
mod snapshot;
mod text_catalog;

pub use audio_catalog::{
    AUDIO_PAGE_SIZE, AudioCatalog, AudioCatalogError, AudioCatalogSummary, AudioTrackItem,
    AudioTrackOgg, AudioTrackPage,
};

pub use snapshot::{
    ASSET_SNAPSHOT_FORMAT_VERSION, AssetSnapshot, AssetSnapshotChange, AssetSnapshotCompareError,
    AssetSnapshotDiff, AssetSnapshotEntry, AssetSnapshotError, AssetSourceKind,
    inspect_asset_snapshot,
};
pub use text_catalog::{
    DEFAULT_TEXT_LANGUAGE_BLOCK, TEXT_IMAGE_RELATION_SNAPSHOT_FORMAT_VERSION, TEXT_PAGE_SIZE,
    TEXT_SNAPSHOT_FORMAT_VERSION, TextAssetLink, TextCatalog, TextCatalogError,
    TextCatalogPageError, TextCatalogSummary, TextImageLink, TextImageRelationEvidence,
    TextImageRelationSnapshot, TextImageRelationSnapshotEntry, TextImageRelationVerification,
    TextLanguageBlockSummary, TextRecordItem, TextRecordPage, TextSnapshot, TextSnapshotChange,
    TextSnapshotCompareError, TextSnapshotDiff, TextSnapshotEntry, TextSourceSummary,
    inspect_text_image_relation_snapshot, inspect_text_snapshot,
};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use dho_catalog::{LayeredAssemblyRule, layered_assembly_rule};
#[cfg(test)]
use dho_catalog::{assembly_plan, composite_assembly_rule};
use dho_core::{IndexParseError, IndexedArchive};
use dho_extract::{
    ExtractError, IndexedAssemblyLayout, LoadedArchive, LoadedGmAtlasArchive,
    LoadedRawImageArchive, LoadedStandaloneImageArchive, RawArchiveLayout, RawImageSpec,
    RawImageVariant, RawPixelFormat, RawResourceKey, ResourceKey, resolve_indexed_assembly_layout,
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
pub const SUPPORTED_ARCHIVE_PREFIXES: [&str; 20] = [
    "cu", "ft", "gm", "im", "kp", "sa", "sb", "sc", "sd", "se", "sf", "sg", "sh", "tm", "wm", "sw",
    "sx", "sy", "sz", "is",
];
pub const VIEWER_CATEGORY_PAGE_SIZE: usize = 64;
const GM_ATLAS_FILE_NUMBERS: &[u32] = &[0, 1, 2, 3];
const GM_ATLAS_IDENTITY_GROUP: u32 = u32::MAX;

#[derive(Debug, Clone, Copy)]
pub(crate) struct RawArchiveDefinition {
    pub prefix: &'static str,
    pub file_numbers: &'static [u32],
    pub layout: RawArchiveLayout,
    pub spec: RawImageSpec,
}

const SH_IMAGE_VARIANTS: &[RawImageVariant] = &[RawImageVariant {
    decoded_size: 65_536,
    width: 256,
    height: 256,
}];
const TM_IMAGE_VARIANTS: &[RawImageVariant] = &[
    RawImageVariant {
        decoded_size: 100_800,
        width: 180,
        height: 140,
    },
    RawImageVariant {
        decoded_size: 101_520,
        width: 180,
        height: 141,
    },
    RawImageVariant {
        decoded_size: 100_080,
        width: 180,
        height: 139,
    },
];
const KP_IMAGE_VARIANTS: &[RawImageVariant] = &[
    RawImageVariant {
        decoded_size: 9_216,
        width: 48,
        height: 48,
    },
    RawImageVariant {
        decoded_size: 262_144,
        width: 256,
        height: 256,
    },
];

pub(crate) const RAW_IMAGE_ARCHIVES: [RawArchiveDefinition; 3] = [
    RawArchiveDefinition {
        prefix: "kp",
        file_numbers: &[0, 10, 100_000],
        layout: RawArchiveLayout::InlineBlockTable,
        spec: RawImageSpec {
            pixel_format: RawPixelFormat::Bgra8,
            variants: KP_IMAGE_VARIANTS,
        },
    },
    RawArchiveDefinition {
        prefix: "sh",
        file_numbers: &[1],
        layout: RawArchiveLayout::BlocksOnly,
        spec: RawImageSpec {
            pixel_format: RawPixelFormat::Gray8,
            variants: SH_IMAGE_VARIANTS,
        },
    },
    RawArchiveDefinition {
        prefix: "tm",
        file_numbers: &[0],
        layout: RawArchiveLayout::InlineBlockTable,
        spec: RawImageSpec {
            pixel_format: RawPixelFormat::Bgra8,
            variants: TM_IMAGE_VARIANTS,
        },
    },
];

/// Resolves the physical subdirectory for an archive while preserving callers that already pass
/// a concrete archive directory.
pub fn resolve_archive_directory(resource_root: impl AsRef<Path>, prefix: &str) -> PathBuf {
    let resource_root = resource_root.as_ref();
    if resource_root.join(format!("{prefix}000000.bin")).is_file()
        || resource_root.join(format!("{prefix}000001.bin")).is_file()
    {
        return resource_root.to_owned();
    }

    let subdirectory = match prefix.to_ascii_lowercase().as_str() {
        "gm" => "local",
        "cu" | "ft" | "kp" | "tm" | "wm" => "0000",
        "sw" | "sx" | "sy" | "sz" => "0002",
        _ => "0001",
    };
    resource_root.join(subdirectory)
}

fn gm_archive_path(resource_root: &Path) -> PathBuf {
    resolve_archive_directory(resource_root, "gm").join("gm000000.bin")
}

fn standalone_image_path(resource_root: &Path, prefix: &str) -> Option<PathBuf> {
    let filename = match prefix {
        "cu" => "00000001.bin",
        "ft" => "00000000.bin",
        "wm" => "10000000.bin",
        _ => return None,
    };
    Some(resolve_archive_directory(resource_root, prefix).join(filename))
}

fn raw_archive_path(resource_root: &Path, definition: RawArchiveDefinition) -> Option<PathBuf> {
    let file_number = *definition.file_numbers.first()?;
    Some(
        resolve_archive_directory(resource_root, definition.prefix)
            .join(format!("{}{file_number:06}.bin", definition.prefix)),
    )
}

fn raw_layered_rule(prefix: &str, key: RawResourceKey) -> Option<LayeredAssemblyRule> {
    layered_assembly_rule(prefix)
        .filter(|rule| rule.contains_source(key.file_number, key.file_block_index))
}

fn raw_canonical_block(prefix: &str, key: RawResourceKey) -> u32 {
    raw_layered_rule(prefix, key).map_or(key.block_index, |rule| rule.canonical_block)
}

#[cfg(test)]
fn indexed_canonical_block(prefix: &str, block_index: u32) -> u32 {
    composite_assembly_rule(prefix, block_index).map_or_else(
        || assembly_plan(prefix, block_index).map_or(block_index, |plan| plan.first_block),
        |rule| rule.canonical_block,
    )
}

#[cfg(test)]
fn indexed_assembled(prefix: &str, block_index: u32) -> bool {
    composite_assembly_rule(prefix, block_index).is_some()
        || assembly_plan(prefix, block_index).is_some()
}

const EQUIPMENT_CATEGORY_NAMES: [&str; 6] = ["몸", "머리", "다리", "팔", "무기·도구", "장신구"];

fn equipment_category_path(prefix: &str, group_code: u32) -> Option<Vec<String>> {
    if !prefix.eq_ignore_ascii_case("sb") {
        return None;
    }
    EQUIPMENT_CATEGORY_NAMES
        .get(usize::try_from(group_code).ok()?)
        .map(|category| vec!["장비".to_owned(), (*category).to_owned()])
}

fn user_verified_group_category_path(prefix: &str, group_code: u32) -> Option<Vec<String>> {
    let category = if prefix.eq_ignore_ascii_case("sa") {
        match group_code {
            2 => Some("선박 그레이드 보너스"),
            _ => None,
        }
    } else if prefix.eq_ignore_ascii_case("sb") {
        match group_code {
            25 => Some("선박데코"),
            26 => Some("선원장비"),
            _ => None,
        }
    } else if prefix.eq_ignore_ascii_case("sc") {
        match group_code {
            4 => Some("돛 무늬"),
            5 => Some("주점 메뉴"),
            6 => Some("포커"),
            7 => Some("이벤트"),
            8 => Some("부관"),
            10 => Some("아팔타멘토 타입"),
            14 => Some("개인농장 시설"),
            16 => Some("테크닉"),
            20 => Some("대학·학술협회"),
            26 => Some("트레져헌트 테마"),
            31 => Some("전승(획득)"),
            32 => Some("트레져헌트 렐릭"),
            33 => Some("레거시 테마"),
            34 => Some("추구 생산"),
            35 => Some("위인의 장 테마"),
            36 => Some("잠재능력"),
            _ => None,
        }
    } else if prefix.eq_ignore_ascii_case("sd") {
        match group_code {
            4 => Some("입항허가"),
            29 => Some("전승(획득/큰이미지)"),
            _ => None,
        }
    } else if prefix.eq_ignore_ascii_case("sf") && group_code == 1 {
        Some("레거시")
    } else if prefix.eq_ignore_ascii_case("sy") && group_code == 0 {
        Some("역사적 사건")
    } else {
        None
    };
    category.map(|category| vec![category.to_owned()])
}

fn master_category_path(source_label: &str, prefix: &str, group_code: u32) -> Vec<String> {
    equipment_category_path(prefix, group_code)
        .or_else(|| user_verified_group_category_path(prefix, group_code))
        .unwrap_or_else(|| vec![source_label.to_owned()])
}

fn linked_category_path(source_label: &str, prefix: &str, group_code: u32) -> Vec<String> {
    master_category_path(source_label, prefix, group_code)
}

fn gm_sprite_category_path(atlas_block_index: u32) -> Vec<String> {
    vec![
        "UI 리소스".to_owned(),
        "GM".to_owned(),
        "원시 렌더링 조각".to_owned(),
        format!("아틀라스 ID {atlas_block_index:02}"),
    ]
}

fn gm_atlas_category_path() -> Vec<String> {
    vec![
        "UI 리소스".to_owned(),
        "GM".to_owned(),
        "원본 아틀라스".to_owned(),
    ]
}

fn physical_category_path(
    source_label: Option<&str>,
    prefix: &str,
    group_code: u32,
    assembled: bool,
    has_groups: bool,
) -> Vec<String> {
    if assembled {
        return vec!["조립 이미지".to_owned(), prefix.to_ascii_uppercase()];
    }
    if prefix.eq_ignore_ascii_case("tm") {
        return vec!["지도".to_owned(), "도시 미니맵".to_owned()];
    }
    if prefix.eq_ignore_ascii_case("gm") {
        return vec!["UI 리소스".to_owned(), "GM".to_owned()];
    }
    if prefix.eq_ignore_ascii_case("cu") {
        return vec!["UI 리소스".to_owned(), "커서".to_owned()];
    }
    if prefix.eq_ignore_ascii_case("ft") {
        return vec!["글꼴".to_owned(), "게임 글리프".to_owned()];
    }
    if prefix.eq_ignore_ascii_case("wm") {
        return vec!["지도".to_owned(), "축소 세계지도 리소스".to_owned()];
    }
    if let Some(path) = user_verified_group_category_path(prefix, group_code) {
        return path;
    }
    if let Some(source_label) = source_label {
        return master_category_path(source_label, prefix, group_code);
    }
    let mut path = vec!["미분류".to_owned(), prefix.to_ascii_uppercase()];
    if has_groups {
        path.push(format!("그룹 {group_code}"));
    }
    path
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
    pub catalog_diagnostics: CatalogDiagnosticsSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedCategorySummary {
    pub path: Vec<String>,
    pub asset_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogDiagnosticsSummary {
    pub total_asset_count: usize,
    pub categorized_asset_count: usize,
    pub unclassified_asset_count: usize,
    pub multiple_category_asset_count: usize,
    pub unclassified_categories: Vec<VerifiedCategorySummary>,
    pub multiple_category_assets: Vec<CatalogMultipleCategoryAsset>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogMultipleCategoryAsset {
    pub archive: String,
    pub identity_kind: String,
    pub primary_id: u32,
    pub secondary_id: u32,
    pub category_paths: Vec<Vec<String>>,
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
    pub text_links: Vec<TextAssetLink>,
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
    pub gm_source: Option<GmSpriteSourceDetail>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GmSpriteSourceDetail {
    pub atlas_id: u32,
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub preview_width: u32,
    pub preview_height: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
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
    gm_archive: Option<LoadedGmAtlasArchive>,
    standalone_archives: HashMap<String, LoadedStandaloneImageArchive>,
    search_assets: Option<Vec<VerifiedSearchAssetRef>>,
    thumbnail_cache: ThumbnailCache,
    text_catalog: Option<TextCatalog>,
    browse_text_catalog: Option<(usize, TextCatalog)>,
    active_text_language_block: Option<usize>,
}

#[derive(Debug, Clone)]
struct VerifiedAssetRef {
    prefix: String,
    key: ResourceKey,
    raw_key: Option<RawResourceKey>,
    canonical_block: u32,
    assembled: bool,
}

type AssetIdentityKey = (String, u8, u32, u32);

fn asset_identity_key(asset: &VerifiedAssetRef) -> AssetIdentityKey {
    if asset.assembled {
        return (asset.prefix.clone(), 0, asset.canonical_block, 0);
    }
    if let Some(raw_key) = asset.raw_key {
        if asset.prefix == "gm" {
            return if asset.key.group_code == GM_ATLAS_IDENTITY_GROUP {
                (
                    asset.prefix.clone(),
                    3,
                    raw_key.file_number,
                    asset.key.icon_id,
                )
            } else {
                (
                    asset.prefix.clone(),
                    4,
                    asset.key.group_code,
                    asset.key.icon_id,
                )
            };
        }
        return (
            asset.prefix.clone(),
            1,
            raw_key.file_number,
            raw_key.file_block_index,
        );
    }
    (
        asset.prefix.clone(),
        2,
        asset.key.group_code,
        asset.key.icon_id,
    )
}

impl VerifiedAssetRef {
    fn icon_id(&self) -> Option<u32> {
        (self.raw_key.is_none() || self.prefix == "gm").then_some(self.key.icon_id)
    }
}

#[derive(Debug, Clone)]
struct VerifiedSearchAssetRef {
    path: Vec<String>,
    asset: VerifiedAssetRef,
    search_text: String,
    search_names: Vec<String>,
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
            self.gm_archive = None;
            self.standalone_archives.clear();
            self.search_assets = None;
            self.thumbnail_cache.clear();
            self.text_catalog = None;
            self.browse_text_catalog = None;
            self.active_text_language_block = None;
            self.resource_directory = Some(path);
        }
    }

    pub fn text_catalog_summary(
        &mut self,
        language_block: usize,
    ) -> Result<TextCatalogSummary, ViewerSessionError> {
        Ok(self.text_catalog_for_language(language_block)?.summary())
    }

    pub fn set_text_language_block(
        &mut self,
        language_block: usize,
    ) -> Result<(), ViewerSessionError> {
        self.text_catalog_for_language(language_block)?;
        if self.active_text_language_block != Some(language_block) {
            self.active_text_language_block = Some(language_block);
            self.search_assets = None;
            self.thumbnail_cache.clear();
        }
        Ok(())
    }

    pub fn text_page(
        &mut self,
        language_block: usize,
        source: Option<&str>,
        query: &str,
        offset: usize,
        page_size: usize,
    ) -> Result<TextRecordPage, ViewerSessionError> {
        self.text_catalog_for_language(language_block)?
            .page(source, query, offset, page_size)
            .map_err(ViewerSessionError::TextPage)
    }

    pub fn text_records_for_keys(
        &mut self,
        keys: &[(String, u32)],
    ) -> Result<Vec<TextRecordItem>, ViewerSessionError> {
        Ok(self.text_catalog()?.records_for_keys(keys))
    }

    fn text_catalog(&mut self) -> Result<&TextCatalog, ViewerSessionError> {
        if self.text_catalog.is_none() {
            let resource_directory = self
                .resource_directory
                .as_deref()
                .ok_or(ViewerSessionError::ResourceDirectoryNotSelected)?;
            let game_directory = resource_directory
                .parent()
                .ok_or(ViewerSessionError::GameDirectoryNotFound)?;
            self.text_catalog = Some(
                TextCatalog::load(game_directory).map_err(ViewerSessionError::OpenTextCatalog)?,
            );
        }
        Ok(self
            .text_catalog
            .as_ref()
            .expect("text catalog initialized"))
    }

    fn text_catalog_for_language(
        &mut self,
        language_block: usize,
    ) -> Result<&TextCatalog, ViewerSessionError> {
        if language_block == DEFAULT_TEXT_LANGUAGE_BLOCK {
            return self.text_catalog();
        }
        let needs_load = self
            .browse_text_catalog
            .as_ref()
            .is_none_or(|(loaded_block, _)| *loaded_block != language_block);
        if needs_load {
            let resource_directory = self
                .resource_directory
                .as_deref()
                .ok_or(ViewerSessionError::ResourceDirectoryNotSelected)?;
            let game_directory = resource_directory
                .parent()
                .ok_or(ViewerSessionError::GameDirectoryNotFound)?;
            let catalog = TextCatalog::load_language_block(game_directory, language_block)
                .map_err(ViewerSessionError::OpenTextCatalog)?;
            self.browse_text_catalog = Some((language_block, catalog));
        }
        Ok(&self
            .browse_text_catalog
            .as_ref()
            .expect("browse text catalog initialized")
            .1)
    }

    fn active_text_catalog(&mut self) -> Result<&TextCatalog, ViewerSessionError> {
        let language_block = self
            .active_text_language_block
            .unwrap_or(DEFAULT_TEXT_LANGUAGE_BLOCK);
        self.text_catalog_for_language(language_block)
    }

    fn text_links_for_asset(&mut self, asset: &VerifiedAssetRef) -> Vec<TextAssetLink> {
        if asset.raw_key.is_some() {
            return Vec::new();
        }
        self.active_text_catalog()
            .ok()
            .map(|catalog| {
                catalog.links_for_image(&asset.prefix, asset.key.group_code, asset.key.icon_id)
            })
            .unwrap_or_default()
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
        let mut unique = BTreeMap::<
            (Vec<String>, AssetIdentityKey),
            (ResourceKey, Option<RawResourceKey>, u32, bool),
        >::new();
        let _ = self.active_text_catalog();
        for asset in added_assets {
            let raw_key = asset.raw_resource_key();
            if asset.source_kind == AssetSourceKind::RawBlock && raw_key.is_none() {
                review_required_count += 1;
                continue;
            }
            let prefix = asset.archive.to_ascii_lowercase();
            let (canonical_block, indexed_is_assembled) = if let Some(key) = raw_key {
                (raw_canonical_block(&asset.archive, key), false)
            } else {
                let archive = self.archive(&prefix)?;
                (
                    archive
                        .assembly_canonical_block(asset.block_index)
                        .unwrap_or(asset.block_index),
                    archive.is_assembled(asset.block_index),
                )
            };
            let paths = if let Some(raw_key) = raw_key {
                vec![physical_category_path(
                    None,
                    &prefix,
                    0,
                    raw_layered_rule(&prefix, raw_key).is_some(),
                    false,
                )]
            } else {
                let mut paths = self
                    .active_text_catalog()
                    .ok()
                    .map(|catalog| {
                        catalog
                            .links_for_image(&prefix, asset.group_code, asset.icon_id)
                            .into_iter()
                            .map(|link| {
                                linked_category_path(&link.source_label, &prefix, asset.group_code)
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if indexed_is_assembled {
                    paths.push(physical_category_path(
                        None,
                        &prefix,
                        asset.group_code,
                        true,
                        true,
                    ));
                }
                if paths.is_empty() {
                    let source_label = self
                        .active_text_catalog()
                        .ok()
                        .and_then(|catalog| {
                            catalog.source_label_for_image_group(&prefix, asset.group_code)
                        })
                        .map(str::to_owned);
                    paths.push(physical_category_path(
                        source_label.as_deref(),
                        &prefix,
                        asset.group_code,
                        false,
                        true,
                    ));
                }
                paths
            };
            for path in paths {
                let identity = asset_identity_key(&VerifiedAssetRef {
                    prefix: prefix.clone(),
                    key: ResourceKey {
                        group_code: asset.group_code,
                        icon_id: asset.icon_id,
                        block_index: asset.block_index,
                    },
                    raw_key,
                    canonical_block,
                    assembled: raw_key
                        .and_then(|key| raw_layered_rule(&prefix, key))
                        .is_some()
                        || indexed_is_assembled,
                });
                unique.entry((path, identity)).or_insert((
                    ResourceKey {
                        group_code: asset.group_code,
                        icon_id: asset.icon_id,
                        block_index: asset.block_index,
                    },
                    raw_key,
                    canonical_block,
                    indexed_is_assembled,
                ));
            }
        }

        let assets = unique
            .into_iter()
            .map(
                |(
                    (path, (prefix, _, _, _)),
                    (key, raw_key, canonical_block, indexed_is_assembled),
                )| {
                    let assembled = raw_key
                        .and_then(|key| raw_layered_rule(&prefix, key))
                        .is_some()
                        || indexed_is_assembled;
                    VerifiedSearchAssetRef {
                        path,
                        asset: VerifiedAssetRef {
                            prefix,
                            key,
                            raw_key,
                            canonical_block,
                            assembled,
                        },
                        search_text: String::new(),
                        search_names: Vec::new(),
                    }
                },
            )
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

    pub fn selected_asset(
        &mut self,
        path: &[String],
        prefix: &str,
        block_index: u32,
    ) -> Result<VerifiedSearchAsset, ViewerSessionError> {
        let asset = self.verified_asset(path, prefix, block_index)?;
        Ok(VerifiedSearchAsset(VerifiedSearchAssetRef {
            path: path.to_vec(),
            asset,
            search_text: path.join(" "),
            search_names: Vec::new(),
        }))
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
        self.asset_detail_from_ref(path, asset)
    }

    pub fn text_image_detail(
        &mut self,
        archive: &str,
        group_code: u32,
        icon_id: u32,
        relation: &str,
    ) -> Result<VerifiedAssetDetail, ViewerSessionError> {
        let prefix = archive.to_ascii_lowercase();
        let record = self
            .archive(&prefix)?
            .records()
            .iter()
            .find(|record| record.group_code == group_code && record.icon_id == icon_id)
            .copied()
            .ok_or_else(|| ViewerSessionError::TextImageNotFound {
                prefix: prefix.clone(),
                group_code,
                icon_id,
            })?;
        let archive = self.archive(&prefix)?;
        let canonical_block = archive
            .assembly_canonical_block(record.block_index)
            .unwrap_or(record.block_index);
        let assembled = archive.is_assembled(record.block_index);
        let asset = VerifiedAssetRef {
            prefix,
            key: ResourceKey {
                group_code,
                icon_id,
                block_index: record.block_index,
            },
            raw_key: None,
            canonical_block,
            assembled,
        };
        self.asset_detail_from_ref(&["텍스트 자료".to_owned(), relation.to_owned()], asset)
    }

    fn asset_detail_from_ref(
        &mut self,
        path: &[String],
        asset: VerifiedAssetRef,
    ) -> Result<VerifiedAssetDetail, ViewerSessionError> {
        if let Some(raw_key) = asset.raw_key {
            if matches!(asset.prefix.as_str(), "cu" | "ft" | "wm") {
                let extracted = self
                    .standalone_archive(&asset.prefix)?
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
                    icon_id: Some(raw_key.file_block_index),
                    block_index: asset.canonical_block,
                    source_width: extracted.source_width,
                    source_height: extracted.source_height,
                    preview_width: extracted.width,
                    preview_height: extracted.height,
                    assembled: false,
                    preview_data_url: png_data_url(&extracted.png),
                    gm_source: None,
                });
            }
            if asset.prefix == "gm" {
                let icon_id = asset.key.icon_id;
                let archive = self.gm_archive()?;
                let extracted = archive
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
                let gm_source = if let Some(source) = archive.sprite_source(raw_key) {
                    let preview = archive
                        .extract_thumbnail_png(
                            source.atlas_key,
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
                    Some(GmSpriteSourceDetail {
                        atlas_id: source.atlas_block_index,
                        atlas_width: source.atlas_width,
                        atlas_height: source.atlas_height,
                        preview_width: preview.width,
                        preview_height: preview.height,
                        x: source.x,
                        y: source.y,
                        width: source.width,
                        height: source.height,
                        preview_data_url: png_data_url(&preview.png),
                    })
                } else {
                    None
                };
                return Ok(VerifiedAssetDetail {
                    path: path.to_vec(),
                    archive: asset.prefix,
                    icon_id: Some(icon_id),
                    block_index: asset.canonical_block,
                    source_width: extracted.source_width,
                    source_height: extracted.source_height,
                    preview_width: extracted.width,
                    preview_height: extracted.height,
                    assembled: false,
                    preview_data_url: png_data_url(&extracted.png),
                    gm_source,
                });
            }
            if asset.assembled {
                let rule = raw_layered_rule(&asset.prefix, raw_key).ok_or_else(|| {
                    ViewerSessionError::AssemblyRuleMissing {
                        prefix: asset.prefix.clone(),
                        block_index: asset.canonical_block,
                    }
                })?;
                let extracted = self
                    .raw_archive(&asset.prefix)?
                    .extract_verified_layered_assembly_thumbnail(
                        rule,
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
                    })?;
                return Ok(VerifiedAssetDetail {
                    path: path.to_vec(),
                    archive: asset.prefix,
                    icon_id: None,
                    block_index: extracted.first_block,
                    source_width: extracted.source_width,
                    source_height: extracted.source_height,
                    preview_width: extracted.width,
                    preview_height: extracted.height,
                    assembled: true,
                    preview_data_url: png_data_url(&extracted.png),
                    gm_source: None,
                });
            }
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
                gm_source: None,
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
                gm_source: None,
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
                gm_source: None,
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
            if matches!(asset.prefix.as_str(), "cu" | "ft" | "wm") {
                let extracted = self
                    .standalone_archive(&asset.prefix)?
                    .extract_png(raw_key, MAX_IMAGE_DECODE_SIZE)
                    .map_err(|source| ViewerSessionError::Extract {
                        prefix: asset.prefix.clone(),
                        block_index: asset.canonical_block,
                        source,
                    })?;
                return Ok(VerifiedAssetPng {
                    archive: asset.prefix,
                    icon_id: Some(raw_key.file_block_index),
                    block_index: asset.canonical_block,
                    width: extracted.width,
                    height: extracted.height,
                    assembled: false,
                    png: extracted.png,
                });
            }
            if asset.prefix == "gm" {
                let icon_id = asset.key.icon_id;
                let extracted = self
                    .gm_archive()?
                    .extract_png(raw_key, MAX_IMAGE_DECODE_SIZE)
                    .map_err(|source| ViewerSessionError::Extract {
                        prefix: asset.prefix.clone(),
                        block_index: asset.canonical_block,
                        source,
                    })?;
                return Ok(VerifiedAssetPng {
                    archive: asset.prefix,
                    icon_id: Some(icon_id),
                    block_index: asset.canonical_block,
                    width: extracted.width,
                    height: extracted.height,
                    assembled: false,
                    png: extracted.png,
                });
            }
            if asset.assembled {
                let rule = raw_layered_rule(&asset.prefix, raw_key).ok_or_else(|| {
                    ViewerSessionError::AssemblyRuleMissing {
                        prefix: asset.prefix.clone(),
                        block_index: asset.canonical_block,
                    }
                })?;
                let extracted = self
                    .raw_archive(&asset.prefix)?
                    .extract_verified_layered_assembly(
                        rule,
                        MAX_IMAGE_DECODE_SIZE,
                        MAX_ASSEMBLED_DECODE_SIZE,
                    )
                    .map_err(|source| ViewerSessionError::Extract {
                        prefix: asset.prefix.clone(),
                        block_index: asset.canonical_block,
                        source,
                    })?;
                return Ok(VerifiedAssetPng {
                    archive: asset.prefix,
                    icon_id: None,
                    block_index: extracted.first_block,
                    width: extracted.width,
                    height: extracted.height,
                    assembled: true,
                    png: extracted.png,
                });
            }
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
        let text_links = self.text_links_for_asset(&asset);

        if let Some(raw_key) = asset.raw_key {
            if matches!(asset.prefix.as_str(), "cu" | "ft" | "wm") {
                let extracted = self
                    .standalone_archive(&asset.prefix)?
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
                    icon_id: Some(raw_key.file_block_index),
                    block_index: asset.canonical_block,
                    source_width: extracted.source_width,
                    source_height: extracted.source_height,
                    thumbnail_width: extracted.width,
                    thumbnail_height: extracted.height,
                    assembled: false,
                    thumbnail_data_url: png_data_url(&extracted.png),
                    text_links,
                };
                self.thumbnail_cache.insert(cache_key, thumbnail.clone());
                return Ok(thumbnail);
            }
            if asset.prefix == "gm" {
                let icon_id = asset.key.icon_id;
                let extracted = self
                    .gm_archive()?
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
                    icon_id: Some(icon_id),
                    block_index: asset.canonical_block,
                    source_width: extracted.source_width,
                    source_height: extracted.source_height,
                    thumbnail_width: extracted.width,
                    thumbnail_height: extracted.height,
                    assembled: false,
                    thumbnail_data_url: png_data_url(&extracted.png),
                    text_links,
                };
                self.thumbnail_cache.insert(cache_key, thumbnail.clone());
                return Ok(thumbnail);
            }
            if asset.assembled {
                let rule = raw_layered_rule(&asset.prefix, raw_key).ok_or_else(|| {
                    ViewerSessionError::AssemblyRuleMissing {
                        prefix: asset.prefix.clone(),
                        block_index: asset.canonical_block,
                    }
                })?;
                let extracted = self
                    .raw_archive(&asset.prefix)?
                    .extract_verified_layered_assembly_thumbnail(
                        rule,
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
                    })?;
                let thumbnail = VerifiedAssetThumbnail {
                    archive: asset.prefix,
                    icon_id: None,
                    block_index: extracted.first_block,
                    source_width: extracted.source_width,
                    source_height: extracted.source_height,
                    thumbnail_width: extracted.width,
                    thumbnail_height: extracted.height,
                    assembled: true,
                    thumbnail_data_url: png_data_url(&extracted.png),
                    text_links: text_links.clone(),
                };
                self.thumbnail_cache.insert(cache_key, thumbnail.clone());
                return Ok(thumbnail);
            }
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
                text_links: text_links.clone(),
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
                text_links: text_links.clone(),
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
                text_links,
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
        let mut assets = self.physical_assets_for_path(path)?;
        let records = self
            .active_text_catalog()
            .map(TextCatalog::linked_images)
            .unwrap_or_default();
        for record in records.into_iter().filter(|record| {
            linked_category_path(
                &record.source_label,
                &record.image.archive,
                record.image.group_code,
            ) == path
        }) {
            let prefix = record.image.archive.to_ascii_lowercase();
            let canonical_block = self
                .archive(&prefix)?
                .assembly_canonical_block(record.image.block_index)
                .unwrap_or(record.image.block_index);
            let assembled = self
                .archive(&prefix)?
                .is_assembled(record.image.block_index);
            let asset = VerifiedAssetRef {
                prefix: prefix.clone(),
                key: ResourceKey {
                    group_code: record.image.group_code,
                    icon_id: record.image.icon_id,
                    block_index: record.image.block_index,
                },
                raw_key: None,
                canonical_block,
                assembled,
            };
            assets.entry(asset_identity_key(&asset)).or_insert(asset);
        }
        Ok(assets.into_values().collect())
    }

    fn physical_assets_for_path(
        &mut self,
        requested_path: &[String],
    ) -> Result<BTreeMap<AssetIdentityKey, VerifiedAssetRef>, ViewerSessionError> {
        let resource_directory = self
            .resource_directory
            .clone()
            .ok_or(ViewerSessionError::ResourceDirectoryNotSelected)?;
        let mut assets = BTreeMap::<AssetIdentityKey, VerifiedAssetRef>::new();
        for prefix in INDEXED_ARCHIVE_PREFIXES {
            if !resolve_archive_directory(&resource_directory, prefix)
                .join(format!("{prefix}000000.bin"))
                .is_file()
            {
                continue;
            }
            let records = self.archive(prefix)?.records().to_vec();
            for record in records {
                let archive = self.archive(prefix)?;
                let canonical_block = archive
                    .assembly_canonical_block(record.block_index)
                    .unwrap_or(record.block_index);
                let assembled = archive.is_assembled(record.block_index);
                let source_label = self
                    .active_text_catalog()
                    .ok()
                    .and_then(|catalog| {
                        catalog.source_label_for_image_group(prefix, record.group_code)
                    })
                    .map(str::to_owned);
                let category_path = physical_category_path(
                    source_label.as_deref(),
                    prefix,
                    record.group_code,
                    assembled,
                    true,
                );
                if category_path != requested_path {
                    continue;
                }
                let asset = VerifiedAssetRef {
                    prefix: prefix.to_owned(),
                    key: ResourceKey {
                        group_code: record.group_code,
                        icon_id: record.icon_id,
                        block_index: record.block_index,
                    },
                    raw_key: None,
                    canonical_block,
                    assembled,
                };
                assets.entry(asset_identity_key(&asset)).or_insert(asset);
            }
        }

        for definition in RAW_IMAGE_ARCHIVES {
            if !raw_archive_path(&resource_directory, definition).is_some_and(|path| path.is_file())
            {
                continue;
            }
            let records = self
                .raw_archive(definition.prefix)?
                .records()
                .collect::<Vec<_>>();
            for record in records {
                let canonical_block = raw_canonical_block(definition.prefix, record.key);
                let assembled = raw_layered_rule(definition.prefix, record.key).is_some();
                let category_path =
                    physical_category_path(None, definition.prefix, 0, assembled, false);
                if category_path != requested_path {
                    continue;
                }
                let asset = VerifiedAssetRef {
                    prefix: definition.prefix.to_owned(),
                    key: ResourceKey {
                        group_code: 0,
                        icon_id: record.key.block_index,
                        block_index: record.key.block_index,
                    },
                    raw_key: Some(record.key),
                    canonical_block,
                    assembled,
                };
                assets.entry(asset_identity_key(&asset)).or_insert(asset);
            }
        }
        if gm_archive_path(&resource_directory).is_file() {
            let atlas_records = self.gm_archive()?.atlas_records().collect::<Vec<_>>();
            if gm_atlas_category_path() == requested_path {
                for record in atlas_records {
                    let asset = VerifiedAssetRef {
                        prefix: "gm".to_owned(),
                        key: ResourceKey {
                            group_code: GM_ATLAS_IDENTITY_GROUP,
                            icon_id: record.atlas_block_index,
                            block_index: record.key.block_index,
                        },
                        raw_key: Some(record.key),
                        canonical_block: record.key.block_index,
                        assembled: false,
                    };
                    assets.insert(asset_identity_key(&asset), asset);
                }
            }
            let sprite_records = self.gm_archive()?.records().collect::<Vec<_>>();
            for record in sprite_records {
                let category_path = gm_sprite_category_path(record.atlas_block_index);
                if category_path != requested_path {
                    continue;
                }
                let asset = VerifiedAssetRef {
                    prefix: "gm".to_owned(),
                    key: ResourceKey {
                        group_code: record.atlas_block_index,
                        icon_id: record.sprite_index,
                        block_index: record.key.block_index,
                    },
                    raw_key: Some(record.key),
                    canonical_block: record.key.block_index,
                    assembled: false,
                };
                assets.insert(asset_identity_key(&asset), asset);
            }
        }
        for prefix in ["cu", "ft", "wm"] {
            if !standalone_image_path(&resource_directory, prefix)
                .is_some_and(|path| path.is_file())
            {
                continue;
            }
            let records = self
                .standalone_archive(prefix)?
                .records()
                .collect::<Vec<_>>();
            for record in records {
                let category_path = physical_category_path(None, prefix, 0, false, false);
                if category_path != requested_path {
                    continue;
                }
                let asset = VerifiedAssetRef {
                    prefix: prefix.to_owned(),
                    key: ResourceKey {
                        group_code: 0,
                        icon_id: record.key.file_block_index,
                        block_index: record.key.block_index,
                    },
                    raw_key: Some(record.key),
                    canonical_block: record.key.block_index,
                    assembled: false,
                };
                assets.insert(asset_identity_key(&asset), asset);
            }
        }
        Ok(assets)
    }

    fn unlinked_assets(
        &mut self,
        requested_prefix: &str,
    ) -> Result<Vec<VerifiedAssetRef>, ViewerSessionError> {
        let prefix = requested_prefix.to_ascii_lowercase();
        let resource_directory = self
            .resource_directory
            .clone()
            .ok_or(ViewerSessionError::ResourceDirectoryNotSelected)?;
        if INDEXED_ARCHIVE_PREFIXES.contains(&prefix.as_str()) {
            return self.unlinked_indexed_assets(&prefix);
        }

        if prefix == "gm" {
            if !gm_archive_path(&resource_directory).is_file() {
                return Ok(Vec::new());
            }
            let atlas_records = self.gm_archive()?.atlas_records().collect::<Vec<_>>();
            let sprite_records = self.gm_archive()?.records().collect::<Vec<_>>();
            let mut assets = atlas_records
                .into_iter()
                .map(|record| VerifiedAssetRef {
                    prefix: prefix.clone(),
                    key: ResourceKey {
                        group_code: GM_ATLAS_IDENTITY_GROUP,
                        icon_id: record.atlas_block_index,
                        block_index: record.key.block_index,
                    },
                    raw_key: Some(record.key),
                    canonical_block: record.key.block_index,
                    assembled: false,
                })
                .collect::<Vec<_>>();
            assets.extend(sprite_records.into_iter().map(|record| VerifiedAssetRef {
                prefix: prefix.clone(),
                key: ResourceKey {
                    group_code: record.atlas_block_index,
                    icon_id: record.sprite_index,
                    block_index: record.key.block_index,
                },
                raw_key: Some(record.key),
                canonical_block: record.key.block_index,
                assembled: false,
            }));
            return Ok(assets);
        }

        if matches!(prefix.as_str(), "cu" | "ft" | "wm") {
            if !standalone_image_path(&resource_directory, &prefix)
                .is_some_and(|path| path.is_file())
            {
                return Ok(Vec::new());
            }
            let records = self
                .standalone_archive(&prefix)?
                .records()
                .collect::<Vec<_>>();
            return Ok(records
                .into_iter()
                .map(|record| VerifiedAssetRef {
                    prefix: prefix.clone(),
                    key: ResourceKey {
                        group_code: 0,
                        icon_id: record.key.file_block_index,
                        block_index: record.key.block_index,
                    },
                    raw_key: Some(record.key),
                    canonical_block: record.key.block_index,
                    assembled: false,
                })
                .collect());
        }

        let Some(definition) = RAW_IMAGE_ARCHIVES
            .iter()
            .find(|definition| definition.prefix == prefix)
            .copied()
        else {
            return Ok(Vec::new());
        };
        if !raw_archive_path(&resource_directory, definition).is_some_and(|path| path.is_file()) {
            return Ok(Vec::new());
        }
        let records = self.raw_archive(&prefix)?.records().collect::<Vec<_>>();
        let mut assets = BTreeMap::<u32, RawResourceKey>::new();
        for record in records {
            if raw_layered_rule(&prefix, record.key).is_some() {
                continue;
            }
            assets
                .entry(raw_canonical_block(&prefix, record.key))
                .or_insert(record.key);
        }
        Ok(assets
            .into_iter()
            .map(|(canonical_block, raw_key)| VerifiedAssetRef {
                prefix: prefix.clone(),
                key: ResourceKey {
                    group_code: 0,
                    icon_id: raw_key.block_index,
                    block_index: raw_key.block_index,
                },
                raw_key: Some(raw_key),
                canonical_block,
                assembled: false,
            })
            .collect())
    }

    fn unlinked_indexed_assets(
        &mut self,
        prefix: &str,
    ) -> Result<Vec<VerifiedAssetRef>, ViewerSessionError> {
        let prefix = prefix.to_ascii_lowercase();
        let resource_directory = self
            .resource_directory
            .clone()
            .ok_or(ViewerSessionError::ResourceDirectoryNotSelected)?;
        if !resolve_archive_directory(&resource_directory, &prefix)
            .join(format!("{prefix}000000.bin"))
            .is_file()
        {
            return Ok(Vec::new());
        }
        let _ = self.active_text_catalog();
        let records = self.archive(&prefix)?.records().to_vec();
        let mut unique = BTreeMap::<u32, ResourceKey>::new();
        for record in records {
            let archive = self.archive(&prefix)?;
            let canonical_block = archive
                .assembly_canonical_block(record.block_index)
                .unwrap_or(record.block_index);
            let assembled = archive.is_assembled(record.block_index);
            let linked = self.active_text_catalog().ok().is_some_and(|catalog| {
                !catalog
                    .links_for_image(&prefix, record.group_code, record.icon_id)
                    .is_empty()
            });
            if linked || assembled {
                continue;
            }
            unique.entry(canonical_block).or_insert(ResourceKey {
                group_code: record.group_code,
                icon_id: record.icon_id,
                block_index: record.block_index,
            });
        }
        Ok(unique
            .into_iter()
            .map(|(canonical_block, key)| VerifiedAssetRef {
                prefix: prefix.clone(),
                key,
                raw_key: None,
                canonical_block,
                assembled: false,
            })
            .collect())
    }

    fn assembled_assets(&mut self) -> Result<Vec<VerifiedAssetRef>, ViewerSessionError> {
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
                let canonical_block = archive
                    .assembly_canonical_block(record.block_index)
                    .unwrap_or(record.block_index);
                if !archive.is_assembled(record.block_index) {
                    continue;
                }
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
                        assembled: true,
                    }),
            );
        }

        for definition in RAW_IMAGE_ARCHIVES {
            if !raw_archive_path(&resource_directory, definition).is_some_and(|path| path.is_file())
            {
                continue;
            }
            let archive = self.raw_archive(definition.prefix)?;
            let mut matching = BTreeMap::<u32, (RawResourceKey, bool)>::new();
            for record in archive.records() {
                let assembled = raw_layered_rule(definition.prefix, record.key).is_some();
                if assembled {
                    let canonical_block = raw_canonical_block(definition.prefix, record.key);
                    matching
                        .entry(canonical_block)
                        .or_insert((record.key, assembled));
                }
            }
            assets.extend(
                matching
                    .into_iter()
                    .map(|(canonical_block, (raw_key, assembled))| VerifiedAssetRef {
                        prefix: definition.prefix.to_owned(),
                        key: ResourceKey {
                            group_code: 0,
                            icon_id: raw_key.block_index,
                            block_index: raw_key.block_index,
                        },
                        raw_key: Some(raw_key),
                        canonical_block,
                        assembled,
                    }),
            );
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
        let mut assets = self
            .verified_search_assets()?
            .iter()
            .filter(|asset| search_asset_matches(asset, &terms))
            .cloned()
            .collect::<Vec<_>>();
        let normalized_query = query.to_lowercase();
        assets.sort_by(|left, right| {
            search_asset_rank(left, &normalized_query)
                .cmp(&search_asset_rank(right, &normalized_query))
                .then_with(|| left.path.cmp(&right.path))
                .then_with(|| left.asset.prefix.cmp(&right.asset.prefix))
                .then_with(|| left.asset.canonical_block.cmp(&right.asset.canonical_block))
        });
        Ok((query.to_owned(), assets))
    }

    fn verified_search_assets(&mut self) -> Result<&[VerifiedSearchAssetRef], ViewerSessionError> {
        if self.search_assets.is_none() {
            let records = self
                .active_text_catalog()
                .map(TextCatalog::linked_images)
                .unwrap_or_default();
            let mut unique =
                BTreeMap::<(Vec<String>, AssetIdentityKey), VerifiedSearchAssetRef>::new();
            for record in records {
                let prefix = record.image.archive.to_ascii_lowercase();
                let archive = self.archive(&prefix)?;
                let canonical_block = archive
                    .assembly_canonical_block(record.image.block_index)
                    .unwrap_or(record.image.block_index);
                let assembled = archive.is_assembled(record.image.block_index);
                let path =
                    linked_category_path(&record.source_label, &prefix, record.image.group_code);
                let search_text = format!(
                    "{} {} {} {} {} {}",
                    record.source,
                    record.source_label,
                    record.id,
                    record.fields.join(" "),
                    record.image.relation,
                    prefix
                );
                let search_names = record
                    .fields
                    .iter()
                    .find(|field| !field.is_empty())
                    .cloned()
                    .into_iter()
                    .collect::<Vec<_>>();
                let asset = VerifiedAssetRef {
                    prefix: prefix.clone(),
                    key: ResourceKey {
                        group_code: record.image.group_code,
                        icon_id: record.image.icon_id,
                        block_index: record.image.block_index,
                    },
                    raw_key: None,
                    canonical_block,
                    assembled,
                };
                unique
                    .entry((path.clone(), asset_identity_key(&asset)))
                    .and_modify(|existing| {
                        existing.search_text.push(' ');
                        existing.search_text.push_str(&search_text);
                        existing.search_names.extend(search_names.clone());
                    })
                    .or_insert(VerifiedSearchAssetRef {
                        path,
                        asset,
                        search_text,
                        search_names,
                    });
            }
            for asset in self.assembled_assets()? {
                let path = physical_category_path(
                    None,
                    &asset.prefix,
                    asset.key.group_code,
                    true,
                    asset.raw_key.is_none(),
                );
                let search_text = format!("{} {}", path.join(" "), asset.prefix);
                unique
                    .entry((path.clone(), asset_identity_key(&asset)))
                    .or_insert(VerifiedSearchAssetRef {
                        path,
                        search_text,
                        search_names: Vec::new(),
                        asset,
                    });
            }
            for prefix in SUPPORTED_ARCHIVE_PREFIXES {
                if prefix == "gm"
                    && gm_archive_path(
                        self.resource_directory
                            .as_deref()
                            .ok_or(ViewerSessionError::ResourceDirectoryNotSelected)?,
                    )
                    .is_file()
                {
                    let atlas_records = self.gm_archive()?.atlas_records().collect::<Vec<_>>();
                    for record in atlas_records {
                        let path = gm_atlas_category_path();
                        let asset = VerifiedAssetRef {
                            prefix: "gm".to_owned(),
                            key: ResourceKey {
                                group_code: GM_ATLAS_IDENTITY_GROUP,
                                icon_id: record.atlas_block_index,
                                block_index: record.key.block_index,
                            },
                            raw_key: Some(record.key),
                            canonical_block: record.key.block_index,
                            assembled: false,
                        };
                        unique.insert(
                            (path.clone(), asset_identity_key(&asset)),
                            VerifiedSearchAssetRef {
                                search_text: format!(
                                    "{} gm atlas {}",
                                    path.join(" "),
                                    record.atlas_block_index
                                ),
                                path,
                                search_names: Vec::new(),
                                asset,
                            },
                        );
                    }
                    let sprite_records = self.gm_archive()?.records().collect::<Vec<_>>();
                    for record in sprite_records {
                        let path = gm_sprite_category_path(record.atlas_block_index);
                        let asset = VerifiedAssetRef {
                            prefix: "gm".to_owned(),
                            key: ResourceKey {
                                group_code: record.atlas_block_index,
                                icon_id: record.sprite_index,
                                block_index: record.key.block_index,
                            },
                            raw_key: Some(record.key),
                            canonical_block: record.key.block_index,
                            assembled: false,
                        };
                        unique.insert(
                            (path.clone(), asset_identity_key(&asset)),
                            VerifiedSearchAssetRef {
                                search_text: format!(
                                    "{} gm sprite {} atlas {}",
                                    path.join(" "),
                                    record.sprite_index,
                                    record.atlas_block_index
                                ),
                                path,
                                search_names: Vec::new(),
                                asset,
                            },
                        );
                    }
                    continue;
                }
                for asset in self.unlinked_assets(prefix)? {
                    let source_label = self
                        .active_text_catalog()
                        .ok()
                        .and_then(|catalog| {
                            catalog
                                .source_label_for_image_group(&asset.prefix, asset.key.group_code)
                        })
                        .map(str::to_owned);
                    let path = physical_category_path(
                        source_label.as_deref(),
                        &asset.prefix,
                        asset.key.group_code,
                        false,
                        asset.raw_key.is_none(),
                    );
                    let search_text =
                        format!("{} {} {}", path.join(" "), prefix, asset.key.icon_id);
                    unique
                        .entry((path.clone(), asset_identity_key(&asset)))
                        .or_insert(VerifiedSearchAssetRef {
                            path,
                            search_text,
                            search_names: Vec::new(),
                            asset,
                        });
                }
            }
            let mut assets = unique.into_values().collect::<Vec<_>>();
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
            let archive = LoadedRawImageArchive::open_files(
                archive_directory,
                &normalized,
                definition.file_numbers,
                definition.layout,
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

    fn gm_archive(&mut self) -> Result<&LoadedGmAtlasArchive, ViewerSessionError> {
        let resource_directory = self
            .resource_directory
            .clone()
            .ok_or(ViewerSessionError::ResourceDirectoryNotSelected)?;
        if self.gm_archive.is_none() {
            let archive = LoadedGmAtlasArchive::open_files(
                resolve_archive_directory(&resource_directory, "gm"),
                GM_ATLAS_FILE_NUMBERS,
                MAX_IMAGE_DECODE_SIZE,
            )
            .map_err(|source| ViewerSessionError::OpenArchive {
                prefix: "gm".to_owned(),
                source,
            })?;
            self.gm_archive = Some(archive);
        }
        Ok(self.gm_archive.as_ref().expect("GM archive initialized"))
    }

    fn standalone_archive(
        &mut self,
        prefix: &str,
    ) -> Result<&LoadedStandaloneImageArchive, ViewerSessionError> {
        let normalized = prefix.to_ascii_lowercase();
        let resource_directory = self
            .resource_directory
            .clone()
            .ok_or(ViewerSessionError::ResourceDirectoryNotSelected)?;
        let path = standalone_image_path(&resource_directory, &normalized).ok_or_else(|| {
            ViewerSessionError::RawArchiveDefinitionMissing {
                prefix: normalized.clone(),
            }
        })?;
        if !self.standalone_archives.contains_key(&normalized) {
            let archive = match normalized.as_str() {
                "cu" => LoadedStandaloneImageArchive::open_cursor(path, MAX_IMAGE_DECODE_SIZE),
                "ft" => LoadedStandaloneImageArchive::open_font(path, MAX_IMAGE_DECODE_SIZE),
                "wm" => LoadedStandaloneImageArchive::open_xftx(path),
                _ => unreachable!("standalone prefix was validated"),
            }
            .map_err(|source| ViewerSessionError::OpenArchive {
                prefix: normalized.clone(),
                source,
            })?;
            self.standalone_archives.insert(normalized.clone(), archive);
        }
        Ok(self
            .standalone_archives
            .get(&normalized)
            .expect("standalone archive initialized"))
    }
}

fn search_asset_matches(asset: &VerifiedSearchAssetRef, terms: &[String]) -> bool {
    let text = format!(
        "{} {} {}",
        asset.path.join(" "),
        asset.asset.prefix,
        asset.search_text
    )
    .to_lowercase();
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

fn search_asset_rank(asset: &VerifiedSearchAssetRef, query: &str) -> u8 {
    let names = asset
        .search_names
        .iter()
        .map(|name| name.to_lowercase())
        .collect::<Vec<_>>();
    if names.iter().any(|name| name == query) {
        0
    } else if names.iter().any(|name| name.starts_with(query)) {
        1
    } else if names.iter().any(|name| name.contains(query)) {
        2
    } else {
        3
    }
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
    let mut verified_assets = BTreeMap::<Vec<String>, BTreeSet<AssetIdentityKey>>::new();
    let mut indexed_assembly_layouts = BTreeMap::<String, IndexedAssemblyLayout>::new();
    let text_catalog = TextCatalog::load(game_directory).ok();
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
        let assemblies = resolve_indexed_assembly_layout(prefix, &index.records);
        for record in &index.records {
            let canonical_block = assemblies
                .canonical_block(record.block_index)
                .unwrap_or(record.block_index);
            let assembled = assemblies.is_assembled(record.block_index);
            let source_label = text_catalog.as_ref().and_then(|catalog| {
                catalog.source_label_for_image_group(prefix, record.group_code)
            });
            let category_path =
                physical_category_path(source_label, prefix, record.group_code, assembled, true);
            verified_assets
                .entry(category_path)
                .or_default()
                .insert(if assembled {
                    (prefix.to_owned(), 0, canonical_block, 0)
                } else {
                    (prefix.to_owned(), 2, record.group_code, record.icon_id)
                });
        }
        archives.push(ArchiveSummary {
            prefix: prefix.to_owned(),
            has_index: true,
            record_count: header.record_count,
            group_count: header.group_count,
            image_block_count: header.image_block_count,
            archive_count: header.archive_count,
        });
        indexed_assembly_layouts.insert(prefix.to_owned(), assemblies);
    }

    for definition in RAW_IMAGE_ARCHIVES {
        let directory = resolve_archive_directory(&resource_directory, definition.prefix);
        if !raw_archive_path(&resource_directory, definition).is_some_and(|path| path.is_file()) {
            continue;
        }
        let archive = LoadedRawImageArchive::open_files(
            directory,
            definition.prefix,
            definition.file_numbers,
            definition.layout,
            definition.spec,
        )
        .map_err(|source| GameDirectoryError::OpenArchive {
            prefix: definition.prefix.to_owned(),
            source,
        })?;
        let records = archive.records().collect::<Vec<_>>();
        for record in &records {
            let canonical_block = raw_canonical_block(definition.prefix, record.key);
            let assembled = raw_layered_rule(definition.prefix, record.key).is_some();
            let category_path =
                physical_category_path(None, definition.prefix, 0, assembled, false);
            verified_assets
                .entry(category_path)
                .or_default()
                .insert(if assembled {
                    (definition.prefix.to_owned(), 0, canonical_block, 0)
                } else {
                    (
                        definition.prefix.to_owned(),
                        1,
                        record.key.file_number,
                        record.key.file_block_index,
                    )
                });
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
    if gm_archive_path(&resource_directory).is_file() {
        let archive = LoadedGmAtlasArchive::open_files(
            resolve_archive_directory(&resource_directory, "gm"),
            GM_ATLAS_FILE_NUMBERS,
            MAX_IMAGE_DECODE_SIZE,
        )
        .map_err(|source| GameDirectoryError::OpenArchive {
            prefix: "gm".to_owned(),
            source,
        })?;
        let atlas_records = archive.atlas_records().collect::<Vec<_>>();
        for record in &atlas_records {
            verified_assets
                .entry(gm_atlas_category_path())
                .or_default()
                .insert((
                    "gm".to_owned(),
                    3,
                    record.key.file_number,
                    record.atlas_block_index,
                ));
        }
        let sprite_records = archive.records().collect::<Vec<_>>();
        for record in &sprite_records {
            verified_assets
                .entry(gm_sprite_category_path(record.atlas_block_index))
                .or_default()
                .insert((
                    "gm".to_owned(),
                    4,
                    record.atlas_block_index,
                    record.sprite_index,
                ));
        }
        archives.push(ArchiveSummary {
            prefix: "gm".to_owned(),
            has_index: false,
            record_count: u32::try_from(atlas_records.len().saturating_add(sprite_records.len()))
                .unwrap_or(u32::MAX),
            group_count: 0,
            image_block_count: u32::try_from(
                atlas_records.len().saturating_add(sprite_records.len()),
            )
            .unwrap_or(u32::MAX),
            archive_count: archive.archive_count(),
        });
    }
    for prefix in ["cu", "ft", "wm"] {
        let Some(path) = standalone_image_path(&resource_directory, prefix) else {
            continue;
        };
        if !path.is_file() {
            continue;
        }
        let archive = match prefix {
            "cu" => LoadedStandaloneImageArchive::open_cursor(&path, MAX_IMAGE_DECODE_SIZE),
            "ft" => LoadedStandaloneImageArchive::open_font(&path, MAX_IMAGE_DECODE_SIZE),
            "wm" => LoadedStandaloneImageArchive::open_xftx(&path),
            _ => unreachable!(),
        }
        .map_err(|source| GameDirectoryError::OpenArchive {
            prefix: prefix.to_owned(),
            source,
        })?;
        let records = archive.records().collect::<Vec<_>>();
        let category_path = physical_category_path(None, prefix, 0, false, false);
        for record in &records {
            verified_assets
                .entry(category_path.clone())
                .or_default()
                .insert((
                    prefix.to_owned(),
                    1,
                    record.key.file_number,
                    record.key.file_block_index,
                ));
        }
        archives.push(ArchiveSummary {
            prefix: prefix.to_owned(),
            has_index: false,
            record_count: u32::try_from(records.len()).unwrap_or(u32::MAX),
            group_count: 0,
            image_block_count: u32::try_from(records.len()).unwrap_or(u32::MAX),
            archive_count: 1,
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

    if let Some(catalog) = text_catalog.as_ref() {
        for record in catalog.linked_images() {
            let prefix = record.image.archive.to_ascii_lowercase();
            let canonical_block = indexed_assembly_layouts
                .get(&prefix)
                .and_then(|layout| layout.canonical_block(record.image.block_index))
                .unwrap_or(record.image.block_index);
            let assembled = indexed_assembly_layouts
                .get(&prefix)
                .is_some_and(|layout| layout.is_assembled(record.image.block_index));
            verified_assets
                .entry(linked_category_path(
                    &record.source_label,
                    &prefix,
                    record.image.group_code,
                ))
                .or_default()
                .insert(if assembled {
                    (prefix, 0, canonical_block, 0)
                } else {
                    (prefix, 2, record.image.group_code, record.image.icon_id)
                });
        }
    }

    let catalog_diagnostics = catalog_diagnostics(&verified_assets);
    Ok(GameDirectorySummary {
        game_directory: game_directory.to_string_lossy().into_owned(),
        resource_directory: resource_directory.to_string_lossy().into_owned(),
        archives,
        verified_categories: verified_assets
            .into_iter()
            .map(|(path, assets)| VerifiedCategorySummary {
                path,
                asset_count: assets.len(),
            })
            .collect(),
        catalog_diagnostics,
    })
}

fn catalog_diagnostics(
    categories: &BTreeMap<Vec<String>, BTreeSet<AssetIdentityKey>>,
) -> CatalogDiagnosticsSummary {
    let mut asset_paths = BTreeMap::<AssetIdentityKey, BTreeSet<Vec<String>>>::new();
    let unclassified_categories = categories
        .iter()
        .filter(|(path, _)| path.first().is_some_and(|segment| segment == "미분류"))
        .map(|(path, assets)| VerifiedCategorySummary {
            path: path.clone(),
            asset_count: assets.len(),
        })
        .collect::<Vec<_>>();
    for (path, assets) in categories {
        for asset in assets {
            asset_paths
                .entry(asset.clone())
                .or_default()
                .insert(path.clone());
        }
    }

    let categorized_asset_count = asset_paths
        .values()
        .filter(|paths| {
            paths.iter().any(|path| {
                path.first()
                    .is_none_or(|segment| segment.as_str() != "미분류")
            })
        })
        .count();
    let unclassified_asset_count = asset_paths.len().saturating_sub(categorized_asset_count);
    let multiple_category_asset_count =
        asset_paths.values().filter(|paths| paths.len() > 1).count();
    let multiple_category_assets = asset_paths
        .iter()
        .filter(|(_, paths)| paths.len() > 1)
        .take(100)
        .map(
            |((archive, kind, primary_id, secondary_id), paths)| CatalogMultipleCategoryAsset {
                archive: archive.clone(),
                identity_kind: match kind {
                    0 => "조립 이미지",
                    1 => "원시 블록",
                    3 => "GM 원본 아틀라스",
                    4 => "GM 스프라이트",
                    _ => "인덱스 이미지",
                }
                .to_owned(),
                primary_id: *primary_id,
                secondary_id: *secondary_id,
                category_paths: paths.iter().cloned().collect(),
            },
        )
        .collect();

    CatalogDiagnosticsSummary {
        total_asset_count: asset_paths.len(),
        categorized_asset_count,
        unclassified_asset_count,
        multiple_category_asset_count,
        unclassified_categories,
        multiple_category_assets,
    }
}

#[derive(Debug)]
pub enum ViewerSessionError {
    ResourceDirectoryNotSelected,
    GameDirectoryNotFound,
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
    OpenTextCatalog(TextCatalogError),
    TextPage(TextCatalogPageError),
    TextImageNotFound {
        prefix: String,
        group_code: u32,
        icon_id: u32,
    },
}

impl fmt::Display for ViewerSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResourceDirectoryNotSelected => {
                write!(formatter, "먼저 게임 폴더를 선택해 주세요.")
            }
            Self::GameDirectoryNotFound => {
                write!(
                    formatter,
                    "선택한 리소스 폴더에서 게임 폴더를 확인하지 못했습니다."
                )
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
            Self::OpenTextCatalog(source) => {
                write!(formatter, "텍스트 자료를 열지 못했습니다: {source}")
            }
            Self::TextPage(source) => {
                write!(formatter, "텍스트 자료를 불러오지 못했습니다: {source}")
            }
            Self::TextImageNotFound {
                prefix,
                group_code,
                icon_id,
            } => write!(
                formatter,
                "연결된 이미지를 찾지 못했습니다: {prefix} 그룹 {group_code}, ID {icon_id}"
            ),
        }
    }
}

impl Error for ViewerSessionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::OpenArchive { source, .. } | Self::Extract { source, .. } => Some(source),
            Self::OpenTextCatalog(source) => Some(source),
            Self::TextPage(source) => Some(source),
            Self::ResourceDirectoryNotSelected
            | Self::GameDirectoryNotFound
            | Self::EmptyCategoryPath
            | Self::EmptySearchQuery
            | Self::InvalidPageSize { .. }
            | Self::RawArchiveDefinitionMissing { .. }
            | Self::CategoryNotFound { .. }
            | Self::AssetNotFound { .. }
            | Self::OffsetOutOfRange { .. }
            | Self::AssemblyRuleMissing { .. } => None,
            Self::TextImageNotFound { .. } => None,
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
                "지원하는 이미지 리소스(im, kp, sa, sb, sc, sd, se, sf, sg, sh, tm, sw, sx, sy, sz, is)를 찾지 못했습니다: {}",
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

    #[test]
    fn maps_user_verified_logical_groups_without_block_ranges() {
        for (archive, group_code, expected) in [
            ("sa", 2, "선박 그레이드 보너스"),
            ("sb", 25, "선박데코"),
            ("sb", 26, "선원장비"),
            ("sc", 4, "돛 무늬"),
            ("sc", 5, "주점 메뉴"),
            ("sc", 6, "포커"),
            ("sc", 7, "이벤트"),
            ("sc", 8, "부관"),
            ("sc", 10, "아팔타멘토 타입"),
            ("sc", 14, "개인농장 시설"),
            ("sc", 16, "테크닉"),
            ("sc", 20, "대학·학술협회"),
            ("sc", 26, "트레져헌트 테마"),
            ("sc", 31, "전승(획득)"),
            ("sc", 32, "트레져헌트 렐릭"),
            ("sc", 33, "레거시 테마"),
            ("sc", 34, "추구 생산"),
            ("sc", 35, "위인의 장 테마"),
            ("sc", 36, "잠재능력"),
            ("sd", 4, "입항허가"),
            ("sd", 29, "전승(획득/큰이미지)"),
            ("sf", 1, "레거시"),
            ("sy", 0, "역사적 사건"),
        ] {
            assert_eq!(
                user_verified_group_category_path(archive, group_code),
                Some(vec![expected.to_owned()])
            );
        }
        assert_eq!(user_verified_group_category_path("sc", 3), None);
    }

    #[test]
    fn reports_unclassified_and_multiple_category_assets_from_reverse_index() {
        let categorized = ("sa".to_owned(), 2, 0, 1);
        let unclassified = ("sb".to_owned(), 2, 21, 42);
        let multiple = ("sc".to_owned(), 2, 16, 7);
        let mut categories = BTreeMap::<Vec<String>, BTreeSet<AssetIdentityKey>>::new();
        categories.insert(
            vec!["스킬".to_owned()],
            BTreeSet::from([categorized, multiple.clone()]),
        );
        categories.insert(vec!["테크닉".to_owned()], BTreeSet::from([multiple]));
        categories.insert(
            vec!["미분류".to_owned(), "SB".to_owned(), "그룹 21".to_owned()],
            BTreeSet::from([unclassified]),
        );

        let diagnostics = catalog_diagnostics(&categories);
        assert_eq!(diagnostics.total_asset_count, 3);
        assert_eq!(diagnostics.categorized_asset_count, 2);
        assert_eq!(diagnostics.unclassified_asset_count, 1);
        assert_eq!(diagnostics.multiple_category_asset_count, 1);
        assert_eq!(diagnostics.unclassified_categories[0].asset_count, 1);
        assert_eq!(diagnostics.multiple_category_assets.len(), 1);
        assert_eq!(
            diagnostics.multiple_category_assets[0].category_paths,
            [vec!["스킬".to_owned()], vec!["테크닉".to_owned()]]
        );
    }

    #[test]
    fn maps_only_world_clock_body_sources_to_the_composite_asset() {
        for block_index in [8_815, 8_818, 8_826, 8_829] {
            assert_eq!(indexed_canonical_block("sd", block_index), 8_815);
            assert!(indexed_assembled("SD", block_index));
        }
        for block_index in [8_814, 8_819, 8_825, 8_830] {
            assert_eq!(indexed_canonical_block("sd", block_index), block_index);
            assert!(!indexed_assembled("sd", block_index));
        }
    }

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
        ] {
            push_u32(&mut bytes, value);
        }
        for record in records {
            for value in [record[4], record[0], record[1], record[2], record[3]] {
                push_u32(&mut bytes, value);
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

    fn write_inline_data_file(path: &Path, raw_blocks: &[Vec<u8>]) {
        let blocks = raw_blocks
            .iter()
            .map(|raw| zlib_block(raw))
            .collect::<Vec<_>>();
        let mut offset = 4 + blocks.len() * 8;
        let mut data = Vec::new();
        push_u32(&mut data, blocks.len() as u32);
        for block in &blocks {
            push_u32(&mut data, offset as u32);
            push_u32(&mut data, block.len() as u32);
            offset += block.len();
        }
        for block in blocks {
            data.extend_from_slice(&block);
        }
        fs::write(path, data).expect("write inline test data file");
    }

    fn write_repeated_inline_data_file(path: &Path, raw: &[u8], count: usize) {
        let block = zlib_block(raw);
        let mut offset = 4 + count * 8;
        let mut data = Vec::with_capacity(offset + block.len() * count);
        push_u32(&mut data, count as u32);
        for _ in 0..count {
            push_u32(&mut data, offset as u32);
            push_u32(&mut data, block.len() as u32);
            offset += block.len();
        }
        for _ in 0..count {
            data.extend_from_slice(&block);
        }
        fs::write(path, data).expect("write repeated inline test data file");
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
            text_links: Vec::new(),
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
        assert_eq!(summary.verified_categories.len(), 12);
        assert_eq!(
            summary
                .verified_categories
                .iter()
                .map(|category| category.asset_count)
                .sum::<usize>(),
            13
        );
        for expected in [
            ["미분류", "SA", "그룹 0"].as_slice(),
            ["미분류", "SB", "그룹 10"].as_slice(),
            ["레거시"].as_slice(),
            ["미분류", "IS", "그룹 20"].as_slice(),
        ] {
            assert!(
                summary
                    .verified_categories
                    .iter()
                    .any(|category| category.path == expected)
            );
        }
        assert!(!summary.verified_categories.iter().any(|category| {
            matches!(
                category.path.first().map(String::as_str),
                Some("UI 이미지" | "이벤트" | "인물" | "클라이언트")
            )
        }));
    }

    #[test]
    fn loads_uncategorized_thumbnails_from_the_secondary_resource_directory() {
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
        let category = ["미분류", "SW", "그룹 0"].map(str::to_owned);
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
    fn summarizes_every_logical_asset_without_legacy_block_categories() {
        let directory = TestDirectory::new();
        let resources = directory.prepare_game();
        write_index_records(
            &resources.join("sb000000.bin"),
            &[
                [100_100, 0, 48, 48, 1],
                [100_101, 0, 48, 48, 1],
                [100_102, 1, 48, 48, 1],
                [1_200_002, 2, 48, 48, 16],
            ],
            3,
        );
        let sd_records = (0..28)
            .map(|offset| [offset + 1, 10_368 + offset, 128, 128, 33])
            .collect::<Vec<_>>();
        write_index_records(&resources.join("sd000000.bin"), &sd_records, 10_396);

        let summary = inspect_game_directory(&directory.0).expect("inspect categorized game");
        let sb_group_one = summary
            .verified_categories
            .iter()
            .find(|category| category.path == ["미분류", "SB", "그룹 1"])
            .expect("unclassified SB group 1");
        let assembled = summary
            .verified_categories
            .iter()
            .find(|category| category.path == ["조립 이미지", "SD"])
            .expect("generic assembled image category");

        assert_eq!(sb_group_one.asset_count, 3);
        assert_eq!(assembled.asset_count, 1);
        assert_eq!(
            summary
                .verified_categories
                .iter()
                .map(|category| category.asset_count)
                .sum::<usize>(),
            5
        );
        assert!(!summary.verified_categories.iter().any(|category| {
            category.path == ["장비", "머리"] || category.path == ["UI 이미지", "예지의 서", "표지"]
        }));
    }

    #[test]
    fn pages_group_fallback_thumbnails_for_each_logical_icon_id() {
        let directory = TestDirectory::new();
        let resources = directory.prepare_game();
        write_index_records(
            &resources.join("sb000000.bin"),
            &[
                [100_100, 0, 2, 1, 1],
                [100_101, 0, 2, 1, 1],
                [100_102, 1, 1, 2, 1],
                [1_200_002, 2, 1, 1, 16],
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
        let category = ["미분류", "SB", "그룹 1"].map(str::to_owned);

        let first = session
            .category_page(&category, 0, 1)
            .expect("load first thumbnail page");
        let second = session
            .category_page(&category, 1, 1)
            .expect("load second thumbnail page");
        let third = session
            .category_page(&category, 2, 1)
            .expect("load third thumbnail page");

        assert_eq!(first.total_count, 3);
        assert_eq!(first.items.len(), 1);
        assert_eq!(first.items[0].block_index, 0);
        assert_eq!(first.items[0].icon_id, Some(100_100));
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
        assert_eq!(second.items[0].block_index, 0);
        assert_eq!(second.items[0].icon_id, Some(100_101));
        assert_eq!(third.items[0].block_index, 1);
        assert_eq!(third.items[0].icon_id, Some(100_102));

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
        assert_eq!(category_assets.len(), 3);
        assert_eq!(category_assets[0].archive(), "sb");
        assert_eq!(category_assets[0].icon_id(), Some(100_100));
        assert_eq!(category_assets[0].block_index(), 0);
        assert!(!category_assets[0].assembled());
        assert_eq!(category_assets[1].icon_id(), Some(100_101));
        assert_eq!(category_assets[1].block_index(), 0);
        assert_eq!(category_assets[2].icon_id(), Some(100_102));
        assert_eq!(category_assets[2].block_index(), 1);
        let category_png = session
            .category_asset_png(&category_assets[2])
            .expect("extract category asset directly");
        assert_eq!(category_png.block_index, 1);
        assert_eq!(&category_png.png[..8], b"\x89PNG\r\n\x1a\n");

        let selected = session
            .selected_asset(&category, "SB", 1)
            .expect("revalidate selected asset");
        assert_eq!(selected.path(), &category);
        assert_eq!(selected.archive(), "sb");
        assert_eq!(selected.block_index(), 1);
        let selected_png = session
            .search_asset_png(&selected)
            .expect("extract selected asset directly");
        assert_eq!(selected_png.block_index, 1);

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

        let error = session.category_page(&category, 3, 1).unwrap_err();
        assert!(matches!(
            error,
            ViewerSessionError::OffsetOutOfRange {
                offset: 3,
                total_count: 3,
            }
        ));
    }

    #[test]
    fn searches_all_assets_and_pages_thumbnails() {
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
            .search_page("SB", 0, 1)
            .expect("search verified category");
        assert_eq!(category_page.total_count, 3);
        assert_eq!(category_page.items.len(), 1);
        assert_eq!(category_page.items[0].path, ["미분류", "SB", "그룹 1"]);
        assert_eq!(category_page.items[0].thumbnail.icon_id, Some(100_100));
        assert!(session.search_assets.is_some());

        let second_page = session
            .search_page("SB", 1, 1)
            .expect("reuse the search index for the next page");
        assert_eq!(second_page.items[0].thumbnail.icon_id, Some(100_101));

        let id_page = session
            .search_page("100100", 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("search exact icon ID");
        assert_eq!(id_page.total_count, 1);
        assert_eq!(id_page.items[0].thumbnail.block_index, 0);

        let search_assets = session
            .search_assets("SB")
            .expect("list matching search assets without thumbnails");
        assert_eq!(search_assets.len(), 3);
        assert_eq!(search_assets[0].path(), ["미분류", "SB", "그룹 1"]);
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
        assert_eq!(update_page.review_required_count, 0);
        assert_eq!(update_page.total_count, 3);
        assert_eq!(update_page.items.len(), 1);
        assert_eq!(update_page.items[0].path, ["미분류", "SB", "그룹 1"]);
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
        let offset_error = session.search_page("SB", 3, 1).unwrap_err();
        assert!(matches!(
            offset_error,
            ViewerSessionError::OffsetOutOfRange {
                offset: 3,
                total_count: 3,
            }
        ));

        session.set_resource_directory(resources.join("other"));
        assert!(session.search_assets.is_none());
    }

    #[test]
    fn displays_and_extracts_uncategorized_sh_raw_blocks_without_an_icon_id() {
        let directory = TestDirectory::new();
        let resources = directory.prepare_game();
        write_data_file(
            &resources.join("sh000001.bin"),
            &[vec![0x11; 256 * 256], vec![0xcc; 256 * 256]],
        );
        let mut session = ViewerSession::default();
        session.set_resource_directory(&resources);
        let category = ["미분류", "SH"].map(str::to_owned);

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
            .search_page("SH 1", 0, VIEWER_CATEGORY_PAGE_SIZE)
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
        assert_eq!(summary.verified_categories[0].path, ["미분류", "SH"]);
        assert_eq!(summary.verified_categories[0].asset_count, 2);
    }

    #[test]
    fn displays_extracts_and_summarizes_tm_inline_minimaps() {
        let directory = TestDirectory::new();
        let primary = directory.prepare_game();
        let resource_root = primary.parent().expect("resource root");
        let inline_directory = resource_root.join("0000");
        fs::create_dir(&inline_directory).expect("create inline resource directory");
        write_inline_data_file(
            &inline_directory.join("tm000000.bin"),
            &[vec![0x11; 180 * 140 * 4], vec![0xcc; 180 * 141 * 4]],
        );
        let mut session = ViewerSession::default();
        session.set_resource_directory(resource_root);
        let category = ["지도", "도시 미니맵"].map(str::to_owned);

        let page = session
            .category_page(&category, 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("load TM minimap page");
        assert_eq!(page.total_count, 2);
        assert_eq!(page.items[0].archive, "tm");
        assert_eq!(page.items[0].icon_id, None);
        assert_eq!(
            (page.items[0].source_width, page.items[0].source_height),
            (180, 140)
        );
        assert_eq!(
            (page.items[1].source_width, page.items[1].source_height),
            (180, 141)
        );

        let detail = session
            .asset_detail(&category, "TM", 1)
            .expect("load TM minimap detail");
        assert_eq!(detail.icon_id, None);
        assert_eq!((detail.source_width, detail.source_height), (180, 141));
        let png = session
            .asset_png(&category, "tm", 0)
            .expect("extract TM minimap PNG");
        assert_eq!((png.width, png.height), (180, 140));
        assert_eq!(&png.png[..8], b"\x89PNG\r\n\x1a\n");

        let search = session
            .search_page("TM 1", 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("search TM minimap");
        assert_eq!(search.total_count, 1);
        assert_eq!(search.items[0].thumbnail.block_index, 1);

        let added = AssetSnapshotEntry::new_raw(
            "tm",
            RawResourceKey {
                block_index: 1,
                file_number: 0,
                file_block_index: 1,
            },
            180,
            141,
        );
        let update = session
            .update_page(&[added], 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("show added TM minimap");
        assert_eq!(update.total_count, 1);
        assert_eq!(update.review_required_count, 0);

        let summary = inspect_game_directory(&directory.0).expect("inspect TM-only game fixture");
        assert_eq!(summary.archives.len(), 1);
        assert_eq!(summary.archives[0].prefix, "tm");
        assert!(!summary.archives[0].has_index);
        assert_eq!(summary.archives[0].record_count, 2);
        assert_eq!(summary.archives[0].group_count, 0);
        assert_eq!(summary.archives[0].image_block_count, 2);
        assert_eq!(summary.archives[0].archive_count, 1);
        assert_eq!(summary.verified_categories.len(), 1);
        assert_eq!(summary.verified_categories[0].path, category);
        assert_eq!(summary.verified_categories[0].asset_count, 2);
    }

    #[test]
    fn displays_extracts_and_snapshots_gm_atlases_and_sprite_records() {
        let directory = TestDirectory::new();
        let primary = directory.prepare_game();
        let resource_root = primary.parent().expect("resource root");
        let gm_directory = resource_root.join("local");
        fs::create_dir(&gm_directory).expect("create GM resource directory");
        for file_number in GM_ATLAS_FILE_NUMBERS {
            let mut decoded = Vec::new();
            decoded.extend_from_slice(&1_u32.to_le_bytes());
            decoded.extend_from_slice(&1_u32.to_le_bytes());
            decoded.extend_from_slice(&40_u32.to_le_bytes());
            decoded.extend_from_slice(&0_u32.to_le_bytes());
            decoded.extend_from_slice(&0.5_f32.to_le_bytes());
            decoded.extend_from_slice(&0.5_f32.to_le_bytes());
            decoded.extend_from_slice(&1.0_f32.to_le_bytes());
            decoded.extend_from_slice(&1.0_f32.to_le_bytes());
            decoded.extend_from_slice(&1_u32.to_le_bytes());
            decoded.extend_from_slice(&1_u32.to_le_bytes());
            decoded.extend_from_slice(&1_u16.to_le_bytes());
            decoded.extend_from_slice(&1_u16.to_le_bytes());
            decoded.extend_from_slice(&21_u32.to_le_bytes());
            decoded.extend_from_slice(file_number.to_le_bytes().as_slice());
            decoded.extend_from_slice(&4_u32.to_le_bytes());
            decoded.extend_from_slice(&4_u32.to_le_bytes());
            decoded.extend_from_slice(&[0x11, 0x22, 0x33, 0xff]);
            fs::write(
                gm_directory.join(format!("gm{file_number:06}.bin")),
                zlib_block(&decoded),
            )
            .expect("write GM fixture");
        }

        let summary = inspect_game_directory(&directory.0).expect("inspect GM fixture");
        let gm = summary
            .archives
            .iter()
            .find(|archive| archive.prefix == "gm")
            .expect("GM archive summary");
        assert_eq!(gm.record_count, 8);
        assert_eq!(gm.archive_count, 4);
        let original_category = gm_atlas_category_path();
        assert!(
            summary.verified_categories.iter().any(|category| {
                category.path == original_category && category.asset_count == 4
            })
        );
        assert_eq!(
            summary
                .verified_categories
                .iter()
                .filter(|category| category.path.starts_with(&[
                    "UI 리소스".to_owned(),
                    "GM".to_owned(),
                    "원시 렌더링 조각".to_owned(),
                ]))
                .count(),
            4
        );

        let mut session = ViewerSession::default();
        session.set_resource_directory(resource_root);
        let atlas_page = session
            .category_page(&original_category, 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("load GM original atlas page");
        assert_eq!(atlas_page.total_count, 4);
        assert_eq!(atlas_page.items[0].block_index, 0);
        assert_eq!(
            (
                atlas_page.items[0].source_width,
                atlas_page.items[0].source_height
            ),
            (1, 1)
        );

        let category = ["UI 리소스", "GM", "원시 렌더링 조각", "아틀라스 ID 00"].map(str::to_owned);
        let page = session
            .category_page(&category, 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("load GM sprite page");
        assert_eq!(page.total_count, 1);
        assert_eq!(page.items[0].archive, "gm");
        assert_eq!(page.items[0].block_index, 4);
        assert_eq!(page.items[0].icon_id, Some(0));
        assert_eq!(
            (page.items[0].source_width, page.items[0].source_height),
            (1, 1)
        );
        let detail = session
            .asset_detail(&category, "gm", 4)
            .expect("load GM sprite detail");
        let source = detail.gm_source.expect("GM sprite source context");
        assert_eq!(source.atlas_id, 0);
        assert_eq!(
            (source.x, source.y, source.width, source.height),
            (0, 0, 1, 1)
        );

        let fourth_category =
            ["UI 리소스", "GM", "원시 렌더링 조각", "아틀라스 ID 03"].map(str::to_owned);
        let png = session
            .asset_png(&fourth_category, "gm", 7)
            .expect("extract GM sprite PNG");
        assert_eq!((png.width, png.height), (1, 1));
        assert_eq!(&png.png[..8], b"\x89PNG\r\n\x1a\n");

        let snapshot = inspect_asset_snapshot(
            session
                .resource_directory()
                .expect("selected resource directory"),
        )
        .expect("snapshot GM fixture");
        assert_eq!(
            snapshot
                .assets
                .iter()
                .filter(|asset| asset.archive == "gm")
                .count(),
            8
        );
    }

    #[test]
    #[ignore = "requires DHO_GAME_DIRECTORY pointing to an installed client"]
    fn matches_real_treasure_hunt_theme_text_to_sc_group_26_images() {
        let game_directory = std::env::var_os("DHO_GAME_DIRECTORY")
            .map(PathBuf::from)
            .expect("DHO_GAME_DIRECTORY must point to an installed client");
        let summary = inspect_game_directory(&game_directory).expect("inspect installed client");
        let mut session = ViewerSession::default();
        session.set_resource_directory(&summary.resource_directory);

        let shining_hill = session
            .search_page("빛나는 언덕", 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("search shining hill treasure hunt theme")
            .items
            .into_iter()
            .find(|item| item.thumbnail.archive == "sc" && item.thumbnail.icon_id == Some(1))
            .expect("SC group 26 treasure hunt theme ID 1");

        assert_eq!(shining_hill.path, ["트레져헌트 테마"]);
        assert_eq!(shining_hill.thumbnail.block_index, 5_229);
        assert!(shining_hill.thumbnail.text_links.iter().any(|link| {
            link.source == "dt000001.bin:treasure_category"
                && link.id == 1
                && link.name == "빛나는 언덕"
        }));
    }

    #[test]
    #[ignore = "requires DHO_GAME_DIRECTORY pointing to an installed client"]
    fn matches_real_legacy_theme_text_to_sc_group_33_images() {
        let game_directory = std::env::var_os("DHO_GAME_DIRECTORY")
            .map(PathBuf::from)
            .expect("DHO_GAME_DIRECTORY must point to an installed client");
        let summary = inspect_game_directory(&game_directory).expect("inspect installed client");
        let mut session = ViewerSession::default();
        session.set_resource_directory(&summary.resource_directory);

        let mathematics = session
            .search_page("수의 예지 세계의 계산기", 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("search mathematics legacy theme")
            .items
            .into_iter()
            .find(|item| item.thumbnail.archive == "sc" && item.thumbnail.icon_id == Some(3))
            .expect("SC group 33 legacy theme ID 3");

        assert_eq!(mathematics.path, ["레거시 테마"]);
        assert_eq!(mathematics.thumbnail.block_index, 5_586);
        assert!(mathematics.thumbnail.text_links.iter().any(|link| {
            link.source == "dt000001.bin:legacy_theme"
                && link.id == 3
                && link.name == "수의 예지 세계의 계산기"
        }));
    }

    #[test]
    #[ignore = "requires DHO_GAME_DIRECTORY pointing to an installed client"]
    fn preserves_distinct_item_entries_that_share_one_real_image_block() {
        let game_directory = std::env::var_os("DHO_GAME_DIRECTORY")
            .map(PathBuf::from)
            .expect("DHO_GAME_DIRECTORY must point to an installed client");
        let summary = inspect_game_directory(&game_directory).expect("inspect installed client");
        let mut session = ViewerSession::default();
        session.set_resource_directory(&summary.resource_directory);

        let true_britain = session
            .search_page("트루 브리튼 교환권", 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("search True Britain exchange ticket")
            .items
            .into_iter()
            .find(|item| item.thumbnail.icon_id == Some(1_561_327))
            .expect("True Britain logical item entry");
        let cutty_sark = session
            .search_page("카티사크 선박 교환권", 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("search Cutty Sark exchange ticket")
            .items
            .into_iter()
            .find(|item| item.thumbnail.icon_id == Some(1_500_871))
            .expect("Cutty Sark logical item entry");

        assert_eq!(true_britain.thumbnail.block_index, 7_402);
        assert_eq!(cutty_sark.thumbnail.block_index, 7_402);
        assert_ne!(true_britain.thumbnail.icon_id, cutty_sark.thumbnail.icon_id);
        assert!(
            true_britain
                .thumbnail
                .text_links
                .iter()
                .any(|link| link.name.trim() == "트루 브리튼 교환권")
        );
        assert!(
            cutty_sark
                .thumbnail
                .text_links
                .iter()
                .any(|link| link.name.trim() == "카티사크 선박 교환권")
        );
    }

    #[test]
    #[ignore = "requires DHO_GAME_DIRECTORY pointing to an installed client"]
    fn opens_gm_and_rebased_sd_assemblies_from_a_real_client() {
        let game_directory = std::env::var_os("DHO_GAME_DIRECTORY")
            .map(PathBuf::from)
            .expect("DHO_GAME_DIRECTORY must point to an installed client");
        let summary = inspect_game_directory(&game_directory).expect("inspect installed client");
        let gm = summary
            .archives
            .iter()
            .find(|archive| archive.prefix == "gm")
            .expect("installed GM archive");
        assert_eq!(gm.record_count, 1_769);
        for atlas_id in 2..=15 {
            assert!(summary.verified_categories.iter().any(|category| {
                category.path == gm_sprite_category_path(atlas_id) && category.asset_count > 0
            }));
        }
        assert_eq!(
            summary
                .archives
                .iter()
                .find(|archive| archive.prefix == "cu")
                .expect("installed cursor archive")
                .record_count,
            12
        );
        assert_eq!(
            summary
                .archives
                .iter()
                .find(|archive| archive.prefix == "ft")
                .expect("installed bitmap font archive")
                .record_count,
            7_486
        );
        assert_eq!(
            summary
                .archives
                .iter()
                .find(|archive| archive.prefix == "wm")
                .expect("installed XFTX world-map archive")
                .record_count,
            13
        );

        let mut session = ViewerSession::default();
        session.set_resource_directory(&summary.resource_directory);
        let gm_atlas_path = gm_atlas_category_path();
        let gm_atlas_page = session
            .category_page(&gm_atlas_path, 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("installed GM original atlas category page");
        assert_eq!(gm_atlas_page.total_count, 60);
        let atlas_detail = session
            .asset_detail(&gm_atlas_path, "gm", gm_atlas_page.items[0].block_index)
            .expect("installed GM original atlas detail");
        assert!(atlas_detail.gm_source.is_none());
        assert!(
            atlas_detail
                .preview_data_url
                .starts_with("data:image/png;base64,")
        );

        let gm_path = ["UI 리소스", "GM", "원시 렌더링 조각", "아틀라스 ID 02"].map(str::to_owned);
        let gm_page = session
            .category_page(&gm_path, 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("installed GM sprite category page");
        assert!(gm_page.total_count > 1);
        assert!(gm_page.items.iter().all(|item| item.source_width <= 128));
        let sprite_detail = session
            .asset_detail(&gm_path, "gm", gm_page.items[0].block_index)
            .expect("installed GM sprite detail");
        let source = sprite_detail
            .gm_source
            .expect("installed GM sprite source context");
        assert_eq!(source.atlas_id, 2);
        assert!(source.atlas_width > 0 && source.atlas_height > 0);
        assert!(source.x < source.atlas_width && source.y < source.atlas_height);
        assert!(source.preview_width > 0 && source.preview_height > 0);
        assert!(
            source
                .preview_data_url
                .starts_with("data:image/png;base64,")
        );

        let gm_five_path =
            ["UI 리소스", "GM", "원시 렌더링 조각", "아틀라스 ID 05"].map(str::to_owned);
        let gm_five_page = session
            .category_page(&gm_five_path, 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("installed GM atlas 05 sprite category page");
        assert_eq!(gm_five_page.total_count, 63);
        assert_eq!(gm_five_page.items.len(), 63);
        assert_eq!(gm_five_page.items[0].icon_id, Some(463));
        assert_eq!(gm_five_page.items[62].icon_id, Some(525));
        assert!(gm_five_page.items.iter().all(|item| {
            (item.source_width, item.source_height) == (16, 16) && !item.assembled
        }));

        let cursor_path = ["UI 리소스", "커서"].map(str::to_owned);
        let cursor_page = session
            .category_page(&cursor_path, 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("installed cursor category page");
        assert_eq!(cursor_page.total_count, 12);
        assert!(
            cursor_page
                .items
                .iter()
                .all(|item| (item.source_width, item.source_height) == (32, 32))
        );

        let font_path = ["글꼴", "게임 글리프"].map(str::to_owned);
        let font_page = session
            .category_page(&font_path, 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("installed bitmap font category page");
        assert_eq!(font_page.total_count, 7_486);
        assert!(
            font_page.items.iter().all(|item| {
                matches!((item.source_width, item.source_height), (8, 16) | (16, 16))
            })
        );

        let world_map_path = ["지도", "축소 세계지도 리소스"].map(str::to_owned);
        let world_map_page = session
            .category_page(&world_map_path, 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("installed XFTX world-map category page");
        assert_eq!(world_map_page.total_count, 13);
        assert!(
            world_map_page
                .items
                .iter()
                .any(|item| { (item.source_width, item.source_height) == (128, 128) })
        );

        let sd_path = ["조립 이미지", "SD"].map(str::to_owned);
        let sd_page = session
            .category_page(&sd_path, 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("rebased installed SD assembly page");
        assert!(sd_page.total_count > 0);
        assert!(sd_page.items.iter().all(|item| item.assembled));
    }

    #[test]
    fn displays_one_assembled_kp_world_map_and_five_overviews() {
        let directory = TestDirectory::new();
        let primary = directory.prepare_game();
        let resource_root = primary.parent().expect("resource root");
        let inline_directory = resource_root.join("0000");
        fs::create_dir(&inline_directory).expect("create inline resource directory");
        write_repeated_inline_data_file(
            &inline_directory.join("kp000000.bin"),
            &[30, 20, 10, 255].repeat(48 * 48),
            2_048,
        );
        write_repeated_inline_data_file(
            &inline_directory.join("kp000010.bin"),
            &[0, 0, 0, 0].repeat(48 * 48),
            2_048,
        );
        write_inline_data_file(
            &inline_directory.join("kp100000.bin"),
            &vec![vec![0x44; 256 * 256 * 4]; 5],
        );
        let mut session = ViewerSession::default();
        session.set_resource_directory(resource_root);
        let world_map = ["조립 이미지", "KP"].map(str::to_owned);
        let overviews = ["미분류", "KP"].map(str::to_owned);

        let world_page = session
            .category_page(&world_map, 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("load assembled KP world map");
        assert_eq!(world_page.total_count, 1);
        assert_eq!(world_page.items[0].archive, "kp");
        assert_eq!(world_page.items[0].icon_id, None);
        assert_eq!(
            (
                world_page.items[0].source_width,
                world_page.items[0].source_height
            ),
            (3_072, 1_536)
        );
        assert!(world_page.items[0].assembled);

        let detail = session
            .asset_detail(&world_map, "KP", 0)
            .expect("load KP world map detail");
        assert_eq!((detail.source_width, detail.source_height), (3_072, 1_536));
        assert!(detail.assembled);
        let png = session
            .asset_png(&world_map, "kp", 0)
            .expect("extract KP world map PNG");
        assert_eq!((png.width, png.height), (3_072, 1_536));
        assert!(png.assembled);
        assert_eq!(&png.png[..8], b"\x89PNG\r\n\x1a\n");

        let overview_page = session
            .category_page(&overviews, 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("load KP overviews");
        assert_eq!(overview_page.total_count, 5);
        assert!(overview_page.items.iter().all(|item| !item.assembled));
        assert!(
            overview_page
                .items
                .iter()
                .all(|item| (item.source_width, item.source_height) == (256, 256))
        );

        let search = session
            .search_page("KP 조립 이미지", 0, VIEWER_CATEGORY_PAGE_SIZE)
            .expect("search KP world map");
        assert_eq!(search.total_count, 1);
        assert_eq!(search.items[0].thumbnail.block_index, 0);

        let update = session
            .update_page(
                &[
                    AssetSnapshotEntry::new_raw(
                        "kp",
                        RawResourceKey {
                            block_index: 7,
                            file_number: 0,
                            file_block_index: 7,
                        },
                        48,
                        48,
                    ),
                    AssetSnapshotEntry::new_raw(
                        "kp",
                        RawResourceKey {
                            block_index: 2_055,
                            file_number: 10,
                            file_block_index: 7,
                        },
                        48,
                        48,
                    ),
                ],
                0,
                VIEWER_CATEGORY_PAGE_SIZE,
            )
            .expect("deduplicate changed KP layers");
        assert_eq!(update.total_count, 1);
        assert_eq!(update.review_required_count, 0);
        assert!(update.items[0].thumbnail.assembled);

        let summary = inspect_game_directory(&directory.0).expect("inspect KP-only fixture");
        assert_eq!(summary.archives.len(), 1);
        assert_eq!(summary.archives[0].prefix, "kp");
        assert_eq!(summary.archives[0].record_count, 4_101);
        assert_eq!(summary.archives[0].archive_count, 3);
        assert_eq!(
            summary.verified_categories,
            [
                VerifiedCategorySummary {
                    path: overviews.to_vec(),
                    asset_count: 5,
                },
                VerifiedCategorySummary {
                    path: world_map.to_vec(),
                    asset_count: 1,
                },
            ]
        );
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
        let category = ["조립 이미지", "SD"].map(str::to_owned);

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

        let selected = session
            .selected_asset(&category, "SD", 10_368)
            .expect("revalidate selected assembly");
        assert!(selected.assembled());
        let selected_png = session
            .search_asset_png(&selected)
            .expect("extract selected assembly directly");
        assert!(selected_png.assembled);
        assert_eq!((selected_png.width, selected_png.height), (782, 404));
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
