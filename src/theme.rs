use eframe::egui::Color32;

pub const BG_BASE: Color32 = Color32::from_rgb(13, 14, 17);
pub const BG_PANEL: Color32 = Color32::from_rgb(19, 20, 23);
pub const BG_CARD: Color32 = Color32::from_rgb(26, 27, 31);
pub const BG_CARD_HOVER: Color32 = Color32::from_rgb(34, 35, 41);
pub const BG_FIELD: Color32 = Color32::from_rgb(28, 29, 34);
pub const BORDER: Color32 = Color32::from_rgb(38, 39, 45);
pub const BORDER_LIGHT: Color32 = Color32::from_rgb(52, 53, 61);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(245, 246, 248);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(155, 157, 166);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(98, 100, 109);
pub const STATUS_GREEN: Color32 = Color32::from_rgb(46, 204, 113);
pub const STATUS_RED: Color32 = Color32::from_rgb(231, 76, 60);
pub const ROW_EVEN: Color32 = Color32::from_rgb(15, 16, 19);
pub const ROW_ODD: Color32 = Color32::from_rgb(18, 19, 23);
pub const DEFAULT_ACCENT: Color32 = Color32::from_rgb(255, 140, 0); // #FF8C00

pub fn parse_hex_color(hex: &str) -> Color32 {
    let clean = hex.trim().trim_start_matches('#');
    if clean.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&clean[0..2], 16),
            u8::from_str_radix(&clean[2..4], 16),
            u8::from_str_radix(&clean[4..6], 16),
        ) {
            return Color32::from_rgb(r, g, b);
        }
    }
    DEFAULT_ACCENT
}

pub struct AccentPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub color: Color32,
}

pub const ACCENT_PRESETS: &[AccentPreset] = &[
    AccentPreset { id: "#3584e4", name: "Blue", color: Color32::from_rgb(0x35, 0x84, 0xE4) },
    AccentPreset { id: "#e01b24", name: "Red", color: Color32::from_rgb(0xE0, 0x1B, 0x24) },
    AccentPreset { id: "#ff7800", name: "Orange", color: Color32::from_rgb(0xFF, 0x78, 0x00) },
    AccentPreset { id: "#e5a50a", name: "Yellow", color: Color32::from_rgb(0xE5, 0xA5, 0x0A) },
    AccentPreset { id: "#99c500", name: "Lime", color: Color32::from_rgb(0x99, 0xC5, 0x00) },
    AccentPreset { id: "#2ec27e", name: "Green", color: Color32::from_rgb(0x2E, 0xC2, 0x7E) },
    AccentPreset { id: "#00b0d6", name: "Cyan", color: Color32::from_rgb(0x00, 0xB0, 0xD6) },
    AccentPreset { id: "#665be0", name: "Indigo", color: Color32::from_rgb(0x66, 0x5B, 0xE0) },
    AccentPreset { id: "#9141ac", name: "Purple", color: Color32::from_rgb(0x91, 0x41, 0xAC) },
    AccentPreset { id: "#d56199", name: "Pink", color: Color32::from_rgb(0xD5, 0x61, 0x99) },
    AccentPreset { id: "#a86530", name: "Brown", color: Color32::from_rgb(0xA8, 0x65, 0x30) },
    AccentPreset { id: "#8b8e94", name: "Slate", color: Color32::from_rgb(0x8B, 0x8E, 0x94) },
    AccentPreset { id: "#ffad2e", name: "Amber", color: Color32::from_rgb(0xFF, 0xAD, 0x2E) },
    AccentPreset { id: "#ffffff", name: "White", color: Color32::from_rgb(0xFF, 0xFF, 0xFF) },
];

pub fn format_duration(seconds: u32) -> String {
    let m = seconds / 60;
    let s = seconds % 60;
    let h = m / 60;
    let rem_m = m % 60;
    if h > 0 {
        format!("{}:{}:{:02}", h, rem_m, s)
    } else {
        format!("{}:{:02}", m, s)
    }
}
