import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";

let usernameInput: HTMLInputElement;
let searchBtn: HTMLButtonElement;
let cancelBtn: HTMLButtonElement;
let searchForm: HTMLFormElement;
let statusEl: HTMLElement;
let resultsList: HTMLElement;
let resultsSection: HTMLElement;
let resultCount: HTMLElement;
let depBanner: HTMLElement;
let depMessage: HTMLElement;

let isSearching = false;
let count = 0;

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

function setStatus(message: string, type: "info" | "searching" | "complete" | "error" = "info") {
  statusEl.className = `status ${type}`;
  statusEl.classList.remove("hidden");

  if (type === "searching") {
    statusEl.innerHTML = `<span class="spinner"></span>${escapeHtml(message)}`;
  } else {
    statusEl.textContent = message;
  }
}

function addResult(result: SherlockResult) {
  count++;
  resultCount.textContent = count.toString();
  resultsSection.classList.remove("hidden");

  const item = document.createElement("div");
  item.className = "result-item";
  item.innerHTML = `
    <div class="result-left">
      <span class="result-site">${escapeHtml(result.site)}</span>
      <a class="result-url" data-url="${escapeHtml(result.url)}">${escapeHtml(result.url)}</a>
    </div>
    <span class="result-badge">Found</span>
  `;

  const link = item.querySelector(".result-url") as HTMLElement;
  link.addEventListener("click", (e) => {
    e.preventDefault();
    const url = link.getAttribute("data-url");
    if (url) {
      openUrl(url);
    }
  });

  resultsList.appendChild(item);
  item.scrollIntoView({ behavior: "smooth", block: "nearest" });
}

function escapeHtml(str: string): string {
  const div = document.createElement("div");
  div.textContent = str;
  return div.innerHTML;
}

function setSearching(searching: boolean) {
  isSearching = searching;
  searchBtn.classList.toggle("hidden", searching);
  cancelBtn.classList.toggle("hidden", !searching);
  usernameInput.disabled = searching;
}

async function checkDependencies() {
  try {
    const deps = await invoke<{ python: boolean; sherlock: boolean; python_path: string }>("check_dependencies");

    if (!deps.python || !deps.sherlock) {
      depBanner.classList.remove("hidden");
      depBanner.classList.add("banner-error");
      depMessage.textContent = "Embedded Python or Sherlock not found. The application may not have been built correctly.";
      searchBtn.disabled = true;
      return;
    }

    // All good - brief confirmation
    depBanner.classList.remove("hidden");
    depBanner.classList.add("banner-success");
    depMessage.textContent = "Ready";
    setTimeout(() => depBanner.classList.add("hidden"), 2000);
  } catch (e) {
    depBanner.classList.remove("hidden");
    depBanner.classList.add("banner-error");
    depMessage.textContent = `Error: ${e}`;
  }
}

async function startSearch() {
  const username = usernameInput.value.trim();
  if (!username) return;

  count = 0;
  resultCount.textContent = "0";
  resultsList.innerHTML = "";
  resultsSection.classList.add("hidden");

  setSearching(true);
  setStatus(`Searching for "${username}" across social networks...`, "searching");

  try {
    await invoke("search_username", { username });
  } catch (e) {
    setStatus(`Error: ${e}`, "error");
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
}

window.addEventListener("DOMContentLoaded", async () => {
  usernameInput = document.querySelector("#username-input")!;
  searchBtn = document.querySelector("#search-btn")!;
  cancelBtn = document.querySelector("#cancel-btn")!;
  searchForm = document.querySelector("#search-form")!;
  statusEl = document.querySelector("#status")!;
  resultsList = document.querySelector("#results-list")!;
  resultsSection = document.querySelector("#results-section")!;
  resultCount = document.querySelector("#result-count")!;
  depBanner = document.querySelector("#dep-banner")!;
  depMessage = document.querySelector("#dep-message")!;

  await listen<SearchEvent>("sherlock-event", (event) => {
    const data = event.payload;

    switch (data.event_type) {
      case "result":
        if (data.result) {
          addResult(data.result);
        }
        break;
      case "info":
        if (isSearching) {
          setStatus(data.message, "searching");
        }
        break;
      case "error":
        setStatus(data.message, "error");
        break;
      case "complete":
        setStatus(data.message, "complete");
        break;
    }
  });

  searchForm.addEventListener("submit", (e) => {
    e.preventDefault();
    if (!isSearching) {
      startSearch();
    }
  });

  cancelBtn.addEventListener("click", cancelSearch);
  usernameInput.focus();

  await checkDependencies();
});
