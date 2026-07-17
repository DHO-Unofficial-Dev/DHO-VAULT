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

function setStatus(kind, title, message) {
  statusCard.dataset.status = kind;
  statusTitle.textContent = title;
  statusMessage.textContent = message;
}

function clearSummary() {
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
      const label = document.createElement("strong");
      const assetCount = document.createElement("span");
      label.textContent = entry.label;
      assetCount.textContent = `${formatNumber(entry.assetCount)}개`;
      item.append(label, assetCount);
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
