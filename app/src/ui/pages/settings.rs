//! The Settings page.
use super::*;

impl App {
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
