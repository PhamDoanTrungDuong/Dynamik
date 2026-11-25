//! The Pomodoro page.
use super::*;

impl App {
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
}
