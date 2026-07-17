// SPDX-License-Identifier: MPL-2.0

use dho_catalog::{CatalogRecordKey, RecordClassification, assembly_plan, classify_record};
use dho_client::SUPPORTED_ARCHIVE_PREFIXES;
use dho_extract::{ExtractError, LoadedArchive, ResourceKey};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::path::PathBuf;

const MAX_SAMPLE_COUNT: usize = 24;
const MAX_SAMPLE_OUTPUT_SIZE: usize = 16 * 1024 * 1024;
const MAX_ASSEMBLED_OUTPUT_SIZE: usize = 64 * 1024 * 1024;
const ICON_ID_GAP_THRESHOLD: u32 = 1_000;
const ICON_ID_BAND_SIZE: u32 = 100_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveGroupSummary {
    pub group_code: u32,
    pub record_count: usize,
    pub unique_block_count: usize,
    pub min_icon_id: u32,
    pub max_icon_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSample {
    pub prefix: String,
    pub group_code: u32,
    pub icon_id: u32,
    pub block_index: u32,
    pub width: u32,
    pub height: u32,
    pub classification: RecordClassification,
    pub has_verified_assembly: bool,
    pub png: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssemblyPreview {
    pub prefix: String,
    pub requested_block_index: u32,
    pub first_block: u32,
    pub last_block: u32,
    pub columns: u32,
    pub rows: u32,
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<ResourceSample>,
    pub png: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveIdRangeSummary {
    pub start_icon_id: u32,
    pub end_icon_id: u32,
    pub record_count: usize,
    pub unique_block_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupIdRanges {
    pub prefix: String,
    pub group_code: u32,
    pub gap_threshold: u32,
    pub ranges: Vec<ArchiveIdRangeSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RangeSamples {
    pub prefix: String,
    pub group_code: u32,
    pub start_icon_id: u32,
    pub end_icon_id: u32,
    pub record_count: usize,
    pub unique_block_count: usize,
    pub first_record: ResourceSample,
    pub last_record: ResourceSample,
    pub samples: Vec<ResourceSample>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveIdBandSummary {
    pub start_icon_id: u32,
    pub end_icon_id: u32,
    pub first_actual_icon_id: u32,
    pub last_actual_icon_id: u32,
    pub record_count: usize,
    pub unique_block_count: usize,
    pub group_codes: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveIdBands {
    pub prefix: String,
    pub band_size: u32,
    pub bands: Vec<ArchiveIdBandSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveBandSamples {
    pub prefix: String,
    pub start_icon_id: u32,
    pub end_icon_id: u32,
    pub record_count: usize,
    pub unique_block_count: usize,
    pub group_codes: Vec<u32>,
    pub first_record: ResourceSample,
    pub last_record: ResourceSample,
    pub samples: Vec<ResourceSample>,
}

#[derive(Debug, Default)]
pub struct CuratorSession {
    resource_directory: Option<PathBuf>,
    archives: HashMap<String, LoadedArchive>,
}

impl CuratorSession {
    pub fn set_resource_directory(&mut self, path: impl Into<PathBuf>) {
        let path = path.into();
        if self.resource_directory.as_ref() != Some(&path) {
            self.archives.clear();
            self.resource_directory = Some(path);
        }
    }

    pub fn group_summaries(
        &mut self,
        prefix: &str,
    ) -> Result<Vec<ArchiveGroupSummary>, CuratorSessionError> {
        let archive = self.archive(prefix)?;
        let mut groups = BTreeMap::<u32, GroupAccumulator>::new();

        for record in archive.records() {
            groups
                .entry(record.group_code)
                .or_insert_with(|| GroupAccumulator::new(record.icon_id))
                .add(record.icon_id, record.block_index);
        }

        Ok(groups
            .into_iter()
            .map(|(group_code, group)| ArchiveGroupSummary {
                group_code,
                record_count: group.record_count,
                unique_block_count: group.block_indices.len(),
                min_icon_id: group.min_icon_id,
                max_icon_id: group.max_icon_id,
            })
            .collect())
    }

    pub fn archive_id_bands(
        &mut self,
        prefix: &str,
    ) -> Result<ArchiveIdBands, CuratorSessionError> {
        let normalized_prefix = normalize_prefix(prefix)?;
        let archive = self.archive(&normalized_prefix)?;
        let mut bands = BTreeMap::<u32, BandAccumulator>::new();

        for record in archive.records() {
            let band_start = record.icon_id / ICON_ID_BAND_SIZE * ICON_ID_BAND_SIZE;
            bands
                .entry(band_start)
                .or_insert_with(|| BandAccumulator::new(record.icon_id))
                .add(record.icon_id, record.block_index, record.group_code);
        }

        Ok(ArchiveIdBands {
            prefix: normalized_prefix,
            band_size: ICON_ID_BAND_SIZE,
            bands: bands
                .into_iter()
                .map(|(start_icon_id, band)| ArchiveIdBandSummary {
                    start_icon_id,
                    end_icon_id: start_icon_id.saturating_add(ICON_ID_BAND_SIZE - 1),
                    first_actual_icon_id: band.first_actual_icon_id,
                    last_actual_icon_id: band.last_actual_icon_id,
                    record_count: band.record_count,
                    unique_block_count: band.block_indices.len(),
                    group_codes: band.group_codes.into_iter().collect(),
                })
                .collect(),
        })
    }

    pub fn archive_band_samples(
        &mut self,
        prefix: &str,
        start_icon_id: u32,
        end_icon_id: u32,
    ) -> Result<ArchiveBandSamples, CuratorSessionError> {
        let normalized_prefix = normalize_prefix(prefix)?;
        if start_icon_id > end_icon_id {
            return Err(CuratorSessionError::InvalidIdRange {
                start_icon_id,
                end_icon_id,
            });
        }
        let archive = self.archive(&normalized_prefix)?;
        let mut records = archive
            .records()
            .iter()
            .filter(|record| (start_icon_id..=end_icon_id).contains(&record.icon_id))
            .copied()
            .collect::<Vec<_>>();
        let record_count = records.len();

        if records.is_empty() {
            return Err(CuratorSessionError::ArchiveRangeNotFound {
                prefix: normalized_prefix,
                start_icon_id,
                end_icon_id,
            });
        }

        records
            .sort_unstable_by_key(|record| (record.icon_id, record.group_code, record.block_index));
        let group_codes = records
            .iter()
            .map(|record| record.group_code)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let first = records.first().expect("band contains a first record");
        let last = records.last().expect("band contains a last record");
        let first_record = extract_sample(
            archive,
            &normalized_prefix,
            ResourceKey {
                group_code: first.group_code,
                icon_id: first.icon_id,
                block_index: first.block_index,
            },
        )?;
        let last_record = extract_sample(
            archive,
            &normalized_prefix,
            ResourceKey {
                group_code: last.group_code,
                icon_id: last.icon_id,
                block_index: last.block_index,
            },
        )?;

        let mut seen_blocks = HashSet::new();
        records.retain(|record| seen_blocks.insert(record.block_index));
        let unique_block_count = records.len();
        let mut samples = Vec::new();
        for index in evenly_spaced_indices(records.len(), MAX_SAMPLE_COUNT) {
            let record = records[index];
            samples.push(extract_sample(
                archive,
                &normalized_prefix,
                ResourceKey {
                    group_code: record.group_code,
                    icon_id: record.icon_id,
                    block_index: record.block_index,
                },
            )?);
        }

        Ok(ArchiveBandSamples {
            prefix: normalized_prefix,
            start_icon_id,
            end_icon_id,
            record_count,
            unique_block_count,
            group_codes,
            first_record,
            last_record,
            samples,
        })
    }

    pub fn group_id_ranges(
        &mut self,
        prefix: &str,
        group_code: u32,
    ) -> Result<GroupIdRanges, CuratorSessionError> {
        let normalized_prefix = normalize_prefix(prefix)?;
        let archive = self.archive(&normalized_prefix)?;
        let mut records = archive
            .records()
            .iter()
            .filter(|record| record.group_code == group_code)
            .copied()
            .collect::<Vec<_>>();

        if records.is_empty() {
            return Err(CuratorSessionError::GroupNotFound {
                prefix: normalized_prefix,
                group_code,
            });
        }

        records.sort_unstable_by_key(|record| (record.icon_id, record.block_index));
        let mut ranges = Vec::new();
        let mut range_start = 0;
        for position in 1..=records.len() {
            let reached_end = position == records.len();
            let found_gap = !reached_end
                && u64::from(records[position].icon_id) - u64::from(records[position - 1].icon_id)
                    > u64::from(ICON_ID_GAP_THRESHOLD);
            if reached_end || found_gap {
                let range_records = &records[range_start..position];
                ranges.push(ArchiveIdRangeSummary {
                    start_icon_id: range_records
                        .first()
                        .expect("range always contains a record")
                        .icon_id,
                    end_icon_id: range_records
                        .last()
                        .expect("range always contains a record")
                        .icon_id,
                    record_count: range_records.len(),
                    unique_block_count: range_records
                        .iter()
                        .map(|record| record.block_index)
                        .collect::<HashSet<_>>()
                        .len(),
                });
                range_start = position;
            }
        }

        Ok(GroupIdRanges {
            prefix: normalized_prefix,
            group_code,
            gap_threshold: ICON_ID_GAP_THRESHOLD,
            ranges,
        })
    }

    pub fn range_samples(
        &mut self,
        prefix: &str,
        group_code: u32,
        start_icon_id: u32,
        end_icon_id: u32,
    ) -> Result<RangeSamples, CuratorSessionError> {
        let normalized_prefix = normalize_prefix(prefix)?;
        if start_icon_id > end_icon_id {
            return Err(CuratorSessionError::InvalidIdRange {
                start_icon_id,
                end_icon_id,
            });
        }
        let archive = self.archive(&normalized_prefix)?;
        let mut records = archive
            .records()
            .iter()
            .filter(|record| {
                record.group_code == group_code
                    && (start_icon_id..=end_icon_id).contains(&record.icon_id)
            })
            .copied()
            .collect::<Vec<_>>();
        let record_count = records.len();

        if records.is_empty() {
            return Err(CuratorSessionError::IdRangeNotFound {
                prefix: normalized_prefix,
                group_code,
                start_icon_id,
                end_icon_id,
            });
        }

        records.sort_unstable_by_key(|record| (record.icon_id, record.block_index));
        let first_record = records.first().expect("range contains a first record");
        let last_record = records.last().expect("range contains a last record");
        let first_record = extract_sample(
            archive,
            &normalized_prefix,
            ResourceKey {
                group_code: first_record.group_code,
                icon_id: first_record.icon_id,
                block_index: first_record.block_index,
            },
        )?;
        let last_record = extract_sample(
            archive,
            &normalized_prefix,
            ResourceKey {
                group_code: last_record.group_code,
                icon_id: last_record.icon_id,
                block_index: last_record.block_index,
            },
        )?;

        let mut seen_blocks = HashSet::new();
        records.retain(|record| seen_blocks.insert(record.block_index));
        let unique_block_count = records.len();

        let mut samples = Vec::new();
        for index in evenly_spaced_indices(records.len(), MAX_SAMPLE_COUNT) {
            let record = records[index];
            let key = ResourceKey {
                group_code: record.group_code,
                icon_id: record.icon_id,
                block_index: record.block_index,
            };
            samples.push(extract_sample(archive, &normalized_prefix, key)?);
        }

        Ok(RangeSamples {
            prefix: normalized_prefix,
            group_code,
            start_icon_id,
            end_icon_id,
            record_count,
            unique_block_count,
            first_record,
            last_record,
            samples,
        })
    }

    pub fn assembly_preview(
        &mut self,
        prefix: &str,
        block_index: u32,
    ) -> Result<AssemblyPreview, CuratorSessionError> {
        let normalized_prefix = normalize_prefix(prefix)?;
        let plan = assembly_plan(&normalized_prefix, block_index).ok_or_else(|| {
            CuratorSessionError::VerifiedAssemblyNotFound {
                prefix: normalized_prefix.clone(),
                block_index,
            }
        })?;
        let archive = self.archive(&normalized_prefix)?;
        let assembled = archive
            .extract_verified_assembly(
                block_index,
                MAX_SAMPLE_OUTPUT_SIZE,
                MAX_ASSEMBLED_OUTPUT_SIZE,
            )
            .map_err(|source| CuratorSessionError::Extract {
                prefix: normalized_prefix.clone(),
                source,
            })?
            .ok_or_else(|| CuratorSessionError::VerifiedAssemblyNotFound {
                prefix: normalized_prefix.clone(),
                block_index,
            })?;

        let mut tiles = Vec::new();
        for tile_block_index in plan.first_block..=plan.last_block {
            let record = archive
                .records()
                .iter()
                .find(|record| record.block_index == tile_block_index)
                .copied()
                .ok_or_else(|| CuratorSessionError::AssemblyTileRecordNotFound {
                    prefix: normalized_prefix.clone(),
                    block_index: tile_block_index,
                })?;
            tiles.push(extract_sample(
                archive,
                &normalized_prefix,
                ResourceKey {
                    group_code: record.group_code,
                    icon_id: record.icon_id,
                    block_index: record.block_index,
                },
            )?);
        }

        Ok(AssemblyPreview {
            prefix: normalized_prefix,
            requested_block_index: block_index,
            first_block: assembled.first_block,
            last_block: assembled.last_block,
            columns: plan.rule.columns,
            rows: plan.rule.rows,
            width: assembled.width,
            height: assembled.height,
            tiles,
            png: assembled.png,
        })
    }

    fn archive(&mut self, prefix: &str) -> Result<&LoadedArchive, CuratorSessionError> {
        let prefix = normalize_prefix(prefix)?;
        let resource_directory = self
            .resource_directory
            .as_deref()
            .ok_or(CuratorSessionError::ResourceDirectoryNotSelected)?;

        if !self.archives.contains_key(&prefix) {
            let archive = LoadedArchive::open(resource_directory, &prefix).map_err(|source| {
                CuratorSessionError::OpenArchive {
                    prefix: prefix.clone(),
                    source,
                }
            })?;
            self.archives.insert(prefix.clone(), archive);
        }

        Ok(self
            .archives
            .get(&prefix)
            .expect("archive was inserted before lookup"))
    }
}

fn extract_sample(
    archive: &LoadedArchive,
    prefix: &str,
    key: ResourceKey,
) -> Result<ResourceSample, CuratorSessionError> {
    let extracted = archive
        .extract_png(key, MAX_SAMPLE_OUTPUT_SIZE)
        .map_err(|source| CuratorSessionError::Extract {
            prefix: prefix.to_owned(),
            source,
        })?;
    Ok(ResourceSample {
        prefix: prefix.to_owned(),
        group_code: key.group_code,
        icon_id: key.icon_id,
        block_index: key.block_index,
        width: extracted.width,
        height: extracted.height,
        classification: classify_record(CatalogRecordKey {
            archive: prefix,
            group_code: key.group_code,
            icon_id: key.icon_id,
            block_index: key.block_index,
        }),
        has_verified_assembly: assembly_plan(prefix, key.block_index).is_some(),
        png: extracted.png,
    })
}

#[derive(Debug)]
struct GroupAccumulator {
    record_count: usize,
    block_indices: HashSet<u32>,
    min_icon_id: u32,
    max_icon_id: u32,
}

#[derive(Debug)]
struct BandAccumulator {
    record_count: usize,
    block_indices: HashSet<u32>,
    group_codes: BTreeSet<u32>,
    first_actual_icon_id: u32,
    last_actual_icon_id: u32,
}

impl BandAccumulator {
    fn new(icon_id: u32) -> Self {
        Self {
            record_count: 0,
            block_indices: HashSet::new(),
            group_codes: BTreeSet::new(),
            first_actual_icon_id: icon_id,
            last_actual_icon_id: icon_id,
        }
    }

    fn add(&mut self, icon_id: u32, block_index: u32, group_code: u32) {
        self.record_count += 1;
        self.block_indices.insert(block_index);
        self.group_codes.insert(group_code);
        self.first_actual_icon_id = self.first_actual_icon_id.min(icon_id);
        self.last_actual_icon_id = self.last_actual_icon_id.max(icon_id);
    }
}

impl GroupAccumulator {
    fn new(icon_id: u32) -> Self {
        Self {
            record_count: 0,
            block_indices: HashSet::new(),
            min_icon_id: icon_id,
            max_icon_id: icon_id,
        }
    }

    fn add(&mut self, icon_id: u32, block_index: u32) {
        self.record_count += 1;
        self.block_indices.insert(block_index);
        self.min_icon_id = self.min_icon_id.min(icon_id);
        self.max_icon_id = self.max_icon_id.max(icon_id);
    }
}

fn normalize_prefix(prefix: &str) -> Result<String, CuratorSessionError> {
    let prefix = prefix.to_ascii_lowercase();
    if SUPPORTED_ARCHIVE_PREFIXES.contains(&prefix.as_str()) {
        Ok(prefix)
    } else {
        Err(CuratorSessionError::UnsupportedPrefix { prefix })
    }
}

fn evenly_spaced_indices(total: usize, limit: usize) -> Vec<usize> {
    if total == 0 || limit == 0 {
        return Vec::new();
    }
    if total <= limit {
        return (0..total).collect();
    }
    if limit == 1 {
        return vec![0];
    }

    (0..limit)
        .map(|position| position * (total - 1) / (limit - 1))
        .collect()
}

#[derive(Debug)]
pub enum CuratorSessionError {
    ResourceDirectoryNotSelected,
    UnsupportedPrefix {
        prefix: String,
    },
    OpenArchive {
        prefix: String,
        source: ExtractError,
    },
    GroupNotFound {
        prefix: String,
        group_code: u32,
    },
    InvalidIdRange {
        start_icon_id: u32,
        end_icon_id: u32,
    },
    IdRangeNotFound {
        prefix: String,
        group_code: u32,
        start_icon_id: u32,
        end_icon_id: u32,
    },
    ArchiveRangeNotFound {
        prefix: String,
        start_icon_id: u32,
        end_icon_id: u32,
    },
    VerifiedAssemblyNotFound {
        prefix: String,
        block_index: u32,
    },
    AssemblyTileRecordNotFound {
        prefix: String,
        block_index: u32,
    },
    Extract {
        prefix: String,
        source: ExtractError,
    },
}

impl fmt::Display for CuratorSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResourceDirectoryNotSelected => {
                write!(formatter, "먼저 게임 폴더를 선택해 주세요.")
            }
            Self::UnsupportedPrefix { prefix } => {
                write!(formatter, "지원하지 않는 MWC 계열입니다: {prefix}")
            }
            Self::OpenArchive { prefix, source } => {
                write!(
                    formatter,
                    "{prefix} 리소스를 여는 데 실패했습니다: {source}"
                )
            }
            Self::GroupNotFound { prefix, group_code } => {
                write!(
                    formatter,
                    "{prefix}에서 원시 그룹 {group_code}를 찾지 못했습니다."
                )
            }
            Self::InvalidIdRange {
                start_icon_id,
                end_icon_id,
            } => write!(
                formatter,
                "ID 구간의 시작값이 끝값보다 큽니다: {start_icon_id}–{end_icon_id}"
            ),
            Self::IdRangeNotFound {
                prefix,
                group_code,
                start_icon_id,
                end_icon_id,
            } => write!(
                formatter,
                "{prefix} 원시 그룹 {group_code}에서 ID {start_icon_id}–{end_icon_id} 구간을 찾지 못했습니다."
            ),
            Self::ArchiveRangeNotFound {
                prefix,
                start_icon_id,
                end_icon_id,
            } => write!(
                formatter,
                "{prefix}에서 ID {start_icon_id}–{end_icon_id} 대역의 레코드를 찾지 못했습니다."
            ),
            Self::VerifiedAssemblyNotFound {
                prefix,
                block_index,
            } => write!(
                formatter,
                "{prefix} 블록 {block_index}에는 사람이 검증한 조립 규칙이 없습니다."
            ),
            Self::AssemblyTileRecordNotFound {
                prefix,
                block_index,
            } => write!(
                formatter,
                "{prefix} 조립 타일 블록 {block_index}의 인덱스 레코드를 찾지 못했습니다."
            ),
            Self::Extract { prefix, source } => {
                write!(
                    formatter,
                    "{prefix} 표본 이미지를 추출하지 못했습니다: {source}"
                )
            }
        }
    }
}

impl Error for CuratorSessionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::OpenArchive { source, .. } | Self::Extract { source, .. } => Some(source),
            Self::ResourceDirectoryNotSelected
            | Self::UnsupportedPrefix { .. }
            | Self::GroupNotFound { .. }
            | Self::InvalidIdRange { .. }
            | Self::IdRangeNotFound { .. }
            | Self::ArchiveRangeNotFound { .. }
            | Self::VerifiedAssemblyNotFound { .. }
            | Self::AssemblyTileRecordNotFound { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dho_catalog::VerificationStatus;
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::fs;
    use std::io::Write;
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let number = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "dho-vault-curator-session-test-{}-{number}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("create test directory");
            Self(path)
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

    fn write_archive(directory: &Path, records: &[[u32; 5]], block_count: u32) {
        let mut index = Vec::new();
        let group_count = records
            .iter()
            .map(|record| record[4])
            .collect::<HashSet<_>>()
            .len() as u32;
        for value in [records.len() as u32, group_count, 1, 1, block_count, 1, 0] {
            push_u32(&mut index, value);
        }
        for record in records {
            for value in record {
                push_u32(&mut index, *value);
            }
        }
        fs::write(directory.join("sb000000.bin"), index).expect("write test index");

        let mut data = Vec::new();
        for value in 0..block_count {
            data.extend(zlib_block(&[value as u8, 0, 0, 255]));
        }
        fs::write(directory.join("sb000001.bin"), data).expect("write test data");
    }

    #[test]
    fn selects_evenly_spaced_indices_including_both_ends() {
        assert_eq!(evenly_spaced_indices(0, 24), Vec::<usize>::new());
        assert_eq!(evenly_spaced_indices(4, 24), [0, 1, 2, 3]);
        assert_eq!(evenly_spaced_indices(10, 4), [0, 3, 6, 9]);
        assert_eq!(evenly_spaced_indices(10, 1), [0]);
    }

    #[test]
    fn requires_a_selected_resource_directory() {
        let error = CuratorSession::default().group_summaries("sb").unwrap_err();

        assert!(matches!(
            error,
            CuratorSessionError::ResourceDirectoryNotSelected
        ));
    }

    #[test]
    fn rejects_an_unsupported_archive_prefix() {
        let directory = TestDirectory::new();
        let mut session = CuratorSession::default();
        session.set_resource_directory(&directory.0);

        let error = session.group_summaries("xy").unwrap_err();

        assert!(matches!(
            error,
            CuratorSessionError::UnsupportedPrefix { ref prefix } if prefix == "xy"
        ));
    }

    #[test]
    fn rejects_a_block_without_a_verified_assembly_rule() {
        let directory = TestDirectory::new();
        let mut session = CuratorSession::default();
        session.set_resource_directory(&directory.0);

        let error = session.assembly_preview("sb", 0).unwrap_err();

        assert!(matches!(
            error,
            CuratorSessionError::VerifiedAssemblyNotFound {
                ref prefix,
                block_index: 0,
            } if prefix == "sb"
        ));
    }

    #[test]
    fn includes_reviewed_and_unreviewed_classifications_in_samples() {
        let directory = TestDirectory::new();
        write_archive(
            &directory.0,
            &[[1_200_001, 0, 1, 1, 9], [1_200_002, 1, 1, 1, 9]],
            2,
        );
        let mut session = CuratorSession::default();
        session.set_resource_directory(&directory.0);

        let samples = session
            .range_samples("sb", 9, 1_200_001, 1_200_002)
            .expect("sample classifications");

        assert_eq!(samples.first_record.classification.category, None);
        assert_eq!(
            samples.first_record.classification.boundary_status,
            VerificationStatus::HumanVerified
        );
        assert_eq!(
            samples.first_record.classification.meaning_status,
            VerificationStatus::Unknown
        );
        assert_eq!(
            samples.last_record.classification,
            RecordClassification::unknown()
        );
    }

    #[test]
    fn splits_large_id_gaps_and_deduplicates_each_range_before_sampling() {
        let directory = TestDirectory::new();
        write_archive(
            &directory.0,
            &[
                [2_500, 2, 1, 1, 9],
                [100, 0, 1, 1, 9],
                [200, 0, 1, 1, 9],
                [400, 1, 1, 1, 10],
            ],
            3,
        );
        let mut session = CuratorSession::default();
        session.set_resource_directory(&directory.0);

        let groups = session.group_summaries("SB").expect("summarize groups");
        let ranges = session.group_id_ranges("sb", 9).expect("split ID ranges");
        let first_samples = session
            .range_samples("sb", 9, 100, 200)
            .expect("sample first range");
        let second_samples = session
            .range_samples("sb", 9, 2_500, 2_500)
            .expect("sample second range");

        assert_eq!(
            groups,
            [
                ArchiveGroupSummary {
                    group_code: 9,
                    record_count: 3,
                    unique_block_count: 2,
                    min_icon_id: 100,
                    max_icon_id: 2_500,
                },
                ArchiveGroupSummary {
                    group_code: 10,
                    record_count: 1,
                    unique_block_count: 1,
                    min_icon_id: 400,
                    max_icon_id: 400,
                },
            ]
        );
        assert_eq!(
            ranges.ranges,
            [
                ArchiveIdRangeSummary {
                    start_icon_id: 100,
                    end_icon_id: 200,
                    record_count: 2,
                    unique_block_count: 1,
                },
                ArchiveIdRangeSummary {
                    start_icon_id: 2_500,
                    end_icon_id: 2_500,
                    record_count: 1,
                    unique_block_count: 1,
                },
            ]
        );
        assert_eq!(ranges.gap_threshold, 1_000);
        assert_eq!(first_samples.record_count, 2);
        assert_eq!(first_samples.unique_block_count, 1);
        assert_eq!(first_samples.first_record.icon_id, 100);
        assert_eq!(first_samples.last_record.icon_id, 200);
        assert_eq!(
            first_samples
                .samples
                .iter()
                .map(|sample| (sample.icon_id, sample.block_index))
                .collect::<Vec<_>>(),
            [(100, 0)]
        );
        assert_eq!(second_samples.samples[0].icon_id, 2_500);
        assert!(
            first_samples
                .samples
                .iter()
                .chain(&second_samples.samples)
                .all(|sample| sample.png.starts_with(b"\x89PNG\r\n\x1a\n"))
        );
        assert!(
            first_samples
                .samples
                .iter()
                .chain(&second_samples.samples)
                .all(|sample| !sample.has_verified_assembly)
        );
    }

    #[test]
    fn groups_archive_records_into_numeric_bands_across_raw_groups() {
        let directory = TestDirectory::new();
        write_archive(
            &directory.0,
            &[
                [99_006, 0, 1, 1, 1],
                [100_100, 1, 1, 1, 1],
                [199_002, 2, 1, 1, 2],
                [200_100, 3, 1, 1, 2],
            ],
            4,
        );
        let mut session = CuratorSession::default();
        session.set_resource_directory(&directory.0);

        let bands = session.archive_id_bands("sb").expect("summarize ID bands");
        let samples = session
            .archive_band_samples("sb", 100_000, 199_999)
            .expect("sample archive band");

        assert_eq!(bands.band_size, 100_000);
        assert_eq!(
            bands.bands,
            [
                ArchiveIdBandSummary {
                    start_icon_id: 0,
                    end_icon_id: 99_999,
                    first_actual_icon_id: 99_006,
                    last_actual_icon_id: 99_006,
                    record_count: 1,
                    unique_block_count: 1,
                    group_codes: vec![1],
                },
                ArchiveIdBandSummary {
                    start_icon_id: 100_000,
                    end_icon_id: 199_999,
                    first_actual_icon_id: 100_100,
                    last_actual_icon_id: 199_002,
                    record_count: 2,
                    unique_block_count: 2,
                    group_codes: vec![1, 2],
                },
                ArchiveIdBandSummary {
                    start_icon_id: 200_000,
                    end_icon_id: 299_999,
                    first_actual_icon_id: 200_100,
                    last_actual_icon_id: 200_100,
                    record_count: 1,
                    unique_block_count: 1,
                    group_codes: vec![2],
                },
            ]
        );
        assert_eq!(samples.group_codes, [1, 2]);
        assert_eq!(samples.first_record.icon_id, 100_100);
        assert_eq!(samples.first_record.group_code, 1);
        assert_eq!(samples.last_record.icon_id, 199_002);
        assert_eq!(samples.last_record.group_code, 2);
        assert_eq!(samples.samples.len(), 2);
    }
}
