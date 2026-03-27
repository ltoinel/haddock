import { openUrl } from "@tauri-apps/plugin-opener";
import type { DOM } from "./dom";
import type { SherlockResult, ResultEntry } from "./types";

export function escapeHtml(str: string): string {
  const div = document.createElement("div");
  div.textContent = str;
  return div.innerHTML;
}

export function showToast(message: string) {
  const toast = document.createElement("div");
  toast.className = "toast";
  toast.textContent = message;
  document.body.appendChild(toast);
  setTimeout(() => toast.remove(), 3000);
}

export function appendDebugLine(dom: DOM, message: string, type: "stdout" | "stderr" | "cmd" | "error" = "stdout") {
  const line = document.createElement("div");
  line.className = `debug-line ${type}`;
  line.textContent = message;
  dom.debugLog.appendChild(line);
  dom.debugLog.scrollTop = dom.debugLog.scrollHeight;
}

export function setSearching(dom: DOM, searching: boolean, isSearchingRef: { value: boolean }) {
  isSearchingRef.value = searching;
  dom.searchBtn.classList.toggle("hidden", searching);
  dom.cancelBtn.classList.toggle("hidden", !searching);
  dom.usernameInput.disabled = searching;
  dom.progressSection.classList.toggle("hidden", !searching);

  if (searching) {
    dom.emptyState.classList.add("hidden");
    dom.progressFill.style.width = "0%";
    dom.progressFill.style.background = "";
    dom.progressFill.classList.add("indeterminate");
  } else {
    dom.progressFill.classList.remove("indeterminate");
    dom.progressFill.style.width = "100%";
  }
}

export function addResult(
  dom: DOM,
  result: SherlockResult,
  allResults: ResultEntry[],
  counters: { found: number; notFound: number },
) {
  if (result.found) {
    counters.found++;
  } else {
    counters.notFound++;
  }

  dom.resultCount.textContent = counters.found.toString();
  dom.resultsToolbar.classList.remove("hidden");

  if (counters.notFound > 0) {
    dom.notFoundCount.textContent = `${counters.notFound} not found`;
    dom.notFoundCount.classList.remove("hidden");
  }

  const card = document.createElement("div");
  card.className = `result-card ${result.found ? "found" : "not-found"}`;
  card.dataset.site = result.site.toLowerCase();

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

  dom.resultsGrid.appendChild(card);
  allResults.push({ site: result.site, url: result.url, found: result.found, element: card });

  const filterVal = dom.filterInput.value.toLowerCase();
  if (filterVal && !result.site.toLowerCase().includes(filterVal)) {
    card.classList.add("hidden");
  }
}

export function filterResults(dom: DOM, allResults: ResultEntry[]) {
  const query = dom.filterInput.value.toLowerCase();
  for (const r of allResults) {
    const match = r.site.toLowerCase().includes(query) || r.url.toLowerCase().includes(query);
    r.element.classList.toggle("hidden", !match);
  }
}

export function clearResults(dom: DOM, allResults: ResultEntry[], counters: { found: number; notFound: number }) {
  counters.found = 0;
  counters.notFound = 0;
  allResults.length = 0;
  dom.resultsGrid.innerHTML = "";
  dom.resultCount.textContent = "0";
  dom.notFoundCount.classList.add("hidden");
  dom.resultsToolbar.classList.add("hidden");
  dom.progressSection.classList.add("hidden");
  dom.emptyState.classList.remove("hidden");
  dom.filterInput.value = "";
}

export function updateTorStatus(dom: DOM, status: string) {
  dom.torStatus.classList.remove("hidden", "tor-connecting", "tor-connected", "tor-error", "tor-stopped");

  if (status === "connecting" || status.startsWith("connecting:")) {
    dom.torStatus.classList.add("tor-connecting");
    const pct = status.includes(":") ? status.split(":")[1] : "...";
    dom.torText.textContent = `Tor ${pct}`;
  } else if (status === "connected") {
    dom.torStatus.classList.add("tor-connected");
    dom.torText.textContent = "Tor OK";
  } else if (status === "error") {
    dom.torStatus.classList.add("tor-error");
    dom.torText.textContent = "Tor error";
    setTimeout(() => dom.torStatus.classList.add("hidden"), 5000);
  } else if (status === "stopped") {
    dom.torStatus.classList.add("tor-stopped");
    dom.torText.textContent = "Tor off";
    setTimeout(() => dom.torStatus.classList.add("hidden"), 3000);
  }
}

export function copyAllUrls(allResults: ResultEntry[]) {
  const found = allResults.filter(r => r.found);
  const urls = found.map(r => r.url).join("\n");
  if (!urls) {
    showToast("No URLs to copy");
    return;
  }
  navigator.clipboard.writeText(urls).then(
    () => showToast(`${found.length} URLs copied to clipboard`),
    () => showToast("Failed to copy to clipboard"),
  );
}
