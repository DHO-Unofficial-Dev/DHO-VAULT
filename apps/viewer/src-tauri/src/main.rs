// SPDX-License-Identifier: MPL-2.0

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod asset_baseline;

use asset_baseline::AssetUpdateStatus;
use dho_client::{
    GameDirectorySummary, VIEWER_CATEGORY_PAGE_SIZE, VerifiedAssetDetail, VerifiedAssetPng,
    VerifiedAssetSearchItem, VerifiedAssetSearchPage, VerifiedCategoryAsset, VerifiedCategoryPage,
    VerifiedSearchAsset, VerifiedUpdatePage, ViewerSession, inspect_game_directory,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;

type SharedViewerSession = Arc<Mutex<ViewerSession>>;
type SharedCategoryExportManager = Arc<CategoryExportManager>;

const VIEWER_PREFERENCES_FILE_NAME: &str = "viewer-preferences.json";

#[derive(Debug, Deserialize, Serialize)]
struct ViewerPreferences {
    game_directory: PathBuf,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenedGameDirectory {
    summary: GameDirectorySummary,
    warning: Option<String>,
}

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

impl From<&VerifiedCategoryAsset> for PageAssetPlan {
    fn from(asset: &VerifiedCategoryAsset) -> Self {
        Self {
            archive: asset.archive().to_owned(),
            icon_id: asset.icon_id(),
            block_index: asset.block_index(),
            assembled: asset.assembled(),
        }
    }
}

impl From<&VerifiedSearchAsset> for PageAssetPlan {
    fn from(asset: &VerifiedSearchAsset) -> Self {
        Self {
            archive: asset.archive().to_owned(),
            icon_id: asset.icon_id(),
            block_index: asset.block_index(),
            assembled: asset.assembled(),
        }
    }
}

#[derive(Debug, Clone)]
enum ExportAsset {
    Category(VerifiedCategoryAsset),
    Search(VerifiedSearchAsset),
}

impl ExportAsset {
    fn plan(&self) -> PageAssetPlan {
        match self {
            Self::Category(asset) => PageAssetPlan::from(asset),
            Self::Search(asset) => PageAssetPlan::from(asset),
        }
    }

    fn extract_png(&self, session: &mut ViewerSession) -> Result<VerifiedAssetPng, String> {
        match self {
            Self::Category(asset) => session.category_asset_png(asset),
            Self::Search(asset) => session.search_asset_png(asset),
        }
        .map_err(|error| error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SearchPageAssetPlan {
    path: Vec<String>,
    asset: PageAssetPlan,
}

impl From<&VerifiedAssetSearchItem> for SearchPageAssetPlan {
    fn from(item: &VerifiedAssetSearchItem) -> Self {
        Self {
            path: item.path.clone(),
            asset: PageAssetPlan {
                archive: item.thumbnail.archive.clone(),
                icon_id: item.thumbnail.icon_id,
                block_index: item.thumbnail.block_index,
                assembled: item.thumbnail.assembled,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum CategoryExportState {
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct CategoryExportStatus {
    job_id: u64,
    state: CategoryExportState,
    completed_count: usize,
    total_count: usize,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StartedCategoryExport {
    job_id: u64,
    total_count: usize,
}

#[derive(Debug)]
struct CategoryExportJob {
    job_id: u64,
    cancel: Arc<AtomicBool>,
    status: Arc<Mutex<CategoryExportStatus>>,
}

#[derive(Debug)]
struct CategoryExportControl {
    job_id: u64,
    cancel: Arc<AtomicBool>,
    status: Arc<Mutex<CategoryExportStatus>>,
}

#[derive(Debug, Default)]
struct CategoryExportManager {
    next_job_id: AtomicU64,
    current: Mutex<Option<CategoryExportJob>>,
}

impl CategoryExportManager {
    fn ensure_idle(&self) -> Result<(), String> {
        let current = self
            .current
            .lock()
            .map_err(|_| "전체 저장 작업 상태를 확인하지 못했습니다.".to_owned())?;
        if let Some(job) = current.as_ref() {
            let status = job
                .status
                .lock()
                .map_err(|_| "전체 저장 진행 상태를 확인하지 못했습니다.".to_owned())?;
            if status.state == CategoryExportState::Running {
                return Err("이미 전체 저장 작업이 진행 중입니다.".to_owned());
            }
        }
        Ok(())
    }

    fn start(&self, total_count: usize) -> Result<CategoryExportControl, String> {
        let mut current = self
            .current
            .lock()
            .map_err(|_| "전체 저장 작업 상태를 시작하지 못했습니다.".to_owned())?;
        if let Some(job) = current.as_ref() {
            let status = job
                .status
                .lock()
                .map_err(|_| "전체 저장 진행 상태를 확인하지 못했습니다.".to_owned())?;
            if status.state == CategoryExportState::Running {
                return Err("이미 전체 저장 작업이 진행 중입니다.".to_owned());
            }
        }
        let job_id = self.next_job_id.fetch_add(1, Ordering::Relaxed) + 1;
        let cancel = Arc::new(AtomicBool::new(false));
        let status = Arc::new(Mutex::new(CategoryExportStatus {
            job_id,
            state: CategoryExportState::Running,
            completed_count: 0,
            total_count,
            error: None,
        }));
        *current = Some(CategoryExportJob {
            job_id,
            cancel: cancel.clone(),
            status: status.clone(),
        });
        Ok(CategoryExportControl {
            job_id,
            cancel,
            status,
        })
    }

    fn status(&self, job_id: u64) -> Result<CategoryExportStatus, String> {
        let current = self
            .current
            .lock()
            .map_err(|_| "전체 저장 작업 상태를 확인하지 못했습니다.".to_owned())?;
        let job = current
            .as_ref()
            .filter(|job| job.job_id == job_id)
            .ok_or_else(|| "전체 저장 작업을 찾지 못했습니다.".to_owned())?;
        let status = job
            .status
            .lock()
            .map_err(|_| "전체 저장 진행 상태를 확인하지 못했습니다.".to_owned())?
            .clone();
        Ok(status)
    }

    fn cancel(&self, job_id: u64) -> Result<bool, String> {
        let current = self
            .current
            .lock()
            .map_err(|_| "전체 저장 작업 상태를 확인하지 못했습니다.".to_owned())?;
        let job = current
            .as_ref()
            .filter(|job| job.job_id == job_id)
            .ok_or_else(|| "전체 저장 작업을 찾지 못했습니다.".to_owned())?;
        let running = job
            .status
            .lock()
            .map_err(|_| "전체 저장 진행 상태를 확인하지 못했습니다.".to_owned())?
            .state
            == CategoryExportState::Running;
        if running {
            job.cancel.store(true, Ordering::Relaxed);
        }
        Ok(running)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BatchSaveOutcome {
    Completed,
    Cancelled,
}

fn viewer_preferences_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join(VIEWER_PREFERENCES_FILE_NAME))
        .map_err(|error| format!("앱 설정 폴더를 확인하지 못했습니다: {error}"))
}

fn viewer_asset_baseline_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join(asset_baseline::FILE_NAME))
        .map_err(|error| format!("앱 설정 폴더를 확인하지 못했습니다: {error}"))
}

fn read_saved_game_directory(preferences_path: &Path) -> Result<Option<PathBuf>, String> {
    let contents = match fs::read(preferences_path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!("마지막 게임 폴더 설정을 읽지 못했습니다: {error}"));
        }
    };
    let preferences: ViewerPreferences = serde_json::from_slice(&contents)
        .map_err(|error| format!("마지막 게임 폴더 설정이 올바르지 않습니다: {error}"))?;
    Ok(Some(preferences.game_directory))
}

fn write_saved_game_directory(
    preferences_path: &Path,
    game_directory: &Path,
) -> Result<(), String> {
    let parent = preferences_path
        .parent()
        .ok_or_else(|| "앱 설정 파일의 상위 폴더를 확인하지 못했습니다".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("앱 설정 폴더를 만들지 못했습니다: {error}"))?;
    let contents = serde_json::to_vec_pretty(&ViewerPreferences {
        game_directory: game_directory.to_path_buf(),
    })
    .map_err(|error| format!("마지막 게임 폴더 설정을 만들지 못했습니다: {error}"))?;
    fs::write(preferences_path, contents)
        .map_err(|error| format!("마지막 게임 폴더 설정을 저장하지 못했습니다: {error}"))
}

async fn inspect_directory(path: PathBuf) -> Result<GameDirectorySummary, String> {
    tauri::async_runtime::spawn_blocking(move || inspect_game_directory(path))
        .await
        .map_err(|error| format!("게임 폴더 확인 작업을 마치지 못했습니다: {error}"))?
        .map_err(|error| error.to_string())
}

fn open_viewer_session(
    session: &State<'_, SharedViewerSession>,
    summary: &GameDirectorySummary,
) -> Result<(), String> {
    session
        .lock()
        .map_err(|_| "이미지 탐색 세션을 열지 못했습니다".to_owned())?
        .set_resource_directory(PathBuf::from(&summary.resource_directory));
    Ok(())
}

fn selected_resource_directory(
    session: &State<'_, SharedViewerSession>,
) -> Result<PathBuf, String> {
    session
        .lock()
        .map_err(|_| "이미지 탐색 세션을 열지 못했습니다.".to_owned())?
        .resource_directory()
        .map(Path::to_path_buf)
        .ok_or_else(|| "먼저 게임 폴더를 선택해 주세요.".to_owned())
}

async fn calculate_asset_update_status(
    baseline_path: PathBuf,
    resource_directory: PathBuf,
) -> Result<AssetUpdateStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        asset_baseline::load(&baseline_path, &resource_directory)
    })
    .await
    .map_err(|error| format!("업데이트 비교 작업이 중단되었습니다: {error}"))?
}

#[tauri::command]
async fn load_saved_game_directory(
    app: tauri::AppHandle,
    session: State<'_, SharedViewerSession>,
) -> Result<Option<OpenedGameDirectory>, String> {
    let preferences_path = viewer_preferences_path(&app)?;
    let Some(path) = read_saved_game_directory(&preferences_path)? else {
        return Ok(None);
    };
    let summary = inspect_directory(path).await?;
    open_viewer_session(&session, &summary)?;
    Ok(Some(OpenedGameDirectory {
        summary,
        warning: None,
    }))
}

#[tauri::command]
async fn pick_game_directory(
    app: tauri::AppHandle,
    session: State<'_, SharedViewerSession>,
) -> Result<Option<OpenedGameDirectory>, String> {
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

    let summary = inspect_directory(path.clone()).await?;
    open_viewer_session(&session, &summary)?;

    let warning = viewer_preferences_path(&app)
        .and_then(|preferences_path| write_saved_game_directory(&preferences_path, &path))
        .err();

    Ok(Some(OpenedGameDirectory { summary, warning }))
}

#[tauri::command]
async fn load_asset_update_status(
    app: tauri::AppHandle,
    session: State<'_, SharedViewerSession>,
) -> Result<AssetUpdateStatus, String> {
    let baseline_path = viewer_asset_baseline_path(&app)?;
    let resource_directory = selected_resource_directory(&session)?;
    calculate_asset_update_status(baseline_path, resource_directory).await
}

#[tauri::command]
async fn create_asset_update_baseline(
    app: tauri::AppHandle,
    session: State<'_, SharedViewerSession>,
) -> Result<AssetUpdateStatus, String> {
    let baseline_path = viewer_asset_baseline_path(&app)?;
    let resource_directory = selected_resource_directory(&session)?;
    tauri::async_runtime::spawn_blocking(move || {
        asset_baseline::create(&baseline_path, &resource_directory)
    })
    .await
    .map_err(|error| format!("업데이트 기준점 저장 작업이 중단되었습니다: {error}"))?
}

#[tauri::command]
async fn load_verified_update_page(
    app: tauri::AppHandle,
    offset: usize,
    session: State<'_, SharedViewerSession>,
) -> Result<VerifiedUpdatePage, String> {
    let baseline_path = viewer_asset_baseline_path(&app)?;
    let resource_directory = selected_resource_directory(&session)?;
    let session = session.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let report = asset_baseline::load_report(&baseline_path, &resource_directory)?;
        session
            .lock()
            .map_err(|_| "이미지 탐색 세션을 열지 못했습니다.".to_owned())?
            .update_page(&report.added_assets, offset, VIEWER_CATEGORY_PAGE_SIZE)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("신규 이미지 목록 작업이 중단되었습니다: {error}"))?
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
async fn load_verified_asset_search_page(
    query: String,
    offset: usize,
    session: State<'_, SharedViewerSession>,
) -> Result<VerifiedAssetSearchPage, String> {
    let session = session.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        session
            .lock()
            .map_err(|_| "이미지 탐색 세션을 열지 못했습니다.".to_owned())?
            .search_page(&query, offset, VIEWER_CATEGORY_PAGE_SIZE)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("검색 결과를 불러오는 작업이 중단되었습니다: {error}"))?
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

#[tauri::command]
async fn save_verified_search_page(
    app: tauri::AppHandle,
    query: String,
    offset: usize,
    session: State<'_, SharedViewerSession>,
) -> Result<Option<SavedVerifiedPage>, String> {
    let selection = app
        .dialog()
        .file()
        .set_title("검색 결과 현재 페이지 PNG 저장 폴더 선택")
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
            .search_page(&query, offset, VIEWER_CATEGORY_PAGE_SIZE)
            .map_err(|error| error.to_string())?;
        let assets = page
            .items
            .iter()
            .map(SearchPageAssetPlan::from)
            .collect::<Vec<_>>();
        save_verified_search_page_assets(&destination, &assets, |asset| {
            session
                .asset_png(&asset.path, &asset.asset.archive, asset.asset.block_index)
                .map_err(|error| error.to_string())
        })?;
        Ok::<_, String>(SavedVerifiedPage {
            saved_count: assets.len(),
        })
    })
    .await
    .map_err(|error| format!("검색 결과 현재 페이지 저장 작업이 중단되었습니다: {error}"))??;

    Ok(Some(saved))
}

#[tauri::command]
async fn start_verified_category_export(
    app: tauri::AppHandle,
    path: Vec<String>,
    session: State<'_, SharedViewerSession>,
    exports: State<'_, SharedCategoryExportManager>,
) -> Result<Option<StartedCategoryExport>, String> {
    exports.ensure_idle()?;
    let selection = app
        .dialog()
        .file()
        .set_title("카테고리 전체 PNG 저장 폴더 선택")
        .blocking_pick_folder();
    let Some(selection) = selection else {
        return Ok(None);
    };
    let destination = selection
        .into_path()
        .map_err(|error| format!("선택한 저장 폴더를 처리하지 못했습니다: {error}"))?;

    let session = session.inner().clone();
    let manifest_session = session.clone();
    let assets = tauri::async_runtime::spawn_blocking(move || {
        manifest_session
            .lock()
            .map_err(|_| "이미지 탐색 세션을 열지 못했습니다.".to_owned())?
            .category_assets(&path)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("전체 저장 목록 확인 작업이 중단되었습니다: {error}"))??;
    let assets = assets
        .into_iter()
        .map(ExportAsset::Category)
        .collect::<Vec<_>>();
    let plans = assets.iter().map(ExportAsset::plan).collect::<Vec<_>>();
    preflight_asset_batch(&destination, &plans)?;

    let exports = exports.inner().clone();
    let total_count = plans.len();
    let control = exports.start(total_count)?;
    let job_id = control.job_id;
    std::mem::drop(tauri::async_runtime::spawn_blocking(move || {
        run_asset_export(
            session,
            destination,
            assets,
            plans,
            control.cancel,
            control.status,
        );
    }));

    Ok(Some(StartedCategoryExport {
        job_id,
        total_count,
    }))
}

#[tauri::command]
async fn start_verified_search_export(
    app: tauri::AppHandle,
    query: String,
    session: State<'_, SharedViewerSession>,
    exports: State<'_, SharedCategoryExportManager>,
) -> Result<Option<StartedCategoryExport>, String> {
    exports.ensure_idle()?;
    let selection = app
        .dialog()
        .file()
        .set_title("검색 결과 전체 PNG 저장 폴더 선택")
        .blocking_pick_folder();
    let Some(selection) = selection else {
        return Ok(None);
    };
    let destination = selection
        .into_path()
        .map_err(|error| format!("선택한 저장 폴더를 처리하지 못했습니다: {error}"))?;

    let session = session.inner().clone();
    let manifest_session = session.clone();
    let assets = tauri::async_runtime::spawn_blocking(move || {
        manifest_session
            .lock()
            .map_err(|_| "이미지 탐색 세션을 열지 못했습니다.".to_owned())?
            .search_assets(&query)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("검색 결과 전체 저장 목록 확인 작업이 중단되었습니다: {error}"))??;
    let assets = assets
        .into_iter()
        .map(ExportAsset::Search)
        .collect::<Vec<_>>();
    let plans = assets.iter().map(ExportAsset::plan).collect::<Vec<_>>();
    preflight_asset_batch(&destination, &plans)?;

    let exports = exports.inner().clone();
    let total_count = plans.len();
    let control = exports.start(total_count)?;
    let job_id = control.job_id;
    std::mem::drop(tauri::async_runtime::spawn_blocking(move || {
        run_asset_export(
            session,
            destination,
            assets,
            plans,
            control.cancel,
            control.status,
        );
    }));

    Ok(Some(StartedCategoryExport {
        job_id,
        total_count,
    }))
}

#[tauri::command]
fn get_verified_asset_export_status(
    job_id: u64,
    exports: State<'_, SharedCategoryExportManager>,
) -> Result<CategoryExportStatus, String> {
    exports.status(job_id)
}

#[tauri::command]
fn cancel_verified_asset_export(
    job_id: u64,
    exports: State<'_, SharedCategoryExportManager>,
) -> Result<bool, String> {
    exports.cancel(job_id)
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

fn preflight_asset_batch(
    directory: &Path,
    assets: &[PageAssetPlan],
) -> Result<Vec<PathBuf>, String> {
    if !directory.is_dir() {
        return Err(format!(
            "저장할 폴더를 찾지 못했습니다: {}",
            directory.display()
        ));
    }

    let mut unique_names = HashSet::with_capacity(assets.len());
    let mut destinations = Vec::with_capacity(assets.len());
    for asset in assets {
        let file_name = page_asset_file_name(asset);
        if !unique_names.insert(file_name.clone()) {
            return Err(format!(
                "저장 목록에 중복된 파일 이름이 있습니다: {file_name}"
            ));
        }
        destinations.push(directory.join(file_name));
    }
    if let Some(existing) = destinations.iter().find(|path| path.exists()) {
        return Err(format!(
            "같은 이름의 파일이 이미 있습니다. 다른 폴더를 선택해 주세요: {}",
            existing.display()
        ));
    }

    Ok(destinations)
}

fn save_asset_batch<F, P>(
    directory: &Path,
    assets: &[PageAssetPlan],
    cancel: &AtomicBool,
    mut load: F,
    mut progress: P,
) -> Result<BatchSaveOutcome, String>
where
    F: FnMut(&PageAssetPlan) -> Result<VerifiedAssetPng, String>,
    P: FnMut(usize, usize) -> Result<(), String>,
{
    let destinations = preflight_asset_batch(directory, assets)?;
    let mut created = Vec::with_capacity(assets.len());
    for (index, (plan, destination)) in assets.iter().zip(&destinations).enumerate() {
        if cancel.load(Ordering::Relaxed) {
            remove_created_files(&created);
            return Ok(BatchSaveOutcome::Cancelled);
        }
        let asset = match load(plan) {
            Ok(asset) => asset,
            Err(error) => {
                remove_created_files(&created);
                return Err(error);
            }
        };
        if cancel.load(Ordering::Relaxed) {
            remove_created_files(&created);
            return Ok(BatchSaveOutcome::Cancelled);
        }
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
        if let Err(error) = progress(index + 1, assets.len()) {
            remove_created_files(&created);
            return Err(error);
        }
    }

    Ok(BatchSaveOutcome::Completed)
}

fn save_verified_asset_page<F>(
    directory: &Path,
    assets: &[PageAssetPlan],
    mut load: F,
) -> Result<(), String>
where
    F: FnMut(&PageAssetPlan) -> Result<VerifiedAssetPng, String>,
{
    let cancel = AtomicBool::new(false);
    match save_asset_batch(directory, assets, &cancel, &mut load, |_, _| Ok(()))? {
        BatchSaveOutcome::Completed => Ok(()),
        BatchSaveOutcome::Cancelled => {
            Err("현재 페이지 저장이 예기치 않게 취소되었습니다.".to_owned())
        }
    }
}

fn save_verified_search_page_assets<F>(
    directory: &Path,
    assets: &[SearchPageAssetPlan],
    mut load: F,
) -> Result<(), String>
where
    F: FnMut(&SearchPageAssetPlan) -> Result<VerifiedAssetPng, String>,
{
    let plans = assets
        .iter()
        .map(|asset| asset.asset.clone())
        .collect::<Vec<_>>();
    let mut asset_index = 0;
    save_verified_asset_page(directory, &plans, |plan| {
        let asset = assets
            .get(asset_index)
            .ok_or_else(|| "검색 결과 저장 순서가 일치하지 않습니다.".to_owned())?;
        asset_index += 1;
        if &asset.asset != plan {
            return Err("검색 결과 저장 계획이 일치하지 않습니다.".to_owned());
        }
        load(asset)
    })
}

fn run_asset_export(
    session: SharedViewerSession,
    destination: PathBuf,
    assets: Vec<ExportAsset>,
    plans: Vec<PageAssetPlan>,
    cancel: Arc<AtomicBool>,
    status: Arc<Mutex<CategoryExportStatus>>,
) {
    let result = (|| {
        if assets.len() != plans.len() {
            return Err("전체 저장 목록의 자산 수가 일치하지 않습니다.".to_owned());
        }
        let mut session = session
            .lock()
            .map_err(|_| "이미지 탐색 세션을 열지 못했습니다.".to_owned())?;
        let mut asset_index = 0;
        save_asset_batch(
            &destination,
            &plans,
            &cancel,
            |_| {
                let asset = assets
                    .get(asset_index)
                    .ok_or_else(|| "전체 저장 자산 순서가 일치하지 않습니다.".to_owned())?;
                asset_index += 1;
                asset.extract_png(&mut session)
            },
            |completed_count, total_count| {
                let mut current = status
                    .lock()
                    .map_err(|_| "전체 저장 진행 상태를 갱신하지 못했습니다.".to_owned())?;
                current.completed_count = completed_count;
                current.total_count = total_count;
                Ok(())
            },
        )
    })();

    if let Ok(mut current) = status.lock() {
        match result {
            Ok(BatchSaveOutcome::Completed) => {
                current.state = CategoryExportState::Completed;
                current.completed_count = current.total_count;
            }
            Ok(BatchSaveOutcome::Cancelled) => {
                current.state = CategoryExportState::Cancelled;
                current.completed_count = 0;
            }
            Err(error) => {
                current.state = CategoryExportState::Failed;
                current.completed_count = 0;
                current.error = Some(error);
            }
        }
    }
}

fn remove_created_files(created: &[PathBuf]) {
    for path in created.iter().rev() {
        let _ = fs::remove_file(path);
    }
}

fn main() {
    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(ViewerSession::default())))
        .manage(Arc::new(CategoryExportManager::default()))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            load_saved_game_directory,
            pick_game_directory,
            load_asset_update_status,
            create_asset_update_baseline,
            load_verified_update_page,
            load_verified_category_page,
            load_verified_asset_search_page,
            load_verified_asset_detail,
            save_verified_asset_png,
            save_verified_category_page,
            save_verified_search_page,
            start_verified_category_export,
            start_verified_search_export,
            get_verified_asset_export_status,
            cancel_verified_asset_export
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

    fn test_search_plan(path: &[&str], block_index: u32) -> SearchPageAssetPlan {
        let item = VerifiedAssetSearchItem {
            path: path.iter().map(|segment| (*segment).to_owned()).collect(),
            thumbnail: dho_client::VerifiedAssetThumbnail {
                archive: "sb".to_owned(),
                icon_id: 100_100 + block_index,
                block_index,
                source_width: 1,
                source_height: 1,
                thumbnail_width: 1,
                thumbnail_height: 1,
                assembled: false,
                thumbnail_data_url: String::new(),
            },
        };
        SearchPageAssetPlan::from(&item)
    }

    #[test]
    fn missing_viewer_preferences_have_no_saved_directory() {
        let directory = test_directory("missing-preferences");
        let preferences_path = directory.join(VIEWER_PREFERENCES_FILE_NAME);

        assert_eq!(
            read_saved_game_directory(&preferences_path).expect("read missing preferences"),
            None
        );
    }

    #[test]
    fn saves_and_reads_the_last_game_directory() {
        let directory = test_directory("preferences-roundtrip");
        let preferences_path = directory.join(VIEWER_PREFERENCES_FILE_NAME);
        let game_directory = PathBuf::from(r"G:\Games\GV Online KR");

        write_saved_game_directory(&preferences_path, &game_directory)
            .expect("write viewer preferences");
        assert_eq!(
            read_saved_game_directory(&preferences_path).expect("read viewer preferences"),
            Some(game_directory)
        );

        fs::remove_dir_all(directory).expect("remove test directory");
    }

    #[test]
    fn overwrites_the_previous_game_directory() {
        let directory = test_directory("preferences-overwrite");
        let preferences_path = directory.join(VIEWER_PREFERENCES_FILE_NAME);
        let replacement = PathBuf::from(r"G:\Games\Replacement");

        write_saved_game_directory(&preferences_path, Path::new(r"G:\Games\Original"))
            .expect("write original preferences");
        write_saved_game_directory(&preferences_path, &replacement)
            .expect("overwrite viewer preferences");
        assert_eq!(
            read_saved_game_directory(&preferences_path).expect("read overwritten preferences"),
            Some(replacement)
        );

        fs::remove_dir_all(directory).expect("remove test directory");
    }

    #[test]
    fn rejects_malformed_viewer_preferences() {
        let directory = test_directory("malformed-preferences");
        fs::create_dir(&directory).expect("create test directory");
        let preferences_path = directory.join(VIEWER_PREFERENCES_FILE_NAME);
        fs::write(&preferences_path, b"not-json").expect("write malformed preferences");

        assert!(read_saved_game_directory(&preferences_path).is_err());

        fs::remove_dir_all(directory).expect("remove test directory");
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
    fn saves_search_page_assets_in_result_order_with_their_paths() {
        let directory = test_directory("search-page-save");
        fs::create_dir(&directory).expect("create test directory");
        let assets = [
            test_search_plan(&["장비", "방어구", "머리"], 1),
            test_search_plan(&["아이템", "소비품"], 2),
        ];
        let mut loaded_paths = Vec::new();

        save_verified_search_page_assets(&directory, &assets, |asset| {
            loaded_paths.push(asset.path.clone());
            Ok(test_asset(&asset.asset))
        })
        .expect("save verified search page");

        assert_eq!(
            loaded_paths,
            assets
                .iter()
                .map(|asset| asset.path.clone())
                .collect::<Vec<_>>()
        );
        assert!(
            assets
                .iter()
                .all(|asset| directory.join(page_asset_file_name(&asset.asset)).is_file())
        );
        fs::remove_dir_all(directory).expect("remove test directory");
    }

    #[test]
    fn rejects_duplicate_file_names_before_writing() {
        let directory = test_directory("page-rollback");
        fs::create_dir(&directory).expect("create test directory");
        let asset = test_plan(1);
        let duplicate_assets = [test_plan(1), test_plan(1)];

        let error =
            save_verified_asset_page(&directory, &duplicate_assets, |plan| Ok(test_asset(plan)))
                .unwrap_err();

        assert!(error.contains("중복된 파일 이름"));
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

    #[test]
    fn removes_created_files_when_a_batch_is_cancelled() {
        let directory = test_directory("cancel-rollback");
        fs::create_dir(&directory).expect("create test directory");
        let assets = [test_plan(1), test_plan(2)];
        let cancel = AtomicBool::new(false);

        let outcome = save_asset_batch(
            &directory,
            &assets,
            &cancel,
            |plan| Ok(test_asset(plan)),
            |completed_count, _| {
                if completed_count == 1 {
                    cancel.store(true, Ordering::Relaxed);
                }
                Ok(())
            },
        )
        .expect("cancel batch save");

        assert_eq!(outcome, BatchSaveOutcome::Cancelled);
        assert!(
            assets
                .iter()
                .all(|asset| !directory.join(page_asset_file_name(asset)).exists())
        );
        fs::remove_dir_all(directory).expect("remove test directory");
    }

    #[test]
    fn manages_one_asset_export_at_a_time() {
        let manager = CategoryExportManager::default();
        let first = manager.start(3).expect("start first export");

        assert_eq!(first.job_id, 1);
        assert_eq!(manager.status(1).expect("first status").total_count, 3);
        assert!(manager.start(2).is_err());
        assert!(manager.cancel(1).expect("cancel first export"));
        assert!(first.cancel.load(Ordering::Relaxed));

        first.status.lock().expect("first status lock").state = CategoryExportState::Cancelled;
        let second = manager.start(2).expect("start second export");
        assert_eq!(second.job_id, 2);
        assert_eq!(manager.status(2).expect("second status").total_count, 2);
    }
}
