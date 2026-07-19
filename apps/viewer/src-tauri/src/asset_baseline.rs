// SPDX-License-Identifier: MPL-2.0

use dho_client::{AssetSnapshot, inspect_asset_snapshot};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const FILE_NAME: &str = "asset-baseline.json";

static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AssetBaseline {
    resource_directory: PathBuf,
    created_at_unix_seconds: u64,
    snapshot: AssetSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetUpdateState {
    MissingBaseline,
    Unchanged,
    ChangesDetected,
    DifferentDirectory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetUpdateStatus {
    state: AssetUpdateState,
    baseline_created_at_unix_seconds: Option<u64>,
    current_count: usize,
    baseline_count: usize,
    added_count: usize,
    removed_count: usize,
    changed_count: usize,
    unchanged_count: usize,
}

pub fn load(path: &Path, resource_directory: &Path) -> Result<AssetUpdateStatus, String> {
    let current = inspect_asset_snapshot(resource_directory)
        .map_err(|error| format!("현재 자산 목록을 확인하지 못했습니다: {error}"))?;
    let baseline = read(path)?;
    compare(baseline.as_ref(), resource_directory, &current)
}

pub fn create(path: &Path, resource_directory: &Path) -> Result<AssetUpdateStatus, String> {
    let current = inspect_asset_snapshot(resource_directory)
        .map_err(|error| format!("현재 자산 목록을 확인하지 못했습니다: {error}"))?;
    let baseline = AssetBaseline {
        resource_directory: resource_directory.to_owned(),
        created_at_unix_seconds: current_unix_seconds()?,
        snapshot: current,
    };
    create_file(path, &baseline)?;
    compare(Some(&baseline), resource_directory, &baseline.snapshot)
}

fn read(path: &Path) -> Result<Option<AssetBaseline>, String> {
    let contents = match fs::read(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("업데이트 기준점을 읽지 못했습니다: {error}")),
    };
    serde_json::from_slice(&contents)
        .map(Some)
        .map_err(|error| format!("업데이트 기준점 파일이 올바르지 않습니다: {error}"))
}

fn create_file(path: &Path, baseline: &AssetBaseline) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "업데이트 기준점 파일의 상위 폴더를 확인하지 못했습니다.".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("앱 설정 폴더를 만들지 못했습니다: {error}"))?;
    let contents = serde_json::to_vec_pretty(baseline)
        .map_err(|error| format!("업데이트 기준점을 만들지 못했습니다: {error}"))?;
    let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(FILE_NAME);
    let temporary = parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        sequence
    ));

    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("업데이트 기준점 임시 파일을 만들지 못했습니다: {error}"))?;
        file.write_all(&contents)
            .map_err(|error| format!("업데이트 기준점 임시 파일을 쓰지 못했습니다: {error}"))?;
        file.sync_all().map_err(|error| {
            format!("업데이트 기준점 임시 파일을 마무리하지 못했습니다: {error}")
        })?;
        drop(file);

        fs::hard_link(&temporary, path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                "업데이트 기준점이 이미 있어 덮어쓰지 않았습니다.".to_owned()
            } else {
                format!("업데이트 기준점 파일을 확정하지 못했습니다: {error}")
            }
        })
    })();
    let _ = fs::remove_file(&temporary);
    result
}

fn current_unix_seconds() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| format!("현재 시간을 확인하지 못했습니다: {error}"))
}

fn compare(
    baseline: Option<&AssetBaseline>,
    resource_directory: &Path,
    current: &AssetSnapshot,
) -> Result<AssetUpdateStatus, String> {
    let current_count = current.assets.len();
    let Some(baseline) = baseline else {
        return Ok(AssetUpdateStatus {
            state: AssetUpdateState::MissingBaseline,
            baseline_created_at_unix_seconds: None,
            current_count,
            baseline_count: 0,
            added_count: 0,
            removed_count: 0,
            changed_count: 0,
            unchanged_count: 0,
        });
    };
    let baseline_count = baseline.snapshot.assets.len();
    if baseline.resource_directory != resource_directory {
        return Ok(AssetUpdateStatus {
            state: AssetUpdateState::DifferentDirectory,
            baseline_created_at_unix_seconds: Some(baseline.created_at_unix_seconds),
            current_count,
            baseline_count,
            added_count: 0,
            removed_count: 0,
            changed_count: 0,
            unchanged_count: 0,
        });
    }

    let diff = baseline
        .snapshot
        .compare_to(current)
        .map_err(|error| error.to_string())?;
    let state = if diff.added.is_empty() && diff.removed.is_empty() && diff.changed.is_empty() {
        AssetUpdateState::Unchanged
    } else {
        AssetUpdateState::ChangesDetected
    };
    Ok(AssetUpdateStatus {
        state,
        baseline_created_at_unix_seconds: Some(baseline.created_at_unix_seconds),
        current_count,
        baseline_count,
        added_count: diff.added.len(),
        removed_count: diff.removed.len(),
        changed_count: diff.changed.len(),
        unchanged_count: diff.unchanged_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_directory(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "dho-vault-viewer-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    fn snapshot(records: &[(u32, u32, u32, u32, u32)]) -> AssetSnapshot {
        AssetSnapshot::new(
            records
                .iter()
                .map(|(group_code, icon_id, block_index, width, height)| {
                    dho_client::AssetSnapshotEntry::new(
                        "sb",
                        *group_code,
                        *icon_id,
                        *block_index,
                        *width,
                        *height,
                    )
                })
                .collect(),
        )
    }

    #[test]
    fn creates_and_reads_a_baseline_without_overwriting_it() {
        let directory = test_directory("asset-baseline-roundtrip");
        let path = directory.join(FILE_NAME);
        let original = AssetBaseline {
            resource_directory: PathBuf::from(r"G:\Games\GV Online KR\0010\0001"),
            created_at_unix_seconds: 1_720_000_000,
            snapshot: snapshot(&[(10, 100, 0, 32, 32)]),
        };
        let replacement = AssetBaseline {
            resource_directory: PathBuf::from(r"G:\Games\Replacement\0010\0001"),
            created_at_unix_seconds: 1_730_000_000,
            snapshot: snapshot(&[(20, 200, 1, 64, 64)]),
        };

        create_file(&path, &original).expect("create asset baseline");
        assert_eq!(
            read(&path)
                .expect("read asset baseline")
                .expect("saved asset baseline")
                .resource_directory,
            original.resource_directory
        );
        assert!(create_file(&path, &replacement).is_err());
        assert_eq!(
            read(&path)
                .expect("read preserved asset baseline")
                .expect("preserved asset baseline")
                .resource_directory,
            original.resource_directory
        );

        fs::remove_dir_all(directory).expect("remove test directory");
    }

    #[test]
    fn compares_the_current_snapshot_with_the_matching_baseline() {
        let resource_directory = PathBuf::from(r"G:\Games\GV Online KR\0010\0001");
        let baseline = AssetBaseline {
            resource_directory: resource_directory.clone(),
            created_at_unix_seconds: 1_720_000_000,
            snapshot: snapshot(&[
                (10, 100, 0, 32, 32),
                (10, 101, 1, 32, 32),
                (10, 102, 2, 32, 32),
            ]),
        };
        let current = snapshot(&[
            (10, 100, 0, 32, 32),
            (10, 101, 1, 64, 32),
            (10, 103, 3, 32, 32),
        ]);

        assert_eq!(
            compare(Some(&baseline), &resource_directory, &current)
                .expect("compare asset baseline"),
            AssetUpdateStatus {
                state: AssetUpdateState::ChangesDetected,
                baseline_created_at_unix_seconds: Some(1_720_000_000),
                current_count: 3,
                baseline_count: 3,
                added_count: 1,
                removed_count: 1,
                changed_count: 1,
                unchanged_count: 1,
            }
        );
    }

    #[test]
    fn distinguishes_missing_unchanged_and_different_directory_baselines() {
        let resource_directory = PathBuf::from(r"G:\Games\GV Online KR\0010\0001");
        let current = snapshot(&[(10, 100, 0, 32, 32)]);
        let missing = compare(None, &resource_directory, &current).expect("missing baseline");
        assert_eq!(missing.state, AssetUpdateState::MissingBaseline);
        assert_eq!(missing.current_count, 1);

        let matching = AssetBaseline {
            resource_directory: resource_directory.clone(),
            created_at_unix_seconds: 1_720_000_000,
            snapshot: current.clone(),
        };
        let unchanged =
            compare(Some(&matching), &resource_directory, &current).expect("unchanged baseline");
        assert_eq!(unchanged.state, AssetUpdateState::Unchanged);
        assert_eq!(unchanged.unchanged_count, 1);

        let different = AssetBaseline {
            resource_directory: PathBuf::from(r"G:\Games\Other\0010\0001"),
            created_at_unix_seconds: 1_710_000_000,
            snapshot: current.clone(),
        };
        let different =
            compare(Some(&different), &resource_directory, &current).expect("different directory");
        assert_eq!(different.state, AssetUpdateState::DifferentDirectory);
        assert_eq!(different.added_count, 0);
    }

    #[test]
    fn rejects_a_malformed_baseline() {
        let directory = test_directory("malformed-asset-baseline");
        fs::create_dir(&directory).expect("create test directory");
        let path = directory.join(FILE_NAME);
        fs::write(&path, b"not-json").expect("write malformed baseline");

        assert!(read(&path).is_err());

        fs::remove_dir_all(directory).expect("remove test directory");
    }
}
