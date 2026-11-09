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
    let track_h = 4.0;
    let track_r = 2.0;
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
    let thumb_r = if is_hovered { 7.5 } else { 6.0 };

    let painter = ui.painter();

    // 1. Background Track
    painter.rect_filled(track_rect, track_r, Color32::from_rgb(28, 30, 38));
    painter.rect_stroke(track_rect, track_r, Stroke::new(1.0, Color32::from_rgb(42, 46, 58)), egui::StrokeKind::Inside);

    // 2. Active Progress Fill
    if progress > 0.001 {
        let active_rect = Rect::from_min_max(
            Pos2::new(rect.min.x, cy - track_h / 2.0),
            Pos2::new(thumb_x, cy + track_h / 2.0),
        );
        painter.rect_filled(active_rect, track_r, accent);
    }

    // 3. Thumb Knob
    if is_hovered {
        painter.circle_filled(thumb_center, thumb_r + 3.0, Color32::from_rgba_premultiplied(accent.r(), accent.g(), accent.b(), 50));
    }
    painter.circle_filled(thumb_center, thumb_r, Color32::WHITE);
    painter.circle_stroke(thumb_center, thumb_r, Stroke::new(1.5, accent));

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
        painter.circle_filled(center, r + 4.0, Color32::from_rgba_premultiplied(accent.r(), accent.g(), accent.b(), 60));
    }

    // Background circle in accent
    painter.circle_filled(center, r, accent);

    // Vector Symbols in Crisp Pure White
    let icon_color = Color32::WHITE;
    if is_playing {
        // Pause: Two rounded vertical bars
        let bar_w = 4.0;
        let bar_h = 15.0;
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
        // Play: Clean geometric triangle pointing right
        let tri_w = 14.0;
        let tri_h = 15.0;
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
    let color = if is_hovered { Color32::WHITE } else { Color32::from_rgb(180, 185, 195) };
    let painter = ui.painter();

    if is_hovered {
        painter.circle_filled(center, size / 2.0, Color32::from_rgb(32, 35, 45));
    }

    // Stop Bar
    let bar_rect = Rect::from_min_max(
        Pos2::new(center.x - 7.0, center.y - 6.5),
        Pos2::new(center.x - 5.0, center.y + 6.5),
    );
    painter.rect_filled(bar_rect, 1.0, color);

    // Left Triangles
    let t1 = Pos2::new(center.x - 5.0, center.y);
    let t2 = Pos2::new(center.x + 1.0, center.y - 5.5);
    let t3 = Pos2::new(center.x + 1.0, center.y + 5.5);
    painter.add(egui::Shape::convex_polygon(vec![t1, t2, t3], color, Stroke::NONE));

    let t4 = Pos2::new(center.x + 1.0, center.y);
    let t5 = Pos2::new(center.x + 7.0, center.y - 5.5);
    let t6 = Pos2::new(center.x + 7.0, center.y + 5.5);
    painter.add(egui::Shape::convex_polygon(vec![t4, t5, t6], color, Stroke::NONE));

    response
}

pub fn next_track_button(ui: &mut Ui, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let center = rect.center();
    let is_hovered = response.hovered();
    let color = if is_hovered { Color32::WHITE } else { Color32::from_rgb(180, 185, 195) };
    let painter = ui.painter();

    if is_hovered {
        painter.circle_filled(center, size / 2.0, Color32::from_rgb(32, 35, 45));
    }

    // Right Triangles
    let t1 = Pos2::new(center.x - 7.0, center.y - 5.5);
    let t2 = Pos2::new(center.x - 1.0, center.y);
    let t3 = Pos2::new(center.x - 7.0, center.y + 5.5);
    painter.add(egui::Shape::convex_polygon(vec![t1, t2, t3], color, Stroke::NONE));

    let t4 = Pos2::new(center.x - 1.0, center.y - 5.5);
    let t5 = Pos2::new(center.x + 5.0, center.y);
    let t6 = Pos2::new(center.x - 1.0, center.y + 5.5);
    painter.add(egui::Shape::convex_polygon(vec![t4, t5, t6], color, Stroke::NONE));

    // Stop Bar
    let bar_rect = Rect::from_min_max(
        Pos2::new(center.x + 5.0, center.y - 6.5),
        Pos2::new(center.x + 7.0, center.y + 6.5),
    );
    painter.rect_filled(bar_rect, 1.0, color);

    response
}

pub fn favourite_button(ui: &mut Ui, is_fav: bool, fav_style: &str, accent: Color32, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let center = rect.center();
    let is_hovered = response.hovered();
    let painter = ui.painter();

    let icon_color = if is_fav {
        accent
    } else if is_hovered {
        Color32::WHITE
    } else {
        Color32::from_rgb(150, 155, 170)
    };

    if is_hovered {
        painter.circle_filled(center, size / 2.0, Color32::from_rgb(32, 35, 45));
    }

    if fav_style == "star" {
        // 5-Point Vector Star
        let outer_r = size * 0.40;
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
            painter.add(egui::Shape::closed_line(points, Stroke::new(1.6, icon_color)));
        }
    } else {
        // Mathematically flawless parametric cardioid heart (60 points)
        let mut points = Vec::with_capacity(64);
        let scale = size * 0.024;
        for i in 0..60 {
            let t = (i as f32) * std::f32::consts::TAU / 60.0;
            let sin_t = t.sin();
            let x = 16.0 * sin_t * sin_t * sin_t;
            let y = -(13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos());
            points.push(Pos2::new(center.x + x * scale, center.y + (y + 1.2) * scale));
        }

        if is_fav {
            painter.add(egui::Shape::convex_polygon(points, icon_color, Stroke::NONE));
        } else {
            painter.add(egui::Shape::closed_line(points, Stroke::new(1.6, icon_color)));
        }
    }

    response
}

pub fn playback_mode_button(ui: &mut Ui, mode: &str, accent: Color32, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let center = rect.center();
    let is_hovered = response.hovered();
    let painter = ui.painter();

    if is_hovered {
        painter.circle_filled(center, size / 2.0, Color32::from_rgb(32, 35, 45));
    }

    let is_active = mode != "normal";
    let icon_color = if is_active {
        accent
    } else if is_hovered {
        Color32::WHITE
    } else {
        Color32::from_rgb(150, 155, 170)
    };

    let stroke = Stroke::new(1.6, icon_color);

    match mode {
        "shuffle" | "shuffle_repeat" => {
            let l = center.x - 7.0;
            let r = center.x + 7.0;
            let t = center.y - 4.5;
            let b = center.y + 4.5;

            painter.line_segment([Pos2::new(l, t), Pos2::new(r, b)], stroke);
            painter.line_segment([Pos2::new(r - 4.0, b), Pos2::new(r, b)], stroke);
            painter.line_segment([Pos2::new(r, b - 4.0), Pos2::new(r, b)], stroke);

            painter.line_segment([Pos2::new(l, b), Pos2::new(r, t)], stroke);
            painter.line_segment([Pos2::new(r - 4.0, t), Pos2::new(r, t)], stroke);
            painter.line_segment([Pos2::new(r, t + 4.0), Pos2::new(r, t)], stroke);
        }
        "repeat" => {
            let w = 6.0;
            let h = 4.5;
            let rect_box = Rect::from_center_size(center, Vec2::new(w * 2.0, h * 2.0));
            painter.rect_stroke(rect_box, 3.0, stroke, egui::StrokeKind::Inside);
            painter.line_segment([Pos2::new(center.x + w - 3.0, center.y - h - 2.5), Pos2::new(center.x + w + 1.5, center.y - h)], stroke);
            painter.line_segment([Pos2::new(center.x + w - 3.0, center.y - h + 2.5), Pos2::new(center.x + w + 1.5, center.y - h)], stroke);
        }
        "repeat_one" => {
            let w = 6.0;
            let h = 4.5;
            let rect_box = Rect::from_center_size(center, Vec2::new(w * 2.0, h * 2.0));
            painter.rect_stroke(rect_box, 3.0, stroke, egui::StrokeKind::Inside);
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                "1",
                egui::FontId::proportional(10.0),
                icon_color,
            );
        }
        _ => {
            let y = center.y;
            painter.line_segment([Pos2::new(center.x - 6.5, y), Pos2::new(center.x + 6.0, y)], stroke);
            painter.line_segment([Pos2::new(center.x + 2.0, y - 3.5), Pos2::new(center.x + 6.0, y)], stroke);
            painter.line_segment([Pos2::new(center.x + 2.0, y + 3.5), Pos2::new(center.x + 6.0, y)], stroke);
        }
    }

    response
}

pub fn header_pill_button(ui: &mut Ui, symbol: &str, label: &str, is_accent: bool, accent: Color32) -> Response {
    let (bg, text_color, stroke) = if is_accent {
        (accent, Color32::WHITE, Stroke::NONE)
    } else {
        (
            Color32::from_rgb(24, 26, 32),
            Color32::from_rgb(220, 225, 235),
            Stroke::new(1.0, Color32::from_rgb(45, 50, 65)),
        )
    };

    let btn = egui::Button::new(
        egui::RichText::new(format!("{}  {}", symbol, label))
            .size(11.5)
            .color(text_color)
            .strong()
    )
    .fill(bg)
    .stroke(stroke)
    .corner_radius(14.0)
    .min_size(Vec2::new(0.0, 28.0));

    ui.add(btn)
}

pub fn volume_speaker_icon(ui: &mut Ui, volume: u8, accent: Color32, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let center = rect.center();
    let is_hovered = response.hovered();
    let painter = ui.painter();

    let color = if is_hovered { Color32::WHITE } else { Color32::from_rgb(160, 166, 180) };

    if is_hovered {
        painter.circle_filled(center, size / 2.0, Color32::from_rgb(32, 35, 45));
    }

    // Minimalist Vector Speaker Horn
    let hx = center.x - 3.0;
    let hy = center.y;

    // Body
    let body_rect = Rect::from_min_max(Pos2::new(hx - 5.0, hy - 3.5), Pos2::new(hx - 2.0, hy + 3.5));
    painter.rect_filled(body_rect, 1.0, color);

    // Cone
    let cone = vec![
        Pos2::new(hx - 2.0, hy - 3.5),
        Pos2::new(hx + 2.5, hy - 7.0),
        Pos2::new(hx + 2.5, hy + 7.0),
        Pos2::new(hx - 2.0, hy + 3.5),
    ];
    painter.add(egui::Shape::convex_polygon(cone, color, Stroke::NONE));

    // Dynamic Sound Wave Arcs based on Volume level
    let wave_stroke = Stroke::new(1.5, if volume > 0 { accent } else { Color32::from_rgb(90, 95, 110) });
    if volume == 0 {
        // Muted 'X' mark
        painter.line_segment([Pos2::new(hx + 4.5, hy - 3.5), Pos2::new(hx + 9.5, hy + 3.5)], Stroke::new(1.5, Color32::from_rgb(220, 80, 80)));
        painter.line_segment([Pos2::new(hx + 4.5, hy + 3.5), Pos2::new(hx + 9.5, hy - 3.5)], Stroke::new(1.5, Color32::from_rgb(220, 80, 80)));
    } else {
        // Wave 1
        painter.line_segment([Pos2::new(hx + 4.5, hy - 3.5), Pos2::new(hx + 6.0, hy)], wave_stroke);
        painter.line_segment([Pos2::new(hx + 6.0, hy), Pos2::new(hx + 4.5, hy + 3.5)], wave_stroke);
        if volume > 40 {
            // Wave 2
            painter.line_segment([Pos2::new(hx + 7.5, hy - 6.0), Pos2::new(hx + 9.5, hy)], wave_stroke);
            painter.line_segment([Pos2::new(hx + 9.5, hy), Pos2::new(hx + 7.5, hy + 6.0)], wave_stroke);
        }
    }

    response
}

pub fn dsp_pill_button(
    ui: &mut Ui,
    label: &str,
    is_active: bool,
    accent: Color32,
    width: f32,
    height: f32,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let is_hovered = response.hovered();
    let is_down = response.is_pointer_button_down_on();
    let painter = ui.painter();

    let (bg, border_color, text_color) = if is_active {
        (
            accent,
            accent,
            Color32::WHITE,
        )
    } else if is_hovered {
        (
            Color32::from_rgb(32, 35, 45),
            Color32::from_rgb(60, 66, 85),
            Color32::from_rgb(220, 225, 235),
        )
    } else {
        (
            Color32::from_rgb(20, 22, 28),
            Color32::from_rgb(38, 42, 54),
            Color32::from_rgb(140, 145, 160),
        )
    };

    let r = 5.0 - if is_down { 0.5 } else { 0.0 };
    painter.rect_filled(rect, r, bg);
    painter.rect_stroke(rect, r, Stroke::new(1.0, border_color), egui::StrokeKind::Inside);

    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(10.5),
        text_color,
    );

    response
}
