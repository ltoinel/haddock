mod events;
mod models;
mod process;
mod tor;
mod validation;

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use events::emit_event;
use models::{DEFAULT_TIMEOUT, SearchOptions, SherlockResult};
use process::{get_python_path, get_tor_path, hide_window};
use validation::{validate_proxy, validate_site_name, validate_username};

const GLOBAL_SEARCH_TIMEOUT_SECS: u64 = 600;

static SEARCH_RUNNING: AtomicBool = AtomicBool::new(false);

#[tauri::command]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
async fn check_dependencies(app: AppHandle) -> Result<serde_json::Value, String> {
    let python_path = get_python_path(&app);
    let tor_available = get_tor_path(&app).is_ok();

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
        "tor": tor_available,
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
    if SEARCH_RUNNING.load(Ordering::SeqCst) {
        return Err("A search is already in progress".to_string());
    }

    if usernames.is_empty() {
        return Err("At least one username is required".to_string());
    }

    for username in &usernames {
        validate_username(username)?;
    }
    validate_proxy(&options.proxy)?;
    for site in &options.sites {
        validate_site_name(site)?;
    }

    let python_path = get_python_path(&app)?;
    let debug = options.debug;

    SEARCH_RUNNING.store(true, Ordering::SeqCst);

    // Start Tor if requested
    let mut tor_process: Option<tokio::process::Child> = None;
    if options.tor {
        match tor::start_tor(&app, debug).await {
            Ok(child) => tor_process = Some(child),
            Err(e) => {
                SEARCH_RUNNING.store(false, Ordering::SeqCst);
                emit_event(&app, "error", &format!("Tor error: {}", e), None);
                emit_event(&app, "complete", "Search aborted — Tor failed to start", None);
                return Err(e);
            }
        }
    }

    let label = usernames.join(", ");
    emit_event(&app, "info", &format!("Searching for: {}", label), None);

    let args = build_sherlock_args(&usernames, &options);

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

    let app_for_read = app.clone();
    let print_all = options.print_all;
    let read_result = tokio::time::timeout(
        std::time::Duration::from_secs(GLOBAL_SEARCH_TIMEOUT_SECS),
        read_sherlock_output(stdout, &mut child, &app_for_read, debug, print_all),
    )
    .await;

    let stderr_output = stderr_handle.await.unwrap_or_default();

    // Stop Tor
    if let Some(mut tor_child) = tor_process {
        if debug {
            emit_event(&app, "debug", "[DEBUG] Stopping Tor...", None);
        }
        let _ = tor_child.kill().await;
        emit_event(&app, "tor-status", "stopped", None);
    }

    match read_result {
        Ok((found_count, checked_count, cancelled)) => {
            if !stderr_output.is_empty() {
                let event_type = if found_count == 0 { "error" } else { "info" };
                emit_event(
                    &app,
                    event_type,
                    &format!("Sherlock stderr: {}", stderr_output.trim()),
                    None,
                );
            }
            if !cancelled {
                SEARCH_RUNNING.store(false, Ordering::SeqCst);
                emit_event(
                    &app,
                    "complete",
                    &format!("Done. {} found across {} sites checked.", found_count, checked_count),
                    None,
                );
            }
        }
        Err(_) => {
            SEARCH_RUNNING.store(false, Ordering::SeqCst);
            emit_event(
                &app,
                "error",
                &format!("Search timed out after {} seconds.", GLOBAL_SEARCH_TIMEOUT_SECS),
                None,
            );
            emit_event(&app, "complete", "Search timed out", None);
        }
    }

    Ok(())
}

fn build_sherlock_args(usernames: &[String], options: &SearchOptions) -> Vec<String> {
    let mut args = vec!["-m".to_string(), "sherlock_project".to_string()];

    for u in usernames {
        args.push(u.clone());
    }

    args.push(if options.print_all { "--print-all" } else { "--print-found" }.to_string());

    if options.timeout > 0 && options.timeout != DEFAULT_TIMEOUT {
        args.push("--timeout".to_string());
        args.push(options.timeout.to_string());
    }

    if options.tor {
        args.push("--proxy".to_string());
        args.push(format!("socks5://127.0.0.1:{}", crate::tor::TOR_SOCKS_PORT));
    } else if !options.proxy.is_empty() {
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
    if options.debug {
        args.push("--verbose".to_string());
    }

    args.push("--no-color".to_string());
    args
}

async fn read_sherlock_output(
    stdout: tokio::process::ChildStdout,
    child: &mut tokio::process::Child,
    app: &AppHandle,
    debug: bool,
    print_all: bool,
) -> (u32, u32, bool) {
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();
    let mut found_count = 0u32;
    let mut checked_count = 0u32;

    while let Ok(Some(line)) = lines.next_line().await {
        if !SEARCH_RUNNING.load(Ordering::SeqCst) {
            let _ = child.kill().await;
            emit_event(app, "complete", "Search cancelled", None);
            return (found_count, checked_count, true);
        }

        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }

        if debug {
            emit_event(app, "debug", &format!("[STDOUT] {}", trimmed), None);
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
                        app,
                        "result",
                        &trimmed,
                        Some(SherlockResult { site, url, found: true }),
                    );
                    emit_event(
                        app,
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

                if print_all {
                    emit_event(
                        app,
                        "result",
                        &trimmed,
                        Some(SherlockResult { site, url: String::new(), found: false }),
                    );
                }

                emit_event(
                    app,
                    "progress",
                    &format!("{} found / {} checked", found_count, checked_count),
                    None,
                );
            }
        } else {
            emit_event(app, "info", &trimmed, None);
        }
    }

    let _ = child.wait().await;
    (found_count, checked_count, false)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_version,
            check_dependencies,
            search_username,
            cancel_search,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
