//! The notch's pages: Music, Pomodoro, Water, Shelf, To-do, Prayer, Widgets, Today, Ask and Settings.
use super::*;

mod music;
mod pomodoro;
mod water;
mod shelf;
mod todo;
#[path = "prayer.rs"] // not `prayer`: that name is the prayer-times module
mod prayer_page;
mod widgets;
mod today;
mod ask;
mod settings;

impl App {
    pub(crate) fn page_title(&self, i: usize) -> (&'static str, &'static str, Color32) {
        match i {
            0 => ("", "", WHITE), // the album art sits where a title would be
            1 => (t("Pomodoro"), "", WHITE),
            2 => (t("Water "), t("Reminder"), BLUE),
            3 => (t("Shelf"), "", WHITE),
            4 => (t("Todo"), t("List"), GREEN),
            5 => (t("Prayer"), "", WHITE),
            TODAY_PAGE => (t("Today"), "", WHITE),
            _ => (t("Widgets"), "", WHITE),
        }
    }

    pub(crate) fn draw_page(&mut self, ui: &mut egui::Ui, i: usize, r: Rect) {
        match i {
            0 => self.page_music(ui, r),
            1 => self.page_pomodoro(ui, r),
            2 => self.page_water(ui, r),
            3 => self.page_shelf(ui, r),
            4 => self.page_todo(ui, r),
            5 => self.page_prayer(ui, r),
            TODAY_PAGE => self.page_today(ui, r),
            _ => self.page_widgets(ui, r),
        }
    }
}
