//! Settings and saved data: plain `key=value` / line files in %LOCALAPPDATA%\MyNotch.
use chrono::NaiveDate;
use std::{collections::BTreeMap, fs, path::PathBuf, str::FromStr};

// Values from the original (see MECHANICS.md).
pub const SIZE_NAMES: [&str; 4] = ["Small", "Medium", "Large", "Extra large"];
pub const SIZE_SCALES: [f32; 4] = [1.0, 1.15, 1.3, 1.5];
pub const BOTTLE_STEP: u32 = 50;
pub const BOTTLE_MIN: u32 = 100;
pub const BOTTLE_MAX: u32 = 2000;
pub const HOURS_STEP: u16 = 30;
pub const FROM_MAX: u16 = 1380;
pub const UNTIL_MAX: u16 = 1440;
pub const MIN_SPAN: u16 = 60;
pub const GOAL_STEP: u32 = 250; // the water goal moves in 0.25 L steps...
pub const GOAL_MIN: u32 = 1000; // ...from 1 L
pub const GOAL_MAX: u32 = 4000; // ...to 4 L

/// A water goal on the 0.25 L steps between 1 and 4 L (the nearest one).
pub fn goal_step(ml: u32) -> u32 {
    ((ml + GOAL_STEP / 2) / GOAL_STEP * GOAL_STEP).clamp(GOAL_MIN, GOAL_MAX)
}
pub const GLASS_ML: u32 = 250; // the Water page's "+250 ml": an ordinary glass
pub const CLIP_MAX_FREE: usize = 5;
pub const SNOOZES: [u32; 3] = [5, 10, 15];
pub const PRAYER_EARLY_MIN: u32 = 10;
pub const WATER_SNOOZE_MIN: u32 = 10; // 0x1400583ab: snooze = now + 10, capped at 23:59
pub const REFILL_FRACTION: f32 = 0.75; // 0x140002222: refill when bottle < 0.75 x one drink

pub const GEMINI_FILE: &str = "gemini.key";
pub const ICAL_FILE: &str = "calendar.key";

pub fn dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    let d = base.join("MyNotch");
    let _ = fs::create_dir_all(&d);
    d
}

pub fn read(name: &str) -> String {
    fs::read_to_string(dir().join(name)).unwrap_or_default()
}

pub fn write(name: &str, text: &str) {
    // Write then rename so a crash mid-write never leaves a half file.
    let p = dir().join(name);
    let tmp = p.with_extension("tmp");
    if fs::write(&tmp, text).is_ok() {
        let _ = fs::rename(&tmp, &p);
    }
}

/// A secret (the Gemini key, the calendar's private address), sealed with Windows DPAPI so only
/// this Windows account on this PC can open it. Empty when missing or unreadable.
pub fn read_secret(name: &str) -> String {
    let sealed = fs::read(dir().join(name)).unwrap_or_default();
    if sealed.is_empty() {
        return String::new();
    }
    dpapi(&sealed, false).and_then(|b| String::from_utf8(b).ok()).unwrap_or_default()
}

/// Seals and saves a secret; an empty one removes the file.
pub fn write_secret(name: &str, text: &str) {
    let p = dir().join(name);
    if text.is_empty() {
        let _ = fs::remove_file(p);
    } else if let Some(sealed) = dpapi(text.as_bytes(), true) {
        let _ = fs::write(p, sealed);
    }
}

fn dpapi(data: &[u8], seal: bool) -> Option<Vec<u8>> {
    use windows_sys::Win32::Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB};
    let input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
    let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
    use std::ptr::{null, null_mut};
    let ok = unsafe {
        if seal {
            CryptProtectData(&input, null(), null(), null(), null(), CRYPTPROTECT_UI_FORBIDDEN, &mut out)
        } else {
            CryptUnprotectData(&input, null_mut(), null(), null(), null(), CRYPTPROTECT_UI_FORBIDDEN, &mut out)
        }
    };
    if ok == 0 || out.pbData.is_null() {
        return None;
    }
    let bytes = unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize) }.to_vec();
    unsafe { windows_sys::Win32::Foundation::LocalFree(out.pbData as _) };
    Some(bytes)
}

pub fn kv(text: &str) -> BTreeMap<String, String> {
    text.lines().filter_map(|l| l.split_once('=')).map(|(k, v)| (k.trim().into(), v.trim().into())).collect()
}

fn get<T: FromStr>(m: &BTreeMap<String, String>, k: &str, d: T) -> T {
    m.get(k).and_then(|v| v.parse().ok()).unwrap_or(d)
}

#[derive(Clone, PartialEq)]
pub struct Settings {
    pub startup: bool,
    pub size: usize,
    pub minimal: bool,
    pub open_click: bool,
    pub sound: bool,
    pub status_ring: usize, // 0 water, 1 tasks done today, 2 no ring
    pub focus: u32,
    pub short_break: u32,
    pub long_break: u32,
    pub todo_start: bool,
    pub todo_end: bool,
    pub snooze: u32,
    pub water_snooze: u32, // minutes the Water pop-up's Snooze waits (5, 10 or 15)
    pub water: bool,
    pub goal: u32,
    pub bottle: u32,
    pub from: u16,
    pub until: u16,
    pub prayer: bool,
    pub prayer_city: usize, // index into prayer's City list (Indonesia, world, then My location)
    pub prayer_method: usize, // 0: the place's usual method, else index + 1 into prayer::METHODS
    pub weather_city: usize, // the Weather widget's place, from the same City list
    pub prayer_lat: f64, // My location, from Windows when asked
    pub prayer_lon: f64,
    pub prayer_reminder: bool,
    pub prayer_early: bool,
    pub prayer_pause_media: bool,
    pub today_page: bool,
    pub prayer_adj: [i32; 5], // minutes; stored as value + 10 like the original
    pub clipboard: bool,
    pub clipboard_max: usize,
    pub clipboard_keep: bool,
    pub shelf_clear: bool,
    pub popup_battery: bool,
    pub popup_wifi: bool,
    pub hide_fullscreen: bool,
    pub welcomed: bool, // the first-start hint has been shown
    pub cal_early: bool, // calendar events also remind 10 minutes before
    pub prayer_asr: usize, // Ashar: 0 as the method does, 1 Shafi'i, 2 Hanafi
    pub auto_next: bool, // Pomodoro: the next part starts by itself
    pub focus_quiet: bool, // notifications wait while focusing
    pub lyrics: bool,
    pub hud_volume: bool,
    pub hud_caps: bool,
    pub hud_language: bool,
    pub hud_brightness: bool,
    pub popup_bluetooth: bool,
    pub notifications: bool,
    pub notification_preview: bool,
    pub mascot: bool,
    pub app_icon: usize, // index into tray::ICON_NAMES
    pub language: usize, // index into i18n::NAMES
    pub widget_city1: usize,
    pub widget_city2: usize,
    pub widget_event: usize,
    pub widget_left: usize,
    pub widget_right: usize,
    pub week_monday: bool,
    pub show_clock: bool,         // the date and time in the notch
    pub page_order: Vec<usize>,   // every page id once, in the order they are paged through
    pub pages_hidden: Vec<usize>, // page ids left out
}

/// Pages the notch has (main.rs PAGES).
pub const PAGE_COUNT: usize = 8;

fn ids(m: &BTreeMap<String, String>, k: &str) -> Vec<usize> {
    m.get(k).map(|v| v.split(',').filter_map(|x| x.trim().parse().ok()).collect()).unwrap_or_default()
}

/// The saved order when it lists each page exactly once, else the usual order.
fn page_order(saved: Vec<usize>) -> Vec<usize> {
    let mut sorted = saved.clone();
    sorted.sort_unstable();
    if sorted == (0..PAGE_COUNT).collect::<Vec<_>>() { saved } else { (0..PAGE_COUNT).collect() }
}

fn join(ids: &[usize]) -> String {
    ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",")
}

impl Settings {
    pub fn load() -> Self {
        Self::from_text(&read("settings.txt"), &read("water.txt"))
    }

    pub fn from_text(text: &str, water: &str) -> Self {
        let m = kv(text);
        let w = kv(water);
        let mut s = Settings {
            startup: get(&m, "startup", false),
            size: get(&m, "size", 0usize).min(3),
            minimal: get(&m, "minimal", true), // rests as the original's thin peek bar unless turned off
            open_click: get(&m, "open_click", false),
            sound: get(&m, "sound", true),
            status_ring: get(&m, "status_ring", 0usize).min(2),
            focus: get(&m, "focus", 25),
            short_break: get(&m, "short_break", 5),
            long_break: get(&m, "long_break", 15),
            todo_start: get(&m, "todo_start", true),
            todo_end: get(&m, "todo_end", true),
            snooze: get(&m, "snooze", 10u32),
            water_snooze: Some(get(&m, "water_snooze", WATER_SNOOZE_MIN)).filter(|v| SNOOZES.contains(v)).unwrap_or(WATER_SNOOZE_MIN),
            // Water settings live in water.txt (keys from 0x1400e31e0), defaults from 0x140049847.
            water: get(&w, "on", 1u8) != 0,
            goal: get(&w, "goal", 1500),
            bottle: get(&w, "bottle", 500u32).clamp(BOTTLE_MIN, BOTTLE_MAX),
            from: get(&w, "start", 480u16).min(FROM_MAX),
            until: get(&w, "end", 1020u16),
            prayer: get(&m, "prayer", false),
            prayer_city: get(&m, "prayer_city", crate::prayer::DEFAULT_CITY).min(crate::prayer::place_count() - 1),
            prayer_method: get(&m, "prayer_method", 0usize).min(crate::prayer::METHODS.len()),
            weather_city: get(&m, "weather_city", get(&m, "prayer_city", crate::prayer::DEFAULT_CITY)).min(crate::prayer::place_count() - 1),
            prayer_lat: get(&m, "prayer_lat", 0.0f64).clamp(-90.0, 90.0),
            prayer_lon: get(&m, "prayer_lon", 0.0f64).clamp(-180.0, 180.0),
            prayer_reminder: get(&m, "prayer_reminder", true),
            prayer_early: get(&m, "prayer_early", false),
            // Both off by default, as in the original's settings table (0x1400e1320).
            prayer_pause_media: get(&m, "prayer_pause_media", false),
            today_page: get(&m, "today_page", false),
            prayer_adj: ["prayer_adj_subuh", "prayer_adj_dzuhur", "prayer_adj_ashar", "prayer_adj_maghrib", "prayer_adj_isya"]
                .map(|k| get(&m, k, 10i32) - 10),
            clipboard: get(&m, "clipboard", false),
            clipboard_max: get(&m, "clipboard_max", CLIP_MAX_FREE).clamp(1, CLIP_MAX_FREE),
            clipboard_keep: get(&m, "clipboard_keep", false),
            shelf_clear: get(&m, "shelf_clear", false),
            popup_battery: get(&m, "popup_battery", true),
            popup_wifi: get(&m, "popup_wifi", true),
            hide_fullscreen: get(&m, "hide_fullscreen", true),
            welcomed: get(&m, "welcomed", false),
            cal_early: get(&m, "cal_early", true),
            prayer_asr: Some(get(&m, "prayer_asr", 0usize)).filter(|&a| a <= 2).unwrap_or(0),
            auto_next: get(&m, "auto_next", false),
            focus_quiet: get(&m, "focus_quiet", false),
            lyrics: get(&m, "lyrics", true),
            hud_volume: get(&m, "hud_volume", true),
            hud_caps: get(&m, "hud_caps", true),
            hud_language: get(&m, "hud_language", true),
            hud_brightness: get(&m, "hud_brightness", true),
            popup_bluetooth: get(&m, "popup_bluetooth", true),
            notifications: get(&m, "notifications", true),
            notification_preview: get(&m, "notification_preview", false),
            mascot: get(&m, "mascot", true),
            app_icon: get(&m, "app_icon", 0usize),
            language: Some(get(&m, "language", crate::i18n::system_default())).filter(|&l| l < crate::i18n::NAMES.len()).unwrap_or_else(crate::i18n::system_default),
            widget_city1: get(&m, "widget_city1", 5usize),
            widget_city2: get(&m, "widget_city2", 13usize),
            widget_event: get(&m, "widget_event", 1usize),
            widget_left: get(&m, "widget_left", 0usize),
            widget_right: get(&m, "widget_right", 1usize),
            week_monday: get(&m, "week_monday", false),
            show_clock: get(&m, "show_clock", true),
            page_order: page_order(ids(&m, "page_order")),
            pages_hidden: ids(&m, "pages_hidden").into_iter().filter(|&i| i < PAGE_COUNT).collect(),
        };
        // Same clamp as the original's loader (0x14004aa9e): until within [from + 60, 24:00].
        s.until = s.until.max(s.from + MIN_SPAN).min(UNTIL_MAX);
        s
    }

    pub fn save(&self) {
        let s = self;
        let lines = [
            format!("startup={}", s.startup),
            format!("size={}", s.size),
            format!("minimal={}", s.minimal),
            format!("open_click={}", s.open_click),
            format!("sound={}", s.sound),
            format!("status_ring={}", s.status_ring),
            format!("focus={}", s.focus),
            format!("short_break={}", s.short_break),
            format!("long_break={}", s.long_break),
            format!("todo_start={}", s.todo_start),
            format!("todo_end={}", s.todo_end),
            format!("snooze={}", s.snooze),
            format!("water_snooze={}", s.water_snooze),
            format!("prayer={}", s.prayer),
            format!("prayer_city={}", s.prayer_city),
            format!("prayer_method={}", s.prayer_method),
            format!("weather_city={}", s.weather_city),
            format!("prayer_lat={}", s.prayer_lat),
            format!("prayer_lon={}", s.prayer_lon),
            format!("prayer_reminder={}", s.prayer_reminder),
            format!("prayer_early={}", s.prayer_early),
            format!("prayer_pause_media={}", s.prayer_pause_media),
            format!("today_page={}", s.today_page),
            format!("prayer_adj_subuh={}", s.prayer_adj[0] + 10),
            format!("prayer_adj_dzuhur={}", s.prayer_adj[1] + 10),
            format!("prayer_adj_ashar={}", s.prayer_adj[2] + 10),
            format!("prayer_adj_maghrib={}", s.prayer_adj[3] + 10),
            format!("prayer_adj_isya={}", s.prayer_adj[4] + 10),
            format!("clipboard={}", s.clipboard),
            format!("clipboard_max={}", s.clipboard_max),
            format!("clipboard_keep={}", s.clipboard_keep),
            format!("shelf_clear={}", s.shelf_clear),
            format!("popup_battery={}", s.popup_battery),
            format!("popup_wifi={}", s.popup_wifi),
            format!("hide_fullscreen={}", s.hide_fullscreen),
            format!("welcomed={}", s.welcomed),
            format!("cal_early={}", s.cal_early),
            format!("prayer_asr={}", s.prayer_asr),
            format!("auto_next={}", s.auto_next),
            format!("focus_quiet={}", s.focus_quiet),
            format!("lyrics={}", s.lyrics),
            format!("hud_volume={}", s.hud_volume),
            format!("hud_caps={}", s.hud_caps),
            format!("hud_language={}", s.hud_language),
            format!("hud_brightness={}", s.hud_brightness),
            format!("popup_bluetooth={}", s.popup_bluetooth),
            format!("notifications={}", s.notifications),
            format!("notification_preview={}", s.notification_preview),
            format!("mascot={}", s.mascot),
            format!("app_icon={}", s.app_icon),
            format!("language={}", s.language),
            format!("widget_city1={}", s.widget_city1),
            format!("widget_city2={}", s.widget_city2),
            format!("widget_event={}", s.widget_event),
            format!("widget_left={}", s.widget_left),
            format!("widget_right={}", s.widget_right),
            format!("week_monday={}", s.week_monday),
            format!("show_clock={}", s.show_clock),
            format!("page_order={}", join(&s.page_order)),
            format!("pages_hidden={}", join(&s.pages_hidden)),
        ];
        write("settings.txt", &lines.join("\n"));
    }

    /// The original's water plan (0x14004ab75): span = max(until - from, 60);
    /// reminders = min(clamp(round(goal / 25), 1, 64), max(floor(span / 15), 1));
    /// each = max(round(goal / reminders), 1); interval = span / reminders;
    /// refills = max(ceil(goal / bottle) - 1, 0).
    pub fn water_plan(&self) -> WaterPlan {
        let span = self.until.saturating_sub(self.from).max(MIN_SPAN) as f32;
        let goal = self.goal as f32;
        let by_goal = (goal / 25.0).round().clamp(1.0, 64.0);
        let reminders = by_goal.min((span / 15.0).floor().max(1.0));
        let each = (goal / reminders).round().max(1.0) as u32;
        let refills = ((goal / self.bottle.max(1) as f32).ceil() as u32).saturating_sub(1);
        WaterPlan { reminders: reminders as u32, interval: span / reminders, each, refills }
    }
}

pub struct WaterPlan {
    pub reminders: u32,
    pub interval: f32, // minutes
    pub each: u32,
    pub refills: u32,
}

#[derive(Clone, Default)]
pub struct Task {
    pub title: String,
    pub start: Option<u16>, // minutes after midnight
    pub dur: u16,           // minutes, 0 = no end
    pub done_day: Option<NaiveDate>,
    pub fired_start: bool,
    pub fired_end: bool,
    pub snooze_until: u32,
    pub fired_day: Option<NaiveDate>, // the day fired_start / fired_end are for
}

pub fn load_tasks() -> Vec<Task> {
    read("tasks.txt").lines().filter_map(task_from).collect()
}

pub fn save_tasks(t: &[Task]) {
    write("tasks.txt", &t.iter().map(task_line).collect::<Vec<_>>().join("\n"));
}

/// One task: "done|start|dur|title". "done" is the day it was done (or "-"), then
/// ";fired=DAY:se" for the reminders already shown that day (s = start, e = time's up).
fn task_line(t: &Task) -> String {
    let mut d = t.done_day.map(|d| d.to_string()).unwrap_or_else(|| "-".into());
    if let Some(day) = t.fired_day.filter(|_| t.fired_start || t.fired_end) {
        d += &format!(";fired={day}:{}{}", if t.fired_start { "s" } else { "" }, if t.fired_end { "e" } else { "" });
    }
    let st = t.start.map(|s| s.to_string()).unwrap_or_else(|| "-".into());
    format!("{d}|{st}|{}|{}", t.dur, t.title.replace(['\n', '|'], " "))
}

fn task_from(l: &str) -> Option<Task> {
    let mut p = l.splitn(4, '|');
    let first = p.next()?;
    let (done, fired) = match first.split_once(";fired=") {
        Some((d, f)) => (d, f.split_once(':')),
        None => (first, None),
    };
    let start = p.next()?;
    let dur = p.next()?;
    let day = |s: &str| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok();
    let which = fired.map_or("", |f| f.1);
    Some(Task {
        done_day: day(done),
        start: start.parse().ok(),
        dur: dur.parse().unwrap_or(0),
        title: p.next()?.to_string(),
        fired_day: fired.and_then(|f| day(f.0)),
        fired_start: which.contains('s'),
        fired_end: which.contains('e'),
        ..Default::default()
    })
}

/// Clipboard entries are stored one per line with backslashes and newlines escaped.
pub fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\n', "\\n").replace('\r', "")
}

pub fn unesc(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n') => out.push('\n'),
                Some(o) => out.push(o),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn page_order_keeps_only_a_full_ordering() {
        assert_eq!(page_order(vec![7, 0, 1, 2, 3, 4, 5, 6]), vec![7, 0, 1, 2, 3, 4, 5, 6]);
        let usual: Vec<usize> = (0..PAGE_COUNT).collect();
        assert_eq!(page_order(vec![0, 1, 1, 2, 3, 4, 5, 6]), usual); // a page twice, one missing
        assert_eq!(page_order(vec![0, 1, 2]), usual);
        assert_eq!(page_order(vec![]), usual);
        let s = Settings::from_text("page_order=3,0,1,2,4,5,6,7\npages_hidden=2,9", "");
        assert_eq!(s.page_order[0], 3);
        assert_eq!(s.pages_hidden, vec![2]); // 9 is no page
    }

    #[test]
    fn dpapi_seals_and_opens() {
        let key = "AIzaSy-example-not-real";
        let sealed = dpapi(key.as_bytes(), true).unwrap();
        assert!(!sealed.windows(key.len()).any(|w| w == key.as_bytes()), "sealed bytes must not hold the key");
        assert_eq!(dpapi(&sealed, false).unwrap(), key.as_bytes());
        assert!(dpapi(b"not sealed", false).is_none());
    }

    #[test]
    fn escape_round_trip() {
        let s = "a\\b\nc|d";
        assert_eq!(unesc(&esc(s)), s);
    }
    #[test]
    fn task_lines_keep_the_day_s_reminders() {
        let day = NaiveDate::from_ymd_opt(2026, 11, 22).unwrap();
        let t = Task { title: "Họp nhóm".into(), start: Some(540), dur: 30, done_day: None, fired_start: true, fired_day: Some(day), ..Default::default() };
        let line = task_line(&t);
        assert_eq!(line, "-;fired=2026-11-22:s|540|30|Họp nhóm");
        let back = task_from(&line).unwrap();
        assert_eq!((back.fired_day, back.fired_start, back.fired_end, back.title.as_str()), (Some(day), true, false, "Họp nhóm"));
        // A line from before reminders were kept still reads.
        let old = task_from("2026-11-21|600|0|Đọc sách").unwrap();
        assert_eq!((old.done_day, old.start, old.fired_day), (NaiveDate::from_ymd_opt(2026, 11, 21), Some(600), None));
    }

    #[test]
    fn water_goal_steps() {
        assert_eq!(goal_step(1500), 1500);
        assert_eq!(goal_step(1600), 1500);
        assert_eq!(goal_step(1630), 1750);
        assert_eq!(goal_step(0), GOAL_MIN);
        assert_eq!(goal_step(9000), GOAL_MAX);
    }

    #[test]
    fn water_plan_default() {
        let s = Settings::from_text("", "");
        assert_eq!((s.goal, s.bottle, s.from, s.until), (1500, 500, 480, 1020));
        let p = s.water_plan();
        // 08:00-17:00 = 540 min -> min(60, 36) = 36 reminders, 15 min apart, 42 ml each, 2 refills.
        assert_eq!((p.reminders, p.each, p.refills), (36, 42, 2));
        assert!((p.interval - 15.0).abs() < 1e-4);
    }
}
