// SPDX-License-Identifier: MPL-2.0

const selectButton = document.querySelector("#select-game-directory");
const changeDirectoryButton = document.querySelector("#change-game-directory");
const connectionView = document.querySelector("#connection-view");
const workspaceView = document.querySelector("#workspace-view");
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
const appCurrentVersion = document.querySelector("#app-current-version");
const appUpdateMessage = document.querySelector("#app-update-message");
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
const updatePanel = document.querySelector("#update-panel");
const updateStatus = document.querySelector("#update-status");
const updateMessage = document.querySelector("#update-message");
const updateCounts = document.querySelector("#update-counts");
const updateAddedCount = document.querySelector("#update-added-count");
const updateChangedCount = document.querySelector("#update-changed-count");
const updateRemovedCount = document.querySelector("#update-removed-count");
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
const detailMetadata = document.querySelector("#detail-metadata");
const detailSourceSize = document.querySelector("#detail-source-size");
const detailPreviewSize = document.querySelector("#detail-preview-size");
const downloadDetailButton = document.querySelector("#download-detail");
const closeDetailButton = document.querySelector("#close-detail");

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

const CATEGORY_EXPORT_POLL_INTERVAL = 250;
const VISIBLE_PAGE_BUTTON_COUNT = 9;
const VISIBLE_PAGE_RADIUS = 4;

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
}

function setDirectorySelectionBusy(busy) {
  selectButton.disabled = busy;
  changeDirectoryButton.disabled = busy;
}

function showConnectionView() {
  document.body.dataset.view = "connection";
  connectionView.hidden = false;
  workspaceView.hidden = true;
}

function showWorkspacePage(pageName) {
  currentWorkspacePage = pageName;
  document.body.dataset.view = "workspace";
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
  settingsMessage.textContent = "";
  updateRequestId += 1;
  currentAssetUpdateStatus = null;
  updatePanel.hidden = true;
  updateStatus.textContent = "";
  updateMessage.textContent = "";
  updateCounts.hidden = true;
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
}

function renderAssetUpdateStatus(status) {
  currentAssetUpdateStatus = status;
  updatePanel.hidden = false;
  updateAddedCount.textContent = formatNumber(status.addedCount);
  updateChangedCount.textContent = formatNumber(status.changedCount);
  updateRemovedCount.textContent = formatNumber(status.removedCount);
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
      "업데이트 변경을 감지했습니다. 신규 항목을 검토하기 전에는 저장된 기준점을 바꾸지 않습니다.";
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
      : "현재 상태를 새 기준점으로 저장하면 지금 표시된 신규·변경·삭제 내역은 다시 볼 수 없습니다. 검토를 마쳤다면 계속하세요.";
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
    appendCategoryBranch(list, root);
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
    image.loading = "lazy";
    image.decoding = "async";
    if (pathItemMode) {
      const category = document.createElement("strong");
      const identity = document.createElement("span");
      const size = document.createElement("span");
      category.textContent = path.join(" > ");
      identity.textContent = `${item.archive.toUpperCase()}${
        item.iconId === null ? "" : ` · ID ${formatNumber(item.iconId)}`
      } · 블록 ${formatNumber(item.blockIndex)}`;
      size.textContent = `${formatNumber(item.sourceWidth)} × ${formatNumber(item.sourceHeight)}`;
      caption.append(category, identity, size);
    } else {
      caption.textContent = `${formatNumber(item.sourceWidth)} × ${formatNumber(item.sourceHeight)}`;
    }
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
  hideCategoryExport();
  categoryExport.hidden = false;
  categoryExportStatus.textContent = "저장할 폴더를 선택해 주세요";
  cancelCategoryExportButton.disabled = true;
  setCategoryExportBusy(true);

  try {
    const searchMode = requestedPage.mode === "search";
    const started = await window.__TAURI__.core.invoke(
      searchMode
        ? "start_verified_search_export"
        : "start_verified_category_export",
      searchMode
        ? { query: requestedPage.query }
        : { path: requestedPage.path },
    );
    if (currentPage !== requestedPage) {
      return;
    }
    if (started === null) {
      hideCategoryExport();
      setCategoryExportBusy(false);
      return;
    }
    currentCategoryExport = started;
    categoryExportProgress.max = Math.max(1, started.totalCount);
    categoryExportProgress.value = 0;
    categoryExportStatus.textContent = `0 / ${formatNumber(started.totalCount)}개 저장 중`;
    cancelCategoryExportButton.disabled = false;
    scheduleCategoryExportPoll();
  } catch (error) {
    currentCategoryExport = null;
    categoryExportStatus.textContent = `전체 저장을 시작하지 못했습니다: ${String(error)}`;
    cancelCategoryExportButton.disabled = true;
    setCategoryExportBusy(false);
  }
}

async function startSelectedAssetExport() {
  if (selectedAssets.size === 0 || categoryExportBusy) {
    return;
  }
  hideCategoryExport();
  categoryExport.hidden = false;
  categoryExportStatus.textContent = "저장할 폴더를 선택해 주세요";
  cancelCategoryExportButton.disabled = true;
  setCategoryExportBusy(true);

  try {
    const started = await window.__TAURI__.core.invoke(
      "start_verified_selected_export",
      { assets: [...selectedAssets.values()] },
    );
    if (started === null) {
      hideCategoryExport();
      setCategoryExportBusy(false);
      return;
    }
    currentCategoryExport = started;
    categoryExportProgress.max = Math.max(1, started.totalCount);
    categoryExportProgress.value = 0;
    categoryExportStatus.textContent = `0 / ${formatNumber(started.totalCount)}개 저장 중`;
    cancelCategoryExportButton.disabled = false;
    scheduleCategoryExportPoll();
  } catch (error) {
    currentCategoryExport = null;
    categoryExportStatus.textContent = `선택 이미지 저장을 시작하지 못했습니다: ${String(error)}`;
    cancelCategoryExportButton.disabled = true;
    setCategoryExportBusy(false);
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
  detailPreview.alt = "";
  detailMessage.textContent = "";
  detailMetadata.hidden = true;
  detailSourceSize.textContent = "";
  detailPreviewSize.textContent = "";
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
  detailTitle.textContent = `${path.at(-1)} 이미지 ${position}`;
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
    detailPreview.src = detail.previewDataUrl;
    detailPreview.alt = `${detail.path.at(-1)} 이미지 ${position} 미리보기`;
    detailPreview.width = detail.previewWidth;
    detailPreview.height = detail.previewHeight;
    detailPreview.hidden = false;
    detailMessage.textContent = detail.assembled
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
      requestedDetail,
    );
    if (currentDetail !== requestedDetail) {
      return;
    }
    if (saved === null) {
      detailMessage.textContent = "저장을 취소했습니다.";
      return;
    }
    detailMessage.textContent = `${saved.fileName} 파일로 저장했습니다.`;
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
      { path, offset },
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
      { query, offset },
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
      { offset },
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
}

function showAvailableAppUpdate(result) {
  availableAppUpdate = result.update;
  appCurrentVersion.textContent = `현재 ${result.currentVersion}`;

  if (availableAppUpdate === null) {
    appUpdateBanner.hidden = true;
    installAppUpdateButton.hidden = true;
    appUpdateMessage.textContent = "현재 최신 버전을 사용하고 있습니다.";
    return;
  }

  appUpdateBannerTitle.textContent = `DHO Vault ${availableAppUpdate.version} 업데이트`;
  appUpdateBannerMessage.textContent = `현재 ${result.currentVersion}에서 새 버전으로 업데이트할 수 있습니다.`;
  appUpdateBanner.hidden = false;
  installAppUpdateButton.hidden = false;
  appUpdateMessage.textContent = availableAppUpdate.notes
    ? `${availableAppUpdate.version} 버전이 있습니다. ${availableAppUpdate.notes}`
    : `${availableAppUpdate.version} 버전을 설치할 수 있습니다.`;
}

async function loadAppVersion() {
  try {
    const version = await window.__TAURI__.core.invoke("get_app_version");
    appCurrentVersion.textContent = `현재 ${version}`;
  } catch {
    appCurrentVersion.textContent = "현재 버전 확인 불가";
  }
}

async function checkForAppUpdate({ automatic = false } = {}) {
  if (appUpdateBusy) {
    return;
  }

  setAppUpdateBusy(true);
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
    appUpdateProgressLabel.textContent = "업데이트 설치 프로그램을 시작했습니다.";
    appUpdateMessage.textContent =
      "앱이 자동으로 종료되지 않았다면 창을 닫고 설치를 마쳐 주세요.";
  } catch (error) {
    appUpdateProgress.hidden = true;
    appUpdateMessage.textContent = `업데이트를 설치하지 못했습니다: ${String(error)}`;
  } finally {
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
}

function renderOpenedGameDirectory(opened, automatic) {
  renderSummary(opened.summary);
  if (opened.warning !== null) {
    settingsMessage.textContent = `게임 리소스는 열었지만 폴더를 기억하지 못했습니다. ${opened.warning} 다음 실행 때 게임 폴더를 다시 선택해야 할 수 있습니다.`;
  } else if (automatic) {
    settingsMessage.textContent = "마지막으로 사용한 게임 폴더를 자동으로 연결했습니다.";
  } else {
    settingsMessage.textContent = "게임 폴더가 연결되어 있습니다.";
  }
  showWorkspacePage("library");
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
    renderOpenedGameDirectory(opened, true);
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
    renderOpenedGameDirectory(opened, false);
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
    renderOpenedGameDirectory(opened, false);
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
    }
  });
}

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
appUpdateBannerInstallButton.addEventListener(
  "click",
  installAvailableAppUpdate,
);
detailDialog.addEventListener("close", () => {
  detailRequestId += 1;
  resetDetail();
});

loadAppVersion();
checkForAppUpdate({ automatic: true });
loadSavedGameDirectory();
