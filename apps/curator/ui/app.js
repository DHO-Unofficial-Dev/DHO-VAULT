// SPDX-License-Identifier: MPL-2.0

const selectButton = document.querySelector("#select-game-directory");
const statusCard = document.querySelector(".status-card");
const statusTitle = document.querySelector("#status-title");
const statusMessage = document.querySelector("#status-message");
const directoryDetails = document.querySelector("#directory-details");
const gameDirectory = document.querySelector("#game-directory");
const resourceDirectory = document.querySelector("#resource-directory");
const archiveList = document.querySelector("#archive-list");
const groupPanel = document.querySelector("#group-panel");
const groupTitle = document.querySelector("#group-title");
const groupStatus = document.querySelector("#group-status");
const groupList = document.querySelector("#group-list");
const rangePanel = document.querySelector("#range-panel");
const rangeTitle = document.querySelector("#range-title");
const rangeStatus = document.querySelector("#range-status");
const rangeDescription = document.querySelector("#range-description");
const rangeList = document.querySelector("#range-list");
const samplePanel = document.querySelector("#sample-panel");
const sampleTitle = document.querySelector("#sample-title");
const sampleStatus = document.querySelector("#sample-status");
const sampleGrid = document.querySelector("#sample-grid");

let sampleUrls = [];

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
  sampleTitle.textContent = "표본 이미지";
  sampleStatus.textContent = "";
  sampleGrid.replaceChildren();
}

function clearRanges() {
  clearSamples();
  rangePanel.hidden = true;
  rangeTitle.textContent = "원시 ID 구간 후보";
  rangeStatus.textContent = "";
  rangeList.replaceChildren();
}

function clearGroups() {
  clearRanges();
  groupPanel.hidden = true;
  groupTitle.textContent = "원시 그룹";
  groupStatus.textContent = "";
  groupList.replaceChildren();
}

function clearSummary() {
  clearGroups();
  directoryDetails.hidden = true;
  gameDirectory.textContent = "";
  resourceDirectory.textContent = "";
  archiveList.replaceChildren();
}

function formatNumber(value) {
  return new Intl.NumberFormat("ko-KR").format(value);
}

function createActionButton(label, action) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "secondary-button";
  button.textContent = label;
  button.addEventListener("click", action);
  return button;
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
      createActionButton("그룹 보기", () => openArchive(archive.prefix)),
    );
    archiveList.append(item);
  }

  setStatus(
    "success",
    "검수할 게임 리소스를 확인했습니다",
    `지원하는 MWC 계열 ${summary.archives.length}개를 찾았습니다.`,
  );
}

function renderGroups(prefix, groups) {
  groupList.replaceChildren();
  for (const group of groups) {
    const item = document.createElement("li");
    const title = document.createElement("strong");
    const detail = document.createElement("span");
    title.textContent = `그룹 ${group.groupCode}`;
    detail.textContent = `레코드 ${formatNumber(group.recordCount)} · 고유 이미지 ${formatNumber(group.uniqueBlockCount)} · ID ${formatNumber(group.minIconId)}–${formatNumber(group.maxIconId)}`;
    item.append(
      title,
      detail,
      createActionButton("구간 보기", () => openGroupRanges(prefix, group.groupCode)),
    );
    groupList.append(item);
  }
}

function renderRanges(result) {
  rangeList.replaceChildren();
  for (const range of result.ranges) {
    const item = document.createElement("li");
    const title = document.createElement("strong");
    const detail = document.createElement("span");
    title.textContent = `ID ${formatNumber(range.startIconId)}–${formatNumber(range.endIconId)}`;
    detail.textContent = `레코드 ${formatNumber(range.recordCount)} · 고유 이미지 ${formatNumber(range.uniqueBlockCount)}`;
    item.append(
      title,
      detail,
      createActionButton("표본 보기", () =>
        openRange(
          result.prefix,
          result.groupCode,
          range.startIconId,
          range.endIconId,
        ),
      ),
    );
    rangeList.append(item);
  }
}

async function openArchive(prefix) {
  clearGroups();
  groupPanel.hidden = false;
  groupTitle.textContent = `${prefix.toUpperCase()} 원시 그룹`;
  groupStatus.textContent = "그룹 정보를 읽는 중…";
  setBusy(true);

  try {
    const groups = await window.__TAURI__.core.invoke("list_archive_groups", { prefix });
    renderGroups(prefix, groups);
    groupStatus.textContent = `${formatNumber(groups.length)}개 그룹`;
    groupPanel.scrollIntoView({ behavior: "smooth", block: "start" });
  } catch (error) {
    groupStatus.textContent = "불러오지 못함";
    groupList.replaceChildren();
    const message = document.createElement("p");
    message.className = "error-message";
    message.textContent = String(error);
    groupList.append(message);
  } finally {
    setBusy(false);
  }
}

function renderSamples(result) {
  sampleGrid.replaceChildren();
  for (const sample of result.samples) {
    const card = document.createElement("figure");
    const imageStage = document.createElement("div");
    const image = document.createElement("img");
    const caption = document.createElement("figcaption");
    const url = URL.createObjectURL(
      new Blob([new Uint8Array(sample.png)], { type: "image/png" }),
    );
    sampleUrls.push(url);
    image.src = url;
    image.alt = `아이콘 ID ${sample.iconId}`;
    image.loading = "lazy";
    caption.textContent = `ID ${formatNumber(sample.iconId)} · 블록 ${formatNumber(sample.blockIndex)} · ${sample.width}×${sample.height}`;
    imageStage.append(image);
    card.append(imageStage, caption);
    sampleGrid.append(card);
  }
}

async function openGroupRanges(prefix, groupCode) {
  clearRanges();
  rangePanel.hidden = false;
  rangeTitle.textContent = `${prefix.toUpperCase()} · 그룹 ${groupCode} · 원시 ID 구간`;
  rangeStatus.textContent = "ID 간격을 확인하는 중…";
  setBusy(true);

  try {
    const result = await window.__TAURI__.core.invoke("list_group_id_ranges", {
      prefix,
      groupCode,
    });
    renderRanges(result);
    rangeDescription.textContent = `인접한 ID 차이가 ${formatNumber(result.gapThreshold)}을 넘는 지점에서 기계적으로 나눴습니다. 카테고리 경계로 확정된 값이 아닙니다.`;
    rangeStatus.textContent = `${formatNumber(result.ranges.length)}개 구간 후보`;
    rangePanel.scrollIntoView({ behavior: "smooth", block: "start" });
  } catch (error) {
    rangeStatus.textContent = "불러오지 못함";
    const message = document.createElement("p");
    message.className = "error-message";
    message.textContent = String(error);
    rangeList.append(message);
  } finally {
    setBusy(false);
  }
}

async function openRange(prefix, groupCode, startIconId, endIconId) {
  clearSamples();
  samplePanel.hidden = false;
  sampleTitle.textContent = `${prefix.toUpperCase()} · 그룹 ${groupCode} · ID ${formatNumber(startIconId)}–${formatNumber(endIconId)}`;
  sampleStatus.textContent = "표본 이미지를 만드는 중…";
  setBusy(true);

  try {
    const result = await window.__TAURI__.core.invoke("sample_archive_range", {
      prefix,
      groupCode,
      startIconId,
      endIconId,
    });
    renderSamples(result);
    sampleStatus.textContent = `고유 이미지 ${formatNumber(result.uniqueBlockCount)}개 중 ${formatNumber(result.samples.length)}개 · 전체 레코드 ${formatNumber(result.recordCount)}개`;
    samplePanel.scrollIntoView({ behavior: "smooth", block: "start" });
  } catch (error) {
    sampleStatus.textContent = "추출하지 못함";
    const message = document.createElement("p");
    message.className = "error-message";
    message.textContent = String(error);
    sampleGrid.append(message);
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
