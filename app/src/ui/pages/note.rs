//! The Note page: quick notes to pin, finish, copy and drag out, or pick out and delete.
use super::*;

/// What a click on a note asks for, done after the list is drawn.
enum NoteAct {
    Copy(usize),
    Drag(String),
    Done(usize),
    Pin(usize),
    Delete(usize),
    Tick(usize),
}

const ROW_H: f32 = 28.0;

/// A push pin, leaning right: round head, short needle.
fn pin_icon(p: &egui::Painter, c: Pos2, col: Color32) {
    let head = c + vec2(1.8, -1.8);
    p.line_segment([head, c + vec2(-4.0, 4.0)], Stroke::new(1.4_f32, col));
    p.line_segment([head + vec2(-3.2, -0.6), head + vec2(0.6, 3.2)], Stroke::new(1.6_f32, col));
    p.circle_filled(head + vec2(1.2, -1.2), 2.6, col);
}

/// A square tick box for choosing notes.
fn tick_box(p: &egui::Painter, c: Pos2, on: bool) {
    let r = Rect::from_center_size(c, vec2(14.0, 14.0));
    if on {
        p.rect_filled(r, 4.0, BLUE);
        let st = Stroke::new(1.8_f32, WHITE);
        p.line_segment([c + vec2(-3.5, 0.0), c + vec2(-1.0, 2.5)], st);
        p.line_segment([c + vec2(-1.0, 2.5), c + vec2(3.5, -2.5)], st);
    } else {
        p.rect_stroke(r, 4.0, Stroke::new(1.5_f32, Color32::from_gray(110)), egui::StrokeKind::Inside);
    }
}

impl App {
    pub(crate) fn page_note(&mut self, ui: &mut egui::Ui, r: Rect) {
        let left = r.left() + 56.0;
        let right = r.right() - 56.0;
        let bar = Rect::from_min_max(pos2(left, r.top() + 44.0), pos2(right, r.top() + 72.0));
        let mut add = None;

        if let Some(chosen) = self.note_select.clone() {
            // Choosing notes: select all / delete (two clicks) / cancel.
            let all = !self.notes.is_empty() && chosen.len() == self.notes.len();
            let label = if all { t("Unselect all") } else { t("Select all") };
            if pill_button(ui, Rect::from_min_size(bar.min, vec2(120.0, bar.height())), label, BTN, "note-all") {
                self.note_select = Some(if all { HashSet::new() } else { (0..self.notes.len()).collect() });
                self.note_delete_at = None;
            }
            let armed = self.note_delete_at.is_some_and(|at| at.elapsed().as_secs_f32() < RESET_CONFIRM_SECS);
            let del = if armed { t("Sure?").to_string() } else { tr("Delete ({})", &[&chosen.len()]) };
            let del_rect = Rect::from_min_size(pos2(bar.left() + 128.0, bar.top()), vec2(120.0, bar.height()));
            if !chosen.is_empty() && pill_button(ui, del_rect, &del, if armed { RED } else { Color32::from_rgb(120, 32, 32) }, "note-del") {
                if armed {
                    let mut i = 0;
                    self.notes.retain(|_| {
                        i += 1;
                        !chosen.contains(&(i - 1))
                    });
                    self.note_select = None;
                    self.note_delete_at = None;
                    self.notes_changed();
                    return;
                }
                self.note_delete_at = Some(Instant::now());
            }
            if pill_button(ui, Rect::from_min_size(pos2(bar.right() - 80.0, bar.top()), vec2(80.0, bar.height())), t("Cancel"), BTN, "note-cancel") {
                self.note_select = None;
                self.note_delete_at = None;
            }
        } else {
            // The input line, the button that turns the copied text into a note, and "Select".
            let input = Rect::from_min_max(bar.min, pos2(bar.right() - 104.0, bar.bottom()));
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(input), |ui| {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.note_draft)
                        .hint_text(t("Write a note…"))
                        .desired_width(input.width())
                        .background_color(BTN)
                        .margin(egui::Margin::symmetric(10, 6))
                        .char_limit(NOTE_MAX_CHARS),
                );
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, t("Write a note…")));
                if self.note_focus {
                    resp.request_focus();
                    self.note_focus = false;
                }
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    add = Some(std::mem::take(&mut self.note_draft));
                    resp.request_focus(); // keep typing the next one
                }
            });
            let clip = pos2(input.right() + 20.0, bar.center().y);
            let resp = ui.interact(Rect::from_center_size(clip, vec2(28.0, 28.0)), ui.id().with("note-from-clip"), Sense::click());
            let name = t("Add the copied text as a note");
            resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
            ui.painter().circle_filled(clip, 14.0, if resp.hovered() { Color32::from_gray(60) } else { BTN });
            clip_icon(ui.painter(), clip, WHITE);
            if resp.on_hover_text(name).clicked() && !tray::clipboard_private() {
                add = self.clipboard.as_mut().and_then(|c| c.get_text().ok());
            }
            if !self.notes.is_empty()
                && pill_button(ui, Rect::from_min_size(pos2(bar.right() - 64.0, bar.top()), vec2(64.0, bar.height())), t("Select"), BTN, "note-select")
            {
                self.note_select = Some(HashSet::new());
            }
        }
        if let Some(text) = add.filter(|t| !t.trim().is_empty()) {
            self.notes.push(Note { created: Utc::now().timestamp(), text: text.trim().to_string(), pinned: false, done: None });
            self.notes_changed();
        }

        // The notes: pinned, open, then done ones; newest first.
        let list = Rect::from_min_max(pos2(left, r.top() + 80.0), pos2(right, r.top() + 160.0)); // ends above the page dots
        if self.notes.is_empty() {
            let p = ui.painter();
            p.text(list.center() - vec2(0.0, 8.0), Align2::CENTER_CENTER, t("No notes yet"), bold(14.0), WHITE);
            p.text(list.center() + vec2(0.0, 12.0), Align2::CENTER_CENTER, t("Type above and press Enter"), reg(12.0), DIM);
            return;
        }
        self.scroll_zone = Some((list, true));
        let notes = self.notes.clone();
        let copied = self.note_copied;
        let chosen = self.note_select.clone();
        let show_time = self.s.note_time;
        let now = Utc::now().timestamp();
        let mut act = None;
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(list), |ui| {
            egui::ScrollArea::vertical().id_salt("notes").auto_shrink(false).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                for (i, n) in notes.iter().enumerate() {
                    let width = ui.available_width() - 6.0; // room for the scroll bar
                    let sense = if chosen.is_some() { Sense::click() } else { Sense::click_and_drag() };
                    let (row, resp) = ui.allocate_exact_size(vec2(width, ROW_H), sense);
                    let ticked = chosen.as_ref().is_some_and(|c| c.contains(&i));
                    let p = ui.painter();
                    let fill = if ticked { BLUE.gamma_multiply(0.25) } else if resp.hovered() { Color32::from_gray(34) } else { CARD };
                    p.rect_filled(row, 8.0, fill);
                    let mark = pos2(row.left() + 14.0, row.center().y);
                    let mut dot_hit = false;
                    if chosen.is_some() {
                        tick_box(p, mark, ticked);
                        if resp.clicked() {
                            act = Some(NoteAct::Tick(i));
                        }
                    } else {
                        // The circle marks it done (its own click target, so a click on the text copies).
                        let dot_resp = ui.interact(Rect::from_center_size(mark, vec2(22.0, 22.0)), ui.id().with(("note-done", i)), Sense::click());
                        let done_name = if n.done.is_some() { t("Not done") } else { t("Done") };
                        dot_resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Checkbox, true, done_name));
                        check_circle(p, mark, 7.0, n.done.is_some());
                        dot_hit = dot_resp.hovered();
                        if dot_resp.clicked() {
                            act = Some(NoteAct::Done(i));
                        }
                    }
                    let mut x = row.left() + 30.0;
                    if n.pinned {
                        pin_icon(p, pos2(x + 4.0, row.center().y), Color32::from_gray(200));
                        x += 16.0;
                    }
                    // When it was written, on the right, unless turned off in Settings.
                    let mut text_right = row.right() - 10.0;
                    if show_time {
                        let when = p.layout_no_wrap(ago(now - n.created), reg(11.0), DIM);
                        text_right -= when.size().x + 10.0;
                        p.galley(pos2(row.right() - 10.0 - when.size().x, row.center().y - when.size().y / 2.0), when, DIM);
                    }
                    let just = copied.is_some_and(|(c, at)| c == i && at.elapsed().as_secs() < 2);
                    let col = if just { GREEN } else if n.done.is_some() { DIM } else { WHITE };
                    let shown = if just { t("Copied").to_string() } else { n.text.replace(['\n', '\r'], " ") };
                    let g = fit_line(p, &shown, reg(13.0), col, text_right - x);
                    let text_w = g.size().x;
                    p.galley(pos2(x, row.center().y - g.size().y / 2.0), g, col);
                    if n.done.is_some() && !just {
                        p.hline(x..=x + text_w, row.center().y, Stroke::new(1.0_f32, DIM));
                    }
                    if chosen.is_some() {
                        resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, ticked, &n.text));
                        continue;
                    }
                    if resp.drag_started() {
                        act = Some(NoteAct::Drag(n.text.clone()));
                    } else if resp.clicked() && !dot_hit {
                        act = Some(NoteAct::Copy(i));
                    }
                    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &n.text));
                    let resp = resp.on_hover_text(t("Click to copy · drag out · right-click for more"));
                    resp.context_menu(|ui| {
                        if ui.button(if n.pinned { t("Unpin") } else { t("Pin") }).clicked() {
                            act = Some(NoteAct::Pin(i));
                            ui.close_menu();
                        }
                        if ui.button(if n.done.is_some() { t("Not done") } else { t("Done") }).clicked() {
                            act = Some(NoteAct::Done(i));
                            ui.close_menu();
                        }
                        if ui.button(t("Delete")).clicked() {
                            act = Some(NoteAct::Delete(i));
                            ui.close_menu();
                        }
                    });
                }
            });
        });
        match act {
            Some(NoteAct::Copy(i)) => {
                let text = self.notes[i].text.clone();
                if let Some(c) = self.clipboard.as_mut() {
                    let _ = c.set_text(text.clone());
                }
                self.last_clip = text;
                self.note_copied = Some((i, Instant::now()));
            }
            Some(NoteAct::Drag(text)) => dragout::text(&text),
            Some(NoteAct::Done(i)) => {
                let today = self.today();
                let n = &mut self.notes[i];
                n.done = if n.done.is_some() { None } else { Some(today) };
                self.notes_changed();
            }
            Some(NoteAct::Pin(i)) => {
                self.notes[i].pinned = !self.notes[i].pinned;
                self.notes_changed();
            }
            Some(NoteAct::Delete(i)) => {
                self.notes.remove(i);
                self.notes_changed();
            }
            Some(NoteAct::Tick(i)) => {
                if let Some(c) = self.note_select.as_mut() {
                    if !c.remove(&i) {
                        c.insert(i);
                    }
                }
                self.note_delete_at = None;
            }
            None => {}
        }
    }

    /// Sorts, trims and saves the notes after any change.
    pub(crate) fn notes_changed(&mut self) {
        let today = self.today();
        tidy_notes(&mut self.notes, today, self.s.note_done_days);
        self.note_copied = None; // places move when the list is sorted again
        if self.shot.is_none() {
            save_notes(&self.notes);
        }
    }
}
