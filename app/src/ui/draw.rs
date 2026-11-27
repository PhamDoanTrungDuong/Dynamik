//! Shapes, icons, buttons and the mascot, drawn with egui's painter.
use super::*;

/// The notch: a black body hanging from the top edge with rounded bottom corners, plus two
/// concave "ears" where it meets the top edge.
pub(crate) fn notch_shape(p: &egui::Painter, body: Rect, rad: f32, ear: f32) {
    p.rect_filled(body, egui::CornerRadius { nw: 0, ne: 0, sw: rad as u8, se: rad as u8 }, BG);
    let mut mesh = egui::epaint::Mesh::default();
    for (corner, dir) in [(body.left_top(), -1.0f32), (body.right_top(), 1.0)] {
        // The ear is the square beside the top corner minus a quarter circle centred one ear
        // out and one ear down; it is a fan seen from the body's top corner.
        let center = corner + vec2(dir * ear, ear);
        let base = mesh.vertices.len() as u32;
        mesh.colored_vertex(corner, BG);
        let n = 10;
        for k in 0..=n {
            let a = (k as f32 / n as f32) * PI / 2.0; // 0: beside the body, PI/2: on the top edge
            mesh.colored_vertex(center + vec2(-dir * ear * a.cos(), -ear * a.sin()), BG);
        }
        for k in 0..n {
            mesh.add_triangle(base, base + 1 + k, base + 2 + k);
        }
    }
    p.add(Shape::mesh(mesh));
}

/// The sky's colour and a short label for a MET Norway symbol ("rainshowers_day", "clearsky_night"…).
pub(crate) fn sky(symbol: &str) -> (Color32, &'static str) {
    let night = symbol.ends_with("_night") || symbol.ends_with("_polartwilight");
    let kind = symbol.split('_').next().unwrap_or("");
    match kind {
        _ if kind.contains("thunder") => (Color32::from_rgb(98, 72, 160), "Thunder"),
        _ if kind.contains("snow") => (Color32::from_rgb(170, 196, 224), "Snow"),
        _ if kind.contains("sleet") => (Color32::from_rgb(120, 150, 180), "Sleet"),
        _ if kind.contains("rain") => (Color32::from_rgb(70, 110, 160), if kind.contains("showers") { "Showers" } else { "Rain" }),
        "fog" => (Color32::from_gray(140), "Fog"),
        "cloudy" => (Color32::from_rgb(120, 132, 150), "Cloudy"),
        "partlycloudy" | "fair" if night => (Color32::from_rgb(60, 70, 140), if kind == "fair" { "Fair" } else { "Partly cloudy" }),
        "partlycloudy" => (Color32::from_rgb(90, 160, 230), "Partly cloudy"),
        "fair" => (Color32::from_rgb(70, 165, 255), "Fair"),
        _ if night => (Color32::from_rgb(52, 58, 130), "Clear sky"),
        _ => (Color32::from_rgb(56, 170, 255), "Clear sky"),
    }
}

/// A small sun, moon, cloud, rain, snow or lightning picture for the symbol.
pub(crate) fn weather_icon(p: &egui::Painter, c: Pos2, symbol: &str, sky: Color32) {
    let night = symbol.ends_with("_night") || symbol.ends_with("_polartwilight");
    let clear = symbol.starts_with("clearsky") || symbol.starts_with("fair") || symbol.starts_with("partlycloudy");
    if clear {
        let sun = c + vec2(-4.0, -5.0);
        if night {
            p.circle_filled(sun, 11.0, Color32::from_rgb(235, 235, 210));
            p.circle_filled(sun + vec2(5.0, -4.0), 9.5, lerp_color(CARD, sky, 0.35)); // a crescent
        } else {
            p.circle_filled(sun, 10.0, Color32::from_rgb(255, 205, 64));
            for k in 0..8 {
                let a = k as f32 * PI / 4.0;
                let d = vec2(a.cos(), a.sin());
                p.line_segment([sun + d * 13.0, sun + d * 17.0], Stroke::new(2.0_f32, Color32::from_rgb(255, 205, 64)));
            }
        }
    }
    if !symbol.starts_with("clearsky") {
        let cloud = Color32::from_gray(if symbol.contains("rain") || symbol.contains("thunder") { 185 } else { 225 });
        let base = c + vec2(4.0, 6.0);
        p.circle_filled(base + vec2(-9.0, 2.0), 7.0, cloud);
        p.circle_filled(base + vec2(0.0, -3.0), 9.0, cloud);
        p.circle_filled(base + vec2(9.0, 2.0), 7.0, cloud);
        p.rect_filled(Rect::from_min_max(base + vec2(-9.0, 2.0), base + vec2(9.0, 9.0)), 0.0, cloud);
        let below = base + vec2(0.0, 14.0);
        if symbol.contains("thunder") {
            p.add(Shape::line(vec![below + vec2(2.0, -2.0), below + vec2(-3.0, 4.0), below + vec2(2.0, 4.0), below + vec2(-2.0, 10.0)], Stroke::new(2.0_f32, Color32::from_rgb(255, 214, 80))));
        } else if symbol.contains("snow") || symbol.contains("sleet") {
            for dx in [-7.0, 0.0, 7.0] {
                p.circle_filled(below + vec2(dx, 2.0), 2.0, WHITE);
            }
        } else if symbol.contains("rain") {
            for dx in [-7.0, 0.0, 7.0] {
                p.line_segment([below + vec2(dx, -1.0), below + vec2(dx - 2.0, 5.0)], Stroke::new(2.0_f32, Color32::from_rgb(120, 180, 255)));
            }
        }
    }
}

pub(crate) fn ring(p: &egui::Painter, c: Pos2, r: f32, w: f32, frac: f32, color: Color32) {
    p.circle_stroke(c, r, Stroke::new(w, color.gamma_multiply(0.22)));
    let frac = frac.clamp(0.0, 1.0);
    if frac > 0.0 {
        let n = (48.0 * frac).max(2.0) as usize;
        let pts: Vec<Pos2> = (0..=n)
            .map(|i| {
                let a = -PI / 2.0 + frac * 2.0 * PI * i as f32 / n as f32;
                c + r * vec2(a.cos(), a.sin())
            })
            .collect();
        p.add(Shape::line(pts, Stroke::new(w, color)));
    }
}

/// The original's little level bars at the right of the header.
pub(crate) fn bars(p: &egui::Painter, c: Pos2, lively: bool, col: Color32) {
    let t = if lively { (Utc::now().timestamp_millis() % 100_000) as f32 / 1000.0 } else { 0.0 };
    let base = [0.55, 1.0, 0.75, 1.0, 0.6];
    for (k, h) in base.iter().enumerate() {
        let wave = if lively { 0.6 + 0.4 * (t * 5.0 + k as f32 * 1.3).sin().abs() } else { 1.0 };
        let hh = 16.0 * h * wave;
        let x = c.x - 10.0 + k as f32 * 5.0;
        p.rect_filled(Rect::from_center_size(pos2(x, c.y), vec2(2.4, hh)), 1.2, col);
    }
}

pub(crate) fn gradient_bar(p: &egui::Painter, r: Rect, frac: f32, a: Color32, b: Color32) {
    p.rect_filled(r, r.height() / 2.0, Color32::from_gray(38));
    let frac = frac.clamp(0.0, 1.0);
    if frac <= 0.0 {
        return;
    }
    let fill = Rect::from_min_size(r.min, vec2(r.width() * frac, r.height()));
    let mut mesh = egui::epaint::Mesh::default();
    let segs = 16;
    for k in 0..=segs {
        let u = k as f32 / segs as f32;
        let x = fill.left() + fill.width() * u;
        let col = lerp_color(a, b, u);
        mesh.colored_vertex(pos2(x, fill.top()), col);
        mesh.colored_vertex(pos2(x, fill.bottom()), col);
        if k > 0 {
            let i = (k * 2) as u32;
            mesh.add_triangle(i - 2, i - 1, i);
            mesh.add_triangle(i - 1, i, i + 1);
        }
    }
    p.add(Shape::mesh(mesh));
    // Round the ends.
    p.circle_filled(pos2(fill.left() + r.height() / 2.0, fill.center().y), r.height() / 2.0, a);
    p.circle_filled(pos2(fill.right() - r.height() / 2.0, fill.center().y), r.height() / 2.0, lerp_color(a, b, 1.0));
}

pub(crate) fn vertical_gradient(p: &egui::Painter, r: Rect, rad: f32, top: Color32, bottom: Color32) {
    p.rect_filled(r, rad, bottom);
    p.rect_filled(Rect::from_min_size(r.min, vec2(r.width(), (r.height() * 0.5).max(rad))), rad, top.gamma_multiply(0.95));
}

pub(crate) fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

pub(crate) fn dashed(p: &egui::Painter, x0: f32, x1: f32, y: f32) {
    let mut x = x0;
    while x < x1 {
        p.line_segment([pos2(x, y), pos2((x + 3.0).min(x1), y)], Stroke::new(1.0_f32, Color32::from_gray(55)));
        x += 6.0;
    }
}

pub(crate) fn dashed_rect(p: &egui::Painter, r: Rect, col: Color32) {
    p.rect_stroke(r, 14.0, Stroke::new(1.3_f32, col), egui::StrokeKind::Inside);
}

pub(crate) fn circle_button(ui: &mut egui::Ui, c: Pos2, rad: f32, fill: Color32, dir: &str) -> bool {
    let rect = Rect::from_center_size(c, vec2(rad * 2.0, rad * 2.0));
    let resp = ui.interact(rect, ui.id().with(("arrow", dir)), Sense::click());
    let p = ui.painter();
    p.circle_filled(c, rad, if resp.hovered() { Color32::from_gray(90) } else { fill });
    let s = if dir == "left" { -1.0 } else { 1.0 };
    let st = Stroke::new(2.0_f32, WHITE);
    p.line_segment([c + vec2(-2.5 * s, -4.5), c + vec2(2.5 * s, 0.0)], st);
    p.line_segment([c + vec2(2.5 * s, 0.0), c + vec2(-2.5 * s, 4.5)], st);
    resp.clicked()
}

/// A round button with a gear (or a tick while what it opens is open); `name` is its tooltip and
/// what screen readers say.
pub(crate) fn gear_button(ui: &mut egui::Ui, c: Pos2, open: bool, name: &str) -> bool {
    let resp = ui.interact(Rect::from_center_size(c, vec2(32.0, 32.0)), ui.id().with("gear"), Sense::click()).on_hover_text(name);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
    let p = ui.painter();
    p.circle_filled(c, 16.0, if resp.hovered() { Color32::from_gray(60) } else { BTN });
    let st = Stroke::new(1.8_f32, WHITE);
    if open {
        p.add(Shape::line(vec![c + vec2(-5.0, 0.5), c + vec2(-1.5, 4.0), c + vec2(5.5, -4.0)], st));
    } else {
        for k in 0..8 {
            let a = k as f32 * PI / 4.0;
            let d = vec2(a.cos(), a.sin());
            p.line_segment([c + d * 5.5, c + d * 8.0], Stroke::new(2.6_f32, WHITE));
        }
        p.circle_stroke(c, 5.0, st);
    }
    resp.clicked()
}

/// A small round button with a "reset" arrow or a "skip" (▶|) mark; `name` is its tooltip and
/// what screen readers say.
pub(crate) fn icon_button(ui: &mut egui::Ui, c: Pos2, rad: f32, kind: &str, name: &str) -> bool {
    let rect = Rect::from_center_size(c, vec2(rad * 2.0, rad * 2.0));
    let resp = ui.interact(rect, ui.id().with(kind), Sense::click()).on_hover_text(name);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
    let p = ui.painter();
    p.circle_filled(c, rad, if resp.hovered() { Color32::from_gray(60) } else { BTN });
    if kind == "skip" {
        p.add(Shape::convex_polygon(vec![c + vec2(-4.0, -4.5), c + vec2(2.0, 0.0), c + vec2(-4.0, 4.5)], WHITE, Stroke::NONE));
        p.rect_filled(Rect::from_center_size(c + vec2(3.6, 0.0), vec2(1.8, 9.0)), 0.5, WHITE);
    } else {
        // A small circular "reset" arrow.
        let pts: Vec<Pos2> = (0..=14).map(|k| {
            let a = -PI * 0.35 + (k as f32 / 14.0) * PI * 1.6;
            c + 5.0 * vec2(a.cos(), a.sin())
        }).collect();
        let end = pts[0];
        p.add(Shape::line(pts, Stroke::new(1.4_f32, WHITE)));
        p.line_segment([end, end + vec2(3.0, -1.0)], Stroke::new(1.4_f32, WHITE));
    }
    resp.clicked()
}

/// A placeholder cover like the original's demo art: pale blue stripes and a blue disc.
pub(crate) fn album_art(p: &egui::Painter, r: Rect) {
    p.rect_filled(r, 10.0, Color32::from_rgb(222, 234, 250));
    let mut y = r.top() + 6.0;
    while y < r.bottom() - 4.0 {
        p.line_segment([pos2(r.left() + 4.0, y), pos2(r.right() - 4.0, y)], Stroke::new(1.5_f32, Color32::from_rgb(190, 212, 242)));
        y += 7.0;
    }
    let c = r.center() + vec2(-6.0, 6.0);
    p.circle_stroke(c, r.width() * 0.3, Stroke::new(2.5_f32, Color32::from_rgb(70, 140, 230)));
    p.circle_filled(c, r.width() * 0.22, Color32::from_rgb(45, 120, 220));
    p.line_segment([r.left_bottom() + vec2(r.width() * 0.25, -2.0), r.right_top() + vec2(-r.width() * 0.25, 2.0)], Stroke::new(1.5_f32, WHITE));
}

/// Media control glyphs: lyrics bubble, previous, play, pause and next.
/// A music control; `tip` is its tooltip and what screen readers say.
pub(crate) fn glyph_button(ui: &mut egui::Ui, c: Pos2, kind: &str, col: Color32, tip: &str) -> bool {
    let rect = Rect::from_center_size(c, vec2(30.0, 26.0));
    let resp = ui.interact(rect, ui.id().with(("glyph", kind)), Sense::click()).on_hover_text(tip);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tip));
    let p = ui.painter();
    if resp.hovered() {
        p.circle_filled(c, 15.0, Color32::from_gray(30));
    }
    let tri = |x: f32, dir: f32, h: f32| vec![pos2(x, c.y - h), pos2(x + dir * h * 1.1, c.y), pos2(x, c.y + h)];
    match kind {
        "play" => {
            p.add(Shape::convex_polygon(tri(c.x - 5.0, 1.0, 7.0), col, Stroke::NONE));
        }
        "pause" => {
            for dx in [-3.5, 3.5] {
                p.rect_filled(Rect::from_center_size(c + vec2(dx, 0.0), vec2(3.5, 13.0)), 1.0, col);
            }
        }
        "next" => {
            p.add(Shape::convex_polygon(tri(c.x - 8.0, 1.0, 5.5), col, Stroke::NONE));
            p.add(Shape::convex_polygon(tri(c.x, 1.0, 5.5), col, Stroke::NONE));
        }
        "prev" => {
            p.add(Shape::convex_polygon(tri(c.x + 8.0, -1.0, 5.5), col, Stroke::NONE));
            p.add(Shape::convex_polygon(tri(c.x, -1.0, 5.5), col, Stroke::NONE));
        }
        _ => {
            let b = Rect::from_center_size(c + vec2(0.0, -1.5), vec2(18.0, 13.0));
            // Outlined speech bubble with two dots, as in reference pictures 12 and 13.
            p.rect_stroke(b, 4.0, Stroke::new(1.6_f32, col), egui::StrokeKind::Inside);
            p.add(Shape::convex_polygon(vec![pos2(b.left() + 4.0, b.bottom() - 1.0), pos2(b.left() + 9.0, b.bottom() - 1.0), pos2(b.left() + 4.0, b.bottom() + 4.0)], col, Stroke::NONE));
            for dx in [-3.0, 3.0] {
                p.circle_filled(b.center() + vec2(dx, 0.0), 1.5, col);
            }
        }
    }
    resp.clicked()
}

pub(crate) fn pill_button(ui: &mut egui::Ui, r: Rect, text: &str, fill: Color32, id: &str) -> bool {
    let resp = ui.interact(r, ui.id().with(("pill", id)), Sense::click());
    let p = ui.painter();
    let f = if resp.hovered() { lerp_color(fill, WHITE, 0.12) } else { fill };
    p.rect_filled(r, r.height() / 2.0, f);
    // Translations can be longer than the English the button was sized for: shrink to fit.
    let wide = p.layout_no_wrap(text.to_string(), bold(14.0), WHITE).size().x;
    let size = if wide > 0.0 { (14.0 * (r.width() - 12.0) / wide).clamp(9.0, 14.0) } else { 14.0 };
    p.text(r.center(), Align2::CENTER_CENTER, text, bold(size), WHITE);
    resp.clicked()
}

pub(crate) fn small_step(ui: &mut egui::Ui, c: Pos2, text: &str, id: &str) -> bool {
    let r = Rect::from_center_size(c, vec2(20.0, 20.0));
    let resp = ui.interact(r, ui.id().with(("step", id)), Sense::click());
    ui.painter().circle_filled(c, 10.0, if resp.hovered() { Color32::from_gray(60) } else { BTN });
    ui.painter().text(c, Align2::CENTER_CENTER, text, bold(13.0), WHITE);
    resp.clicked()
}

pub(crate) fn toggle(ui: &mut egui::Ui, at: Pos2, on: bool) -> bool {
    let r = Rect::from_min_size(at, vec2(44.0, 24.0));
    let resp = ui.interact(r, ui.id().with("toggle"), Sense::click());
    let t = ui.ctx().animate_bool(ui.id().with("toggle-anim"), on);
    let p = ui.painter();
    p.rect_filled(r, 12.0, lerp_color(Color32::from_gray(60), BLUE, t));
    let knob = pos2(r.left() + 12.0 + t * 20.0, r.center().y);
    p.circle_filled(knob, 9.0, WHITE);
    if on {
        p.circle_filled(knob, 3.0, BLUE);
    }
    resp.clicked()
}

pub(crate) fn check_circle(p: &egui::Painter, c: Pos2, r: f32, done: bool) {
    if done {
        p.circle_filled(c, r, GREEN);
        let st = Stroke::new(2.0_f32, Color32::BLACK);
        p.line_segment([c + vec2(-r * 0.45, 0.0), c + vec2(-r * 0.1, r * 0.35)], st);
        p.line_segment([c + vec2(-r * 0.1, r * 0.35), c + vec2(r * 0.45, -r * 0.35)], st);
    } else {
        p.circle_stroke(c, r - 1.0, Stroke::new(1.5_f32, Color32::from_gray(110)));
    }
}

pub(crate) fn bolt(p: &egui::Painter, c: Pos2, s: f32, col: Color32) {
    let pts = [
        c + vec2(0.15 * s, -s),
        c + vec2(-0.55 * s, 0.15 * s),
        c + vec2(-0.05 * s, 0.15 * s),
        c + vec2(-0.15 * s, s),
        c + vec2(0.55 * s, -0.15 * s),
        c + vec2(0.05 * s, -0.15 * s),
    ];
    p.add(Shape::convex_polygon(pts[..3].to_vec(), col, Stroke::NONE));
    p.add(Shape::convex_polygon(pts[3..].to_vec(), col, Stroke::NONE));
    p.add(Shape::convex_polygon(vec![pts[1], pts[2], pts[4], pts[5]], col, Stroke::NONE));
}

pub(crate) fn moon_icon(p: &egui::Painter, c: Pos2, r: f32) {
    p.circle_filled(c, r, WHITE);
    p.circle_filled(c + vec2(r * 0.45, -r * 0.3), r * 0.9, BG);
}

/// A small page of a calendar: a frame with a filled top band.
pub(crate) fn calendar_icon(p: &egui::Painter, c: Pos2, col: Color32) {
    let r = Rect::from_center_size(c, vec2(20.0, 18.0));
    p.rect_stroke(r, 3.0, Stroke::new(1.6_f32, col), egui::StrokeKind::Inside);
    p.rect_filled(Rect::from_min_max(r.min, pos2(r.right(), r.top() + 5.0)), egui::CornerRadius { nw: 3, ne: 3, sw: 0, se: 0 }, col);
    p.circle_filled(c + vec2(0.0, 3.0), 2.0, col);
}

pub(crate) fn drop_icon(p: &egui::Painter, c: Pos2, r: f32, col: Color32) {
    p.circle_filled(c + vec2(0.0, r * 0.25), r * 0.7, col);
    p.add(Shape::convex_polygon(vec![c + vec2(0.0, -r), c + vec2(r * 0.66, r * 0.05), c + vec2(-r * 0.66, r * 0.05)], col, Stroke::NONE));
}

pub(crate) fn folder_icon(p: &egui::Painter, c: Pos2, col: Color32) {
    let r = Rect::from_center_size(c + vec2(0.0, 1.0), vec2(14.0, 10.0));
    p.rect_stroke(r, 2.0, Stroke::new(1.3_f32, col), egui::StrokeKind::Inside);
    p.line_segment([r.left_top() + vec2(1.0, -2.0), r.left_top() + vec2(6.0, -2.0)], Stroke::new(1.3_f32, col));
}

pub(crate) fn clip_icon(p: &egui::Painter, c: Pos2, col: Color32) {
    let r = Rect::from_center_size(c + vec2(0.0, 1.0), vec2(11.0, 13.0));
    p.rect_stroke(r, 2.0, Stroke::new(1.3_f32, col), egui::StrokeKind::Inside);
    p.rect_filled(Rect::from_center_size(pos2(c.x, r.top()), vec2(6.0, 3.0)), 1.0, col);
}

/// A colourful tile per file type (pictures get a little landscape like the original).
pub(crate) fn file_icon(p: &egui::Painter, r: Rect, path: &std::path::Path) {
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let (top, label) = match ext.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" => (Color32::from_rgb(77, 182, 255), ""),
        "pdf" => (Color32::from_rgb(239, 68, 68), "PDF"),
        "doc" | "docx" | "txt" | "md" => (Color32::from_rgb(59, 130, 246), "DOC"),
        "xls" | "xlsx" | "csv" => (Color32::from_rgb(34, 197, 94), "XLS"),
        "zip" | "rar" | "7z" => (ORANGE, "ZIP"),
        "mp3" | "wav" | "flac" | "mp4" | "mkv" | "mov" => (PURPLE, "▶"),
        _ if path.is_dir() => (Color32::from_rgb(250, 204, 21), "DIR"),
        _ => (Color32::from_gray(110), "FILE"),
    };
    p.rect_filled(r, 12.0, top);
    if label.is_empty() {
        let sea = Rect::from_min_max(pos2(r.left(), r.center().y + 4.0), r.max);
        p.rect_filled(sea, egui::CornerRadius { nw: 0, ne: 0, sw: 12, se: 12 }, Color32::from_rgb(37, 150, 210));
        p.rect_filled(Rect::from_min_max(pos2(r.left(), r.center().y + 2.0), pos2(r.right(), r.center().y + 7.0)), 0.0, Color32::from_rgb(250, 214, 140));
        p.circle_filled(pos2(r.right() - 15.0, r.top() + 16.0), 7.0, Color32::from_rgb(253, 243, 180));
    } else {
        p.text(r.center(), Align2::CENTER_CENTER, label, bold(14.0), WHITE);
    }
}

pub(crate) fn sun_icon(p: &egui::Painter, c: Pos2, col: Color32) {
    p.circle_filled(c, 4.5, col);
    for i in 0..8 {
        let a = i as f32 * PI / 4.0;
        let d = vec2(a.cos(), a.sin());
        p.line_segment([c + d * 7.5, c + d * 10.5], Stroke::new(1.8_f32, col));
    }
}

pub(crate) fn bluetooth_rune(p: &egui::Painter, c: Pos2, s: f32, col: Color32) {
    let st = Stroke::new(1.6_f32, col);
    let (top, bot) = (c + vec2(0.0, -s * 1.3), c + vec2(0.0, s * 1.3));
    p.line_segment([top, bot], st);
    p.add(Shape::line(vec![c + vec2(-s * 0.8, -s * 0.65), c + vec2(s * 0.8, s * 0.65), bot], st));
    p.add(Shape::line(vec![c + vec2(-s * 0.8, s * 0.65), c + vec2(s * 0.8, -s * 0.65), top], st));
}

pub(crate) fn headphone_icon(p: &egui::Painter, c: Pos2, s: f32, col: Color32) {
    let pts: Vec<Pos2> = (0..=16).map(|i| {
        let a = PI + PI * i as f32 / 16.0;
        c + vec2(0.0, 2.0) + s * vec2(a.cos(), a.sin())
    }).collect();
    p.add(Shape::line(pts, Stroke::new(1.6_f32, col)));
    for x in [-s, s] {
        p.rect_filled(Rect::from_center_size(c + vec2(x * 0.9, s * 0.55), vec2(3.5, 7.0)), 1.5, col);
    }
}

/// A device's mark: headphones, a mouse, a keyboard, a game controller, or the Bluetooth rune.
pub(crate) fn device_icon(p: &egui::Painter, c: Pos2, s: f32, col: Color32, kind: media::Gadget) {
    let st = Stroke::new(1.5_f32, col);
    match kind {
        media::Gadget::Headset => headphone_icon(p, c, s, col),
        media::Gadget::Mouse => {
            p.rect_stroke(Rect::from_center_size(c, vec2(s * 1.1, s * 1.7)), s * 0.55, st, egui::StrokeKind::Middle);
            p.line_segment([c - vec2(0.0, s * 0.85), c - vec2(0.0, s * 0.25)], st);
        }
        media::Gadget::Keyboard => {
            let r = Rect::from_center_size(c, vec2(s * 2.0, s * 1.2));
            p.rect_stroke(r, 2.0, st, egui::StrokeKind::Middle);
            for k in 0..3 {
                p.circle_filled(pos2(r.left() + s * (0.5 + k as f32 * 0.5), c.y - s * 0.15), 1.0, col);
            }
            p.line_segment([pos2(r.left() + s * 0.5, c.y + s * 0.3), pos2(r.right() - s * 0.5, c.y + s * 0.3)], st);
        }
        media::Gadget::Controller => {
            p.rect_stroke(Rect::from_center_size(c, vec2(s * 2.1, s * 1.2)), s * 0.6, st, egui::StrokeKind::Middle);
            p.line_segment([c + vec2(-s * 0.75, 0.0), c + vec2(-s * 0.25, 0.0)], st);
            p.line_segment([c + vec2(-s * 0.5, -s * 0.25), c + vec2(-s * 0.5, s * 0.25)], st);
            p.circle_filled(c + vec2(s * 0.5, 0.0), 1.3, col);
        }
        media::Gadget::Other => bluetooth_rune(p, c, s * 0.75, col),
    }
}

pub(crate) fn speaker_icon(p: &egui::Painter, c: Pos2, col: Color32, muted: bool) {
    p.rect_filled(Rect::from_center_size(c + vec2(-6.0, 0.0), vec2(5.0, 8.0)), 1.0, col);
    p.add(Shape::convex_polygon(vec![c + vec2(-4.0, -4.0), c + vec2(2.0, -9.0), c + vec2(2.0, 9.0), c + vec2(-4.0, 4.0)], col, Stroke::NONE));
    if muted {
        p.line_segment([c + vec2(6.0, -4.0), c + vec2(13.0, 4.0)], Stroke::new(2.0_f32, col));
        p.line_segment([c + vec2(13.0, -4.0), c + vec2(6.0, 4.0)], Stroke::new(2.0_f32, col));
    } else {
        for (k, r) in [6.0f32, 11.0].iter().enumerate() {
            let pts: Vec<Pos2> = (0..=8).map(|i| {
                let a = -0.8 + 1.6 * i as f32 / 8.0;
                c + vec2(2.0, 0.0) + *r * vec2(a.cos(), a.sin())
            }).collect();
            p.add(Shape::line(pts, Stroke::new(1.8_f32 - k as f32 * 0.3, col)));
        }
    }
}

/// The mascot (reference picture 15): two white eyes on the black pill with a cyan-to-purple glow
/// along the bottom; it blinks now and then (blink timing estimated).
pub(crate) fn mascot(p: &egui::Painter, r: Rect) {
    // Soft glow filling the lower half and following the pill's rounded bottom corners (same radius
    // as notch_shape): cyan on the left, bright blue in the middle, purple fading out on the right.
    // A mesh has no anti-aliasing, so the glow stops short of the shape's edge and fades out over
    // the last few pixels; the black body's smooth edge stays outermost.
    const FEATHER: f32 = 1.5;
    let rad = (r.height() * 0.2).clamp(12.0, 32.0);
    // Bottom edge at x of the shape inset by `inset`: straight, rising along the corner arcs.
    let edge = |x: f32, inset: f32| {
        let rr = rad - inset;
        let into = (rr - (x - r.left() - inset)).max(rr - (r.right() - inset - x)).clamp(0.0, rr);
        r.bottom() - inset - rr + (rr * rr - into * into).sqrt()
    };
    let top = r.top() + r.height() * 0.3;
    let mid = r.top() + r.height() * 0.68;
    // Glow strength depends on height alone (0 at `top`, 0.3 at `mid`, full at the very bottom);
    // the corners just cut it off. Peaking at each column's own edge drew a bright rim round them.
    let strength = |y: f32| {
        if y < mid {
            0.3 * ((y - top) / (mid - top)).max(0.0)
        } else {
            0.3 + 0.7 * ((y - mid) / (r.bottom() - mid)).min(1.0)
        }
    };
    let (x0, x1) = (r.left() + 0.5, r.right() - 0.5);
    let mut mesh = egui::epaint::Mesh::default();
    let segs = 96;
    for k in 0..=segs {
        let u = k as f32 / segs as f32;
        let x = x0 + (x1 - x0) * u;
        let col = if u < 0.5 {
            lerp_color(Color32::from_rgb(34, 211, 238), Color32::from_rgb(59, 110, 255), u * 2.0)
        } else {
            lerp_color(Color32::from_rgb(59, 110, 255), Color32::from_rgb(110, 50, 200), (u - 0.5) * 2.0)
        };
        // The right end fades towards the dark corner, as in the reference; both ends fade out
        // over FEATHER so the sides are as soft as the bottom.
        let side = ((x - x0).min(x1 - x) / FEATHER).clamp(0.0, 1.0);
        let col = col.gamma_multiply((1.0 - 0.45 * ((u - 0.6) / 0.4).clamp(0.0, 1.0)) * side);
        let (inner, outer) = (edge(x, FEATHER + 0.5), edge(x, 0.5));
        let m = mid.min(inner);
        mesh.colored_vertex(pos2(x, top), Color32::TRANSPARENT);
        mesh.colored_vertex(pos2(x, m), col.gamma_multiply(strength(m)));
        mesh.colored_vertex(pos2(x, inner), col.gamma_multiply(strength(inner)));
        mesh.colored_vertex(pos2(x, outer), Color32::TRANSPARENT);
        if k > 0 {
            let i = (k * 4) as u32;
            for row in 0..3 {
                let (a, b) = (i - 4 + row, i + row);
                mesh.add_triangle(a, a + 1, b);
                mesh.add_triangle(a + 1, b, b + 1);
            }
        }
    }
    p.add(Shape::mesh(mesh));
    let open = blink_open(Utc::now().timestamp_millis());
    for dx in [-16.0, 16.0] {
        let c = pos2(r.center().x + dx, r.center().y - 1.0);
        // Closed eyes flatten into a slightly wider line (reference picture 15, second frame).
        let size = vec2(5.0 + 2.0 * (1.0 - open), (8.0 * open).max(1.2));
        // Soft blue halo, fading outwards.
        for (grow, alpha) in [(6.0, 8), (3.5, 14), (1.8, 26)] {
            p.add(Shape::ellipse_filled(c, size + vec2(grow, grow), Color32::from_rgba_unmultiplied(120, 170, 255, alpha)));
        }
        p.add(Shape::ellipse_filled(c, size, WHITE));
    }
}

/// How open the mascot's eyes are (1 = open) at this time: a smooth close-and-open over BLINK_MS.
pub(crate) fn blink_open(ms: i64) -> f32 {
    let b = ms.rem_euclid(BLINK_EVERY_MS);
    if b >= BLINK_MS {
        return 1.0;
    }
    1.0 - (std::f32::consts::PI * b as f32 / BLINK_MS as f32).sin() * 0.9
}
