//! The Water page, its hourly chart and its settings.
use super::*;

impl App {
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
}
