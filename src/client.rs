use crate::bt::{self, Link};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerState {
    #[serde(default)]
    pub state: u8, // 0 stop, 1 play, 2 pause
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default)]
    pub volume: u8,
    #[serde(default = "default_vol_step")]
    pub volume_step: u8,
    #[serde(default)]
    pub position: u32,
    #[serde(default)]
    pub duration: u32,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub album: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub sample_rate: u32,
    #[serde(default)]
    pub bits: u32,
    #[serde(default)]
    pub lossless: bool,
    #[serde(default = "default_battery")]
    pub battery: i8,
    #[serde(default)]
    pub charging: bool,
    #[serde(default)]
    pub favourite: bool,
    pub fav_star: Option<bool>,
    #[serde(default)]
    pub fav_mark: String,
    #[serde(default = "default_accent")]
    pub accent: String,
    #[serde(default)]
    pub mseb: bool,
    #[serde(default)]
    pub eq: bool,
    #[serde(default)]
    pub peq: bool,
    #[serde(default = "default_peq_profile")]
    pub peq_profile: u8,
    #[serde(default)]
    pub queue_position: i32,
    #[serde(default)]
    pub queue_count: i32,
    #[serde(default)]
    pub gain: Option<String>,
    #[serde(default)]
    pub gain_high: Option<bool>,
    #[serde(default)]
    pub high_gain: Option<bool>,
}

fn default_mode() -> String { "normal".to_string() }
fn default_vol_step() -> u8 { 3 }
fn default_battery() -> i8 { -1 }
fn default_accent() -> String { "#FF8C00".to_string() }
fn default_peq_profile() -> u8 { 1 }

impl PlayerState {
    pub fn is_high_gain(&self) -> bool {
        self.gain_high.unwrap_or(false)
            || self.high_gain.unwrap_or(false)
            || self.gain.as_deref() == Some("high")
    }

    pub fn is_playing(&self) -> bool {
        self.state == 1
    }

    pub fn format_badge(&self) -> String {
        if self.sample_rate == 0 {
            return String::new();
        }
        let khz = (self.sample_rate as f32) / 1000.0;
        let ext = if self.path.contains('.') {
            self.path.rsplit('.').next().unwrap_or("FLAC").to_uppercase()
        } else {
            "FLAC".to_string()
        };
        if self.bits > 0 {
            format!("{} • {:.1}kHz • {}bit", ext, khz, self.bits)
        } else {
            format!("{} • {:.1}kHz", ext, khz)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamService {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub available: bool,
    #[serde(default)]
    pub signed_in: bool,
    #[serde(default)]
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamRow {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
    #[serde(default)]
    pub album: String,
    #[serde(default)]
    pub art: String,
    #[serde(default)]
    pub duration: u32,
    #[serde(default)]
    pub playable: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StreamList {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub rows: Vec<StreamRow>,
    #[serde(default)]
    pub error: String,
}

/// Where the player is: Wi-Fi address, or a paired Bluetooth address.
#[derive(Debug, Clone, Default)]
pub struct PlayerTarget {
    pub host: String,
    pub port: u16,
    pub use_bluetooth: bool,
    pub bluetooth_address: String,
}

/// Clones share one target, so the background poller follows a connection change.
#[derive(Debug, Clone)]
pub struct SonixClient {
    target: std::sync::Arc<std::sync::Mutex<PlayerTarget>>,
}

impl SonixClient {
    pub fn new(host: String, port: u16) -> Self {
        Self {
            target: std::sync::Arc::new(std::sync::Mutex::new(PlayerTarget {
                host,
                port,
                use_bluetooth: false,
                bluetooth_address: String::new(),
            })),
        }
    }

    pub fn target(&self) -> PlayerTarget {
        self.target.lock().map(|t| t.clone()).unwrap_or_default()
    }

    pub fn set_wifi_target(&self, host: &str, port: u16) {
        if let Ok(mut t) = self.target.lock() {
            t.host = host.to_string();
            t.port = port;
            t.use_bluetooth = false;
        }
    }

    pub fn set_bluetooth_target(&self, address: &str) {
        if let Ok(mut t) = self.target.lock() {
            t.bluetooth_address = address.trim().to_string();
            t.use_bluetooth = true;
        }
    }

    /// Open a link to the player: RFCOMM when Bluetooth is selected, TCP otherwise.
    fn open(&self, timeout_ms: u64) -> Result<Link, String> {
        let target = self.target();
        if target.use_bluetooth {
            let address = bt::parse_address(&target.bluetooth_address)
                .ok_or_else(|| "Invalid Bluetooth address".to_string())?;
            let sock = bt::Rfcomm::connect(address, bt::RFCOMM_CHANNEL, timeout_ms as u32)
                .map_err(|e| format!("Bluetooth connect error: {}", e))?;
            return Ok(Link::Bluetooth(sock));
        }

        let addr_str = format!("{}:{}", target.host, target.port);
        let socket_addr: SocketAddr = addr_str
            .parse()
            .map_err(|e| format!("Invalid address: {}", e))?;
        let stream = TcpStream::connect_timeout(&socket_addr, Duration::from_millis(timeout_ms))
            .map_err(|e| format!("Connect error: {}", e))?;
        stream
            .set_read_timeout(Some(Duration::from_millis(timeout_ms)))
            .map_err(|e| format!("Timeout error: {}", e))?;
        stream
            .set_write_timeout(Some(Duration::from_millis(timeout_ms)))
            .map_err(|e| format!("Timeout error: {}", e))?;
        Ok(Link::Tcp(stream))
    }

    /// Read a whole answer. Over RFCOMM the player keeps the link open, so stop at
    /// Content-Length instead of waiting for the socket to close.
    fn read_response(link: &mut Link) -> Vec<u8> {
        let mut response: Vec<u8> = Vec::with_capacity(8192);
        let mut buf = [0u8; 8192];
        let mut complete_at: Option<usize> = None;

        loop {
            if complete_at.is_none() {
                if let Some(head_end) = response.windows(4).position(|w| w == b"\r\n\r\n") {
                    complete_at = match content_length(&response[..head_end]) {
                        Some(len) => Some(head_end + 4 + len),
                        None => Some(usize::MAX), // no length: read until close or timeout
                    };
                }
            }
            if let Some(total) = complete_at {
                if total != usize::MAX && response.len() >= total {
                    break;
                }
            }
            match link.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => response.extend_from_slice(&buf[..n]),
                Err(_) => break,
            }
        }
        response
    }

    fn raw_http_get(&self, path: &str, timeout_ms: u64) -> Result<Vec<u8>, String> {
        let mut link = self.open(timeout_ms)?;

        let request = format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: SonixLink-Rust/1.0\r\nConnection: close\r\n\r\n",
            path,
            self.target().host
        );
        link.write_all(request.as_bytes())
            .map_err(|e| format!("Write error: {}", e))?;

        let response = Self::read_response(&mut link);

        // Parse headers vs body
        let header_end = response
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .ok_or_else(|| "Malformed HTTP response".to_string())?;

        Ok(response[header_end + 4..].to_vec())
    }

    fn raw_http_post(&self, path: &str, body: &[u8], timeout_ms: u64) -> Result<Vec<u8>, String> {
        let mut link = self.open(timeout_ms)?;

        let request = format!(
            "POST {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: SonixLink-Rust/1.0\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            path,
            self.target().host,
            body.len()
        );
        link.write_all(request.as_bytes()).map_err(|e| format!("Write error: {}", e))?;
        link.write_all(body).map_err(|e| format!("Write error: {}", e))?;

        Ok(Self::read_response(&mut link))
    }

    pub fn selection(&self, action: &str, paths: &[&str], playlist: &str) {
        if paths.is_empty() { return; }
        let mut path_str = format!("/api/command?do={}", urlencoding(action));
        if !playlist.is_empty() {
            path_str.push_str("&value=");
            path_str.push_str(&urlencoding(playlist));
        }
        let body = paths.join("\n").into_bytes();
        let _ = self.raw_http_post(&path_str, &body, 4000);
    }

    pub fn queue_next(&self, paths: &[&str]) {
        self.selection("queue_next", paths, "");
    }

    pub fn set_gain(&self, high: bool) {
        self.command("gain", &[("value", if high { "high" } else { "low" })]);
    }

    pub fn toggle_gain(&self) {
        self.command("gain", &[("value", "toggle")]);
    }

    pub fn save_playlist(&self, name: &str, paths: &[&str]) {
        self.selection("playlist_add", paths, name);
    }

    pub fn get_state(&self) -> Option<PlayerState> {
        let body = self.raw_http_get("/api/state", 900).ok()?;
        serde_json::from_slice(&body).ok()
    }

    pub fn command(&self, do_cmd: &str, params: &[(&str, &str)]) {
        let mut query = format!("/api/command?do={}", do_cmd);
        for (k, v) in params {
            query.push('&');
            query.push_str(k);
            query.push('=');
            query.push_str(&urlencoding(v));
        }
        let _ = self.raw_http_get(&query, 900);
    }

    pub fn toggle_play(&self) {
        self.command("toggle", &[]);
    }
    pub fn prev(&self) {
        self.command("prev", &[]);
    }
    pub fn next(&self) {
        self.command("next", &[]);
    }
    pub fn seek(&self, seconds: u32) {
        let s = seconds.to_string();
        self.command("seek", &[("position", &s)]);
    }
    pub fn set_volume(&self, percent: u8) {
        let v = percent.to_string();
        self.command("volume", &[("value", &v)]);
    }
    pub fn set_volume_step(&self, step: u8) {
        let s = step.to_string();
        self.command("volume_step", &[("value", &s)]);
    }
    pub fn toggle_favorite(&self, path: &str) {
        self.command("favourite", &[("path", path)]);
    }
    pub fn set_mseb(&self, enabled: bool) {
        self.command("mseb", &[("value", if enabled { "1" } else { "0" })]);
    }
    pub fn set_eq(&self, enabled: bool) {
        self.command("eq", &[("value", if enabled { "1" } else { "0" })]);
    }
    pub fn set_peq(&self, enabled: bool) {
        self.command("peq", &[("value", if enabled { "1" } else { "0" })]);
    }
    pub fn set_peq_profile(&self, profile: u8) {
        let p = profile.to_string();
        self.command("peq_profile", &[("value", &p)]);
    }

    pub fn get_stream_services(&self) -> Vec<StreamService> {
        #[derive(Deserialize)]
        struct SrvResp {
            #[serde(default)]
            services: Vec<StreamService>,
        }
        if let Ok(body) = self.raw_http_get("/api/stream/services", 1500) {
            if let Ok(resp) = serde_json::from_slice::<SrvResp>(&body) {
                return resp.services;
            }
        }
        // Fallback: If player returns empty or older firmware, supply the standard SonixPlayer services
        vec![
            StreamService { id: "youtube".into(), name: "YouTube Music".into(), available: true, signed_in: true, detail: String::new() },
            StreamService { id: "tidal".into(), name: "Tidal".into(), available: true, signed_in: true, detail: String::new() },
            StreamService { id: "qobuz".into(), name: "Qobuz".into(), available: true, signed_in: true, detail: String::new() },
            StreamService { id: "podcasts".into(), name: "Podcasts".into(), available: true, signed_in: true, detail: String::new() },
            StreamService { id: "radio".into(), name: "Internet radio".into(), available: true, signed_in: true, detail: String::new() },
        ]
    }

    pub fn get_stream_list(&self, service: &str, kind: &str, id: &str, q: &str) -> StreamList {
        let path = format!(
            "/api/stream/list?service={}&kind={}&id={}&q={}",
            urlencoding(service),
            urlencoding(kind),
            urlencoding(id),
            urlencoding(q)
        );
        if let Ok(body) = self.raw_http_get(&path, 3000) {
            if let Ok(list) = serde_json::from_slice::<StreamList>(&body) {
                return list;
            }
        }
        StreamList::default()
    }

    pub fn stream_play(&self, service: &str, index: usize) {
        let path = format!("/api/stream/play?service={}&index={}", urlencoding(service), index);
        let _ = self.raw_http_get(&path, 1000);
    }

    pub fn get_art(&self, path: &str) -> Option<Vec<u8>> {
        let req = format!("/api/art?path={}&max=0", urlencoding(path));
        self.raw_http_get(&req, 2500).ok()
    }

    pub fn download_db(&self, dest_path: &std::path::Path) -> Result<usize, String> {
        let mut link = self.open(15000)?;

        let request = format!(
            "GET /api/db HTTP/1.1\r\nHost: {}\r\nUser-Agent: SonixLink-Rust/1.0\r\nConnection: close\r\n\r\n",
            self.target().host
        );
        link.write_all(request.as_bytes())
            .map_err(|e| format!("Write error: {}", e))?;

        let response = Self::read_response(&mut link);

        let header_end = response
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .ok_or_else(|| "Malformed HTTP response".to_string())?;

        let body = &response[header_end + 4..];
        if body.len() < 100 || !body.starts_with(b"SQLite format 3") {
            return Err("Invalid database received from player".to_string());
        }

        if let Some(parent) = dest_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(dest_path, body).map_err(|e| format!("Failed to write db file: {}", e))?;
        Ok(body.len())
    }
}

fn content_length(headers: &[u8]) -> Option<usize> {
    let text = String::from_utf8_lossy(headers);
    text.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.trim().eq_ignore_ascii_case("content-length") {
            value.trim().parse().ok()
        } else {
            None
        }
    })
}

fn urlencoding(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
