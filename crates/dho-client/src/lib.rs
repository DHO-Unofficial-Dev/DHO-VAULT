// SPDX-License-Identifier: MPL-2.0

//! Read-only discovery and inspection of a DHO game client installation.

use dho_catalog::{CatalogRecordKey, VerificationStatus, assembly_plan, classify_record};
use dho_core::{IndexParseError, IndexedArchive};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const SUPPORTED_ARCHIVE_PREFIXES: [&str; 4] = ["sb", "sc", "sd", "is"];

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
