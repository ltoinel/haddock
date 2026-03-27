use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const MAX_USERNAME_LEN: usize = 64;
const GLOBAL_SEARCH_TIMEOUT_SECS: u64 = 600;
const TOR_SOCKS_PORT: u16 = 9050;
const TOR_STARTUP_TIMEOUT_SECS: u64 = 60;

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
    tor: bool,
    debug: bool,
}

static SEARCH_RUNNING: AtomicBool = AtomicBool::new(false);

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

    Err("Embedded Python not found. Please reinstall the application.".to_string())
}

fn get_tor_path(app: &AppHandle) -> Result<PathBuf, String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;

    let tor_path = resource_dir.join("tor").join("tor.exe");
    if tor_path.exists() {
        return Ok(tor_path);
    }

    let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tor")
        .join("tor.exe");
    if dev_path.exists() {
        return Ok(dev_path);
    }

    Err("Embedded Tor not found. Please reinstall the application.".to_string())
}

/// Start the embedded Tor process and wait for it to be ready.
async fn start_tor(
    app: &AppHandle,
    debug: bool,
) -> Result<tokio::process::Child, String> {
    let tor_path = get_tor_path(app)?;
    let tor_dir = tor_path
        .parent()
        .ok_or("Invalid Tor path")?;

    let data_dir = tor_dir.join("data_dir");
    let _ = std::fs::create_dir_all(&data_dir);

    emit_event(app, "info", "Starting Tor...", None);

    if debug {
        emit_event(
            app,
            "debug",
            &format!("[DEBUG] Tor path: {}", tor_path.display()),
            None,
        );
    }

    let mut cmd = Command::new(&tor_path);
    cmd.arg("--SocksPort")
        .arg(TOR_SOCKS_PORT.to_string())
        .arg("--DataDirectory")
        .arg(data_dir.to_str().unwrap_or("data_dir"))
        .arg("--GeoIPFile")
        .arg(tor_dir.join("geoip").to_str().unwrap_or(""))
        .arg("--GeoIPv6File")
        .arg(tor_dir.join("geoip6").to_str().unwrap_or(""))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    hide_window(&mut cmd);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start Tor: {}", e))?;

    // Wait for Tor to bootstrap (look for "100%" in stdout)
    let stdout = child.stdout.take().ok_or("Failed to capture Tor stdout")?;
    let app_clone = app.clone();

    let bootstrap_result = tokio::time::timeout(
        std::time::Duration::from_secs(TOR_STARTUP_TIMEOUT_SECS),
        async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let trimmed = line.trim().to_string();
                if debug && !trimmed.is_empty() {
                    emit_event(
                        &app_clone,
                        "debug",
                        &format!("[TOR] {}", trimmed),
                        None,
                    );
                }
                if trimmed.contains("Bootstrapped 100%") || trimmed.contains("Done") {
                    return true;
                }
            }
            false
        },
    )
    .await;

    match bootstrap_result {
        Ok(true) => {
            emit_event(app, "info", "Tor connected. Searching anonymously...", None);
            Ok(child)
        }
        Ok(false) => {
            let _ = child.kill().await;
            Err("Tor process exited before bootstrapping".to_string())
        }
        Err(_) => {
            let _ = child.kill().await;
            Err(format!(
                "Tor failed to connect within {} seconds",
                TOR_STARTUP_TIMEOUT_SECS
            ))
        }
    }
}

fn validate_proxy(proxy: &str) -> Result<(), String> {
    if proxy.is_empty() {
        return Ok(());
    }
    if !proxy
        .chars()
        .all(|c| c.is_alphanumeric() || ".:/-_@[]".contains(c))
    {
        return Err("Invalid proxy URL: contains forbidden characters".to_string());
    }
    if !(proxy.starts_with("http://")
        || proxy.starts_with("https://")
        || proxy.starts_with("socks4://")
        || proxy.starts_with("socks5://"))
    {
        return Err(
            "Invalid proxy URL: must start with http://, https://, socks4:// or socks5://"
                .to_string(),
        );
    }
    Ok(())
}

fn validate_site_name(site: &str) -> Result<(), String> {
    if site.is_empty() {
        return Err("Site name cannot be empty".to_string());
    }
    if !site
        .chars()
        .all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == ' ' || c == '_')
    {
        return Err(format!(
            "Invalid site name '{}': only alphanumeric, dots, hyphens, underscores and spaces are allowed",
            site
        ));
    }
    Ok(())
}

#[tauri::command]
async fn check_dependencies(app: AppHandle) -> Result<serde_json::Value, String> {
    let python_path = get_python_path(&app);
    let tor_path = get_tor_path(&app);

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
        "tor": tor_path.is_ok(),
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
        if username.trim().is_empty() {
            return Err("Username cannot be empty".to_string());
        }
        if username.len() > MAX_USERNAME_LEN {
            return Err(format!(
                "Username '{}' is too long (max {} characters)",
                username, MAX_USERNAME_LEN
            ));
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
        match start_tor(&app, debug).await {
            Ok(child) => {
                tor_process = Some(child);
            }
            Err(e) => {
                SEARCH_RUNNING.store(false, Ordering::SeqCst);
                emit_event(&app, "error", &format!("Tor error: {}", e), None);
                emit_event(&app, "complete", "Search aborted — Tor failed to start", None);
                return Err(e);
            }
        }
    }

    let label = usernames.join(", ");
    emit_event(
        &app,
        "info",
        &format!("Searching for: {}", label),
        None,
    );

    // Build Sherlock args
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

    // Tor proxy takes precedence over manual proxy
    if options.tor {
        args.push("--proxy".to_string());
        args.push(format!("socks5://127.0.0.1:{}", TOR_SOCKS_PORT));
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
        async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            let mut found_count = 0u32;
            let mut checked_count = 0u32;

            while let Ok(Some(line)) = lines.next_line().await {
                if !SEARCH_RUNNING.load(Ordering::SeqCst) {
                    let _ = child.kill().await;
                    emit_event(&app_for_read, "complete", "Search cancelled", None);
                    return (found_count, checked_count, true);
                }

                let trimmed = line.trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }

                if debug {
                    emit_event(
                        &app_for_read,
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
                                &app_for_read,
                                "result",
                                &trimmed,
                                Some(SherlockResult {
                                    site,
                                    url,
                                    found: true,
                                }),
                            );
                            emit_event(
                                &app_for_read,
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
                                &app_for_read,
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
                            &app_for_read,
                            "progress",
                            &format!("{} found / {} checked", found_count, checked_count),
                            None,
                        );
                    }
                } else {
                    emit_event(&app_for_read, "info", &trimmed, None);
                }
            }

            let _ = child.wait().await;
            (found_count, checked_count, false)
        },
    )
    .await;

    let stderr_output = stderr_handle.await.unwrap_or_default();

    // Stop Tor if we started it
    if let Some(mut tor) = tor_process {
        if debug {
            emit_event(&app, "debug", "[DEBUG] Stopping Tor...", None);
        }
        let _ = tor.kill().await;
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
                    &format!(
                        "Done. {} found across {} sites checked.",
                        found_count, checked_count
                    ),
                    None,
                );
            }
        }
        Err(_) => {
            SEARCH_RUNNING.store(false, Ordering::SeqCst);
            emit_event(
                &app,
                "error",
                &format!(
                    "Search timed out after {} seconds. The process was terminated.",
                    GLOBAL_SEARCH_TIMEOUT_SECS
                ),
                None,
            );
            emit_event(&app, "complete", "Search timed out", None);
        }
    }

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
