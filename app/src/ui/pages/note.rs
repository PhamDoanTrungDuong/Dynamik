//! The Note page: quick notes to pin, finish, copy and drag out.
use super::*;

/// What a click on a note asks for, done after the list is drawn.
enum NoteAct {
    Copy(usize),
    Drag(String),
    Done(usize),
    Pin(usize),
    Delete(usize),
}

impl App {
    pub(crate) fn page_note(&mut self, ui: &mut egui::Ui, r: Rect) {
        let left = r.left() + 56.0;
        let right = r.right() - 56.0;

        // The input line, and the button that turns the copied text into a note.
        let input = Rect::from_min_max(pos2(left, r.top() + 44.0), pos2(right - 36.0, r.top() + 72.0));
        let mut add = None;
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
        let clip = pos2(right - 13.0, input.center().y);
        let resp = ui.interact(Rect::from_center_size(clip, vec2(28.0, 28.0)), ui.id().with("note-from-clip"), Sense::click());
        let name = t("Add the copied text as a note");
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
        ui.painter().circle_filled(clip, 14.0, if resp.hovered() { Color32::from_gray(60) } else { BTN });
        clip_icon(ui.painter(), clip, WHITE);
        if resp.on_hover_text(name).clicked() && !tray::clipboard_private() {
            add = self.clipboard.as_mut().and_then(|c| c.get_text().ok());
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
        let mut act = None;
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(list), |ui| {
            egui::ScrollArea::vertical().id_salt("notes").auto_shrink(false).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                for (i, n) in notes.iter().enumerate() {
                    let (row, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::click_and_drag());
                    let p = ui.painter();
                    if resp.hovered() {
                        p.rect_filled(row, 8.0, Color32::from_gray(24));
                    }
                    // The circle marks it done (its own click target, so a click on the text copies).
                    let dot = pos2(row.left() + 12.0, row.center().y);
                    let dot_resp = ui.interact(Rect::from_center_size(dot, vec2(22.0, 22.0)), ui.id().with(("note-done", i)), Sense::click());
                    let done_name = if n.done.is_some() { t("Not done") } else { t("Done") };
                    dot_resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Checkbox, true, done_name));
                    check_circle(p, dot, 7.0, n.done.is_some());
                    if dot_resp.clicked() {
                        act = Some(NoteAct::Done(i));
                    }
                    let mut x = row.left() + 28.0;
                    if n.pinned {
                        p.circle_filled(pos2(x + 3.0, row.center().y), 3.0, ORANGE);
                        x += 12.0;
                    }
                    let just = copied.is_some_and(|(c, at)| c == i && at.elapsed().as_secs() < 2);
                    let col = if n.done.is_some() { DIM } else { WHITE };
                    let shown = if just { t("Copied").to_string() } else { n.text.replace(['\n', '\r'], " ") };
                    let g = fit_line(p, &shown, reg(13.0), col, row.right() - x - 6.0);
                    let at = pos2(x, row.center().y - g.size().y / 2.0);
                    let width = g.size().x;
                    p.galley(at, g, col);
                    if n.done.is_some() && !just {
                        p.hline(x..=x + width, row.center().y, Stroke::new(1.0_f32, DIM));
                    }
                    if resp.drag_started() {
                        act = Some(NoteAct::Drag(n.text.clone()));
                    } else if resp.clicked() && !dot_resp.hovered() {
                        act = Some(NoteAct::Copy(i));
                    }
                    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &n.text));
                    let resp = resp.on_hover_text(t("Click to copy · drag out · right-click for more"));
                    resp.context_menu(|ui| {
                        if ui.button(if n.pinned { t("Unpin") } else { t("Pin") }).clicked() {
                            act = Some(NoteAct::Pin(i));
                            ui.close_menu();
                        }
                        if ui.button(done_name).clicked() {
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
            None => {}
        }
    }

    /// Sorts, trims and saves the notes after any change.
    pub(crate) fn notes_changed(&mut self) {
        let today = self.today();
        tidy_notes(&mut self.notes, today);
        self.note_copied = None; // places move when the list is sorted again
        if self.shot.is_none() {
            save_notes(&self.notes);
        }
    }
}
