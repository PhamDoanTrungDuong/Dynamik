//! The Settings page's building blocks: cards, rows, switches, pickers and text boxes.
use super::*;

/// Dark controls for Settings: combo boxes, number fields and their drop-down lists.
pub(crate) fn settings_style(ui: &mut egui::Ui) {
    let style = |v: &mut egui::Visuals| {
        for (w, fill) in [(&mut v.widgets.inactive, BTN), (&mut v.widgets.hovered, Color32::from_gray(48)), (&mut v.widgets.active, Color32::from_gray(56)), (&mut v.widgets.open, Color32::from_gray(48))] {
            w.bg_fill = fill;
            w.weak_bg_fill = fill;
            w.bg_stroke = Stroke::NONE;
            w.corner_radius = 8.into();
            w.fg_stroke.color = WHITE;
            w.expansion = 0.0;
        }
        v.selection.bg_fill = BLUE.gamma_multiply(0.45);
        v.window_fill = Color32::from_gray(28);
        v.window_stroke = Stroke::new(1.0_f32, Color32::from_gray(48));
        v.window_corner_radius = 10.into();
        v.window_shadow = egui::Shadow::NONE;
        v.popup_shadow = egui::Shadow::NONE;
    };
    style(&mut ui.style_mut().visuals);
    // Drop-down lists are drawn with the context's style, not this Ui's.
    ui.ctx().style_mut(|s| style(&mut s.visuals));
    ui.spacing_mut().button_padding = vec2(10.0, 4.0);
    ui.spacing_mut().item_spacing.y = 0.0;
    ui.spacing_mut().scroll.bar_width = 4.0;
}

/// A group of rows on a dark rounded card, under its title (none for the section's first card,
/// which sits under the section's own heading).
pub(crate) fn card(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(10.0);
    if !title.is_empty() {
        ui.label(egui::RichText::new(t(title).to_uppercase()).font(bold(11.0)).color(DIM));
        ui.add_space(4.0);
    }
    egui::Frame::new().fill(CARD).corner_radius(12).inner_margin(egui::Margin::symmetric(12, 2)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        // Rows draw a line above themselves, except the first one on the card.
        let top = ui.cursor().top();
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("card_top"), top));
        add(ui);
    });
}

/// "Name — hint": the name, with the hint dimmer underneath.
pub(crate) fn row_text(ui: &mut egui::Ui, label: &str) {
    let label = t(label);
    let Some((name, hint)) = label.split_once(" — ") else {
        ui.label(egui::RichText::new(label).font(reg(14.0)).color(WHITE));
        return;
    };
    let hint = Some(hint);
    ui.vertical(|ui| {
        ui.label(egui::RichText::new(name).font(reg(14.0)).color(WHITE));
        if let Some(h) = hint {
            let mut c = h.chars();
            let h: String = c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default();
            ui.label(egui::RichText::new(h).font(reg(12.0)).color(DIM));
        }
    });
}

pub(crate) fn row_label(ui: &mut egui::Ui, label: &str, h: f32) {
    ui.allocate_ui_with_layout(vec2(ui.available_width(), h), egui::Layout::left_to_right(egui::Align::Center), |ui| row_text(ui, label));
}

/// One row: text on the left, the control on the right, centred in a fixed height.
pub(crate) fn row(ui: &mut egui::Ui, label: &str, control: impl FnOnce(&mut egui::Ui)) {
    let h = if label.contains(" — ") { 48.0 } else { 38.0 };
    let r = ui.allocate_ui_with_layout(vec2(ui.available_width(), h), egui::Layout::right_to_left(egui::Align::Center), |ui| {
        control(ui);
        ui.add_space(10.0);
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| row_text(ui, label));
    }).response.rect;
    let card_top = ui.ctx().data(|d| d.get_temp::<f32>(egui::Id::new("card_top"))).unwrap_or(f32::MIN);
    if r.top() > card_top + 1.0 {
        ui.painter().hline(r.x_range(), r.top(), Stroke::new(1.0_f32, Color32::from_gray(34)));
    }
}

/// A switch like the Water page's, reported to screen readers as a checkbox with its label.
pub(crate) fn switch_row(ui: &mut egui::Ui, label: &str, v: &mut bool) {
    row(ui, label, |ui| {
        switch(ui, v, label);
    });
}

/// The switch on its own; true when it was flipped.
pub(crate) fn switch(ui: &mut egui::Ui, v: &mut bool, label: &str) -> bool {
    let (r, resp) = ui.allocate_exact_size(vec2(40.0, 22.0), Sense::click());
    if resp.clicked() {
        *v = !*v;
    }
    let name = t(label).split(" — ").next().unwrap_or(label);
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *v, name));
    let t = ui.ctx().animate_bool(resp.id, *v);
    let p = ui.painter();
    p.rect_filled(r, 11.0, lerp_color(Color32::from_gray(60), BLUE, t));
    p.circle_filled(pos2(r.left() + 11.0 + t * 18.0, r.center().y), 8.0, WHITE);
    resp.clicked()
}

/// A small round button that moves a row; reported to screen readers by what it does.
pub(crate) fn move_button(ui: &mut egui::Ui, text: &str, what: &str) -> bool {
    let (r, resp) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), what));
    let p = ui.painter();
    p.circle_filled(r.center(), 11.0, if resp.hovered() { Color32::from_gray(60) } else { BTN });
    p.text(r.center(), Align2::CENTER_CENTER, text, bold(13.0), WHITE);
    resp.clicked()
}

/// A row with a one-line text box; true when it changed.
pub(crate) fn text_row(ui: &mut egui::Ui, label: &str, v: &mut String, hint: &str) -> bool {
    let mut changed = false;
    row(ui, label, |ui| {
        let name = t(label).split(" — ").next().unwrap_or(label).to_string();
        let resp = ui.add(egui::TextEdit::singleline(v).hint_text(hint).desired_width(170.0).background_color(BTN).margin(egui::Margin::symmetric(8, 5)));
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, &name));
        changed = resp.changed();
    });
    changed
}

/// A row with a hidden text box for a secret; true when it changed.
pub(crate) fn secret_row(ui: &mut egui::Ui, label: &str, v: &mut String) -> bool {
    let mut changed = false;
    row(ui, label, |ui| {
        let name = t(label).split(" — ").next().unwrap_or(label).to_string();
        let resp = ui.add(egui::TextEdit::singleline(v).password(true).hint_text(t("Paste here")).desired_width(170.0).background_color(BTN).margin(egui::Margin::symmetric(8, 5)));
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, &name));
        changed = resp.changed();
    });
    changed
}

pub(crate) fn number<T: egui::emath::Numeric>(ui: &mut egui::Ui, label: &str, v: &mut T, range: std::ops::RangeInclusive<T>, suffix: &str) {
    row(ui, label, |ui| {
        ui.add(egui::DragValue::new(v).range(range).suffix(t(suffix)));
    });
}

pub(crate) fn combo(ui: &mut egui::Ui, label: &str, v: &mut usize, items: &[&str]) -> bool {
    let old = *v;
    row(ui, label, |ui| {
        egui::ComboBox::from_id_salt(label)
            .width(170.0)
            .wrap_mode(egui::TextWrapMode::Truncate)
            .selected_text(t(items.get(*v).copied().unwrap_or("")))
            .show_ui(ui, |ui| {
                for (i, it) in items.iter().enumerate() {
                    ui.selectable_value(v, i, t(it));
                }
            });
    });
    *v != old
}

/// A combo choosing one of `values`, shown as `names`; a value not in the list shows the middle one.
pub(crate) fn pick<T: Copy + PartialEq>(ui: &mut egui::Ui, label: &str, v: &mut T, values: &[T], names: &[&str]) -> bool {
    let mut i = values.iter().position(|x| x == v).unwrap_or(values.len() / 2);
    let changed = combo(ui, label, &mut i, names);
    if changed {
        *v = values[i];
    }
    changed
}

/// The City list as Settings shows it: "Town — Region", and "My location" last.
pub(crate) fn place_names() -> Vec<String> {
    (0..prayer::place_count())
        .map(|i| if i == prayer::here_index() { t("My location").into() } else { let (r, c) = prayer::place_name(i); format!("{c} — {r}") })
        .collect()
}
