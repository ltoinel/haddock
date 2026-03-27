/** Safely query a DOM element by selector. Throws a clear error if missing. */
export function getElement<T extends HTMLElement>(selector: string): T {
  const el = document.querySelector<T>(selector);
  if (!el) {
    throw new Error(`Missing DOM element: ${selector}`);
  }
  return el;
}

/** All DOM references used by the app, resolved once at startup. */
export function bindDOM() {
  return {
    usernameInput: getElement<HTMLInputElement>("#username-input"),
    searchBtn: getElement<HTMLButtonElement>("#search-btn"),
    cancelBtn: getElement<HTMLButtonElement>("#cancel-btn"),
    searchForm: getElement<HTMLFormElement>("#search-form"),
    depDot: getElement("#dep-dot"),
    depText: getElement("#dep-text"),
    optionsToggle: getElement<HTMLButtonElement>("#options-toggle"),
    optionsChevron: getElement("#options-chevron"),
    optionsPanel: getElement("#options-panel"),
    progressSection: getElement("#progress-section"),
    progressText: getElement("#progress-text"),
    progressCounter: getElement("#progress-counter"),
    progressFill: getElement("#progress-fill"),
    resultsToolbar: getElement("#results-toolbar"),
    resultsGrid: getElement("#results-grid"),
    resultCount: getElement("#result-count"),
    notFoundCount: getElement("#not-found-count"),
    filterInput: getElement<HTMLInputElement>("#filter-input"),
    copyBtn: getElement<HTMLButtonElement>("#copy-btn"),
    clearBtn: getElement<HTMLButtonElement>("#clear-btn"),
    emptyState: getElement("#empty-state"),
    optTimeout: getElement<HTMLInputElement>("#opt-timeout"),
    optProxy: getElement<HTMLInputElement>("#opt-proxy"),
    optSites: getElement<HTMLInputElement>("#opt-sites"),
    optNsfw: getElement<HTMLInputElement>("#opt-nsfw"),
    optPrintAll: getElement<HTMLInputElement>("#opt-print-all"),
    optBrowse: getElement<HTMLInputElement>("#opt-browse"),
    optTor: getElement<HTMLInputElement>("#opt-tor"),
    torStatus: getElement("#tor-status"),
    torText: getElement("#tor-text"),
    debugConsole: getElement("#debug-console"),
    debugLog: getElement("#debug-log"),
    debugClearBtn: getElement<HTMLButtonElement>("#debug-clear"),
    debugExpandBtn: getElement<HTMLButtonElement>("#debug-expand"),
    expandIcon: getElement("#debug-expand-icon"),
    app: getElement("#app"),
    sherlockLink: getElement("#sherlock-link"),
    appVersion: getElement("#app-version"),
  };
}

export type DOM = ReturnType<typeof bindDOM>;
