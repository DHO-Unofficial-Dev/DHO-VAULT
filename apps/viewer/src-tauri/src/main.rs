// SPDX-License-Identifier: MPL-2.0

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dho_client::{
    GameDirectorySummary, VIEWER_CATEGORY_PAGE_SIZE, VerifiedAssetDetail, VerifiedCategoryPage,
    ViewerSession, inspect_game_directory,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;
use tauri_plugin_dialog::DialogExt;

type SharedViewerSession = Arc<Mutex<ViewerSession>>;

#[tauri::command]
async fn pick_game_directory(
    app: tauri::AppHandle,
    session: State<'_, SharedViewerSession>,
) -> Result<Option<GameDirectorySummary>, String> {
    let selection = app
        .dialog()
        .file()
        .set_title("대항해시대 온라인 설치 폴더 선택")
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
        .map_err(|_| "이미지 탐색 세션을 열지 못했습니다.".to_owned())?
        .set_resource_directory(PathBuf::from(&summary.resource_directory));

    Ok(Some(summary))
}

#[tauri::command]
async fn load_verified_category_page(
    path: Vec<String>,
    offset: usize,
    session: State<'_, SharedViewerSession>,
) -> Result<VerifiedCategoryPage, String> {
    let session = session.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        session
            .lock()
            .map_err(|_| "이미지 탐색 세션을 열지 못했습니다.".to_owned())?
            .category_page(&path, offset, VIEWER_CATEGORY_PAGE_SIZE)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("썸네일을 불러오는 작업이 중단되었습니다: {error}"))?
}

#[tauri::command]
async fn load_verified_asset_detail(
    path: Vec<String>,
    archive: String,
    block_index: u32,
    session: State<'_, SharedViewerSession>,
) -> Result<VerifiedAssetDetail, String> {
    let session = session.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        session
            .lock()
            .map_err(|_| "이미지 탐색 세션을 열지 못했습니다.".to_owned())?
            .asset_detail(&path, &archive, block_index)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("상세 이미지를 불러오는 작업이 중단되었습니다: {error}"))?
}

fn main() {
    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(ViewerSession::default())))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            pick_game_directory,
            load_verified_category_page,
            load_verified_asset_detail
        ])
        .run(tauri::generate_context!())
        .expect("failed to run DHO-VAULT viewer");
}
