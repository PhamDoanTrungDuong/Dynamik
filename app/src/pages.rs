//! The notch's pages: Music, Pomodoro, Water, Shelf, To-do, Prayer, Widgets, Today, Ask and Settings.
use super::*;

impl App {
    pub(crate) fn page_title(&self, i: usize) -> (&'static str, &'static str, Color32) {
        match i {
            0 => ("", "", WHITE), // the album art sits where a title would be
            1 => (t("Pomodoro"), "", WHITE),
            2 => (t("Water "), t("Reminder"), BLUE),
            3 => (t("Shelf"), "", WHITE),
            4 => (t("Todo"), t("List"), GREEN),
            5 => (t("Prayer"), "", WHITE),
            TODAY_PAGE => (t("Today"), "", WHITE),
            _ => (t("Widgets"), "", WHITE),
        }
    }

    pub(crate) fn draw_page(&mut self, ui: &mut egui::Ui, i: usize, r: Rect) {
        match i {
            0 => self.page_music(ui, r),
            1 => self.page_pomodoro(ui, r),
            2 => self.page_water(ui, r),
            3 => self.page_shelf(ui, r),
            4 => self.page_todo(ui, r),
            5 => self.page_prayer(ui, r),
            TODAY_PAGE => self.page_today(ui, r),
            _ => self.page_widgets(ui, r),
        }
    }

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

    pub(crate) fn page_pomodoro(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = ui.painter().clone();
        let x = r.left() + 60.0;
        let label = match self.mode {
            Mode::Focus => tr("Focus · {} of {}", &[&(self.rounds % LONG_BREAK_EVERY + 1).min(LONG_BREAK_EVERY), &LONG_BREAK_EVERY]),
            Mode::Short => t("Short break").into(),
            Mode::Long => t("Long break").into(),
        };
        p.text(pos2(x, r.top() + 50.0), Align2::LEFT_CENTER, label, reg(13.0), DIM);
        p.text(pos2(x - 2.0, r.top() + 80.0), Align2::LEFT_CENTER, mmss(self.left), bold(36.0), WHITE);
        let done_at = self.now + Duration::seconds(self.left.max(0.0) as i64);
        let tg = p.layout_no_wrap(done_at.format("%H:%M").to_string(), bold(16.0), WHITE);
        let tw = tg.size().x;
        p.galley(pos2(x, r.top() + 112.0 - tg.size().y / 2.0), tg, WHITE);
        p.text(pos2(x + tw + 18.0, r.top() + 112.0), Align2::LEFT_CENTER, t("until done"), bold(15.0), PURPLE);
        let frac = 1.0 - (self.left / self.mode_len(self.mode)).clamp(0.0, 1.0) as f32;
        gradient_bar(&p, Rect::from_min_size(pos2(x, r.top() + 135.0), vec2(166.0, 5.0)), frac, PURPLE, Color32::from_rgb(99, 102, 241));
        p.text(pos2(x, r.top() + 154.0), Align2::LEFT_CENTER, tr("Focused today: {}", &[&dur_text(self.focus_today())]), reg(12.0), DIM);
        p.line_segment([pos2(r.left() + 250.0, r.top() + 46.0), pos2(r.left() + 250.0, r.top() + 158.0)], Stroke::new(1.0_f32, Color32::from_gray(40)));

        let rx = r.left() + 268.0;
        p.text(pos2(rx, r.top() + 52.0), Align2::LEFT_CENTER, t("This round"), reg(13.0), DIM);
        let full = self.left >= self.mode_len(self.mode);
        let text = if self.running { t("Pause") } else if full { t("Start") } else { t("Resume") };
        // Space starts and pauses too, on this page and while nothing is being typed.
        let typing = ui.ctx().memory(|m| m.focused().is_some());
        let space = self.page == 1 && !typing && ui.input(|i| i.modifiers.is_none() && i.key_pressed(egui::Key::Space));
        if pill_button(ui, Rect::from_min_size(pos2(r.left() + 354.0, r.top() + 40.0), vec2(78.0, 26.0)), text, BTN, "pp") || space {
            self.running = !self.running;
            self.last_tick = Instant::now();
        }
        // ↺ starts this part over; on a part not started yet, it starts the 4 rounds over.
        let untouched = !self.running && full;
        let all = untouched && (!self.rounds.is_multiple_of(LONG_BREAK_EVERY) || self.mode != Mode::Focus);
        let name = if all { t("Start the 4 rounds over") } else { t("Start this part over") };
        if icon_button(ui, pos2(r.left() + 450.0, r.top() + 53.0), 11.0, "reset", name) {
            self.running = false;
            if all {
                self.rounds = 0;
                self.mode = Mode::Focus;
            }
            self.left = self.mode_len(self.mode);
        }
        if icon_button(ui, pos2(r.left() + 478.0, r.top() + 53.0), 11.0, "skip", t("Skip to the next part")) {
            self.next_part(true);
        }
        // Round track: 4 focus rounds then the long break.
        let y = r.top() + 96.0;
        let seg = 34.0;
        let cur = self.rounds % LONG_BREAK_EVERY;
        for k in 0..=LONG_BREAK_EVERY {
            let sx = rx + k as f32 * (seg + 12.0);
            let done = k < cur || (k == LONG_BREAK_EVERY && false);
            let active = (k == cur && self.mode == Mode::Focus) || (k == LONG_BREAK_EVERY && self.mode == Mode::Long);
            let w = if k == LONG_BREAK_EVERY { 22.0 } else { seg - 4.0 };
            let col = if done { PURPLE } else { Color32::from_gray(38) };
            p.rect_filled(Rect::from_min_size(pos2(sx, y - 3.0), vec2(w, 6.0)), 3.0, col);
            if active {
                let fx = sx + w * frac;
                p.rect_filled(Rect::from_min_size(pos2(sx, y - 3.0), vec2(w * frac, 6.0)), 3.0, PURPLE);
                p.circle_filled(pos2(fx, y), 7.0, Color32::from_rgba_unmultiplied(139, 92, 246, 70));
                p.circle_filled(pos2(fx, y), 4.0, WHITE);
            }
            if k < LONG_BREAK_EVERY {
                p.circle_filled(pos2(sx + seg + 2.0, y), 3.0, if k < cur { Color32::from_rgb(45, 212, 191) } else { Color32::from_gray(38) });
            }
            let lbl = if k == LONG_BREAK_EVERY { t("Long").to_string() } else { (k + 1).to_string() };
            let lc = if active { WHITE } else { DIM };
            p.text(pos2(sx + w / 2.0, y + 22.0), Align2::CENTER_CENTER, lbl, bold(11.0), lc);
        }
        let total = LONG_BREAK_EVERY * self.s.focus + (LONG_BREAK_EVERY - 1) * self.s.short_break + self.s.long_break;
        p.text(pos2(rx, r.top() + 143.0), Align2::LEFT_CENTER, tr("{} · {} sessions", &[&dur_text(total), &LONG_BREAK_EVERY]), reg(12.0), DIM);
    }

    pub(crate) fn page_water(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = ui.painter().clone();
        let x = r.left() + 58.0;
        let liters = self.drank as f32 / 1000.0;
        let big = p.layout_no_wrap(format!("{liters:.1}"), bold(36.0), WHITE);
        let bw = big.size().x;
        p.galley(pos2(x, r.top() + 64.0 - big.size().y / 2.0), big, WHITE);
        p.text(pos2(x + bw + 6.0, r.top() + 72.0), Align2::LEFT_CENTER, format!("/ {} L", litres(self.s.goal)), reg(15.0), DIM);
        if toggle(ui, pos2(r.left() + 210.0, r.top() + 50.0), self.s.water) {
            self.s.water = !self.s.water;
            self.save_water();
        }
        let pct = (self.drank * 100 / self.s.goal.max(1)).min(999);
        let status = if self.drank >= self.s.goal {
            t("Goal reached").to_string()
        } else if !self.s.water {
            t("Reminders off").to_string()
        } else {
            self.next_water_slot().map(|at| tr("next {}", &[&hm(at.ceil() as u32)])).unwrap_or_else(|| t("of your goal").into())
        };
        let ty = r.top() + 98.0;
        // Just after a drink, the line offers to take it back instead.
        let undo = self.last_drink.filter(|d| d.3.elapsed().as_secs_f32() < UNDO_SECS);
        if let Some((ml, ..)) = undo {
            let said = p.layout_no_wrap(tr("Drank {} ml · ", &[&ml]), reg(14.0), DIM);
            let sw = said.size().x;
            p.galley(pos2(x, ty - said.size().y / 2.0), said, DIM);
            let link = Rect::from_min_size(pos2(x + sw, ty - 10.0), vec2(60.0, 20.0));
            let resp = ui.interact(link, ui.id().with("undo"), Sense::click());
            resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, t("Undo")));
            p.text(pos2(x + sw, ty), Align2::LEFT_CENTER, t("Undo"), bold(14.0), if resp.hovered() { WHITE } else { BLUE });
            if resp.clicked() {
                self.undo_drink();
            }
        } else {
            let a = p.layout_no_wrap(t("Today · ").into(), reg(14.0), DIM);
            let b = p.layout_no_wrap(format!("{pct}%"), bold(14.0), BLUE);
            let (aw, bwid) = (a.size().x, b.size().x);
            p.galley(pos2(x, ty - a.size().y / 2.0), a, DIM);
            p.galley(pos2(x + aw, ty - b.size().y / 2.0), b, BLUE);
            p.text(pos2(x + aw + bwid, ty), Align2::LEFT_CENTER, format!(" · {status}"), reg(14.0), DIM);
        }
        gradient_bar(&p, Rect::from_min_size(pos2(x, r.top() + 112.0), vec2(196.0, 6.0)), self.drank as f32 / self.s.goal.max(1) as f32, Color32::from_rgb(56, 189, 248), BLUE);
        let each = self.s.water_plan().each;
        // Drink: the planned sip; +250 ml: a glass; the gear opens the water settings.
        if pill_button(ui, Rect::from_min_size(pos2(x, r.top() + 132.0), vec2(80.0, 32.0)), t("Drink"), BLUE_DARK, "drink") {
            self.drink();
        }
        if pill_button(ui, Rect::from_min_size(pos2(x + 84.0, r.top() + 132.0), vec2(80.0, 32.0)), &format!("+{GLASS_ML} ml"), BTN, "glass") {
            self.drink_ml(GLASS_ML);
        }
        let set_label = if self.water_settings { t("Done") } else { t("Settings") };
        if gear_button(ui, pos2(x + 186.0, r.top() + 148.0), self.water_settings, set_label) {
            self.water_settings = !self.water_settings;
        }
        p.line_segment([pos2(r.left() + 272.0, r.top() + 44.0), pos2(r.left() + 272.0, r.top() + 166.0)], Stroke::new(1.0_f32, Color32::from_gray(40)));
        let right = Rect::from_min_max(pos2(r.left() + 286.0, r.top() + 44.0), pos2(r.right() - 52.0, r.top() + 170.0));
        if self.water_settings {
            self.water_settings_ui(ui, right);
        } else {
            self.water_chart(&p, right);
        }
        let _ = each;
    }

    /// Hourly bars over the active hours, scaled like the original (0x140048333):
    /// top = ceil(max(2 x goal / active hours, biggest hour) / 100) x 100.
    pub(crate) fn water_chart(&self, p: &egui::Painter, r: Rect) {
        let h0 = (self.s.from / 60) as usize;
        let h1 = (self.s.until.div_ceil(60) as usize).clamp(h0 + 1, 24);
        let span_h = ((self.s.until - self.s.from) as f32 / 60.0).max(1.0);
        let biggest = self.hours[h0..h1].iter().copied().max().unwrap_or(0) as f32;
        let top = ((2.0 * self.s.goal as f32 / span_h).max(biggest) / 100.0).ceil().max(1.0) * 100.0;
        let plot = Rect::from_min_max(pos2(r.left() + 6.0, r.top() + 10.0), pos2(r.right() - 44.0, r.bottom() - 26.0));
        for (k, v) in [(0.0, "0 ml".to_string()), (0.5, format!("{}", top / 2.0)), (1.0, format!("{top}"))] {
            let y = plot.bottom() - k * plot.height();
            dashed(p, plot.left(), plot.right(), y);
            p.text(pos2(plot.right() + 8.0, y), Align2::LEFT_CENTER, v, reg(11.0), DIM);
        }
        p.line_segment([pos2(plot.left(), plot.bottom()), pos2(plot.right(), plot.bottom())], Stroke::new(1.0_f32, BLUE_DARK));
        let n = h1 - h0;
        let slot = plot.width() / n as f32;
        let bw = (slot * 0.5).min(9.0);
        for (k, hour) in (h0..h1).enumerate() {
            let cx = plot.left() + slot * (k as f32 + 0.5);
            let track = Rect::from_center_size(pos2(cx, plot.center().y), vec2(bw, plot.height()));
            p.rect_filled(track, bw / 2.0, Color32::from_gray(22));
            let v = self.hours[hour] as f32 / top;
            if v > 0.0 {
                let bar = Rect::from_min_max(pos2(cx - bw / 2.0, plot.bottom() - v.min(1.0) * plot.height()), pos2(cx + bw / 2.0, plot.bottom()));
                vertical_gradient(p, bar, bw / 2.0, Color32::from_rgb(56, 152, 255), Color32::from_rgb(29, 78, 216));
            }
            if (hour - h0).is_multiple_of(3) {
                p.text(pos2(cx, r.bottom() - 10.0), Align2::CENTER_CENTER, format!("{hour:02}"), reg(11.0), DIM);
            }
        }
    }

    pub(crate) fn water_settings_ui(&mut self, ui: &mut egui::Ui, r: Rect) {
        let before = self.s.clone();
        let p = ui.painter().clone();
        let lx = r.left();
        let vx = r.left() + 150.0;
        let rows = [(t("Goal"), 0.0), (t("Bottle"), 30.0), (t("Hours"), 60.0)];
        for (name, dy) in rows {
            p.text(pos2(lx, r.top() + 14.0 + dy), Align2::LEFT_CENTER, name, reg(13.0), DIM);
        }
        let y0 = r.top() + 14.0;
        // Goal: 1 to 4 L in steps of 0.25 L.
        let mut goal = goal_step(self.s.goal);
        if small_step(ui, pos2(vx - 70.0, y0), "-", "g-") { goal = goal_step(goal.saturating_sub(GOAL_STEP)); }
        p.text(pos2(vx - 30.0, y0), Align2::CENTER_CENTER, format!("{} L", litres(goal)), bold(13.0), WHITE);
        if small_step(ui, pos2(vx + 10.0, y0), "+", "g+") { goal = goal_step(goal + GOAL_STEP); }
        self.s.goal = goal;
        let y1 = y0 + 30.0;
        if small_step(ui, pos2(vx - 70.0, y1), "-", "b-") { self.s.bottle = self.s.bottle.saturating_sub(BOTTLE_STEP).max(BOTTLE_MIN); }
        p.text(pos2(vx - 30.0, y1), Align2::CENTER_CENTER, format!("{} ml", self.s.bottle), bold(13.0), WHITE);
        if small_step(ui, pos2(vx + 10.0, y1), "+", "b+") { self.s.bottle = (self.s.bottle + BOTTLE_STEP).min(BOTTLE_MAX); }
        let y2 = y0 + 60.0;
        let (mut from, mut until) = (self.s.from, self.s.until);
        if small_step(ui, pos2(lx + 50.0, y2), "-", "f-") { from = from.saturating_sub(HOURS_STEP); }
        p.text(pos2(lx + 82.0, y2), Align2::CENTER_CENTER, hm(from as u32), bold(13.0), WHITE);
        if small_step(ui, pos2(lx + 114.0, y2), "+", "f+") { from = (from + HOURS_STEP).min(FROM_MAX); }
        p.text(pos2(lx + 132.0, y2), Align2::CENTER_CENTER, t("to"), reg(12.0), DIM);
        if small_step(ui, pos2(lx + 150.0, y2), "-", "u-") { until = until.saturating_sub(HOURS_STEP); }
        p.text(pos2(lx + 182.0, y2), Align2::CENTER_CENTER, hm(until as u32), bold(13.0), WHITE);
        if small_step(ui, pos2(lx + 214.0, y2), "+", "u+") { until = (until + HOURS_STEP).min(UNTIL_MAX); }
        self.s.from = from;
        // "until" always stays at least an hour after "from" (original rule).
        self.s.until = until.max(from + MIN_SPAN).min(UNTIL_MAX);
        let plan = self.s.water_plan();
        p.text(pos2(lx, y0 + 90.0), Align2::LEFT_CENTER, tr("{} reminders · {} refills · {} ml each", &[&plan.reminders, &plan.refills, &plan.each]), reg(12.0), DIM);
        let armed = self.reset_armed.is_some_and(|t| t.elapsed().as_secs_f32() < RESET_CONFIRM_SECS);
        if pill_button(ui, Rect::from_min_size(pos2(r.right() - 64.0, y0 + 78.0), vec2(64.0, 24.0)), if armed { t("Sure?") } else { t("Reset") }, if armed { RED } else { BTN }, "reset") {
            if armed {
                // Reset (0x140053982): today's total, the shown reminders, the hourly amounts, a full bottle.
                self.drank = 0;
                self.fired = self.water_slots_past();
                self.hours = [0; 24];
                self.in_bottle = self.s.bottle;
                self.reset_armed = None;
                self.last_drink = None;
                self.save_water();
            } else {
                self.reset_armed = Some(Instant::now());
            }
        }
        self.in_bottle = self.in_bottle.min(self.s.bottle);
        if self.s != before {
            // Any water setting change starts today's reminders over (0x140053c97), from now on:
            // slots already past count as shown, so a change doesn't set off a catch-up at once.
            self.fired = self.water_slots_past();
            self.save_water();
        }
    }

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

    pub(crate) fn page_todo(&mut self, ui: &mut egui::Ui, r: Rect) {
        let today = self.today();
        let p = ui.painter().clone();
        let x = r.left() + 58.0;
        let done = self.done_today();
        p.text(pos2(x, r.top() + 64.0), Align2::LEFT_CENTER, format!("{done}/{}", self.tasks.len()), bold(36.0), WHITE);
        p.text(pos2(x, r.top() + 96.0), Align2::LEFT_CENTER, t("Task Done Today"), reg(14.0), DIM);
        if pill_button(ui, Rect::from_min_size(pos2(x - 4.0, r.top() + 120.0), vec2(90.0, 32.0)), t("Add Task"), BTN, "add") {
            self.form = Some((None, String::new(), String::new(), 0));
        }
        let edit_label = if self.todo_edit { t("Done") } else { t("Edit Task") };
        if pill_button(ui, Rect::from_min_size(pos2(x + 92.0, r.top() + 120.0), vec2(90.0, 32.0)), edit_label, BTN, "edit") {
            self.todo_edit = !self.todo_edit;
        }
        let list = Rect::from_min_max(pos2(r.left() + 262.0, r.top() + 44.0), pos2(r.right() - 52.0, r.top() + 170.0));

        if let Some((idx, title, time, dur)) = &mut self.form {
            let mut close = false;
            let mut save = false;
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(list), |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                ui.label(egui::RichText::new(if idx.is_some() { t("Edit Task") } else { t("Add Task") }).font(bold(14.0)));
                ui.add(egui::TextEdit::singleline(title).hint_text(t("Task title")).desired_width(list.width()));
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(t("Starts")).color(DIM));
                    ui.add(egui::TextEdit::singleline(time).hint_text("HH:MM").desired_width(52.0));
                    ui.label(egui::RichText::new(t("for")).color(DIM));
                    ui.add(egui::DragValue::new(dur).range(0..=480).suffix(t(" min")));
                });
                let ok = !title.trim().is_empty() && (time.trim().is_empty() || parse_hm(time).is_some());
                ui.horizontal(|ui| {
                    if ui.add_enabled(ok, egui::Button::new(t("Save")).corner_radius(12.0)).clicked() {
                        save = true;
                    }
                    if ui.add(egui::Button::new(t("Cancel")).corner_radius(12.0)).clicked() {
                        close = true;
                    }
                });
            });
            if save {
                let (idx, title, time, dur) = self.form.take().unwrap();
                let t = Task { title: title.trim().into(), start: parse_hm(&time), dur, ..Default::default() };
                match idx {
                    Some(i) if i < self.tasks.len() => self.tasks[i] = Task { done_day: self.tasks[i].done_day, ..t },
                    _ => self.tasks.push(t),
                }
                data::save_tasks(&self.tasks);
            } else if close {
                self.form = None;
            }
            return;
        }

        let events = self.calendar.lock().unwrap().events.clone();
        if self.tasks.is_empty() && events.is_empty() {
            p.text(list.center(), Align2::CENTER_CENTER, t("No tasks."), reg(14.0), DIM);
            return;
        }
        let mut toggle_i = None;
        let mut remove = None;
        let mut edit = None;
        self.scroll_zone = Some((list, true)); // the wheel scrolls this list, not the pages
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(list), |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                // Today's calendar events come first; they are read from the calendar, so not editable here.
                for e in &events {
                    let (row, _) = ui.allocate_exact_size(vec2(list.width() - 12.0, 34.0), Sense::hover());
                    let pp = ui.painter();
                    pp.rect_filled(row.shrink2(vec2(0.0, 3.0)), 9.0, BLUE_DARK.gamma_multiply(0.45));
                    calendar_icon(pp, pos2(row.left() + 20.0, row.center().y), BLUE);
                    let title = if e.title.is_empty() { t("(No title)") } else { e.title.as_str() };
                    pp.text(pos2(row.left() + 38.0, row.center().y), Align2::LEFT_CENTER, ellipsize(title, 26), reg(14.0), WHITE);
                    let when = match e.start {
                        Some(s) => hm(s),
                        None if e.carried => tr("until {}", &[&hm(e.dur)]),
                        None => t("All day").to_string(),
                    };
                    pp.text(pos2(row.right() - 10.0, row.center().y), Align2::RIGHT_CENTER, when, reg(12.0), DIM);
                }
                for (i, t) in self.tasks.iter().enumerate() {
                    let (row, resp) = ui.allocate_exact_size(vec2(list.width() - 12.0, 34.0), Sense::click());
                    let is_done = t.done_day == Some(today); // done counts for the day it was done
                    let fill = if is_done { GREEN_DARK } else if resp.hovered() { Color32::from_gray(32) } else { CARD };
                    let pp = ui.painter();
                    pp.rect_filled(row.shrink2(vec2(0.0, 3.0)), 9.0, fill);
                    check_circle(pp, pos2(row.left() + 20.0, row.center().y), 9.0, is_done);
                    let col = if is_done { Color32::from_gray(185) } else { WHITE };
                    let g = pp.layout_no_wrap(ellipsize(&t.title, 26), reg(14.0), col);
                    let gw = g.size().x;
                    let ty = row.center().y;
                    pp.galley(pos2(row.left() + 38.0, ty - g.size().y / 2.0), g, col);
                    if is_done {
                        pp.line_segment([pos2(row.left() + 38.0, ty), pos2(row.left() + 38.0 + gw, ty)], Stroke::new(1.2_f32, col));
                    }
                    if let Some(s) = t.start {
                        pp.text(pos2(row.right() - (if self.todo_edit { 34.0 } else { 10.0 }), ty), Align2::RIGHT_CENTER, hm(s as u32), reg(12.0), DIM);
                    }
                    if self.todo_edit {
                        let xr = Rect::from_center_size(pos2(row.right() - 14.0, ty), vec2(20.0, 20.0));
                        pp.text(xr.center(), Align2::CENTER_CENTER, "×", bold(16.0), RED);
                        if ui.interact(xr, ui.id().with(("del", i)), Sense::click()).clicked() {
                            remove = Some(i);
                        } else if resp.clicked() {
                            edit = Some((Some(i), t.title.clone(), t.start.map(|s| hm(s as u32)).unwrap_or_default(), t.dur));
                        }
                    } else if resp.clicked() {
                        toggle_i = Some(i);
                    }
                }
            });
        });
        if let Some(i) = toggle_i {
            let t = &mut self.tasks[i];
            t.done_day = if t.done_day == Some(today) { None } else { Some(today) };
            data::save_tasks(&self.tasks);
        }
        if let Some(i) = remove {
            self.tasks.remove(i);
            data::save_tasks(&self.tasks);
        }
        if edit.is_some() {
            self.form = edit;
        }
    }

    pub(crate) fn page_prayer(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = ui.painter().clone();
        let x = r.left() + 60.0;
        if !self.s.prayer {
            p.text(pos2(x, r.top() + 70.0), Align2::LEFT_CENTER, t("Prayer times"), bold(26.0), WHITE);
            p.text(pos2(x, r.top() + 100.0), Align2::LEFT_CENTER, t("are off"), reg(15.0), DIM);
            if pill_button(ui, Rect::from_min_size(pos2(x, r.top() + 122.0), vec2(150.0, 30.0)), t("Turn on"), BLUE_DARK, "pon") {
                self.s.prayer = true;
                self.s.save();
            }
            return;
        }
        // "My location" without a position yet: say so (times for 0°, 0° would be wrong), and offer to look.
        if !self.prayer_placed() {
            let state = *self.locating.lock().unwrap();
            let (head, line) = match state {
                Locating::Busy => (t("Finding where you are…"), ""),
                Locating::Failed => (t("Couldn't find your location"), t("Check Windows Settings › Privacy › Location, or pick a city in Settings")),
                _ => (t("No location yet"), t("Prayer times need to know where you are")),
            };
            p.text(pos2(x, r.top() + 66.0), Align2::LEFT_CENTER, head, bold(20.0), WHITE);
            p.text(pos2(x, r.top() + 92.0), Align2::LEFT_CENTER, line, reg(13.0), DIM);
            if state != Locating::Busy && pill_button(ui, Rect::from_min_size(pos2(x, r.top() + 116.0), vec2(170.0, 30.0)), t("Find my location"), BLUE_DARK, "ploc") {
                find_location(&self.locating);
            }
            return;
        }
        let today = self.today();
        let city = self.city_name();
        let city = city.as_str();
        // The city just after the page title, however long the title is in this language.
        let title_w = p.layout_no_wrap(t("Prayer").into(), bold(17.0), WHITE).size().x;
        let room = r.center().x - 120.0 - (r.left() + 22.0 + title_w + 12.0);
        let g = fit_line(&p, city, reg(13.0), DIM, room);
        p.galley(pos2(r.left() + 22.0 + title_w + 12.0, r.top() + 24.0 - g.size().y / 2.0), g, DIM);
        // Today's Hijri date, above the day arc (worked out, so it can be a day from a sighted calendar).
        let (hy, hmon, hd) = prayer::hijri(today);
        let hijri = format!("{hd} {} {hy}", i18n::t(prayer::HIJRI_MONTHS[(hmon as usize).clamp(1, 12) - 1]));
        p.text(pos2(r.right() - 38.0, r.top() + 52.0), Align2::RIGHT_CENTER, hijri, reg(11.0), DIM);
        let t = self.prayer_times(today);
        let m = self.now_min();
        let now_f = m as f32 + self.now.second() as f32 / 60.0;
        let next = t.iter().position(|&x| x > m);
        let (name, at, label) = match next {
            Some(i) => (i18n::t(prayer::NAMES[i]), t[i], i18n::t("Next")),
            None => (i18n::t(prayer::NAMES[0]), self.prayer_times(today + Duration::days(1))[0], i18n::t("Next, tomorrow")),
        };
        p.text(pos2(x, r.top() + 46.0), Align2::LEFT_CENTER, label, reg(13.0), DIM);
        p.text(pos2(x - 1.0, r.top() + 76.0), Align2::LEFT_CENTER, name, bold(32.0), WHITE);
        let until = (at as i64 - m as i64).rem_euclid(1440);
        let ag = p.layout_no_wrap(hm(at), bold(17.0), WHITE);
        let aw = ag.size().x;
        p.galley(pos2(x, r.top() + 112.0 - ag.size().y / 2.0), ag, WHITE);
        p.text(pos2(x + aw + 18.0, r.top() + 112.0), Align2::LEFT_CENTER, tr("in {}", &[&dur_text(until as u32)]), bold(15.0), BLUE);
        let prev = next.and_then(|i| i.checked_sub(1)).map(|i| t[i]).unwrap_or(t[4]);
        let gap = (at as i64 - prev as i64).rem_euclid(1440).max(1) as f32;
        let frac = 1.0 - until as f32 / gap;
        gradient_bar(&p, Rect::from_min_size(pos2(x, r.top() + 136.0), vec2(166.0, 5.0)), frac, BLUE, PURPLE);
        p.line_segment([pos2(r.left() + 250.0, r.top() + 46.0), pos2(r.left() + 250.0, r.top() + 162.0)], Stroke::new(1.0_f32, Color32::from_gray(40)));

        // Day arc: the sun travels from Subuh to Isya, past part in blue.
        let ax0 = r.left() + 276.0;
        let ax1 = r.right() - 52.0;
        let base = r.top() + 124.0;
        let height = 62.0;
        let (t0, t1) = (t[0] as f32, t[4] as f32);
        let xy = |tm: f32| {
            let u = ((tm - t0) / (t1 - t0)).clamp(0.0, 1.0);
            pos2(ax0 + u * (ax1 - ax0), base - height * (PI * (u * 0.82 + 0.02)).sin())
        };
        p.line_segment([pos2(ax0, base + 2.0), pos2(ax1, base + 2.0)], Stroke::new(1.0_f32, Color32::from_gray(45)));
        let steps = 60;
        let pts: Vec<Pos2> = (0..=steps).map(|k| xy(t0 + (t1 - t0) * k as f32 / steps as f32)).collect();
        let cut = (((now_f - t0) / (t1 - t0)).clamp(0.0, 1.0) * steps as f32) as usize;
        p.add(Shape::line(pts[cut..].to_vec(), Stroke::new(1.5_f32, Color32::from_gray(70))));
        if cut > 0 {
            p.add(Shape::line(pts[..=cut].to_vec(), Stroke::new(2.5_f32, BLUE)));
        }
        // Sunrise: a small hollow sun on the arc, no label (it isn't a prayer).
        let rise = prayer::sunrise_for(today, self.s.prayer_city, (self.s.prayer_lat, self.s.prayer_lon));
        p.circle_stroke(xy(rise as f32), 4.0, Stroke::new(1.3_f32, Color32::from_rgb(253, 230, 160)));
        // Labels at least LABEL_GAP apart, so close times (Maghrib and Isha far north) don't overlap.
        let label_x = spread(&t.map(|pt| xy(pt as f32).x), PRAYER_LABEL_GAP, ax0 - 10.0, ax1 + 14.0);
        for (i, &pt) in t.iter().enumerate() {
            let pos = xy(pt as f32);
            p.circle_filled(pos, 3.0, if (pt as f32) <= now_f { Color32::from_rgb(147, 197, 253) } else { Color32::from_gray(150) });
            let is_next = next == Some(i);
            let col = if is_next { WHITE } else { DIM };
            let f = if is_next { bold(11.0) } else { reg(11.0) };
            p.text(pos2(label_x[i], base + 16.0), Align2::CENTER_CENTER, i18n::t(prayer::NAMES[i]), f.clone(), col);
            p.text(pos2(label_x[i], base + 30.0), Align2::CENTER_CENTER, hm(pt), reg(10.0), DIM);
        }
        // Imsak (10 minutes before Fajr, kept in Ramadan) and sunrise, under the countdown.
        let imsak = t[0].saturating_sub(IMSAK_BEFORE_MIN);
        p.text(pos2(x, r.top() + 156.0), Align2::LEFT_CENTER, tr("Imsak {} · Sunrise {}", &[&hm(imsak), &hm(rise)]), reg(11.0), DIM);
        if now_f >= t0 && now_f <= t1 {
            let sun = xy(now_f);
            for (rad, a) in [(14.0, 30), (10.0, 70), (6.5, 255)] {
                p.circle_filled(sun, rad, Color32::from_rgba_unmultiplied(253, 230, 160, a));
            }
        }
    }

    pub(crate) fn page_widgets(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = ui.painter().clone();
        let a = Rect::from_min_size(pos2(r.left() + 56.0, r.top() + 42.0), vec2(212.0, 126.0));
        let b = Rect::from_min_size(pos2(r.left() + 280.0, r.top() + 42.0), vec2(212.0, 126.0));
        for (rect, kind) in [(a, self.s.widget_left), (b, self.s.widget_right)] {
            p.rect_filled(rect, 14.0, CARD);
            self.widget(&p, rect, kind);
            self.widget_opens(ui, rect, kind);
        }
    }

    /// A widget with a page behind it opens that page when clicked: the week → To-do, the next
    /// prayer → Prayer, focus → Pomodoro, water → Water (if that page is shown).
    pub(crate) fn widget_opens(&mut self, ui: &mut egui::Ui, rect: Rect, kind: usize) {
        let to = match kind {
            0 => 4,
            2 => 5,
            3 => 1,
            7 => 2,
            _ => return,
        };
        if !self.order().contains(&to) {
            return;
        }
        let name = tr("Open {}", &[&t(PAGES[to])]);
        let resp = ui.interact(rect, ui.id().with(("widget", rect.left() as i32)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
        if resp.hovered() {
            ui.painter().rect_stroke(rect, 14.0, Stroke::new(1.0_f32, Color32::from_gray(60)), egui::StrokeKind::Inside);
        }
        if resp.clicked() {
            self.page = to;
        }
    }

    /// Recent notifications on the left, the week (the "This week" widget) on the right.
    pub(crate) fn page_today(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = ui.painter().clone();
        let a = Rect::from_min_size(pos2(r.left() + 56.0, r.top() + 42.0), vec2(212.0, 126.0));
        let b = Rect::from_min_size(pos2(r.left() + 280.0, r.top() + 42.0), vec2(212.0, 126.0));
        p.rect_filled(a, 14.0, CARD);
        p.text(a.min + vec2(14.0, 15.0), Align2::LEFT_CENTER, t("NOTIFICATIONS"), bold(11.0), DIM);
        if self.recent.is_empty() {
            p.text(a.center() - vec2(0.0, 4.0), Align2::CENTER_CENTER, t("No new notifications"), bold(13.0), WHITE);
            let hint = if self.s.notifications { t("New ones show up here") } else { t("Notifications are off in Settings") };
            p.text(a.center() + vec2(0.0, 16.0), Align2::CENTER_CENTER, hint, reg(11.0), DIM);
        } else {
            if pill_button(ui, Rect::from_min_size(pos2(a.right() - 58.0, a.top() + 6.0), vec2(48.0, 18.0)), t("Clear"), BTN, "today_clear") {
                self.recent.clear();
            }
            let now = Utc::now().timestamp();
            // All of them (up to RECENT_MAX), scrolling; text cut by real width.
            let list = Rect::from_min_max(pos2(a.left() + 4.0, a.top() + 26.0), pos2(a.right() - 4.0, a.bottom() - 6.0));
            self.scroll_zone = Some((list, true));
            let preview = self.s.notification_preview;
            // One row per app, its newest notification shown with how many it has ("Zalo · 3").
            let groups = group_by_app(&self.recent);
            let logos: Vec<Option<egui::TextureHandle>> = groups.iter().map(|g| self.logo_texture(ui.ctx(), &g.1)).collect();
            let (mut open, mut remove) = (None, None);
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(list), |ui| {
                egui::ScrollArea::vertical().id_salt("today-recent").auto_shrink(false).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for ((app, id, ts, (title, body), count), logo) in groups.iter().zip(&logos) {
                        // Click: open the app that sent it. Right-click: take that app's off the list.
                        let (row, resp) = ui.allocate_exact_size(vec2(list.width() - 8.0, 32.0), Sense::click());
                        let hint = if can_open_app(id) { tr("Click to open {} · right-click to remove", &[app]) } else { t("Right-click to remove").to_string() };
                        let resp = resp.on_hover_text(&hint);
                        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("{app}: {title}")));
                        if resp.clicked() && can_open_app(id) {
                            open = Some(id.clone());
                        }
                        if resp.secondary_clicked() {
                            remove = Some((app.clone(), id.clone()));
                        }
                        let p = ui.painter();
                        if resp.hovered() {
                            p.rect_filled(row, 8.0, Color32::from_gray(30));
                        }
                        let icon = pos2(row.left() + 18.0, row.center().y);
                        match logo {
                            Some(tex) => {
                                p.rect_filled(Rect::from_center_size(icon, vec2(22.0, 22.0)), 6.0, Color32::from_gray(38));
                                egui::Image::new(tex).corner_radius(4).paint_at(ui, Rect::from_center_size(icon, vec2(16.0, 16.0)));
                            }
                            None => {
                                p.circle_filled(icon, 10.0, app_color(app));
                                let initial: String = app.chars().next().map(|c| c.to_uppercase().collect()).unwrap_or_default();
                                p.text(icon, Align2::CENTER_CENTER, initial, bold(11.0), WHITE);
                            }
                        }
                        let (x, right) = (row.left() + 36.0, row.right() - 6.0);
                        let when = p.layout_no_wrap(ago(now - ts), reg(10.0), DIM);
                        let ww = when.size().x;
                        p.galley(pos2(right - ww, row.top() + 9.0 - when.size().y / 2.0), when, DIM);
                        let label = if *count > 1 { format!("{app} · {count}") } else { app.clone() };
                        let who = fit_line(p, &label, reg(10.0), DIM, right - x - ww - 8.0);
                        p.galley(pos2(x, row.top() + 9.0 - who.size().y / 2.0), who, DIM);
                        let line = if preview && !body.is_empty() { format!("{title}: {body}") } else { title.clone() };
                        let said = fit_line(p, &line, bold(12.0), WHITE, right - x);
                        p.galley(pos2(x, row.top() + 23.0 - said.size().y / 2.0), said, WHITE);
                    }
                });
            });
            if let Some(id) = open {
                open_app(&id);
            }
            if let Some((app, id)) = remove {
                self.recent.retain(|(_, n)| !(n.0 == app && n.3 == id));
            }
        }
        p.rect_filled(b, 14.0, CARD);
        self.widget(&p, b, 0);
        self.widget_opens(ui, b, 0);
    }

    pub(crate) fn widget(&self, p: &egui::Painter, r: Rect, kind: usize) {
        let title = |t: &str| p.text(r.min + vec2(14.0, 15.0), Align2::LEFT_CENTER, t, bold(11.0), DIM);
        let today = self.today();
        match kind {
            0 => {
                title(t("THIS WEEK"));
                let ws = if self.s.week_monday { Weekday::Mon } else { Weekday::Sun };
                let start = today.week(ws).first_day();
                for k in 0..7 {
                    let d = start + Duration::days(k);
                    let cx = r.left() + 23.0 + k as f32 * 27.5;
                    let letter = i18n::day_letter(d.weekday());
                    p.text(pos2(cx, r.top() + 36.0), Align2::CENTER_CENTER, letter, bold(10.0), DIM);
                    let col = if d < today { DIM } else { WHITE };
                    if d == today {
                        p.circle_filled(pos2(cx, r.top() + 58.0), 11.0, BLUE);
                    }
                    p.text(pos2(cx, r.top() + 58.0), Align2::CENTER_CENTER, d.day().to_string(), bold(13.0), col);
                }
                // Today: open tasks and the calendar's events, and whichever comes next of either.
                let open = self.tasks.iter().filter(|t| t.done_day != Some(today)).count();
                let events = self.calendar.lock().unwrap().events.clone();
                p.circle_filled(pos2(r.left() + 18.0, r.top() + 95.0), 4.0, GREEN);
                let head = if events.is_empty() { tr("{} tasks today", &[&open]) } else { tr("{} tasks · {} events today", &[&open, &events.len()]) };
                p.text(pos2(r.left() + 28.0, r.top() + 95.0), Align2::LEFT_CENTER, head, bold(12.0), WHITE);
                let m = self.now_min();
                let task_next = self.tasks.iter().filter(|t| t.done_day != Some(today) && t.start.is_some_and(|s| s as u32 >= m)).map(|t| (t.start.unwrap() as u32, t.title.clone()));
                let event_next = events.iter().filter_map(|e| e.start.filter(|&s| s >= m).map(|s| (s, if e.title.is_empty() { t("(No title)").to_string() } else { e.title.clone() })));
                let line = task_next.chain(event_next).min_by_key(|(s, _)| *s).map_or_else(|| t("Nothing scheduled").to_string(), |(s, title)| tr("Next: {}, {}", &[&title, &hm(s)]));
                let g = fit_line(p, &line, reg(11.0), DIM, r.right() - 14.0 - (r.left() + 28.0));
                p.galley(pos2(r.left() + 28.0, r.top() + 111.0 - g.size().y / 2.0), g, DIM);
            }
            1 => {
                title(t("BATTERY"));
                let c = pos2(r.left() + 50.0, r.top() + 72.0);
                match self.battery {
                    Some((pct, ac)) => {
                        let col = if pct <= 20 && !ac { RED } else { GREEN };
                        ring(p, c, 28.0, 6.0, pct as f32 / 100.0, col);
                        if ac { bolt(p, c, 11.0, WHITE) } else { p.text(c, Align2::CENTER_CENTER, format!("{pct}"), bold(14.0), WHITE); }
                        p.text(pos2(r.left() + 92.0, r.top() + 50.0), Align2::LEFT_CENTER, format!("{pct}%"), bold(24.0), WHITE);
                        p.text(pos2(r.left() + 92.0, r.top() + 76.0), Align2::LEFT_CENTER, if ac { t("Charging") } else { t("On battery") }, bold(12.0), if ac { GREEN } else { WHITE });
                        p.text(pos2(r.left() + 92.0, r.top() + 93.0), Align2::LEFT_CENTER, if ac { t("Plugged in") } else { t("Unplugged") }, reg(11.0), DIM);
                    }
                    None => {
                        ring(p, c, 28.0, 6.0, 1.0, GREEN);
                        bolt(p, c, 11.0, WHITE);
                        p.text(pos2(r.left() + 92.0, r.top() + 62.0), Align2::LEFT_CENTER, t("Plugged in"), bold(14.0), WHITE);
                        p.text(pos2(r.left() + 92.0, r.top() + 82.0), Align2::LEFT_CENTER, t("No battery"), reg(11.0), DIM);
                    }
                }
            }
            2 => {
                title(t("NEXT PRAYER"));
                if self.s.prayer && !self.prayer_placed() {
                    p.text(pos2(r.left() + 14.0, r.top() + 60.0), Align2::LEFT_CENTER, t("No location yet"), bold(16.0), WHITE);
                    p.text(pos2(r.left() + 14.0, r.top() + 80.0), Align2::LEFT_CENTER, t("Find it in Settings › Prayer times"), reg(12.0), DIM);
                } else if self.s.prayer {
                    let t = self.prayer_times(today);
                    let m = self.now_min();
                    let (name, at) = match t.iter().position(|&x| x > m) {
                        Some(i) => (i18n::t(prayer::NAMES[i]), t[i]),
                        None => (i18n::t(prayer::NAMES[0]), self.prayer_times(today + Duration::days(1))[0]),
                    };
                    p.text(pos2(r.left() + 14.0, r.top() + 56.0), Align2::LEFT_CENTER, name, bold(26.0), WHITE);
                    p.text(pos2(r.left() + 14.0, r.top() + 88.0), Align2::LEFT_CENTER, hm(at), bold(16.0), BLUE);
                    p.text(pos2(r.left() + 14.0, r.top() + 108.0), Align2::LEFT_CENTER, self.city_name(), reg(11.0), DIM);
                } else {
                    p.text(pos2(r.left() + 14.0, r.top() + 60.0), Align2::LEFT_CENTER, t("Prayer times"), bold(16.0), WHITE);
                    p.text(pos2(r.left() + 14.0, r.top() + 80.0), Align2::LEFT_CENTER, t("are off"), reg(12.0), DIM);
                }
            }
            3 => {
                title(t("FOCUS TODAY"));
                p.text(pos2(r.left() + 14.0, r.top() + 58.0), Align2::LEFT_CENTER, dur_text(self.focus_today()), bold(24.0), WHITE);
                let days: Vec<u32> = (0..7).rev().map(|i| self.focus_log.get(&(today - Duration::days(i))).copied().unwrap_or(0)).collect();
                let max = days.iter().copied().max().unwrap_or(0).max(1) as f32;
                for (k, v) in days.iter().enumerate() {
                    let h = 30.0 * *v as f32 / max;
                    let x = r.left() + 16.0 + k as f32 * 26.0;
                    p.rect_filled(Rect::from_min_max(pos2(x, r.bottom() - 16.0 - h.max(2.0)), pos2(x + 14.0, r.bottom() - 16.0)), 3.0, if k == 6 { PURPLE } else { Color32::from_gray(50) });
                }
            }
            4 => {
                title(t("WORLD CLOCK"));
                for (k, idx) in [self.s.widget_city1, self.s.widget_city2].into_iter().enumerate() {
                    let (name, _, _) = WORLD[idx.min(WORLD.len() - 1)];
                    let y = r.top() + 46.0 + k as f32 * 40.0;
                    let there = world_time(idx);
                    p.text(pos2(r.left() + 14.0, y), Align2::LEFT_CENTER, name, reg(13.0), DIM);
                    // How far that clock is from this PC's, e.g. "+1 h", "−6 h", "same time".
                    p.text(pos2(r.left() + 14.0, y + 15.0), Align2::LEFT_CENTER, offset_text((there - self.now).num_minutes()), reg(10.0), FAINT);
                    p.text(pos2(r.right() - 14.0, y + 6.0), Align2::RIGHT_CENTER, there.format("%H:%M").to_string(), bold(22.0), WHITE);
                }
            }
            5 => {
                title(t("MOON"));
                let (phase, lit) = moon(Utc::now().naive_utc());
                let c = pos2(r.left() + 46.0, r.top() + 72.0);
                p.circle_filled(c, 24.0, Color32::from_gray(45));
                p.circle_filled(c + vec2(48.0 * (1.0 - lit as f32) * if phase < 4 { -1.0 } else { 1.0 }, 0.0), 24.0, CARD);
                p.circle_stroke(c, 24.0, Stroke::new(1.0_f32, Color32::from_gray(70)));
                p.text(pos2(r.left() + 84.0, r.top() + 62.0), Align2::LEFT_CENTER, t(MOON[phase]), bold(13.0), WHITE);
                p.text(pos2(r.left() + 84.0, r.top() + 82.0), Align2::LEFT_CENTER, tr("{}% lit", &[&format!("{:.0}", lit * 100.0)]), reg(12.0), DIM);
            }
            6 => {
                title(t("COUNTDOWN"));
                let ev = self.s.widget_event.min(EVENTS.len() - 1);
                // "My own date" counts to the person's date, under the name they gave it.
                let (when, name) = if ev == MY_EVENT {
                    let name = self.s.widget_my_name.trim();
                    (my_date(today, &self.s.widget_my_date), if name.is_empty() { t(EVENTS[ev]).to_string() } else { name.to_string() })
                } else {
                    (next_event(today, ev), t(EVENTS[ev]).to_string())
                };
                let (big, small) = match when {
                    Some(d) if d == today => (t("Today").to_string(), name),
                    Some(d) => ((d - today).num_days().to_string(), tr("days to {}", &[&name])),
                    None if ev == MY_EVENT => ("—".into(), t("Set the date in Settings › Widgets").to_string()),
                    None => ("—".into(), name),
                };
                p.text(pos2(r.left() + 14.0, r.top() + 60.0), Align2::LEFT_CENTER, big, bold(32.0), WHITE);
                let g = fit_line(p, &small, reg(12.0), DIM, r.width() - 28.0);
                p.galley(pos2(r.left() + 14.0, r.top() + 96.0 - g.size().y / 2.0), g, DIM);
            }
            WEATHER_WIDGET => self.weather_widget(p, r),
            _ => {
                title(t("WATER"));
                let c = pos2(r.left() + 50.0, r.top() + 72.0);
                ring(p, c, 28.0, 6.0, self.drank as f32 / self.s.goal.max(1) as f32, BLUE);
                drop_icon(p, c, 10.0, BLUE);
                p.text(pos2(r.left() + 92.0, r.top() + 60.0), Align2::LEFT_CENTER, format!("{:.1} L", self.drank as f32 / 1000.0), bold(22.0), WHITE);
                p.text(pos2(r.left() + 92.0, r.top() + 84.0), Align2::LEFT_CENTER, tr("of {} L", &[&litres(self.s.goal)]), reg(12.0), DIM);
                // The last 7 days against the goal (a full bar = goal reached); today in blue.
                for (k, ml) in self.water_week().iter().enumerate() {
                    let h = 14.0 * (*ml as f32 / self.s.goal.max(1) as f32).min(1.0);
                    let x = r.left() + 92.0 + k as f32 * 13.0;
                    let base = r.top() + 112.0;
                    p.rect_filled(Rect::from_min_max(pos2(x, base - 14.0), pos2(x + 8.0, base)), 2.0, Color32::from_gray(38));
                    p.rect_filled(Rect::from_min_max(pos2(x, base - h.max(1.5)), pos2(x + 8.0, base)), 2.0, if k == 6 { BLUE } else { Color32::from_rgb(52, 98, 160) });
                }
            }
        }
    }

    /// The date, the weather and a soft light in the sky's colour (MET Norway's next-hour symbol).
    pub(crate) fn weather_widget(&self, p: &egui::Painter, r: Rect) {
        let w = self.weather.lock().unwrap().clone();
        let (sky, label) = w.as_ref().map_or((Color32::from_gray(70), ""), |w| sky(&w.symbol));
        // The light: soft rings fading out from the top right, kept inside the card.
        let glow = p.with_clip_rect(r.intersect(p.clip_rect()));
        let c = r.right_top() + vec2(-30.0, 26.0);
        for k in 0..32 {
            let f = k as f32 / 32.0;
            glow.circle_filled(c, 130.0 * (1.0 - f) + 6.0, sky.gamma_multiply(0.025 + 0.02 * f));
        }
        p.text(r.min + vec2(14.0, 15.0), Align2::LEFT_CENTER, t("WEATHER"), bold(11.0), DIM);
        p.text(pos2(r.right() - 14.0, r.top() + 15.0), Align2::RIGHT_CENTER, i18n::date(self.now).rsplit_once(' ').map_or(String::new(), |d| d.0.to_string()), reg(11.0), DIM);
        let place = if self.s.weather_city == prayer::here_index() { t("My location").to_string() } else { prayer::place_name(self.s.weather_city).1.to_string() };
        match w {
            Some(w) => {
                weather_icon(p, pos2(r.left() + 40.0, r.top() + 66.0), &w.symbol, sky);
                let deg = |c: f64| if self.s.fahrenheit { format!("{:.0}°", c * 9.0 / 5.0 + 32.0) } else { format!("{c:.0}°") };
                p.text(pos2(r.left() + 78.0, r.top() + 58.0), Align2::LEFT_CENTER, deg(w.temp), bold(32.0), WHITE);
                p.text(pos2(r.left() + 78.0, r.top() + 86.0), Align2::LEFT_CENTER, t(label), reg(13.0), WHITE);
                // Today's high and low, and the rain to come in the next 6 hours (when there is some).
                if let (Some(hi), Some(lo)) = (w.high, w.low) {
                    p.text(pos2(r.right() - 14.0, r.top() + 52.0), Align2::RIGHT_CENTER, format!("↑{} ↓{}", deg(hi), deg(lo)), reg(12.0), WHITE);
                }
                if let Some(mm) = w.rain_6h.filter(|&mm| mm >= 0.1) {
                    p.text(pos2(r.right() - 14.0, r.top() + 70.0), Align2::RIGHT_CENTER, tr("Rain {} mm", &[&format!("{mm:.1}")]), reg(11.0), Color32::from_rgb(147, 197, 253));
                }
            }
            None => {
                let why = if self.s.weather_city == prayer::here_index() && (self.s.prayer_lat, self.s.prayer_lon) == (0.0, 0.0) { t("Find my location in Settings") } else { t("Getting the weather…") };
                p.text(pos2(r.left() + 14.0, r.top() + 66.0), Align2::LEFT_CENTER, why, reg(13.0), DIM);
            }
        }
        p.text(pos2(r.left() + 14.0, r.bottom() - 14.0), Align2::LEFT_CENTER, ellipsize(&place, 24), reg(11.0), DIM);
        p.text(pos2(r.right() - 14.0, r.bottom() - 14.0), Align2::RIGHT_CENTER, "MET Norway", reg(9.0), FAINT);
    }

    /// Ask Nusa: a sum ending in "=" answers as you type; questions go to Gemini (needs a key).
    pub(crate) fn page_ask(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = ui.painter().clone();
        p.text(pos2(r.left() + 22.0, r.top() + 24.0), Align2::LEFT_CENTER, t("Ask Nusa"), bold(17.0), WHITE);
        if pill_button(ui, Rect::from_min_size(pos2(r.right() - 76.0, r.top() + 12.0), vec2(56.0, 24.0)), t("Close"), BTN, "aclose") {
            self.ask_open = false;
            return;
        }
        let field = Rect::from_min_size(pos2(r.left() + 22.0, r.top() + 48.0), vec2(r.width() - 44.0, 34.0));
        p.rect_filled(field, 17.0, CARD);
        settings_style(ui);
        let edit = ui.put(
            field.shrink2(vec2(14.0, 7.0)),
            // lock_focus: Tab stays in the box (a one-line box adds no tab character) and translates.
            egui::TextEdit::singleline(&mut self.ask_text).hint_text(t("Ask anything, or a sum like 12*3=")).frame(false).font(reg(15.0)).lock_focus(true),
        );
        if std::mem::take(&mut self.ask_focus) {
            edit.request_focus();
        }
        let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let tab = edit.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Tab));
        if edit.changed() {
            self.ask_gen += 1; // the question changed: an answer still on its way is for the old one
            self.ask_waiting = false;
            self.ask_copyable = false;
        }
        // Gemini's answer to the latest question.
        if self.ask_waiting {
            let reply = self.ask_reply.lock().unwrap().clone();
            if reply.0 == self.ask_gen {
                self.ask_waiting = false;
                self.ask_answer = reply.1;
                self.ask_copyable = reply.2;
            }
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.ask_open = false;
            self.held = false;
            return;
        }
        // "100usd to vnd" on Enter reads as "100usd to vnd=".
        if enter && calc::currency(&self.ask_text).is_some() && !self.ask_text.trim_end().ends_with('=') {
            self.ask_text.push('=');
        }
        let q = self.ask_text.trim();
        if let Some((amount, from, to)) = calc::currency(q).filter(|_| q.ends_with('=')) {
            // Asked again each frame, so the answer appears as soon as the rates arrive.
            self.ask_answer = match online::convert(&self.rates, amount, &from, &to) {
                Some(v) => format!("= {} {to}", calc::money(v)),
                None => match online::rates_state(&self.rates) {
                    online::RatesState::Coming => t("Getting today's rates…").to_string(),
                    online::RatesState::Offline => t("Rates need the internet — try again in a minute").to_string(),
                    online::RatesState::Here => tr("Unknown currency code: {} or {}", &[&from, &to]),
                },
            };
            if enter {
                edit.request_focus();
            }
        } else if tab && !q.is_empty() {
            let rule = format!(
                "Translate the user's text into {}. If it is already in that language, translate it into English. Reply with the translation only.",
                i18n::ENGLISH_NAMES[i18n::lang()]
            );
            self.ask_gemini(&rule, q.to_string());
        } else if edit.changed() || enter {
            match calc::eval(q) {
                // A sum answers once it ends with "=", or on Enter.
                Some(v) if q.ends_with('=') || enter => self.ask_answer = format!("= {}", calc::show(v)),
                None if enter && !q.is_empty() => {
                    let rule = "You answer inside a small notch on a Windows PC. Answer in at most three short sentences, \
                        in plain text with no Markdown, in the language of the question.";
                    self.ask_gemini(rule, q.to_string());
                }
                _ => self.ask_answer.clear(),
            }
        }
        if enter || tab {
            edit.request_focus();
            // Kept (in memory only) for ↑ / ↓, the newest last, without repeating the last one.
            let asked = self.ask_text.trim().to_string();
            if !asked.is_empty() && self.ask_history.last() != Some(&asked) {
                self.ask_history.push(asked);
                if self.ask_history.len() > ASK_HISTORY_MAX {
                    self.ask_history.remove(0);
                }
            }
            self.ask_hist_at = None;
        }
        // ↑ brings back the questions asked before, one by one; ↓ goes forward again, to an empty box.
        let (up, down) = ui.input(|i| (i.key_pressed(egui::Key::ArrowUp), i.key_pressed(egui::Key::ArrowDown)));
        if edit.has_focus() && (up || down) && !self.ask_history.is_empty() {
            let last = self.ask_history.len() - 1;
            self.ask_hist_at = match (self.ask_hist_at, up) {
                (None, true) => Some(last),
                (Some(k), true) => Some(k.saturating_sub(1)),
                (Some(k), false) if k < last => Some(k + 1),
                _ => None,
            };
            self.ask_text = self.ask_hist_at.map_or_else(String::new, |k| self.ask_history[k].clone());
            // The cursor goes to the end of the text brought back.
            if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), edit.id) {
                let end = egui::text::CCursor::new(self.ask_text.chars().count());
                state.cursor.set_char_range(Some(egui::text::CCursorRange::one(end)));
                state.store(ui.ctx(), edit.id);
            }
            self.ask_gen += 1;
            self.ask_waiting = false;
            self.ask_copyable = false;
            self.ask_answer.clear();
        }
        if !self.ask_answer.is_empty() {
            // A real answer (a sum, a conversion, Gemini's reply) can be copied with one click.
            let copyable = self.ask_answer.starts_with('=') || self.ask_copyable;
            if copyable {
                let done = self.copied.as_ref().is_some_and(|(c, at)| *c == answer_text(&self.ask_answer) && at.elapsed().as_secs() < 2);
                let label = if done { t("Copied") } else { t("Copy") };
                if pill_button(ui, Rect::from_min_size(pos2(r.right() - 92.0, r.top() + 96.0), vec2(66.0, 24.0)), label, BTN, "acopy") {
                    let text = answer_text(&self.ask_answer);
                    if let Some(c) = self.clipboard.as_mut() {
                        let _ = c.set_text(text.clone());
                    }
                    self.last_clip = text.clone();
                    self.copied = Some((text, Instant::now()));
                }
            }
            let text_right = if copyable { r.right() - 104.0 } else { r.right() - 30.0 };
            if self.ask_answer.starts_with('=') {
                let g = fit_line(&p, &self.ask_answer, bold(30.0), WHITE, text_right - (r.left() + 30.0));
                p.galley(pos2(r.left() + 30.0, r.top() + 116.0 - g.size().y / 2.0), g, WHITE);
            } else {
                // Gemini's answers wrap and scroll, and can be selected to copy.
                let area = Rect::from_min_max(pos2(r.left() + 30.0, r.top() + 92.0), pos2(text_right, r.bottom() - 28.0));
                let col = if self.ask_waiting { DIM } else { WHITE };
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(area), |ui| {
                    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                        ui.add(egui::Label::new(egui::RichText::new(&self.ask_answer).font(reg(14.0)).color(col)).wrap());
                    });
                });
            }
        }
        p.text(pos2(r.center().x, r.bottom() - 14.0), Align2::CENTER_CENTER, t("Enter to ask · Tab to translate · Esc to close"), reg(11.0), DIM);
    }

    /// Sends `question` to Gemini on its own thread; the answer shows when it comes back.
    pub(crate) fn ask_gemini(&mut self, rule: &str, question: String) {
        self.ask_copyable = false;
        if self.gemini_key.trim().is_empty() {
            self.ask_answer = t("Ask Nusa needs a Gemini key — add one in Settings").to_string();
            return;
        }
        self.ask_gen += 1;
        self.ask_waiting = true;
        self.ask_answer = t("Thinking…").to_string();
        let (n, key, rule, out) = (self.ask_gen, self.gemini_key.trim().to_string(), rule.to_string(), self.ask_reply.clone());
        std::thread::spawn(move || {
            let reply = online::gemini(&key, &rule, &question);
            let ok = reply.is_ok();
            let answer = match reply {
                Ok(a) => a,
                Err(online::AskError::Key) => t("Gemini turned the key down — check it in Settings").to_string(),
                Err(online::AskError::Busy) => t("Too many questions just now — try again in a minute").to_string(),
                Err(online::AskError::Offline) => t("Couldn't reach Gemini — check the internet").to_string(),
                Err(online::AskError::Empty) => t("Gemini gave no answer to that").to_string(),
            };
            *out.lock().unwrap() = (n, answer, ok);
            tray::wake();
        });
    }

    pub(crate) fn page_settings(&mut self, ui: &mut egui::Ui, r: Rect) {
        let before = self.s.clone();
        let locating = self.locating.clone();
        let (gemini, ical, ical_url) = (&mut self.gemini_key, &mut self.ical_text, self.ical_url.clone());
        let cal = self.calendar.lock().unwrap().clone();
        let reset_at = &mut self.settings_reset;
        let inner = Rect::from_min_max(pos2(r.left() + 18.0, r.top() + 44.0), pos2(r.right() - 18.0, r.bottom() - 10.0));
        settings_style(ui);
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(inner), |ui| {
            let s = &mut self.s;
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.set_width(ui.available_width() - 8.0); // room for the scroll bar
                card(ui, "General", |ui| {
                    combo(ui, "Language", &mut s.language, &i18n::NAMES);
                    switch_row(ui, "Start with Windows", &mut s.startup);
                    combo(ui, "Size", &mut s.size, &SIZE_NAMES);
                    switch_row(ui, "Show the date and time — the clock in the notch and on each page", &mut s.show_clock);
                    combo(ui, "Tray icon", &mut s.app_icon, &tray::ICON_NAMES);
                    switch_row(ui, "Minimal notch — shrinks to a thin bar when nothing is going on", &mut s.minimal);
                    switch_row(ui, "Mascot — a little friend that lives in the notch", &mut s.mascot);
                    pick(ui, "Open the notch on", &mut s.open_click, &[false, true], &["Hover", "Click"]);
                    // The shortcuts, and whether another app has taken one.
                    let taken = tray::hotkeys_taken();
                    row(ui, "Shortcuts — Ctrl+Alt+Space asks Nusa, Ctrl+Alt+T copies text from the screen", |ui| {
                        let (text, col) = match taken {
                            None => (String::new(), DIM), // not registered yet
                            Some(0) => (t("Ready").to_string(), GREEN),
                            Some(b) => (tr("Taken: {}", &[&taken_hint(b).body]), RED),
                        };
                        ui.add(egui::Label::new(egui::RichText::new(text).font(reg(12.0)).color(col)).wrap());
                    });
                    combo(ui, "Status ring", &mut s.status_ring, &["Water", "Tasks done today", "No ring"]);
                    switch_row(ui, "Reminder sounds — chimes for Pomodoro, To-do and Water", &mut s.sound);
                });
                card(ui, "Pomodoro", |ui| {
                    number(ui, "Focus", &mut s.focus, 1..=120, " min");
                    number(ui, "Short break", &mut s.short_break, 1..=60, " min");
                    number(ui, "Long break", &mut s.long_break, 1..=60, " min");
                    switch_row(ui, "Go on by itself — the next break or focus starts without a click", &mut s.auto_next);
                    switch_row(ui, "Quiet focus — notifications wait until focus ends", &mut s.focus_quiet);
                });
                card(ui, "Water reminders", |ui| {
                    pick(ui, "\"Snooze\" waits", &mut s.water_snooze, &SNOOZES, &["5 min", "10 min", "15 min"]);
                });
                card(ui, "To-do reminders", |ui| {
                    switch_row(ui, "When a task starts", &mut s.todo_start);
                    switch_row(ui, "When time is up", &mut s.todo_end);
                    pick(ui, "\"Later\" waits", &mut s.snooze, &SNOOZES, &["5 min", "10 min", "15 min"]);
                });
                card(ui, "Prayer times", |ui| {
                    switch_row(ui, "Show prayer times — worked out on this PC, for any city", &mut s.prayer);
                    ui.add_enabled_ui(s.prayer, |ui| {
                        let names = place_names();
                        combo(ui, "City", &mut s.prayer_city, &names.iter().map(String::as_str).collect::<Vec<_>>());
                        if s.prayer_city == prayer::here_index() {
                            let state = *locating.lock().unwrap();
                            let label = match state {
                                Locating::Busy => t("Finding where you are…").to_string(),
                                Locating::Failed => t("Couldn't find your location — check Windows Settings › Privacy › Location, or pick a city").to_string(),
                                _ if s.prayer_lat == 0.0 && s.prayer_lon == 0.0 => t("Not found yet").to_string(),
                                _ => format!("{:.2}, {:.2}", s.prayer_lat, s.prayer_lon),
                            };
                            row(ui, "Find my location", |ui| {
                                if move_button(ui, "⌖", t("Find my location")) {
                                    find_location(&locating);
                                }
                                ui.add_space(8.0);
                                ui.label(egui::RichText::new(label).font(reg(12.0)).color(DIM));
                            });
                        }
                        let mut methods: Vec<&str> = vec![t("Usual for the city")];
                        methods.extend(prayer::METHODS.iter().map(|m| m.name));
                        combo(ui, "Method", &mut s.prayer_method, &methods);
                        combo(ui, "Asr time", &mut s.prayer_asr, &["As the method does", "Shafi'i (shadow ×1)", "Hanafi (shadow ×2)"]);
                        switch_row(ui, "Remind me at prayer time", &mut s.prayer_reminder);
                        ui.add_enabled_ui(s.prayer_reminder, |ui| switch_row(ui, "Also 10 minutes before", &mut s.prayer_early));
                        switch_row(ui, "Pause media at prayer time", &mut s.prayer_pause_media);
                        ui.add_enabled_ui(s.prayer_pause_media, |ui| switch_row(ui, "Play it again 10 minutes later", &mut s.prayer_resume_media));
                        row_label(ui, "Adjust — minutes added to each time", 40.0);
                        ui.columns(prayer::NAMES.len(), |cols| {
                            for (i, c) in cols.iter_mut().enumerate() {
                                c.vertical_centered(|c| {
                                    c.label(egui::RichText::new(i18n::t(prayer::NAMES[i])).font(reg(12.0)).color(DIM));
                                    c.add(egui::DragValue::new(&mut s.prayer_adj[i]).range(-10..=10));
                                });
                            }
                        });
                        ui.add_space(8.0);
                    });
                });
                card(ui, "Shelf", |ui| {
                    switch_row(ui, "Start with an empty Shelf", &mut s.shelf_clear);
                    switch_row(ui, "Clipboard history — text you copy, ready to paste again", &mut s.clipboard);
                    ui.add_enabled_ui(s.clipboard, |ui| {
                        number(ui, "Keep up to", &mut s.clipboard_max, 1..=CLIP_MAX_FREE, " items");
                        switch_row(ui, "Remember after restart", &mut s.clipboard_keep);
                    });
                });
                card(ui, "Popups", |ui| {
                    switch_row(ui, "Volume", &mut s.hud_volume);
                    switch_row(ui, "Brightness", &mut s.hud_brightness);
                    switch_row(ui, "Caps Lock", &mut s.hud_caps);
                    switch_row(ui, "Input language", &mut s.hud_language);
                    switch_row(ui, "Battery", &mut s.popup_battery);
                    switch_row(ui, "Wi-Fi", &mut s.popup_wifi);
                    switch_row(ui, "Bluetooth headset", &mut s.popup_bluetooth);
                    switch_row(ui, "Now playing — when the song changes", &mut s.popup_song);
                    switch_row(ui, "Hide in full screen — games, videos and slides that fill the screen", &mut s.hide_fullscreen);
                });
                card(ui, "Notifications", |ui| {
                    switch_row(ui, "Show notifications — new Windows notifications appear here (after restart)", &mut s.notifications);
                    ui.add_enabled_ui(s.notifications, |ui| switch_row(ui, "Show the message text", &mut s.notification_preview));
                    switch_row(ui, "Today page — recent notifications and your week", &mut s.today_page);
                });
                card(ui, "Music", |ui| {
                    switch_row(ui, "Show lyrics — from LRCLIB, a community lyrics database (after restart)", &mut s.lyrics);
                });
                card(ui, "Ask Nusa", |ui| {
                    // Kept sealed with Windows DPAPI, never in settings.txt.
                    if secret_row(ui, "Gemini key — free at aistudio.google.com/apikey; kept sealed on this PC", gemini) {
                        write_secret(GEMINI_FILE, gemini.trim());
                    }
                });
                card(ui, "Google Calendar", |ui| {
                    if secret_row(ui, "Secret iCal address — Google Calendar › Settings › your calendar › Integrate calendar", ical) {
                        write_secret(ICAL_FILE, ical.trim());
                        *ical_url.lock().unwrap() = ical.trim().to_string();
                    }
                    let state = match cal.status {
                        _ if ical.trim().is_empty() => t("Today's events show in To-do and remind you when they start").to_string(),
                        _ if !ical.trim().starts_with("https://") && !ical.trim().starts_with("webcal://") => t("The address should start with https://").to_string(),
                        calendar::Status::Ready => tr("{} events today", &[&cal.events.len()]),
                        calendar::Status::Failed => t("Couldn't read the calendar — check the address and the internet").to_string(),
                        _ => t("Reading the calendar…").to_string(),
                    };
                    ui.label(egui::RichText::new(state).font(reg(12.0)).color(DIM));
                    switch_row(ui, "Also 10 minutes before", &mut s.cal_early);
                });
                card(ui, "Pages", |ui| {
                    // Reorder with the arrows, leave pages out with the switch; one always stays.
                    let order = s.page_order.clone();
                    let shown = order.iter().filter(|&&p| !s.pages_hidden.contains(&p) && (p != TODAY_PAGE || s.today_page)).count();
                    for (i, &pg) in order.iter().enumerate() {
                        row(ui, PAGES[pg], |ui| {
                            let mut on = if pg == TODAY_PAGE { s.today_page } else { !s.pages_hidden.contains(&pg) };
                            let was = on;
                            if switch(ui, &mut on, PAGES[pg]) && !(was && shown == 1) {
                                if pg == TODAY_PAGE {
                                    s.today_page = on;
                                } else if on {
                                    s.pages_hidden.retain(|&p| p != pg);
                                } else {
                                    s.pages_hidden.push(pg);
                                }
                            }
                            ui.add_space(8.0);
                            if i + 1 < order.len() && move_button(ui, "↓", t("Move down")) {
                                s.page_order.swap(i, i + 1);
                            }
                            if i > 0 && move_button(ui, "↑", t("Move up")) {
                                s.page_order.swap(i, i - 1);
                            }
                        });
                    }
                });
                card(ui, "Widgets", |ui| {
                    combo(ui, "Left widget", &mut s.widget_left, &WIDGETS);
                    combo(ui, "Right widget", &mut s.widget_right, &WIDGETS);
                    if s.widget_left == WEATHER_WIDGET || s.widget_right == WEATHER_WIDGET {
                        let names = place_names();
                        combo(ui, "Weather for", &mut s.weather_city, &names.iter().map(String::as_str).collect::<Vec<_>>());
                        switch_row(ui, "Degrees in °F", &mut s.fahrenheit);
                    }
                    let cities: Vec<&str> = WORLD.iter().map(|w| w.0).collect();
                    combo(ui, "World clock, first city", &mut s.widget_city1, &cities);
                    combo(ui, "World clock, second city", &mut s.widget_city2, &cities);
                    combo(ui, "Countdown to", &mut s.widget_event, &EVENTS);
                    if s.widget_event == MY_EVENT {
                        text_row(ui, "Name — what you count down to", &mut s.widget_my_name, t("Birthday, exam…"));
                        text_row(ui, "Date — DD/MM every year, or DD/MM/YYYY once", &mut s.widget_my_date, "31/12");
                    }
                    pick(ui, "Week starts on", &mut s.week_monday, &[false, true], &["Sunday", "Monday"]);
                });
                card(ui, "Data", |ui| {
                    // Everything the app keeps lives in one folder (to back up, or carry to another PC).
                    row(ui, "Data folder — settings, water, tasks and history", |ui| {
                        if ui.add(egui::Button::new(t("Open")).corner_radius(12.0)).clicked() {
                            let _ = std::process::Command::new("explorer.exe").arg(dir()).spawn();
                        }
                    });
                    // Two clicks within RESET_CONFIRM_SECS, as the Water page's Reset.
                    let armed = reset_at.is_some_and(|at| at.elapsed().as_secs_f32() < RESET_CONFIRM_SECS);
                    row(ui, "Restore default settings — water, tasks, history, keys and the calendar stay", |ui| {
                        let label = if armed { t("Sure?") } else { t("Restore") };
                        let button = egui::Button::new(egui::RichText::new(label).color(WHITE)).corner_radius(12.0).fill(if armed { RED } else { BTN });
                        if ui.add(button).clicked() {
                            if armed {
                                *s = default_settings_keeping(s);
                                *reset_at = None;
                            } else {
                                *reset_at = Some(Instant::now());
                            }
                        }
                    });
                });
                ui.add_space(8.0);
            });
        });
        if self.s != before {
            if self.s.startup != before.startup && !set_startup(self.s.startup) {
                self.s.startup = before.startup; // Windows didn't take it: show it as it really is
                self.alert = Some(Alert { title: t("Couldn't change Start with Windows").into(), body: t("Windows didn't allow it — try again").into(), kind: Kind::Hint, at: Instant::now() });
            }
            if self.s.language != before.language {
                i18n::set(self.s.language);
                setup_fonts(ui.ctx());
                self.s.language = i18n::lang();
            }
            if self.s.app_icon != before.app_icon && self.shot.is_none() {
                tray::set_icon(self.s.app_icon);
            }
            self.clips.truncate(self.s.clipboard_max);
            self.save_clips();
            if self.mode == Mode::Focus && !self.running && self.s.focus != before.focus {
                self.left = self.mode_len(Mode::Focus);
            }
            self.s.save();
        }
    }
}
