use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_host")]
    pub last_host: String,
    #[serde(default = "default_port")]
    pub last_port: u16,
    #[serde(default = "default_accent_pref")]
    pub accent_preference: String,
    #[serde(default)]
    pub device_accent: String,
    #[serde(default = "default_fav_pref")]
    pub fav_mark_preference: String,
    #[serde(default = "default_fav_device")]
    pub device_fav_mark: String,
    #[serde(default = "default_vol_step")]
    pub volume_step: u8,
    #[serde(default)]
    pub db_stamp: String,
    #[serde(default = "default_col_widths")]
    pub col_widths: [f32; 3],
}

pub fn default_col_widths() -> [f32; 3] {
    [280.0, 180.0, 180.0]
}

fn default_host() -> String {
    "192.168.0.17".to_string()
}
fn default_port() -> u16 {
    7800
}
fn default_accent_pref() -> String {
    "device".to_string()
}
fn default_fav_pref() -> String {
    "device".to_string()
}
fn default_fav_device() -> String {
    "heart".to_string()
}
fn default_vol_step() -> u8 {
    3
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            last_host: default_host(),
            last_port: default_port(),
            accent_preference: default_accent_pref(),
            device_accent: String::new(),
            fav_mark_preference: default_fav_pref(),
            device_fav_mark: default_fav_device(),
            volume_step: default_vol_step(),
            db_stamp: String::new(),
            col_widths: default_col_widths(),
        }
    }
}

impl AppSettings {
    pub fn get_file_path() -> PathBuf {
        let current_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));
        let local_path = current_dir.join("settings.json");
        if local_path.exists() {
            return local_path;
        }

        // Check if root workspace has settings.json
        let root_settings = PathBuf::from("settings.json");
        if root_settings.exists() {
            return root_settings;
        }

        local_path
    }

    pub fn load() -> Self {
        let path = Self::get_file_path();
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(settings) = serde_json::from_str(&content) {
                return settings;
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let path = Self::get_file_path();
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }

    pub fn effective_accent(&self) -> &str {
        if self.accent_preference != "device" && !self.accent_preference.is_empty() {
            return &self.accent_preference;
        }
        if !self.device_accent.is_empty() {
            return &self.device_accent;
        }
        "#FF8C00" // Default TempoTec V1 theme accent
    }

    pub fn effective_fav_mark(&self) -> &str {
        match self.fav_mark_preference.as_str() {
            "star" => "star",
            "heart" => "heart",
            _ => {
                if self.device_fav_mark.eq_ignore_ascii_case("star") {
                    "star"
                } else {
                    "heart"
                }
            }
        }
    }
}
