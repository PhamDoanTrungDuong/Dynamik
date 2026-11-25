//! The Music page: cover, song, controls, progress and lyrics.
use super::*;

impl App {
    /// Music, laid out like reference pictures 13 (plain) and 12 (lyrics open beside it).
    pub(crate) fn page_music(&mut self, ui: &mut egui::Ui, r: Rect) {
        let now = self.media.now.lock().map(|n| n.clone()).unwrap_or_default();
        let p = ui.painter().clone();
        let lyr = self.lyrics_open;
        let art = Rect::from_min_size(r.min + if lyr { vec2(28.0, 16.0) } else { vec2(30.0, 18.0) }, vec2(78.0, 78.0));
        // The song's real cover; the drawn one only while the player gives none.
        match self.art_texture(ui.ctx(), &now) {
            Some(tex) => {
                egui::Image::new(&tex).corner_radius(10).paint_at(ui, art);
            }
            None => album_art(&p, art),
        }
        // The player's icon on the cover's corner, so it's clear where the music comes from.
        if now.active() && !now.app_id.is_empty() {
            let c = art.right_bottom() - vec2(4.0, 4.0);
            p.circle_filled(c, 13.0, BG);
            // With several players open (Spotify and a browser, say), the icon switches between them.
            if now.players > 1 {
                let name = tr("Switch player ({} open)", &[&now.players]);
                let resp = ui.interact(Rect::from_center_size(c, vec2(26.0, 26.0)), ui.id().with("player"), Sense::click()).on_hover_text(&name);
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
                p.circle_stroke(c, 12.5, Stroke::new(1.5_f32, if resp.hovered() { WHITE } else { DIM }));
                if resp.clicked() {
                    self.media.send(media::Cmd::NextPlayer);
                }
            }
            match self.logo_texture(ui.ctx(), &now.app_id) {
                Some(tex) => {
                    p.circle_filled(c, 11.0, Color32::from_gray(38));
                    egui::Image::new(&tex).corner_radius(4).paint_at(ui, Rect::from_center_size(c, vec2(15.0, 15.0)));
                }
                None => {
                    p.circle_filled(c, 11.0, app_color(&now.app_name));
                    let initial: String = now.app_name.chars().next().map(|ch| ch.to_uppercase().collect()).unwrap_or_default();
                    p.text(c, Align2::CENTER_CENTER, initial, bold(11.0), WHITE);
                }
            }
        }
        let tx = art.right() + 22.0;
        let (title, sub) = if now.active() {
            let sub = if now.album.is_empty() { now.artist.clone() } else { format!("{} - {}", now.artist, now.album) };
            (now.title.clone(), sub)
        } else {
            (t("Nothing playing").to_string(), t("Play music in any app").to_string())
        };
        // Cut by width, not letters: up to the lyrics column, or short of the page arrow.
        let room = if lyr { r.left() + 322.0 } else { r.right() - 48.0 } - tx;
        let line = |text: &str, font: FontId, col: Color32, y: f32| {
            let g = fit_line(&p, text, font, col, room);
            p.galley(pos2(tx, y - g.size().y / 2.0), g, col);
        };
        line(&title, bold(18.0), WHITE, art.top() + 44.0);
        line(&sub, reg(14.0), DIM, art.top() + 65.0);

        // Progress row: elapsed, bar, remaining.
        let row_y = r.top() + if lyr { 116.0 } else { 120.0 };
        let (bar_l, bar_r, rem_x) = if lyr { (r.left() + 98.0, r.left() + 238.0, r.left() + 306.0) } else { (r.left() + 100.0, r.right() - 106.0, r.right() - 34.0) };
        let (pos, dur) = (now.pos(), now.duration);
        p.text(pos2(r.left() + 32.0, row_y), Align2::LEFT_CENTER, mmss(pos), reg(15.0), WHITE);
        let bar = Rect::from_min_max(pos2(bar_l, row_y - 2.0), pos2(bar_r, row_y + 2.0));
        p.rect_filled(bar, 2.0, Color32::from_gray(55));
        if dur > 0.0 {
            // Click or drag along the bar to seek; while dragging it follows the pointer.
            let resp = ui.interact(bar.expand2(vec2(4.0, 8.0)), ui.id().with("seek"), Sense::click_and_drag());
            let at = resp.interact_pointer_pos().map(|q| ((q.x - bar.left()) / bar.width()).clamp(0.0, 1.0));
            if let (Some(u), true) = (at, resp.clicked() || resp.drag_stopped()) {
                self.media.send(media::Cmd::Seek(u as f64 * dur));
            }
            let f = at.filter(|_| resp.dragged()).unwrap_or((pos / dur).clamp(0.0, 1.0) as f32);
            p.rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * f, bar.height())), 2.0, now.tint.map_or(WHITE, |[r, g, b]| Color32::from_rgb(r, g, b)));
        }
        let rem = if dur > 0.0 { format!("-{}", mmss(dur - pos)) } else { "-00:00".into() };
        p.text(pos2(rem_x, row_y), Align2::RIGHT_CENTER, rem, reg(15.0), WHITE);

        // Controls.
        let cy = r.top() + if lyr { 150.0 } else { 155.0 };
        let (lx, mid, gap) = if lyr { (r.left() + 68.0, r.left() + 170.0, 52.0) } else { (r.left() + 68.0, r.center().x, 62.0) };
        let on = if now.active() { WHITE } else { FAINT };
        if glyph_button(ui, pos2(lx, cy), "lyrics", if self.s.lyrics { WHITE } else { FAINT }, t("Lyrics")) && self.s.lyrics {
            self.lyrics_open = !self.lyrics_open;
        }
        // Previous / next where the player has them; where it doesn't (a single browser video, say),
        // the same buttons skip back / forward 10 s, marked with a small "10".
        let (pos, dur) = (now.pos(), now.duration);
        let back: Option<(media::Cmd, &str)> = if now.can_prev {
            Some((media::Cmd::Prev, t("Previous")))
        } else if now.can_seek {
            Some((media::Cmd::Seek((pos - SKIP_SECS).max(0.0)), t("Back 10 seconds")))
        } else {
            None
        };
        let fwd: Option<(media::Cmd, &str)> = if now.can_next {
            Some((media::Cmd::Next, t("Next")))
        } else if now.can_seek && dur > 0.0 {
            Some((media::Cmd::Seek((pos + SKIP_SECS).min(dur)), t("Forward 10 seconds")))
        } else {
            None
        };
        for (cmd, kind, x, seeking) in [(back, "prev", mid - gap, !now.can_prev), (fwd, "next", mid + gap, !now.can_next)] {
            let col = if cmd.is_some() { on } else { FAINT };
            let tip = cmd.as_ref().map_or(t("This player doesn't allow it"), |c| c.1);
            if glyph_button(ui, pos2(x, cy), kind, col, tip) {
                if let Some((c, _)) = cmd {
                    self.media.send(c);
                }
            }
            if seeking && now.can_seek && now.active() {
                p.text(pos2(x, cy + 13.0), Align2::CENTER_CENTER, "10", reg(9.0), DIM);
            }
        }
        if glyph_button(ui, pos2(mid, cy), if now.playing { "pause" } else { "play" }, on, if now.playing { t("Pause") } else { t("Play") }) {
            self.media.send(media::Cmd::Toggle);
        }

        if lyr {
            let col = Rect::from_min_max(pos2(r.left() + 344.0, r.top() + 14.0), pos2(r.right() - 30.0, r.bottom() - 8.0));
            p.line_segment([pos2(col.left() - 16.0, r.top() + 30.0), pos2(col.left() - 16.0, r.bottom() - 30.0)], Stroke::new(1.0_f32, Color32::from_gray(40)));
            let clip = p.with_clip_rect(col);
            match now.lyrics_state {
                media::LyricsState::Found => {
                    // The sung line sits in the middle in bold white; neighbours fade with distance.
                    let cur = now.line().unwrap_or(0);
                    let mut jump = None;
                    for (k, (t, line)) in now.lyrics.iter().enumerate() {
                        let d = k as f32 - cur as f32;
                        if d.abs() > 3.0 {
                            continue;
                        }
                        let y = col.center().y - 6.0 + d * 38.0;
                        // Click a line to jump there; it brightens under the pointer.
                        let hit = Rect::from_min_max(pos2(col.left(), y - 18.0), pos2(col.right(), y + 18.0)).intersect(col);
                        let resp = ui.interact(hit, ui.id().with(("lyric", k)), Sense::click());
                        if resp.clicked() {
                            jump = Some(*t);
                        }
                        let (font, mut color) = if k == cur { (bold(19.0), WHITE) } else { (bold(17.0), Color32::from_gray((120.0 - d.abs() * 25.0).max(40.0) as u8)) };
                        if resp.hovered() && k != cur {
                            color = Color32::from_gray(200);
                        }
                        let g = clip.layout(line.clone(), font, color, col.width());
                        clip.galley(pos2(col.left(), y - g.size().y / 2.0), g, color);
                    }
                    if let Some(t) = jump {
                        self.media.send(media::Cmd::Seek(t));
                    }
                }
                state => {
                    let msg = match state {
                        media::LyricsState::Loading => t("Loading lyrics…"),
                        _ if !now.active() => t("Lyrics show here while music plays"),
                        _ => t("No lyrics found for this song"),
                    };
                    clip.text(pos2(col.left(), col.center().y), Align2::LEFT_CENTER, msg, bold(16.0), DIM);
                    clip.text(pos2(col.left(), col.center().y + 24.0), Align2::LEFT_CENTER, t("From LRCLIB, a community-contributed lyrics database"), reg(11.0), FAINT);
                }
            }
        }
    }
}
