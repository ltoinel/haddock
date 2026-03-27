use tauri::AppHandle;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::events::emit_event;
use crate::process::{get_tor_path, hide_window};

const TOR_SOCKS_PORT: u16 = 9050;
const TOR_STARTUP_TIMEOUT_SECS: u64 = 60;

/// Start the embedded Tor process and wait for it to bootstrap.
pub async fn start_tor(
    app: &AppHandle,
    debug: bool,
) -> Result<tokio::process::Child, String> {
    let tor_path = get_tor_path(app)?;
    let tor_dir = tor_path.parent().ok_or("Invalid Tor path")?;

    let data_dir = tor_dir.join("data_dir");
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create Tor data directory: {}", e))?;

    emit_event(app, "info", "Starting Tor...", None);
    emit_event(app, "tor-status", "connecting", None);

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
                    emit_event(&app_clone, "debug", &format!("[TOR] {}", trimmed), None);
                }
                if trimmed.contains("Bootstrapped") {
                    if let Some(pct) = trimmed.split("Bootstrapped ").nth(1) {
                        let progress = pct.split('%').next().unwrap_or("").trim();
                        emit_event(
                            &app_clone,
                            "tor-status",
                            &format!("connecting:{}%", progress),
                            None,
                        );
                    }
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
            emit_event(app, "tor-status", "connected", None);
            Ok(child)
        }
        Ok(false) => {
            emit_event(app, "tor-status", "error", None);
            let _ = child.kill().await;
            Err("Tor process exited before bootstrapping".to_string())
        }
        Err(_) => {
            emit_event(app, "tor-status", "error", None);
            let _ = child.kill().await;
            Err(format!(
                "Tor failed to connect within {} seconds",
                TOR_STARTUP_TIMEOUT_SECS
            ))
        }
    }
}
