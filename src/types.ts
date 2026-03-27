export type EventType = "result" | "info" | "error" | "debug" | "tor-status" | "progress" | "complete";

export interface SherlockResult {
  site: string;
  url: string;
  found: boolean;
}

export interface SearchEvent {
  event_type: EventType;
  message: string;
  result: SherlockResult | null;
}

export interface SearchOptions {
  timeout: number;
  proxy: string;
  sites: string[];
  nsfw: boolean;
  print_all: boolean;
  browse: boolean;
  tor: boolean;
  debug: boolean;
}

export interface ResultEntry {
  site: string;
  url: string;
  found: boolean;
  element: HTMLElement;
}

export const DEFAULT_TIMEOUT = 60;
