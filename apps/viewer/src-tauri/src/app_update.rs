// SPDX-License-Identifier: MPL-2.0

use serde::Serialize;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use tauri_plugin_updater::{Update, UpdaterExt};

const STABLE_UPDATE_ENDPOINT: &str =
    "https://github.com/DHO-Unofficial-Dev/DHO-VAULT/releases/latest/download/latest.json";
const RC_UPDATE_ENDPOINT: &str =
    "https://github.com/DHO-Unofficial-Dev/DHO-VAULT/releases/download/updater-rc/latest.json";

fn update_endpoint(is_prerelease: bool) -> &'static str {
    if is_prerelease {
        RC_UPDATE_ENDPOINT
    } else {
        STABLE_UPDATE_ENDPOINT
    }
}

#[derive(Default)]
pub(crate) struct AppUpdateState {
    pending: Mutex<Option<Update>>,
    checking: AtomicBool,
    installing: AtomicBool,
}

#[derive(Debug)]
struct BusyGuard<'a>(&'a AtomicBool);

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn begin_operation<'a>(flag: &'a AtomicBool, message: &str) -> Result<BusyGuard<'a>, String> {
    flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map(|_| BusyGuard(flag))
        .map_err(|_| message.to_owned())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppUpdateCheckResult {
    current_version: String,
    update: Option<AppUpdateMetadata>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppUpdateMetadata {
    version: String,
    notes: Option<String>,
    published_at: Option<String>,
}

impl From<&Update> for AppUpdateMetadata {
    fn from(update: &Update) -> Self {
        Self {
            version: update.version.clone(),
            notes: update.body.clone(),
            published_at: update.date.map(|date| date.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub(crate) enum AppUpdateDownloadEvent {
    #[serde(rename_all = "camelCase")]
    Started {
        content_length: Option<u64>,
    },
    #[serde(rename_all = "camelCase")]
    Progress {
        chunk_length: usize,
    },
    Finished,
}

#[tauri::command]
pub(crate) fn get_app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
pub(crate) async fn check_app_update(
    app: AppHandle,
    state: State<'_, AppUpdateState>,
) -> Result<AppUpdateCheckResult, String> {
    let _guard = begin_operation(&state.checking, "이미 앱 업데이트를 확인하고 있습니다.")?;
    if state.installing.load(Ordering::Acquire) {
        return Err("앱 업데이트를 설치하는 동안에는 다시 확인할 수 없습니다.".to_owned());
    }

    let endpoint = update_endpoint(!app.package_info().version.pre.is_empty())
        .parse()
        .map_err(|error| format!("업데이트 주소가 올바르지 않습니다: {error}"))?;
    let updater = app
        .updater_builder()
        .endpoints(vec![endpoint])
        .and_then(|builder| builder.build())
        .map_err(|error| format!("업데이트 확인을 준비하지 못했습니다: {error}"))?;
    let update = updater
        .check()
        .await
        .map_err(|error| format!("업데이트 정보를 확인하지 못했습니다: {error}"))?;
    let metadata = update.as_ref().map(AppUpdateMetadata::from);

    *state
        .pending
        .lock()
        .map_err(|_| "업데이트 설치 정보를 저장하지 못했습니다.".to_owned())? = update;

    Ok(AppUpdateCheckResult {
        current_version: app.package_info().version.to_string(),
        update: metadata,
    })
}

#[tauri::command]
pub(crate) async fn install_app_update(
    state: State<'_, AppUpdateState>,
    on_event: Channel<AppUpdateDownloadEvent>,
) -> Result<(), String> {
    let _guard = begin_operation(&state.installing, "이미 앱 업데이트를 설치하고 있습니다.")?;
    if state.checking.load(Ordering::Acquire) {
        return Err("앱 업데이트 확인이 끝난 뒤 다시 시도해 주세요.".to_owned());
    }

    let update = state
        .pending
        .lock()
        .map_err(|_| "업데이트 설치 정보를 읽지 못했습니다.".to_owned())?
        .clone()
        .ok_or_else(|| {
            "설치할 앱 업데이트가 없습니다. 먼저 업데이트를 확인해 주세요.".to_owned()
        })?;

    let mut started = false;
    update
        .download_and_install(
            |chunk_length, content_length| {
                if !started {
                    started = true;
                    let _ = on_event.send(AppUpdateDownloadEvent::Started { content_length });
                }
                let _ = on_event.send(AppUpdateDownloadEvent::Progress { chunk_length });
            },
            || {
                let _ = on_event.send(AppUpdateDownloadEvent::Finished);
            },
        )
        .await
        .map_err(|error| format!("앱 업데이트를 설치하지 못했습니다: {error}"))?;

    *state
        .pending
        .lock()
        .map_err(|_| "완료된 업데이트 정보를 정리하지 못했습니다.".to_owned())? = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prevents_overlapping_operations() {
        let flag = AtomicBool::new(false);
        let guard = begin_operation(&flag, "busy").expect("start operation");

        assert!(flag.load(Ordering::Acquire));
        assert_eq!(begin_operation(&flag, "busy").unwrap_err(), "busy");

        drop(guard);
        assert!(!flag.load(Ordering::Acquire));
        assert!(begin_operation(&flag, "busy").is_ok());
    }

    #[test]
    fn selects_an_update_channel_from_the_installed_version() {
        assert_eq!(update_endpoint(false), STABLE_UPDATE_ENDPOINT);
        assert_eq!(update_endpoint(true), RC_UPDATE_ENDPOINT);
    }

    #[test]
    fn serializes_download_events_for_the_viewer_contract() {
        let started = serde_json::to_value(AppUpdateDownloadEvent::Started {
            content_length: Some(4_096),
        })
        .expect("serialize started event");
        let progress = serde_json::to_value(AppUpdateDownloadEvent::Progress {
            chunk_length: 1_024,
        })
        .expect("serialize progress event");

        assert_eq!(started["event"], "started");
        assert_eq!(started["data"]["contentLength"], 4_096);
        assert_eq!(progress["event"], "progress");
        assert_eq!(progress["data"]["chunkLength"], 1_024);
    }
}
