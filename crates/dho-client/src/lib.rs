// SPDX-License-Identifier: MPL-2.0

//! Read-only discovery and inspection of a DHO game client installation.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use dho_catalog::{CatalogRecordKey, VerificationStatus, assembly_plan, classify_record};
use dho_core::{IndexParseError, IndexedArchive};
use dho_extract::{ExtractError, LoadedArchive, ResourceKey};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const SUPPORTED_ARCHIVE_PREFIXES: [&str; 4] = ["sb", "sc", "sd", "is"];
pub const VIEWER_CATEGORY_PAGE_SIZE: usize = 32;

const THUMBNAIL_MAX_WIDTH: u32 = 160;
const THUMBNAIL_MAX_HEIGHT: u32 = 160;
const DETAIL_MAX_WIDTH: u32 = 1024;
const DETAIL_MAX_HEIGHT: u32 = 1024;
const MAX_IMAGE_DECODE_SIZE: usize = 64 * 1024 * 1024;
const MAX_ASSEMBLED_DECODE_SIZE: usize = 128 * 1024 * 1024;
const MAX_THUMBNAIL_DECODE_SIZE: usize =
    THUMBNAIL_MAX_WIDTH as usize * THUMBNAIL_MAX_HEIGHT as usize * 4;
const MAX_DETAIL_DECODE_SIZE: usize = DETAIL_MAX_WIDTH as usize * DETAIL_MAX_HEIGHT as usize * 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveIndexSummary {
    pub prefix: String,
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
    pub archives: Vec<ArchiveIndexSummary>,
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
pub struct VerifiedAssetThumbnail {
    pub archive: String,
    pub icon_id: u32,
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
    pub icon_id: u32,
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
    pub icon_id: u32,
    pub block_index: u32,
    pub width: u32,
    pub height: u32,
    pub assembled: bool,
    pub png: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct ViewerSession {
    resource_directory: Option<PathBuf>,
    archives: HashMap<String, LoadedArchive>,
}

#[derive(Debug, Clone)]
struct VerifiedAssetRef {
    prefix: String,
    key: ResourceKey,
    canonical_block: u32,
    assembled: bool,
}

impl ViewerSession {
    pub fn set_resource_directory(&mut self, path: impl Into<PathBuf>) {
        let path = path.into();
        if self.resource_directory.as_ref() != Some(&path) {
            self.archives.clear();
            self.resource_directory = Some(path);
        }
    }

    pub fn category_page(
        &mut self,
        path: &[String],
        offset: usize,
        page_size: usize,
    ) -> Result<VerifiedCategoryPage, ViewerSessionError> {
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

        let end = offset.saturating_add(page_size).min(assets.len());
        let selected = assets[offset..end].to_vec();
        let mut items = Vec::with_capacity(selected.len());
        for asset in selected {
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
                    icon_id: asset.key.icon_id,
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
                    icon_id: asset.key.icon_id,
                    block_index: asset.canonical_block,
                    source_width: extracted.source_width,
                    source_height: extracted.source_height,
                    thumbnail_width: extracted.width,
                    thumbnail_height: extracted.height,
                    assembled: false,
                    thumbnail_data_url: png_data_url(&extracted.png),
                }
            };
            items.push(thumbnail);
        }

        Ok(VerifiedCategoryPage {
            path: path.to_vec(),
            offset,
            page_size,
            total_count: assets.len(),
            items,
        })
    }

    pub fn asset_detail(
        &mut self,
        path: &[String],
        prefix: &str,
        block_index: u32,
    ) -> Result<VerifiedAssetDetail, ViewerSessionError> {
        let asset = self.verified_asset(path, prefix, block_index)?;
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
                icon_id: asset.key.icon_id,
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
                icon_id: asset.key.icon_id,
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
                icon_id: asset.key.icon_id,
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
                icon_id: asset.key.icon_id,
                block_index: asset.canonical_block,
                width: extracted.width,
                height: extracted.height,
                assembled: false,
                png: extracted.png,
            })
        }
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

        for prefix in SUPPORTED_ARCHIVE_PREFIXES {
            if !resource_directory
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
                        canonical_block,
                        assembled: assembly_plan(prefix, canonical_block).is_some(),
                    }),
            );
        }

        Ok(assets)
    }

    fn archive(&mut self, prefix: &str) -> Result<&LoadedArchive, ViewerSessionError> {
        let resource_directory = self
            .resource_directory
            .clone()
            .ok_or(ViewerSessionError::ResourceDirectoryNotSelected)?;
        if !self.archives.contains_key(prefix) {
            let archive = LoadedArchive::open(&resource_directory, prefix).map_err(|source| {
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
}

fn category_matches(segments: &[&str], path: &[String]) -> bool {
    segments.len() == path.len()
        && segments
            .iter()
            .zip(path)
            .all(|(segment, expected)| *segment == expected)
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

    let resource_directory = game_directory.join("0010").join("0001");
    if !resource_directory.is_dir() {
        return Err(GameDirectoryError::MissingResourceDirectory {
            path: resource_directory,
        });
    }

    let mut archives = Vec::new();
    let mut verified_assets = BTreeMap::<Vec<&'static str>, BTreeSet<(String, u32)>>::new();
    for prefix in SUPPORTED_ARCHIVE_PREFIXES {
        let path = resource_directory.join(format!("{prefix}000000.bin"));
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
        archives.push(ArchiveIndexSummary {
            prefix: prefix.to_owned(),
            record_count: header.record_count,
            group_count: header.group_count,
            image_block_count: header.image_block_count,
            archive_count: header.archive_count,
        });
    }

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
            | Self::InvalidPageSize { .. }
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
            Self::NoSupportedArchives { path } => write!(
                formatter,
                "지원하는 MWC 인덱스(sb, sc, sd, is)를 찾지 못했습니다: {}",
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

    #[test]
    fn reports_supported_archive_headers() {
        let directory = TestDirectory::new();
        let resources = directory.prepare_game();
        write_index(&resources.join("sb000000.bin"), 10);
        write_index(&resources.join("is000000.bin"), 20);

        let summary = inspect_game_directory(&directory.0).expect("inspect game directory");

        assert_eq!(
            summary
                .archives
                .iter()
                .map(|archive| archive.prefix.as_str())
                .collect::<Vec<_>>(),
            ["sb", "is"]
        );
        assert_eq!(summary.archives[0].record_count, 1);
        assert_eq!(summary.archives[0].group_count, 1);
        assert_eq!(summary.archives[0].image_block_count, 1);
        assert_eq!(summary.archives[0].archive_count, 1);
        assert_eq!(
            summary.verified_categories,
            [VerifiedCategorySummary {
                path: ["장비", "방어구", "몸"].map(str::to_owned).to_vec(),
                asset_count: 1,
            }]
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
