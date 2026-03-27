use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Windows flag to prevent child processes from spawning a visible console window.
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Clone, Serialize)]
struct SherlockResult {
    site: String,
    url: String,
    found: bool,
}

#[derive(Clone, Serialize)]
struct SearchEvent {
    event_type: String,
    message: String,
    result: Option<SherlockResult>,
}

#[derive(Deserialize)]
struct SearchOptions {
    timeout: u32,
    proxy: String,
    sites: Vec<String>,
    nsfw: bool,
    print_all: bool,
    browse: bool,
    csv: bool,
    xlsx: bool,
    debug: bool,
}

static SEARCH_RUNNING: AtomicBool = AtomicBool::new(false);

/// Apply CREATE_NO_WINDOW on Windows to hide console windows.
#[cfg(target_os = "windows")]
fn hide_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(target_os = "windows"))]
fn hide_window(_cmd: &mut Command) {}

fn get_python_path(app: &AppHandle) -> Result<PathBuf, String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;

    let python_path = resource_dir.join("python-embed").join("python.exe");
    if python_path.exists() {
        return Ok(python_path);
    }

    let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("python-embed")
        .join("python.exe");
    if dev_path.exists() {
        return Ok(dev_path);
    }

    Err("Embedded Python not found. Run scripts/setup-python.ps1 first.".to_string())
}

#[tauri::command]
async fn check_dependencies(app: AppHandle) -> Result<serde_json::Value, String> {
    let python_path = get_python_path(&app);

    let (python_ok, sherlock_ok) = match &python_path {
        Ok(path) => {
            let mut cmd = Command::new(path);
            cmd.arg("--version")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
            hide_window(&mut cmd);
            let python_ok = cmd.status().await.map(|s| s.success()).unwrap_or(false);

            let sherlock_ok = if python_ok {
                let mut cmd = Command::new(path);
                cmd.args(["-m", "sherlock_project", "--version"])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null());
                hide_window(&mut cmd);
                cmd.status().await.map(|s| s.success()).unwrap_or(false)
            } else {
                false
            };

            (python_ok, sherlock_ok)
        }
        Err(_) => (false, false),
    };

    Ok(serde_json::json!({
        "python": python_ok,
        "sherlock": sherlock_ok,
        "python_path": python_path.as_ref().map(|p| p.display().to_string()).unwrap_or_default(),
    }))
}

#[tauri::command]
async fn cancel_search() -> Result<(), String> {
    SEARCH_RUNNING.store(false, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
async fn search_username(
    app: AppHandle,
    usernames: Vec<String>,
    options: SearchOptions,
) -> Result<(), String> {
    if usernames.is_empty() {
        return Err("At least one username is required".to_string());
    }

    for username in &usernames {
        if username.trim().is_empty() {
            return Err("Username cannot be empty".to_string());
        }
        if !username
            .chars()
            .all(|c| c.is_alphanumeric() || c == '.' || c == '_' || c == '-' || c == '?')
        {
            return Err(format!(
                "Invalid username '{}': only alphanumeric, dots, underscores, hyphens and ? are allowed",
                username
            ));
        }
    }

    let python_path = get_python_path(&app)?;
    let debug = options.debug;

    SEARCH_RUNNING.store(true, Ordering::SeqCst);

    let label = usernames.join(", ");
    emit_event(
        &app,
        "info",
        &format!("Searching for: {}", label),
        None,
    );

    // Build command args
    let mut args: Vec<String> = vec!["-m".to_string(), "sherlock_project".to_string()];

    for u in &usernames {
        args.push(u.clone());
    }

    if options.print_all {
        args.push("--print-all".to_string());
    } else {
        args.push("--print-found".to_string());
    }

    if options.timeout > 0 && options.timeout != 60 {
        args.push("--timeout".to_string());
        args.push(options.timeout.to_string());
    }

    if !options.proxy.is_empty() {
        args.push("--proxy".to_string());
        args.push(options.proxy.clone());
    }

    for site in &options.sites {
        args.push("--site".to_string());
        args.push(site.clone());
    }

    if options.nsfw {
        args.push("--nsfw".to_string());
    }

    if options.browse {
        args.push("--browse".to_string());
    }

    if options.csv {
        args.push("--csv".to_string());
    }

    if options.xlsx {
        args.push("--xlsx".to_string());
    }

    if debug {
        args.push("--verbose".to_string());
    }

    args.push("--no-color".to_string());

    if debug {
        emit_event(
            &app,
            "debug",
            &format!("[DEBUG] Command: {} {}", python_path.display(), args.join(" ")),
            None,
        );
    }

    let mut cmd = Command::new(&python_path);
    cmd.args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    hide_window(&mut cmd);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start sherlock: {}", e))?;

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

    // Read stderr in background — emit lines in debug mode
    let app_for_stderr = app.clone();
    let stderr_handle = tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        let mut error_output = String::new();
        while let Ok(Some(line)) = lines.next_line().await {
            if !line.trim().is_empty() {
                if debug {
                    emit_event(
                        &app_for_stderr,
                        "debug",
                        &format!("[STDERR] {}", line.trim()),
                        None,
                    );
                }
                error_output.push_str(&line);
                error_output.push('\n');
            }
        }
        error_output
    });

    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();
    let mut found_count = 0u32;
    let mut checked_count = 0u32;

    while let Ok(Some(line)) = lines.next_line().await {
        if !SEARCH_RUNNING.load(Ordering::SeqCst) {
            let _ = child.kill().await;
            emit_event(&app, "complete", "Search cancelled", None);
            return Ok(());
        }

        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }

        if debug {
            emit_event(
                &app,
                "debug",
                &format!("[STDOUT] {}", trimmed),
                None,
            );
        }

        if trimmed.starts_with("[+]") {
            if let Some(rest) = trimmed.strip_prefix("[+]") {
                let rest = rest.trim();
                if let Some((site, _)) = rest.split_once(':') {
                    let site = site.trim().to_string();
                    let url = rest[site.len() + 1..].trim().to_string();
                    found_count += 1;
                    checked_count += 1;
                    emit_event(
                        &app,
                        "result",
                        &trimmed,
                        Some(SherlockResult {
                            site,
                            url,
                            found: true,
                        }),
                    );
                    emit_event(
                        &app,
                        "progress",
                        &format!("{} found / {} checked", found_count, checked_count),
                        None,
                    );
                }
            }
        } else if trimmed.starts_with("[-]") {
            if let Some(rest) = trimmed.strip_prefix("[-]") {
                let rest = rest.trim();
                let site = if let Some((s, _)) = rest.split_once(':') {
                    s.trim().to_string()
                } else {
                    rest.to_string()
                };
                checked_count += 1;

                if options.print_all {
                    emit_event(
                        &app,
                        "result",
                        &trimmed,
                        Some(SherlockResult {
                            site,
                            url: String::new(),
                            found: false,
                        }),
                    );
                }

                emit_event(
                    &app,
                    "progress",
                    &format!("{} found / {} checked", found_count, checked_count),
                    None,
                );
            }
        } else {
            emit_event(&app, "info", &trimmed, None);
        }
    }

    let _ = child.wait().await;
    let stderr_output = stderr_handle.await.unwrap_or_default();

    if !stderr_output.is_empty() && found_count == 0 {
        emit_event(
            &app,
            "error",
            &format!("Sherlock error: {}", stderr_output.trim()),
            None,
        );
    }

    SEARCH_RUNNING.store(false, Ordering::SeqCst);
    emit_event(
        &app,
        "complete",
        &format!(
            "Done. {} found across {} sites checked.",
            found_count, checked_count
        ),
        None,
    );

    Ok(())
}

fn emit_event(app: &AppHandle, event_type: &str, message: &str, result: Option<SherlockResult>) {
    let _ = app.emit(
        "sherlock-event",
        SearchEvent {
            event_type: event_type.to_string(),
            message: message.to_string(),
            result,
        },
    );
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            check_dependencies,
            search_username,
            cancel_search,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
