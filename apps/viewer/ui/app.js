// SPDX-License-Identifier: MPL-2.0

const selectButton = document.querySelector("#select-game-directory");
const statusCard = document.querySelector(".status-card");
const statusTitle = document.querySelector("#status-title");
const statusMessage = document.querySelector("#status-message");
const directoryDetails = document.querySelector("#directory-details");
const gameDirectory = document.querySelector("#game-directory");
const resourceDirectory = document.querySelector("#resource-directory");
const archiveList = document.querySelector("#archive-list");
const categoryPanel = document.querySelector("#category-panel");
const categoryStatus = document.querySelector("#category-status");
const categoryGroups = document.querySelector("#category-groups");
const galleryPanel = document.querySelector("#gallery-panel");
const galleryTitle = document.querySelector("#gallery-title");
const galleryStatus = document.querySelector("#gallery-status");
const galleryGrid = document.querySelector("#gallery-grid");
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
const closeDetailButton = document.querySelector("#close-detail");

let currentPage = null;
let galleryRequestId = 0;
let detailRequestId = 0;

function setStatus(kind, title, message) {
  statusCard.dataset.status = kind;
  statusTitle.textContent = title;
  statusMessage.textContent = message;
}

function clearSummary() {
  closeDetail();
  galleryRequestId += 1;
  currentPage = null;
  galleryPanel.hidden = true;
  galleryTitle.textContent = "카테고리를 선택해 주세요";
  galleryStatus.textContent = "";
  galleryGrid.replaceChildren();
  pagePosition.textContent = "";
  categoryPanel.hidden = true;
  categoryStatus.textContent = "";
  categoryGroups.replaceChildren();
  directoryDetails.hidden = true;
  gameDirectory.textContent = "";
  resourceDirectory.textContent = "";
  archiveList.replaceChildren();
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
  galleryTitle.textContent = page.path.join(" > ");

  for (const [index, item] of page.items.entries()) {
    const button = document.createElement("button");
    const frame = document.createElement("div");
    const image = document.createElement("img");
    const caption = document.createElement("span");
    const position = page.offset + index + 1;
    button.type = "button";
    button.className = "gallery-item";
    button.setAttribute(
      "aria-label",
      `${page.path.at(-1)} 이미지 ${position} 상세 보기`,
    );
    frame.className = "thumbnail-frame";
    caption.className = "gallery-caption";
    image.src = item.thumbnailDataUrl;
    image.alt = `${page.path.at(-1)} 이미지 ${position}`;
    image.width = item.thumbnailWidth;
    image.height = item.thumbnailHeight;
    image.loading = "lazy";
    image.decoding = "async";
    caption.textContent = `${formatNumber(item.sourceWidth)} × ${formatNumber(item.sourceHeight)}`;
    frame.append(image);
    button.append(frame, caption);
    button.addEventListener("click", () => {
      loadAssetDetail(page.path, item, position);
    });
    galleryGrid.append(button);
  }

  const first = page.offset + 1;
  const last = page.offset + page.items.length;
  const currentNumber = Math.floor(page.offset / page.pageSize) + 1;
  const pageCount = Math.ceil(page.totalCount / page.pageSize);
  galleryStatus.textContent = `${formatNumber(first)}–${formatNumber(last)} / ${formatNumber(page.totalCount)}개`;
  pagePosition.textContent = `${formatNumber(currentNumber)} / ${formatNumber(pageCount)} 페이지`;
  previousPage.disabled = page.offset === 0;
  nextPage.disabled = last >= page.totalCount;
}

function resetDetail() {
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
  } catch (error) {
    if (requestId === detailRequestId) {
      detailContent.dataset.status = "error";
      detailMessage.textContent = String(error);
    }
  }
}

async function loadCategoryPage(path, offset) {
  const requestId = ++galleryRequestId;
  for (const button of categoryGroups.querySelectorAll(".category-button")) {
    button.disabled = true;
  }
  setSelectedCategory(path);
  galleryPanel.hidden = false;
  galleryTitle.textContent = path.join(" > ");
  galleryStatus.textContent = "썸네일을 불러오는 중입니다";
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
      renderGallery(page);
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
        button.disabled = false;
      }
    }
  }
}

function formatNumber(value) {
  return new Intl.NumberFormat("ko-KR").format(value);
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

selectButton.addEventListener("click", async () => {
  selectButton.disabled = true;
  clearSummary();
  setStatus("loading", "게임 폴더를 확인하는 중입니다", "폴더 선택 창이 열려 있습니다.");

  try {
    const summary = await window.__TAURI__.core.invoke("pick_game_directory");
    if (summary === null) {
      setStatus("idle", "선택을 취소했습니다", "원할 때 게임 폴더를 다시 선택할 수 있습니다.");
      return;
    }
    renderSummary(summary);
  } catch (error) {
    setStatus("error", "게임 폴더를 확인하지 못했습니다", String(error));
  } finally {
    selectButton.disabled = false;
  }
});

previousPage.addEventListener("click", () => {
  if (currentPage !== null && currentPage.offset > 0) {
    loadCategoryPage(
      currentPage.path,
      Math.max(0, currentPage.offset - currentPage.pageSize),
    );
  }
});

nextPage.addEventListener("click", () => {
  if (
    currentPage !== null &&
    currentPage.offset + currentPage.items.length < currentPage.totalCount
  ) {
    loadCategoryPage(
      currentPage.path,
      currentPage.offset + currentPage.pageSize,
    );
  }
});

closeDetailButton.addEventListener("click", closeDetail);
detailDialog.addEventListener("close", () => {
  detailRequestId += 1;
  resetDetail();
});
