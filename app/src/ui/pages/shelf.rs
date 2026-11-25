//! The Shelf page: dropped files and the clipboard history.
use super::*;

impl App {
    pub(crate) fn page_shelf(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = ui.painter().clone();
        // Files / Clipboard switch at the top right.
        let sw = Rect::from_min_size(pos2(r.right() - 112.0, r.top() + 13.0), vec2(82.0, 22.0));
        p.rect_filled(sw, 11.0, BTN);
        for (k, v) in [ShelfView::Files, ShelfView::Clipboard].into_iter().enumerate() {
            let cell = Rect::from_min_size(pos2(sw.left() + 2.0 + k as f32 * 40.0, sw.top() + 2.0), vec2(38.0, 18.0));
            if self.shelf_view == v {
                p.rect_filled(cell, 9.0, Color32::from_rgb(99, 102, 241));
            }
            if k == 0 { folder_icon(&p, cell.center(), WHITE) } else { clip_icon(&p, cell.center(), WHITE) }
            if ui.interact(cell, ui.id().with(("sv", k)), Sense::click()).clicked() {
                self.shelf_view = v;
            }
        }
        let area = Rect::from_min_max(pos2(r.left() + 56.0, r.top() + 46.0), pos2(r.right() - 56.0, r.top() + 166.0));
        let files_over = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
        if self.shelf_view == ShelfView::Files || files_over {
            if !self.shelf.is_empty()
                && pill_button(ui, Rect::from_min_size(pos2(r.left() + 80.0, r.top() + 13.0), vec2(50.0, 22.0)), t("Clear"), BTN, "clear")
            {
                self.shelf.clear();
                self.save_shelf();
            }
            if self.shelf.is_empty() || files_over {
                let col = if files_over { BLUE } else { Color32::from_gray(60) };
                dashed_rect(&p, area.shrink2(vec2(4.0, 6.0)), col);
                let t = if files_over { t("Release to add") } else { t("Drop files here") };
                p.text(area.center() - vec2(0.0, 8.0), Align2::CENTER_CENTER, t, bold(15.0), WHITE);
                p.text(area.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, i18n::t("Drag out anytime · click to open"), reg(12.0), DIM);
                return;
            }
            let (mut remove, mut drag) = (None, None);
            // Side by side; more than fit scroll sideways (the wheel scrolls them, not the pages).
            self.scroll_zone = Some((area, false));
            let shelf = self.shelf.clone();
            sideways(ui, area, "shelf-files", |ui| {
                for (i, path) in shelf.iter().enumerate() {
                    // 82 wide: five fit with an edge of the sixth showing, so more is plainly there.
                    let (cell, resp) = ui.allocate_exact_size(vec2(82.0, area.height()), Sense::click_and_drag());
                    // A file deleted or moved since it was dropped stays, faded, until it's removed.
                    let meta = std::fs::metadata(path).ok();
                    let mut p = ui.painter().clone();
                    if meta.is_none() {
                        p.multiply_opacity(0.4);
                    }
                    let cx = cell.center().x;
                    let icon = Rect::from_center_size(pos2(cx, cell.top() + 34.0), vec2(56.0, 56.0));
                    file_icon(&p, icon, path);
                    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    let g = fit_line(&p, &name, bold(12.0), WHITE, 76.0);
                    p.galley(pos2(cx - g.size().x / 2.0, cell.top() + 76.0 - g.size().y / 2.0), g, WHITE);
                    let size = meta.as_ref().map_or_else(|| t("Not there any more").to_string(), |m| human_size(m.len()));
                    p.text(pos2(cx, cell.top() + 92.0), Align2::CENTER_CENTER, size, reg(11.0), DIM);
                    if resp.clicked() && meta.is_some() {
                        let _ = std::process::Command::new("explorer").arg(path).spawn();
                    }
                    if resp.drag_started() && meta.is_some() {
                        drag = Some(path.clone());
                    }
                    if resp.secondary_clicked() {
                        remove = Some(i);
                    }
                    let hint = if meta.is_some() { t("Click to open · drag out · right-click to remove") } else { t("Moved or deleted · right-click to remove") };
                    resp.on_hover_text(hint);
                }
            });
            if let Some(path) = drag {
                dragout::files(&[path]);
            }
            if let Some(i) = remove {
                self.shelf.remove(i);
                self.save_shelf();
            }
        } else {
            if !self.s.clipboard {
                p.text(area.center() - vec2(0.0, 8.0), Align2::CENTER_CENTER, t("Clipboard history is off"), bold(15.0), WHITE);
                p.text(area.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, t("Turn it on in Settings (right-click the notch)"), reg(12.0), DIM);
                return;
            }
            if self.clips.is_empty() {
                p.text(area.center() - vec2(0.0, 8.0), Align2::CENTER_CENTER, t("Copy some text"), bold(15.0), WHITE);
                p.text(area.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, t("It will wait here, ready to paste again"), reg(12.0), DIM);
                return;
            }
            // Clear empties the history (and what "Remember after restart" kept on disk).
            if pill_button(ui, Rect::from_min_size(pos2(r.left() + 80.0, r.top() + 13.0), vec2(50.0, 22.0)), t("Clear"), BTN, "clear-clips") {
                self.clips.clear();
                self.save_clips();
                return;
            }
            let (mut copy, mut drag, mut remove) = (None, None, None);
            let now = Utc::now().timestamp();
            self.scroll_zone = Some((area, false));
            let clips = self.clips.clone();
            let copied = self.copied.clone();
            sideways(ui, area.translate(vec2(0.0, -2.0)), "shelf-clips", |ui| {
                for (i, (ts, text)) in clips.iter().enumerate() {
                    let (cell, resp) = ui.allocate_exact_size(vec2(106.0, 104.0), Sense::click_and_drag());
                    let card = Rect::from_min_size(cell.min, vec2(98.0, 104.0));
                    if resp.drag_started() {
                        drag = Some(text.clone());
                    }
                    let p = ui.painter();
                    p.rect_filled(card, 12.0, if resp.hovered() { Color32::from_gray(30) } else { CARD });
                    if i == 0 {
                        p.rect_stroke(card, 12.0, Stroke::new(1.5_f32, Color32::from_rgb(99, 102, 241)), egui::StrokeKind::Inside);
                    }
                    let just = copied.as_ref().is_some_and(|(c, at)| c == text && at.elapsed().as_secs() < 2);
                    let body = if just { t("Copied").to_string() } else { text.clone() };
                    let g = p.layout(body, bold(12.0), WHITE, card.width() - 22.0);
                    let rows: Vec<_> = g.rows.iter().take(3).collect();
                    let clipped = rows.len() < g.rows.len();
                    let shown: String = clip_text(text, &g, 3, clipped, just);
                    let g2 = p.layout(shown, bold(12.0), WHITE, card.width() - 22.0);
                    p.galley(card.min + vec2(11.0, 12.0), g2, WHITE);
                    p.text(pos2(card.left() + 11.0, card.bottom() - 16.0), Align2::LEFT_CENTER, ago(now - ts), reg(11.0), DIM);
                    if resp.clicked() {
                        copy = Some(text.clone());
                    }
                    if resp.secondary_clicked() {
                        remove = Some(i);
                    }
                    resp.on_hover_text(t("Click to copy · drag into a chat · right-click to remove"));
                }
            });
            if let Some(i) = remove {
                self.clips.remove(i);
                self.save_clips();
            }
            // Above the page dots (it used to run into them).
            p.text(pos2(area.left() + 4.0, r.top() + 160.0), Align2::LEFT_CENTER, tr("{} items · click to copy · drag into a chat", &[&self.clips.len()]), reg(12.0), DIM);
            if let Some(t) = drag {
                dragout::text(&t);
            }
            if let Some(t) = copy {
                if let Some(c) = self.clipboard.as_mut() {
                    let _ = c.set_text(t.clone());
                }
                self.last_clip = t.clone();
                self.copied = Some((t, Instant::now()));
            }
        }
    }
}
