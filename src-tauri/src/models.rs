use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize)]
pub struct SherlockResult {
    pub site: String,
    pub url: String,
    pub found: bool,
}

#[derive(Clone, Serialize)]
pub struct SearchEvent {
    pub event_type: String,
    pub message: String,
    pub result: Option<SherlockResult>,
}

#[derive(Deserialize)]
pub struct SearchOptions {
    pub timeout: u32,
    pub proxy: String,
    pub sites: Vec<String>,
    pub nsfw: bool,
    pub print_all: bool,
    pub browse: bool,
    pub tor: bool,
    pub debug: bool,
}

/// Default Sherlock timeout in seconds (must match frontend default).
pub const DEFAULT_TIMEOUT: u32 = 60;
