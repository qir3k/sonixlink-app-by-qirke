#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code, unused_variables, non_snake_case)]

mod bt;
mod client;
mod custom_widgets;
mod db;
mod settings;
mod theme;
mod ui;
mod updater;

use eframe::egui::{self, IconData};
use ui::SonixApp;

fn main() -> eframe::Result<()> {
    let icon_data = load_embedded_icon();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 820.0])
        .with_min_inner_size([980.0, 620.0])
        .with_title("SonixLink - Remote Player for Windows");

    if let Some(icon) = icon_data {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "SonixLink",
        native_options,
        Box::new(|cc| Ok(Box::new(SonixApp::new(cc)))),
    )
}

fn load_embedded_icon() -> Option<IconData> {
    const ICON_BYTES: &[u8] = include_bytes!("../icon.png");
    if let Ok(image) = image::load_from_memory(ICON_BYTES) {
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();
        return Some(IconData {
            rgba: rgba.into_raw(),
            width,
            height,
        });
    }
    None
}
