import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";

// --- DOM refs ---
let usernameInput: HTMLInputElement;
let searchBtn: HTMLButtonElement;
let cancelBtn: HTMLButtonElement;
let searchForm: HTMLFormElement;
let depDot: HTMLElement;
let depText: HTMLElement;
let optionsToggle: HTMLButtonElement;
let optionsChevron: HTMLElement;
let optionsPanel: HTMLElement;
let progressSection: HTMLElement;
let progressText: HTMLElement;
let progressCounter: HTMLElement;
let progressFill: HTMLElement;
let resultsToolbar: HTMLElement;
let resultsGrid: HTMLElement;
let resultCount: HTMLElement;
let notFoundCount: HTMLElement;
let filterInput: HTMLInputElement;
let copyBtn: HTMLButtonElement;
let clearBtn: HTMLButtonElement;
let emptyState: HTMLElement;

// Options
let optTimeout: HTMLInputElement;
let optProxy: HTMLInputElement;
let optSites: HTMLInputElement;
let optNsfw: HTMLInputElement;
let optPrintAll: HTMLInputElement;
let optBrowse: HTMLInputElement;
let optCsv: HTMLInputElement;
let optXlsx: HTMLInputElement;
let optDebug: HTMLInputElement;
let debugConsole: HTMLElement;
let debugLog: HTMLElement;
let debugClearBtn: HTMLButtonElement;

// --- State ---
let isSearching = false;
let foundCount = 0;
let notFoundTotal = 0;
let allResults: ResultEntry[] = [];

interface SherlockResult {
  site: string;
  url: string;
  found: boolean;
}

interface SearchEvent {
  event_type: string;
  message: string;
  result: SherlockResult | null;
}

interface SearchOptions {
  timeout: number;
  proxy: string;
  sites: string[];
  nsfw: boolean;
  print_all: boolean;
  browse: boolean;
  csv: boolean;
  xlsx: boolean;
  debug: boolean;
}

interface ResultEntry {
  site: string;
  url: string;
  found: boolean;
  element: HTMLElement;
}

// --- Helpers ---
function escapeHtml(str: string): string {
  const div = document.createElement("div");
  div.textContent = str;
  return div.innerHTML;
}

function showToast(message: string) {
  const toast = document.createElement("div");
  toast.className = "toast";
  toast.textContent = message;
  document.body.appendChild(toast);
  setTimeout(() => toast.remove(), 3000);
}

function appendDebugLine(message: string, type: "stdout" | "stderr" | "cmd" | "error" = "stdout") {
  if (!optDebug.checked) return;
  debugConsole.classList.remove("hidden");
  const line = document.createElement("div");
  line.className = `debug-line ${type}`;
  line.textContent = message;
  debugLog.appendChild(line);
  debugLog.scrollTop = debugLog.scrollHeight;
}

function getOptions(): SearchOptions {
  const sitesRaw = optSites.value.trim();
  const sites = sitesRaw ? sitesRaw.split(",").map(s => s.trim()).filter(Boolean) : [];

  return {
    timeout: Math.max(1, parseInt(optTimeout.value) || 60),
    proxy: optProxy.value.trim(),
    sites,
    nsfw: optNsfw.checked,
    print_all: optPrintAll.checked,
    browse: optBrowse.checked,
    csv: optCsv.checked,
    xlsx: optXlsx.checked,
    debug: optDebug.checked,
  };
}

// --- UI Updates ---
function setSearching(searching: boolean) {
  isSearching = searching;
  searchBtn.classList.toggle("hidden", searching);
  cancelBtn.classList.toggle("hidden", !searching);
  usernameInput.disabled = searching;
  progressSection.classList.toggle("hidden", !searching);

  if (searching) {
    emptyState.classList.add("hidden");
    progressFill.style.width = "0%";
    progressFill.style.background = "";
    progressFill.classList.add("indeterminate");
  } else {
    progressFill.classList.remove("indeterminate");
    progressFill.style.width = "100%";
  }
}

function addResult(result: SherlockResult) {
  if (result.found) {
    foundCount++;
  } else {
    notFoundTotal++;
  }

  resultCount.textContent = foundCount.toString();
  resultsToolbar.classList.remove("hidden");

  if (notFoundTotal > 0) {
    notFoundCount.textContent = `${notFoundTotal} not found`;
    notFoundCount.classList.remove("hidden");
  }

  const card = document.createElement("div");
  card.className = `result-card ${result.found ? "found" : "not-found"}`;
  card.dataset.site = result.site.toLowerCase();
  card.dataset.found = result.found ? "1" : "0";

  const initials = result.site.substring(0, 2);
  card.innerHTML = `
    <div class="result-avatar">${escapeHtml(initials)}</div>
    <div class="result-info">
      <div class="result-site-name">${escapeHtml(result.site)}</div>
      ${result.url ? `<a class="result-url" data-url="${escapeHtml(result.url)}" title="${escapeHtml(result.url)}">${escapeHtml(result.url)}</a>` : `<span class="result-url">Not found</span>`}
    </div>
    <span class="result-tag ${result.found ? "found" : "not-found"}">${result.found ? "Found" : "N/A"}</span>
  `;

  if (result.found && result.url) {
    const link = card.querySelector(".result-url") as HTMLElement;
    link.addEventListener("click", (e) => {
      e.preventDefault();
      const url = link.getAttribute("data-url");
      if (url && (url.startsWith("https://") || url.startsWith("http://"))) {
        openUrl(url);
      }
    });
  }

  resultsGrid.appendChild(card);

  allResults.push({ site: result.site, url: result.url, found: result.found, element: card });

  // Apply current filter
  const filterVal = filterInput.value.toLowerCase();
  if (filterVal && !result.site.toLowerCase().includes(filterVal)) {
    card.classList.add("hidden");
  }
}

function filterResults() {
  const query = filterInput.value.toLowerCase();
  for (const r of allResults) {
    const match = r.site.toLowerCase().includes(query) || r.url.toLowerCase().includes(query);
    r.element.classList.toggle("hidden", !match);
  }
}

function clearResults() {
  foundCount = 0;
  notFoundTotal = 0;
  allResults = [];
  resultsGrid.innerHTML = "";
  resultCount.textContent = "0";
  notFoundCount.classList.add("hidden");
  resultsToolbar.classList.add("hidden");
  progressSection.classList.add("hidden");
  emptyState.classList.remove("hidden");
  filterInput.value = "";
}

function copyAllUrls() {
  const urls = allResults.filter(r => r.found).map(r => r.url).join("\n");
  if (!urls) {
    showToast("No URLs to copy");
    return;
  }
  navigator.clipboard.writeText(urls).then(() => {
    showToast(`${allResults.filter(r => r.found).length} URLs copied to clipboard`);
  });
}

// --- Core ---
async function checkDependencies() {
  try {
    const deps = await invoke<{ python: boolean; sherlock: boolean; python_path: string }>("check_dependencies");

    if (!deps.python || !deps.sherlock) {
      depDot.className = "dot dot-error";
      depText.textContent = "Sherlock unavailable — please reinstall";
      searchBtn.disabled = true;
      return;
    }

    depDot.className = "dot dot-ok";
    depText.textContent = "Ready";
  } catch (e) {
    depDot.className = "dot dot-error";
    depText.textContent = `Error`;
  }
}

async function startSearch() {
  const raw = usernameInput.value.trim();
  if (!raw) return;

  const usernames = raw.split(/\s+/).filter(Boolean);
  const options = getOptions();

  // Reset
  foundCount = 0;
  notFoundTotal = 0;
  allResults = [];
  resultsGrid.innerHTML = "";
  resultCount.textContent = "0";
  notFoundCount.classList.add("hidden");
  filterInput.value = "";
  resultsToolbar.classList.remove("hidden");
  emptyState.classList.add("hidden");

  setSearching(true);
  progressText.textContent = `Searching for ${usernames.join(", ")}...`;
  progressCounter.textContent = "";
  debugLog.innerHTML = "";

  try {
    await invoke("search_username", { usernames, options });
  } catch (e) {
    progressText.textContent = `Error: ${e}`;
    progressFill.style.background = "var(--error)";
  }

  setSearching(false);
}

async function cancelSearch() {
  try {
    await invoke("cancel_search");
  } catch (_) {
    // ignore
  }
  setSearching(false);
  progressText.textContent = "Search cancelled";
}

// --- Init ---
window.addEventListener("DOMContentLoaded", async () => {
  // Bind DOM
  usernameInput = document.querySelector("#username-input")!;
  searchBtn = document.querySelector("#search-btn")!;
  cancelBtn = document.querySelector("#cancel-btn")!;
  searchForm = document.querySelector("#search-form")!;
  depDot = document.querySelector("#dep-dot")!;
  depText = document.querySelector("#dep-text")!;
  optionsToggle = document.querySelector("#options-toggle")!;
  optionsChevron = document.querySelector("#options-chevron")!;
  optionsPanel = document.querySelector("#options-panel")!;
  progressSection = document.querySelector("#progress-section")!;
  progressText = document.querySelector("#progress-text")!;
  progressCounter = document.querySelector("#progress-counter")!;
  progressFill = document.querySelector("#progress-fill")!;
  resultsToolbar = document.querySelector("#results-toolbar")!;
  resultsGrid = document.querySelector("#results-grid")!;
  resultCount = document.querySelector("#result-count")!;
  notFoundCount = document.querySelector("#not-found-count")!;
  filterInput = document.querySelector("#filter-input")!;
  copyBtn = document.querySelector("#copy-btn")!;
  clearBtn = document.querySelector("#clear-btn")!;
  emptyState = document.querySelector("#empty-state")!;

  optTimeout = document.querySelector("#opt-timeout")!;
  optProxy = document.querySelector("#opt-proxy")!;
  optSites = document.querySelector("#opt-sites")!;
  optNsfw = document.querySelector("#opt-nsfw")!;
  optPrintAll = document.querySelector("#opt-print-all")!;
  optBrowse = document.querySelector("#opt-browse")!;
  optCsv = document.querySelector("#opt-csv")!;
  optXlsx = document.querySelector("#opt-xlsx")!;
  optDebug = document.querySelector("#opt-debug")!;
  debugConsole = document.querySelector("#debug-console")!;
  debugLog = document.querySelector("#debug-log")!;
  debugClearBtn = document.querySelector("#debug-clear")!;

  // Events
  await listen<SearchEvent>("sherlock-event", (event) => {
    const data = event.payload;
    switch (data.event_type) {
      case "result":
        if (data.result) addResult(data.result);
        break;
      case "info":
        if (isSearching) progressText.textContent = data.message;
        break;
      case "error":
        progressText.textContent = data.message;
        break;
      case "debug":
        if (data.message.startsWith("[DEBUG] Command:")) {
          appendDebugLine(data.message, "cmd");
        } else if (data.message.startsWith("[STDERR]")) {
          appendDebugLine(data.message, "stderr");
        } else {
          appendDebugLine(data.message, "stdout");
        }
        break;
      case "progress":
        progressCounter.textContent = data.message;
        break;
      case "complete":
        progressText.textContent = data.message;
        progressFill.classList.remove("indeterminate");
        progressFill.style.width = "100%";
        break;
    }
  });

  searchForm.addEventListener("submit", (e) => {
    e.preventDefault();
    if (!isSearching) startSearch();
  });

  cancelBtn.addEventListener("click", cancelSearch);

  optionsToggle.addEventListener("click", () => {
    optionsPanel.classList.toggle("hidden");
    optionsChevron.classList.toggle("open");
  });

  let filterTimer: ReturnType<typeof setTimeout>;
  filterInput.addEventListener("input", () => {
    clearTimeout(filterTimer);
    filterTimer = setTimeout(filterResults, 150);
  });
  copyBtn.addEventListener("click", copyAllUrls);
  clearBtn.addEventListener("click", clearResults);

  optDebug.addEventListener("change", () => {
    debugConsole.classList.toggle("hidden", !optDebug.checked);
  });

  debugClearBtn.addEventListener("click", () => {
    debugLog.innerHTML = "";
  });

  usernameInput.focus();
  await checkDependencies();
});
