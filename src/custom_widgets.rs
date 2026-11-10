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

    let thumb_x = rect.min.x + progress * rect.width();
    let thumb_center = Pos2::new(thumb_x, cy);
    let thumb_r = 6.0;

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
    painter.circle_filled(thumb_center, thumb_r, Color32::WHITE);
    painter.circle_stroke(thumb_center, thumb_r, Stroke::new(1.5, accent));

    (response, changed)
}

pub fn play_pause_button(ui: &mut Ui, is_playing: bool, accent: Color32, diameter: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(diameter), Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let center = rect.center();
    let r = diameter / 2.0 - 1.0;

    let painter = ui.painter();

    // Static outer ring: depth without anything lighting up or shifting on hover
    painter.circle_stroke(
        center,
        r + 3.0,
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 70)),
    );

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
    let (rect, response) = ui.allocate_exact_size(Vec2::new(size, 46.0), Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let center = rect.center();
    let color = Color32::from_rgb(180, 185, 195);
    let painter = ui.painter();

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
    let (rect, response) = ui.allocate_exact_size(Vec2::new(size, 46.0), Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let center = rect.center();
    let color = Color32::from_rgb(180, 185, 195);
    let painter = ui.painter();

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
    let (rect, response) = ui.allocate_exact_size(Vec2::new(size, 46.0), Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let center = rect.center();
    let painter = ui.painter();

    let icon_color = if is_fav {
        accent
    } else {
        Color32::from_rgb(150, 155, 170)
    };

    if fav_style == "star" {
        // 5-Point Vector Star, sized to sit like the heart
        let outer_r = 8.0;
        let inner_r = outer_r * 0.45;
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
        // Smooth symmetrical vector heart with zero triangulation glitches
        // Key anchor points
        let top_cleft = Pos2::new(center.x, center.y - 2.8);
        let bot_tip = Pos2::new(center.x, center.y + 7.5);

        // Right lobe: 2 cubic bezier segments
        let mut right_curve = Vec::with_capacity(18);
        right_curve.push(top_cleft);
        // Segment 1: top cleft to right outer peak
        let p0 = top_cleft;
        let p1 = Pos2::new(center.x + 2.4, center.y - 7.0);
        let p2 = Pos2::new(center.x + 7.5, center.y - 5.5);
        let p3 = Pos2::new(center.x + 7.6, center.y - 1.5);
        for i in 1..=8 {
            let t = i as f32 / 8.0;
            let it = 1.0 - t;
            let pt = Pos2::new(
                it * it * it * p0.x + 3.0 * it * it * t * p1.x + 3.0 * it * t * t * p2.x + t * t * t * p3.x,
                it * it * it * p0.y + 3.0 * it * it * t * p1.y + 3.0 * it * t * t * p2.y + t * t * t * p3.y,
            );
            right_curve.push(pt);
        }
        // Segment 2: right outer peak to bottom tip
        let q0 = p3;
        let q1 = Pos2::new(center.x + 7.6, center.y + 2.5);
        let q2 = Pos2::new(center.x + 3.8, center.y + 5.2);
        let q3 = bot_tip;
        for i in 1..=8 {
            let t = i as f32 / 8.0;
            let it = 1.0 - t;
            let pt = Pos2::new(
                it * it * it * q0.x + 3.0 * it * it * t * q1.x + 3.0 * it * t * t * q2.x + t * t * t * q3.x,
                it * it * it * q0.y + 3.0 * it * it * t * q1.y + 3.0 * it * t * t * q2.y + t * t * t * q3.y,
            );
            right_curve.push(pt);
        }

        // Left lobe is the exact mirror across center.x
        let left_curve: Vec<Pos2> = right_curve
            .iter()
            .rev()
            .map(|pt| Pos2::new(center.x - (pt.x - center.x), pt.y))
            .collect();

        // Full perimeter: top cleft -> right lobe -> bottom tip -> left lobe
        let mut silhouette = right_curve;
        silhouette.extend(left_curve.into_iter().skip(1));

        if is_fav {
            // One triangle fan from an interior hub. Filling two halves separately left an
            // anti-aliasing seam down the middle of the heart.
            let hub = Pos2::new(center.x, center.y + 0.6);
            let mut mesh = egui::Mesh::default();
            mesh.colored_vertex(hub, icon_color);
            for p in &silhouette {
                mesh.colored_vertex(*p, icon_color);
            }
            let n = silhouette.len() as u32;
            for i in 1..n {
                mesh.add_triangle(0, i, i + 1);
            }
            mesh.add_triangle(0, n, 1);
            painter.add(egui::Shape::mesh(mesh));
            // Feather the rim so the fan keeps a smooth outline
            painter.add(egui::Shape::closed_line(silhouette, Stroke::new(0.9, icon_color)));
        } else {
            painter.add(egui::Shape::closed_line(silhouette, Stroke::new(1.6, icon_color)));
        }
    }

    response
}

pub fn playback_mode_button(ui: &mut Ui, mode: &str, accent: Color32, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(size, 46.0), Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let center = rect.center();
    let painter = ui.painter();

    let is_active = mode != "normal";
    let icon_color = if is_active {
        accent
    } else {
        Color32::from_rgb(150, 155, 170)
    };

    let stroke = Stroke::new(1.6, icon_color);

    match mode {
        "shuffle" | "shuffle_repeat" => {
            // Authentic curved crossing arrows (Spotify / Apple Music style)
            let x_left = center.x - 7.5;
            let x_right = center.x + 7.5;
            let y_top = center.y - 4.5;
            let y_bot = center.y + 4.5;

            // Arrow 1: Top-left to Bottom-right S-curve
            let mut pts_top_down = Vec::with_capacity(12);
            pts_top_down.push(Pos2::new(x_left, y_top));
            pts_top_down.push(Pos2::new(center.x - 3.5, y_top));
            let p0 = Pos2::new(center.x - 3.5, y_top);
            let p1 = Pos2::new(center.x - 0.8, y_top);
            let p2 = Pos2::new(center.x + 0.8, y_bot);
            let p3 = Pos2::new(center.x + 3.5, y_bot);
            for i in 1..=6 {
                let t = i as f32 / 6.0;
                let it = 1.0 - t;
                pts_top_down.push(Pos2::new(
                    it * it * it * p0.x + 3.0 * it * it * t * p1.x + 3.0 * it * t * t * p2.x + t * t * t * p3.x,
                    it * it * it * p0.y + 3.0 * it * it * t * p1.y + 3.0 * it * t * t * p2.y + t * t * t * p3.y,
                ));
            }
            pts_top_down.push(Pos2::new(x_right, y_bot));
            painter.add(egui::Shape::line(pts_top_down, stroke));

            // Arrowhead at bottom-right pointing right
            painter.line_segment([Pos2::new(x_right - 3.2, y_bot - 2.8), Pos2::new(x_right, y_bot)], stroke);
            painter.line_segment([Pos2::new(x_right - 3.2, y_bot + 2.8), Pos2::new(x_right, y_bot)], stroke);

            // Arrow 2: Bottom-left to Top-right S-curve
            let mut pts_bot_up = Vec::with_capacity(12);
            pts_bot_up.push(Pos2::new(x_left, y_bot));
            pts_bot_up.push(Pos2::new(center.x - 3.5, y_bot));
            let q0 = Pos2::new(center.x - 3.5, y_bot);
            let q1 = Pos2::new(center.x - 0.8, y_bot);
            let q2 = Pos2::new(center.x + 0.8, y_top);
            let q3 = Pos2::new(center.x + 3.5, y_top);
            for i in 1..=6 {
                let t = i as f32 / 6.0;
                let it = 1.0 - t;
                pts_bot_up.push(Pos2::new(
                    it * it * it * q0.x + 3.0 * it * it * t * q1.x + 3.0 * it * t * t * q2.x + t * t * t * q3.x,
                    it * it * it * q0.y + 3.0 * it * it * t * q1.y + 3.0 * it * t * t * q2.y + t * t * t * q3.y,
                ));
            }
            pts_bot_up.push(Pos2::new(x_right, y_top));
            painter.add(egui::Shape::line(pts_bot_up, stroke));

            // Arrowhead at top-right pointing right
            painter.line_segment([Pos2::new(x_right - 3.2, y_top - 2.8), Pos2::new(x_right, y_top)], stroke);
            painter.line_segment([Pos2::new(x_right - 3.2, y_top + 2.8), Pos2::new(x_right, y_top)], stroke);
        }
        "repeat" | "repeat_all" | "repeat_one" => {
            // A rounded loop that flows into an arrowhead on its top edge.
            let (l, r) = (center.x - 8.0, center.x + 8.0);
            let (t, b) = (center.y - 4.5, center.y + 4.5);
            let rad = 2.6;
            let gap_x = center.x - 1.5;
            let mut pts = vec![Pos2::new(gap_x + 4.0, t)];
            let arc = |cx: f32, cy: f32, a0: f32, a1: f32, pts: &mut Vec<Pos2>| {
                for i in 0..=4 {
                    let a = a0 + (a1 - a0) * (i as f32 / 4.0);
                    pts.push(Pos2::new(cx + a.cos() * rad, cy + a.sin() * rad));
                }
            };
            use std::f32::consts::{FRAC_PI_2, PI};
            pts.push(Pos2::new(r - rad, t));
            arc(r - rad, t + rad, -FRAC_PI_2, 0.0, &mut pts);
            pts.push(Pos2::new(r, b - rad));
            arc(r - rad, b - rad, 0.0, FRAC_PI_2, &mut pts);
            pts.push(Pos2::new(l + rad, b));
            arc(l + rad, b - rad, FRAC_PI_2, PI, &mut pts);
            pts.push(Pos2::new(l, t + rad));
            arc(l + rad, t + rad, PI, PI + FRAC_PI_2, &mut pts);
            pts.push(Pos2::new(gap_x, t));
            painter.add(egui::Shape::line(pts, stroke));

            // Arrowhead closing the loop, pointing right into the gap.
            painter.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(gap_x - 0.4, t - 2.6),
                    Pos2::new(gap_x + 2.4, t),
                    Pos2::new(gap_x - 0.4, t + 2.6),
                ],
                icon_color,
                Stroke::NONE,
            ));

            if mode == "repeat_one" {
                painter.text(
                    Pos2::new(center.x + 1.0, center.y + 0.3),
                    egui::Align2::CENTER_CENTER,
                    "1",
                    egui::FontId::proportional(7.5),
                    icon_color,
                );
            }
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
    let font_id = egui::FontId::proportional(11.5);
    let galley = ui.painter().layout_no_wrap(label.to_string(), font_id.clone(), Color32::WHITE);
    let is_play = symbol == "▶" || symbol == "play";
    let is_shuffle = symbol == "⇄" || symbol == "shuffle";
    let is_plus = symbol == "+" || symbol == "＋" || symbol == "plus";
    let is_refresh = symbol == "⟳" || symbol == "refresh" || symbol == "remix";
    let has_icon = is_play || is_shuffle || is_plus || is_refresh;
    let icon_w = if has_icon { 11.0 } else { 0.0 };
    let gap = if has_icon { 6.0 } else { 0.0 };
    let btn_w = (galley.size().x + icon_w + gap + 26.0).max(72.0);

    let (rect, response) = ui.allocate_exact_size(Vec2::new(btn_w, 28.0), Sense::click());
    let is_hovered = response.hovered();

    let bg = if is_accent {
        if is_hovered {
            Color32::from_rgb(
                ((accent.r() as f32) * 1.12).min(255.0) as u8,
                ((accent.g() as f32) * 1.12).min(255.0) as u8,
                ((accent.b() as f32) * 1.12).min(255.0) as u8,
            )
        } else {
            accent
        }
    } else {
        if is_hovered { Color32::from_rgb(34, 38, 48) } else { Color32::from_rgb(24, 26, 32) }
    };

    let stroke = if is_accent {
        Stroke::NONE
    } else {
        Stroke::new(1.0, Color32::from_rgb(45, 50, 65))
    };
    let text_color = if is_accent { Color32::WHITE } else { Color32::from_rgb(220, 225, 235) };

    let painter = ui.painter();
    painter.rect_filled(rect, 14.0, bg);
    if stroke.width > 0.0 {
        painter.rect_stroke(rect, 14.0, stroke, egui::StrokeKind::Inside);
    }

    let cy = rect.center().y;
    let content_w = galley.size().x + icon_w + gap;
    let start_x = rect.center().x - content_w / 2.0;

    if is_play {
        // Crisp vector play triangle - 100% reliable, never a missing glyph box!
        let tri_w = 8.0;
        let tri_h = 9.0;
        let p1 = Pos2::new(start_x, cy - tri_h / 2.0);
        let p2 = Pos2::new(start_x, cy + tri_h / 2.0);
        let p3 = Pos2::new(start_x + tri_w, cy);
        painter.add(egui::Shape::convex_polygon(vec![p1, p2, p3], text_color, Stroke::NONE));
    } else if is_shuffle {
        // Clean vector shuffle crossing arrows
        let sx = start_x;
        painter.line_segment([Pos2::new(sx, cy - 2.5), Pos2::new(sx + 7.0, cy - 2.5)], Stroke::new(1.3, text_color));
        painter.line_segment([Pos2::new(sx + 5.0, cy - 4.5), Pos2::new(sx + 7.5, cy - 2.5)], Stroke::new(1.3, text_color));
        painter.line_segment([Pos2::new(sx + 5.0, cy - 0.5), Pos2::new(sx + 7.5, cy - 2.5)], Stroke::new(1.3, text_color));

        painter.line_segment([Pos2::new(sx + 7.5, cy + 2.5), Pos2::new(sx + 0.5, cy + 2.5)], Stroke::new(1.3, text_color));
        painter.line_segment([Pos2::new(sx + 2.5, cy + 0.5), Pos2::new(sx, cy + 2.5)], Stroke::new(1.3, text_color));
        painter.line_segment([Pos2::new(sx + 2.5, cy + 4.5), Pos2::new(sx, cy + 2.5)], Stroke::new(1.3, text_color));
    } else if is_plus {
        let sx = start_x + 4.5;
        painter.line_segment([Pos2::new(sx, cy - 4.5), Pos2::new(sx, cy + 4.5)], Stroke::new(1.4, text_color));
        painter.line_segment([Pos2::new(sx - 4.5, cy), Pos2::new(sx + 4.5, cy)], Stroke::new(1.4, text_color));
    } else if is_refresh {
        let sx = start_x + 4.5;
        let r = 4.0;
        painter.circle_stroke(Pos2::new(sx, cy), r, Stroke::new(1.3, text_color));
        painter.line_segment([Pos2::new(sx + r - 2.5, cy - r - 1.5), Pos2::new(sx + r, cy - r)], Stroke::new(1.3, text_color));
        painter.line_segment([Pos2::new(sx + r + 2.5, cy - r - 1.5), Pos2::new(sx + r, cy - r)], Stroke::new(1.3, text_color));
    }

    let text_x = if has_icon { start_x + icon_w + gap } else { start_x };
    painter.text(
        Pos2::new(text_x, cy),
        egui::Align2::LEFT_CENTER,
        label,
        font_id,
        text_color,
    );

    response
}

pub fn battery_icon(ui: &mut Ui, percent: i8, charging: bool, accent: Color32, height: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(height * 1.9, height), Sense::hover());
    let p = ui.painter();
    let pct = percent.clamp(0, 100) as f32;
    let outline = Color32::from_rgb(110, 115, 128);
    let nub_w = (height * 0.2).max(2.0);

    // Shell and terminal
    let body = Rect::from_min_max(rect.min, Pos2::new(rect.max.x - nub_w, rect.max.y));
    p.rect_filled(body, 2.0, Color32::from_rgb(20, 22, 28));
    p.rect_stroke(body, 2.0, Stroke::new(1.0, outline), egui::StrokeKind::Inside);
    p.rect_filled(
        Rect::from_min_size(
            Pos2::new(body.max.x, rect.center().y - height * 0.16),
            Vec2::new(nub_w, height * 0.32),
        ),
        1.0,
        outline,
    );

    // Charge level
    let inner = body.shrink(1.8);
    let level_w = inner.width() * pct / 100.0;
    let level_color = if pct <= 15.0 {
        crate::theme::STATUS_RED
    } else if charging {
        crate::theme::STATUS_GREEN
    } else {
        accent
    };
    if level_w > 0.5 {
        p.rect_filled(Rect::from_min_size(inner.min, Vec2::new(level_w, inner.height())), 1.0, level_color);
    }

    // Charging bolt: two triangles, stays crisp at icon size
    if charging {
        let c = body.center();
        let s = height * 0.45;
        let upper = vec![
            Pos2::new(c.x + s * 0.35, c.y - s),
            Pos2::new(c.x - s * 0.45, c.y + s * 0.05),
            Pos2::new(c.x + s * 0.05, c.y + s * 0.05),
        ];
        let lower = vec![
            Pos2::new(c.x - s * 0.35, c.y + s),
            Pos2::new(c.x + s * 0.45, c.y - s * 0.05),
            Pos2::new(c.x - s * 0.05, c.y - s * 0.05),
        ];
        p.add(egui::Shape::convex_polygon(upper, Color32::WHITE, Stroke::NONE));
        p.add(egui::Shape::convex_polygon(lower, Color32::WHITE, Stroke::NONE));
    }

    response
}

pub fn volume_speaker_icon(ui: &mut Ui, volume: u8, accent: Color32, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let center = rect.center();
    let painter = ui.painter();

    let color = Color32::from_rgb(160, 166, 180);

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
    let painter = ui.painter();

    let (bg, border_color, text_color) = if is_active {
        (accent, accent, Color32::WHITE)
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

    let draw_rect = rect;
    let r = 5.0;
    painter.rect_filled(draw_rect, r, bg);
    painter.rect_stroke(draw_rect, r, Stroke::new(1.0, border_color), egui::StrokeKind::Inside);

    painter.text(
        draw_rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(10.5),
        text_color,
    );

    response
}

pub fn context_menu_item(ui: &mut Ui, label: &str, is_accent: bool, accent: Color32) -> Response {
    // Fixed width: inside a popup `available_width()` is the whole screen, which stretched
    // the menu across the window.
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(170.0, 27.0), Sense::click());
    let is_hov = resp.hovered();
    let is_down = resp.is_pointer_button_down_on();
    let p = ui.painter();

    if is_down {
        p.rect_filled(rect, 4.0, Color32::from_rgb(45, 50, 65));
    } else if is_hov {
        p.rect_filled(rect, 4.0, Color32::from_rgb(32, 35, 46));
    }

    let text_color = if is_hov && is_accent {
        accent
    } else if is_hov {
        Color32::WHITE
    } else {
        Color32::from_rgb(215, 220, 230)
    };

    p.text(
        Pos2::new(rect.left() + 10.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(12.0),
        text_color,
    );

    resp
}
