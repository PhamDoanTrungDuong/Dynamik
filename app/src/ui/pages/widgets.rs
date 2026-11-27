//! The Widgets page and every widget.
use super::*;

impl App {
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
                        // On battery: how long it lasts, when Windows has an estimate.
                        let sub = match battery_minutes_left() {
                            Some(m) if !ac => tr("{} left", &[&dur_text(m)]),
                            _ => if ac { t("Plugged in") } else { t("Unplugged") }.to_string(),
                        };
                        p.text(pos2(r.left() + 92.0, r.top() + 93.0), Align2::LEFT_CENTER, sub, reg(11.0), DIM);
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
}
