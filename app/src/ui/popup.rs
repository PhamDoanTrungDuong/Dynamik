//! The resting notch and the pop-ups that drop out of it.
use super::*;

impl App {
    /// Volume, Caps Lock and keyboard-language changes show a short indicator, unless a pop-up
    /// that needs an answer is up.
    pub(crate) fn watch_indicators(&mut self) {
        let free = self.alert.as_ref().is_none_or(|a| matches!(a.kind, Kind::Hud(..) | Kind::Info | Kind::Battery(..) | Kind::Bluetooth(_) | Kind::Notice(..) | Kind::Song));
        let vol = *self.volume.lock().unwrap();
        if let (Some((v, m)), Some((ov, om))) = (vol, self.last_volume) {
            if self.s.hud_volume && free && (m != om || (v - ov).abs() > 0.004) {
                let kind = if m { HudKind::Muted } else { HudKind::Volume };
                self.alert = Some(Alert { title: t("Volume").into(), body: String::new(), kind: Kind::Hud(kind, v), at: Instant::now() });
            }
        }
        if vol.is_some() {
            self.last_volume = vol;
        }
        let (seq, level) = tray::brightness();
        if seq != self.last_bright {
            self.last_bright = seq;
            if self.s.hud_brightness && free {
                self.alert = Some(Alert { title: t("Brightness").into(), body: String::new(), kind: Kind::Hud(HudKind::Brightness, level), at: Instant::now() });
            }
        }
        // "Now playing": a new song while the notch is closed shows its cover and name for a moment.
        // The first song read at start is just noted (it isn't news).
        let song = self.media.now.lock().ok().map(|n| (n.active() && n.playing, format!("{}\u{1}{}", n.title, n.artist)));
        if let Some((playing, key)) = song {
            let news = self.last_song.as_ref().is_some_and(|k| *k != key) && playing;
            if news && self.s.popup_song && free && !self.expanded && self.started.elapsed().as_secs() > SONG_QUIET_START_SECS {
                let (title, artist) = key.split_once('\u{1}').map(|(a, b)| (a.to_string(), b.to_string())).unwrap_or_default();
                self.alert = Some(Alert { title, body: artist, kind: Kind::Song, at: Instant::now() });
            }
            if playing || self.last_song.is_none() {
                self.last_song = Some(key);
            }
        }
        // A headset that was not connected at the last look shows once; those connected at start do not.
        let heads = self.bluetooth.lock().unwrap().clone();
        if let Some(heads) = heads {
            let now: HashSet<u64> = heads.iter().map(|h| h.0).collect();
            let mut wait = false;
            if let Some(old) = &self.last_headsets {
                let gone = old.iter().find(|id| !now.contains(id)).and_then(|id| self.headset_names.get(id).cloned());
                if let Some((_, name, battery)) = heads.iter().find(|h| !old.contains(&h.0)) {
                    if self.s.popup_bluetooth && free {
                        let body = battery.map_or(t("Connected").to_string(), |b| tr("Connected · {}%", &[&b]));
                        self.alert = Some(Alert { title: name.clone(), body, kind: Kind::Bluetooth(*battery), at: Instant::now() });
                    }
                    // A pop-up waiting for an answer is up: show this one after it, not never.
                    wait = self.s.popup_bluetooth && !free;
                } else if let Some(name) = gone {
                    // One that went away (turned off, out of range, put in its case).
                    if self.s.popup_bluetooth && self.s.popup_headset_off && free {
                        self.alert = Some(Alert { title: name, body: t("Disconnected").into(), kind: Kind::Bluetooth(None), at: Instant::now() });
                    }
                    wait = self.s.popup_bluetooth && self.s.popup_headset_off && !free;
                }
            }
            // A headset's battery running low says so once, until it is charged again.
            let low = heads.iter().find(|h| h.2.is_some_and(|b| b <= HEADSET_LOW) && !self.headset_low_told.contains(&h.0)).cloned();
            self.headset_low_told.retain(|id| heads.iter().any(|h| h.0 == *id && h.2.is_some_and(|b| b <= HEADSET_LOW + 10)));
            if let (Some((id, name, battery)), false) = (low, wait) {
                if self.s.popup_bluetooth && self.s.popup_headset_off && free && self.alert.as_ref().is_none_or(|a| !matches!(a.kind, Kind::Bluetooth(_))) {
                    self.headset_low_told.insert(id);
                    self.alert = Some(Alert { title: name, body: tr("Headset battery low · {}%", &[&battery.unwrap_or(0)]), kind: Kind::Bluetooth(battery), at: Instant::now() });
                }
            }
            if !wait {
                self.headset_names = heads.iter().map(|h| (h.0, h.1.clone())).collect();
                self.last_headsets = Some(now);
            }
        }
        let new: Vec<media::Notice> = self.notices.lock().unwrap().drain(..).collect();
        // The Today page lists them, newest first; kept in memory only, so nothing private is written to disk.
        let ts = Utc::now().timestamp();
        if self.s.notifications {
            self.recent.splice(0..0, new.iter().rev().map(|n| (ts, n.clone())));
        } else {
            // Turned off while running (the watcher keeps going until restart): keep nothing.
            self.recent.clear();
        }
        self.recent.truncate(RECENT_MAX);
        let arrived = new.len() as u32;
        // While focusing (with "quiet focus" on) they wait, and the newest shows once focus ends.
        let quiet = self.s.focus_quiet && self.running && self.mode == Mode::Focus;
        let mut show = None;
        if let Some(last) = new.into_iter().last().filter(|_| self.s.notifications) {
            let waiting = self.held_notice.take().map_or(0, |(_, n)| n + 1);
            if quiet {
                self.held_notice = Some((last, waiting + arrived - 1));
            } else if free {
                show = Some((last, waiting + arrived - 1));
            } else if waiting > 0 {
                self.held_notice = Some((last, waiting + arrived - 1)); // still waiting behind a pop-up
            }
        } else if !quiet && self.alert.is_none() {
            show = self.held_notice.take();
        }
        if let Some(((app, title, body, id), more)) = show {
            let body = if self.s.notification_preview { body } else { String::new() };
            // The newest shows; "+N" counts the ones it covers (all still on the Today page).
            let covered = match &self.alert {
                Some(Alert { kind: Kind::Notice(_, _, more), .. }) => more + 1,
                _ => 0,
            };
            self.alert = Some(Alert { title, body, kind: Kind::Notice(app, id, covered + more), at: Instant::now() });
        }
        // A network change shows once it has held for 1.5 s, like the original's debounce.
        let net = self.network.lock().unwrap().clone();
        if let Some(net) = net {
            match &self.last_network {
                None => self.last_network = Some(net),
                Some(last) if *last == net => self.network_change = None,
                Some(_) => match &self.network_change {
                    Some((seen, at)) if *seen == net => {
                        if at.elapsed().as_secs_f32() >= NET_SETTLE_SECS {
                            self.network_change = None;
                            self.last_network = Some(net.clone());
                            if self.s.popup_wifi && free {
                                let body = [t("Disconnected"), t("No internet access"), t("Connected")][net.1.min(2) as usize];
                                self.alert = Some(Alert { title: net.0, body: body.into(), kind: Kind::Network(net.1), at: Instant::now() });
                            }
                        }
                    }
                    _ => self.network_change = Some((net, Instant::now())),
                },
            }
        }
        let caps = caps_lock();
        if caps != self.last_caps {
            self.last_caps = caps;
            if self.s.hud_caps && free {
                self.alert = Some(Alert { title: t("Caps Lock").into(), body: if caps { t("On") } else { t("Off") }.into(), kind: Kind::Hud(HudKind::Caps, caps as u8 as f32), at: Instant::now() });
            }
        }
        let lang = input_lang();
        if lang != self.last_lang && lang != 0 {
            self.last_lang = lang;
            if self.s.hud_language && free {
                self.alert = Some(Alert { title: t("Input language").into(), body: lang_name(lang), kind: Kind::Hud(HudKind::Lang, 0.0), at: Instant::now() });
            }
        }
    }

    /// Nothing to show: no session running or paused, no song playing, no mascot to look at.
    pub(crate) fn idle(&self) -> bool {
        !self.running && self.left >= self.mode_len(self.mode) && !self.s.mascot && !self.media.now.lock().is_ok_and(|n| n.playing)
    }

    pub(crate) fn draw_collapsed(&mut self, ui: &mut egui::Ui, r: Rect) {
        if self.target_size() == PEEK {
            return; // the bar itself is all there is
        }
        let y = r.center().y;
        let timer = self.running || self.left < self.mode_len(self.mode);
        // A song playing (and no session counting down): its cover on the left, the level bars in
        // its colour on the right, the clock between. It takes the mascot's place while it plays.
        let now = self.media.now.lock().map(|n| n.clone()).unwrap_or_default();
        if now.playing && !timer {
            let c = pos2(r.left() + 24.0, y);
            match self.art_texture(ui.ctx(), &now) {
                Some(tex) => {
                    egui::Image::new(&tex).corner_radius(6).paint_at(ui, Rect::from_center_size(c, vec2(22.0, 22.0)));
                }
                None => album_art(ui.painter(), Rect::from_center_size(c, vec2(22.0, 22.0))),
            }
            let p = ui.painter();
            if self.s.show_clock {
                p.text(r.center(), Align2::CENTER_CENTER, self.now.format("%H:%M").to_string(), bold(18.0), WHITE);
            }
            bars(p, pos2(r.right() - 24.0, y), false, self.music_tint());
            return;
        }
        let p = ui.painter();
        if self.s.mascot && !self.running && self.left >= self.mode_len(self.mode) {
            mascot(p, r);
            return;
        }
        if self.s.status_ring < 2 {
            self.status_ring(p, pos2(r.left() + 26.0, y), 9.0);
        }
        // A session's countdown always shows; the clock only with the clock switch on.
        if timer {
            p.text(r.center(), Align2::CENTER_CENTER, mmss(self.left), bold(18.0), PURPLE);
        } else if self.s.show_clock {
            p.text(r.center(), Align2::CENTER_CENTER, self.now.format("%H:%M").to_string(), bold(18.0), WHITE);
        }
        if !self.s.minimal {
            bars(p, pos2(r.right() - 26.0, y), self.running, self.music_tint());
        }
    }

    pub(crate) fn draw_popup(&mut self, ui: &mut egui::Ui, r: Rect) {
        let Some(a) = &self.alert else { return };
        let (title, body) = (a.title.clone(), a.body.clone());
        let since = a.at.elapsed().as_secs_f32();
        let p = ui.painter().clone();
        let icon = pos2(r.left() + 46.0, r.center().y);
        let tx = r.left() + 84.0;
        if !matches!(a.kind, Kind::Hud(..) | Kind::Notice(..) | Kind::Song) {
            p.text(pos2(tx, r.center().y - 9.0), Align2::LEFT_CENTER, &title, bold(17.0), WHITE);
            p.text(pos2(tx, r.center().y + 13.0), Align2::LEFT_CENTER, &body, reg(14.0), DIM);
        }
        let right = r.right() - 28.0;
        let mut close = false;
        match a.kind {
            Kind::Battery(pct, ac) => {
                ring(&p, icon, 14.0, 4.0, pct as f32 / 100.0, if ac || pct > 20 { GREEN } else { RED });
                bolt(&p, icon, 6.0, WHITE);
                p.text(pos2(right, r.center().y), Align2::RIGHT_CENTER, format!("{pct}%"), reg(28.0), WHITE);
            }
            Kind::Prayer(at, _) => {
                moon_icon(&p, icon, 11.0);
                p.text(pos2(right, r.center().y - 4.0), Align2::RIGHT_CENTER, hm(at), bold(17.0), WHITE);
                let bar = Rect::from_min_max(pos2(r.left() + 32.0, r.bottom() - 12.0), pos2(r.right() - 32.0, r.bottom() - 7.0));
                gradient_bar(&p, bar, 1.0, BLUE, PURPLE);
            }
            Kind::Event(at) => {
                calendar_icon(&p, icon, BLUE);
                p.text(pos2(right, r.center().y), Align2::RIGHT_CENTER, hm(at), bold(17.0), WHITE);
            }
            Kind::Drink | Kind::Refill => {
                drop_icon(&p, icon, 12.0, BLUE);
                let refill = matches!(a.kind, Kind::Refill);
                let first = if refill { t("Refilled") } else { t("Drank") };
                if pill_button(ui, Rect::from_min_size(pos2(right - 168.0, r.center().y - 15.0), vec2(84.0, 30.0)), first, BLUE_DARK, "p1") {
                    if refill {
                        // Refilled fills the bottle, then asks for the drink again (0x1400583ab).
                        self.in_bottle = self.s.bottle;
                        self.save_water();
                        let each = self.s.water_plan().each;
                        self.alert = Some(Alert { title: t("Time to Drink").into(), body: format!("{each} ml"), kind: Kind::Drink, at: Instant::now() });
                        return;
                    }
                    self.drink();
                    close = true;
                }
                if pill_button(ui, Rect::from_min_size(pos2(right - 78.0, r.center().y - 15.0), vec2(78.0, 30.0)), t("Snooze"), BTN, "p2") {
                    self.water_snooze = Some((self.now_min() + self.s.water_snooze).min(1439));
                    close = true;
                }
            }
            Kind::TaskStart(i) | Kind::TaskEnd(i) => {
                let end = matches!(a.kind, Kind::TaskEnd(_));
                check_circle(&p, icon, 13.0, false);
                if pill_button(ui, Rect::from_min_size(pos2(right - 152.0, r.center().y - 15.0), vec2(70.0, 30.0)), t("Done"), GREEN_DARK, "p1") {
                    if let Some(t) = self.tasks.get_mut(i) {
                        t.done_day = Some(self.now.date());
                    }
                    data::save_tasks(&self.tasks);
                    close = true;
                }
                if pill_button(ui, Rect::from_min_size(pos2(right - 76.0, r.center().y - 15.0), vec2(76.0, 30.0)), t("Later"), BTN, "p2") {
                    let m = self.now_min();
                    if let Some(t) = self.tasks.get_mut(i) {
                        t.snooze_until = m + self.s.snooze;
                        if end { t.fired_end = false } else { t.fired_start = false }
                    }
                    close = true;
                }
            }
            Kind::Hud(kind, level) => {
                let c = pos2(r.left() + 40.0, r.center().y);
                match kind {
                    HudKind::Volume | HudKind::Muted | HudKind::Brightness => {
                        if matches!(kind, HudKind::Brightness) {
                            sun_icon(&p, c, WHITE);
                        } else {
                            speaker_icon(&p, c, WHITE, matches!(kind, HudKind::Muted));
                        }
                        let bar = Rect::from_min_max(pos2(r.left() + 72.0, r.center().y - 3.0), pos2(r.right() - 74.0, r.center().y + 3.0));
                        let lv = if matches!(kind, HudKind::Muted) { 0.0 } else { level };
                        gradient_bar(&p, bar, lv, WHITE, WHITE);
                        p.text(pos2(r.right() - 26.0, r.center().y), Align2::RIGHT_CENTER, format!("{}", (lv * 100.0).round()), bold(17.0), WHITE);
                    }
                    HudKind::Caps | HudKind::Lang => {
                        let tag = if matches!(kind, HudKind::Caps) { "⇪".to_string() } else { body.clone() };
                        let box_ = Rect::from_center_size(c, vec2(34.0, 26.0));
                        p.rect_stroke(box_, 6.0, Stroke::new(1.6_f32, WHITE), egui::StrokeKind::Inside);
                        if matches!(kind, HudKind::Caps) {
                            p.add(Shape::convex_polygon(vec![c + vec2(0.0, -8.0), c + vec2(7.0, 0.0), c + vec2(-7.0, 0.0)], WHITE, Stroke::NONE));
                            p.rect_filled(Rect::from_center_size(c + vec2(0.0, 4.5), vec2(6.0, 7.0)), 1.0, WHITE);
                        } else {
                            p.text(c, Align2::CENTER_CENTER, &tag, bold(12.0), WHITE);
                        }
                        p.text(pos2(r.left() + 76.0, r.center().y), Align2::LEFT_CENTER, &title, bold(16.0), WHITE);
                        let col = if matches!(kind, HudKind::Caps) && level > 0.5 { GREEN } else { DIM };
                        p.text(pos2(r.right() - 26.0, r.center().y), Align2::RIGHT_CENTER, &body, bold(16.0), col);
                    }
                }
            }
            Kind::Network(level) => {
                // Signal arcs, grey when there's no connection.
                let col = if level == 2 { WHITE } else { DIM };
                for (k, rad) in [5.0f32, 10.0, 15.0].iter().enumerate() {
                    let pts: Vec<Pos2> = (0..=10).map(|i| {
                        let a = -PI * 0.75 + PI * 0.5 * i as f32 / 10.0;
                        icon + vec2(0.0, 8.0) + *rad * vec2(a.cos(), a.sin())
                    }).collect();
                    p.add(Shape::line(pts, Stroke::new(2.2_f32, if (k as u8) < level.max(1) * 2 { col } else { FAINT })));
                }
                p.circle_filled(icon + vec2(0.0, 8.0), 2.0, col);
            }
            Kind::Bluetooth(battery) => {
                // Blue ring holding the battery level around the Bluetooth rune; a headphone badge on the right.
                ring(&p, icon, 14.0, 4.0, battery.map_or(1.0, |b| b as f32 / 100.0), BLUE);
                bluetooth_rune(&p, icon, 6.0, WHITE);
                let badge = pos2(right - 16.0, r.center().y);
                p.circle_filled(badge, 20.0, Color32::from_rgb(12, 30, 44));
                p.circle_stroke(badge, 20.0, Stroke::new(1.5_f32, BLUE_DARK));
                headphone_icon(&p, badge, 8.0, WHITE);
            }
            Kind::Notice(ref app, ref id, more) => {
                // One calm line: the app's logo (or its initial in a dot of its own colour), who sent
                // it, then (preview on) what they said.
                let (app, id) = (app.clone(), id.clone());
                let y = r.center().y;
                let dot = pos2(r.left() + 30.0, y);
                // On arrival the icon pops in, and a cyan-to-violet light runs once along the bottom.
                let s = pop_in(since);
                if let Some(tex) = self.logo_texture(ui.ctx(), &id) {
                    // On a dark plate, so logos drawn for light backgrounds still show.
                    p.rect_filled(Rect::from_center_size(dot, vec2(26.0, 26.0) * s), 7.0 * s, Color32::from_gray(38));
                    egui::Image::new(&tex).corner_radius(5).paint_at(ui, Rect::from_center_size(dot, vec2(20.0, 20.0) * s));
                } else {
                    p.circle_filled(dot, 12.0 * s, app_color(&app));
                    let initial: String = app.chars().next().map(|c| c.to_uppercase().collect()).unwrap_or_default();
                    p.text(dot, Align2::CENTER_CENTER, initial, bold(13.0 * s), WHITE);
                }
                let u = since / NOTICE_SWEEP_SECS;
                if u < 1.0 {
                    let fade = (1.0 - u) * (u * 6.0).min(1.0);
                    let col = lerp_color(Color32::from_rgb(34, 211, 238), Color32::from_rgb(110, 50, 200), u).gamma_multiply(fade);
                    let cx = r.left() + 30.0 + (r.width() - 60.0) * u;
                    p.line_segment([pos2(cx - 50.0, r.bottom() - 2.5), pos2(cx + 50.0, r.bottom() - 2.5)], Stroke::new(2.5_f32, col));
                }
                // Line 1: who sent it, with the app's name dim on the right (a group's name alone
                // doesn't say which app). Line 2, when there is one: what they said, full width.
                let x = dot.x + 22.0;
                let mut right = r.right() - 20.0;
                let y1 = if body.is_empty() { y } else { y - 10.0 };
                if more > 0 {
                    // How many more came in while this one showed: a small pill at line 1's end.
                    let tag = p.layout_no_wrap(format!("+{more}"), bold(10.0), WHITE);
                    let badge = Rect::from_min_size(pos2(right - tag.size().x - 10.0, y1 - 7.5), vec2(tag.size().x + 10.0, 15.0));
                    p.rect_filled(badge, 7.5, BLUE);
                    p.galley(badge.center() - tag.size() / 2.0, tag, WHITE);
                    right = badge.left() - 8.0;
                }
                let name = p.layout_no_wrap(app.clone(), reg(11.0), DIM);
                let nw = if app.is_empty() || app == title { 0.0 } else { name.size().x + 10.0 };
                let two = !body.is_empty();
                let who = fit_line(&p, &title, bold(14.0), WHITE, right - x - nw);
                p.galley(pos2(x, y1 - who.size().y / 2.0), who, WHITE);
                if nw > 0.0 {
                    p.galley(pos2(right - name.size().x, y1 - name.size().y / 2.0), name, DIM);
                }
                if two {
                    let said = fit_line(&p, &body, reg(13.0), DIM, r.right() - 20.0 - x);
                    p.galley(pos2(x, y + 10.0 - said.size().y / 2.0), said, DIM);
                }
            }
            Kind::Song => {
                // The cover, the song and who sings it, and the level bars in the cover's colour.
                let now = self.media.now.lock().map(|n| n.clone()).unwrap_or_default();
                let cover = Rect::from_center_size(pos2(r.left() + 34.0, r.center().y), vec2(36.0, 36.0));
                match self.art_texture(ui.ctx(), &now) {
                    Some(tex) => {
                        egui::Image::new(&tex).corner_radius(8).paint_at(ui, cover);
                    }
                    None => album_art(&p, cover),
                }
                let x = cover.right() + 12.0;
                let room = r.right() - 52.0 - x;
                let song = fit_line(&p, &title, bold(14.0), WHITE, room);
                p.galley(pos2(x, r.center().y - 9.0 - song.size().y / 2.0), song, WHITE);
                let who = fit_line(&p, &body, reg(12.0), DIM, room);
                p.galley(pos2(x, r.center().y + 10.0 - who.size().y / 2.0), who, DIM);
                bars(&p, pos2(r.right() - 30.0, r.center().y), true, self.music_tint());
            }
            Kind::About | Kind::Hint => {
                // A violet dot beside the name and version (title and body are drawn above).
                ring(&p, icon, 14.0, 4.0, 1.0, PURPLE);
            }
            Kind::Info => {
                // The original waits here with one button that starts the next part (0x14000143f).
                ring(&p, icon, 14.0, 4.0, 1.0, PURPLE);
                let label = if self.mode == Mode::Focus { t("Focus") } else { t("Break") };
                if pill_button(ui, Rect::from_min_size(pos2(right - 84.0, r.center().y - 15.0), vec2(84.0, 30.0)), label, Color32::from_rgb(76, 29, 149), "phase") {
                    self.running = true;
                    self.last_tick = Instant::now();
                    close = true;
                }
            }
        }
        if close {
            self.alert = None;
        }
    }
}
