use eframe::egui::{self, Color32, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

pub fn rounded_slider(
    ui: &mut Ui,
    value: &mut f32,
    min: f32,
    max: f32,
    accent: Color32,
    width: f32,
    height: f32,
) -> (Response, bool) {
    let (rect, mut response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click_and_drag());
    let mut changed = false;

    let cy = rect.center().y;
    let track_h = 6.0;
    let track_r = 3.0;
    let track_rect = Rect::from_min_max(
        Pos2::new(rect.min.x, cy - track_h / 2.0),
        Pos2::new(rect.max.x, cy + track_h / 2.0),
    );

    let progress = if max > min {
        ((*value - min) / (max - min)).clamp(0.0, 1.0)
    } else {
        0.0
    };

    if response.dragged() || response.clicked() {
        if let Some(mouse_pos) = response.interact_pointer_pos() {
            let pct = ((mouse_pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
            *value = min + pct * (max - min);
            changed = true;
            response.mark_changed();
        }
    }

    let is_hovered = response.hovered() || response.dragged();
    let thumb_x = rect.min.x + progress * rect.width();
    let thumb_center = Pos2::new(thumb_x, cy);
    let thumb_r = if is_hovered { 8.5 } else { 7.0 };

    let painter = ui.painter();

    // 1. Background Track
    painter.rect_filled(track_rect, track_r, Color32::from_rgb(32, 35, 45));
    painter.rect_stroke(track_rect, track_r, Stroke::new(1.0, Color32::from_rgb(45, 50, 65)), egui::StrokeKind::Inside);

    // 2. Active Progress Fill
    if progress > 0.001 {
        let active_rect = Rect::from_min_max(
            Pos2::new(rect.min.x, cy - track_h / 2.0),
            Pos2::new(thumb_x, cy + track_h / 2.0),
        );
        painter.rect_filled(active_rect, track_r, accent);
    }

    // 3. Thumb Glow Halo (when hovered or dragged)
    if is_hovered {
        painter.circle_filled(thumb_center, thumb_r + 4.0, Color32::from_rgba_premultiplied(accent.r(), accent.g(), accent.b(), 60));
    }

    // 4. Thumb Knob
    painter.circle_filled(thumb_center, thumb_r, Color32::WHITE);
    painter.circle_stroke(thumb_center, thumb_r, Stroke::new(1.5, accent));
    // Center micro dot
    painter.circle_filled(thumb_center, 2.5, accent);

    (response, changed)
}

pub fn play_pause_button(ui: &mut Ui, is_playing: bool, accent: Color32, diameter: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(diameter), Sense::click());
    let is_hovered = response.hovered();
    let is_down = response.is_pointer_button_down_on();
    let center = rect.center();
    let r = diameter / 2.0 - if is_down { 1.5 } else if is_hovered { 0.0 } else { 1.0 };

    let painter = ui.painter();

    // Soft glowing halo on hover
    if is_hovered {
        painter.circle_filled(center, r + 4.0, Color32::from_rgba_premultiplied(accent.r(), accent.g(), accent.b(), 50));
    }

    // Background circle in accent
    painter.circle_filled(center, r, accent);

    // Vector Symbols in Crisp Pure White
    let icon_color = Color32::WHITE;
    if is_playing {
        // Pause: Two rounded vertical bars
        let bar_w = 4.5;
        let bar_h = 16.0;
        let gap = 5.0;
        let left_bar = Rect::from_min_max(
            Pos2::new(center.x - gap / 2.0 - bar_w, center.y - bar_h / 2.0),
            Pos2::new(center.x - gap / 2.0, center.y + bar_h / 2.0),
        );
        let right_bar = Rect::from_min_max(
            Pos2::new(center.x + gap / 2.0, center.y - bar_h / 2.0),
            Pos2::new(center.x + gap / 2.0 + bar_w, center.y + bar_h / 2.0),
        );
        painter.rect_filled(left_bar, 1.5, icon_color);
        painter.rect_filled(right_bar, 1.5, icon_color);
    } else {
        // Play: Anti-aliased Equilateral Triangle pointing right
        let tri_w = 15.0;
        let tri_h = 16.0;
        let p1 = Pos2::new(center.x - tri_w / 3.0, center.y - tri_h / 2.0);
        let p2 = Pos2::new(center.x - tri_w / 3.0, center.y + tri_h / 2.0);
        let p3 = Pos2::new(center.x + 2.0 * tri_w / 3.0, center.y);
        painter.add(egui::Shape::convex_polygon(vec![p1, p2, p3], icon_color, Stroke::NONE));
    }

    response
}

pub fn prev_track_button(ui: &mut Ui, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let center = rect.center();
    let is_hovered = response.hovered();
    let color = if is_hovered { Color32::WHITE } else { Color32::from_rgb(200, 205, 215) };
    let painter = ui.painter();

    if is_hovered {
        painter.circle_filled(center, size / 2.0, Color32::from_rgb(34, 37, 48));
    }

    // Stop Bar
    let bar_rect = Rect::from_min_max(
        Pos2::new(center.x - 7.0, center.y - 7.0),
        Pos2::new(center.x - 5.0, center.y + 7.0),
    );
    painter.rect_filled(bar_rect, 1.0, color);

    // Left Triangles
    let t1 = Pos2::new(center.x - 5.0, center.y);
    let t2 = Pos2::new(center.x + 1.0, center.y - 6.0);
    let t3 = Pos2::new(center.x + 1.0, center.y + 6.0);
    painter.add(egui::Shape::convex_polygon(vec![t1, t2, t3], color, Stroke::NONE));

    let t4 = Pos2::new(center.x + 1.0, center.y);
    let t5 = Pos2::new(center.x + 7.0, center.y - 6.0);
    let t6 = Pos2::new(center.x + 7.0, center.y + 6.0);
    painter.add(egui::Shape::convex_polygon(vec![t4, t5, t6], color, Stroke::NONE));

    response
}

pub fn next_track_button(ui: &mut Ui, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let center = rect.center();
    let is_hovered = response.hovered();
    let color = if is_hovered { Color32::WHITE } else { Color32::from_rgb(200, 205, 215) };
    let painter = ui.painter();

    if is_hovered {
        painter.circle_filled(center, size / 2.0, Color32::from_rgb(34, 37, 48));
    }

    // Right Triangles
    let t1 = Pos2::new(center.x - 7.0, center.y - 6.0);
    let t2 = Pos2::new(center.x - 1.0, center.y);
    let t3 = Pos2::new(center.x - 7.0, center.y + 6.0);
    painter.add(egui::Shape::convex_polygon(vec![t1, t2, t3], color, Stroke::NONE));

    let t4 = Pos2::new(center.x - 1.0, center.y - 6.0);
    let t5 = Pos2::new(center.x + 5.0, center.y);
    let t6 = Pos2::new(center.x - 1.0, center.y + 6.0);
    painter.add(egui::Shape::convex_polygon(vec![t4, t5, t6], color, Stroke::NONE));

    // Stop Bar
    let bar_rect = Rect::from_min_max(
        Pos2::new(center.x + 5.0, center.y - 7.0),
        Pos2::new(center.x + 7.0, center.y + 7.0),
    );
    painter.rect_filled(bar_rect, 1.0, color);

    response
}

pub fn favourite_button(ui: &mut Ui, is_fav: bool, fav_style: &str, accent: Color32, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let center = rect.center();
    let is_hovered = response.hovered();
    let painter = ui.painter();

    let icon_color = if is_fav { accent } else if is_hovered { Color32::WHITE } else { Color32::from_rgb(160, 166, 181) };

    if is_fav && is_hovered {
        painter.circle_filled(center, size / 2.0, Color32::from_rgba_premultiplied(accent.r(), accent.g(), accent.b(), 40));
    }

    if fav_style == "star" {
        // 5-Point Vector Star
        let outer_r = size * 0.42;
        let inner_r = outer_r * 0.42;
        let mut points = Vec::with_capacity(10);
        for i in 0..10 {
            let angle = std::f32::consts::PI / 5.0 * (i as f32) - std::f32::consts::PI / 2.0;
            let r = if i % 2 == 0 { outer_r } else { inner_r };
            points.push(Pos2::new(center.x + angle.cos() * r, center.y + angle.sin() * r));
        }
        if is_fav {
            painter.add(egui::Shape::convex_polygon(points, icon_color, Stroke::NONE));
        } else {
            painter.add(egui::Shape::closed_line(points, Stroke::new(1.8, icon_color)));
        }
    } else {
        // Vector Heart
        let s = size * 0.40;
        let c1 = Pos2::new(center.x - s * 0.45, center.y - s * 0.3);
        let c2 = Pos2::new(center.x + s * 0.45, center.y - s * 0.3);
        let r = s * 0.45;
        let tip = Pos2::new(center.x, center.y + s * 0.65);

        if is_fav {
            painter.circle_filled(c1, r, icon_color);
            painter.circle_filled(c2, r, icon_color);
            let tri = vec![
                Pos2::new(center.x - r * 0.9, center.y - s * 0.1),
                Pos2::new(center.x + r * 0.9, center.y - s * 0.1),
                tip,
            ];
            painter.add(egui::Shape::convex_polygon(tri, icon_color, Stroke::NONE));
        } else {
            painter.circle_stroke(c1, r, Stroke::new(1.8, icon_color));
            painter.circle_stroke(c2, r, Stroke::new(1.8, icon_color));
            painter.line_segment([Pos2::new(center.x - r * 0.9, center.y - s * 0.1), tip], Stroke::new(1.8, icon_color));
            painter.line_segment([Pos2::new(center.x + r * 0.9, center.y - s * 0.1), tip], Stroke::new(1.8, icon_color));
        }
    }

    response
}

pub fn volume_speaker_icon(ui: &mut Ui, volume: u8, accent: Color32, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let center = rect.center();
    let is_hovered = response.hovered();
    let painter = ui.painter();

    let color = if is_hovered { Color32::WHITE } else { Color32::from_rgb(180, 185, 195) };

    // Speaker body (Left rectangle + middle cone)
    let b_rect = Rect::from_min_max(
        Pos2::new(center.x - 7.0, center.y - 3.5),
        Pos2::new(center.x - 3.0, center.y + 3.5),
    );
    painter.rect_filled(b_rect, 1.0, color);

    let cone = vec![
        Pos2::new(center.x - 3.0, center.y - 3.5),
        Pos2::new(center.x + 1.0, center.y - 7.0),
        Pos2::new(center.x + 1.0, center.y + 7.0),
        Pos2::new(center.x - 3.0, center.y + 3.5),
    ];
    painter.add(egui::Shape::convex_polygon(cone, color, Stroke::NONE));

    // Sound waves or Mute slash
    if volume == 0 {
        // Red Mute Cross
        let red = Color32::from_rgb(231, 76, 60);
        painter.line_segment([Pos2::new(center.x + 3.0, center.y - 4.0), Pos2::new(center.x + 7.0, center.y + 4.0)], Stroke::new(1.8, red));
        painter.line_segment([Pos2::new(center.x + 7.0, center.y - 4.0), Pos2::new(center.x + 3.0, center.y + 4.0)], Stroke::new(1.8, red));
    } else {
        let wave_stroke = Stroke::new(1.5, accent);
        // Wave 1
        painter.line_segment([Pos2::new(center.x + 3.5, center.y - 3.0), Pos2::new(center.x + 5.0, center.y)], wave_stroke);
        painter.line_segment([Pos2::new(center.x + 5.0, center.y), Pos2::new(center.x + 3.5, center.y + 3.0)], wave_stroke);

        // Wave 2
        if volume > 35 {
            painter.line_segment([Pos2::new(center.x + 6.0, center.y - 5.5), Pos2::new(center.x + 8.0, center.y)], wave_stroke);
            painter.line_segment([Pos2::new(center.x + 8.0, center.y), Pos2::new(center.x + 6.0, center.y + 5.5)], wave_stroke);
        }
    }

    response
}

pub fn dsp_pill_button(ui: &mut Ui, label: &str, active: bool, accent: Color32, width: f32, height: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let is_hovered = response.hovered();
    let r = height / 2.0;
    let painter = ui.painter();

    if active {
        // Glowing Illuminated Accent Pill
        painter.rect_filled(rect, r, accent);
        if is_hovered {
            painter.rect_stroke(rect, r, Stroke::new(1.5, Color32::WHITE), egui::StrokeKind::Inside);
        }
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(10.5),
            Color32::WHITE,
        );
    } else {
        // Sleek Dark Matte Card
        let bg = if is_hovered { Color32::from_rgb(34, 37, 48) } else { Color32::from_rgb(26, 28, 35) };
        let border = if is_hovered { Color32::from_rgb(50, 55, 71) } else { Color32::from_rgb(36, 39, 51) };
        let text_c = if is_hovered { Color32::WHITE } else { Color32::from_rgb(160, 166, 181) };
        painter.rect_filled(rect, r, bg);
        painter.rect_stroke(rect, r, Stroke::new(1.0, border), egui::StrokeKind::Inside);
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(10.5),
            text_c,
        );
    }

    response
}
