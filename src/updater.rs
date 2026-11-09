use serde::Deserialize;
use std::os::windows::process::CommandExt;
use std::process::Command;

const REPO: &str = "qir3k/sonixlink-app-by-qirke";
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Default)]
pub struct ReleaseInfo {
    pub tag: String,
    pub version: String,
    pub exe_url: String,
    pub exe_name: String,
    pub notes: String,
    pub is_newer: bool,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    body: Option<String>,
    assets: Vec<GhAsset>,
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn parse_version(s: &str) -> (u32, u32, u32) {
    let clean = s.trim().trim_start_matches('v');
    let mut parts = clean.split('.');
    let major = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let patch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    (major, minor, patch)
}

pub fn is_newer_version(remote: &str, local: &str) -> bool {
    let r = parse_version(remote);
    let l = parse_version(local);
    r > l
}

pub fn check_for_updates() -> Result<ReleaseInfo, String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);

    let output = Command::new("curl.exe")
        .args([
            "-s",
            "-H",
            "User-Agent: SonixLink-Desktop-Updater",
            "-H",
            "Accept: application/vnd.github+json",
            &url,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to run curl: {}", e))?;

    if !output.status.success() {
        return Err("Failed to query GitHub Releases".to_string());
    }

    let release: GhRelease = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Invalid release response: {}", e))?;

    let tag = release.tag_name.clone();
    let version = tag.trim_start_matches('v').to_string();
    let is_newer = is_newer_version(&version, current_version());

    let exe_asset = release
        .assets
        .iter()
        .find(|a| a.name.ends_with(".exe") && a.name.contains("windows-x64"))
        .or_else(|| release.assets.iter().find(|a| a.name.ends_with(".exe")))
        .ok_or_else(|| "No Windows executable found in release".to_string())?;

    Ok(ReleaseInfo {
        tag,
        version,
        exe_url: exe_asset.browser_download_url.clone(),
        exe_name: exe_asset.name.clone(),
        notes: release.body.unwrap_or_default(),
        is_newer,
    })
}

pub fn download_and_install_update(exe_url: &str) -> Result<(), String> {
    let current_exe = std::env::current_exe().map_err(|e| format!("Cannot locate current exe: {}", e))?;
    let target_dir = current_exe.parent().unwrap_or_else(|| std::path::Path::new("."));
    let new_exe = target_dir.join("SonixLink_new.exe");

    // 1. Download new executable using curl
    let status = Command::new("curl.exe")
        .args(["-L", "-o", new_exe.to_str().unwrap(), exe_url])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|e| format!("Failed to download update: {}", e))?;

    if !status.success() || !new_exe.exists() {
        return Err("Update download failed".to_string());
    }

    let cur_str = current_exe.to_str().unwrap();
    let new_str = new_exe.to_str().unwrap();

    // 2. Spawn delayed replacement batch script and exit
    let batch_cmd = format!(
        "timeout /t 1 /nobreak > NUL & move /y \"{}\" \"{}\" & start \"\" \"{}\"",
        new_str, cur_str, cur_str
    );

    Command::new("cmd.exe")
        .args(["/C", &batch_cmd])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Failed to spawn updater process: {}", e))?;

    // 3. Exit current process to free the file lock
    std::process::exit(0);
}
