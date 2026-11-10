use crate::client::{PlayerState, SonixClient, StreamList, StreamService};
use crate::custom_widgets::*;
use crate::db::{LibraryDb, SimpleItem, TrackItem};
use crate::settings::AppSettings;
use crate::theme::*;
use eframe::egui::{self, Align, Color32, Frame, Layout, ScrollArea, Stroke, TextureHandle, Ui, Vec2};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct StreamPage {
    pub service_id: String,
    pub kind: String,
    pub id: String,
    pub title: String,
}

pub fn col_resize_separator(ui: &mut Ui, width: &mut f32) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(8.0, 16.0), egui::Sense::drag());
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    if resp.dragged() {
        *width = (*width + resp.drag_delta().x).clamp(60.0, 700.0);
    }
    let center_x = rect.center().x;
    let stroke_color = if resp.hovered() || resp.dragged() {
        Color32::from_rgb(220, 220, 225)
    } else {
        BORDER_LIGHT
    };
    ui.painter().line_segment(
        [
            egui::pos2(center_x, rect.top() + 2.0),
            egui::pos2(center_x, rect.bottom() - 2.0),
        ],
        Stroke::new(1.0, stroke_color),
    );
}

pub struct SonixApp {
    pub client: SonixClient,
    pub settings: AppSettings,
    pub db: LibraryDb,
    pub current_state: Option<PlayerState>,
    pub current_nav: String,
    pub drilldown_category: Option<(String, String)>,
    pub search_query: String,

    pub tracks: Vec<TrackItem>,
    pub simple_items: Vec<SimpleItem>,
    pub stream_services: Vec<StreamService>,
    pub active_stream_service: Option<String>,
    pub stream_list: StreamList,
    pub stream_stack: Vec<StreamPage>,

    pub seek_val: f32,
    pub is_seeking: bool,
    pub volume_val: f32,

    pub show_settings: bool,
    pub show_dsp: bool,
    pub show_devices: bool,
    pub manual_host_input: String,

    pub is_syncing_db: bool,
    pub sync_db_rx: Receiver<bool>,
    pub sync_db_tx: Sender<bool>,

    pub update_info: Option<crate::updater::ReleaseInfo>,
    pub is_checking_update: bool,
    pub is_downloading_update: bool,
    pub update_status_msg: Option<String>,
    pub update_rx: Receiver<Result<crate::updater::ReleaseInfo, String>>,
    pub update_tx: Sender<Result<crate::updater::ReleaseInfo, String>>,

    pub last_poll: Instant,
    pub state_rx: Receiver<PlayerState>,
    pub state_tx: Sender<PlayerState>,

    pub art_texture: Option<TextureHandle>,
    pub loaded_art_path: String,
    pub art_rx: Receiver<(String, Vec<u8>)>,
    pub art_tx: Sender<(String, Vec<u8>)>,
}

impl SonixApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // High-end Dark Monochrome Visuals
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = BG_PANEL;
        visuals.window_fill = BG_CARD;
        visuals.faint_bg_color = BG_BASE;
        visuals.extreme_bg_color = BG_BASE;
        cc.egui_ctx.set_visuals(visuals);

        let settings = AppSettings::load();
        let client = SonixClient::new(settings.last_host.clone(), settings.last_port);
        let db = LibraryDb::new();

        let (state_tx, state_rx) = channel();
        let (art_tx, art_rx) = channel();
        let (sync_db_tx, sync_db_rx) = channel();
        let (update_tx, update_rx) = channel();

        // Background poller thread
        let client_clone = client.clone();
        let tx_clone = state_tx.clone();
        std::thread::spawn(move || loop {
            if let Some(state) = client_clone.get_state() {
                let _ = tx_clone.send(state);
            }
            std::thread::sleep(Duration::from_millis(900));
        });

        let mut app = Self {
            client,
            settings,
            db,
            current_state: None,
            current_nav: "tracks".to_string(),
            drilldown_category: None,
            search_query: String::new(),

            tracks: Vec::new(),
            simple_items: Vec::new(),
            stream_services: Vec::new(),
            active_stream_service: None,
            stream_list: StreamList::default(),
            stream_stack: Vec::new(),

            seek_val: 0.0,
            is_seeking: false,
            volume_val: 0.0,

            show_settings: false,
            show_dsp: false,
            show_devices: false,
            manual_host_input: "192.168.0.17".to_string(),

            is_syncing_db: false,
            sync_db_rx,
            sync_db_tx,

            update_info: None,
            is_checking_update: false,
            is_downloading_update: false,
            update_status_msg: None,
            update_rx,
            update_tx,

            last_poll: Instant::now(),
            state_rx,
            state_tx,

            art_texture: None,
            loaded_art_path: String::new(),
            art_rx,
            art_tx,
        };

        app.load_view_data();
        if app.tracks.is_empty() {
            app.trigger_sync_db();
        }
        app.trigger_check_update();
        app
    }

    pub fn trigger_check_update(&mut self) {
        if self.is_checking_update || self.is_downloading_update {
            return;
        }
        self.is_checking_update = true;
        self.update_status_msg = Some("Checking for updates...".to_string());
        let tx = self.update_tx.clone();
        std::thread::spawn(move || {
            let res = crate::updater::check_for_updates();
            let _ = tx.send(res);
        });
    }

    pub fn trigger_install_update(&mut self, url: String) {
        if self.is_downloading_update {
            return;
        }
        self.is_downloading_update = true;
        self.update_status_msg = Some("Downloading update and restarting...".to_string());
        std::thread::spawn(move || {
            let _ = crate::updater::download_and_install_update(&url);
        });
    }

    pub fn trigger_sync_db(&mut self) {
        if self.is_syncing_db {
            return;
        }
        self.is_syncing_db = true;
        let client = self.client.clone();
        let tx = self.sync_db_tx.clone();
        let db_path = crate::db::LibraryDb::find_db_path();
        std::thread::spawn(move || {
            let res = client.download_db(&db_path).is_ok();
            let _ = tx.send(res);
        });
    }

    pub fn effective_accent_color(&self) -> Color32 {
        parse_hex_color(self.settings.effective_accent())
    }

    pub fn load_view_data(&mut self) {
        if let Some((ref cat, ref name)) = self.drilldown_category {
            match cat.as_str() {
                "albums" => self.tracks = self.db.get_tracks_by_album(name),
                "artists" => self.tracks = self.db.get_tracks_by_artist(name),
                "genres" => self.tracks = self.db.get_tracks_by_genre(name),
                "years" => self.tracks = self.db.get_tracks_by_year(name),
                "playlists" => self.tracks = self.db.get_tracks_by_playlist(name),
                _ => self.tracks = self.db.get_all_tracks(),
            }
            return;
        }

        match self.current_nav.as_str() {
            "tracks" => self.tracks = self.db.get_all_tracks(),
            "favourites" => self.tracks = self.db.get_favourites(),
            "playlists" => self.simple_items = self.db.get_playlists(),
            "smartmix" => {
                let seed = self.current_state.as_ref().map(|s| s.path.as_str()).unwrap_or("");
                self.tracks = self.db.generate_smartmix(seed);
            }
            "albums" => self.simple_items = self.db.get_albums(),
            "artists" => self.simple_items = self.db.get_artists(),
            "genres" => self.simple_items = self.db.get_genres(),
            "years" => self.simple_items = self.db.get_years(),
            "streaming" => {
                let client = self.client.clone();
                self.stream_services = client.get_stream_services();
            }
            _ => self.tracks = self.db.get_all_tracks(),
        }
    }

    fn open_stream_service(&mut self, service_id: &str) {
        self.active_stream_service = Some(service_id.to_string());
        self.stream_stack.clear();
        let client = self.client.clone();
        self.stream_list = client.get_stream_list(service_id, "menu", "", "");
    }

    fn check_art_download(&mut self, path: &str) {
        if path.is_empty() || path == self.loaded_art_path {
            return;
        }
        self.loaded_art_path = path.to_string();
        self.art_texture = None;

        let client = self.client.clone();
        let tx = self.art_tx.clone();
        let p = path.to_string();
        std::thread::spawn(move || {
            if let Some(bytes) = client.get_art(&p) {
                let _ = tx.send((p, bytes));
            }
        });
    }

    fn draw_vinyl_placeholder(&self, ui: &mut Ui, size: f32) {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
        let center = rect.center();
        let painter = ui.painter();

        // Matte dark card base
        painter.rect_filled(rect, 8.0, BG_CARD);
        painter.rect_stroke(rect, 8.0, Stroke::new(1.0, BORDER), egui::StrokeKind::Inside);

        // Outer Vinyl Disc
        painter.circle_filled(center, size * 0.44, Color32::from_rgb(16, 17, 22));
        painter.circle_stroke(center, size * 0.44, Stroke::new(1.2, Color32::from_rgb(32, 35, 45)));

        // Concentric Vinyl Grooves
        for r_pct in &[0.38, 0.31, 0.24, 0.17] {
            painter.circle_stroke(center, size * r_pct, Stroke::new(1.0, Color32::from_rgb(24, 26, 34)));
        }

        // Center Label Disc
        let label_accent = self.effective_accent_color();
        painter.circle_filled(center, size * 0.12, Color32::from_rgb(22, 24, 32));
        painter.circle_stroke(center, size * 0.12, Stroke::new(1.5, label_accent));
        // Center spindle hole
        painter.circle_filled(center, size * 0.03, BG_BASE);
    }
}

impl eframe::App for SonixApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Process database sync completion
        while let Ok(success) = self.sync_db_rx.try_recv() {
            self.is_syncing_db = false;
            if success {
                self.db = LibraryDb::new();
                self.load_view_data();
            }
        }

        // Process update check completion
        while let Ok(res) = self.update_rx.try_recv() {
            self.is_checking_update = false;
            match res {
                Ok(info) => {
                    if info.is_newer {
                        self.update_status_msg = Some(format!("SonixLink {} is available", info.tag));
                        self.update_info = Some(info);
                    } else {
                        self.update_status_msg = Some(format!("You have the latest version (v{})", crate::updater::current_version()));
                    }
                }
                Err(e) => {
                    self.update_status_msg = Some(format!("Update check failed: {}", e));
                }
            }
        }

        // 1. Process incoming state from background thread
        let mut first_connect = false;
        while let Ok(state) = self.state_rx.try_recv() {
            if self.current_state.is_none() {
                first_connect = true;
            }
            if let Some(star) = state.fav_star {
                self.settings.device_fav_mark = if star { "star".into() } else { "heart".into() };
            }
            if !state.fav_mark.is_empty() {
                self.settings.device_fav_mark = state.fav_mark.clone();
            }
            if !state.accent.is_empty() {
                self.settings.device_accent = state.accent.clone();
            }

            if !self.is_seeking {
                self.seek_val = state.position as f32;
            }
            self.volume_val = state.volume as f32;

            self.check_art_download(&state.path);
            self.current_state = Some(state);
        }

        if first_connect {
            self.trigger_sync_db();
        }

        // 2. Process incoming artwork image
        while let Ok((path, bytes)) = self.art_rx.try_recv() {
            if path == self.loaded_art_path {
                if let Ok(img) = image::load_from_memory(&bytes) {
                    let rgba = img.to_rgba8();
                    let (w, h) = rgba.dimensions();
                    let color_img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                    self.art_texture = Some(ctx.load_texture("album_art", color_img, egui::TextureOptions::LINEAR));
                }
            }
        }

        let accent = self.effective_accent_color();

        // 3. Three-Panel Layout: Pure Dark Monochrome with Accent Highlight
        egui::Panel::left("left_sidebar")
            .exact_size(220.0)
            .resizable(false)
            .frame(Frame::new().fill(BG_PANEL).inner_margin(12.0))
            .show(ui, |ui| {
                self.render_left_sidebar(ui, accent);
            });

        egui::Panel::right("right_panel")
            .exact_size(280.0)
            .resizable(false)
            .frame(Frame::new().fill(BG_PANEL).inner_margin(14.0))
            .show(ui, |ui| {
                self.render_right_panel(ui, accent);
            });

        egui::CentralPanel::default()
            .frame(Frame::new().fill(BG_BASE).inner_margin(Vec2::new(16.0, 14.0)))
            .show(ui, |ui| {
                self.render_center_panel(ui, accent);
            });

        // Modals
        if self.show_settings {
            self.render_settings_modal(&ctx, accent);
        }
        if self.show_dsp {
            self.render_dsp_modal(&ctx, accent);
        }
        if self.show_devices {
            self.render_devices_modal(&ctx, accent);
        }

        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

impl SonixApp {
    // -------------------------------------------------------------------------
    // LEFT SIDEBAR (Clean Monochrome Typography)
    // -------------------------------------------------------------------------
    fn render_left_sidebar(&mut self, ui: &mut Ui, accent: Color32) {
        // 1. Sleek Brand Header (No cartoon emojis)
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(3.5, 24.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 1.5, accent);
            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("SONIXLINK").color(TEXT_PRIMARY).size(13.5).strong());
                ui.label(egui::RichText::new("TempoTec Variations V1").color(TEXT_MUTED).size(9.0));
            });
        });

        ui.add_space(10.0);

        // 2. Connection Card (Clean pro-audio design)
        Frame::new()
            .fill(BG_CARD)
            .stroke(Stroke::new(1.0, BORDER))
            .corner_radius(6.0)
            .inner_margin(Vec2::new(10.0, 8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (dot_color, dev_name) = if self.current_state.is_some() {
                        (STATUS_GREEN, "TempoTec V1")
                    } else {
                        (STATUS_RED, "Disconnected")
                    };
                    ui.label(egui::RichText::new("●").color(dot_color).size(10.0));
                    ui.label(egui::RichText::new(dev_name).color(TEXT_PRIMARY).size(11.0).strong());

                    if let Some(ref s) = self.current_state {
                        if s.battery >= 0 {
                            let charge_txt = if s.charging { format!("{}% ⚡", s.battery) } else { format!("{}%", s.battery) };
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(egui::RichText::new(charge_txt).color(TEXT_SECONDARY).size(10.0));
                            });
                        }
                    }
                });

                ui.label(egui::RichText::new(format!("{}:{}", self.client.host, self.client.port)).color(TEXT_MUTED).size(9.5));

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new("Devices").corner_radius(4.0)).clicked() {
                        self.show_devices = true;
                    }
                    let sync_lbl = if self.is_syncing_db { "Syncing..." } else { "Sync DB" };
                    if ui.add(egui::Button::new(sync_lbl).corner_radius(4.0)).clicked() {
                        self.trigger_sync_db();
                    }
                });
            });

        ui.add_space(12.0);

        // 3. Navigation Links (Minimalist Typography, No cartoon icons)
        let bottom_height = 42.0;
        let nav_height = (ui.available_height() - bottom_height).max(100.0);

        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), nav_height),
            Layout::top_down(Align::Min),
            |ui| {
                ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.label(egui::RichText::new("LIBRARY").color(TEXT_MUTED).size(9.0).strong());
                    ui.add_space(2.0);
                    self.nav_button(ui, "tracks", "All Tracks", accent);
                    self.nav_button(ui, "albums", "Albums", accent);
                    self.nav_button(ui, "artists", "Artists", accent);
                    self.nav_button(ui, "genres", "Genres", accent);
                    self.nav_button(ui, "years", "Years", accent);

                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("COLLECTION").color(TEXT_MUTED).size(9.0).strong());
                    ui.add_space(2.0);
                    self.nav_button(ui, "favourites", "Favourites", accent);
                    self.nav_button(ui, "playlists", "Playlists", accent);
                    self.nav_button(ui, "smartmix", "✦ Smart Mix", accent);

                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("ONLINE").color(TEXT_MUTED).size(9.0).strong());
                    ui.add_space(2.0);
                    self.nav_button(ui, "streaming", "Streaming", accent);
                });
            },
        );

        ui.add_space(8.0);

        // 4. Bottom Pinned Settings Button
        if ui.add(egui::Button::new(egui::RichText::new("Settings").size(11.0).strong()).min_size(Vec2::new(ui.available_width(), 30.0)).corner_radius(5.0)).clicked() {
            self.show_settings = true;
        }
    }

    fn nav_button(&mut self, ui: &mut Ui, key: &str, label: &str, accent: Color32) {
        let is_active = self.current_nav == key;
        let text = egui::RichText::new(label).size(12.0);
        let btn = if is_active {
            egui::Button::new(text.color(Color32::WHITE).strong())
                .fill(BG_CARD)
                .stroke(Stroke::new(1.0, accent))
                .corner_radius(5.0)
                .min_size(Vec2::new(ui.available_width(), 28.0))
        } else {
            egui::Button::new(text.color(TEXT_SECONDARY))
                .fill(Color32::TRANSPARENT)
                .corner_radius(5.0)
                .min_size(Vec2::new(ui.available_width(), 28.0))
        };

        if ui.add(btn).clicked() {
            self.drilldown_category = None;
            self.search_query.clear();
            self.current_nav = key.to_string();
            self.load_view_data();
        }
    }

    // -------------------------------------------------------------------------
    // CENTER PANEL (Main View)
    // -------------------------------------------------------------------------
    fn render_center_panel(&mut self, ui: &mut Ui, accent: Color32) {
        // Drill-down header if viewing songs for a specific Album, Artist, Genre, Year, or Playlist
        if let Some((ref cat, ref name)) = self.drilldown_category.clone() {
            let mut back_clicked = false;
            let cat_label = match cat.as_str() {
                "albums" => "Albums",
                "artists" => "Artists",
                "genres" => "Genres",
                "years" => "Years",
                "playlists" => "Playlists",
                _ => "Library",
            };

            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(format!("‹ Back to {}", cat_label)).corner_radius(4.0)).clicked() {
                    back_clicked = true;
                }
                ui.add_space(6.0);
                ui.label(egui::RichText::new(name).color(TEXT_PRIMARY).size(16.0).strong());
                ui.label(egui::RichText::new(format!("{} tracks", self.tracks.len())).color(TEXT_MUTED).size(11.0));

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);
                    if header_pill_button(ui, "▶", "Play All", true, accent).clicked() {
                        if let Some(first) = self.tracks.first() {
                            if cat == "playlists" {
                                self.client.command("play_path", &[("path", &first.path), ("list", "playlist"), ("value", name)]);
                            } else {
                                self.client.command("play_path", &[("path", &first.path)]);
                            }
                        }
                    }
                    if header_pill_button(ui, "⇄", "Shuffle", false, accent).clicked() {
                        self.client.command("mode", &[("value", "shuffle")]);
                        if let Some(first) = self.tracks.first() {
                            if cat == "playlists" {
                                self.client.command("play_path", &[("path", &first.path), ("list", "playlist"), ("value", name)]);
                            } else {
                                self.client.command("play_path", &[("path", &first.path)]);
                            }
                        }
                    }
                });
            });

            if back_clicked {
                self.drilldown_category = None;
                self.load_view_data();
                return;
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("Search in category...")
                        .desired_width(ui.available_width() - 8.0)
                );
            });
            ui.add_space(10.0);
            self.render_tracks_table(ui, accent);
            return;
        }

        // 1. Header Bar
        ui.horizontal(|ui| {
            let title = match self.current_nav.as_str() {
                "tracks" => "All Tracks",
                "favourites" => "Favourites",
                "playlists" => "Playlists",
                "smartmix" => "✦ Smart Mix",
                "albums" => "Albums",
                "artists" => "Artists",
                "genres" => "Genres",
                "years" => "Years",
                "streaming" => "Streaming Services",
                _ => "Library",
            };
            ui.label(egui::RichText::new(title).color(TEXT_PRIMARY).size(17.0).strong());

            let count_str = match self.current_nav.as_str() {
                "tracks" => format!("{} tracks", self.tracks.len()),
                "favourites" => format!("{} favourites", self.tracks.len()),
                "playlists" => format!("{} playlists", self.simple_items.len()),
                "smartmix" => format!("{} tracks in mix", self.tracks.len()),
                "albums" => format!("{} albums", self.simple_items.len()),
                "artists" => format!("{} artists", self.simple_items.len()),
                "streaming" => format!("{} services", self.stream_services.len()),
                _ => format!("{} items", self.simple_items.len()),
            };
            ui.label(egui::RichText::new(count_str).color(TEXT_MUTED).size(11.0));

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);
                if header_pill_button(ui, "▶", if self.current_nav == "smartmix" { "Play Mix" } else { "Play All" }, true, accent).clicked() {
                    if let Some(first) = self.tracks.first() {
                        self.client.command("play_path", &[("path", &first.path)]);
                    }
                }
                if header_pill_button(ui, "⇄", "Shuffle", false, accent).clicked() {
                    self.client.command("mode", &[("value", "shuffle")]);
                    if let Some(first) = self.tracks.first() {
                        self.client.command("play_path", &[("path", &first.path)]);
                    }
                }
                if self.current_nav == "smartmix" {
                    if header_pill_button(ui, "＋", "Save Playlist", false, accent).clicked() {
                        let paths: Vec<&str> = self.tracks.iter().map(|t| t.path.as_str()).collect();
                        let seed_title = self.tracks.first().map(|t| t.title.as_str()).unwrap_or("Mix");
                        let pl_name = format!("Smart Mix - {}", seed_title);
                        self.client.save_playlist(&pl_name, &paths);
                    }
                    if header_pill_button(ui, "⟳", "Re-mix", false, accent).clicked() {
                        let seed = self.current_state.as_ref().map(|s| s.path.as_str()).unwrap_or("");
                        self.tracks = self.db.generate_smartmix(seed);
                    }
                }
            });
        });

        // 2. Minimalist Search Field
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("Search tracks, artists, albums...")
                    .desired_width(ui.available_width() - 8.0)
            );
        });

        ui.add_space(10.0);

        // 3. Body View: Streaming vs Table
        if self.current_nav == "streaming" {
            self.render_streaming_view(ui, accent);
        } else if self.current_nav == "tracks" || self.current_nav == "favourites" || self.current_nav == "smartmix" {
            self.render_tracks_table(ui, accent);
        } else {
            self.render_simple_table(ui, accent);
        }
    }

    fn render_tracks_table(&mut self, ui: &mut Ui, accent: Color32) {
        let avail_w = (ui.available_width() - 12.0).max(320.0);
        let w_num = 34.0;
        let w_time = 52.0;

        let mut w_title = self.settings.col_widths[0];
        let mut w_artist = self.settings.col_widths[1];
        let mut w_album = self.settings.col_widths[2];

        // Table Header with Resizable Dividers
        Frame::new().fill(BG_PANEL).inner_margin(Vec2::new(8.0, 6.0)).corner_radius(4.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_sized([w_num, 16.0], egui::Label::new(egui::RichText::new("#").size(10.0).strong().color(TEXT_MUTED)).selectable(false));

                ui.add_sized([w_title, 16.0], egui::Label::new(egui::RichText::new("TITLE").size(10.0).strong().color(TEXT_MUTED)).selectable(false));
                col_resize_separator(ui, &mut w_title);

                ui.add_sized([w_artist, 16.0], egui::Label::new(egui::RichText::new("ARTIST").size(10.0).strong().color(TEXT_MUTED)).selectable(false));
                col_resize_separator(ui, &mut w_artist);

                ui.add_sized([w_album, 16.0], egui::Label::new(egui::RichText::new("ALBUM").size(10.0).strong().color(TEXT_MUTED)).selectable(false));
                col_resize_separator(ui, &mut w_album);

                ui.add_sized([w_time, 16.0], egui::Label::new(egui::RichText::new("TIME").size(10.0).strong().color(TEXT_MUTED)).selectable(false));
            });
        });

        if (w_title - self.settings.col_widths[0]).abs() > 0.5
            || (w_artist - self.settings.col_widths[1]).abs() > 0.5
            || (w_album - self.settings.col_widths[2]).abs() > 0.5
        {
            self.settings.col_widths = [w_title, w_artist, w_album];
            self.settings.save();
        }

        ui.add_space(4.0);

        let mut smartmix_seed = None;
        let mut queue_track = None;
        let mut play_track = None;
        let query = self.search_query.to_lowercase();

        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for (idx, track) in self.tracks.iter().enumerate() {
                if !query.is_empty() && !track.title.to_lowercase().contains(&query) && !track.artist.to_lowercase().contains(&query) {
                    continue;
                }

                let row_bg = if idx % 2 == 0 { ROW_EVEN } else { ROW_ODD };
                let is_current = self.current_state.as_ref().map(|s| s.path == track.path).unwrap_or(false);

                let fill = if is_current {
                    Color32::from_rgba_premultiplied(accent.r(), accent.g(), accent.b(), 40)
                } else {
                    row_bg
                };

                let frame_resp = Frame::new()
                    .fill(fill)
                    .corner_radius(4.0)
                    .inner_margin(Vec2::new(8.0, 4.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let num_str = if track.track_num > 0 { format!("{:02}", track.track_num) } else { format!("{:02}", idx + 1) };
                            let num_color = if is_current { accent } else { TEXT_MUTED };
                            ui.add_sized([w_num, 18.0], egui::Label::new(egui::RichText::new(num_str).size(11.0).color(num_color)).selectable(false));

                            let title_color = if is_current { accent } else { TEXT_PRIMARY };
                            ui.add_sized([w_title, 18.0], egui::Label::new(egui::RichText::new(&track.title).size(12.0).strong().color(title_color)).truncate().selectable(false));

                            ui.add_space(8.0);
                            ui.add_sized([w_artist, 18.0], egui::Label::new(egui::RichText::new(&track.artist).size(11.0).color(TEXT_SECONDARY)).truncate().selectable(false));

                            ui.add_space(8.0);
                            ui.add_sized([w_album, 18.0], egui::Label::new(egui::RichText::new(&track.album).size(11.0).color(TEXT_MUTED)).truncate().selectable(false));

                            let dur_str = if track.duration > 0 { format_duration(track.duration) } else { "—".to_string() };
                            ui.add_sized([w_time, 18.0], egui::Label::new(egui::RichText::new(dur_str).size(11.0).color(TEXT_MUTED)).selectable(false));
                        });
                    });

                let row_resp = frame_resp.response.interact(egui::Sense::click());
                if row_resp.hovered() && !is_current {
                    ui.painter().rect_filled(frame_resp.response.rect, 4.0, BG_CARD_HOVER);
                }
                if is_current {
                    ui.painter().rect_stroke(frame_resp.response.rect, 4.0, Stroke::new(1.0, Color32::from_rgba_premultiplied(accent.r(), accent.g(), accent.b(), 100)), egui::StrokeKind::Inside);
                }

                if row_resp.clicked() || row_resp.double_clicked() {
                    play_track = Some(track.path.clone());
                }

                row_resp.context_menu(|ui| {
                    if ui.button("▶ Play Now").clicked() {
                        play_track = Some(track.path.clone());
                        ui.close();
                    }
                    if ui.button("Toggle Favourite").clicked() {
                        self.client.toggle_favorite(&track.path);
                        ui.close();
                    }
                    if ui.button("Queue Next").clicked() {
                        queue_track = Some(track.path.clone());
                        ui.close();
                    }
                    if ui.button("✦ Start Smart Mix").clicked() {
                        smartmix_seed = Some(track.path.clone());
                        ui.close();
                    }
                });
            }
        });

        if let Some(path) = play_track {
            if let Some((ref cat, ref name)) = self.drilldown_category {
                if cat == "playlists" {
                    self.client.command("play_path", &[("path", &path), ("list", "playlist"), ("value", name)]);
                } else {
                    self.client.command("play_path", &[("path", &path)]);
                }
            } else {
                self.client.command("play_path", &[("path", &path)]);
            }
        }

        if let Some(p) = queue_track {
            self.client.queue_next(&[&p]);
        }
        if let Some(seed) = smartmix_seed {
            self.current_nav = "smartmix".to_string();
            self.drilldown_category = None;
            self.search_query.clear();
            self.tracks = self.db.generate_smartmix(&seed);
        }
    }

    fn render_simple_table(&mut self, ui: &mut Ui, accent: Color32) {
        let mut clicked_item: Option<String> = None;
        let query = self.search_query.to_lowercase();

        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for (idx, item) in self.simple_items.iter().enumerate() {
                if !query.is_empty() && !item.name.to_lowercase().contains(&query) {
                    continue;
                }
                let fill = if idx % 2 == 0 { ROW_EVEN } else { ROW_ODD };
                let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 38.0), egui::Sense::click());
                let is_hovered = response.hovered();
                let painter = ui.painter();

                let bg_color = if is_hovered {
                    BG_CARD_HOVER
                } else {
                    fill
                };

                painter.rect_filled(rect, 4.0, bg_color);
                if is_hovered {
                    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, BORDER_LIGHT), egui::StrokeKind::Inside);
                }

                // Left: Name
                let name_color = if is_hovered { accent } else { TEXT_PRIMARY };
                let text_pos = egui::Pos2::new(rect.min.x + 12.0, rect.center().y);
                painter.text(
                    text_pos,
                    egui::Align2::LEFT_CENTER,
                    &item.name,
                    egui::FontId::proportional(12.5),
                    name_color,
                );

                // Right: Track count and chevron
                let count_str = format!("{} tracks   ›", item.track_count);
                let count_pos = egui::Pos2::new(rect.max.x - 12.0, rect.center().y);
                painter.text(
                    count_pos,
                    egui::Align2::RIGHT_CENTER,
                    count_str,
                    egui::FontId::proportional(11.0),
                    TEXT_MUTED,
                );

                if response.clicked() {
                    clicked_item = Some(item.name.clone());
                }
            }
        });

        if let Some(name) = clicked_item {
            self.drilldown_category = Some((self.current_nav.clone(), name));
            self.search_query.clear();
            self.load_view_data();
        }
    }

    // -------------------------------------------------------------------------
    // STREAMING SERVICES VIEW (Clean Monochrome Cards)
    // -------------------------------------------------------------------------
    fn render_streaming_view(&mut self, ui: &mut Ui, accent: Color32) {
        if let Some(ref service_id) = self.active_stream_service.clone() {
            let mut back_clicked = false;
            let mut to_play: Option<usize> = None;
            let mut to_open: Option<String> = None;

            ui.horizontal(|ui| {
                if ui.add(egui::Button::new("‹ Back").corner_radius(4.0)).clicked() {
                    back_clicked = true;
                }
                ui.label(egui::RichText::new(format!("Streaming: {}", service_id.to_uppercase())).size(13.5).strong().color(accent));
            });

            if back_clicked {
                self.active_stream_service = None;
                self.load_view_data();
                return;
            }

            ui.add_space(8.0);
            ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                for (idx, row) in self.stream_list.rows.iter().enumerate() {
                    let fill = if idx % 2 == 0 { ROW_EVEN } else { ROW_ODD };
                    Frame::new().fill(fill).corner_radius(4.0).inner_margin(8.0).show(ui, |ui| {
                        let resp = ui.horizontal(|ui| {
                            let icon = if row.playable { "▶" } else { "▪" };
                            ui.label(egui::RichText::new(icon).color(accent).size(11.0));
                            ui.label(egui::RichText::new(&row.title).size(12.0).strong().color(TEXT_PRIMARY));
                            if !row.subtitle.is_empty() {
                                ui.label(egui::RichText::new(&row.subtitle).size(11.0).color(TEXT_MUTED));
                            }
                        });

                        if resp.response.interact(egui::Sense::click()).clicked() {
                            if row.playable {
                                to_play = Some(idx);
                            } else {
                                to_open = Some(row.id.clone());
                            }
                        }
                    });
                }
            });

            if let Some(idx) = to_play {
                self.client.stream_play(service_id, idx);
            }
            if let Some(row_id) = to_open {
                let client = self.client.clone();
                self.stream_list = client.get_stream_list(service_id, "open", &row_id, "");
            }
        } else {
            // Service List
            ui.label(egui::RichText::new("Connected streaming addons on your player:").color(TEXT_SECONDARY).size(11.5));
            ui.add_space(10.0);

            let mut chosen_service: Option<String> = None;
            for service in &self.stream_services {
                let service_id = service.id.clone();
                Frame::new()
                    .fill(BG_CARD)
                    .stroke(Stroke::new(1.0, BORDER))
                    .corner_radius(6.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(&service.name).color(TEXT_PRIMARY).size(13.5).strong());
                                let status_text = if !service.available {
                                    "Not configured on player"
                                } else if service.id == "youtube" || service.signed_in {
                                    "Ready to stream"
                                } else {
                                    "Signed out"
                                };
                                ui.label(egui::RichText::new(status_text).color(TEXT_MUTED).size(10.0));
                            });

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.add(egui::Button::new("Browse").corner_radius(4.0).fill(accent)).clicked() {
                                    chosen_service = Some(service_id.clone());
                                }
                            });
                        });
                    });
                ui.add_space(6.0);
            }

            if let Some(s) = chosen_service {
                self.open_stream_service(&s);
            }
        }
    }

    // -------------------------------------------------------------------------
    // RIGHT PANEL (Now Playing)
    // -------------------------------------------------------------------------
    fn render_right_panel(&mut self, ui: &mut Ui, accent: Color32) {
        ui.vertical_centered(|ui| {
            // 1. Album Artwork (dynamically scaled to fit without scrolling)
            let art_size = (ui.available_height() * 0.32).clamp(120.0, 185.0);
                if let Some(ref tex) = self.art_texture {
                    ui.add(egui::Image::new(tex).fit_to_exact_size(Vec2::splat(art_size)).corner_radius(8.0));
                } else {
                    self.draw_vinyl_placeholder(ui, art_size);
                }

                ui.add_space(6.0);

                // 2. Track Title & Artist
                let (title, artist, badge) = if let Some(ref s) = self.current_state {
                    let t = if !s.title.is_empty() { s.title.clone() } else { "TempoTec Variations V1".to_string() };
                    let a = if !s.artist.is_empty() { s.artist.clone() } else { s.album.clone() };
                    let b = s.format_badge();
                    (t, a, b)
                } else {
                    ("No Track Playing".to_string(), "Connect player to start".to_string(), String::new())
                };

                ui.add(egui::Label::new(egui::RichText::new(title).size(13.5).strong().color(TEXT_PRIMARY)).truncate());
                ui.add(egui::Label::new(egui::RichText::new(artist).size(11.0).color(TEXT_SECONDARY)).truncate());

                if !badge.is_empty() {
                    ui.add_space(4.0);
                    Frame::new()
                        .fill(BG_CARD)
                        .stroke(Stroke::new(1.0, BORDER))
                        .corner_radius(8.0)
                        .inner_margin(Vec2::new(8.0, 2.0))
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new(badge).size(9.0).color(accent).strong());
                        });
                }

                ui.add_space(8.0);
                if ui.add(
                    egui::Button::new(egui::RichText::new("✦ Smart Mix").size(10.5).color(accent))
                        .fill(BG_CARD)
                        .stroke(Stroke::new(1.0, accent))
                        .corner_radius(12.0)
                        .min_size(Vec2::new(90.0, 24.0))
                ).clicked() {
                    let seed = self.current_state.as_ref().map(|s| s.path.as_str()).unwrap_or("");
                    self.current_nav = "smartmix".to_string();
                    self.drilldown_category = None;
                    self.search_query.clear();
                    self.tracks = self.db.generate_smartmix(seed);
                }

                ui.add_space(8.0);

                // 3. Rounded Seekbar & Timestamps
                let total_sec = self.current_state.as_ref().map(|s| s.duration).unwrap_or(0);
                let max_f = (total_sec as f32).max(1.0);

                let slider_w = (ui.available_width() - 8.0).max(180.0);
                let (seek_resp, seek_changed) = rounded_slider(ui, &mut self.seek_val, 0.0, max_f, accent, slider_w, 16.0);
                if seek_resp.drag_started() {
                    self.is_seeking = true;
                }
                if seek_resp.drag_stopped() || seek_changed {
                    self.is_seeking = false;
                    self.client.seek(self.seek_val as u32);
                }

                ui.horizontal(|ui| {
                    let cur_str = format_duration(self.seek_val as u32);
                    let tot_str = format_duration(total_sec);
                    ui.label(egui::RichText::new(cur_str).size(9.5).color(TEXT_MUTED));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(egui::RichText::new(tot_str).size(9.5).color(TEXT_MUTED));
                    });
                });

                ui.add_space(6.0);

                // 4. Vector Transport Controls (All centered vertically)
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(10.0, 0.0);

                    // Mode Toggle (Shuffle / Repeat / Normal)
                    let mode_str = self.current_state.as_ref().map(|s| s.mode.clone()).unwrap_or_else(|| "normal".to_string());
                    if playback_mode_button(ui, &mode_str, accent, 32.0).clicked() {
                        self.client.command("mode", &[("value", "next")]);
                    }

                    // Previous Track
                    if prev_track_button(ui, 32.0).clicked() {
                        self.client.prev();
                    }

                    // Large Circular Play / Pause Vector Button
                    let is_playing = self.current_state.as_ref().map(|s| s.is_playing()).unwrap_or(false);
                    if play_pause_button(ui, is_playing, accent, 46.0).clicked() {
                        if let Some(ref mut s) = self.current_state {
                            s.state = if s.state == 1 { 2 } else { 1 };
                        }
                        self.client.toggle_play();
                    }

                    // Next Track
                    if next_track_button(ui, 32.0).clicked() {
                        self.client.next();
                    }

                    // Mathematical Flawless Vector Heart / Star
                    let is_fav = self.current_state.as_ref().map(|s| s.favourite).unwrap_or(false);
                    let fav_style = self.settings.effective_fav_mark();
                    if favourite_button(ui, is_fav, &fav_style, accent, 32.0).clicked() {
                        if let Some(ref mut s) = self.current_state {
                            s.favourite = !is_fav;
                            self.client.toggle_favorite(&s.path);
                        }
                    }
                });

                ui.add_space(6.0);

                // 5. Hardware DSP Quick-Pills Card
                Frame::new()
                    .fill(BG_CARD)
                    .stroke(Stroke::new(1.0, BORDER))
                    .corner_radius(6.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("HARDWARE DSP").size(8.5).strong().color(TEXT_MUTED));
                        });

                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                            let mseb_on = self.current_state.as_ref().map(|s| s.mseb).unwrap_or(false);
                            if dsp_pill_button(ui, "MSEB", mseb_on, accent, 48.0, 22.0).clicked() {
                                if let Some(ref mut s) = self.current_state {
                                    s.mseb = !mseb_on;
                                }
                                self.client.set_mseb(!mseb_on);
                            }

                            let eq_on = self.current_state.as_ref().map(|s| s.eq).unwrap_or(false);
                            if dsp_pill_button(ui, "EQ", eq_on, accent, 40.0, 22.0).clicked() {
                                if let Some(ref mut s) = self.current_state {
                                    s.eq = !eq_on;
                                }
                                self.client.set_eq(!eq_on);
                            }

                            let peq_on = self.current_state.as_ref().map(|s| s.peq).unwrap_or(false);
                            if dsp_pill_button(ui, "PEQ", peq_on, accent, 44.0, 22.0).clicked() {
                                if let Some(ref mut s) = self.current_state {
                                    s.peq = !peq_on;
                                }
                                self.client.set_peq(!peq_on);
                            }

                            let prof = self.current_state.as_ref().map(|s| s.peq_profile).unwrap_or(1);
                            let p_label = format!("P{}", prof);
                            // Only illuminated when peq_on is true!
                            if dsp_pill_button(ui, &p_label, peq_on, accent, 32.0, 22.0).clicked() {
                                let next_p = if prof == 1 { 2 } else { 1 };
                                if let Some(ref mut s) = self.current_state {
                                    s.peq_profile = next_p;
                                }
                                self.client.set_peq_profile(next_p);
                            }

                            let high_gain = self.current_state.as_ref().map(|s| s.is_high_gain()).unwrap_or(false);
                            let gain_lbl = if high_gain { "HIGH" } else { "LOW" };
                            if dsp_pill_button(ui, gain_lbl, high_gain, accent, 42.0, 22.0).clicked() {
                                if let Some(ref mut s) = self.current_state {
                                    s.high_gain = Some(!high_gain);
                                    s.gain_high = Some(!high_gain);
                                    s.gain = Some(if !high_gain { "high".into() } else { "low".into() });
                                }
                                self.client.toggle_gain();
                            }
                        });
                    });

                ui.add_space(6.0);

                // 6. Modern Rounded Volume Slider
                ui.horizontal(|ui| {
                    let cur_vol = self.volume_val as u8;
                    if volume_speaker_icon(ui, cur_vol, accent, 24.0).clicked() {
                        let new_v = if cur_vol > 0 { 0 } else { 25 };
                        self.client.set_volume(new_v);
                    }

                    let vol_w = (ui.available_width() - 40.0).max(100.0);
                    let (vol_resp, vol_changed) = rounded_slider(ui, &mut self.volume_val, 0.0, 100.0, accent, vol_w, 16.0);
                    if vol_changed || vol_resp.drag_stopped() {
                        self.client.set_volume(self.volume_val as u8);
                    }

                    ui.label(egui::RichText::new(format!("{}%", cur_vol)).size(10.0).color(TEXT_MUTED));
                });
            });
    }

    // -------------------------------------------------------------------------
    // MODALS: Settings, DSP, Devices
    // -------------------------------------------------------------------------
    fn render_settings_modal(&mut self, ctx: &egui::Context, accent: Color32) {
        egui::Window::new("Settings")
            .collapsible(false)
            .resizable(false)
            .default_width(450.0)
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Accent Color:").strong().color(TEXT_PRIMARY));
                ui.add_space(6.0);

                let presets = ACCENT_PRESETS;
                egui::Grid::new("accent_grid").spacing(Vec2::new(8.0, 8.0)).show(ui, |ui| {
                    for (i, preset) in presets.iter().enumerate() {
                        let color = preset.color;
                        let is_sel = self.settings.accent_preference == preset.id;
                        let mut btn = egui::Button::new(egui::RichText::new(preset.name).size(11.0).color(Color32::WHITE))
                            .fill(color)
                            .corner_radius(5.0)
                            .min_size(Vec2::new(95.0, 26.0));

                        if is_sel {
                            btn = btn.stroke(Stroke::new(2.0, Color32::WHITE));
                        }

                        if ui.add(btn).clicked() {
                            self.settings.accent_preference = preset.id.to_string();
                            self.settings.save();
                        }

                        if (i + 1) % 4 == 0 {
                            ui.end_row();
                        }
                    }
                });

                ui.add_space(10.0);
                if ui.selectable_label(self.settings.accent_preference == "device", "Follow Player Hardware Accent").clicked() {
                    self.settings.accent_preference = "device".to_string();
                    self.settings.save();
                }

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                // Favourite Mark
                Frame::new().fill(BG_CARD).stroke(Stroke::new(1.0, BORDER)).corner_radius(6.0).inner_margin(12.0).show(ui, |ui| {
                    ui.label(egui::RichText::new("Favourite Icon Style").size(12.0).strong().color(TEXT_PRIMARY));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.selectable_label(self.settings.fav_mark_preference == "heart", "Heart").clicked() {
                            self.settings.fav_mark_preference = "heart".to_string();
                            self.settings.save();
                        }
                        if ui.selectable_label(self.settings.fav_mark_preference == "star", "Star").clicked() {
                            self.settings.fav_mark_preference = "star".to_string();
                            self.settings.save();
                        }
                    });
                });

                ui.add_space(8.0);

                // Volume Step
                Frame::new().fill(BG_CARD).stroke(Stroke::new(1.0, BORDER)).corner_radius(6.0).inner_margin(12.0).show(ui, |ui| {
                    ui.label(egui::RichText::new("Volume Step").size(12.0).strong().color(TEXT_PRIMARY));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        for s in [1, 2, 3, 4, 5] {
                            if ui.selectable_label(self.settings.volume_step == s, format!("{}%", s)).clicked() {
                                self.settings.volume_step = s;
                                self.settings.save();
                                self.client.set_volume_step(s);
                            }
                        }
                    });
                });

                ui.add_space(8.0);

                // Application Updates
                let mut do_check = false;
                let mut to_install = None;
                let update_opt = self.update_info.clone();
                let is_checking = self.is_checking_update;
                let is_downloading = self.is_downloading_update;

                Frame::new().fill(BG_CARD).stroke(Stroke::new(1.0, BORDER)).corner_radius(6.0).inner_margin(12.0).show(ui, |ui| {
                    ui.label(egui::RichText::new("Application Updates").size(12.0).strong().color(TEXT_PRIMARY));
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(format!("Current version: v{} (Windows x64)", crate::updater::current_version())).size(10.5).color(TEXT_SECONDARY));

                    if let Some(ref msg) = self.update_status_msg {
                        ui.add_space(2.0);
                        ui.label(egui::RichText::new(msg).size(10.5).color(accent));
                    }

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let check_lbl = if is_checking { "Checking..." } else { "Check for Updates" };
                        if ui.add(egui::Button::new(check_lbl).corner_radius(4.0)).clicked() {
                            do_check = true;
                        }

                        if let Some(ref update) = update_opt {
                            if update.is_newer {
                                if is_downloading {
                                    ui.label(egui::RichText::new("Updating...").color(accent).size(11.0));
                                } else {
                                    if ui.add(egui::Button::new(format!("Install {}", update.tag)).fill(accent).corner_radius(4.0)).clicked() {
                                        to_install = Some(update.exe_url.clone());
                                    }
                                }
                            }
                        }
                    });
                });

                if do_check {
                    self.trigger_check_update();
                }
                if let Some(url) = to_install {
                    self.trigger_install_update(url);
                }

                ui.add_space(10.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(egui::Button::new("Done").corner_radius(5.0).fill(accent)).clicked() {
                        self.show_settings = false;
                    }
                });
            });
    }

    fn render_dsp_modal(&mut self, ctx: &egui::Context, accent: Color32) {
        egui::Window::new("Hardware Equalizer & DSP Settings")
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Configure audio engines on your TempoTec V1 / SonixPlayer:").size(10.5).color(TEXT_MUTED));
                ui.add_space(8.0);

                let mseb = self.current_state.as_ref().map(|s| s.mseb).unwrap_or(false);
                let eq = self.current_state.as_ref().map(|s| s.eq).unwrap_or(false);
                let peq = self.current_state.as_ref().map(|s| s.peq).unwrap_or(false);
                let prof = self.current_state.as_ref().map(|s| s.peq_profile).unwrap_or(1);

                Frame::new().fill(BG_CARD).stroke(Stroke::new(1.0, BORDER)).corner_radius(6.0).inner_margin(12.0).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("MSEB Engine:").strong().color(TEXT_PRIMARY));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let label = if mseb { "Enabled" } else { "Disabled" };
                            if ui.add(egui::Button::new(label).fill(if mseb { accent } else { BG_PANEL }).corner_radius(4.0)).clicked() {
                                self.client.set_mseb(!mseb);
                            }
                        });
                    });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Graphic EQ:").strong().color(TEXT_PRIMARY));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let label = if eq { "Enabled" } else { "Disabled" };
                            if ui.add(egui::Button::new(label).fill(if eq { accent } else { BG_PANEL }).corner_radius(4.0)).clicked() {
                                self.client.set_eq(!eq);
                            }
                        });
                    });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Parametric EQ (PEQ):").strong().color(TEXT_PRIMARY));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let label = if peq { "Enabled" } else { "Disabled" };
                            if ui.add(egui::Button::new(label).fill(if peq { accent } else { BG_PANEL }).corner_radius(4.0)).clicked() {
                                self.client.set_peq(!peq);
                            }
                        });
                    });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Active PEQ Profile:").strong().color(TEXT_PRIMARY));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.selectable_label(prof == 2, "Profile 2").clicked() {
                                self.client.set_peq_profile(2);
                            }
                            if ui.selectable_label(prof == 1, "Profile 1").clicked() {
                                self.client.set_peq_profile(1);
                            }
                        });
                    });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Amplifier Gain:").strong().color(TEXT_PRIMARY));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let high_gain = self.current_state.as_ref().map(|s| s.is_high_gain()).unwrap_or(false);
                            if ui.selectable_label(high_gain, "High (+6dB)").clicked() {
                                if let Some(ref mut s) = self.current_state {
                                    s.high_gain = Some(true);
                                    s.gain_high = Some(true);
                                    s.gain = Some("high".into());
                                }
                                self.client.set_gain(true);
                            }
                            if ui.selectable_label(!high_gain, "Low (0dB)").clicked() {
                                if let Some(ref mut s) = self.current_state {
                                    s.high_gain = Some(false);
                                    s.gain_high = Some(false);
                                    s.gain = Some("low".into());
                                }
                                self.client.set_gain(false);
                            }
                        });
                    });
                });

                ui.add_space(10.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(egui::Button::new("Close").corner_radius(5.0)).clicked() {
                        self.show_dsp = false;
                    }
                });
            });
    }

    fn render_devices_modal(&mut self, ctx: &egui::Context, accent: Color32) {
        egui::Window::new("Devices on Network")
            .collapsible(false)
            .resizable(false)
            .default_width(380.0)
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Target Player Address:").size(11.0).color(TEXT_SECONDARY));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label("IP:");
                    ui.add(egui::TextEdit::singleline(&mut self.manual_host_input).desired_width(180.0));
                    if ui.add(egui::Button::new("Connect").fill(accent).corner_radius(4.0)).clicked() {
                        self.client.host = self.manual_host_input.clone();
                        self.settings.last_host = self.manual_host_input.clone();
                        self.settings.save();
                        self.current_state = None;
                        self.trigger_sync_db();
                        self.show_devices = false;
                    }
                });

                ui.add_space(10.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(egui::Button::new("Close").corner_radius(5.0)).clicked() {
                        self.show_devices = false;
                    }
                });
            });
    }
}
