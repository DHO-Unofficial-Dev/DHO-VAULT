// SPDX-License-Identifier: MPL-2.0

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dho_client::{
    GameDirectorySummary, VIEWER_CATEGORY_PAGE_SIZE, VerifiedAssetDetail, VerifiedAssetPng,
    VerifiedCategoryPage, ViewerSession, inspect_game_directory,
};
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SavedVerifiedPage {
    saved_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PageAssetPlan {
    archive: String,
    icon_id: u32,
    block_index: u32,
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

#[tauri::command]
async fn save_verified_category_page(
    app: tauri::AppHandle,
    path: Vec<String>,
    offset: usize,
    session: State<'_, SharedViewerSession>,
) -> Result<Option<SavedVerifiedPage>, String> {
    let selection = app
        .dialog()
        .file()
        .set_title("현재 페이지 PNG 저장 폴더 선택")
        .blocking_pick_folder();
    let Some(selection) = selection else {
        return Ok(None);
    };
    let destination = selection
        .into_path()
        .map_err(|error| format!("선택한 저장 폴더를 처리하지 못했습니다: {error}"))?;
    let session = session.inner().clone();
    let saved = tauri::async_runtime::spawn_blocking(move || {
        let mut session = session
            .lock()
            .map_err(|_| "이미지 탐색 세션을 열지 못했습니다.".to_owned())?;
        let page = session
            .category_page(&path, offset, VIEWER_CATEGORY_PAGE_SIZE)
            .map_err(|error| error.to_string())?;
        let plans = page
            .items
            .iter()
            .map(|asset| PageAssetPlan {
                archive: asset.archive.clone(),
                icon_id: asset.icon_id,
                block_index: asset.block_index,
                assembled: asset.assembled,
            })
            .collect::<Vec<_>>();
        save_verified_asset_page(&destination, &plans, |plan| {
            session
                .asset_png(&path, &plan.archive, plan.block_index)
                .map_err(|error| error.to_string())
        })?;
        Ok::<_, String>(SavedVerifiedPage {
            saved_count: plans.len(),
        })
    })
    .await
    .map_err(|error| format!("현재 페이지 저장 작업이 중단되었습니다: {error}"))??;

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

fn page_asset_file_name(asset: &PageAssetPlan) -> String {
    let suffix = if asset.assembled { "-assembled" } else { "" };
    format!(
        "dho-vault-{}-icon-{}-block-{:06}{suffix}.png",
        asset.archive.to_ascii_lowercase(),
        asset.icon_id,
        asset.block_index
    )
}

fn save_verified_asset_page<F>(
    directory: &Path,
    assets: &[PageAssetPlan],
    mut load: F,
) -> Result<(), String>
where
    F: FnMut(&PageAssetPlan) -> Result<VerifiedAssetPng, String>,
{
    if !directory.is_dir() {
        return Err(format!(
            "저장할 폴더를 찾지 못했습니다: {}",
            directory.display()
        ));
    }

    let destinations = assets
        .iter()
        .map(|asset| directory.join(page_asset_file_name(asset)))
        .collect::<Vec<_>>();
    if let Some(existing) = destinations.iter().find(|path| path.exists()) {
        return Err(format!(
            "같은 이름의 파일이 이미 있습니다. 다른 폴더를 선택해 주세요: {}",
            existing.display()
        ));
    }

    let mut created = Vec::with_capacity(assets.len());
    for (plan, destination) in assets.iter().zip(&destinations) {
        let asset = match load(plan) {
            Ok(asset) => asset,
            Err(error) => {
                remove_created_files(&created);
                return Err(error);
            }
        };
        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(destination)?;
            created.push(destination.clone());
            file.write_all(&asset.png)
        })();
        if let Err(error) = result {
            remove_created_files(&created);
            return Err(format!(
                "PNG 파일을 저장하지 못했습니다 ({}): {error}",
                destination.display()
            ));
        }
    }

    Ok(())
}

fn remove_created_files(created: &[PathBuf]) {
    for path in created.iter().rev() {
        let _ = fs::remove_file(path);
    }
}

fn main() {
    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(ViewerSession::default())))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            pick_game_directory,
            load_verified_category_page,
            load_verified_asset_detail,
            save_verified_asset_png,
            save_verified_category_page
        ])
        .run(tauri::generate_context!())
        .expect("failed to run DHO-VAULT viewer");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

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

    fn test_plan(block_index: u32) -> PageAssetPlan {
        PageAssetPlan {
            archive: "sb".to_owned(),
            icon_id: 100_100 + block_index,
            block_index,
            assembled: false,
        }
    }

    fn test_asset(plan: &PageAssetPlan) -> VerifiedAssetPng {
        VerifiedAssetPng {
            archive: plan.archive.clone(),
            icon_id: plan.icon_id,
            block_index: plan.block_index,
            width: 1,
            height: 1,
            assembled: false,
            png: b"test-png".to_vec(),
        }
    }

    #[test]
    fn creates_a_safe_default_png_file_name() {
        assert_eq!(default_asset_file_name("SB", 42), "dho-vault-sb-000042.png");
        assert_eq!(
            default_asset_file_name("../unsafe", 7),
            "dho-vault-unsafe-000007.png"
        );
    }

    #[test]
    fn saves_a_page_without_overwriting_existing_files() {
        let directory = test_directory("page-save");
        fs::create_dir(&directory).expect("create test directory");
        let assets = [test_plan(1), test_plan(2)];

        save_verified_asset_page(&directory, &assets, |plan| Ok(test_asset(plan)))
            .expect("save verified asset page");
        assert_eq!(
            fs::read(directory.join(page_asset_file_name(&assets[0]))).expect("read saved PNG"),
            b"test-png"
        );
        assert!(
            save_verified_asset_page(&directory, &assets, |plan| Ok(test_asset(plan))).is_err()
        );

        fs::remove_dir_all(directory).expect("remove test directory");
    }

    #[test]
    fn removes_files_created_before_a_page_save_failure() {
        let directory = test_directory("page-rollback");
        fs::create_dir(&directory).expect("create test directory");
        let asset = test_plan(1);
        let duplicate_assets = [test_plan(1), test_plan(1)];

        let error =
            save_verified_asset_page(&directory, &duplicate_assets, |plan| Ok(test_asset(plan)))
                .unwrap_err();

        assert!(error.contains("저장하지 못했습니다"));
        assert!(!directory.join(page_asset_file_name(&asset)).exists());
        fs::remove_dir_all(directory).expect("remove test directory");
    }

    #[test]
    fn removes_files_created_before_a_later_extraction_failure() {
        let directory = test_directory("extraction-rollback");
        fs::create_dir(&directory).expect("create test directory");
        let assets = [test_plan(1), test_plan(2)];

        let error = save_verified_asset_page(&directory, &assets, |plan| {
            if plan.block_index == 2 {
                Err("두 번째 이미지 추출 실패".to_owned())
            } else {
                Ok(test_asset(plan))
            }
        })
        .unwrap_err();

        assert_eq!(error, "두 번째 이미지 추출 실패");
        assert!(!directory.join(page_asset_file_name(&assets[0])).exists());
        fs::remove_dir_all(directory).expect("remove test directory");
    }
}
