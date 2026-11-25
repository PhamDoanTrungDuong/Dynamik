//! The Today page: recent notifications and the week.
use super::*;

impl App {
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
}
