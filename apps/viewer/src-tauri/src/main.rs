// SPDX-License-Identifier: MPL-2.0

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod game_directory;

use game_directory::{GameDirectorySummary, inspect_game_directory};
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
async fn pick_game_directory(
    app: tauri::AppHandle,
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

    inspect_game_directory(path)
        .map(Some)
        .map_err(|error| error.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![pick_game_directory])
        .run(tauri::generate_context!())
        .expect("failed to run DHO-VAULT viewer");
}
