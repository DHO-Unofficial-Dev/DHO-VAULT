// SPDX-License-Identifier: MPL-2.0

const selectButton = document.querySelector("#select-game-directory");
const changeDirectoryButton = document.querySelector("#change-game-directory");
const connectionView = document.querySelector("#connection-view");
const workspaceView = document.querySelector("#workspace-view");
const titlebarNavigation = document.querySelector("#titlebar-navigation");
const titlebarLanguageControl = document.querySelector(
  "#titlebar-language-control",
);
const minimizeWindowButton = document.querySelector("#minimize-window");
const closeWindowButton = document.querySelector("#close-window");
const titlebarVersion = document.querySelector("#titlebar-version");
const windowStatus = document.querySelector("#window-status");
const statusAppUpdateButton = document.querySelector("#status-app-update");
const navigationButtons = document.querySelectorAll(".nav-button");
const appPages = document.querySelectorAll(".app-page");
const statusCard = document.querySelector(".status-card");
const statusTitle = document.querySelector("#status-title");
const statusMessage = document.querySelector("#status-message");
const appUpdateBanner = document.querySelector("#app-update-banner");
const appUpdateBannerTitle = document.querySelector(
  "#app-update-banner-title",
);
const appUpdateBannerMessage = document.querySelector(
  "#app-update-banner-message",
);
const appUpdateBannerInstallButton = document.querySelector(
  "#app-update-banner-install",
);
const settingsMessage = document.querySelector("#settings-message");
const directoryDetails = document.querySelector("#directory-details");
const gameDirectory = document.querySelector("#game-directory");
const resourceDirectory = document.querySelector("#resource-directory");
const archiveList = document.querySelector("#archive-list");
const archiveStatus = document.querySelector("#archive-status");
const catalogDiagnosticsCard = document.querySelector("#catalog-diagnostics-card");
const catalogDiagnosticsStatus = document.querySelector("#catalog-diagnostics-status");
const diagnosticsTotalCount = document.querySelector("#diagnostics-total-count");
const diagnosticsCategorizedCount = document.querySelector("#diagnostics-categorized-count");
const diagnosticsUnclassifiedCount = document.querySelector("#diagnostics-unclassified-count");
const diagnosticsMultipleCount = document.querySelector("#diagnostics-multiple-count");
const diagnosticsUnclassifiedSummary = document.querySelector(
  "#diagnostics-unclassified-summary",
);
const diagnosticsUnclassifiedList = document.querySelector(
  "#diagnostics-unclassified-list",
);
const diagnosticsMultipleSummary = document.querySelector(
  "#diagnostics-multiple-summary",
);
const diagnosticsMultipleList = document.querySelector(
  "#diagnostics-multiple-list",
);
const appCurrentVersion = document.querySelector("#app-current-version");
const appUpdateMessage = document.querySelector("#app-update-message");
const appUpdateReleaseNotes = document.querySelector(
  "#app-update-release-notes",
);
const appUpdateReleaseNotesContent = document.querySelector(
  "#app-update-release-notes-content",
);
const appUpdateProgress = document.querySelector("#app-update-progress");
const appUpdateProgressLabel = document.querySelector(
  "#app-update-progress-label",
);
const appUpdateProgressBar = document.querySelector(
  "#app-update-progress-bar",
);
const checkAppUpdateButton = document.querySelector("#check-app-update");
const installAppUpdateButton = document.querySelector(
  "#install-app-update",
);
const imageExportMode = document.querySelector("#image-export-mode");
const updatePanel = document.querySelector("#update-panel");
const updateStatus = document.querySelector("#update-status");
const updateMessage = document.querySelector("#update-message");
const updateCounts = document.querySelector("#update-counts");
const updateAddedCount = document.querySelector("#update-added-count");
const updateChangedCount = document.querySelector("#update-changed-count");
const updateRemovedCount = document.querySelector("#update-removed-count");
const updateAddedRelationCount = document.querySelector(
  "#update-added-relation-count",
);
const updateRemovedRelationCount = document.querySelector(
  "#update-removed-relation-count",
);
const updateRemovedRelations = document.querySelector(
  "#update-removed-relations",
);
const updateRemovedRelationsSummary = document.querySelector(
  "#update-removed-relations-summary",
);
const updateRemovedRelationsList = document.querySelector(
  "#update-removed-relations-list",
);
const updateActions = document.querySelector("#update-actions");
const createUpdateBaselineButton = document.querySelector(
  "#create-update-baseline",
);
const viewUpdateAssetsButton = document.querySelector(
  "#view-update-assets",
);
const refreshUpdateBaselineButton = document.querySelector(
  "#refresh-update-baseline",
);
const categoryPanel = document.querySelector("#category-panel");
const categoryStatus = document.querySelector("#category-status");
const categoryGroups = document.querySelector("#category-groups");
const assetSearchForm = document.querySelector("#asset-search");
const assetSearchQuery = document.querySelector("#asset-search-query");
const assetSearchSubmit = document.querySelector("#asset-search-submit");
const galleryPanel = document.querySelector("#gallery-panel");
const galleryTitle = document.querySelector("#gallery-title");
const galleryStatus = document.querySelector("#gallery-status");
const galleryGrid = document.querySelector("#gallery-grid");
const toggleSelectionButton = document.querySelector("#toggle-selection");
const saveAllButton = document.querySelector("#save-all");
const selectionBar = document.querySelector("#selection-bar");
const selectionCount = document.querySelector("#selection-count");
const selectCurrentPageButton = document.querySelector("#select-current-page");
const clearCurrentPageButton = document.querySelector("#clear-current-page");
const clearSelectionButton = document.querySelector("#clear-selection");
const saveSelectionButton = document.querySelector("#save-selection");
const categoryExport = document.querySelector("#category-export");
const categoryExportStatus = document.querySelector("#category-export-status");
const categoryExportProgress = document.querySelector(
  "#category-export-progress",
);
const cancelCategoryExportButton = document.querySelector(
  "#cancel-category-export",
);
const galleryNavigations = document.querySelectorAll(
  "[data-gallery-navigation]",
);
const firstPageButtons = document.querySelectorAll(
  '[data-page-action="first"]',
);
const previousPageButtons = document.querySelectorAll(
  '[data-page-action="previous"]',
);
const nextPageButtons = document.querySelectorAll(
  '[data-page-action="next"]',
);
const lastPageButtons = document.querySelectorAll(
  '[data-page-action="last"]',
);
const pageJumpForms = document.querySelectorAll("[data-page-jump]");
const pageNumberLists = document.querySelectorAll("[data-page-number-list]");
const pageNumberInputs = document.querySelectorAll(".page-number-input");
const pageCountLabels = document.querySelectorAll(".page-count");
const detailDialog = document.querySelector("#asset-detail");
const detailTitle = document.querySelector("#detail-title");
const detailContent = document.querySelector("#detail-content");
const detailPreview = document.querySelector("#detail-preview");
const detailMessage = document.querySelector("#detail-message");
const detailRecordList = document.querySelector("#detail-record-list");
const detailMetadata = document.querySelector("#detail-metadata");
const detailSourceSize = document.querySelector("#detail-source-size");
const detailPreviewSize = document.querySelector("#detail-preview-size");
const detailGmSource = document.querySelector("#detail-gm-source");
const detailGmSourceLabel = document.querySelector("#detail-gm-source-label");
const detailGmSourceImage = document.querySelector("#detail-gm-source-image");
const detailGmSourceHighlight = document.querySelector("#detail-gm-source-highlight");
const downloadDetailButton = document.querySelector("#download-detail");
const closeDetailButton = document.querySelector("#close-detail");
const textCatalogStatus = document.querySelector("#text-catalog-status");
const textLanguageBlock = document.querySelector("#text-language-block");
const textSourceAllButton = document.querySelector("#text-source-all");
const textSourceList = document.querySelector("#text-source-list");
const textUpdateStatus = document.querySelector("#text-update-status");
const textUpdateMessage = document.querySelector("#text-update-message");
const textUpdateCounts = document.querySelector("#text-update-counts");
const textUpdateAddedCount = document.querySelector("#text-update-added-count");
const textUpdateChangedCount = document.querySelector("#text-update-changed-count");
const textUpdateRemovedCount = document.querySelector("#text-update-removed-count");
const textUpdateActions = document.querySelector("#text-update-actions");
const createTextBaselineButton = document.querySelector("#create-text-baseline");
const viewTextChangesButton = document.querySelector("#view-text-changes");
const refreshTextBaselineButton = document.querySelector("#refresh-text-baseline");
const textBrowserTitle = document.querySelector("#text-browser-title");
const textPageStatus = document.querySelector("#text-page-status");
const textSearchForm = document.querySelector("#text-search");
const textSearchQuery = document.querySelector("#text-search-query");
const textSearchSubmit = document.querySelector("#text-search-submit");
const textRecordList = document.querySelector("#text-record-list");
const textFirstPageButton = document.querySelector("#text-first-page");
const textPreviousPageButton = document.querySelector("#text-previous-page");
const textNextPageButton = document.querySelector("#text-next-page");
const textLastPageButton = document.querySelector("#text-last-page");
const textPageNumberList = document.querySelector("#text-page-number-list");
const textPageJumpForm = document.querySelector("#text-page-jump");
const textPageInput = document.querySelector("#text-page-input");
const textPageCount = document.querySelector("#text-page-count");
const textPageJumpButton = document.querySelector("#text-page-jump-button");
const audioCatalogStatus = document.querySelector("#audio-catalog-status");
const audioCatalogDescription = document.querySelector("#audio-catalog-description");
const audioSearchForm = document.querySelector("#audio-search");
const audioSearchQuery = document.querySelector("#audio-search-query");
const audioTrackList = document.querySelector("#audio-track-list");
const audioFirstPageButton = document.querySelector("#audio-first-page");
const audioPreviousPageButton = document.querySelector("#audio-previous-page");
const audioNextPageButton = document.querySelector("#audio-next-page");
const audioLastPageButton = document.querySelector("#audio-last-page");
const audioPageCount = document.querySelector("#audio-page-count");
const audioPlayerTitle = document.querySelector("#audio-player-title");
const audioPlayerMessage = document.querySelector("#audio-player-message");
const audioPlayer = document.querySelector("#audio-player");
const audioTrackMetadata = document.querySelector("#audio-track-metadata");
const audioTrackFile = document.querySelector("#audio-track-file");
const audioTrackSize = document.querySelector("#audio-track-size");
const audioTrackFormat = document.querySelector("#audio-track-format");
const audioTrackLoop = document.querySelector("#audio-track-loop");
const saveAudioTrackButton = document.querySelector("#save-audio-track");

let currentPage = null;
let galleryRequestId = 0;
let detailRequestId = 0;
let currentDetail = null;
let currentCategoryExport = null;
let categoryExportPollTimer = null;
let categoryExportBusy = false;
let categoryExportCancelRequested = false;
let updateRequestId = 0;
let currentAssetUpdateStatus = null;
const rememberedPageOffsets = new Map();
const selectedAssets = new Map();
let selectionMode = false;
let currentWorkspacePage = "library";
let availableAppUpdate = null;
let appUpdateBusy = false;
let appUpdateDownloadedBytes = 0;
let appUpdateActivity = null;
let textCatalogLoaded = false;
let currentTextLanguageBlock = 1;
let currentTextSource = null;
let currentTextPage = null;
let textChangeMode = false;
let textRequestId = 0;
let textUpdateRequestId = 0;
let audioCatalogLoaded = false;
let currentAudioPage = null;
let currentAudioTrack = null;
let audioRequestId = 0;
let audioTrackRequestId = 0;

const CATEGORY_EXPORT_POLL_INTERVAL = 250;
const VISIBLE_PAGE_BUTTON_COUNT = 9;
const VISIBLE_PAGE_RADIUS = 4;
const IMAGE_EXPORT_MODE_STORAGE_KEY = "dho-vault-image-export-mode";
const WINDOW_STATE_STORAGE_KEY = "dho-vault-window-state-v1";
const MIN_WINDOW_WIDTH = 720;
const MIN_WINDOW_HEIGHT = 480;
let windowStateSaveTimer = null;

function loadImageExportMode() {
  try {
    const stored = window.localStorage.getItem(IMAGE_EXPORT_MODE_STORAGE_KEY);
    if ([...imageExportMode.options].some((option) => option.value === stored)) {
      imageExportMode.value = stored;
    }
  } catch {
    imageExportMode.value = "service_upscaled";
  }
}

function currentImageExportMode() {
  return imageExportMode.value === "original"
    ? "original"
    : "service_upscaled";
}

function imageExportModeLabel(mode) {
  return mode === "original" ? "원본 픽셀 유지" : "서비스용 선명 확대";
}

function selectionKey(path, item) {
  return JSON.stringify([
    path,
    item.archive.toLocaleLowerCase("en-US"),
    item.blockIndex,
  ]);
}

function selectionEntry(path, item) {
  return {
    path: [...path],
    archive: item.archive,
    blockIndex: item.blockIndex,
  };
}

function currentPageSelectionEntries() {
  if (currentPage === null) {
    return [];
  }
  const pathItemMode = currentPage.mode === "search" || currentPage.mode === "update";
  return currentPage.items.map((entry) => {
    const path = pathItemMode ? entry.path : currentPage.path;
    const item = pathItemMode ? entry.thumbnail : entry;
    return {
      key: selectionKey(path, item),
      value: selectionEntry(path, item),
    };
  });
}

function updateSelectionControls() {
  const count = selectedAssets.size;
  const libraryVisible = currentWorkspacePage === "library";
  const hasPageItems = libraryVisible && (currentPage?.items.length ?? 0) > 0;
  const hasCurrentPageSelection = currentPageSelectionEntries().some(({ key }) =>
    selectedAssets.has(key),
  );
  const active = libraryVisible && (selectionMode || count > 0);
  document.body.dataset.selectionActive = String(active);
  selectionBar.hidden = !active;
  selectionCount.textContent = `${formatNumber(count)}개 선택`;
  saveSelectionButton.textContent = `선택한 ${formatNumber(count)}개 저장`;
  saveSelectionButton.disabled =
    categoryExportBusy || !libraryVisible || count === 0;
  selectCurrentPageButton.disabled = categoryExportBusy || !hasPageItems;
  clearCurrentPageButton.disabled =
    categoryExportBusy || !hasCurrentPageSelection;
  clearSelectionButton.disabled = categoryExportBusy || count === 0;
  toggleSelectionButton.disabled = categoryExportBusy || !hasPageItems;
  toggleSelectionButton.textContent = selectionMode ? "선택 완료" : "선택";
}

function setSelectionMode(enabled) {
  selectionMode = enabled;
  if (currentPage !== null) {
    renderGallery(currentPage);
    return;
  }
  updateSelectionControls();
}

function clearSelections() {
  selectedAssets.clear();
  if (currentPage !== null) {
    renderGallery(currentPage);
    return;
  }
  updateSelectionControls();
}

function pageMemoryKey(page) {
  if (page.mode === "update") {
    return "update";
  }
  if (page.mode === "search") {
    return `search:${page.query.trim().toLocaleLowerCase("ko-KR")}`;
  }
  return `category:${page.path.join("\u0000")}`;
}

function rememberedPageOffset(page) {
  return rememberedPageOffsets.get(pageMemoryKey(page)) ?? 0;
}

function rememberPage(page) {
  rememberedPageOffsets.set(pageMemoryKey(page), page.offset);
}

function pageOffsetIsOutOfRange(error, offset) {
  return (
    offset > 0 &&
    String(error).includes("이미지 시작 위치가 카테고리 범위를 벗어났습니다")
  );
}

function totalPages(page) {
  return Math.ceil(page.totalCount / page.pageSize);
}

function currentPageNumber(page) {
  return page.totalCount === 0
    ? 0
    : Math.floor(page.offset / page.pageSize) + 1;
}

function setPageNavigationHidden(hidden) {
  for (const navigation of galleryNavigations) {
    navigation.hidden = hidden;
  }
}

function setAllPageNavigationControlsDisabled(disabled) {
  for (const button of document.querySelectorAll(
    "[data-gallery-navigation] button",
  )) {
    button.disabled = disabled;
  }
  for (const input of pageNumberInputs) {
    input.disabled = disabled;
  }
}

function setPageNavigationLoading() {
  setPageNavigationHidden(false);
  for (const list of pageNumberLists) {
    list.replaceChildren();
  }
  setAllPageNavigationControlsDisabled(true);
  for (const input of pageNumberInputs) {
    input.value = "";
  }
  for (const label of pageCountLabels) {
    label.textContent = "불러오는 중";
  }
}

function renderPageNavigation(page) {
  const pageNumber = currentPageNumber(page);
  const pageTotal = totalPages(page);
  const firstPage = pageNumber <= 1;
  const lastPage = pageNumber === 0 || pageNumber >= pageTotal;
  setPageNavigationHidden(false);

  for (const button of firstPageButtons) {
    button.disabled = categoryExportBusy || firstPage;
  }
  for (const button of previousPageButtons) {
    button.disabled = categoryExportBusy || firstPage;
  }
  for (const button of nextPageButtons) {
    button.disabled = categoryExportBusy || lastPage;
  }
  for (const button of lastPageButtons) {
    button.disabled = categoryExportBusy || lastPage;
  }
  let firstVisiblePage = Math.max(1, pageNumber - VISIBLE_PAGE_RADIUS);
  const lastVisiblePage = Math.min(
    pageTotal,
    firstVisiblePage + VISIBLE_PAGE_BUTTON_COUNT - 1,
  );
  firstVisiblePage = Math.max(
    1,
    lastVisiblePage - VISIBLE_PAGE_BUTTON_COUNT + 1,
  );
  for (const list of pageNumberLists) {
    list.replaceChildren();
    for (
      let visiblePage = firstVisiblePage;
      visiblePage <= lastVisiblePage;
      visiblePage += 1
    ) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "secondary-button page-number-button";
      button.textContent = formatNumber(visiblePage);
      button.setAttribute(
        "aria-label",
        `${formatNumber(visiblePage)}페이지로 이동`,
      );
      if (visiblePage === pageNumber) {
        button.setAttribute("aria-current", "page");
      }
      button.disabled = categoryExportBusy || visiblePage === pageNumber;
      button.addEventListener("click", () => navigateToPage(visiblePage));
      list.append(button);
    }
  }
  for (const input of pageNumberInputs) {
    input.value = String(pageNumber);
    input.max = String(pageTotal);
    input.disabled = categoryExportBusy || pageTotal === 0;
  }
  for (const label of pageCountLabels) {
    label.textContent = `${formatNumber(pageTotal)} 페이지`;
  }
  for (const form of pageJumpForms) {
    form.querySelector('button[type="submit"]').disabled =
      categoryExportBusy || pageTotal === 0;
  }
}

function setPageNavigationBusy(busy) {
  if (currentPage === null || busy) {
    setAllPageNavigationControlsDisabled(true);
    return;
  }
  renderPageNavigation(currentPage);
}

function scrollGalleryToTop() {
  galleryPanel.scrollIntoView({ block: "start" });
}

function setStatus(kind, title, message) {
  statusCard.dataset.status = kind;
  statusTitle.textContent = title;
  statusMessage.textContent = message;
  setWindowStatus(
    kind === "loading" ? "busy" : kind === "error" ? "error" : "idle",
    kind === "loading"
      ? "게임 폴더 확인 중"
      : kind === "error"
        ? "게임 폴더 확인 필요"
        : "준비됨",
  );
}

function setWindowStatus(kind, message) {
  windowStatus.dataset.status = kind;
  windowStatus.textContent = message;
}

function setDirectorySelectionBusy(busy) {
  selectButton.disabled = busy;
  changeDirectoryButton.disabled = busy;
}

function showConnectionView() {
  document.body.dataset.view = "connection";
  titlebarNavigation.hidden = true;
  titlebarLanguageControl.hidden = true;
  connectionView.hidden = false;
  workspaceView.hidden = true;
}

function showWorkspacePage(pageName) {
  currentWorkspacePage = pageName;
  document.body.dataset.view = "workspace";
  titlebarNavigation.hidden = false;
  titlebarLanguageControl.hidden = !["library", "text"].includes(pageName);
  connectionView.hidden = true;
  workspaceView.hidden = false;
  for (const page of appPages) {
    page.hidden = page.dataset.page !== pageName;
  }
  for (const button of navigationButtons) {
    button.setAttribute(
      "aria-pressed",
      String(button.dataset.page === pageName),
    );
  }
  updateSelectionControls();
}

function currentWindow() {
  return window.__TAURI__.window.getCurrentWindow();
}

function savedWindowState() {
  try {
    const value = JSON.parse(
      window.localStorage.getItem(WINDOW_STATE_STORAGE_KEY) ?? "null",
    );
    if (
      Number.isFinite(value?.width) &&
      Number.isFinite(value?.height) &&
      value.width >= MIN_WINDOW_WIDTH &&
      value.height >= MIN_WINDOW_HEIGHT
    ) {
      return {
        width: Math.round(value.width),
        height: Math.round(value.height),
        maximized: value.maximized === true,
      };
    }
  } catch {
    // Invalid or unavailable local storage falls back to the configured size.
  }
  return null;
}

async function persistWindowState() {
  if (windowStateSaveTimer !== null) {
    window.clearTimeout(windowStateSaveTimer);
    windowStateSaveTimer = null;
  }
  try {
    const appWindow = currentWindow();
    const maximized = await appWindow.isMaximized();
    const previous = savedWindowState();
    const width = maximized ? previous?.width : window.innerWidth;
    const height = maximized ? previous?.height : window.innerHeight;
    if (width === undefined || height === undefined) {
      return;
    }
    window.localStorage.setItem(
      WINDOW_STATE_STORAGE_KEY,
      JSON.stringify({ width, height, maximized }),
    );
  } catch {
    // Window persistence is a convenience; it must never prevent app use.
  }
}

async function initializeWindowState() {
  const appWindow = currentWindow();
  const saved = savedWindowState();
  try {
    if (saved !== null) {
      const { LogicalSize } = window.__TAURI__.window;
      await appWindow.setSize(new LogicalSize(saved.width, saved.height));
      if (saved.maximized) {
        await appWindow.maximize();
      }
    }
    await appWindow.onResized(() => {
      if (windowStateSaveTimer !== null) {
        window.clearTimeout(windowStateSaveTimer);
      }
      windowStateSaveTimer = window.setTimeout(persistWindowState, 250);
    });
  } catch {
    // Keep the configured default size if restoring is unavailable.
  }
}

function focusAvailableAppUpdate() {
  if (availableAppUpdate === null) {
    return;
  }
  if (!workspaceView.hidden) {
    showWorkspacePage("settings");
    document
      .querySelector(".app-update-card")
      .scrollIntoView({ block: "start" });
    installAppUpdateButton.focus();
    return;
  }
  appUpdateBanner.scrollIntoView({ block: "start" });
  appUpdateBannerInstallButton.focus();
}

function renderStatusAppUpdate() {
  if (appUpdateActivity !== null) {
    statusAppUpdateButton.hidden = false;
    statusAppUpdateButton.disabled = true;
    statusAppUpdateButton.textContent =
      appUpdateActivity === "installing"
        ? "업데이트 설치 중"
        : "업데이트 확인 중";
    return;
  }
  if (availableAppUpdate === null) {
    statusAppUpdateButton.hidden = true;
    statusAppUpdateButton.disabled = false;
    statusAppUpdateButton.textContent = "";
    return;
  }
  statusAppUpdateButton.hidden = false;
  statusAppUpdateButton.disabled = categoryExportBusy;
  statusAppUpdateButton.textContent = `v${availableAppUpdate.version} 업데이트`;
  statusAppUpdateButton.title = categoryExportBusy
    ? "이미지 저장이 끝나면 업데이트 화면을 열 수 있습니다."
    : "업데이트 화면 열기";
}

function clearCategoryExportPoll() {
  if (categoryExportPollTimer !== null) {
    window.clearTimeout(categoryExportPollTimer);
    categoryExportPollTimer = null;
  }
}

function hideCategoryExport() {
  clearCategoryExportPoll();
  currentCategoryExport = null;
  categoryExportCancelRequested = false;
  categoryExport.hidden = true;
  categoryExportStatus.textContent = "";
  categoryExportProgress.value = 0;
  categoryExportProgress.max = 1;
  cancelCategoryExportButton.disabled = true;
  cancelCategoryExportButton.textContent = "저장 취소";
}

function setCategoryExportBusy(busy) {
  categoryExportBusy = busy;
  setDirectorySelectionBusy(busy);
  createUpdateBaselineButton.disabled = busy;
  viewUpdateAssetsButton.disabled = busy;
  refreshUpdateBaselineButton.disabled = busy;
  assetSearchQuery.disabled = busy;
  assetSearchSubmit.disabled = busy;
  for (const button of navigationButtons) {
    button.disabled = busy;
  }
  for (const button of categoryGroups.querySelectorAll(".category-button")) {
    button.disabled = busy;
  }
  for (const button of galleryGrid.querySelectorAll("button")) {
    button.disabled = busy;
  }
  const hasPageItems = (currentPage?.items.length ?? 0) > 0;
  saveAllButton.disabled = busy || !hasPageItems;
  setPageNavigationBusy(busy);
  updateSelectionControls();
  renderStatusAppUpdate();
}

function setTextSourceSelection(source) {
  currentTextSource = source;
  textSourceAllButton.setAttribute("aria-pressed", String(source === null));
  for (const button of textSourceList.querySelectorAll(".text-source-button")) {
    button.setAttribute(
      "aria-pressed",
      String(button.dataset.source === source),
    );
  }
}

function resetAudioWorkspace() {
  audioCatalogLoaded = false;
  currentAudioPage = null;
  currentAudioTrack = null;
  audioRequestId += 1;
  audioTrackRequestId += 1;
  audioCatalogStatus.textContent = "";
  audioCatalogDescription.textContent =
    "클라이언트의 KOVS 오디오를 필요할 때 한 곡씩 Ogg Vorbis로 해제합니다.";
  audioSearchQuery.value = "";
  audioTrackList.replaceChildren();
  audioTrackList.dataset.status = "empty";
  audioPageCount.textContent = "0 페이지";
  audioFirstPageButton.disabled = true;
  audioPreviousPageButton.disabled = true;
  audioNextPageButton.disabled = true;
  audioLastPageButton.disabled = true;
  audioPlayer.pause();
  audioPlayer.removeAttribute("src");
  audioPlayer.load();
  audioPlayerTitle.textContent = "곡을 선택해 주세요";
  audioPlayerMessage.textContent =
    "목록에서 한 곡을 누르면 해당 파일만 해제하여 재생합니다.";
  audioTrackMetadata.hidden = true;
  saveAudioTrackButton.disabled = true;
}

function renderAudioPage(page) {
  currentAudioPage = page;
  audioTrackList.replaceChildren();
  audioTrackList.dataset.status = page.items.length === 0 ? "empty" : "ready";
  if (page.items.length === 0) {
    audioTrackList.textContent = page.query
      ? "검색 결과가 없습니다."
      : "표시할 오디오가 없습니다.";
  }
  for (const item of page.items) {
    const button = document.createElement("button");
    const id = document.createElement("strong");
    const detail = document.createElement("span");
    button.type = "button";
    button.className = "audio-track-button";
    button.dataset.audioId = String(item.id);
    button.setAttribute("aria-pressed", String(currentAudioTrack?.id === item.id));
    id.textContent = `ID ${String(item.id).padStart(6, "0")}`;
    detail.textContent = `${formatBytes(item.payloadBytes)}${item.loopStartSample > 0 ? " · 루프 있음" : ""}`;
    button.append(id, detail);
    button.addEventListener("click", () => loadAudioTrack(item.id));
    audioTrackList.append(button);
  }
  const pageTotal = page.totalCount === 0
    ? 0
    : Math.ceil(page.totalCount / page.pageSize);
  const pageCurrent = pageTotal === 0
    ? 0
    : Math.floor(page.offset / page.pageSize) + 1;
  audioPageCount.textContent = `${formatNumber(pageCurrent)} / ${formatNumber(pageTotal)} 페이지`;
  audioFirstPageButton.disabled = pageCurrent <= 1;
  audioPreviousPageButton.disabled = pageCurrent <= 1;
  audioNextPageButton.disabled = pageCurrent === 0 || pageCurrent >= pageTotal;
  audioLastPageButton.disabled = pageCurrent === 0 || pageCurrent >= pageTotal;
  audioCatalogStatus.textContent = page.totalCount === 0
    ? "0개"
    : `${formatNumber(page.totalCount)}개 중 ${formatNumber(page.offset + 1)}–${formatNumber(Math.min(page.offset + page.items.length, page.totalCount))}`;
}

async function loadAudioPage(offset = 0) {
  const requestId = ++audioRequestId;
  audioTrackList.dataset.status = "loading";
  audioTrackList.textContent = "오디오 목록을 불러오는 중입니다.";
  try {
    const page = await window.__TAURI__.core.invoke("load_audio_track_page", {
      query: audioSearchQuery.value.trim(),
      offset,
    });
    if (requestId === audioRequestId) {
      renderAudioPage(page);
    }
  } catch (error) {
    if (requestId === audioRequestId) {
      audioTrackList.dataset.status = "empty";
      audioTrackList.textContent = `오디오 목록을 불러오지 못했습니다: ${String(error)}`;
      audioCatalogStatus.textContent = "오류";
    }
  }
}

async function loadAudioWorkspace() {
  if (audioCatalogLoaded) {
    return;
  }
  audioCatalogStatus.textContent = "확인 중";
  try {
    const summary = await window.__TAURI__.core.invoke("load_audio_catalog_summary");
    audioCatalogLoaded = true;
    audioCatalogStatus.textContent = `${formatNumber(summary.trackCount)}곡 · ${formatBytes(summary.totalPayloadBytes)}`;
    audioCatalogDescription.textContent = summary.unrecognizedFileCount > 0
      ? `검증된 KOVS 오디오 ${formatNumber(summary.trackCount)}곡을 표시합니다. 형식이 확인되지 않은 BIN ${formatNumber(summary.unrecognizedFileCount)}개는 임의 해석하지 않았습니다.`
      : `검증된 KOVS 오디오 ${formatNumber(summary.trackCount)}곡을 표시합니다.`;
    await loadAudioPage(0);
  } catch (error) {
    audioCatalogStatus.textContent = "오류";
    audioTrackList.dataset.status = "empty";
    audioTrackList.textContent = `오디오 자료를 열지 못했습니다: ${String(error)}`;
  }
}

async function loadAudioTrack(id) {
  const requestId = ++audioTrackRequestId;
  audioPlayer.pause();
  audioPlayerTitle.textContent = `ID ${String(id).padStart(6, "0")}`;
  audioPlayerMessage.textContent = "선택한 오디오 한 곡을 해제하고 있습니다.";
  audioTrackMetadata.hidden = true;
  saveAudioTrackButton.disabled = true;
  try {
    const track = await window.__TAURI__.core.invoke("load_audio_track", { id });
    if (requestId !== audioTrackRequestId) {
      return;
    }
    currentAudioTrack = track;
    audioPlayer.src = track.audioDataUrl;
    audioPlayerTitle.textContent = `오디오 ${String(track.id).padStart(6, "0")}`;
    audioPlayerMessage.textContent = "원본 KOVS 래퍼를 해제한 Ogg Vorbis 스트림입니다.";
    audioTrackFile.textContent = track.fileName;
    audioTrackSize.textContent = formatBytes(track.payloadBytes);
    audioTrackFormat.textContent = track.sampleRate
      ? `Ogg Vorbis · ${formatNumber(track.sampleRate)} Hz · ${track.channels ?? "?"}채널`
      : "Ogg Vorbis";
    audioTrackLoop.textContent = track.loopStartSample > 0
      ? `${track.loopStartSeconds.toFixed(3)}초 · ${formatNumber(track.loopStartSample)} 샘플`
      : "지정 없음";
    audioTrackMetadata.hidden = false;
    saveAudioTrackButton.disabled = false;
    for (const button of audioTrackList.querySelectorAll(".audio-track-button")) {
      button.setAttribute("aria-pressed", String(Number(button.dataset.audioId) === track.id));
    }
    await audioPlayer.play().catch(() => {});
  } catch (error) {
    if (requestId === audioTrackRequestId) {
      currentAudioTrack = null;
      audioPlayerMessage.textContent = `오디오를 열지 못했습니다: ${String(error)}`;
    }
  }
}

function renderTextCatalog(summary) {
  textCatalogLoaded = true;
  renderLanguageSelector(summary);
  textCatalogStatus.textContent = `${formatNumber(summary.totalCount)}개 문구`;
  textSourceList.replaceChildren();
  for (const source of summary.sources) {
    const button = document.createElement("button");
    const title = document.createElement("strong");
    const description = document.createElement("span");
    button.type = "button";
    button.className = "text-source-button";
    button.dataset.source = source.fileName;
    button.setAttribute("aria-pressed", "false");
    title.textContent = `${source.label} · ${formatNumber(source.recordCount)}`;
    description.textContent = `${source.fileName} — ${source.description}`;
    button.append(title, description);
    button.addEventListener("click", () => {
      textChangeMode = false;
      textSearchQuery.value = "";
      setTextSourceSelection(source.fileName);
      textBrowserTitle.textContent = source.label;
      loadTextPage(0);
    });
    textSourceList.append(button);
  }
  setTextSourceSelection(currentTextSource);
}

function renderLanguageSelector(summary) {
  currentTextLanguageBlock = summary.selectedLanguageBlock;
  textLanguageBlock.replaceChildren();
  for (const block of summary.languageBlocks) {
    const option = document.createElement("option");
    option.value = String(block.index);
    const compactLabel = block.label.replace("블록 ", "");
    option.textContent = block.isDefault ? `${compactLabel} · 기본` : compactLabel;
    textLanguageBlock.append(option);
  }
  textLanguageBlock.value = String(currentTextLanguageBlock);
  textLanguageBlock.disabled = false;
}

function renderTextRecord(item, changeKind = null) {
  const card = document.createElement("article");
  const heading = document.createElement("header");
  const source = document.createElement("span");
  const id = document.createElement("span");
  card.className = "text-record-card";
  heading.className = "text-record-heading";
  source.className = "text-record-source";
  id.className = "text-record-id";
  source.textContent = `${item.sourceLabel} · ${item.source}`;
  id.textContent = `ID ${formatNumber(item.id)}`;
  if (changeKind !== null) {
    const badge = document.createElement("span");
    badge.className = "text-record-change";
    badge.textContent = changeKind === "added" ? "신규" : "변경";
    source.prepend(badge);
  }
  heading.append(source, id);
  card.append(heading);
  const fields = item.fields.length === 0 ? ["(텍스트 필드 없음)"] : item.fields;
  for (const field of fields) {
    const paragraph = document.createElement("p");
    paragraph.className = "text-record-field";
    paragraph.textContent = field === "" ? "(빈 문자열)" : field;
    card.append(paragraph);
  }
  if (item.imageLinks.length > 0) {
    const links = document.createElement("div");
    links.className = "text-image-links";
    for (const link of item.imageLinks) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "text-image-link-button";
      button.textContent = `${link.relation} 보기`;
      button.title = [
        `${link.archive.toUpperCase()} · 그룹 ${link.groupCode} · 이미지 ID ${link.iconId}`,
        relationEvidenceLabel(link.evidenceType),
        relationVerificationLabel(link.verificationStatus),
      ].join(" · ");
      button.addEventListener("click", () => loadTextImageDetail(link, item));
      links.append(button);
    }
    card.append(links);
  }
  return card;
}

function renderTextPage(page, updateMode = false) {
  currentTextPage = page;
  textRecordList.replaceChildren();
  textRecordList.dataset.status = page.items.length === 0 ? "empty" : "ready";
  if (page.items.length === 0) {
    textRecordList.textContent = updateMode
      ? "표시할 신규·변경 문구가 없습니다."
      : "조건에 맞는 텍스트가 없습니다.";
  } else {
    for (const entry of page.items) {
      const item = updateMode ? entry.record : entry;
      textRecordList.append(
        renderTextRecord(item, updateMode ? entry.changeKind : null),
      );
    }
  }
  const pageSize = page.pageSize;
  const pageTotal = Math.ceil(page.totalCount / pageSize);
  const pageCurrent = page.totalCount === 0 ? 0 : Math.floor(page.offset / pageSize) + 1;
  textPageNumberList.replaceChildren();
  let firstVisiblePage = Math.max(1, pageCurrent - VISIBLE_PAGE_RADIUS);
  const lastVisiblePage = Math.min(
    pageTotal,
    firstVisiblePage + VISIBLE_PAGE_BUTTON_COUNT - 1,
  );
  firstVisiblePage = Math.max(
    1,
    lastVisiblePage - VISIBLE_PAGE_BUTTON_COUNT + 1,
  );
  for (
    let visiblePage = firstVisiblePage;
    visiblePage <= lastVisiblePage;
    visiblePage += 1
  ) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "secondary-button page-number-button";
    button.textContent = formatNumber(visiblePage);
    button.setAttribute("aria-label", `${formatNumber(visiblePage)}페이지로 이동`);
    if (visiblePage === pageCurrent) {
      button.setAttribute("aria-current", "page");
      button.disabled = true;
    }
    button.addEventListener("click", () => navigateToTextPage(visiblePage));
    textPageNumberList.append(button);
  }
  textPageInput.value = pageCurrent === 0 ? "" : String(pageCurrent);
  textPageInput.max = String(pageTotal);
  textPageInput.disabled = pageTotal === 0;
  textPageCount.textContent = `${formatNumber(pageTotal)} 페이지`;
  textPageJumpButton.disabled = pageTotal === 0;
  textFirstPageButton.disabled = pageCurrent <= 1;
  textPreviousPageButton.disabled = pageCurrent <= 1;
  textNextPageButton.disabled = pageCurrent === 0 || pageCurrent >= pageTotal;
  textLastPageButton.disabled = pageCurrent === 0 || pageCurrent >= pageTotal;
  textPageStatus.textContent =
    page.totalCount === 0
      ? "0개"
      : `${formatNumber(page.totalCount)}개 중 ${formatNumber(page.offset + 1)}–${formatNumber(Math.min(page.offset + page.items.length, page.totalCount))}`;
}

async function loadTextPage(offset = 0) {
  const requestId = ++textRequestId;
  textRecordList.dataset.status = "loading";
  textRecordList.textContent = "텍스트를 불러오는 중입니다.";
  textPageStatus.textContent = "불러오는 중";
  textPageNumberList.replaceChildren();
  textFirstPageButton.disabled = true;
  textPreviousPageButton.disabled = true;
  textNextPageButton.disabled = true;
  textLastPageButton.disabled = true;
  textPageInput.disabled = true;
  textPageJumpButton.disabled = true;
  textPageCount.textContent = "불러오는 중";
  try {
    const page = textChangeMode
      ? await window.__TAURI__.core.invoke("load_text_update_page", { offset })
      : await window.__TAURI__.core.invoke("load_text_record_page", {
          languageBlock: currentTextLanguageBlock,
          source: currentTextSource,
          query: textSearchQuery.value.trim(),
          offset,
        });
    if (requestId === textRequestId) {
      renderTextPage(page, textChangeMode);
    }
  } catch (error) {
    if (requestId === textRequestId) {
      currentTextPage = null;
      textRecordList.dataset.status = "empty";
      textRecordList.textContent = `텍스트를 불러오지 못했습니다: ${String(error)}`;
      textPageStatus.textContent = "오류";
      textPageNumberList.replaceChildren();
      textPageInput.value = "";
      textPageCount.textContent = "0 페이지";
    }
  }
}

function navigateToTextPage(pageNumber) {
  if (currentTextPage === null) {
    return;
  }
  const pageTotal = Math.ceil(
    currentTextPage.totalCount / currentTextPage.pageSize,
  );
  const normalized = Math.min(
    Math.max(1, Math.trunc(Number(pageNumber) || 1)),
    Math.max(1, pageTotal),
  );
  loadTextPage((normalized - 1) * currentTextPage.pageSize);
}

function renderTextUpdateStatus(status) {
  textUpdateAddedCount.textContent = formatNumber(status.addedCount);
  textUpdateChangedCount.textContent = formatNumber(status.changedCount);
  textUpdateRemovedCount.textContent = formatNumber(status.removedCount);
  textUpdateCounts.hidden =
    status.state === "missing_baseline" || status.state === "different_directory";
  const hasVisibleChanges =
    status.state === "changes_detected" &&
    status.addedCount + status.changedCount > 0;
  createTextBaselineButton.hidden = status.state !== "missing_baseline";
  viewTextChangesButton.hidden = !hasVisibleChanges;
  refreshTextBaselineButton.hidden = ![
    "changes_detected",
    "different_directory",
  ].includes(status.state);
  textUpdateActions.hidden =
    createTextBaselineButton.hidden &&
    viewTextChangesButton.hidden &&
    refreshTextBaselineButton.hidden;
  createTextBaselineButton.disabled = false;
  refreshTextBaselineButton.disabled = false;
  if (status.state === "missing_baseline") {
    textUpdateStatus.textContent = `${formatNumber(status.currentCount)}개 문구 확인`;
    textUpdateMessage.textContent =
      "현재 텍스트를 기준점으로 저장하면 다음 게임 업데이트에서 신규·변경 문구를 ID 기준으로 찾습니다.";
  } else if (status.state === "unchanged") {
    textUpdateStatus.textContent = `${formatBaselineDate(status.baselineCreatedAtUnixSeconds)} 기준`;
    textUpdateMessage.textContent = "저장한 기준점과 현재 텍스트가 같습니다.";
  } else if (status.state === "changes_detected") {
    textUpdateStatus.textContent = `${formatBaselineDate(status.baselineCreatedAtUnixSeconds)} 이후 변경`;
    textUpdateMessage.textContent =
      "신규·변경 문구를 확인할 수 있습니다. 삭제된 문구는 개수만 표시합니다.";
  } else {
    textUpdateStatus.textContent = "다른 게임 폴더의 기준점";
    textUpdateMessage.textContent =
      "현재 연결한 게임 폴더와 기준점이 달라 비교하지 않았습니다.";
  }
}

async function loadTextUpdateStatus() {
  const requestId = ++textUpdateRequestId;
  textUpdateStatus.textContent = "확인 중";
  textUpdateMessage.textContent = "저장된 텍스트 기준점과 현재 클라이언트를 비교하고 있습니다.";
  textUpdateCounts.hidden = true;
  textUpdateActions.hidden = true;
  try {
    const status = await window.__TAURI__.core.invoke("load_text_update_status");
    if (requestId === textUpdateRequestId) {
      renderTextUpdateStatus(status);
    }
  } catch (error) {
    if (requestId === textUpdateRequestId) {
      textUpdateStatus.textContent = "확인하지 못함";
      textUpdateMessage.textContent = `텍스트 업데이트를 비교하지 못했습니다: ${String(error)}`;
    }
  }
}

async function loadSharedLanguageSelector() {
  textLanguageBlock.disabled = true;
  const summary = await window.__TAURI__.core.invoke("load_text_catalog_summary", {
    languageBlock: currentTextLanguageBlock,
  });
  renderLanguageSelector(summary);
  return summary;
}

async function loadTextWorkspace() {
  if (textCatalogLoaded) {
    return;
  }
  textCatalogStatus.textContent = "확인 중";
  textLanguageBlock.disabled = true;
  textRecordList.dataset.status = "loading";
  textRecordList.textContent = "텍스트 자료를 여는 중입니다.";
  try {
    const summary = await window.__TAURI__.core.invoke("load_text_catalog_summary", {
      languageBlock: currentTextLanguageBlock,
    });
    renderTextCatalog(summary);
    await Promise.all([loadTextPage(0), loadTextUpdateStatus()]);
  } catch (error) {
    textLanguageBlock.disabled = false;
    textCatalogStatus.textContent = "오류";
    textRecordList.dataset.status = "empty";
    textRecordList.textContent = `텍스트 자료를 열지 못했습니다: ${String(error)}`;
  }
}

function clearSummary() {
  closeDetail();
  hideCategoryExport();
  categoryExportBusy = false;
  rememberedPageOffsets.clear();
  selectedAssets.clear();
  selectionMode = false;
  galleryRequestId += 1;
  currentPage = null;
  galleryPanel.hidden = true;
  galleryTitle.textContent = "카테고리를 선택해 주세요";
  galleryStatus.textContent = "";
  toggleSelectionButton.disabled = true;
  toggleSelectionButton.textContent = "선택";
  saveAllButton.disabled = true;
  saveAllButton.textContent = "카테고리 전체 저장";
  galleryGrid.replaceChildren();
  galleryGrid.dataset.status = "empty";
  setPageNavigationHidden(true);
  categoryPanel.hidden = true;
  categoryStatus.textContent = "";
  categoryGroups.replaceChildren();
  assetSearchQuery.value = "";
  updateSelectionControls();
  directoryDetails.hidden = true;
  gameDirectory.textContent = "";
  resourceDirectory.textContent = "";
  archiveList.replaceChildren();
  archiveStatus.textContent = "";
  catalogDiagnosticsCard.hidden = true;
  catalogDiagnosticsStatus.textContent = "";
  diagnosticsTotalCount.textContent = "0";
  diagnosticsCategorizedCount.textContent = "0";
  diagnosticsUnclassifiedCount.textContent = "0";
  diagnosticsMultipleCount.textContent = "0";
  diagnosticsUnclassifiedList.replaceChildren();
  diagnosticsMultipleList.replaceChildren();
  settingsMessage.textContent = "";
  updateRequestId += 1;
  currentAssetUpdateStatus = null;
  updatePanel.hidden = true;
  updateStatus.textContent = "";
  updateMessage.textContent = "";
  updateCounts.hidden = true;
  updateRemovedRelations.hidden = true;
  updateRemovedRelationsList.replaceChildren();
  updateActions.hidden = true;
  createUpdateBaselineButton.hidden = true;
  createUpdateBaselineButton.disabled = false;
  createUpdateBaselineButton.textContent = "현재 상태를 기준점으로 저장";
  viewUpdateAssetsButton.hidden = true;
  viewUpdateAssetsButton.disabled = false;
  refreshUpdateBaselineButton.hidden = true;
  refreshUpdateBaselineButton.disabled = false;
  refreshUpdateBaselineButton.textContent = "검토 완료 후 기준점 갱신";
  toggleSelectionButton.hidden = true;
  saveAllButton.hidden = true;
  textCatalogLoaded = false;
  currentTextLanguageBlock = 1;
  currentTextSource = null;
  currentTextPage = null;
  textChangeMode = false;
  textRequestId += 1;
  textUpdateRequestId += 1;
  textCatalogStatus.textContent = "";
  textLanguageBlock.replaceChildren();
  const defaultLanguageOption = document.createElement("option");
  defaultLanguageOption.value = "1";
  defaultLanguageOption.textContent = "한국어 · 블록 #1";
  textLanguageBlock.append(defaultLanguageOption);
  textLanguageBlock.value = "1";
  textLanguageBlock.disabled = false;
  textSourceList.replaceChildren();
  textSourceAllButton.setAttribute("aria-pressed", "true");
  textBrowserTitle.textContent = "전체 텍스트";
  textSearchQuery.value = "";
  textRecordList.replaceChildren();
  textRecordList.dataset.status = "empty";
  textPageStatus.textContent = "";
  textPageNumberList.replaceChildren();
  textPageInput.value = "";
  textPageCount.textContent = "0 페이지";
  textFirstPageButton.disabled = true;
  textPreviousPageButton.disabled = true;
  textNextPageButton.disabled = true;
  textLastPageButton.disabled = true;
  textPageInput.disabled = true;
  textPageJumpButton.disabled = true;
  textUpdateStatus.textContent = "";
  textUpdateMessage.textContent = "텍스트 기준점을 확인하고 있습니다.";
  textUpdateCounts.hidden = true;
  textUpdateActions.hidden = true;
  resetAudioWorkspace();
}

function renderAssetUpdateStatus(status) {
  currentAssetUpdateStatus = status;
  updatePanel.hidden = false;
  updateAddedCount.textContent = formatNumber(status.addedCount);
  updateChangedCount.textContent = formatNumber(status.changedCount);
  updateRemovedCount.textContent = formatNumber(status.removedCount);
  updateAddedRelationCount.textContent = formatNumber(status.addedRelationCount);
  updateRemovedRelationCount.textContent = formatNumber(status.removedRelationCount);
  updateRemovedRelationsList.replaceChildren();
  updateRemovedRelations.hidden = status.removedRelationCount === 0;
  updateRemovedRelationsSummary.textContent =
    `사라진 텍스트·이미지 연결 ${formatNumber(status.removedRelationCount)}개`;
  for (const relation of status.removedRelations) {
    const item = document.createElement("li");
    const source = document.createElement("strong");
    const target = document.createElement("span");
    source.textContent = `${relation.sourceLabel} · ID ${formatNumber(relation.textId)}`;
    target.textContent =
      `${relation.archive.toUpperCase()} 그룹 ${formatNumber(relation.groupCode)} ID ${formatNumber(relation.iconId)} · ${relation.relation} · ${relationEvidenceLabel(relation.evidenceType)} · ${relationVerificationLabel(relation.verificationStatus)}`;
    item.append(source, target);
    updateRemovedRelationsList.append(item);
  }
  if (status.removedRelations.length < status.removedRelationCount) {
    const remainder = document.createElement("li");
    remainder.textContent = `앞의 ${formatNumber(status.removedRelations.length)}개만 표시합니다.`;
    updateRemovedRelationsList.append(remainder);
  }
  updateCounts.hidden =
    status.state === "missing_baseline" ||
    status.state === "different_directory";
  const canViewNewAssets =
    status.state === "changes_detected" && status.addedCount > 0;
  const canRefreshBaseline =
    status.state === "changes_detected" ||
    status.state === "different_directory";
  updateActions.hidden =
    status.state !== "missing_baseline" &&
    !canViewNewAssets &&
    !canRefreshBaseline;
  createUpdateBaselineButton.hidden = status.state !== "missing_baseline";
  createUpdateBaselineButton.disabled = false;
  createUpdateBaselineButton.textContent = "현재 상태를 기준점으로 저장";
  viewUpdateAssetsButton.hidden = !canViewNewAssets;
  viewUpdateAssetsButton.disabled = false;
  refreshUpdateBaselineButton.hidden = !canRefreshBaseline;
  refreshUpdateBaselineButton.disabled = false;
  refreshUpdateBaselineButton.textContent =
    status.state === "different_directory"
      ? "현재 폴더로 기준점 변경"
      : "검토 완료 후 기준점 갱신";

  if (status.state === "missing_baseline") {
    updateStatus.textContent = `${formatNumber(status.currentCount)}개 자산 확인`;
    updateMessage.textContent =
      "아직 비교 기준점이 없습니다. 현재 상태를 저장하면 다음 클라이언트 업데이트부터 새로 추가된 자산을 찾을 수 있습니다.";
    return;
  }
  if (status.state === "unchanged") {
    updateStatus.textContent = `${formatBaselineDate(status.baselineCreatedAtUnixSeconds)} 기준`;
    updateMessage.textContent = `저장된 기준점과 현재 ${formatNumber(status.currentCount)}개 자산이 같습니다.`;
    return;
  }
  if (status.state === "changes_detected") {
    updateStatus.textContent = `${formatBaselineDate(status.baselineCreatedAtUnixSeconds)} 이후 변경`;
    updateMessage.textContent =
      "이미지 또는 텍스트·이미지 연결 변경을 감지했습니다. 검토하기 전에는 저장된 기준점을 바꾸지 않습니다.";
    return;
  }

  updateStatus.textContent = `${formatBaselineDate(status.baselineCreatedAtUnixSeconds)} 기준`;
  updateMessage.textContent =
    "저장된 기준점이 현재 게임 폴더와 달라 비교하지 않았습니다. 기존 기준점은 변경하지 않았습니다.";
}

function renderAssetUpdateError(error) {
  currentAssetUpdateStatus = null;
  updatePanel.hidden = false;
  updateStatus.textContent = "확인하지 못함";
  updateMessage.textContent = `업데이트 상태를 확인하지 못했습니다: ${String(error)}`;
  updateCounts.hidden = true;
  updateRemovedRelations.hidden = true;
  updateActions.hidden = true;
  createUpdateBaselineButton.hidden = true;
  viewUpdateAssetsButton.hidden = true;
  refreshUpdateBaselineButton.hidden = true;
}

async function loadAssetUpdateStatus() {
  const requestId = ++updateRequestId;
  currentAssetUpdateStatus = null;
  updatePanel.hidden = false;
  updateStatus.textContent = "확인 중";
  updateMessage.textContent = "저장된 기준점과 현재 클라이언트를 비교하고 있습니다.";
  updateCounts.hidden = true;
  updateRemovedRelations.hidden = true;
  updateActions.hidden = true;
  createUpdateBaselineButton.hidden = true;
  viewUpdateAssetsButton.hidden = true;
  refreshUpdateBaselineButton.hidden = true;

  try {
    const status = await window.__TAURI__.core.invoke(
      "load_asset_update_status",
    );
    if (requestId === updateRequestId) {
      renderAssetUpdateStatus(status);
    }
  } catch (error) {
    if (requestId === updateRequestId) {
      renderAssetUpdateError(error);
    }
  }
}

async function createAssetUpdateBaseline() {
  const requestId = ++updateRequestId;
  setDirectorySelectionBusy(true);
  createUpdateBaselineButton.disabled = true;
  createUpdateBaselineButton.textContent = "저장 중…";
  updateStatus.textContent = "기준점 저장 중";
  updateMessage.textContent = "현재 자산 목록을 안전하게 저장하고 있습니다.";

  try {
    const status = await window.__TAURI__.core.invoke(
      "create_asset_update_baseline",
    );
    if (requestId === updateRequestId) {
      renderAssetUpdateStatus(status);
    }
  } catch (error) {
    if (requestId === updateRequestId) {
      renderAssetUpdateError(error);
    }
  } finally {
    if (requestId === updateRequestId) {
      setDirectorySelectionBusy(false);
    }
  }
}

function dismissUpdateGallery() {
  if (currentPage?.mode !== "update") {
    return;
  }
  rememberedPageOffsets.delete("update");
  galleryRequestId += 1;
  closeDetail();
  currentPage = null;
  galleryPanel.hidden = false;
  galleryTitle.textContent = "카테고리를 선택해 주세요";
  galleryStatus.textContent = "";
  galleryGrid.replaceChildren();
  galleryGrid.dataset.status = "empty";
  setPageNavigationHidden(true);
  setPageNavigationBusy(true);
  toggleSelectionButton.hidden = true;
  saveAllButton.hidden = true;
}

async function refreshAssetUpdateBaseline() {
  const previous = currentAssetUpdateStatus;
  if (
    previous === null ||
    (previous.state !== "changes_detected" &&
      previous.state !== "different_directory")
  ) {
    return;
  }
  const confirmation =
    previous.state === "different_directory"
      ? "기존 게임 폴더의 기준점을 현재 선택한 폴더 기준으로 교체합니다. 계속할까요?"
      : "현재 상태를 새 기준점으로 저장하면 지금 표시된 이미지·연결 변경 내역은 다시 볼 수 없습니다. 검토를 마쳤다면 계속하세요.";
  if (!window.confirm(confirmation)) {
    return;
  }

  const requestId = ++updateRequestId;
  setDirectorySelectionBusy(true);
  createUpdateBaselineButton.disabled = true;
  viewUpdateAssetsButton.disabled = true;
  refreshUpdateBaselineButton.disabled = true;
  refreshUpdateBaselineButton.textContent = "갱신 중…";
  updateStatus.textContent = "기준점 갱신 중";
  updateMessage.textContent = "현재 자산 목록을 새 기준점으로 안전하게 저장하고 있습니다.";

  try {
    const status = await window.__TAURI__.core.invoke(
      "refresh_asset_update_baseline",
    );
    if (requestId === updateRequestId) {
      dismissUpdateGallery();
      renderAssetUpdateStatus(status);
    }
  } catch (error) {
    if (requestId === updateRequestId) {
      renderAssetUpdateError(error);
    }
  } finally {
    if (requestId === updateRequestId) {
      setDirectorySelectionBusy(false);
    }
  }
}

function categoryTreeNode(segment, path) {
  return {
    segment,
    path,
    category: null,
    children: new Map(),
    assetCount: 0,
  };
}

function categoryTree(categories) {
  const roots = new Map();
  const sorted = [...categories].sort((left, right) =>
    left.path.join("\u0000").localeCompare(right.path.join("\u0000"), "ko"),
  );

  for (const category of sorted) {
    let path = [];
    let children = roots;
    let node = null;
    for (const segment of category.path) {
      path = [...path, segment];
      if (!children.has(segment)) {
        children.set(segment, categoryTreeNode(segment, path));
      }
      node = children.get(segment);
      children = node.children;
    }
    node.category = category;
  }

  function totalAssets(node) {
    node.assetCount = node.category?.assetCount ?? 0;
    for (const child of node.children.values()) {
      node.assetCount += totalAssets(child);
    }
    return node.assetCount;
  }
  for (const root of roots.values()) {
    totalAssets(root);
  }
  return [...roots.values()];
}

function appendCategoryButton(list, labelText, category) {
  const item = document.createElement("li");
  const button = document.createElement("button");
  const label = document.createElement("strong");
  const assetCount = document.createElement("span");
  button.type = "button";
  button.className = "category-button";
  button.dataset.path = JSON.stringify(category.path);
  button.setAttribute("aria-pressed", "false");
  label.textContent = labelText;
  assetCount.textContent = `${formatNumber(category.assetCount)}개`;
  button.append(label, assetCount);
  button.addEventListener("click", () => {
    loadCategoryPage(
      category.path,
      rememberedPageOffset({ mode: "category", path: category.path }),
    );
  });
  item.append(button);
  list.append(item);
}

function appendCategoryBranch(list, node) {
  const item = document.createElement("li");
  const details = document.createElement("details");
  const summary = document.createElement("summary");
  const label = document.createElement("strong");
  const assetCount = document.createElement("span");
  const children = document.createElement("ul");
  details.className = "category-tree-branch";
  details.dataset.path = JSON.stringify(node.path);
  label.textContent = node.segment;
  assetCount.textContent = `${formatNumber(node.assetCount)}개`;
  children.className = "category-tree-list";
  summary.append(label, assetCount);

  if (node.category !== null) {
    appendCategoryButton(children, "전체", node.category);
  }
  for (const child of node.children.values()) {
    if (child.children.size > 0) {
      appendCategoryBranch(children, child);
    } else if (child.category !== null) {
      appendCategoryButton(children, child.segment, child.category);
    }
  }

  details.append(summary, children);
  item.append(details);
  list.append(item);
}

function renderCategories(categories) {
  categoryGroups.replaceChildren();
  const list = document.createElement("ul");
  list.className = "category-tree category-tree-list";

  for (const root of categoryTree(categories)) {
    if (root.category !== null && root.children.size === 0) {
      appendCategoryButton(list, root.segment, root.category);
    } else {
      appendCategoryBranch(list, root);
    }
  }

  categoryGroups.append(list);

  const totalAssets = categories.reduce(
    (total, category) => total + category.assetCount,
    0,
  );
  categoryStatus.textContent = `${formatNumber(categories.length)}개 카테고리 · ${formatNumber(totalAssets)}개 이미지`;
  categoryPanel.hidden = false;
  galleryPanel.hidden = false;
  galleryGrid.dataset.status = "empty";
}

function setSelectedCategory(path) {
  const selected = path.join("\u0000");
  for (const button of categoryGroups.querySelectorAll(".category-button")) {
    const buttonPath = JSON.parse(button.dataset.path ?? "[]");
    button.setAttribute(
      "aria-pressed",
      buttonPath.join("\u0000") === selected ? "true" : "false",
    );
  }
  for (const branch of categoryGroups.querySelectorAll(
    ".category-tree-branch",
  )) {
    const branchPath = JSON.parse(branch.dataset.path ?? "[]");
    if (
      branchPath.length <= path.length &&
      branchPath.every((segment, index) => path[index] === segment)
    ) {
      branch.open = true;
    }
  }
  categoryGroups
    .querySelector('.category-button[aria-pressed="true"]')
    ?.scrollIntoView({ block: "nearest" });
}

function renderGallery(page) {
  currentPage = page;
  rememberPage(page);
  galleryGrid.replaceChildren();
  galleryGrid.dataset.status = "ready";
  setPageNavigationHidden(false);
  const searchMode = page.mode === "search";
  const updateMode = page.mode === "update";
  const pathItemMode = searchMode || updateMode;
  galleryTitle.textContent = updateMode
    ? "이번 업데이트 신규"
    : searchMode
      ? `검색: ${page.query}`
      : page.path.join(" > ");

  for (const [index, entry] of page.items.entries()) {
    const path = pathItemMode ? entry.path : page.path;
    const item = pathItemMode ? entry.thumbnail : entry;
    const key = selectionKey(path, item);
    const selected = selectedAssets.has(key);
    const card = document.createElement("div");
    const button = document.createElement("button");
    const frame = document.createElement("div");
    const image = document.createElement("img");
    const caption = document.createElement("span");
    const marker = document.createElement("span");
    const detailButton = document.createElement("button");
    const position = page.offset + index + 1;
    card.className = "gallery-item";
    card.dataset.selected = String(selected);
    card.dataset.selectionMode = String(selectionMode);
    button.type = "button";
    button.className = "gallery-item-main";
    button.disabled = categoryExportBusy;
    button.setAttribute("aria-pressed", String(selected));
    button.setAttribute(
      "aria-label",
      selectionMode
        ? `${path.at(-1)} 이미지 ${position} ${selected ? "선택 해제" : "선택"}`
        : `${path.at(-1)} 이미지 ${position} 상세 보기`,
    );
    frame.className = "thumbnail-frame";
    caption.className = "gallery-caption";
    marker.className = "selection-marker";
    marker.textContent = "✓";
    marker.hidden = !selectionMode && !selected;
    marker.setAttribute("aria-hidden", "true");
    detailButton.type = "button";
    detailButton.className = "secondary-button gallery-detail-button";
    detailButton.textContent = "상세";
    detailButton.hidden = !selectionMode;
    detailButton.disabled = categoryExportBusy;
    detailButton.setAttribute(
      "aria-label",
      `${path.at(-1)} 이미지 ${position} 상세 보기`,
    );
    image.src = item.thumbnailDataUrl;
    image.alt = `${path.at(-1)} 이미지 ${position}`;
    image.width = item.thumbnailWidth;
    image.height = item.thumbnailHeight;
    image.dataset.upscaled = String(
      Math.max(item.sourceWidth, item.sourceHeight) <= 128,
    );
    image.loading = "lazy";
    image.decoding = "async";
    const linkedTexts = item.textLinks ?? [];
    const linkedName = document.createElement("strong");
    linkedName.textContent = linkedTexts.length
      ? linkedTexts
          .slice(0, 2)
          .map((link) => link.name || `ID ${formatNumber(link.id)}`)
          .join(" · ")
      : item.iconId === null
        ? `${path.at(-1)} 이미지 ${position}`
        : `ID ${formatNumber(item.iconId)}`;
    caption.append(linkedName);
    frame.append(image);
    button.append(frame, caption);
    button.addEventListener("click", () => {
      if (!selectionMode) {
        loadAssetDetail(path, item, position);
        return;
      }
      if (selectedAssets.has(key)) {
        selectedAssets.delete(key);
      } else {
        selectedAssets.set(key, selectionEntry(path, item));
      }
      const isSelected = selectedAssets.has(key);
      card.dataset.selected = String(isSelected);
      button.setAttribute("aria-pressed", String(isSelected));
      button.setAttribute(
        "aria-label",
        `${path.at(-1)} 이미지 ${position} ${isSelected ? "선택 해제" : "선택"}`,
      );
      updateSelectionControls();
    });
    detailButton.addEventListener("click", () => {
      loadAssetDetail(path, item, position);
    });
    card.append(marker, button, detailButton);
    galleryGrid.append(card);
  }

  galleryStatus.textContent = galleryPageStatus(page);
  renderPageNavigation(page);
  toggleSelectionButton.hidden = false;
  saveAllButton.hidden = updateMode;
  saveAllButton.disabled =
    updateMode || categoryExportBusy || page.items.length === 0;
  saveAllButton.textContent = searchMode
    ? "검색 결과 전체 저장"
    : "카테고리 전체 저장";
  updateSelectionControls();
}

function galleryPageStatus(page) {
  if (page.totalCount === 0) {
    return page.mode === "update" && page.reviewRequiredCount > 0
      ? `Viewer에 표시할 검증 이미지가 없습니다 · 분류 검토 필요 ${formatNumber(page.reviewRequiredCount)}개`
      : "표시할 이미지가 없습니다";
  }
  const first = page.offset + 1;
  const last = page.offset + page.items.length;
  const range = `${formatNumber(first)}–${formatNumber(last)} / ${formatNumber(page.totalCount)}개`;
  return page.mode === "update" && page.reviewRequiredCount > 0
    ? `${range} · 분류 검토 필요 ${formatNumber(page.reviewRequiredCount)}개`
    : range;
}

function selectCurrentPage() {
  for (const { key, value } of currentPageSelectionEntries()) {
    selectedAssets.set(key, value);
  }
  selectionMode = true;
  if (currentPage !== null) {
    renderGallery(currentPage);
  }
}

function clearCurrentPageSelection() {
  for (const { key } of currentPageSelectionEntries()) {
    selectedAssets.delete(key);
  }
  if (currentPage !== null) {
    renderGallery(currentPage);
  }
}

function scheduleCategoryExportPoll() {
  clearCategoryExportPoll();
  categoryExportPollTimer = window.setTimeout(
    pollCategoryExport,
    CATEGORY_EXPORT_POLL_INTERVAL,
  );
}

function finishCategoryExport(message) {
  clearCategoryExportPoll();
  currentCategoryExport = null;
  categoryExportCancelRequested = false;
  categoryExportStatus.textContent = message;
  cancelCategoryExportButton.disabled = true;
  cancelCategoryExportButton.textContent = "저장 취소";
  setCategoryExportBusy(false);
  setWindowStatus("ready", message);
}

async function pollCategoryExport() {
  if (currentCategoryExport === null) {
    return;
  }
  const requestedExport = currentCategoryExport;

  try {
    const status = await window.__TAURI__.core.invoke(
      "get_verified_asset_export_status",
      { jobId: requestedExport.jobId },
    );
    if (currentCategoryExport !== requestedExport) {
      return;
    }
    categoryExportProgress.max = Math.max(1, status.totalCount);
    categoryExportProgress.value = status.completedCount;

    if (status.state === "running") {
      categoryExportStatus.textContent = categoryExportCancelRequested
        ? `${formatNumber(status.completedCount)} / ${formatNumber(status.totalCount)}개 · 취소 준비 중`
        : `${formatNumber(status.completedCount)} / ${formatNumber(status.totalCount)}개 저장 중`;
      setWindowStatus(
        "busy",
        categoryExportCancelRequested
          ? "이미지 저장 취소 준비 중"
          : `이미지 ${formatNumber(status.completedCount)} / ${formatNumber(status.totalCount)}개 저장 중`,
      );
      scheduleCategoryExportPoll();
      return;
    }
    if (status.state === "completed") {
      finishCategoryExport(
        `${formatNumber(status.completedCount)}개 이미지 저장 완료`,
      );
      return;
    }
    if (status.state === "cancelled") {
      categoryExportProgress.value = 0;
      finishCategoryExport("저장을 취소했고 생성한 파일을 정리했습니다.");
      return;
    }
    categoryExportProgress.value = 0;
    finishCategoryExport(
      `저장하지 못했습니다: ${status.error ?? "알 수 없는 오류"}`,
    );
  } catch (error) {
    finishCategoryExport(
      `저장 상태를 확인하지 못했습니다: ${String(error)}`,
    );
  }
}

async function startAssetExport() {
  if (
    currentPage === null ||
    currentPage.mode === "update" ||
    currentPage.items.length === 0 ||
    categoryExportBusy
  ) {
    return;
  }
  const requestedPage = currentPage;
  const exportMode = currentImageExportMode();
  hideCategoryExport();
  categoryExport.hidden = false;
  categoryExportStatus.textContent = "저장할 폴더를 선택해 주세요";
  setWindowStatus("busy", "저장할 폴더 선택 중");
  cancelCategoryExportButton.disabled = true;
  setCategoryExportBusy(true);

  try {
    const searchMode = requestedPage.mode === "search";
    const started = await window.__TAURI__.core.invoke(
      searchMode
        ? "start_verified_search_export"
        : "start_verified_category_export",
      searchMode
        ? { query: requestedPage.query, exportMode }
        : { path: requestedPage.path, exportMode },
    );
    if (currentPage !== requestedPage) {
      return;
    }
    if (started === null) {
      hideCategoryExport();
      setCategoryExportBusy(false);
      setWindowStatus("ready", "게임 폴더 연결됨");
      return;
    }
    currentCategoryExport = started;
    categoryExportProgress.max = Math.max(1, started.totalCount);
    categoryExportProgress.value = 0;
    categoryExportStatus.textContent = `0 / ${formatNumber(started.totalCount)}개 ${imageExportModeLabel(exportMode)}로 저장 중`;
    setWindowStatus("busy", `이미지 0 / ${formatNumber(started.totalCount)}개 저장 중`);
    cancelCategoryExportButton.disabled = false;
    scheduleCategoryExportPoll();
  } catch (error) {
    currentCategoryExport = null;
    categoryExportStatus.textContent = `전체 저장을 시작하지 못했습니다: ${String(error)}`;
    cancelCategoryExportButton.disabled = true;
    setCategoryExportBusy(false);
    setWindowStatus("error", "이미지 저장 실패");
  }
}

async function startSelectedAssetExport() {
  if (selectedAssets.size === 0 || categoryExportBusy) {
    return;
  }
  hideCategoryExport();
  categoryExport.hidden = false;
  categoryExportStatus.textContent = "저장할 폴더를 선택해 주세요";
  setWindowStatus("busy", "저장할 폴더 선택 중");
  cancelCategoryExportButton.disabled = true;
  setCategoryExportBusy(true);
  const exportMode = currentImageExportMode();

  try {
    const started = await window.__TAURI__.core.invoke(
      "start_verified_selected_export",
      { assets: [...selectedAssets.values()], exportMode },
    );
    if (started === null) {
      hideCategoryExport();
      setCategoryExportBusy(false);
      setWindowStatus("ready", "게임 폴더 연결됨");
      return;
    }
    currentCategoryExport = started;
    categoryExportProgress.max = Math.max(1, started.totalCount);
    categoryExportProgress.value = 0;
    categoryExportStatus.textContent = `0 / ${formatNumber(started.totalCount)}개 ${imageExportModeLabel(exportMode)}로 저장 중`;
    setWindowStatus("busy", `이미지 0 / ${formatNumber(started.totalCount)}개 저장 중`);
    cancelCategoryExportButton.disabled = false;
    scheduleCategoryExportPoll();
  } catch (error) {
    currentCategoryExport = null;
    categoryExportStatus.textContent = `선택 이미지 저장을 시작하지 못했습니다: ${String(error)}`;
    cancelCategoryExportButton.disabled = true;
    setCategoryExportBusy(false);
    setWindowStatus("error", "이미지 저장 실패");
  }
}

async function cancelCategoryExport() {
  if (currentCategoryExport === null) {
    return;
  }
  const requestedExport = currentCategoryExport;
  categoryExportCancelRequested = true;
  cancelCategoryExportButton.disabled = true;
  cancelCategoryExportButton.textContent = "취소 중…";
  categoryExportStatus.textContent = "현재 이미지 처리가 끝나면 취소합니다";
  setWindowStatus("busy", "이미지 저장 취소 준비 중");

  try {
    await window.__TAURI__.core.invoke("cancel_verified_asset_export", {
      jobId: requestedExport.jobId,
    });
  } catch (error) {
    if (currentCategoryExport === requestedExport) {
      categoryExportCancelRequested = false;
      categoryExportStatus.textContent = `취소를 요청하지 못했습니다: ${String(error)}`;
      cancelCategoryExportButton.disabled = false;
      cancelCategoryExportButton.textContent = "저장 취소";
    }
  }
}

function resetDetail() {
  currentDetail = null;
  downloadDetailButton.disabled = true;
  downloadDetailButton.textContent = "PNG 저장";
  detailContent.dataset.status = "idle";
  detailPreview.hidden = true;
  detailPreview.removeAttribute("src");
  detailPreview.removeAttribute("style");
  detailPreview.removeAttribute("data-upscaled");
  detailPreview.alt = "";
  detailMessage.textContent = "";
  detailRecordList.replaceChildren();
  detailRecordList.hidden = true;
  detailMetadata.hidden = true;
  detailSourceSize.textContent = "";
  detailPreviewSize.textContent = "";
  detailGmSource.hidden = true;
  detailGmSourceLabel.textContent = "";
  detailGmSourceImage.removeAttribute("src");
  detailGmSourceImage.alt = "";
  detailGmSourceHighlight.removeAttribute("style");
}

function showDetailPreview(detail, alt) {
  const largestSide = Math.max(detail.previewWidth, detail.previewHeight);
  const scale = largestSide <= 64 ? 4 : largestSide <= 128 ? 3 : largestSide <= 192 ? 2 : 1;
  detailPreview.src = detail.previewDataUrl;
  detailPreview.alt = alt;
  detailPreview.width = detail.previewWidth;
  detailPreview.height = detail.previewHeight;
  detailPreview.style.width = `${detail.previewWidth * scale}px`;
  detailPreview.style.height = `${detail.previewHeight * scale}px`;
  detailPreview.dataset.upscaled = String(scale > 1);
  detailPreview.hidden = false;
}

function renderDetailRecords(records) {
  detailRecordList.replaceChildren();
  detailRecordList.hidden = records.length === 0;
  for (const record of records) {
    const section = document.createElement("section");
    const name = document.createElement("h3");
    const description = document.createElement("p");
    const attributes = document.createElement("div");
    const source = document.createElement("p");
    const fields = record.fields ?? [];
    const attributeStart = record.description ? 2 : 1;
    section.className = "detail-record";
    name.textContent = record.name || `ID ${formatNumber(record.id)}`;
    description.className = "detail-record-description";
    description.textContent = record.description || "설명 없음";
    attributes.className = "detail-attribute-list";
    for (const field of fields.slice(attributeStart)) {
      const value = document.createElement("span");
      value.textContent = field === "" ? "(빈 값)" : field;
      attributes.append(value);
    }
    source.className = "detail-record-source";
    source.textContent = [
      record.sourceLabel,
      record.relation,
      relationEvidenceLabel(record.evidenceType),
      relationVerificationLabel(record.verificationStatus),
      Number.isFinite(record.id) ? `ID ${formatNumber(record.id)}` : null,
    ]
      .filter(Boolean)
      .join(" · ");
    section.append(name, description);
    if (attributes.childElementCount > 0) {
      section.append(attributes);
    }
    section.append(source);
    detailRecordList.append(section);
  }
}

function renderGmSource(source) {
  if (!source) {
    detailGmSource.hidden = true;
    return;
  }

  const left = (source.x / source.atlasWidth) * 100;
  const top = (source.y / source.atlasHeight) * 100;
  const width = (source.width / source.atlasWidth) * 100;
  const height = (source.height / source.atlasHeight) * 100;
  detailGmSourceLabel.textContent = [
    `아틀라스 ID ${String(source.atlasBlockIndex).padStart(2, "0")}`,
    `${formatNumber(source.atlasWidth)} × ${formatNumber(source.atlasHeight)}`,
    `x ${formatNumber(source.x)}, y ${formatNumber(source.y)}`,
    `${formatNumber(source.width)} × ${formatNumber(source.height)}`,
  ].join(" · ");
  detailGmSourceImage.src = source.previewDataUrl;
  detailGmSourceImage.alt = `GM 원본 아틀라스 ID ${String(source.atlasBlockIndex).padStart(2, "0")}`;
  detailGmSourceHighlight.style.left = `${left}%`;
  detailGmSourceHighlight.style.top = `${top}%`;
  detailGmSourceHighlight.style.width = `${width}%`;
  detailGmSourceHighlight.style.height = `${height}%`;
  detailGmSource.hidden = false;
}

function closeDetail() {
  detailRequestId += 1;
  resetDetail();
  if (detailDialog.open) {
    detailDialog.close();
  }
}

async function loadAssetDetail(path, item, position) {
  const requestId = ++detailRequestId;
  resetDetail();
  const linkedName = item.textLinks?.find((link) => link.name)?.name;
  detailTitle.textContent = linkedName ?? `${path.at(-1)} 이미지 ${position}`;
  detailContent.dataset.status = "loading";
  detailMessage.textContent = "선택한 이미지를 불러오는 중입니다";
  if (!detailDialog.open) {
    detailDialog.showModal();
  }

  try {
    const detail = await window.__TAURI__.core.invoke(
      "load_verified_asset_detail",
      {
        path,
        archive: item.archive,
        blockIndex: item.blockIndex,
      },
    );
    if (requestId !== detailRequestId) {
      return;
    }
    detailContent.dataset.status = "ready";
    showDetailPreview(
      detail,
      `${linkedName ?? detail.path.at(-1)} 미리보기`,
    );
    renderGmSource(detail.gmSource);
    const linkedRecords = item.textLinks ?? [];
    renderDetailRecords(linkedRecords);
    detailMessage.textContent = linkedRecords.length
      ? ""
      : detail.assembled
        ? "검증된 조립 완성본입니다."
        : "선택한 이미지의 큰 미리보기입니다.";
    detailSourceSize.textContent = `${formatNumber(detail.sourceWidth)} × ${formatNumber(detail.sourceHeight)}`;
    detailPreviewSize.textContent = `${formatNumber(detail.previewWidth)} × ${formatNumber(detail.previewHeight)}`;
    detailMetadata.hidden = false;
    currentDetail = {
      path: detail.path,
      archive: detail.archive,
      blockIndex: detail.blockIndex,
    };
    downloadDetailButton.disabled = false;
  } catch (error) {
    if (requestId === detailRequestId) {
      detailContent.dataset.status = "error";
      detailMessage.textContent = String(error);
    }
  }
}

async function loadTextImageDetail(link, item) {
  const requestId = ++detailRequestId;
  resetDetail();
  const recordName = item.fields.find((field) => field !== "") ?? `ID ${item.id}`;
  detailTitle.textContent = `${recordName} · ${link.relation}`;
  detailContent.dataset.status = "loading";
  detailMessage.textContent = "텍스트에 연결된 이미지를 불러오는 중입니다";
  if (!detailDialog.open) {
    detailDialog.showModal();
  }

  try {
    const detail = await window.__TAURI__.core.invoke(
      "load_text_image_detail",
      {
        archive: link.archive,
        groupCode: link.groupCode,
        iconId: link.iconId,
        relation: link.relation,
      },
    );
    if (requestId !== detailRequestId) {
      return;
    }
    detailContent.dataset.status = "ready";
    showDetailPreview(detail, `${recordName} ${link.relation} 미리보기`);
    renderGmSource(detail.gmSource);
    renderDetailRecords([
      {
        name: recordName,
        description: item.fields[1] ?? "",
        fields: item.fields,
        sourceLabel: item.sourceLabel,
        relation: link.relation,
        id: item.id,
      },
    ]);
    detailMessage.textContent = "";
    detailSourceSize.textContent = `${formatNumber(detail.sourceWidth)} × ${formatNumber(detail.sourceHeight)}`;
    detailPreviewSize.textContent = `${formatNumber(detail.previewWidth)} × ${formatNumber(detail.previewHeight)}`;
    detailMetadata.hidden = false;
  } catch (error) {
    if (requestId === detailRequestId) {
      detailContent.dataset.status = "error";
      detailMessage.textContent = String(error);
    }
  }
}

async function saveCurrentDetail() {
  if (currentDetail === null) {
    return;
  }
  const requestedDetail = currentDetail;
  downloadDetailButton.disabled = true;
  downloadDetailButton.textContent = "저장 중…";
  detailMessage.textContent = "저장할 위치를 선택해 주세요.";

  try {
    const saved = await window.__TAURI__.core.invoke(
      "save_verified_asset_png",
      { ...requestedDetail, exportMode: currentImageExportMode() },
    );
    if (currentDetail !== requestedDetail) {
      return;
    }
    if (saved === null) {
      detailMessage.textContent = "저장을 취소했습니다.";
      return;
    }
    const resized =
      saved.sourceWidth !== saved.width || saved.sourceHeight !== saved.height;
    const sizeSummary = resized
      ? ` (${formatNumber(saved.sourceWidth)} × ${formatNumber(saved.sourceHeight)} → ${formatNumber(saved.width)} × ${formatNumber(saved.height)})`
      : ` (${formatNumber(saved.width)} × ${formatNumber(saved.height)})`;
    detailMessage.textContent = `${saved.fileName} 파일로 저장했습니다${sizeSummary}.`;
  } catch (error) {
    if (currentDetail === requestedDetail) {
      detailMessage.textContent = `저장하지 못했습니다: ${String(error)}`;
    }
  } finally {
    if (currentDetail === requestedDetail) {
      downloadDetailButton.disabled = false;
      downloadDetailButton.textContent = "PNG 저장";
    }
  }
}

async function loadCategoryPage(path, offset) {
  const requestId = ++galleryRequestId;
  if (!categoryExportBusy) {
    hideCategoryExport();
  }
  currentPage = null;
  for (const button of categoryGroups.querySelectorAll(".category-button")) {
    button.disabled = true;
  }
  assetSearchQuery.disabled = true;
  assetSearchSubmit.disabled = true;
  setSelectedCategory(path);
  galleryPanel.hidden = false;
  setPageNavigationLoading();
  galleryTitle.textContent = path.join(" > ");
  galleryStatus.textContent = "썸네일을 불러오는 중입니다";
  toggleSelectionButton.hidden = false;
  toggleSelectionButton.disabled = true;
  saveAllButton.disabled = true;
  galleryGrid.dataset.status = "loading";
  galleryGrid.replaceChildren();

  try {
    const page = await window.__TAURI__.core.invoke(
      "load_verified_category_page",
      { languageBlock: currentTextLanguageBlock, path, offset },
    );
    if (requestId === galleryRequestId) {
      renderGallery({ ...page, mode: "category" });
    }
  } catch (error) {
    if (requestId === galleryRequestId) {
      if (pageOffsetIsOutOfRange(error, offset)) {
        rememberedPageOffsets.delete(
          pageMemoryKey({ mode: "category", path }),
        );
        loadCategoryPage(path, 0);
        return;
      }
      currentPage = null;
      galleryGrid.dataset.status = "error";
      galleryStatus.textContent = `이미지를 불러오지 못했습니다: ${String(error)}`;
      setPageNavigationHidden(true);
    }
  } finally {
    if (requestId === galleryRequestId) {
      for (const button of categoryGroups.querySelectorAll(
        ".category-button",
      )) {
        button.disabled = categoryExportBusy;
      }
      assetSearchQuery.disabled = categoryExportBusy;
      assetSearchSubmit.disabled = categoryExportBusy;
    }
  }
}

async function loadSearchPage(query, offset) {
  const requestId = ++galleryRequestId;
  if (!categoryExportBusy) {
    hideCategoryExport();
  }
  currentPage = null;
  for (const button of categoryGroups.querySelectorAll(".category-button")) {
    button.disabled = true;
    button.setAttribute("aria-pressed", "false");
  }
  assetSearchQuery.disabled = true;
  assetSearchSubmit.disabled = true;
  galleryPanel.hidden = false;
  setPageNavigationLoading();
  galleryTitle.textContent = `검색: ${query}`;
  galleryStatus.textContent = "검색 결과를 불러오는 중입니다";
  toggleSelectionButton.hidden = false;
  toggleSelectionButton.disabled = true;
  saveAllButton.disabled = true;
  galleryGrid.dataset.status = "loading";
  galleryGrid.replaceChildren();

  try {
    const page = await window.__TAURI__.core.invoke(
      "load_verified_asset_search_page",
      { languageBlock: currentTextLanguageBlock, query, offset },
    );
    if (requestId === galleryRequestId) {
      renderGallery({ ...page, mode: "search" });
    }
  } catch (error) {
    if (requestId === galleryRequestId) {
      if (pageOffsetIsOutOfRange(error, offset)) {
        rememberedPageOffsets.delete(
          pageMemoryKey({ mode: "search", query }),
        );
        loadSearchPage(query, 0);
        return;
      }
      currentPage = null;
      galleryGrid.dataset.status = "error";
      galleryStatus.textContent = `검색 결과를 불러오지 못했습니다: ${String(error)}`;
      setPageNavigationHidden(true);
    }
  } finally {
    if (requestId === galleryRequestId) {
      for (const button of categoryGroups.querySelectorAll(
        ".category-button",
      )) {
        button.disabled = categoryExportBusy;
      }
      assetSearchQuery.disabled = categoryExportBusy;
      assetSearchSubmit.disabled = categoryExportBusy;
    }
  }
}

async function loadUpdatePage(offset) {
  const requestId = ++galleryRequestId;
  if (!categoryExportBusy) {
    hideCategoryExport();
  }
  currentPage = null;
  for (const button of categoryGroups.querySelectorAll(".category-button")) {
    button.disabled = true;
    button.setAttribute("aria-pressed", "false");
  }
  assetSearchQuery.disabled = true;
  assetSearchSubmit.disabled = true;
  galleryPanel.hidden = false;
  setPageNavigationLoading();
  galleryTitle.textContent = "이번 업데이트 신규";
  galleryStatus.textContent = "신규 이미지를 불러오는 중입니다";
  toggleSelectionButton.hidden = false;
  saveAllButton.hidden = true;
  toggleSelectionButton.disabled = true;
  saveAllButton.disabled = true;
  galleryGrid.dataset.status = "loading";
  galleryGrid.replaceChildren();

  try {
    const page = await window.__TAURI__.core.invoke(
      "load_verified_update_page",
      { languageBlock: currentTextLanguageBlock, offset },
    );
    if (requestId === galleryRequestId) {
      renderGallery({ ...page, mode: "update" });
    }
  } catch (error) {
    if (requestId === galleryRequestId) {
      if (pageOffsetIsOutOfRange(error, offset)) {
        rememberedPageOffsets.delete("update");
        loadUpdatePage(0);
        return;
      }
      currentPage = null;
      galleryGrid.dataset.status = "error";
      galleryStatus.textContent = `신규 이미지를 불러오지 못했습니다: ${String(error)}`;
      setPageNavigationHidden(true);
    }
  } finally {
    if (requestId === galleryRequestId) {
      for (const button of categoryGroups.querySelectorAll(
        ".category-button",
      )) {
        button.disabled = categoryExportBusy;
      }
      assetSearchQuery.disabled = categoryExportBusy;
      assetSearchSubmit.disabled = categoryExportBusy;
    }
  }
}

function loadPage(page, offset) {
  if (page.mode === "update") {
    loadUpdatePage(offset);
  } else if (page.mode === "search") {
    loadSearchPage(page.query, offset);
  } else {
    loadCategoryPage(page.path, offset);
  }
}

function navigateToPage(pageNumber) {
  if (currentPage === null || categoryExportBusy) {
    return;
  }
  const requestedPage = currentPage;
  const pageTotal = totalPages(requestedPage);
  if (pageTotal === 0 || !Number.isFinite(pageNumber)) {
    renderPageNavigation(requestedPage);
    return;
  }
  const targetPage = Math.min(
    pageTotal,
    Math.max(1, Math.trunc(pageNumber)),
  );
  const targetOffset = (targetPage - 1) * requestedPage.pageSize;
  if (targetOffset === requestedPage.offset) {
    renderPageNavigation(requestedPage);
    return;
  }
  scrollGalleryToTop();
  loadPage(requestedPage, targetOffset);
}

function formatNumber(value) {
  return new Intl.NumberFormat("ko-KR").format(value);
}

function relationEvidenceLabel(value) {
  const labels = {
    master_type_rule: "근거: 마스터 타입 규칙",
    explicit_record_reference: "근거: 레코드 참조 필드",
    visual_review: "근거: 시각 검증",
  };
  return labels[value] ?? "근거: 미기록";
}

function relationVerificationLabel(value) {
  const labels = {
    candidate: "상태: 후보",
    human_verified: "상태: 사용자 검증",
    rejected: "상태: 폐기",
  };
  return labels[value] ?? "상태: 미기록";
}

function formatBytes(value) {
  if (!Number.isFinite(value) || value <= 0) {
    return "0 B";
  }
  const units = ["B", "KB", "MB", "GB"];
  const unitIndex = Math.min(
    Math.floor(Math.log(value) / Math.log(1024)),
    units.length - 1,
  );
  const scaled = value / 1024 ** unitIndex;
  return `${scaled.toLocaleString("ko-KR", {
    maximumFractionDigits: unitIndex === 0 ? 0 : 1,
  })} ${units[unitIndex]}`;
}

function setAppUpdateBusy(busy) {
  appUpdateBusy = busy;
  checkAppUpdateButton.disabled = busy;
  installAppUpdateButton.disabled = busy;
  appUpdateBannerInstallButton.disabled = busy;
  renderStatusAppUpdate();
}

function showAppUpdateReleaseNotes(notes) {
  const normalizedNotes = notes?.trim() ?? "";
  appUpdateReleaseNotes.open = false;
  appUpdateReleaseNotesContent.textContent = normalizedNotes;
  appUpdateReleaseNotes.hidden = normalizedNotes.length === 0;
}

function showAvailableAppUpdate(result) {
  availableAppUpdate = result.update;
  appCurrentVersion.textContent = `현재 ${result.currentVersion}`;

  if (availableAppUpdate === null) {
    appUpdateBanner.hidden = true;
    installAppUpdateButton.hidden = true;
    showAppUpdateReleaseNotes(null);
    appUpdateMessage.textContent = "현재 최신 버전을 사용하고 있습니다.";
    renderStatusAppUpdate();
    return;
  }

  appUpdateBannerTitle.textContent = `DHO Vault ${availableAppUpdate.version} 업데이트`;
  appUpdateBannerMessage.textContent = `현재 ${result.currentVersion}에서 새 버전으로 업데이트할 수 있습니다.`;
  appUpdateBanner.hidden = false;
  installAppUpdateButton.hidden = false;
  showAppUpdateReleaseNotes(availableAppUpdate.notes);
  appUpdateMessage.textContent = availableAppUpdate.notes
    ? `${availableAppUpdate.version} 버전을 설치할 수 있습니다. 자세한 변경 사항은 릴리스 노트를 확인해 주세요.`
    : `${availableAppUpdate.version} 버전을 설치할 수 있습니다.`;
  renderStatusAppUpdate();
}

async function loadAppVersion() {
  try {
    const version = await window.__TAURI__.core.invoke("get_app_version");
    appCurrentVersion.textContent = `현재 ${version}`;
    titlebarVersion.textContent = `v${version}`;
  } catch {
    appCurrentVersion.textContent = "현재 버전 확인 불가";
    titlebarVersion.textContent = "";
  }
}

async function checkForAppUpdate({ automatic = false } = {}) {
  if (appUpdateBusy) {
    return;
  }

  appUpdateActivity = "checking";
  setAppUpdateBusy(true);
  showAppUpdateReleaseNotes(null);
  appUpdateMessage.textContent = "새 버전이 있는지 확인하고 있습니다.";
  try {
    const result = await window.__TAURI__.core.invoke("check_app_update");
    showAvailableAppUpdate(result);
  } catch (error) {
    if (!automatic) {
      appUpdateMessage.textContent = `업데이트를 확인하지 못했습니다: ${String(error)}`;
    } else {
      appUpdateMessage.textContent =
        "자동 업데이트 확인을 완료하지 못했습니다. 인터넷 연결 후 다시 확인할 수 있습니다.";
    }
  } finally {
    appUpdateActivity = null;
    setAppUpdateBusy(false);
  }
}

function renderAppUpdateDownloadEvent(message) {
  if (message.event === "started") {
    appUpdateDownloadedBytes = 0;
    appUpdateProgress.hidden = false;
    if (message.data.contentLength === null) {
      appUpdateProgressBar.removeAttribute("value");
      appUpdateProgressLabel.textContent = "업데이트를 내려받는 중입니다.";
    } else {
      appUpdateProgressBar.max = message.data.contentLength;
      appUpdateProgressBar.value = 0;
      appUpdateProgressLabel.textContent = `0 B / ${formatBytes(message.data.contentLength)}`;
    }
    return;
  }

  if (message.event === "progress") {
    appUpdateDownloadedBytes += message.data.chunkLength;
    if (appUpdateProgressBar.hasAttribute("value")) {
      appUpdateProgressBar.value = appUpdateDownloadedBytes;
      appUpdateProgressLabel.textContent = `${formatBytes(appUpdateDownloadedBytes)} / ${formatBytes(appUpdateProgressBar.max)}`;
    } else {
      appUpdateProgressLabel.textContent = `${formatBytes(appUpdateDownloadedBytes)} 내려받음`;
    }
    return;
  }

  if (message.event === "finished") {
    appUpdateProgressLabel.textContent =
      "다운로드를 마쳤습니다. 서명을 확인하고 설치를 시작합니다.";
  }
}

async function installAvailableAppUpdate() {
  if (appUpdateBusy || availableAppUpdate === null) {
    return;
  }

  appUpdateActivity = "installing";
  setAppUpdateBusy(true);
  appUpdateProgress.hidden = false;
  appUpdateProgressBar.removeAttribute("value");
  appUpdateProgressLabel.textContent = "업데이트 다운로드를 준비하고 있습니다.";
  appUpdateMessage.textContent =
    "설치가 시작되면 DHO Vault가 자동으로 종료될 수 있습니다.";

  try {
    const onEvent = new window.__TAURI__.core.Channel();
    onEvent.onmessage = renderAppUpdateDownloadEvent;
    await window.__TAURI__.core.invoke("install_app_update", { onEvent });
    availableAppUpdate = null;
    appUpdateBanner.hidden = true;
    installAppUpdateButton.hidden = true;
    showAppUpdateReleaseNotes(null);
    appUpdateProgressLabel.textContent = "업데이트 설치 프로그램을 시작했습니다.";
    appUpdateMessage.textContent =
      "앱이 자동으로 종료되지 않았다면 창을 닫고 설치를 마쳐 주세요.";
  } catch (error) {
    appUpdateProgress.hidden = true;
    appUpdateMessage.textContent = `업데이트를 설치하지 못했습니다: ${String(error)}`;
  } finally {
    appUpdateActivity = null;
    setAppUpdateBusy(false);
  }
}

function formatBaselineDate(unixSeconds) {
  if (unixSeconds === null) {
    return "저장 시각 없음";
  }
  return new Intl.DateTimeFormat("ko-KR", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(unixSeconds * 1000));
}

function renderSummary(summary) {
  gameDirectory.textContent = summary.gameDirectory;
  resourceDirectory.textContent = summary.resourceDirectory;
  directoryDetails.hidden = false;
  archiveList.replaceChildren();

  for (const archive of summary.archives) {
    const item = document.createElement("li");
    const prefix = document.createElement("strong");
    const detail = document.createElement("span");
    prefix.textContent = archive.prefix.toUpperCase();
    detail.textContent = archive.hasIndex
      ? `레코드 ${formatNumber(archive.recordCount)} · 그룹 ${formatNumber(archive.groupCount)} · 이미지 블록 ${formatNumber(archive.imageBlockCount)} · 데이터 파일 ${formatNumber(archive.archiveCount)}`
      : `원시 이미지 블록 ${formatNumber(archive.imageBlockCount)} · 데이터 파일 ${formatNumber(archive.archiveCount)}`;
    item.append(prefix, detail);
    archiveList.append(item);
  }

  archiveStatus.textContent = `${formatNumber(summary.archives.length)}개 확인`;

  renderCategories(summary.verifiedCategories);
  renderCatalogDiagnostics(summary.catalogDiagnostics);
}

function renderCatalogDiagnostics(diagnostics) {
  if (!diagnostics) {
    catalogDiagnosticsCard.hidden = true;
    return;
  }
  catalogDiagnosticsCard.hidden = false;
  diagnosticsTotalCount.textContent = formatNumber(diagnostics.totalAssetCount);
  diagnosticsCategorizedCount.textContent = formatNumber(
    diagnostics.categorizedAssetCount,
  );
  diagnosticsUnclassifiedCount.textContent = formatNumber(
    diagnostics.unclassifiedAssetCount,
  );
  diagnosticsMultipleCount.textContent = formatNumber(
    diagnostics.multipleCategoryAssetCount,
  );
  catalogDiagnosticsStatus.textContent = diagnostics.multipleCategoryAssetCount > 0
    ? `${formatNumber(diagnostics.multipleCategoryAssetCount)}개 검토 필요`
    : "카테고리 충돌 없음";

  diagnosticsUnclassifiedList.replaceChildren();
  diagnosticsUnclassifiedSummary.textContent =
    `미분류 그룹 ${formatNumber(diagnostics.unclassifiedCategories.length)}개`;
  for (const category of diagnostics.unclassifiedCategories) {
    const item = document.createElement("li");
    const path = document.createElement("strong");
    const count = document.createElement("span");
    path.textContent = category.path.join(" > ");
    count.textContent = `${formatNumber(category.assetCount)}개`;
    item.append(path, count);
    diagnosticsUnclassifiedList.append(item);
  }
  if (diagnostics.unclassifiedCategories.length === 0) {
    diagnosticsUnclassifiedList.textContent = "미분류 이미지가 없습니다.";
  }

  diagnosticsMultipleList.replaceChildren();
  diagnosticsMultipleSummary.textContent =
    `다중 카테고리 ${formatNumber(diagnostics.multipleCategoryAssetCount)}개`;
  for (const asset of diagnostics.multipleCategoryAssets) {
    const item = document.createElement("li");
    const key = document.createElement("strong");
    const paths = document.createElement("span");
    if (asset.identityKind === "인덱스 이미지") {
      key.textContent = `${asset.archive.toUpperCase()} · 그룹 ${asset.primaryId} · ID ${asset.secondaryId}`;
    } else if (asset.identityKind === "원시 블록") {
      key.textContent = `${asset.archive.toUpperCase()} · 파일 ${asset.primaryId} · 블록 ${asset.secondaryId}`;
    } else if (asset.identityKind === "GM 원본 아틀라스") {
      key.textContent = `GM · 파일 ${asset.primaryId} · 아틀라스 ID ${asset.secondaryId}`;
    } else if (asset.identityKind === "GM 스프라이트") {
      key.textContent = `GM · 아틀라스 ID ${asset.primaryId} · 스프라이트 ID ${asset.secondaryId}`;
    } else {
      key.textContent = `${asset.archive.toUpperCase()} · 조립 블록 ${asset.primaryId}`;
    }
    paths.textContent = asset.categoryPaths
      .map((path) => path.join(" > "))
      .join(" ↔ ");
    item.append(key, paths);
    diagnosticsMultipleList.append(item);
  }
  if (diagnostics.multipleCategoryAssetCount === 0) {
    diagnosticsMultipleList.textContent = "다중 카테고리 항목이 없습니다.";
  } else if (
    diagnostics.multipleCategoryAssets.length < diagnostics.multipleCategoryAssetCount
  ) {
    const remainder = document.createElement("li");
    remainder.textContent = `앞의 ${formatNumber(diagnostics.multipleCategoryAssets.length)}개만 표시합니다.`;
    diagnosticsMultipleList.append(remainder);
  }

}

async function renderOpenedGameDirectory(opened, automatic) {
  renderSummary(opened.summary);
  if (opened.warning !== null) {
    settingsMessage.textContent = `게임 리소스는 열었지만 폴더를 기억하지 못했습니다. ${opened.warning} 다음 실행 때 게임 폴더를 다시 선택해야 할 수 있습니다.`;
  } else if (automatic) {
    settingsMessage.textContent = "마지막으로 사용한 게임 폴더를 자동으로 연결했습니다.";
  } else {
    settingsMessage.textContent = "게임 폴더가 연결되어 있습니다.";
  }
  setWindowStatus("ready", "게임 폴더 연결됨");
  showWorkspacePage("library");
  await loadSharedLanguageSelector();
}

async function loadSavedGameDirectory() {
  setDirectorySelectionBusy(true);
  clearSummary();
  showConnectionView();
  setStatus(
    "loading",
    "마지막 게임 폴더를 확인하는 중입니다",
    "저장된 폴더가 없으면 새 게임 폴더를 선택할 수 있습니다.",
  );

  try {
    const opened = await window.__TAURI__.core.invoke(
      "load_saved_game_directory",
    );
    if (opened === null) {
      setStatus(
        "idle",
        "게임 폴더를 선택해 주세요",
        "처음 한 번 정상 폴더를 선택하면 다음 실행부터 자동으로 엽니다.",
      );
      return;
    }
    await renderOpenedGameDirectory(opened, true);
    await loadAssetUpdateStatus();
  } catch (error) {
    setStatus(
      "error",
      "저장된 게임 폴더를 열지 못했습니다",
      `${String(error)} 아래 버튼으로 현재 게임 폴더를 다시 선택해 주세요.`,
    );
  } finally {
    setDirectorySelectionBusy(false);
  }
}

selectButton.addEventListener("click", async () => {
  setDirectorySelectionBusy(true);
  clearSummary();
  showConnectionView();
  setStatus("loading", "게임 폴더를 확인하는 중입니다", "폴더 선택 창이 열려 있습니다.");

  try {
    const opened = await window.__TAURI__.core.invoke("pick_game_directory");
    if (opened === null) {
      setStatus("idle", "선택을 취소했습니다", "원할 때 게임 폴더를 다시 선택할 수 있습니다.");
      return;
    }
    await renderOpenedGameDirectory(opened, false);
    await loadAssetUpdateStatus();
  } catch (error) {
    setStatus("error", "게임 폴더를 확인하지 못했습니다", String(error));
  } finally {
    setDirectorySelectionBusy(false);
  }
});

changeDirectoryButton.addEventListener("click", async () => {
  setDirectorySelectionBusy(true);
  settingsMessage.textContent = "새 게임 폴더를 확인하고 있습니다.";

  try {
    const opened = await window.__TAURI__.core.invoke("pick_game_directory");
    if (opened === null) {
      settingsMessage.textContent = "게임 폴더 변경을 취소했습니다. 현재 연결을 유지합니다.";
      return;
    }
    clearSummary();
    await renderOpenedGameDirectory(opened, false);
    await loadAssetUpdateStatus();
  } catch (error) {
    settingsMessage.textContent = `게임 폴더를 변경하지 못했습니다: ${String(error)}`;
  } finally {
    setDirectorySelectionBusy(false);
  }
});

for (const button of navigationButtons) {
  button.addEventListener("click", () => {
    if (!categoryExportBusy) {
      showWorkspacePage(button.dataset.page);
      if (button.dataset.page === "text") {
        loadTextWorkspace();
      } else if (button.dataset.page === "audio") {
        loadAudioWorkspace();
      }
    }
  });
}

audioSearchForm.addEventListener("submit", (event) => {
  event.preventDefault();
  loadAudioPage(0);
});

audioFirstPageButton.addEventListener("click", () => loadAudioPage(0));

audioPreviousPageButton.addEventListener("click", () => {
  if (currentAudioPage !== null) {
    loadAudioPage(Math.max(0, currentAudioPage.offset - currentAudioPage.pageSize));
  }
});

audioNextPageButton.addEventListener("click", () => {
  if (currentAudioPage !== null) {
    loadAudioPage(currentAudioPage.offset + currentAudioPage.pageSize);
  }
});

audioLastPageButton.addEventListener("click", () => {
  if (currentAudioPage !== null && currentAudioPage.totalCount > 0) {
    const lastOffset =
      (Math.ceil(currentAudioPage.totalCount / currentAudioPage.pageSize) - 1) *
      currentAudioPage.pageSize;
    loadAudioPage(lastOffset);
  }
});

audioPlayer.addEventListener("ended", () => {
  if (currentAudioTrack?.loopStartSample > 0) {
    audioPlayer.currentTime = currentAudioTrack.loopStartSeconds;
    audioPlayer.play().catch(() => {});
  }
});

saveAudioTrackButton.addEventListener("click", async () => {
  if (currentAudioTrack === null) {
    return;
  }
  saveAudioTrackButton.disabled = true;
  try {
    const saved = await window.__TAURI__.core.invoke("save_audio_track_ogg", {
      id: currentAudioTrack.id,
    });
    if (saved !== null) {
      setWindowStatus("ready", `${saved.fileName} · ${formatBytes(saved.payloadBytes)} 저장 완료`);
    }
  } catch (error) {
    setWindowStatus("error", `오디오를 저장하지 못했습니다: ${String(error)}`);
  } finally {
    saveAudioTrackButton.disabled = currentAudioTrack === null;
  }
});

textLanguageBlock.addEventListener("change", async () => {
  const selected = Number.parseInt(textLanguageBlock.value, 10);
  if (!Number.isInteger(selected) || selected < 0) {
    textLanguageBlock.value = String(currentTextLanguageBlock);
    return;
  }
  const previous = currentTextLanguageBlock;
  currentTextLanguageBlock = selected;
  textLanguageBlock.disabled = true;
  closeDetail();
  try {
    const summary = await window.__TAURI__.core.invoke(
      "load_text_catalog_summary",
      { languageBlock: currentTextLanguageBlock },
    );
    renderLanguageSelector(summary);
    textCatalogLoaded = false;
    textChangeMode = false;
    currentTextPage = null;
    textSearchQuery.value = "";
    textBrowserTitle.textContent = "전체 텍스트";
    setTextSourceSelection(null);
    if (currentWorkspacePage === "text") {
      renderTextCatalog(summary);
      await Promise.all([loadTextPage(0), loadTextUpdateStatus()]);
    } else if (currentPage !== null) {
      loadPage(currentPage, currentPage.offset);
    }
  } catch (error) {
    currentTextLanguageBlock = previous;
    textLanguageBlock.value = String(previous);
    textLanguageBlock.disabled = false;
    setWindowStatus("error", `언어 블록을 바꾸지 못했습니다: ${String(error)}`);
  }
});

textSourceAllButton.addEventListener("click", () => {
  textChangeMode = false;
  textSearchQuery.value = "";
  setTextSourceSelection(null);
  textBrowserTitle.textContent = "전체 텍스트";
  loadTextPage(0);
});

textSearchForm.addEventListener("submit", (event) => {
  event.preventDefault();
  textChangeMode = false;
  textBrowserTitle.textContent = textSearchQuery.value.trim()
    ? `“${textSearchQuery.value.trim()}” 검색`
    : currentTextSource === null
      ? "전체 텍스트"
      : textSourceList.querySelector(
          `.text-source-button[data-source="${CSS.escape(currentTextSource)}"] strong`,
        )?.textContent ?? "텍스트 자료";
  loadTextPage(0);
});

textPreviousPageButton.addEventListener("click", () => {
  if (currentTextPage !== null) {
    loadTextPage(Math.max(0, currentTextPage.offset - currentTextPage.pageSize));
  }
});

textFirstPageButton.addEventListener("click", () => navigateToTextPage(1));

textNextPageButton.addEventListener("click", () => {
  if (currentTextPage !== null) {
    loadTextPage(currentTextPage.offset + currentTextPage.pageSize);
  }
});

textLastPageButton.addEventListener("click", () => {
  if (currentTextPage !== null) {
    navigateToTextPage(
      Math.ceil(currentTextPage.totalCount / currentTextPage.pageSize),
    );
  }
});

textPageJumpForm.addEventListener("submit", (event) => {
  event.preventDefault();
  if (textPageInput.value.trim() === "") {
    textPageInput.focus();
    return;
  }
  navigateToTextPage(textPageInput.value);
});

viewTextChangesButton.addEventListener("click", () => {
  textChangeMode = true;
  setTextSourceSelection(null);
  textSearchQuery.value = "";
  textBrowserTitle.textContent = "신규·변경 문구";
  loadTextPage(0);
  document.querySelector(".text-browser-card").scrollIntoView({ block: "start" });
});

createTextBaselineButton.addEventListener("click", async () => {
  createTextBaselineButton.disabled = true;
  textUpdateStatus.textContent = "기준점 저장 중";
  try {
    const status = await window.__TAURI__.core.invoke(
      "create_text_update_baseline",
    );
    renderTextUpdateStatus(status);
  } catch (error) {
    textUpdateStatus.textContent = "저장 실패";
    textUpdateMessage.textContent = String(error);
    createTextBaselineButton.disabled = false;
  }
});

refreshTextBaselineButton.addEventListener("click", async () => {
  refreshTextBaselineButton.disabled = true;
  textUpdateStatus.textContent = "기준점 갱신 중";
  try {
    const status = await window.__TAURI__.core.invoke(
      "refresh_text_update_baseline",
    );
    textChangeMode = false;
    renderTextUpdateStatus(status);
  } catch (error) {
    textUpdateStatus.textContent = "갱신 실패";
    textUpdateMessage.textContent = String(error);
    refreshTextBaselineButton.disabled = false;
  }
});

for (const button of firstPageButtons) {
  button.addEventListener("click", () => navigateToPage(1));
}

for (const button of previousPageButtons) {
  button.addEventListener("click", () => {
    if (currentPage !== null) {
      navigateToPage(currentPageNumber(currentPage) - 1);
    }
  });
}

for (const button of nextPageButtons) {
  button.addEventListener("click", () => {
    if (currentPage !== null) {
      navigateToPage(currentPageNumber(currentPage) + 1);
    }
  });
}

for (const button of lastPageButtons) {
  button.addEventListener("click", () => {
    if (currentPage !== null) {
      navigateToPage(totalPages(currentPage));
    }
  });
}

for (const input of pageNumberInputs) {
  input.addEventListener("input", () => {
    for (const otherInput of pageNumberInputs) {
      if (otherInput !== input) {
        otherInput.value = input.value;
      }
    }
  });
  input.addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
      event.preventDefault();
      input.closest("form").requestSubmit();
    }
  });
}

for (const form of pageJumpForms) {
  form.addEventListener("submit", (event) => {
    event.preventDefault();
    const input = form.querySelector(".page-number-input");
    if (input.value.trim() === "") {
      if (currentPage !== null) {
        renderPageNavigation(currentPage);
      }
      input.focus();
      return;
    }
    navigateToPage(Number(input.value));
  });
}

assetSearchForm.addEventListener("submit", (event) => {
  event.preventDefault();
  const query = assetSearchQuery.value.trim();
  if (query.length === 0 || categoryExportBusy) {
    assetSearchQuery.focus();
    return;
  }
  loadSearchPage(query, rememberedPageOffset({ mode: "search", query }));
});

closeDetailButton.addEventListener("click", closeDetail);
minimizeWindowButton.addEventListener("click", () =>
  currentWindow().minimize(),
);
closeWindowButton.addEventListener("click", async () => {
  try {
    await persistWindowState();
  } finally {
    await currentWindow().close();
  }
});
statusAppUpdateButton.addEventListener("click", focusAvailableAppUpdate);
downloadDetailButton.addEventListener("click", saveCurrentDetail);
toggleSelectionButton.addEventListener("click", () => {
  setSelectionMode(!selectionMode);
});
selectCurrentPageButton.addEventListener("click", selectCurrentPage);
clearCurrentPageButton.addEventListener("click", clearCurrentPageSelection);
clearSelectionButton.addEventListener("click", clearSelections);
saveSelectionButton.addEventListener("click", startSelectedAssetExport);
saveAllButton.addEventListener("click", startAssetExport);
createUpdateBaselineButton.addEventListener(
  "click",
  createAssetUpdateBaseline,
);
viewUpdateAssetsButton.addEventListener("click", () => {
  showWorkspacePage("library");
  loadUpdatePage(rememberedPageOffset({ mode: "update" }));
});
refreshUpdateBaselineButton.addEventListener(
  "click",
  refreshAssetUpdateBaseline,
);
cancelCategoryExportButton.addEventListener("click", cancelCategoryExport);
checkAppUpdateButton.addEventListener("click", () => checkForAppUpdate());
installAppUpdateButton.addEventListener("click", installAvailableAppUpdate);
imageExportMode.addEventListener("change", () => {
  try {
    window.localStorage.setItem(
      IMAGE_EXPORT_MODE_STORAGE_KEY,
      currentImageExportMode(),
    );
  } catch {
    // The selected mode still applies for the current run when storage is unavailable.
  }
});
appUpdateBannerInstallButton.addEventListener(
  "click",
  installAvailableAppUpdate,
);
detailDialog.addEventListener("close", () => {
  detailRequestId += 1;
  resetDetail();
});

loadImageExportMode();
initializeWindowState();
loadAppVersion();
checkForAppUpdate({ automatic: true });
loadSavedGameDirectory();
