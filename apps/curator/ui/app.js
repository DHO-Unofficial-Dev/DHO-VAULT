// SPDX-License-Identifier: MPL-2.0

const selectButton = document.querySelector("#select-game-directory");
const statusCard = document.querySelector(".status-card");
const statusTitle = document.querySelector("#status-title");
const statusMessage = document.querySelector("#status-message");
const directoryDetails = document.querySelector("#directory-details");
const gameDirectory = document.querySelector("#game-directory");
const resourceDirectory = document.querySelector("#resource-directory");
const archiveList = document.querySelector("#archive-list");
const bandPanel = document.querySelector("#band-panel");
const bandTitle = document.querySelector("#band-title");
const bandStatus = document.querySelector("#band-status");
const bandList = document.querySelector("#band-list");
const samplePanel = document.querySelector("#sample-panel");
const sampleTitle = document.querySelector("#sample-title");
const sampleStatus = document.querySelector("#sample-status");
const boundaryGrid = document.querySelector("#boundary-grid");
const sampleGrid = document.querySelector("#sample-grid");

let sampleUrls = [];

function formatNumber(value) {
  return new Intl.NumberFormat("ko-KR").format(value);
}

function formatIdRange(start, end) {
  return `${formatNumber(start)}–${formatNumber(end)}`;
}

function setStatus(kind, title, message) {
  statusCard.dataset.status = kind;
  statusTitle.textContent = title;
  statusMessage.textContent = message;
}

function setBusy(busy) {
  for (const button of document.querySelectorAll("button")) {
    button.disabled = busy;
  }
}

function revokeSampleUrls() {
  for (const url of sampleUrls) {
    URL.revokeObjectURL(url);
  }
  sampleUrls = [];
}

function clearSamples() {
  revokeSampleUrls();
  samplePanel.hidden = true;
  sampleTitle.textContent = "대역 표본 이미지";
  sampleStatus.textContent = "";
  boundaryGrid.replaceChildren();
  sampleGrid.replaceChildren();
}

function clearBands() {
  clearSamples();
  bandPanel.hidden = true;
  bandTitle.textContent = "100,000 단위 ID 대역";
  bandStatus.textContent = "";
  bandList.replaceChildren();
}

function clearSummary() {
  clearBands();
  directoryDetails.hidden = true;
  gameDirectory.textContent = "";
  resourceDirectory.textContent = "";
  archiveList.replaceChildren();
}

function createActionButton(label, action) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "secondary-button";
  button.textContent = label;
  button.addEventListener("click", action);
  return button;
}

function createErrorItem(error) {
  const item = document.createElement("li");
  item.className = "error-message";
  item.textContent = String(error);
  return item;
}

function renderSummary(summary) {
  gameDirectory.textContent = summary.gameDirectory;
  resourceDirectory.textContent = summary.resourceDirectory;
  directoryDetails.hidden = false;
  archiveList.replaceChildren();

  for (const archive of summary.archives) {
    const item = document.createElement("li");
    const prefix = document.createElement("strong");
    const content = document.createElement("div");
    const detail = document.createElement("span");
    const state = document.createElement("small");
    prefix.textContent = archive.prefix.toUpperCase();
    detail.textContent = `레코드 ${formatNumber(archive.recordCount)} · 원시 그룹 ${formatNumber(archive.groupCount)} · 이미지 블록 ${formatNumber(archive.imageBlockCount)} · 데이터 파일 ${formatNumber(archive.archiveCount)}`;
    state.textContent = "분류 미확정";
    content.append(detail, state);
    item.append(
      prefix,
      content,
      createActionButton("ID 대역 보기", () => openArchiveBands(archive.prefix)),
    );
    archiveList.append(item);
  }

  setStatus(
    "success",
    "검수할 게임 리소스를 확인했습니다",
    `지원하는 MWC 계열 ${summary.archives.length}개를 찾았습니다.`,
  );
}

function renderBands(result) {
  bandList.replaceChildren();
  for (const band of result.bands) {
    const item = document.createElement("li");
    const title = document.createElement("strong");
    const detail = document.createElement("span");
    const groups = band.groupCodes.map((value) => formatNumber(value)).join(", ");
    title.textContent = `ID ${formatIdRange(band.startIconId, band.endIconId)}`;
    detail.textContent = `실제 ID ${formatIdRange(band.firstActualIconId, band.lastActualIconId)} · 레코드 ${formatNumber(band.recordCount)} · 고유 이미지 ${formatNumber(band.uniqueBlockCount)} · 원시 그룹 ${groups}`;
    item.append(
      title,
      detail,
      createActionButton("표본 보기", () =>
        openBand(result.prefix, band.startIconId, band.endIconId),
      ),
    );
    bandList.append(item);
  }
}

async function openArchiveBands(prefix) {
  clearBands();
  bandPanel.hidden = false;
  bandTitle.textContent = `${prefix.toUpperCase()} · 100,000 단위 ID 대역`;
  bandStatus.textContent = "아카이브 전체 ID를 묶는 중…";
  setBusy(true);

  try {
    const result = await window.__TAURI__.core.invoke("list_archive_id_bands", {
      prefix,
    });
    renderBands(result);
    bandStatus.textContent = `${formatNumber(result.bands.length)}개 대역 · 단위 ${formatNumber(result.bandSize)}`;
    bandPanel.scrollIntoView({ behavior: "smooth", block: "start" });
  } catch (error) {
    bandStatus.textContent = "불러오지 못함";
    bandList.replaceChildren(createErrorItem(error));
  } finally {
    setBusy(false);
  }
}

function createSampleCard(sample, boundaryLabel = null) {
  const card = document.createElement("figure");
  const imageStage = document.createElement("div");
  const image = document.createElement("img");
  const caption = document.createElement("figcaption");
  const url = URL.createObjectURL(
    new Blob([new Uint8Array(sample.png)], { type: "image/png" }),
  );
  sampleUrls.push(url);
  image.src = url;
  image.alt = `${boundaryLabel ? `${boundaryLabel} · ` : ""}아이콘 ID ${sample.iconId}`;
  image.loading = "lazy";
  caption.textContent = `ID ${formatNumber(sample.iconId)} · 그룹 ${formatNumber(sample.groupCode)} · 블록 ${formatNumber(sample.blockIndex)} · ${sample.width}×${sample.height}`;
  if (boundaryLabel !== null) {
    const badge = document.createElement("strong");
    badge.textContent = boundaryLabel;
    imageStage.append(badge);
    card.className = "boundary-card";
  }
  imageStage.append(image);
  card.append(imageStage, caption);
  return card;
}

function renderSamples(result) {
  boundaryGrid.replaceChildren();
  sampleGrid.replaceChildren();

  const sameBoundary =
    result.firstRecord.iconId === result.lastRecord.iconId &&
    result.firstRecord.groupCode === result.lastRecord.groupCode &&
    result.firstRecord.blockIndex === result.lastRecord.blockIndex;
  boundaryGrid.append(
    createSampleCard(result.firstRecord, sameBoundary ? "처음 · 끝" : "처음"),
  );
  if (!sameBoundary) {
    boundaryGrid.append(createSampleCard(result.lastRecord, "끝"));
  }

  for (const sample of result.samples) {
    sampleGrid.append(createSampleCard(sample));
  }
}

async function openBand(prefix, startIconId, endIconId) {
  clearSamples();
  samplePanel.hidden = false;
  sampleTitle.textContent = `${prefix.toUpperCase()} · ID ${formatIdRange(startIconId, endIconId)}`;
  sampleStatus.textContent = "대역 표본 이미지를 만드는 중…";
  setBusy(true);

  try {
    const result = await window.__TAURI__.core.invoke("sample_archive_band", {
      prefix,
      startIconId,
      endIconId,
    });
    renderSamples(result);
    const groups = result.groupCodes.map((value) => formatNumber(value)).join(", ");
    sampleStatus.textContent = `고유 이미지 ${formatNumber(result.uniqueBlockCount)}개 중 ${formatNumber(result.samples.length)}개 · 레코드 ${formatNumber(result.recordCount)}개 · 원시 그룹 ${groups}`;
    samplePanel.scrollIntoView({ behavior: "smooth", block: "start" });
  } catch (error) {
    sampleStatus.textContent = "추출하지 못함";
    sampleGrid.replaceChildren(createErrorItem(error));
  } finally {
    setBusy(false);
  }
}

selectButton.addEventListener("click", async () => {
  setBusy(true);
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
    setBusy(false);
  }
});

window.addEventListener("beforeunload", revokeSampleUrls);
