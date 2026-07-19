// SPDX-License-Identifier: MPL-2.0

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod archive_session;

use archive_session::{
    ArchiveBandSamples, ArchiveGroupSummary, ArchiveIdBands, AssemblyPreview, CuratorSession,
    GroupIdRanges, RangeSamples,
};
use dho_client::{GameDirectorySummary, inspect_game_directory};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;
use tauri_plugin_dialog::DialogExt;

type SharedCuratorSession = Arc<Mutex<CuratorSession>>;

#[tauri::command]
async fn pick_game_directory(
    app: tauri::AppHandle,
    session: State<'_, SharedCuratorSession>,
) -> Result<Option<GameDirectorySummary>, String> {
    let selection = app
        .dialog()
        .file()
        .set_title("검수할 대항해시대 온라인 설치 폴더 선택")
        .blocking_pick_folder();
    let Some(selection) = selection else {
        return Ok(None);
    };
    let path = selection
        .into_path()
        .map_err(|error| format!("선택한 폴더 경로를 처리하지 못했습니다: {error}"))?;
    let summary = inspect_game_directory(path).map_err(|error| error.to_string())?;

    session
        .lock()
        .map_err(|_| "검수 세션을 열지 못했습니다.".to_owned())?
        .set_resource_directory(PathBuf::from(&summary.resource_directory));

    Ok(Some(summary))
}

#[tauri::command]
async fn list_archive_groups(
    prefix: String,
    session: State<'_, SharedCuratorSession>,
) -> Result<Vec<ArchiveGroupSummary>, String> {
    let session = session.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        session
            .lock()
            .map_err(|_| "검수 세션을 열지 못했습니다.".to_owned())?
            .group_summaries(&prefix)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("그룹 정보를 불러오는 작업이 중단되었습니다: {error}"))?
}

#[tauri::command]
async fn list_archive_id_bands(
    prefix: String,
    session: State<'_, SharedCuratorSession>,
) -> Result<ArchiveIdBands, String> {
    let session = session.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        session
            .lock()
            .map_err(|_| "검수 세션을 열지 못했습니다.".to_owned())?
            .archive_id_bands(&prefix)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("ID 대역을 불러오는 작업이 중단되었습니다: {error}"))?
}

#[tauri::command]
async fn sample_archive_band(
    prefix: String,
    start_icon_id: u32,
    end_icon_id: u32,
    session: State<'_, SharedCuratorSession>,
) -> Result<ArchiveBandSamples, String> {
    let session = session.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        session
            .lock()
            .map_err(|_| "검수 세션을 열지 못했습니다.".to_owned())?
            .archive_band_samples(&prefix, start_icon_id, end_icon_id)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("ID 대역 표본을 만드는 작업이 중단되었습니다: {error}"))?
}

#[tauri::command]
async fn list_group_id_ranges(
    prefix: String,
    group_code: u32,
    session: State<'_, SharedCuratorSession>,
) -> Result<GroupIdRanges, String> {
    let session = session.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        session
            .lock()
            .map_err(|_| "검수 세션을 열지 못했습니다.".to_owned())?
            .group_id_ranges(&prefix, group_code)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("ID 구간을 찾는 작업이 중단되었습니다: {error}"))?
}

#[tauri::command]
async fn sample_archive_range(
    prefix: String,
    group_code: u32,
    start_icon_id: u32,
    end_icon_id: u32,
    session: State<'_, SharedCuratorSession>,
) -> Result<RangeSamples, String> {
    let session = session.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        session
            .lock()
            .map_err(|_| "검수 세션을 열지 못했습니다.".to_owned())?
            .range_samples(&prefix, group_code, start_icon_id, end_icon_id)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("표본 이미지를 만드는 작업이 중단되었습니다: {error}"))?
}

#[tauri::command]
async fn preview_verified_assembly(
    prefix: String,
    block_index: u32,
    session: State<'_, SharedCuratorSession>,
) -> Result<AssemblyPreview, String> {
    let session = session.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        session
            .lock()
            .map_err(|_| "검수 세션을 열지 못했습니다.".to_owned())?
            .assembly_preview(&prefix, block_index)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("조립 미리보기 작업이 중단되었습니다: {error}"))?
}

#[tauri::command]
async fn preview_candidate_assembly(
    prefix: String,
    block_index: u32,
    session: State<'_, SharedCuratorSession>,
) -> Result<AssemblyPreview, String> {
    let session = session.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        session
            .lock()
            .map_err(|_| "검수 세션을 열지 못했습니다.".to_owned())?
            .candidate_assembly_preview(&prefix, block_index)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("조립 후보 미리보기 작업이 중단되었습니다: {error}"))?
}

fn main() {
    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(CuratorSession::default())))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            pick_game_directory,
            list_archive_id_bands,
            sample_archive_band,
            list_archive_groups,
            list_group_id_ranges,
            sample_archive_range,
            preview_verified_assembly,
            preview_candidate_assembly
        ])
        .run(tauri::generate_context!())
        .expect("failed to run DHO Vault Curator");
}
