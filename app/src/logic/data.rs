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
pub const BATTERY_LOWS: [u8; 3] = [15, 20, 30]; // the "Low battery" levels to choose from
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
    pub battery_low: u8, // "Low battery" at or under this % (15, 20 or 30)
    pub popup_wifi: bool,
    pub hide_fullscreen: bool,
    pub welcomed: bool, // the first-start hint has been shown
    pub cal_early: bool, // calendar events also remind 10 minutes before
    pub prayer_asr: usize, // Ashar: 0 as the method does, 1 Shafi'i, 2 Hanafi
    pub prayer_resume_media: bool, // music paused at prayer time plays again 10 minutes later
    pub fahrenheit: bool, // the Weather widget in °F
    pub auto_next: bool, // Pomodoro: the next part starts by itself
    pub focus_quiet: bool, // notifications wait while focusing
    pub lyrics: bool,
    pub hud_volume: bool,
    pub hud_caps: bool,
    pub hud_language: bool,
    pub hud_brightness: bool,
    pub popup_bluetooth: bool,
    pub popup_song: bool, // "Now playing" when the song changes
    pub popup_headset_off: bool, // a headset going away, or its battery running low
    pub notifications: bool,
    pub notification_preview: bool,
    pub mascot: bool,
    pub app_icon: usize, // index into tray::ICON_NAMES
    pub language: usize, // index into i18n::NAMES
    pub widget_city1: usize,
    pub widget_city2: usize,
    pub widget_event: usize,
    pub widget_my_name: String, // the Countdown's own event: its name...
    pub widget_my_date: String, // ...and date, "DD/MM" (every year) or "DD/MM/YYYY"
    pub widget_left: usize,
    pub widget_right: usize,
    pub week_monday: bool,
    pub show_clock: bool,         // the date and time in the notch
    pub page_order: Vec<usize>,   // every page id once, in the order they are paged through
    pub pages_hidden: Vec<usize>, // page ids left out
}

/// Pages the notch has (main.rs PAGES).
pub const PAGE_COUNT: usize = 9;
pub const NOTE_PAGE: usize = 8; // the newest page: its id comes last, its place is after To-do
/// The usual order of the pages.
pub const DEFAULT_ORDER: [usize; PAGE_COUNT] = [0, 1, 2, 3, 4, NOTE_PAGE, 5, 6, 7];

fn ids(m: &BTreeMap<String, String>, k: &str) -> Vec<usize> {
    m.get(k).map(|v| v.split(',').filter_map(|x| x.trim().parse().ok()).collect()).unwrap_or_default()
}

/// The saved order when it lists each page exactly once, else the usual order. An order saved
/// before the Note page existed keeps its pages: the untouched usual order becomes today's usual
/// order, a rearranged one gets Note at the end.
fn page_order(saved: Vec<usize>) -> Vec<usize> {
    let mut sorted = saved.clone();
    sorted.sort_unstable();
    if sorted == (0..PAGE_COUNT).collect::<Vec<_>>() {
        saved
    } else if sorted == (0..NOTE_PAGE).collect::<Vec<_>>() && saved != sorted {
        saved.into_iter().chain([NOTE_PAGE]).collect()
    } else {
        DEFAULT_ORDER.to_vec()
    }
}

fn join(ids: &[usize]) -> String {
    ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",")
}

const PRAYER_ADJ_KEYS: [&str; 5] = ["prayer_adj_subuh", "prayer_adj_dzuhur", "prayer_adj_ashar", "prayer_adj_maghrib", "prayer_adj_isya"];

/// The settings kept in settings.txt under their own field name, listed once: the default, then
/// (after `|v|`) the check a value gets when read. `from_text` and `to_text` both come from it;
/// the few that need more (water.txt, the prayer offsets, text, page lists) are written out inside.
macro_rules! simple_settings {
    ($($f:ident = $d:expr $(, |$v:ident| $fix:expr)?;)*) => {
        impl Settings {
            pub fn from_text(text: &str, water: &str) -> Self {
                let m = kv(text);
                let w = kv(water);
                let mut s = Settings {
                    $($f: {
                        let x = get(&m, stringify!($f), $d);
                        $(let $v = x; let x = $fix;)?
                        x
                    },)*
                    // Water settings live in water.txt (keys from 0x1400e31e0), defaults from 0x140049847.
                    water: get(&w, "on", 1u8) != 0,
                    goal: get(&w, "goal", 1500),
                    bottle: get(&w, "bottle", 500u32).clamp(BOTTLE_MIN, BOTTLE_MAX),
                    from: get(&w, "start", 480u16).min(FROM_MAX),
                    until: get(&w, "end", 1020u16),
                    weather_city: get(&m, "weather_city", get(&m, "prayer_city", crate::prayer::DEFAULT_CITY)).min(crate::prayer::place_count() - 1),
                    prayer_adj: PRAYER_ADJ_KEYS.map(|k| get(&m, k, 10i32) - 10),
                    widget_my_name: m.get("widget_my_name").cloned().unwrap_or_default(),
                    widget_my_date: m.get("widget_my_date").cloned().unwrap_or_default(),
                    page_order: page_order(ids(&m, "page_order")),
                    pages_hidden: ids(&m, "pages_hidden").into_iter().filter(|&i| i < PAGE_COUNT).collect(),
                };
                // Same clamp as the original's loader (0x14004aa9e): until within [from + 60, 24:00].
                s.until = s.until.max(s.from + MIN_SPAN).min(UNTIL_MAX);
                s
            }

            /// settings.txt's text (the water settings live in water.txt).
            pub fn to_text(&self) -> String {
                let s = self;
                let mut lines = vec![$(format!(concat!(stringify!($f), "={}"), s.$f)),*];
                lines.push(format!("weather_city={}", s.weather_city));
                lines.extend(PRAYER_ADJ_KEYS.iter().zip(s.prayer_adj).map(|(k, a)| format!("{k}={}", a + 10)));
                lines.push(format!("widget_my_name={}", s.widget_my_name.replace(['\n', '\r'], " ")));
                lines.push(format!("widget_my_date={}", s.widget_my_date.trim()));
                lines.push(format!("page_order={}", join(&s.page_order)));
                lines.push(format!("pages_hidden={}", join(&s.pages_hidden)));
                lines.join("\n")
            }
        }
    };
}

simple_settings! {
    startup = false;
    size = 0usize, |v| v.min(3);
    minimal = true; // rests as the original's thin peek bar unless turned off
    open_click = false;
    sound = true;
    status_ring = 0usize, |v| v.min(2);
    focus = 25;
    short_break = 5;
    long_break = 15;
    todo_start = true;
    todo_end = true;
    snooze = 10u32;
    water_snooze = WATER_SNOOZE_MIN, |v| if SNOOZES.contains(&v) { v } else { WATER_SNOOZE_MIN };
    prayer = false;
    prayer_city = crate::prayer::DEFAULT_CITY, |v| v.min(crate::prayer::place_count() - 1);
    prayer_method = 0usize, |v| v.min(crate::prayer::METHODS.len());
    prayer_lat = 0.0f64, |v| v.clamp(-90.0, 90.0);
    prayer_lon = 0.0f64, |v| v.clamp(-180.0, 180.0);
    prayer_reminder = true;
    prayer_early = false;
    // Both off by default, as in the original's settings table (0x1400e1320).
    prayer_pause_media = false;
    today_page = false;
    clipboard = false;
    clipboard_max = CLIP_MAX_FREE, |v| v.clamp(1, CLIP_MAX_FREE);
    clipboard_keep = false;
    shelf_clear = false;
    popup_battery = true;
    battery_low = 20u8, |v| if BATTERY_LOWS.contains(&v) { v } else { 20 };
    popup_wifi = true;
    hide_fullscreen = true;
    welcomed = false;
    cal_early = true;
    prayer_asr = 0usize, |v| if v <= 2 { v } else { 0 };
    prayer_resume_media = false;
    fahrenheit = false;
    auto_next = false;
    focus_quiet = false;
    lyrics = true;
    hud_volume = true;
    hud_caps = true;
    hud_language = true;
    hud_brightness = true;
    popup_bluetooth = true;
    popup_song = true;
    popup_headset_off = true;
    notifications = true;
    notification_preview = false;
    mascot = true;
    app_icon = 0usize;
    language = crate::i18n::system_default(), |v| if v < crate::i18n::NAMES.len() { v } else { crate::i18n::system_default() };
    widget_city1 = 5usize;
    widget_city2 = 13usize;
    widget_event = 1usize;
    widget_left = 0usize;
    widget_right = 1usize;
    week_monday = false;
    show_clock = true;
}

impl Settings {
    pub fn load() -> Self {
        Self::from_text(&read("settings.txt"), &read("water.txt"))
    }

    pub fn save(&self) {
        write("settings.txt", &self.to_text());
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

pub const NOTE_MAX_CHARS: usize = 500;
pub const NOTES_MAX: usize = 200;
const NOTE_DONE_KEEP_DAYS: i64 = 3; // a finished note goes this many days after it was done

/// A quick note on the Note page.
#[derive(Clone, PartialEq, Debug)]
pub struct Note {
    pub created: i64, // unix seconds
    pub text: String,
    pub pinned: bool,
    pub done: Option<NaiveDate>, // the day it was marked done
}

pub fn load_notes() -> Vec<Note> {
    let text = read("notes.txt");
    let notes: Vec<Note> = text.lines().filter_map(note_from).collect();
    // A line that doesn't read would be gone at the next save: keep the file as it was first.
    if notes.len() < text.lines().filter(|l| !l.trim().is_empty()).count() {
        write("notes.bak", &text);
    }
    notes
}

pub fn save_notes(n: &[Note]) {
    write("notes.txt", &n.iter().map(note_line).collect::<Vec<_>>().join("\n"));
}

/// One note: "created|flags|text", flags "p" (pinned) and "d2026-11-25" (done that day), comma-separated.
fn note_line(n: &Note) -> String {
    let mut flags = vec![];
    if n.pinned {
        flags.push("p".to_string());
    }
    if let Some(d) = n.done {
        flags.push(format!("d{d}"));
    }
    format!("{}|{}|{}", n.created, flags.join(","), esc(&n.text))
}

fn note_from(l: &str) -> Option<Note> {
    let mut p = l.splitn(3, '|');
    let created = p.next()?.parse().ok()?;
    let flags = p.next()?;
    let text = unesc(p.next()?);
    let done = flags.split(',').find_map(|f| NaiveDate::parse_from_str(f.strip_prefix('d')?, "%Y-%m-%d").ok());
    Some(Note { created, text, pinned: flags.split(',').any(|f| f == "p"), done })
}

/// Cuts long notes, drops ones done over NOTE_DONE_KEEP_DAYS ago, keeps at most NOTES_MAX (the
/// oldest done ones go first, then the oldest open ones), and sorts: pinned, open, done; newest first.
pub fn tidy_notes(n: &mut Vec<Note>, today: NaiveDate) {
    for note in n.iter_mut() {
        if note.text.chars().count() > NOTE_MAX_CHARS {
            note.text = note.text.chars().take(NOTE_MAX_CHARS).collect();
        }
    }
    n.retain(|note| note.done.is_none_or(|d| (today - d).num_days() <= NOTE_DONE_KEEP_DAYS));
    let rank = |note: &Note| if note.done.is_some() { 2 } else if note.pinned { 0 } else { 1 };
    n.sort_by(|a, b| rank(a).cmp(&rank(b)).then(b.created.cmp(&a.created)));
    while n.len() > NOTES_MAX {
        // The last one is the oldest done note while any are left, else the oldest open one.
        let last = n.iter().rposition(|note| !note.pinned || note.done.is_some()).unwrap_or(n.len() - 1);
        n.remove(last);
    }
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
        assert_eq!(page_order(vec![7, 0, 1, 2, 3, 4, 5, 6, 8]), vec![7, 0, 1, 2, 3, 4, 5, 6, 8]);
        let usual: Vec<usize> = DEFAULT_ORDER.to_vec();
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
    fn settings_text_is_unchanged() {
        // Every key and default exactly as settings.txt held them before the keys were listed once.
        const GOLDEN: [&str; 67] = [
            "app_icon=0", "auto_next=false", "battery_low=20", "cal_early=true", "clipboard=false", "clipboard_keep=false",
            "clipboard_max=5", "fahrenheit=false", "focus=25", "focus_quiet=false", "hide_fullscreen=true",
            "hud_brightness=true", "hud_caps=true", "hud_language=true", "hud_volume=true", "language=1", "long_break=15",
            "lyrics=true", "mascot=true", "minimal=true", "notification_preview=false", "notifications=true",
            "open_click=false", "page_order=0,1,2,3,4,8,5,6,7", "pages_hidden=", "popup_battery=true", "popup_bluetooth=true",
            "popup_headset_off=true", "popup_song=true", "popup_wifi=true", "prayer=false", "prayer_adj_ashar=10",
            "prayer_adj_dzuhur=10", "prayer_adj_isya=10", "prayer_adj_maghrib=10", "prayer_adj_subuh=10", "prayer_asr=0",
            "prayer_city=17", "prayer_early=false", "prayer_lat=0", "prayer_lon=0", "prayer_method=0",
            "prayer_pause_media=false", "prayer_reminder=true", "prayer_resume_media=false", "shelf_clear=false",
            "short_break=5", "show_clock=true", "size=0", "snooze=10", "sound=true", "startup=false", "status_ring=0",
            "today_page=false", "todo_end=true", "todo_start=true", "water_snooze=10", "weather_city=17",
            "week_monday=false", "welcomed=false", "widget_city1=5", "widget_city2=13", "widget_event=1", "widget_left=0",
            "widget_my_date=", "widget_my_name=", "widget_right=1",
        ];
        let text = Settings::from_text("language=1", "").to_text();
        let mut lines: Vec<&str> = text.lines().collect();
        lines.sort();
        assert_eq!(lines, GOLDEN);
    }

    #[test]
    fn settings_round_trip() {
        let mut s = Settings::from_text("language=2", "");
        s.size = 2;
        s.battery_low = 30;
        s.prayer_adj = [-3, 0, 4, 0, 10];
        s.prayer_lat = 10.78;
        s.widget_my_name = "Sinh\nnhật".into();
        s.widget_my_date = "02/09".into();
        s.page_order = vec![7, 0, 1, 2, 3, 4, NOTE_PAGE, 5, 6];
        s.pages_hidden = vec![1];
        let back = Settings::from_text(&s.to_text(), "");
        assert_eq!(back.widget_my_name, "Sinh nhật"); // kept on one line
        s.widget_my_name = "Sinh nhật".into();
        assert!(back == s);
    }

    #[test]
    fn older_settings_files_fill_defaults() {
        let mut want = Settings::from_text("language=1", "");
        want.size = 1;
        assert!(Settings::from_text("language=1\nsize=1", "") == want);
    }

    #[test]
    fn bad_values_fall_back_like_before() {
        let s = Settings::from_text("size=9\nbattery_low=17\nlanguage=99\nprayer_asr=5\nwater_snooze=7\nclipboard_max=0", "");
        assert_eq!((s.size, s.battery_low, s.prayer_asr, s.water_snooze, s.clipboard_max), (3, 20, 0, 10, 1));
        assert!(s.language < crate::i18n::NAMES.len());
    }

    #[test]
    fn old_page_orders_gain_note() {
        // Saved before the Note page: the usual order turns into today's usual one (Note after To-do)...
        assert_eq!(Settings::from_text("page_order=0,1,2,3,4,5,6,7", "").page_order, DEFAULT_ORDER.to_vec());
        // ...a rearranged one keeps its order with Note at the end, and hidden pages stay hidden.
        let s = Settings::from_text("page_order=7,0,1,2,3,4,5,6\npages_hidden=2", "");
        assert_eq!((s.page_order, s.pages_hidden), (vec![7, 0, 1, 2, 3, 4, 5, 6, NOTE_PAGE], vec![2]));
        assert_eq!(Settings::from_text("page_order=1,1", "").page_order, DEFAULT_ORDER.to_vec());
    }

    #[test]
    fn note_lines_round_trip() {
        let day = NaiveDate::from_ymd_opt(2026, 11, 25).unwrap();
        let n = Note { created: 1_764_000_000, text: "Gọi anh Tám | hợp đồng\nlink C:\\docs".into(), pinned: true, done: Some(day) };
        let line = note_line(&n);
        assert!(!line.contains('\n'), "{line:?}");
        let back = note_from(&line).unwrap();
        assert_eq!((back.created, back.text.as_str(), back.pinned, back.done), (n.created, n.text.as_str(), true, Some(day)));
        let plain = note_from(&note_line(&Note { created: 5, text: "a".into(), pinned: false, done: None })).unwrap();
        assert_eq!((plain.pinned, plain.done), (false, None));
        assert!(note_from("not a note").is_none());
        assert!(note_from("x|p|text").is_none()); // no time
    }

    #[test]
    fn notes_tidy() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 25).unwrap();
        let ago = |d: i64| today - chrono::Duration::days(d);
        let note = |created: i64, pinned: bool, done: Option<NaiveDate>| Note { created, text: format!("n{created}"), pinned, done };
        let mut n = vec![note(1, false, None), note(2, true, None), note(3, false, Some(ago(1))), note(4, false, Some(ago(4))), note(5, false, None)];
        tidy_notes(&mut n, today);
        // Pinned first, then open newest first, then done; done over 3 days ago is gone.
        assert_eq!(n.iter().map(|n| n.created).collect::<Vec<_>>(), vec![2, 5, 1, 3]);
        let mut long = vec![Note { created: 1, text: "x".repeat(600), pinned: false, done: None }];
        tidy_notes(&mut long, today);
        assert_eq!(long[0].text.chars().count(), NOTE_MAX_CHARS);
        // Too many: the oldest done ones go first, then the oldest open ones.
        let mut many: Vec<Note> = (0..NOTES_MAX as i64).map(|i| note(100 + i, false, None)).collect();
        many.push(note(1, false, Some(today)));
        many.push(note(10_000, false, None));
        tidy_notes(&mut many, today);
        assert_eq!(many.len(), NOTES_MAX);
        assert!(many.iter().all(|n| n.created != 1), "the done one goes first");
        assert!(many.iter().any(|n| n.created == 10_000) && many.iter().all(|n| n.created != 100), "then the oldest open one");
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
