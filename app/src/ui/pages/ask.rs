//! The Ask Nusa box and its Gemini question.
use super::*;

impl App {
    /// Ask Nusa: a sum ending in "=" answers as you type; questions go to Gemini (needs a key).
    pub(crate) fn page_ask(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = ui.painter().clone();
        p.text(pos2(r.left() + 22.0, r.top() + 24.0), Align2::LEFT_CENTER, t("Ask Nusa"), bold(17.0), WHITE);
        if pill_button(ui, Rect::from_min_size(pos2(r.right() - 76.0, r.top() + 12.0), vec2(56.0, 24.0)), t("Close"), BTN, "aclose") {
            self.ask_open = false;
            return;
        }
        let field = Rect::from_min_size(pos2(r.left() + 22.0, r.top() + 48.0), vec2(r.width() - 44.0, 34.0));
        p.rect_filled(field, 17.0, CARD);
        settings_style(ui);
        let edit = ui.put(
            field.shrink2(vec2(14.0, 7.0)),
            // lock_focus: Tab stays in the box (a one-line box adds no tab character) and translates.
            egui::TextEdit::singleline(&mut self.ask_text).hint_text(t("Ask anything, or a sum like 12*3=")).frame(false).font(reg(15.0)).lock_focus(true),
        );
        if std::mem::take(&mut self.ask_focus) {
            edit.request_focus();
        }
        let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let tab = edit.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Tab));
        if edit.changed() {
            self.ask_gen += 1; // the question changed: an answer still on its way is for the old one
            self.ask_waiting = false;
            self.ask_copyable = false;
        }
        // Gemini's answer to the latest question.
        if self.ask_waiting {
            let reply = self.ask_reply.lock().unwrap().clone();
            if reply.0 == self.ask_gen {
                self.ask_waiting = false;
                self.ask_answer = reply.1;
                self.ask_copyable = reply.2;
            }
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.ask_open = false;
            self.held = false;
            return;
        }
        // "100usd to vnd" on Enter reads as "100usd to vnd=".
        if enter && calc::currency(&self.ask_text).is_some() && !self.ask_text.trim_end().ends_with('=') {
            self.ask_text.push('=');
        }
        let q = self.ask_text.trim();
        if let Some((amount, from, to)) = calc::currency(q).filter(|_| q.ends_with('=')) {
            // Asked again each frame, so the answer appears as soon as the rates arrive.
            self.ask_answer = match online::convert(&self.rates, amount, &from, &to) {
                Some(v) => format!("= {} {to}", calc::money(v)),
                None => match online::rates_state(&self.rates) {
                    online::RatesState::Coming => t("Getting today's rates…").to_string(),
                    online::RatesState::Offline => t("Rates need the internet — try again in a minute").to_string(),
                    online::RatesState::Here => tr("Unknown currency code: {} or {}", &[&from, &to]),
                },
            };
            if enter {
                edit.request_focus();
            }
        } else if tab && !q.is_empty() {
            let rule = format!(
                "Translate the user's text into {}. If it is already in that language, translate it into English. Reply with the translation only.",
                i18n::ENGLISH_NAMES[i18n::lang()]
            );
            self.ask_gemini(&rule, q.to_string());
        } else if edit.changed() || enter {
            match calc::eval(q) {
                // A sum answers once it ends with "=", or on Enter.
                Some(v) if q.ends_with('=') || enter => self.ask_answer = format!("= {}", calc::show(v)),
                None if enter && !q.is_empty() => {
                    let rule = "You answer inside a small notch on a Windows PC. Answer in at most three short sentences, \
                        in plain text with no Markdown, in the language of the question.";
                    self.ask_gemini(rule, q.to_string());
                }
                _ => self.ask_answer.clear(),
            }
        }
        if enter || tab {
            edit.request_focus();
            // Kept (in memory only) for ↑ / ↓, the newest last, without repeating the last one.
            let asked = self.ask_text.trim().to_string();
            if !asked.is_empty() && self.ask_history.last() != Some(&asked) {
                self.ask_history.push(asked);
                if self.ask_history.len() > ASK_HISTORY_MAX {
                    self.ask_history.remove(0);
                }
            }
            self.ask_hist_at = None;
        }
        // ↑ brings back the questions asked before, one by one; ↓ goes forward again, to an empty box.
        let (up, down) = ui.input(|i| (i.key_pressed(egui::Key::ArrowUp), i.key_pressed(egui::Key::ArrowDown)));
        if edit.has_focus() && (up || down) && !self.ask_history.is_empty() {
            let last = self.ask_history.len() - 1;
            self.ask_hist_at = match (self.ask_hist_at, up) {
                (None, true) => Some(last),
                (Some(k), true) => Some(k.saturating_sub(1)),
                (Some(k), false) if k < last => Some(k + 1),
                _ => None,
            };
            self.ask_text = self.ask_hist_at.map_or_else(String::new, |k| self.ask_history[k].clone());
            // The cursor goes to the end of the text brought back.
            if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), edit.id) {
                let end = egui::text::CCursor::new(self.ask_text.chars().count());
                state.cursor.set_char_range(Some(egui::text::CCursorRange::one(end)));
                state.store(ui.ctx(), edit.id);
            }
            self.ask_gen += 1;
            self.ask_waiting = false;
            self.ask_copyable = false;
            self.ask_answer.clear();
        }
        if !self.ask_answer.is_empty() {
            // A real answer (a sum, a conversion, Gemini's reply) can be copied with one click.
            let copyable = self.ask_answer.starts_with('=') || self.ask_copyable;
            if copyable {
                let done = self.copied.as_ref().is_some_and(|(c, at)| *c == answer_text(&self.ask_answer) && at.elapsed().as_secs() < 2);
                let label = if done { t("Copied") } else { t("Copy") };
                if pill_button(ui, Rect::from_min_size(pos2(r.right() - 92.0, r.top() + 96.0), vec2(66.0, 24.0)), label, BTN, "acopy") {
                    let text = answer_text(&self.ask_answer);
                    if let Some(c) = self.clipboard.as_mut() {
                        let _ = c.set_text(text.clone());
                    }
                    self.last_clip = text.clone();
                    self.copied = Some((text, Instant::now()));
                }
            }
            let text_right = if copyable { r.right() - 104.0 } else { r.right() - 30.0 };
            if self.ask_answer.starts_with('=') {
                let g = fit_line(&p, &self.ask_answer, bold(30.0), WHITE, text_right - (r.left() + 30.0));
                p.galley(pos2(r.left() + 30.0, r.top() + 116.0 - g.size().y / 2.0), g, WHITE);
            } else {
                // Gemini's answers wrap and scroll, and can be selected to copy.
                let area = Rect::from_min_max(pos2(r.left() + 30.0, r.top() + 92.0), pos2(text_right, r.bottom() - 28.0));
                let col = if self.ask_waiting { DIM } else { WHITE };
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(area), |ui| {
                    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                        ui.add(egui::Label::new(egui::RichText::new(&self.ask_answer).font(reg(14.0)).color(col)).wrap());
                    });
                });
            }
        }
        p.text(pos2(r.center().x, r.bottom() - 14.0), Align2::CENTER_CENTER, t("Enter to ask · Tab to translate · Esc to close"), reg(11.0), DIM);
    }

    /// Sends `question` to Gemini on its own thread; the answer shows when it comes back.
    pub(crate) fn ask_gemini(&mut self, rule: &str, question: String) {
        self.ask_copyable = false;
        if self.gemini_key.trim().is_empty() {
            self.ask_answer = t("Ask Nusa needs a Gemini key — add one in Settings").to_string();
            return;
        }
        self.ask_gen += 1;
        self.ask_waiting = true;
        self.ask_answer = t("Thinking…").to_string();
        let (n, key, rule, out) = (self.ask_gen, self.gemini_key.trim().to_string(), rule.to_string(), self.ask_reply.clone());
        std::thread::spawn(move || {
            let reply = online::gemini(&key, &rule, &question);
            let ok = reply.is_ok();
            let answer = match reply {
                Ok(a) => a,
                Err(online::AskError::Key) => t("Gemini turned the key down — check it in Settings").to_string(),
                Err(online::AskError::Busy) => t("Too many questions just now — try again in a minute").to_string(),
                Err(online::AskError::Offline) => t("Couldn't reach Gemini — check the internet").to_string(),
                Err(online::AskError::Empty) => t("Gemini gave no answer to that").to_string(),
            };
            *out.lock().unwrap() = (n, answer, ok);
            tray::wake();
        });
    }
}
