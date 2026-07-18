// SPDX-License-Identifier: MPL-2.0

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dho_client::{
    GameDirectorySummary, VIEWER_CATEGORY_PAGE_SIZE, VerifiedAssetDetail, VerifiedCategoryPage,
    ViewerSession, inspect_game_directory,
};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;
use tauri_plugin_dialog::DialogExt;

type SharedViewerSession = Arc<Mutex<ViewerSession>>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SavedVerifiedAsset {
    file_name: String,
    width: u32,
    height: u32,
    assembled: bool,
}

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

#[tauri::command]
async fn save_verified_asset_png(
    app: tauri::AppHandle,
    path: Vec<String>,
    archive: String,
    block_index: u32,
    session: State<'_, SharedViewerSession>,
) -> Result<Option<SavedVerifiedAsset>, String> {
    let file_name = default_asset_file_name(&archive, block_index);
    let selection = app
        .dialog()
        .file()
        .set_title("PNG 이미지 저장")
        .add_filter("PNG 이미지", &["png"])
        .set_file_name(file_name)
        .blocking_save_file();
    let Some(selection) = selection else {
        return Ok(None);
    };
    let mut destination = selection
        .into_path()
        .map_err(|error| format!("선택한 저장 경로를 처리하지 못했습니다: {error}"))?;
    match destination
        .extension()
        .and_then(|extension| extension.to_str())
    {
        None => {
            destination.set_extension("png");
        }
        Some(extension) if extension.eq_ignore_ascii_case("png") => {}
        Some(_) => return Err("파일 이름의 확장자를 .png로 지정해 주세요.".to_owned()),
    }
    let saved_file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("image.png")
        .to_owned();
    let session = session.inner().clone();
    let saved = tauri::async_runtime::spawn_blocking(move || {
        let asset = session
            .lock()
            .map_err(|_| "이미지 탐색 세션을 열지 못했습니다.".to_owned())?
            .asset_png(&path, &archive, block_index)
            .map_err(|error| error.to_string())?;
        fs::write(&destination, &asset.png).map_err(|error| {
            format!(
                "PNG 파일을 저장하지 못했습니다 ({}): {error}",
                destination.display()
            )
        })?;
        Ok::<_, String>(SavedVerifiedAsset {
            file_name: saved_file_name,
            width: asset.width,
            height: asset.height,
            assembled: asset.assembled,
        })
    })
    .await
    .map_err(|error| format!("PNG 저장 작업이 중단되었습니다: {error}"))??;

    Ok(Some(saved))
}

fn default_asset_file_name(archive: &str, block_index: u32) -> String {
    let safe_archive = archive
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(8)
        .collect::<String>()
        .to_ascii_lowercase();
    let safe_archive = if safe_archive.is_empty() {
        "image"
    } else {
        &safe_archive
    };
    format!("dho-vault-{safe_archive}-{block_index:06}.png")
}

fn main() {
    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(ViewerSession::default())))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            pick_game_directory,
            load_verified_category_page,
            load_verified_asset_detail,
            save_verified_asset_png
        ])
        .run(tauri::generate_context!())
        .expect("failed to run DHO-VAULT viewer");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_a_safe_default_png_file_name() {
        assert_eq!(default_asset_file_name("SB", 42), "dho-vault-sb-000042.png");
        assert_eq!(
            default_asset_file_name("../unsafe", 7),
            "dho-vault-unsafe-000007.png"
        );
    }
}
