use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tokio::process::Command;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(target_os = "windows")]
const PYTHON_EXE: &str = "python.exe";
#[cfg(not(target_os = "windows"))]
const PYTHON_EXE: &str = "python3";

#[cfg(target_os = "windows")]
const TOR_EXE: &str = "tor.exe";
#[cfg(not(target_os = "windows"))]
const TOR_EXE: &str = "tor";

/// Apply CREATE_NO_WINDOW on Windows to hide console windows.
pub fn hide_window(cmd: &mut Command) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = cmd;
    }
}

/// Resolve the path to an embedded executable in the resource directory.
fn get_embedded_path(app: &AppHandle, subdir: &str, exe: &str) -> Result<PathBuf, String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;

    let prod_path = resource_dir.join(subdir).join(exe);
    if prod_path.exists() {
        return Ok(prod_path);
    }

    let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(subdir)
        .join(exe);
    if dev_path.exists() {
        return Ok(dev_path);
    }

    Err(format!(
        "Embedded {} not found in {}. Please reinstall the application.",
        exe, subdir
    ))
}

pub fn get_python_path(app: &AppHandle) -> Result<PathBuf, String> {
    get_embedded_path(app, "python-embed", PYTHON_EXE)
}

pub fn get_tor_path(app: &AppHandle) -> Result<PathBuf, String> {
    get_embedded_path(app, "tor", TOR_EXE)
}
