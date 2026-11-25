//! The To-do page, with today's Google Calendar events.
use super::*;

impl App {
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
}
