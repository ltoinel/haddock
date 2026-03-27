import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";

import { bindDOM } from "./dom";
import type { SearchEvent, SearchOptions, ResultEntry } from "./types";
import { DEFAULT_TIMEOUT } from "./types";
import {
  addResult,
  appendDebugLine,
  clearResults,
  copyAllUrls,
  filterResults,
  setSearching,
  updateTorStatus,
} from "./ui";

window.addEventListener("DOMContentLoaded", async () => {
  const dom = bindDOM();

  // --- Shared state ---
  const isSearching = { value: false };
  const counters = { found: 0, notFound: 0 };
  let allResults: ResultEntry[] = [];

  // --- Helpers ---
  function getOptions(): SearchOptions {
    const sitesRaw = dom.optSites.value.trim();
    const sites = sitesRaw ? sitesRaw.split(",").map(s => s.trim()).filter(Boolean) : [];
    return {
      timeout: Math.max(1, parseInt(dom.optTimeout.value) || DEFAULT_TIMEOUT),
      proxy: dom.optProxy.value.trim(),
      sites,
      nsfw: dom.optNsfw.checked,
      print_all: dom.optPrintAll.checked,
      browse: dom.optBrowse.checked,
      tor: dom.optTor.checked,
      debug: true,
    };
  }

  async function checkDependencies() {
    try {
      const deps = await invoke<{ python: boolean; sherlock: boolean }>("check_dependencies");
      if (!deps.python || !deps.sherlock) {
        dom.depDot.className = "dot dot-error";
        dom.depText.textContent = "Sherlock unavailable — please reinstall";
        dom.searchBtn.disabled = true;
        return;
      }
      dom.depDot.className = "dot dot-ok";
      dom.depText.textContent = "Ready";
    } catch {
      dom.depDot.className = "dot dot-error";
      dom.depText.textContent = "Error";
    }
  }

  async function startSearch() {
    const raw = dom.usernameInput.value.trim();
    if (!raw) return;

    const usernames = raw.split(/\s+/).filter(Boolean);
    const options = getOptions();

    clearResults(dom, allResults, counters);
    allResults = [];
    dom.resultsToolbar.classList.remove("hidden");
    dom.emptyState.classList.add("hidden");

    setSearching(dom, true, isSearching);
    dom.progressText.textContent = `Searching for ${usernames.join(", ")}...`;
    dom.progressCounter.textContent = "";
    dom.debugLog.innerHTML = "";

    try {
      await invoke("search_username", { usernames, options });
    } catch (e) {
      dom.progressText.textContent = `Error: ${e}`;
      dom.progressFill.style.background = "var(--error)";
    }

    setSearching(dom, false, isSearching);
  }

  async function cancelSearch() {
    try { await invoke("cancel_search"); } catch { /* ignore */ }
    setSearching(dom, false, isSearching);
    dom.progressText.textContent = "Search cancelled";
  }

  // --- Event listener ---
  await listen<SearchEvent>("sherlock-event", (event) => {
    const { event_type, message, result } = event.payload;
    switch (event_type) {
      case "result":
        if (result) addResult(dom, result, allResults, counters);
        break;
      case "info":
        if (isSearching.value) dom.progressText.textContent = message;
        break;
      case "error":
        dom.progressText.textContent = message;
        break;
      case "debug":
        if (message.startsWith("[DEBUG] Command:")) appendDebugLine(dom, message, "cmd");
        else if (message.startsWith("[STDERR]")) appendDebugLine(dom, message, "stderr");
        else appendDebugLine(dom, message, "stdout");
        break;
      case "tor-status":
        updateTorStatus(dom, message);
        break;
      case "progress":
        dom.progressCounter.textContent = message;
        break;
      case "complete":
        dom.progressText.textContent = message;
        dom.progressFill.classList.remove("indeterminate");
        dom.progressFill.style.width = "100%";
        break;
    }
  });

  // --- UI event bindings ---
  dom.searchForm.addEventListener("submit", (e) => {
    e.preventDefault();
    if (!isSearching.value) startSearch();
  });

  dom.cancelBtn.addEventListener("click", cancelSearch);

  dom.optionsToggle.addEventListener("click", () => {
    dom.optionsPanel.classList.toggle("hidden");
    dom.optionsChevron.classList.toggle("open");
  });

  let filterTimer: ReturnType<typeof setTimeout>;
  dom.filterInput.addEventListener("input", () => {
    clearTimeout(filterTimer);
    filterTimer = setTimeout(() => filterResults(dom, allResults), 150);
  });

  // Tor ↔ Proxy mutual exclusion
  const TOR_PROXY = "socks5://127.0.0.1:9050";
  dom.optTor.addEventListener("change", () => {
    if (dom.optTor.checked) {
      dom.optProxy.value = TOR_PROXY;
      dom.optProxy.disabled = true;
    } else {
      dom.optProxy.value = "";
      dom.optProxy.disabled = false;
    }
  });
  dom.optProxy.addEventListener("input", () => {
    if (dom.optProxy.value.trim() && dom.optTor.checked) {
      dom.optTor.checked = false;
      dom.optProxy.disabled = false;
    }
  });

  dom.copyBtn.addEventListener("click", () => copyAllUrls(allResults));
  dom.clearBtn.addEventListener("click", () => {
    clearResults(dom, allResults, counters);
    allResults = [];
  });

  // --- Debug console ---
  type DebugState = "collapsed" | "open" | "expanded";

  function getDebugState(): DebugState {
    if (dom.debugConsole.classList.contains("collapsed")) return "collapsed";
    if (dom.debugConsole.classList.contains("expanded")) return "expanded";
    return "open";
  }

  function setDebugState(state: DebugState) {
    dom.debugConsole.classList.remove("collapsed", "expanded");
    if (state === "collapsed") {
      dom.debugConsole.classList.add("collapsed");
      dom.expandIcon.innerHTML = '<path d="m18 15-6-6-6 6"/>';
      dom.app.style.paddingBottom = "36px";
    } else if (state === "expanded") {
      dom.debugConsole.classList.add("expanded");
      dom.expandIcon.innerHTML = '<path d="m6 9 6 6 6-6"/>';
      dom.app.style.paddingBottom = "calc(50vh + 50px)";
    } else {
      dom.expandIcon.innerHTML = '<path d="m18 15-6-6-6 6"/>';
      dom.app.style.paddingBottom = "200px";
    }
  }

  dom.debugExpandBtn.addEventListener("click", (e) => {
    e.stopPropagation();
    const s = getDebugState();
    if (s === "collapsed") setDebugState("open");
    else if (s === "open") setDebugState("expanded");
    else setDebugState("collapsed");
  });

  document.querySelector(".debug-sheet-header")!.addEventListener("click", (e) => {
    if ((e.target as HTMLElement).closest(".debug-sheet-actions")) return;
    setDebugState(getDebugState() === "collapsed" ? "open" : "collapsed");
  });

  setDebugState("collapsed");

  dom.debugClearBtn.addEventListener("click", () => { dom.debugLog.innerHTML = ""; });

  dom.sherlockLink.addEventListener("click", (e) => {
    e.preventDefault();
    openUrl("https://github.com/sherlock-project/sherlock");
  });

  // --- Init ---
  dom.usernameInput.focus();
  await checkDependencies();
});
