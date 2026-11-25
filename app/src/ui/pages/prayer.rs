//! The Prayer page: next prayer, sun arc and Hijri date.
use super::*;

impl App {
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
}
