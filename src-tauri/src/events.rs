use tauri::{AppHandle, Emitter};

use crate::models::{SearchEvent, SherlockResult};

pub fn emit_event(
    app: &AppHandle,
    event_type: &str,
    message: &str,
    result: Option<SherlockResult>,
) {
    let _ = app.emit(
        "sherlock-event",
        SearchEvent {
            event_type: event_type.to_string(),
            message: message.to_string(),
            result,
        },
    );
}
