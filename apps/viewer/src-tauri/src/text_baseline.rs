// SPDX-License-Identifier: MPL-2.0

use dho_client::{TextSnapshot, inspect_text_snapshot};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const FILE_NAME: &str = "text-baseline.json";

static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TextBaseline {
    game_directory: PathBuf,
    created_at_unix_seconds: u64,
    snapshot: TextSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TextUpdateState {
    MissingBaseline,
    Unchanged,
    ChangesDetected,
    DifferentDirectory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TextChangeKind {
    Added,
    Changed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextUpdateStatus {
    pub state: TextUpdateState,
    pub baseline_created_at_unix_seconds: Option<u64>,
    pub current_count: usize,
    pub baseline_count: usize,
    pub added_count: usize,
    pub removed_count: usize,
    pub changed_count: usize,
    pub unchanged_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextUpdateKey {
    pub source: String,
    pub id: u32,
    pub change_kind: TextChangeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextUpdateReport {
    pub status: TextUpdateStatus,
    pub visible_changes: Vec<TextUpdateKey>,
}

pub fn load_report(path: &Path, game_directory: &Path) -> Result<TextUpdateReport, String> {
    let current = inspect_text_snapshot(game_directory)
        .map_err(|error| format!("현재 텍스트 목록을 확인하지 못했습니다: {error}"))?;
    let baseline = read(path)?;
    compare_report(baseline.as_ref(), game_directory, &current)
}

pub fn create(path: &Path, game_directory: &Path) -> Result<TextUpdateStatus, String> {
    let current = inspect_text_snapshot(game_directory)
        .map_err(|error| format!("현재 텍스트 목록을 확인하지 못했습니다: {error}"))?;
    let baseline = TextBaseline {
        game_directory: game_directory.to_owned(),
        created_at_unix_seconds: current_unix_seconds()?,
        snapshot: current,
    };
    create_file(path, &baseline)?;
    compare_report(Some(&baseline), game_directory, &baseline.snapshot).map(|report| report.status)
}

pub fn refresh(path: &Path, game_directory: &Path) -> Result<TextUpdateStatus, String> {
    let current = inspect_text_snapshot(game_directory)
        .map_err(|error| format!("현재 텍스트 목록을 확인하지 못했습니다: {error}"))?;
    let baseline = TextBaseline {
        game_directory: game_directory.to_owned(),
        created_at_unix_seconds: current_unix_seconds()?,
        snapshot: current,
    };
    replace_file(path, &baseline)?;
    compare_report(Some(&baseline), game_directory, &baseline.snapshot).map(|report| report.status)
}

fn read(path: &Path) -> Result<Option<TextBaseline>, String> {
    let contents = match fs::read(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("텍스트 기준점을 읽지 못했습니다: {error}")),
    };
    serde_json::from_slice(&contents)
        .map(Some)
        .map_err(|error| format!("텍스트 기준점 파일이 올바르지 않습니다: {error}"))
}

fn create_file(path: &Path, baseline: &TextBaseline) -> Result<(), String> {
    let temporary = write_temporary_file(path, baseline, "tmp")?;
    let result = fs::hard_link(&temporary, path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            "텍스트 기준점이 이미 있어 덮어쓰지 않았습니다.".to_owned()
        } else {
            format!("텍스트 기준점 파일을 확정하지 못했습니다: {error}")
        }
    });
    let _ = fs::remove_file(&temporary);
    result
}

fn replace_file(path: &Path, baseline: &TextBaseline) -> Result<(), String> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => return Err("텍스트 기준점 경로가 파일이 아닙니다.".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err("갱신할 텍스트 기준점이 없습니다.".to_owned());
        }
        Err(error) => return Err(format!("기존 텍스트 기준점을 확인하지 못했습니다: {error}")),
    }

    let temporary = write_temporary_file(path, baseline, "tmp")?;
    let backup = temporary_path(path, "bak")?;
    if let Err(error) = fs::rename(path, &backup) {
        let _ = fs::remove_file(&temporary);
        return Err(format!("기존 텍스트 기준점을 백업하지 못했습니다: {error}"));
    }
    if let Err(error) = fs::rename(&temporary, path) {
        let restore = fs::rename(&backup, path);
        let _ = fs::remove_file(&temporary);
        return match restore {
            Ok(()) => Err(format!(
                "새 텍스트 기준점을 확정하지 못해 기존 기준점을 복구했습니다: {error}"
            )),
            Err(restore_error) => Err(format!(
                "새 텍스트 기준점을 확정하지 못했고 기존 기준점도 복구하지 못했습니다: {error}; 복구 오류: {restore_error}"
            )),
        };
    }
    let _ = fs::remove_file(&backup);
    Ok(())
}

fn write_temporary_file(
    path: &Path,
    baseline: &TextBaseline,
    extension: &str,
) -> Result<PathBuf, String> {
    let parent = path
        .parent()
        .ok_or_else(|| "텍스트 기준점 파일의 상위 폴더를 확인하지 못했습니다.".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("앱 설정 폴더를 만들지 못했습니다: {error}"))?;
    let contents = serde_json::to_vec_pretty(baseline)
        .map_err(|error| format!("텍스트 기준점을 만들지 못했습니다: {error}"))?;
    let temporary = temporary_path(path, extension)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| format!("텍스트 기준점 임시 파일을 만들지 못했습니다: {error}"))?;
    if let Err(error) = file.write_all(&contents).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(&temporary);
        return Err(format!(
            "텍스트 기준점 임시 파일을 쓰지 못했습니다: {error}"
        ));
    }
    Ok(temporary)
}

fn temporary_path(path: &Path, extension: &str) -> Result<PathBuf, String> {
    let parent = path
        .parent()
        .ok_or_else(|| "텍스트 기준점 파일의 상위 폴더를 확인하지 못했습니다.".to_owned())?;
    let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(FILE_NAME);
    Ok(parent.join(format!(
        ".{file_name}.{}.{}.{extension}",
        std::process::id(),
        sequence
    )))
}

fn current_unix_seconds() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| format!("현재 시간을 확인하지 못했습니다: {error}"))
}

fn compare_report(
    baseline: Option<&TextBaseline>,
    game_directory: &Path,
    current: &TextSnapshot,
) -> Result<TextUpdateReport, String> {
    let current_count = current.records.len();
    let Some(baseline) = baseline else {
        return Ok(TextUpdateReport {
            status: TextUpdateStatus {
                state: TextUpdateState::MissingBaseline,
                baseline_created_at_unix_seconds: None,
                current_count,
                baseline_count: 0,
                added_count: 0,
                removed_count: 0,
                changed_count: 0,
                unchanged_count: 0,
            },
            visible_changes: Vec::new(),
        });
    };
    let baseline_count = baseline.snapshot.records.len();
    if baseline.game_directory != game_directory {
        return Ok(TextUpdateReport {
            status: TextUpdateStatus {
                state: TextUpdateState::DifferentDirectory,
                baseline_created_at_unix_seconds: Some(baseline.created_at_unix_seconds),
                current_count,
                baseline_count,
                added_count: 0,
                removed_count: 0,
                changed_count: 0,
                unchanged_count: 0,
            },
            visible_changes: Vec::new(),
        });
    }

    let diff = baseline
        .snapshot
        .compare_to(current)
        .map_err(|error| error.to_string())?;
    let state = if diff.added.is_empty() && diff.removed.is_empty() && diff.changed.is_empty() {
        TextUpdateState::Unchanged
    } else {
        TextUpdateState::ChangesDetected
    };
    let mut visible_changes = diff
        .added
        .iter()
        .map(|record| TextUpdateKey {
            source: record.source.clone(),
            id: record.id,
            change_kind: TextChangeKind::Added,
        })
        .collect::<Vec<_>>();
    visible_changes.extend(diff.changed.iter().map(|change| TextUpdateKey {
        source: change.current.source.clone(),
        id: change.current.id,
        change_kind: TextChangeKind::Changed,
    }));
    visible_changes
        .sort_by(|left, right| left.source.cmp(&right.source).then(left.id.cmp(&right.id)));
    Ok(TextUpdateReport {
        status: TextUpdateStatus {
            state,
            baseline_created_at_unix_seconds: Some(baseline.created_at_unix_seconds),
            current_count,
            baseline_count,
            added_count: diff.added.len(),
            removed_count: diff.removed.len(),
            changed_count: diff.changed.len(),
            unchanged_count: diff.unchanged_count,
        },
        visible_changes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dho_client::TextSnapshotEntry;

    fn snapshot(records: &[(&str, u32, u64)]) -> TextSnapshot {
        TextSnapshot::new(
            records
                .iter()
                .map(|(source, id, content_hash)| TextSnapshotEntry {
                    source: (*source).to_owned(),
                    id: *id,
                    content_hash: *content_hash,
                })
                .collect(),
        )
    }

    #[test]
    fn reports_added_and_changed_text_as_visible_changes() {
        let game_directory = PathBuf::from(r"G:\Games\GV Online KR");
        let baseline = TextBaseline {
            game_directory: game_directory.clone(),
            created_at_unix_seconds: 1_720_000_000,
            snapshot: snapshot(&[("dt000005.bin", 100, 1), ("dt000005.bin", 200, 2)]),
        };
        let current = snapshot(&[("dt000005.bin", 100, 9), ("dt000005.bin", 300, 3)]);
        let report = compare_report(Some(&baseline), &game_directory, &current)
            .expect("compare text baseline");
        assert_eq!(report.status.state, TextUpdateState::ChangesDetected);
        assert_eq!(report.status.added_count, 1);
        assert_eq!(report.status.changed_count, 1);
        assert_eq!(report.status.removed_count, 1);
        assert_eq!(report.visible_changes.len(), 2);
        assert_eq!(report.visible_changes[0].id, 100);
        assert_eq!(
            report.visible_changes[0].change_kind,
            TextChangeKind::Changed
        );
        assert_eq!(report.visible_changes[1].id, 300);
        assert_eq!(report.visible_changes[1].change_kind, TextChangeKind::Added);
    }
}
