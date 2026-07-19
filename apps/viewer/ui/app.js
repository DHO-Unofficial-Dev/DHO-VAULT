// SPDX-License-Identifier: MPL-2.0

const selectButton = document.querySelector("#select-game-directory");
const statusCard = document.querySelector(".status-card");
const statusTitle = document.querySelector("#status-title");
const statusMessage = document.querySelector("#status-message");
const directoryDetails = document.querySelector("#directory-details");
const gameDirectory = document.querySelector("#game-directory");
const resourceDirectory = document.querySelector("#resource-directory");
const archiveList = document.querySelector("#archive-list");
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
const savePageButton = document.querySelector("#save-page");
const saveAllButton = document.querySelector("#save-all");
const categoryExport = document.querySelector("#category-export");
const categoryExportStatus = document.querySelector("#category-export-status");
const categoryExportProgress = document.querySelector(
  "#category-export-progress",
);
const cancelCategoryExportButton = document.querySelector(
  "#cancel-category-export",
);
const previousPage = document.querySelector("#previous-page");
const nextPage = document.querySelector("#next-page");
const pagePosition = document.querySelector("#page-position");
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

const CATEGORY_EXPORT_POLL_INTERVAL = 250;

function setStatus(kind, title, message) {
  statusCard.dataset.status = kind;
  statusTitle.textContent = title;
  statusMessage.textContent = message;
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
  selectButton.disabled = busy;
  createUpdateBaselineButton.disabled = busy;
  viewUpdateAssetsButton.disabled = busy;
  refreshUpdateBaselineButton.disabled = busy;
  assetSearchQuery.disabled = busy;
  assetSearchSubmit.disabled = busy;
  for (const button of categoryGroups.querySelectorAll(".category-button")) {
    button.disabled = busy;
  }
  for (const button of galleryGrid.querySelectorAll(".gallery-item")) {
    button.disabled = busy;
  }
  const hasPageItems = (currentPage?.items.length ?? 0) > 0;
  savePageButton.disabled = busy || !hasPageItems;
  saveAllButton.disabled = busy || !hasPageItems;
  if (currentPage === null) {
    previousPage.disabled = true;
    nextPage.disabled = true;
  } else {
    previousPage.disabled = busy || currentPage.offset === 0;
    nextPage.disabled =
      busy ||
      currentPage.offset + currentPage.items.length >= currentPage.totalCount;
  }
}

function clearSummary() {
  closeDetail();
  hideCategoryExport();
  categoryExportBusy = false;
  galleryRequestId += 1;
  currentPage = null;
  galleryPanel.hidden = true;
  galleryTitle.textContent = "카테고리를 선택해 주세요";
  galleryStatus.textContent = "";
  savePageButton.disabled = true;
  savePageButton.textContent = "현재 페이지 저장";
  saveAllButton.disabled = true;
  saveAllButton.textContent = "카테고리 전체 저장";
  galleryGrid.replaceChildren();
  pagePosition.textContent = "";
  categoryPanel.hidden = true;
  categoryStatus.textContent = "";
  categoryGroups.replaceChildren();
  assetSearchQuery.value = "";
  directoryDetails.hidden = true;
  gameDirectory.textContent = "";
  resourceDirectory.textContent = "";
  archiveList.replaceChildren();
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
  savePageButton.hidden = false;
  saveAllButton.hidden = false;
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
  selectButton.disabled = true;
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
      selectButton.disabled = false;
    }
  }
}

function dismissUpdateGallery() {
  if (currentPage?.mode !== "update") {
    return;
  }
  galleryRequestId += 1;
  closeDetail();
  currentPage = null;
  galleryPanel.hidden = true;
  galleryTitle.textContent = "카테고리를 선택해 주세요";
  galleryStatus.textContent = "";
  galleryGrid.replaceChildren();
  pagePosition.textContent = "";
  previousPage.disabled = true;
  nextPage.disabled = true;
  savePageButton.hidden = false;
  saveAllButton.hidden = false;
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
  selectButton.disabled = true;
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
      selectButton.disabled = false;
    }
  }
}

function renderCategories(categories) {
  categoryGroups.replaceChildren();
  const groups = new Map();
  const sorted = [...categories].sort((left, right) =>
    left.path.join("\u0000").localeCompare(right.path.join("\u0000"), "ko"),
  );

  for (const category of sorted) {
    const [domain, ...leaf] = category.path;
    if (!groups.has(domain)) {
      groups.set(domain, []);
    }
    groups.get(domain).push({
      label: leaf.length === 0 ? "전체" : leaf.join(" > "),
      path: category.path,
      assetCount: category.assetCount,
    });
  }

  for (const [domain, entries] of groups) {
    const section = document.createElement("section");
    const heading = document.createElement("div");
    const title = document.createElement("h3");
    const count = document.createElement("span");
    const list = document.createElement("ul");
    const domainTotal = entries.reduce(
      (total, entry) => total + entry.assetCount,
      0,
    );
    section.className = "category-domain";
    heading.className = "category-domain-heading";
    title.textContent = domain;
    count.textContent = `${formatNumber(domainTotal)}개`;
    list.className = "category-list";
    heading.append(title, count);

    for (const entry of entries) {
      const item = document.createElement("li");
      const button = document.createElement("button");
      const label = document.createElement("strong");
      const assetCount = document.createElement("span");
      button.type = "button";
      button.className = "category-button";
      button.dataset.path = JSON.stringify(entry.path);
      button.setAttribute("aria-pressed", "false");
      label.textContent = entry.label;
      assetCount.textContent = `${formatNumber(entry.assetCount)}개`;
      button.append(label, assetCount);
      button.addEventListener("click", () => {
        loadCategoryPage(entry.path, 0);
      });
      item.append(button);
      list.append(item);
    }

    section.append(heading, list);
    categoryGroups.append(section);
  }

  const totalAssets = categories.reduce(
    (total, category) => total + category.assetCount,
    0,
  );
  categoryStatus.textContent = `${formatNumber(categories.length)}개 카테고리 · ${formatNumber(totalAssets)}개 이미지`;
  categoryPanel.hidden = false;
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
}

function renderGallery(page) {
  currentPage = page;
  galleryGrid.replaceChildren();
  galleryGrid.dataset.status = "ready";
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
    const button = document.createElement("button");
    const frame = document.createElement("div");
    const image = document.createElement("img");
    const caption = document.createElement("span");
    const position = page.offset + index + 1;
    button.type = "button";
    button.className = "gallery-item";
    button.disabled = categoryExportBusy;
    button.setAttribute(
      "aria-label",
      `${path.at(-1)} 이미지 ${position} 상세 보기`,
    );
    frame.className = "thumbnail-frame";
    caption.className = "gallery-caption";
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
      identity.textContent =
        `${item.archive.toUpperCase()} · ID ${formatNumber(item.iconId)}` +
        ` · 블록 ${formatNumber(item.blockIndex)}`;
      size.textContent = `${formatNumber(item.sourceWidth)} × ${formatNumber(item.sourceHeight)}`;
      caption.append(category, identity, size);
    } else {
      caption.textContent = `${formatNumber(item.sourceWidth)} × ${formatNumber(item.sourceHeight)}`;
    }
    frame.append(image);
    button.append(frame, caption);
    button.addEventListener("click", () => {
      loadAssetDetail(path, item, position);
    });
    galleryGrid.append(button);
  }

  const last = page.offset + page.items.length;
  const currentNumber =
    page.totalCount === 0 ? 0 : Math.floor(page.offset / page.pageSize) + 1;
  const pageCount = Math.ceil(page.totalCount / page.pageSize);
  galleryStatus.textContent = galleryPageStatus(page);
  pagePosition.textContent = `${formatNumber(currentNumber)} / ${formatNumber(pageCount)} 페이지`;
  savePageButton.hidden = updateMode;
  saveAllButton.hidden = updateMode;
  savePageButton.disabled =
    updateMode || categoryExportBusy || page.items.length === 0;
  saveAllButton.disabled =
    updateMode || categoryExportBusy || page.items.length === 0;
  saveAllButton.textContent = searchMode
    ? "검색 결과 전체 저장"
    : "카테고리 전체 저장";
  previousPage.disabled = categoryExportBusy || page.offset === 0;
  nextPage.disabled = categoryExportBusy || last >= page.totalCount;
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

async function saveCurrentPage() {
  if (
    currentPage === null ||
    currentPage.mode === "update" ||
    currentPage.items.length === 0
  ) {
    return;
  }
  const requestedPage = currentPage;
  savePageButton.disabled = true;
  saveAllButton.disabled = true;
  savePageButton.textContent = "저장 중…";
  galleryStatus.textContent = "저장할 폴더를 선택해 주세요";

  try {
    const searchMode = requestedPage.mode === "search";
    const saved = await window.__TAURI__.core.invoke(
      searchMode ? "save_verified_search_page" : "save_verified_category_page",
      searchMode
        ? { query: requestedPage.query, offset: requestedPage.offset }
        : { path: requestedPage.path, offset: requestedPage.offset },
    );
    if (currentPage !== requestedPage) {
      return;
    }
    const pageStatus = galleryPageStatus(requestedPage);
    galleryStatus.textContent =
      saved === null
        ? pageStatus
        : `${pageStatus} · ${formatNumber(saved.savedCount)}개 저장 완료`;
  } catch (error) {
    if (currentPage === requestedPage) {
      galleryStatus.textContent = `${galleryPageStatus(requestedPage)} · 저장하지 못했습니다: ${String(error)}`;
    }
  } finally {
    if (currentPage === requestedPage) {
      savePageButton.disabled = categoryExportBusy;
      saveAllButton.disabled =
        categoryExportBusy || requestedPage.items.length === 0;
      savePageButton.textContent = "현재 페이지 저장";
    }
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
  galleryTitle.textContent = path.join(" > ");
  galleryStatus.textContent = "썸네일을 불러오는 중입니다";
  savePageButton.disabled = true;
  savePageButton.textContent = "현재 페이지 저장";
  saveAllButton.disabled = true;
  galleryGrid.dataset.status = "loading";
  galleryGrid.replaceChildren();
  pagePosition.textContent = "불러오는 중";
  previousPage.disabled = true;
  nextPage.disabled = true;

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
      currentPage = null;
      galleryGrid.dataset.status = "error";
      galleryStatus.textContent = "이미지를 불러오지 못했습니다";
      pagePosition.textContent = String(error);
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
  galleryTitle.textContent = `검색: ${query}`;
  galleryStatus.textContent = "검색 결과를 불러오는 중입니다";
  savePageButton.disabled = true;
  savePageButton.textContent = "현재 페이지 저장";
  saveAllButton.disabled = true;
  galleryGrid.dataset.status = "loading";
  galleryGrid.replaceChildren();
  pagePosition.textContent = "불러오는 중";
  previousPage.disabled = true;
  nextPage.disabled = true;

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
      currentPage = null;
      galleryGrid.dataset.status = "error";
      galleryStatus.textContent = "검색 결과를 불러오지 못했습니다";
      pagePosition.textContent = String(error);
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
  galleryTitle.textContent = "이번 업데이트 신규";
  galleryStatus.textContent = "신규 이미지를 불러오는 중입니다";
  savePageButton.hidden = true;
  saveAllButton.hidden = true;
  savePageButton.disabled = true;
  saveAllButton.disabled = true;
  galleryGrid.dataset.status = "loading";
  galleryGrid.replaceChildren();
  pagePosition.textContent = "불러오는 중";
  previousPage.disabled = true;
  nextPage.disabled = true;

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
      currentPage = null;
      galleryGrid.dataset.status = "error";
      galleryStatus.textContent = "신규 이미지를 불러오지 못했습니다";
      pagePosition.textContent = String(error);
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

function formatNumber(value) {
  return new Intl.NumberFormat("ko-KR").format(value);
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
    detail.textContent = `레코드 ${formatNumber(archive.recordCount)} · 그룹 ${formatNumber(archive.groupCount)} · 이미지 블록 ${formatNumber(archive.imageBlockCount)} · 데이터 파일 ${formatNumber(archive.archiveCount)}`;
    item.append(prefix, detail);
    archiveList.append(item);
  }

  renderCategories(summary.verifiedCategories);

  setStatus(
    "success",
    "게임 리소스를 확인했습니다",
    `지원하는 MWC 인덱스 ${summary.archives.length}개와 확인된 카테고리 ${summary.verifiedCategories.length}개를 찾았습니다.`,
  );
}

function renderOpenedGameDirectory(opened, automatic) {
  renderSummary(opened.summary);
  if (opened.warning !== null) {
    setStatus(
      "warning",
      "게임 리소스는 열었지만 폴더를 기억하지 못했습니다",
      `${opened.warning} 다음 실행 때 게임 폴더를 다시 선택해야 할 수 있습니다.`,
    );
  } else if (automatic) {
    setStatus(
      "success",
      "마지막 게임 폴더를 자동으로 열었습니다",
      `지원하는 MWC 인덱스 ${opened.summary.archives.length}개와 확인된 카테고리 ${opened.summary.verifiedCategories.length}개를 찾았습니다.`,
    );
  }
}

async function loadSavedGameDirectory() {
  selectButton.disabled = true;
  clearSummary();
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
    selectButton.disabled = false;
  }
}

selectButton.addEventListener("click", async () => {
  selectButton.disabled = true;
  clearSummary();
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
    selectButton.disabled = false;
  }
});

previousPage.addEventListener("click", () => {
  if (currentPage !== null && currentPage.offset > 0) {
    loadPage(
      currentPage,
      Math.max(0, currentPage.offset - currentPage.pageSize),
    );
  }
});

nextPage.addEventListener("click", () => {
  if (
    currentPage !== null &&
    currentPage.offset + currentPage.items.length < currentPage.totalCount
  ) {
    loadPage(currentPage, currentPage.offset + currentPage.pageSize);
  }
});

assetSearchForm.addEventListener("submit", (event) => {
  event.preventDefault();
  const query = assetSearchQuery.value.trim();
  if (query.length === 0 || categoryExportBusy) {
    assetSearchQuery.focus();
    return;
  }
  loadSearchPage(query, 0);
});

closeDetailButton.addEventListener("click", closeDetail);
downloadDetailButton.addEventListener("click", saveCurrentDetail);
savePageButton.addEventListener("click", saveCurrentPage);
saveAllButton.addEventListener("click", startAssetExport);
createUpdateBaselineButton.addEventListener(
  "click",
  createAssetUpdateBaseline,
);
viewUpdateAssetsButton.addEventListener("click", () => {
  loadUpdatePage(0);
});
refreshUpdateBaselineButton.addEventListener(
  "click",
  refreshAssetUpdateBaseline,
);
cancelCategoryExportButton.addEventListener("click", cancelCategoryExport);
detailDialog.addEventListener("close", () => {
  detailRequestId += 1;
  resetDetail();
});

loadSavedGameDirectory();
