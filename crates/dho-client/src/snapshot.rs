// SPDX-License-Identifier: MPL-2.0

use crate::{
    INDEXED_ARCHIVE_PREFIXES, RAW_IMAGE_ARCHIVES, raw_archive_path, resolve_archive_directory,
};
use dho_core::{IndexParseError, IndexedArchive};
use dho_extract::{ExtractError, LoadedRawImageArchive, RawResourceKey};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const ASSET_SNAPSHOT_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetSnapshot {
    pub format_version: u32,
    pub assets: Vec<AssetSnapshotEntry>,
}

impl AssetSnapshot {
    pub fn new(mut assets: Vec<AssetSnapshotEntry>) -> Self {
        assets.sort();
        Self {
            format_version: ASSET_SNAPSHOT_FORMAT_VERSION,
            assets,
        }
    }

    pub fn compare_to(
        &self,
        current: &Self,
    ) -> Result<AssetSnapshotDiff, AssetSnapshotCompareError> {
        if self.format_version != ASSET_SNAPSHOT_FORMAT_VERSION
            || current.format_version != ASSET_SNAPSHOT_FORMAT_VERSION
        {
            return Err(AssetSnapshotCompareError {
                supported: ASSET_SNAPSHOT_FORMAT_VERSION,
                previous: self.format_version,
                current: current.format_version,
            });
        }
        Ok(compare_asset_snapshots(self, current))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetSnapshotEntry {
    pub archive: String,
    #[serde(default)]
    pub source_kind: AssetSourceKind,
    pub group_code: u32,
    pub icon_id: u32,
    pub block_index: u32,
    pub width: u32,
    pub height: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_file_number: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_block_index: Option<u32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetSourceKind {
    #[default]
    Indexed,
    RawBlock,
}

impl AssetSnapshotEntry {
    pub fn new(
        archive: impl Into<String>,
        group_code: u32,
        icon_id: u32,
        block_index: u32,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            archive: archive.into().to_ascii_lowercase(),
            source_kind: AssetSourceKind::Indexed,
            group_code,
            icon_id,
            block_index,
            width,
            height,
            data_file_number: None,
            file_block_index: None,
        }
    }

    pub fn new_raw(
        archive: impl Into<String>,
        key: RawResourceKey,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            archive: archive.into().to_ascii_lowercase(),
            source_kind: AssetSourceKind::RawBlock,
            group_code: 0,
            icon_id: key.block_index,
            block_index: key.block_index,
            width,
            height,
            data_file_number: Some(key.file_number),
            file_block_index: Some(key.file_block_index),
        }
    }

    pub fn raw_resource_key(&self) -> Option<RawResourceKey> {
        if self.source_kind != AssetSourceKind::RawBlock {
            return None;
        }
        Some(RawResourceKey {
            block_index: self.block_index,
            file_number: self.data_file_number?,
            file_block_index: self.file_block_index?,
        })
    }

    fn identity(&self) -> AssetIdentity {
        AssetIdentity {
            archive: self.archive.clone(),
            source_kind: self.source_kind,
            group_code: self.group_code,
            icon_id: self.icon_id,
            block_index: self.block_index,
            data_file_number: self.data_file_number,
            file_block_index: self.file_block_index,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetSnapshotChange {
    pub previous: AssetSnapshotEntry,
    pub current: AssetSnapshotEntry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetSnapshotDiff {
    pub added: Vec<AssetSnapshotEntry>,
    pub removed: Vec<AssetSnapshotEntry>,
    pub changed: Vec<AssetSnapshotChange>,
    pub unchanged_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct AssetIdentity {
    archive: String,
    source_kind: AssetSourceKind,
    group_code: u32,
    icon_id: u32,
    block_index: u32,
    data_file_number: Option<u32>,
    file_block_index: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetSnapshotCompareError {
    pub supported: u32,
    pub previous: u32,
    pub current: u32,
}

impl fmt::Display for AssetSnapshotCompareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "지원하지 않는 자산 스냅샷 형식입니다: 지원={}, 이전={}, 현재={}",
            self.supported, self.previous, self.current
        )
    }
}

impl Error for AssetSnapshotCompareError {}

pub fn inspect_asset_snapshot(
    resource_directory: impl AsRef<Path>,
) -> Result<AssetSnapshot, AssetSnapshotError> {
    let resource_directory = resource_directory.as_ref();
    if !resource_directory.is_dir() {
        return Err(AssetSnapshotError::NotDirectory {
            path: resource_directory.to_owned(),
        });
    }

    let mut assets = Vec::new();
    let mut archives = 0;
    for prefix in INDEXED_ARCHIVE_PREFIXES {
        let path = resolve_archive_directory(resource_directory, prefix)
            .join(format!("{prefix}000000.bin"));
        if !path.is_file() {
            continue;
        }
        archives += 1;
        let bytes = fs::read(&path).map_err(|source| AssetSnapshotError::ReadIndex {
            path: path.clone(),
            source,
        })?;
        let index =
            IndexedArchive::parse(&bytes).map_err(|source| AssetSnapshotError::ParseIndex {
                archive: prefix.to_owned(),
                path,
                source,
            })?;
        assets.extend(index.records.into_iter().map(|record| {
            AssetSnapshotEntry::new(
                prefix,
                record.group_code,
                record.icon_id,
                record.block_index,
                record.width,
                record.height,
            )
        }));
    }

    for definition in RAW_IMAGE_ARCHIVES {
        let directory = resolve_archive_directory(resource_directory, definition.prefix);
        if !raw_archive_path(resource_directory, definition).is_some_and(|path| path.is_file()) {
            continue;
        }
        archives += 1;
        let archive = LoadedRawImageArchive::open_files(
            directory,
            definition.prefix,
            definition.file_numbers,
            definition.layout,
            definition.spec,
        )
        .map_err(|source| AssetSnapshotError::OpenRawArchive {
            archive: definition.prefix.to_owned(),
            source,
        })?;
        assets.extend(archive.records().map(|record| {
            AssetSnapshotEntry::new_raw(definition.prefix, record.key, record.width, record.height)
        }));
    }

    if archives == 0 {
        return Err(AssetSnapshotError::NoSupportedArchives {
            path: resource_directory.to_owned(),
        });
    }
    Ok(AssetSnapshot::new(assets))
}

fn compare_asset_snapshots(previous: &AssetSnapshot, current: &AssetSnapshot) -> AssetSnapshotDiff {
    let previous = group_assets(&previous.assets);
    let current = group_assets(&current.assets);
    let identities = previous
        .keys()
        .chain(current.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();
    let mut unchanged_count = 0;

    for identity in identities {
        let mut previous_entries = previous.get(&identity).cloned().unwrap_or_default();
        let mut current_entries = current.get(&identity).cloned().unwrap_or_default();

        let mut current_index = 0;
        while current_index < current_entries.len() {
            let Some(previous_index) = previous_entries
                .iter()
                .position(|entry| entry == &current_entries[current_index])
            else {
                current_index += 1;
                continue;
            };
            previous_entries.remove(previous_index);
            current_entries.remove(current_index);
            unchanged_count += 1;
        }

        let changed_count = previous_entries.len().min(current_entries.len());
        for (previous, current) in previous_entries
            .drain(..changed_count)
            .zip(current_entries.drain(..changed_count))
        {
            changed.push(AssetSnapshotChange { previous, current });
        }
        removed.extend(previous_entries);
        added.extend(current_entries);
    }

    AssetSnapshotDiff {
        added,
        removed,
        changed,
        unchanged_count,
    }
}

fn group_assets(assets: &[AssetSnapshotEntry]) -> BTreeMap<AssetIdentity, Vec<AssetSnapshotEntry>> {
    let mut grouped = BTreeMap::<AssetIdentity, Vec<AssetSnapshotEntry>>::new();
    for asset in assets {
        grouped
            .entry(asset.identity())
            .or_default()
            .push(asset.clone());
    }
    for entries in grouped.values_mut() {
        entries.sort();
    }
    grouped
}

#[derive(Debug)]
pub enum AssetSnapshotError {
    NotDirectory {
        path: PathBuf,
    },
    ReadIndex {
        path: PathBuf,
        source: io::Error,
    },
    ParseIndex {
        archive: String,
        path: PathBuf,
        source: IndexParseError,
    },
    OpenRawArchive {
        archive: String,
        source: ExtractError,
    },
    NoSupportedArchives {
        path: PathBuf,
    },
}

impl fmt::Display for AssetSnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotDirectory { path } => {
                write!(
                    formatter,
                    "리소스 경로가 폴더가 아닙니다: {}",
                    path.display()
                )
            }
            Self::ReadIndex { path, source } => write!(
                formatter,
                "자산 스냅샷 인덱스를 읽지 못했습니다 ({}): {source}",
                path.display()
            ),
            Self::ParseIndex {
                archive,
                path,
                source,
            } => write!(
                formatter,
                "{archive} 자산 스냅샷 인덱스를 해석하지 못했습니다 ({}): {source}",
                path.display()
            ),
            Self::OpenRawArchive { archive, source } => write!(
                formatter,
                "{archive} 원시 이미지 묶음을 열지 못했습니다: {source}"
            ),
            Self::NoSupportedArchives { path } => write!(
                formatter,
                "지원하는 이미지 리소스(im, kp, sa, sb, sc, sd, se, sf, sg, sh, tm, sw, sx, sy, sz, is)를 찾지 못했습니다: {}",
                path.display()
            ),
        }
    }
}

impl Error for AssetSnapshotError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadIndex { source, .. } => Some(source),
            Self::ParseIndex { source, .. } => Some(source),
            Self::OpenRawArchive { source, .. } => Some(source),
            Self::NotDirectory { .. } | Self::NoSupportedArchives { .. } => None,
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
                "dho-vault-asset-snapshot-test-{}-{number}",
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

    fn entry(
        archive: &str,
        group_code: u32,
        icon_id: u32,
        block_index: u32,
        width: u32,
        height: u32,
    ) -> AssetSnapshotEntry {
        AssetSnapshotEntry::new(archive, group_code, icon_id, block_index, width, height)
    }

    fn write_index(path: &Path, records: &[[u32; 5]]) {
        let mut bytes = Vec::new();
        for value in [records.len() as u32, 1, 1, 1, records.len() as u32, 1, 0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for record in records {
            for value in record {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        fs::write(path, bytes).expect("write test index");
    }

    fn zlib_block(raw: &[u8]) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(raw).expect("write zlib input");
        let compressed = encoder.finish().expect("finish zlib stream");
        let mut block = b"MWC\x1a".to_vec();
        block.extend_from_slice(&(raw.len() as u32).to_le_bytes());
        block.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        block.extend_from_slice(&compressed);
        block
    }

    fn inline_archive(raw_blocks: &[Vec<u8>]) -> Vec<u8> {
        let blocks = raw_blocks
            .iter()
            .map(|raw| zlib_block(raw))
            .collect::<Vec<_>>();
        let mut offset = 4 + blocks.len() * 8;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(blocks.len() as u32).to_le_bytes());
        for block in &blocks {
            bytes.extend_from_slice(&(offset as u32).to_le_bytes());
            bytes.extend_from_slice(&(block.len() as u32).to_le_bytes());
            offset += block.len();
        }
        for block in blocks {
            bytes.extend_from_slice(&block);
        }
        bytes
    }

    #[test]
    fn compares_added_removed_changed_and_unchanged_assets() {
        let previous = AssetSnapshot::new(vec![
            entry("sb", 10, 100, 0, 32, 32),
            entry("sb", 10, 101, 1, 32, 32),
            entry("sc", 20, 900, 4, 64, 64),
        ]);
        let current = AssetSnapshot::new(vec![
            entry("sb", 10, 100, 0, 32, 32),
            entry("sb", 10, 101, 1, 64, 32),
            entry("sb", 10, 102, 2, 32, 32),
        ]);

        let diff = previous.compare_to(&current).expect("compare snapshots");

        assert_eq!(diff.unchanged_count, 1);
        assert_eq!(diff.added, [entry("sb", 10, 102, 2, 32, 32)]);
        assert_eq!(diff.removed, [entry("sc", 20, 900, 4, 64, 64)]);
        assert_eq!(
            diff.changed,
            [AssetSnapshotChange {
                previous: entry("sb", 10, 101, 1, 32, 32),
                current: entry("sb", 10, 101, 1, 64, 32),
            }]
        );
    }

    #[test]
    fn preserves_duplicate_records_and_ignores_input_order() {
        let duplicate = entry("SB", 10, 100, 0, 32, 32);
        let previous = AssetSnapshot::new(vec![
            entry("sc", 20, 200, 1, 64, 64),
            duplicate.clone(),
            duplicate.clone(),
        ]);
        let current = AssetSnapshot::new(vec![
            duplicate.clone(),
            entry("sc", 20, 200, 1, 64, 64),
            duplicate.clone(),
        ]);

        let diff = previous.compare_to(&current).expect("compare snapshots");

        assert_eq!(previous, current);
        assert_eq!(diff.unchanged_count, 3);
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
        assert!(diff.changed.is_empty());
    }

    #[test]
    fn round_trips_the_versioned_snapshot_format() {
        let snapshot = AssetSnapshot::new(vec![entry("SB", 10, 100, 0, 32, 64)]);

        let json = serde_json::to_string(&snapshot).expect("serialize asset snapshot");
        let restored: AssetSnapshot =
            serde_json::from_str(&json).expect("deserialize asset snapshot");

        assert!(json.contains("\"formatVersion\":1"));
        assert!(json.contains("\"groupCode\":10"));
        assert_eq!(restored, snapshot);
    }

    #[test]
    fn reads_old_indexed_entries_without_source_fields() {
        let json = r#"{"formatVersion":1,"assets":[{"archive":"sb","groupCode":1,"iconId":2,"blockIndex":3,"width":4,"height":5}]}"#;

        let restored: AssetSnapshot =
            serde_json::from_str(json).expect("deserialize legacy indexed snapshot");

        assert_eq!(restored.assets, [entry("sb", 1, 2, 3, 4, 5)]);
    }

    #[test]
    fn rejects_an_unsupported_snapshot_format_before_comparison() {
        let previous = AssetSnapshot {
            format_version: ASSET_SNAPSHOT_FORMAT_VERSION + 1,
            assets: vec![],
        };
        let current = AssetSnapshot::new(vec![]);

        assert_eq!(
            previous.compare_to(&current),
            Err(AssetSnapshotCompareError {
                supported: ASSET_SNAPSHOT_FORMAT_VERSION,
                previous: ASSET_SNAPSHOT_FORMAT_VERSION + 1,
                current: ASSET_SNAPSHOT_FORMAT_VERSION,
            })
        );
    }

    #[test]
    fn reads_supported_indexes_into_a_sorted_snapshot() {
        let directory = TestDirectory::new();
        write_index(
            &directory.0.join("sb000000.bin"),
            &[[200, 2, 32, 64, 10], [100, 1, 16, 16, 10]],
        );
        write_index(&directory.0.join("im000000.bin"), &[[0, 0, 128, 128, 1]]);
        write_index(&directory.0.join("is000000.bin"), &[[5, 0, 128, 128, 1]]);
        fs::write(directory.0.join("other000000.bin"), []).expect("write unrelated file");

        let snapshot = inspect_asset_snapshot(&directory.0).expect("inspect asset snapshot");

        assert_eq!(snapshot.format_version, ASSET_SNAPSHOT_FORMAT_VERSION);
        assert_eq!(
            snapshot.assets,
            [
                entry("im", 1, 0, 0, 128, 128),
                entry("is", 1, 5, 0, 128, 128),
                entry("sb", 10, 100, 1, 16, 16),
                entry("sb", 10, 200, 2, 32, 64),
            ]
        );
    }

    #[test]
    fn reads_indexes_from_both_resource_subdirectories() {
        let directory = TestDirectory::new();
        let primary = directory.0.join("0001");
        let secondary = directory.0.join("0002");
        fs::create_dir(&primary).expect("create primary resource directory");
        fs::create_dir(&secondary).expect("create secondary resource directory");
        write_index(&primary.join("sb000000.bin"), &[[100, 1, 16, 16, 10]]);
        write_index(&secondary.join("sw000000.bin"), &[[0, 0, 80, 80, 0]]);

        let snapshot = inspect_asset_snapshot(&directory.0).expect("inspect split resources");

        assert_eq!(
            snapshot.assets,
            [
                entry("sb", 10, 100, 1, 16, 16),
                entry("sw", 0, 0, 0, 80, 80),
            ]
        );
    }

    #[test]
    fn reads_raw_sh_blocks_with_physical_file_identity() {
        let directory = TestDirectory::new();
        let primary = directory.0.join("0001");
        fs::create_dir(&primary).expect("create primary resource directory");
        let mut data = zlib_block(&vec![0x11; 256 * 256]);
        data.extend_from_slice(&zlib_block(&vec![0xcc; 256 * 256]));
        fs::write(primary.join("sh000001.bin"), data).expect("write SH raw archive");

        let snapshot = inspect_asset_snapshot(&directory.0).expect("inspect SH raw snapshot");

        assert_eq!(snapshot.assets.len(), 2);
        assert_eq!(snapshot.assets[0].source_kind, AssetSourceKind::RawBlock);
        assert_eq!(snapshot.assets[0].data_file_number, Some(1));
        assert_eq!(snapshot.assets[0].file_block_index, Some(0));
        assert_eq!(snapshot.assets[1].block_index, 1);
        assert_eq!(snapshot.assets[1].file_block_index, Some(1));
    }

    #[test]
    fn reads_inline_tm_blocks_from_file_zero_with_variable_dimensions() {
        let directory = TestDirectory::new();
        let inline = directory.0.join("0000");
        fs::create_dir(&inline).expect("create inline resource directory");
        fs::write(
            inline.join("tm000000.bin"),
            inline_archive(&[vec![0x11; 180 * 140 * 4], vec![0xcc; 180 * 141 * 4]]),
        )
        .expect("write TM inline archive");

        let snapshot = inspect_asset_snapshot(&directory.0).expect("inspect TM snapshot");

        assert_eq!(snapshot.assets.len(), 2);
        assert_eq!(snapshot.assets[0].archive, "tm");
        assert_eq!(snapshot.assets[0].source_kind, AssetSourceKind::RawBlock);
        assert_eq!(snapshot.assets[0].data_file_number, Some(0));
        assert_eq!(snapshot.assets[0].file_block_index, Some(0));
        assert_eq!(
            (snapshot.assets[0].width, snapshot.assets[0].height),
            (180, 140)
        );
        assert_eq!(
            (snapshot.assets[1].width, snapshot.assets[1].height),
            (180, 141)
        );
    }

    #[test]
    fn preserves_all_three_kp_file_identities() {
        let directory = TestDirectory::new();
        let inline = directory.0.join("0000");
        fs::create_dir(&inline).expect("create inline resource directory");
        fs::write(
            inline.join("kp000000.bin"),
            inline_archive(&[vec![0x11; 48 * 48 * 4], vec![0x22; 48 * 48 * 4]]),
        )
        .expect("write KP base layer");
        fs::write(
            inline.join("kp000010.bin"),
            inline_archive(&[vec![0; 48 * 48 * 4], vec![0x33; 48 * 48 * 4]]),
        )
        .expect("write KP overlay layer");
        fs::write(
            inline.join("kp100000.bin"),
            inline_archive(&[vec![0x44; 256 * 256 * 4]]),
        )
        .expect("write KP overview");

        let snapshot = inspect_asset_snapshot(&directory.0).expect("inspect KP snapshot");

        assert_eq!(snapshot.assets.len(), 5);
        assert_eq!(
            snapshot
                .assets
                .iter()
                .map(|entry| entry.data_file_number)
                .collect::<Vec<_>>(),
            [Some(0), Some(0), Some(10), Some(10), Some(100_000)]
        );
        assert_eq!(snapshot.assets[4].block_index, 4);
        assert_eq!(snapshot.assets[4].file_block_index, Some(0));
        assert_eq!(
            (snapshot.assets[4].width, snapshot.assets[4].height),
            (256, 256)
        );
    }

    #[test]
    fn reports_missing_and_malformed_snapshot_indexes() {
        let missing = TestDirectory::new();
        assert!(matches!(
            inspect_asset_snapshot(missing.0.join("absent")),
            Err(AssetSnapshotError::NotDirectory { .. })
        ));
        assert!(matches!(
            inspect_asset_snapshot(&missing.0),
            Err(AssetSnapshotError::NoSupportedArchives { .. })
        ));

        let malformed = TestDirectory::new();
        fs::write(malformed.0.join("sb000000.bin"), [0; 8]).expect("write malformed index");
        assert!(matches!(
            inspect_asset_snapshot(&malformed.0),
            Err(AssetSnapshotError::ParseIndex { archive, .. }) if archive == "sb"
        ));
    }
}
