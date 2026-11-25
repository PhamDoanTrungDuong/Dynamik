#![windows_subsystem = "windows"]
//! A notch at the top of the screen: Pomodoro, Water, Shelf, To-do, Prayer and Widgets pages,
//! drawn to match the original's black notch with page dots and spring motion.
// Modules live in folders by role; #[path] keeps their names (crate::tray, crate::media...) as before.
#[path = "logic/data.rs"]
mod data;
#[path = "logic/calc.rs"]
mod calc;
#[path = "logic/prayer.rs"]
mod prayer;
#[path = "logic/i18n.rs"]
mod i18n;
#[path = "net/online.rs"]
mod online;
#[path = "net/calendar.rs"]
mod calendar;
#[path = "platform/tray.rs"]
mod tray;
#[path = "platform/system.rs"]
mod system;
#[path = "platform/media.rs"]
mod media;
#[path = "platform/dragout.rs"]
mod dragout;
#[path = "platform/ocr.rs"]
mod ocr;
#[path = "ui/draw.rs"]
mod draw;
#[path = "ui/controls.rs"]
mod controls;
#[path = "ui/popup.rs"]
mod popup;
#[path = "ui/demo.rs"]
mod demo;
#[path = "ui/pages/mod.rs"]
mod pages;

use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, Timelike, Utc, Weekday};
use data::*;
use i18n::{t, tr};
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontFamily, FontId, Pos2, Rect, Sense, Shape, Stroke, Vec2};
use std::{collections::{BTreeMap, HashSet}, f32::consts::PI, path::PathBuf, time::Instant};
use system::*;
use draw::*;
use controls::*;
use demo::*;

// Look of the original (reference pictures).
const BG: Color32 = Color32::BLACK;
const CARD: Color32 = Color32::from_rgb(20, 20, 20);
const BTN: Color32 = Color32::from_rgb(34, 34, 34);
const ARROW: Color32 = Color32::from_rgb(64, 64, 64);
const WHITE: Color32 = Color32::from_rgb(245, 245, 245);
const DIM: Color32 = Color32::from_rgb(140, 140, 140);
const FAINT: Color32 = Color32::from_rgb(90, 90, 90);
const BLUE: Color32 = Color32::from_rgb(56, 152, 255);
const BLUE_DARK: Color32 = Color32::from_rgb(18, 66, 120);
const GREEN: Color32 = Color32::from_rgb(46, 204, 113);
const GREEN_DARK: Color32 = Color32::from_rgb(16, 48, 26);
const PURPLE: Color32 = Color32::from_rgb(139, 92, 246);
const RED: Color32 = Color32::from_rgb(239, 68, 68);
const ORANGE: Color32 = Color32::from_rgb(245, 158, 11);

// Drawn sizes measured from the reference pictures (points). The original sizes its window per
// state in 0x140056ff1 (pill 160x34, minimal 150x30, open 520 wide, all x screen scale).
const COLLAPSED: Vec2 = vec2(196.0, 38.0);
const MINIMAL: Vec2 = vec2(150.0, 32.0);
const ABOUT_SECS: f32 = 5.0;
const HINT_SECS: f32 = 12.0; // the first-start hint and "shortcut taken": long enough to read
// Scroll (points) that turns one page: under one wheel notch (egui counts a notch as 40),
// read unsmoothed so a notch arrives whole; touchpad swipes add up to it.
const WHEEL_STEP: f32 = 30.0;
const WHEEL_REST_SECS: f32 = 0.35; // after a turn, the rest of that flick is ignored
const NOTICE: Vec2 = vec2(380.0, 44.0); // a notification: one calm line (the original's 320 x 36 state)
const NOTICE_TALL: Vec2 = vec2(420.0, 60.0); // ...and with the message on a second line
const PEEK: Vec2 = vec2(56.0, 8.0); // the original's peek state: a thin bar while nothing is going on
const EXPANDED: Vec2 = vec2(548.0, 184.0);
const POPUP: Vec2 = vec2(404.0, 80.0);
const SETTINGS: Vec2 = vec2(660.0, 450.0); // the section list on the left, one section at a time on the right
const LYRICS: Vec2 = vec2(676.0, 184.0); // Music page with lyrics beside it (reference picture 12)
const WIN: Vec2 = vec2(LYRICS.x + 2.0 * EAR_MAX, SETTINGS.y);
const EAR_MAX: f32 = 14.0; // side margin of the pill states in 0x140056ff1
// Springs set on every state change in 0x14007dae7: (stiffness, damping).
const SPRING: (f32, f32) = (230.0, 19.0); // most changes: open, close, pop-up (a little bounce)
const SPRING_FROM_POPUP: (f32, f32) = (110.0, 15.5); // leaving a pop-up for the pill
const SPRING_TO_PEEK: (f32, f32) = (200.0, 29.0); // into the small peek state (no bounce)
// Battery pop-ups (0x140056c76): warnings below 21 % (now the chosen level, battery_low) and 11 %; they stay 4.5 s (timer 4 in 0x14007f665).
const BATTERY_CRITICAL: u8 = 11;
const BATTERY_POPUP_SECS: f32 = 4.5;
// Prayer reminders (0x140002345): at prayer time 10 s (timer 4, 10000 ms), the early one 6 s (6000 ms).
const PRAYER_POPUP_SECS: f32 = 10.0;
const PRAYER_EARLY_SECS: f32 = 6.0;
const PRAYER_WINDOW: (f32, f32) = (-0.05, 2.0); // minutes around the due time
const PRAYER_CATCHUP_MIN: f32 = 30.0; // a missed prayer reminder still shows this long after
const PRAYER_LABEL_GAP: f32 = 42.0; // the day arc's labels stay this far apart (points)
const PRAYER_RESUME_MIN: u64 = 10; // music paused at prayer time plays again this long after (if asked)
const IMSAK_BEFORE_MIN: u32 = 10; // Imsak, as Indonesia keeps it: 10 minutes before Subuh/Fajr
// The indicator (volume, Caps Lock, input language) pop-up: its hide timer is 1.4 s (timer 6, 0x14007df9e).
const HUD_SECS: f32 = 1.4;
// Network pop-up (sub_14000311f): a change must hold 1.5 s (timer 16), then shows for 3.2 s (timer 4).
const NET_SETTLE_SECS: f32 = 1.5;
const NET_POPUP_SECS: f32 = 3.2;
// Bluetooth headset and mirrored-notification pop-ups: how long they stay is estimated (not found).
const BT_POPUP_SECS: f32 = 3.2;
const NOTICE_POPUP_SECS: f32 = 4.5; // a short notification; longer ones get more (notice_secs)
const NOTICE_MAX_SECS: f32 = 8.0;
const NOTICE_POP_SECS: f32 = 0.45; // the icon's pop on arrival
const NOTICE_SWEEP_SECS: f32 = 0.9; // the light running along the bottom on arrival
const NOTICE_AFTER_HOVER_SECS: f32 = 1.0; // after the pointer leaves a notification it may close
const HUD: Vec2 = vec2(346.0, 64.0); // state 4 of 0x140056ff1 is 310 x 60 plus the pill's drawn extra

// Page order of the original (its page dots): Music first, Widgets last.
// Today comes last and only with its setting on (the original's `today_page`, off by default).
// Ids are fixed (they are saved in page_order); Note came later, so it is 8 but sits after To-do.
const PAGES: [&str; PAGE_COUNT] = ["Music", "Pomodoro", "Water", "Shelf", "To-do", "Prayer", "Widgets", "Today", "Note"];
const SHELF_PAGE: usize = 3;
const TODAY_PAGE: usize = 7;
// How many notifications the Today page remembers (estimated; not found in the original).
const RECENT_MAX: usize = 20;
// The mascot blinks every 4.2 s; the lids close and open again over 180 ms (estimated).
const BLINK_EVERY_MS: i64 = 4200;
const BLINK_MS: i64 = 180;
const LONG_BREAK_EVERY: u32 = 4;
const WATER_LOG_DAYS: usize = 60; // days of water history kept
const CAL_EARLY_MIN: u32 = 10; // calendar events also remind this many minutes before
const SONG_POPUP_SECS: f32 = 3.0; // "Now playing" stays this long
const SONG_QUIET_START_SECS: u64 = 10; // ...and doesn't show in the app's first seconds
const HEADSET_LOW: u8 = 20; // a headset's battery at or under this (%) says so once
const ASK_HISTORY_MAX: usize = 20; // questions ↑ can bring back
const SKIP_SECS: f64 = 10.0; // back / forward, for a player with no previous or next track
const UNDO_SECS: f32 = 10.0; // how long the Water page offers to take a drink back
const SLEEP_GAP_SECS: f64 = 5.0; // a gap this long between looks while Pomodoro runs = the PC slept
const RESET_CONFIRM_SECS: f32 = 3.0;
const MOON: [&str; 8] = ["New Moon", "Waxing Crescent", "First Quarter", "Waxing Gibbous", "Full Moon", "Waning Gibbous", "Last Quarter", "Waning Crescent"];
// Countdown events. Settings keep an event's place in this list, so new ones go at the end.
const EVENTS: [&str; 9] = ["New Year", "Ramadan", "Eid al-Fitr", "Eid al-Adha", "Indonesia's Independence Day", "Christmas", "Lunar New Year (Tết)", "Vietnam's National Day", "My own date"];
const MY_EVENT: usize = 8; // "My own date": a name and date of the person's choosing
// The first day of the lunar year (Chinese calendar). shortcut: Vietnam's Tết is the same day in
// these years as far as known (it has differed by a day, as in 2007); extend the table past 2040.
const LUNAR_NEW_YEAR: [(i32, u32, u32); 15] = [
    (2026, 2, 17), (2027, 2, 6), (2028, 1, 26), (2029, 2, 13), (2030, 2, 3), (2031, 1, 23), (2032, 2, 11), (2033, 1, 31),
    (2034, 2, 19), (2035, 2, 8), (2036, 1, 28), (2037, 2, 15), (2038, 2, 4), (2039, 1, 24), (2040, 2, 12),
];
const WIDGETS: [&str; 9] = ["This week", "Battery", "Next prayer", "Focus today", "World clock", "Moon", "Countdown", "Water", "Weather"];
const WEATHER_WIDGET: usize = 8;
// Estimated Islamic dates (official announcements can differ by a day).
const RAMADAN: [(i32, u32, u32); 5] = [(2026, 2, 18), (2027, 2, 8), (2028, 1, 28), (2029, 1, 16), (2030, 1, 5)];
const EID_FITR: [(i32, u32, u32); 5] = [(2026, 3, 20), (2027, 3, 10), (2028, 2, 27), (2029, 2, 14), (2030, 2, 4)];
const EID_ADHA: [(i32, u32, u32); 5] = [(2026, 5, 27), (2027, 5, 16), (2028, 5, 5), (2029, 4, 24), (2030, 4, 13)];

#[derive(Clone, Copy)]
enum Dst { No, Eu, Us, Au }
// Order and defaults (Tokyo, London) from the original's table at 0x1400e40d8.
// World clock cities. Settings keep a city's place in this list, so new ones go at the end.
const WORLD: [(&str, f64, Dst); 27] = [
    ("Jakarta", 7.0, Dst::No), ("Makassar", 8.0, Dst::No), ("Jayapura", 9.0, Dst::No), ("Singapore", 8.0, Dst::No),
    ("Kuala Lumpur", 8.0, Dst::No), ("Tokyo", 9.0, Dst::No), ("Seoul", 9.0, Dst::No), ("Beijing", 8.0, Dst::No),
    ("Mumbai", 5.5, Dst::No), ("Dubai", 4.0, Dst::No), ("Mecca", 3.0, Dst::No), ("Istanbul", 3.0, Dst::No),
    ("Cairo", 2.0, Dst::Eu), ("London", 0.0, Dst::Eu), ("Paris", 1.0, Dst::Eu), ("Berlin", 1.0, Dst::Eu),
    ("New York", -5.0, Dst::Us), ("Los Angeles", -8.0, Dst::Us), ("Sydney", 10.0, Dst::Au),
    ("Hà Nội", 7.0, Dst::No), ("TP. Hồ Chí Minh", 7.0, Dst::No), ("Bangkok", 7.0, Dst::No), ("Manila", 8.0, Dst::No),
    ("Hong Kong", 8.0, Dst::No), ("Taipei", 8.0, Dst::No), ("Moscow", 3.0, Dst::No), ("Toronto", -5.0, Dst::Us),
];

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    let shot = args.iter().position(|a| a == "--screenshot").and_then(|i| args.get(i + 1)).map(PathBuf::from);
    // --page N opens on that page (used to check each page's look).
    // --resting keeps the notch closed (and --mascot turns the mascot on) for checking those looks;
    // --settings opens Settings.
    let resting = args.iter().any(|a| a == "--resting");
    let mascot_on = args.iter().any(|a| a == "--mascot");
    let page: usize = args.iter().position(|a| a == "--page").and_then(|i| args.get(i + 1)).and_then(|p| p.parse().ok()).unwrap_or(0);
    // Screenshots run beside the real notch, so only a real start is limited to one.
    if shot.is_none() && tray::already_running() {
        return Ok(());
    }
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("My Notch")
            .with_inner_size(if shot.is_some() { WIN } else { COLLAPSED + vec2(2.0 * EAR_MAX, 0.0) })
            .with_position([600.0, 0.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false)
            .with_taskbar(false)
            .with_window_level(egui::WindowLevel::AlwaysOnTop)
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native("My Notch", opts, Box::new(|cc| Ok(Box::new(App::new(&cc.egui_ctx, shot, page, resting, mascot_on)))))
}

fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("bold".into()))
}
fn reg(size: f32) -> FontId {
    FontId::proportional(size)
}

/// Segoe UI like the original, when Windows has it; egui's own fonts stay as fallbacks.
fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let dir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
    let load = |f: &str| std::fs::read(format!(r"{dir}\Fonts\{f}")).ok();
    let mut bold_family = vec![];
    if let Some(b) = load("segoeui.ttf") {
        fonts.font_data.insert("segoe".into(), std::sync::Arc::new(egui::FontData::from_owned(b)));
        fonts.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "segoe".into());
    }
    // Segoe UI has no Chinese; Microsoft YaHei comes with Windows. Loaded only for Chinese (~20 MB).
    if i18n::lang() == i18n::ZH {
        if let Some(b) = load("msyh.ttc").or_else(|| load("msyh.ttf")).or_else(|| load("simsun.ttc")).or_else(|| load("Deng.ttf")) {
            fonts.font_data.insert("yahei".into(), std::sync::Arc::new(egui::FontData::from_owned(b)));
            let fam = fonts.families.get_mut(&FontFamily::Proportional).unwrap();
            fam.insert(fam.len().min(1), "yahei".into());
        } else {
            // No Chinese font on this Windows: English reads, boxes don't. Callers save what was kept.
            i18n::set(i18n::EN);
        }
    }
    if let Some(b) = load("seguisb.ttf").or_else(|| load("segoeuib.ttf")) {
        fonts.font_data.insert("segoe-bold".into(), std::sync::Arc::new(egui::FontData::from_owned(b)));
        bold_family.push("segoe-bold".to_string());
    }
    bold_family.extend(fonts.families[&FontFamily::Proportional].iter().cloned());
    fonts.families.insert(FontFamily::Name("bold".into()), bold_family);
    ctx.set_fonts(fonts);
}

#[derive(Clone, Copy, PartialEq)]
enum Mode { Focus, Short, Long }

#[derive(Clone, Copy)]
enum HudKind { Volume, Muted, Brightness, Caps, Lang }
/// Asking Windows where this PC is, for "My location".
#[derive(Clone, Copy, PartialEq)]
enum Locating { Idle, Busy, Failed, Got(f64, f64) }
enum Kind { Drink, Refill, TaskStart(usize), TaskEnd(usize), Event(u32), Hint, Song, Prayer(u32, bool), Battery(u8, bool), Hud(HudKind, f32), Network(u8), Bluetooth(Option<u8>), Notice(String, String, u32), Info, About }
struct Alert { title: String, body: String, kind: Kind, at: Instant }

/// A critically-tuned spring for one value.
#[derive(Clone, Copy)]
struct Spring { x: f32, v: f32 }
impl Spring {
    fn step(&mut self, target: f32, dt: f32, (k, c): (f32, f32)) -> bool {
        let a = k * (target - self.x) - c * self.v;
        self.v += a * dt;
        self.x += self.v * dt;
        let settled = (target - self.x).abs() < 0.3 && self.v.abs() < 0.5;
        if settled {
            self.x = target;
            self.v = 0.0;
        }
        !settled
    }
}

#[derive(PartialEq, Clone, Copy)]
enum ShelfView { Files, Clipboard }

struct App {
    s: Settings,
    now: NaiveDateTime,
    page: usize,
    page_anim: f32,
    settings_open: bool,
    // The Ask box (Ctrl + Alt + Space): what is typed, the answer, and a focus request for the field.
    ask_open: bool,
    ask_text: String,
    ask_answer: String,
    ask_focus: bool,
    // Gemini: the user's key (kept sealed on disk), and the answer to question number `ask_gen`
    // while `ask_waiting`; an older question's late answer is dropped.
    gemini_key: String,
    ask_gen: u64,
    ask_waiting: bool,
    ask_reply: std::sync::Arc<std::sync::Mutex<(u64, String, bool)>>, // (question number, text, whether it is an answer)
    ask_history: Vec<String>, // questions asked (in memory only), for ↑ / ↓
    ask_hist_at: Option<usize>, // which one ↑ / ↓ shows now
    ask_copyable: bool, // the shown text is Gemini's answer (not "Thinking..." or an error)
    // Calendar: the secret iCal address (sealed too), what the watcher read, and reminders shown.
    ical_text: String,
    ical_url: std::sync::Arc<std::sync::Mutex<String>>,
    calendar: std::sync::Arc<std::sync::Mutex<calendar::Today>>,
    cal_fired: HashSet<(NaiveDate, u32, String, bool)>, // (day, start, title, the early one)
    // Shortcuts: whether a taken one still has to be told. Page turning by scroll: the scroll
    // gathered so far, when it last moved, and until when a flick is ignored after a turn;
    // `scroll_zone` is a list on the page that scrolls itself (no page turns over it).
    hotkeys_told: bool,
    wheel_acc: f32,
    wheel_last: Instant,
    wheel_rest: Instant,
    scroll_zone: Option<(Rect, bool)>, // (where, whether sideways scroll may still turn pages)
    logos: std::collections::HashMap<String, egui::TextureHandle>, // notification logos by app id
    art_tex: Option<(String, egui::TextureHandle)>, // the playing song's cover, by song
    locating: std::sync::Arc<std::sync::Mutex<Locating>>,
    ocr: std::sync::Arc<std::sync::Mutex<Option<ocr::Outcome>>>,
    expanded: bool,
    held: bool, // opened from the tray: stays open until the pointer has come in and left
    last_inside: Instant,
    applied: Option<[i32; 3]>,
    alert: Option<Alert>,
    shot: Option<PathBuf>,
    frames: u32,
    w: Spring,
    h: Spring,
    content: Spring, // 0..1 content opacity
    spring: (f32, f32),
    last_target: Vec2,
    // to-do
    tasks: Vec<Task>,
    notes: Vec<Note>,
    note_draft: String, // the Note page's input line
    note_focus: bool,   // put the cursor in the note box on the next frame (Ctrl+Alt+N)
    note_copied: Option<(usize, Instant)>, // which note was just copied (by place: texts can repeat)
    note_select: Option<HashSet<usize>>, // choosing notes to delete: the places ticked
    note_delete_at: Option<Instant>,      // the first click of "Delete (N)" (a second within 3 s deletes)
    form: Option<(Option<usize>, String, String, u16)>, // (editing index, title, "HH:MM", minutes)
    todo_edit: bool,
    // water
    water_day: NaiveDate,
    drank: u32,
    in_bottle: u32,
    hours: [u32; 24],
    fired: u64,
    water_snooze: Option<u32>,
    water_settings: bool,
    reset_armed: Option<Instant>,
    settings_reset: Option<Instant>, // the first click of Settings › Restore (a second one within 3 s restores)
    settings_tab: usize, // the Settings section chosen on the left
    last_drink: Option<(u32, usize, u32, Instant)>, // (ml, hour, ml taken from the bottle, when) for Undo
    water_log: BTreeMap<NaiveDate, u32>, // ml drunk on earlier days (water_log.txt)
    // focus
    mode: Mode,
    left: f64,
    running: bool,
    rounds: u32,
    focus_log: BTreeMap<NaiveDate, u32>,
    rounds_day: NaiveDate, // the day `rounds` counts for
    prayer_paused: Option<Instant>, // when music was paused for prayer (for "Play again after")
    last_song: Option<String>, // the song last seen playing (title and artist), for "Now playing"
    started: Instant,
    battery_news: Option<Alert>, // a battery pop-up waiting for one that needs an answer to go
    held_notice: Option<(media::Notice, u32)>, // a notification kept back during quiet focus, and how many more
    focus_acc: f64,
    last_tick: Instant,
    // shelf
    shelf: Vec<PathBuf>,
    shelf_view: ShelfView,
    clips: Vec<(i64, String)>, // (unix seconds, text)
    clipboard: Option<arboard::Clipboard>,
    last_clip: String,
    copied: Option<(String, Instant)>,
    // system
    battery: Option<(u8, bool)>,
    prayer_fired: HashSet<(NaiveDate, usize, bool)>,
    hidden: bool, // hidden while something runs full screen
    media: media::Media,
    lyrics_open: bool,
    resting: bool,
    volume: std::sync::Arc<std::sync::Mutex<Option<(f32, bool)>>>,
    last_volume: Option<(f32, bool)>,
    last_caps: bool,
    last_lang: u16,
    network: std::sync::Arc<std::sync::Mutex<Option<(String, u8)>>>,
    last_network: Option<(String, u8)>,
    bluetooth: std::sync::Arc<std::sync::Mutex<Option<Vec<media::Headset>>>>,
    last_headsets: Option<HashSet<u64>>,
    headset_names: std::collections::HashMap<u64, String>, // the headsets last seen, to name one that goes away
    headset_low_told: HashSet<u64>, // headsets already told of a low battery (until charged again)
    notices: std::sync::Arc<std::sync::Mutex<Vec<media::Notice>>>,
    // Where the Weather widget looks (None while it isn't shown), what it found, and currency rates.
    weather_place: std::sync::Arc<std::sync::Mutex<Option<(f64, f64)>>>,
    weather: std::sync::Arc<std::sync::Mutex<Option<online::Weather>>>,
    rates: online::SharedRates,
    last_bright: u32,
    recent: Vec<(i64, media::Notice)>,
    network_change: Option<((String, u8), Instant)>,
}

impl App {
    fn new(ctx: &egui::Context, shot: Option<PathBuf>, page: usize, resting: bool, mascot_on: bool) -> Self {
        let live = shot.is_none();
        if live {
            tray::start(ctx);
        }
        let mut v = egui::Visuals::dark();
        v.panel_fill = Color32::TRANSPARENT;
        v.window_fill = BG;
        v.override_text_color = Some(WHITE);
        v.extreme_bg_color = CARD;
        v.selection.bg_fill = BLUE_DARK;
        ctx.set_visuals(v);
        let mut s = Settings::load();
        if live {
            tray::set_icon(s.app_icon);
        }
        let weather_place = std::sync::Arc::new(std::sync::Mutex::new(None));
        // --lang N shows a screenshot in that language (in memory only).
        if let Some(l) = shot.as_ref().and(std::env::args().skip_while(|a| a != "--lang").nth(1)).and_then(|l| l.parse().ok()) {
            s.language = l;
        }
        i18n::set(s.language);
        setup_fonts(ctx);
        s.language = i18n::lang();
        s.mascot |= mascot_on;
        // A screenshot of the Today page needs the page on (in memory only; settings are saved only from Settings).
        s.today_page |= shot.is_some() && page == TODAY_PAGE;
        let shot_demo = shot.is_some() && page == TODAY_PAGE && std::env::args().any(|a| a == "--demo");
        let shot_notes = shot.is_some() && page == NOTE_PAGE && std::env::args().any(|a| a == "--demo");
        let shot_demo_ask = shot.is_some() && std::env::args().any(|a| a == "--ask") && std::env::args().any(|a| a == "--demo");
        let now = Local::now().naive_local();
        let w = kv(&read("water.txt"));
        let water_day = w.get("day").and_then(|d| d.parse().ok()).unwrap_or(now.date());
        let same_day = water_day == now.date();
        let num = |k: &str| w.get(k).and_then(|d| d.parse::<u64>().ok());
        let drank = if same_day { num("taken").unwrap_or(0) as u32 } else { 0 };
        let fired = if same_day { num("fired").unwrap_or(0) } else { 0 };
        let mut hours = [0u32; 24];
        if same_day {
            for (i, v) in w.get("hours").map(|h| h.split(',').collect::<Vec<_>>()).unwrap_or_default().iter().take(24).enumerate() {
                hours[i] = v.trim().parse().unwrap_or(0);
            }
        }
        let in_bottle = (num("inb").unwrap_or(s.bottle as u64) as u32).min(s.bottle);
        let mut water_log: BTreeMap<NaiveDate, u32> = read("water_log.txt")
            .lines()
            .filter_map(|l| l.split_once(' ').and_then(|(d, ml)| Some((d.parse().ok()?, ml.trim().parse().ok()?))))
            .collect();
        // water.txt from an earlier day: that day's total joins the history, and today starts here.
        if !same_day {
            water_log.insert(water_day, num("taken").unwrap_or(0) as u32);
        }
        let water_day = if same_day { water_day } else { now.date() };
        let focus_log = read("focus.txt")
            .lines()
            .filter_map(|l| l.split_once(' ').and_then(|(d, m)| Some((d.parse().ok()?, m.parse().ok()?))))
            .collect();
        let shelf = if s.shelf_clear { vec![] } else { read("shelf.txt").lines().filter(|l| !l.is_empty()).map(PathBuf::from).collect() };
        let clips = if s.clipboard_keep {
            read("clipboard.txt")
                .lines()
                .filter_map(|l| l.split_once('\t').map(|(t, c)| (t.parse().unwrap_or(0), unesc(c))))
                .take(s.clipboard_max)
                .collect()
        } else {
            vec![]
        };
        let mut clipboard = arboard::Clipboard::new().ok();
        let last_clip = clipboard.as_mut().and_then(|c| c.get_text().ok()).unwrap_or_default();
        let left = s.focus as f64 * 60.0;
        let start = if shot.is_some() && !resting { EXPANDED } else { COLLAPSED };
        // The very first start says how to open the notch, once.
        let welcome = live && !s.welcomed;
        if welcome {
            s.welcomed = true;
            s.save();
        }
        // "Start with Windows" shows what Windows will really do (Task Manager can turn it off), and an
        // entry left pointing at an exe that is gone (moved, reinstalled) points at this one again.
        if live {
            let real = startup_state();
            let was = s.startup;
            s.startup = real.as_ref().is_some_and(|(_, approved)| *approved);
            if let Some((path, true)) = &real {
                if !std::path::Path::new(path).exists() {
                    set_startup(true);
                }
            }
            if s.startup != was {
                s.save();
            }
        }
        // Screenshots never read the secrets.
        let secret = |name: &str| if live { read_secret(name) } else { String::new() };
        let ical_text = secret(ICAL_FILE);
        let ical_url = std::sync::Arc::new(std::sync::Mutex::new(ical_text.clone()));
        App {
            gemini_key: secret(GEMINI_FILE), ask_gen: 0, ask_waiting: false, ask_reply: Default::default(), ask_copyable: shot_demo_ask, ask_history: Vec::new(), ask_hist_at: None,
            calendar: if live {
                calendar::watch(ical_url.clone())
            } else {
                // --demo on a screenshot of To-do shows made-up events.
                let demo = std::env::args().any(|a| a == "--demo") && (page == 4 || page == TODAY_PAGE);
                let ev = |title: &str, start, dur, carried| calendar::Event { title: title.into(), start, dur, carried };
                let events = if demo { vec![ev("Night shift", None, 90, true), ev("Team trip", None, 0, false), ev("Standup", Some(540), 30, false)] } else { vec![] };
                std::sync::Arc::new(std::sync::Mutex::new(calendar::Today { status: calendar::Status::Ready, events }))
            },
            ical_text, ical_url, cal_fired: HashSet::new(),
            hotkeys_told: !live, wheel_acc: 0.0, wheel_last: Instant::now(), wheel_rest: Instant::now(), scroll_zone: None, logos: Default::default(), art_tex: None,
            now, page: page.min(PAGES.len() - 1), page_anim: page.min(PAGES.len() - 1) as f32, settings_open: shot.is_some() && std::env::args().any(|a| a == "--settings"),
            ask_open: shot.is_some() && std::env::args().any(|a| a == "--ask"),
            // --ask --demo shows a made-up long answer, to check how it wraps.
            ask_text: if shot_demo_ask { "Why is the sky blue?".into() } else { String::new() },
            ask_answer: if shot_demo_ask { DEMO_ANSWER.into() } else { String::new() },
            ask_focus: false,
            locating: std::sync::Arc::new(std::sync::Mutex::new(Locating::Idle)),
            ocr: Default::default(),
            expanded: shot.is_some() && !resting, held: false, resting, last_inside: Instant::now(), applied: None,
            alert: if shot.is_some() { demo_alert() } else if welcome { Some(welcome_hint(s.open_click)) } else { None }, shot, frames: 0,
            w: Spring { x: start.x, v: 0.0 }, h: Spring { x: start.y, v: 0.0 }, content: Spring { x: 0.0, v: 0.0 }, spring: SPRING, last_target: start,
            tasks: load_tasks(), form: None, todo_edit: false,
            // Tidied at start too, so notes done over 3 days ago are gone before the first change.
            notes: if shot_notes { demo_notes() } else { let mut n = load_notes(); tidy_notes(&mut n, now.date(), s.note_done_days); n },
            note_draft: String::new(), note_focus: false, note_copied: None, note_delete_at: None,
            // A screenshot of choosing notes: --page 8 --demo --select (the first two ticked).
            note_select: (shot_notes && std::env::args().any(|a| a == "--select")).then(|| HashSet::from([0, 1])),
            water_day, drank, in_bottle, hours, fired, water_snooze: None, water_settings: false, reset_armed: None, settings_reset: None,
            // A screenshot can open a section: --settings 4.
            settings_tab: std::env::args().skip_while(|a| a != "--settings").nth(1).and_then(|n| n.parse().ok()).unwrap_or(0),
            // --page 2 --demo shows the Water page just after a 250 ml drink (with its Undo).
            water_log,
            last_drink: (!live && page == 2 && std::env::args().any(|a| a == "--demo")).then(|| (250, now.hour() as usize, 0, Instant::now())),
            mode: Mode::Focus, left, running: false, rounds: 0, rounds_day: now.date(), prayer_paused: None, last_song: None, started: Instant::now(), battery_news: None, held_notice: None, focus_log, focus_acc: 0.0, last_tick: Instant::now(),
            // --clips: a screenshot of the clipboard side.
            shelf, shelf_view: if !live && std::env::args().any(|a| a == "--clips") { ShelfView::Clipboard } else { ShelfView::Files },
            clips, clipboard, last_clip, copied: None,
            battery: battery(), prayer_fired: HashSet::new(), hidden: false, media: demo_music(media::Media::start(s.lyrics, live), !live && page == 0), lyrics_open: false, volume: media::volume_watch(), last_volume: None, last_caps: caps_lock(), last_lang: input_lang(), network: media::network_watch(), last_network: None,
            bluetooth: if live { media::bluetooth_watch() } else { Default::default() }, last_headsets: None, headset_names: Default::default(), headset_low_told: HashSet::new(),
            notices: if live && s.notifications { media::notification_watch() } else { Default::default() },
            weather_place: weather_place.clone(),
            // --weather SYMBOL fills a screenshot's Weather widget (no network).
            weather: if live {
                online::weather_watch(weather_place)
            } else {
                let demo = std::env::args().skip_while(|a| a != "--weather").nth(1).map(|symbol| online::Weather { temp: 28.0, symbol, high: Some(31.0), low: Some(24.0), rain_6h: Some(2.4) });
                std::sync::Arc::new(std::sync::Mutex::new(demo))
            },
            rates: Default::default(), last_bright: 0, recent: if shot_demo { demo_recent() } else { Vec::new() }, network_change: None, s,
        }
    }

    /// The pages shown, in the user's order; Today only with its setting on. Never empty.
    fn order(&self) -> Vec<usize> {
        let shown: Vec<usize> = self.s.page_order.iter().copied()
            .filter(|&p| !self.s.pages_hidden.contains(&p) && (p != TODAY_PAGE || self.s.today_page))
            .collect();
        if shown.is_empty() { vec![0] } else { shown }
    }

    /// The prayer place's name: the chosen city, or "My location".
    fn city_name(&self) -> String {
        if self.s.prayer_city == prayer::here_index() { t("My location").into() } else { prayer::place_name(self.s.prayer_city).1.into() }
    }

    /// Where the current page sits in `order`.
    fn pos(&self) -> usize {
        self.order().iter().position(|&p| p == self.page).unwrap_or(0)
    }
    fn today(&self) -> NaiveDate { self.now.date() }
    fn now_min(&self) -> u32 { self.now.hour() * 60 + self.now.minute() }

    fn notify(&mut self, title: &str, body: &str, kind: Kind) {
        if self.s.sound {
            beep();
        }
        self.alert = Some(Alert { title: title.into(), body: body.into(), kind, at: Instant::now() });
    }

    fn save_water(&self) {
        // Same keys as the original's water.txt (0x1400e31e0).
        let s = &self.s;
        let hours = self.hours.iter().map(|h| h.to_string()).collect::<Vec<_>>().join(",");
        let text = format!(
            "bottle={}\ngoal={}\nstart={}\nend={}\nday={}\ntaken={}\ninb={}\nfired={}\nhours={}\non={}",
            s.bottle, s.goal, s.from, s.until, self.water_day, self.drank, self.in_bottle, self.fired, hours, s.water as u8
        );
        write("water.txt", &text);
        // One line a day ("2026-11-22 1250"), the last 60 days, today's as it stands.
        let mut log = self.water_log.clone();
        log.insert(self.water_day, self.drank);
        let keep = log.len().saturating_sub(WATER_LOG_DAYS);
        write("water_log.txt", &log.iter().skip(keep).map(|(d, ml)| format!("{d} {ml}")).collect::<Vec<_>>().join("\n"));
    }
    fn save_shelf(&self) {
        write("shelf.txt", &self.shelf.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join("\n"));
    }
    fn save_clips(&self) {
        let text = if self.s.clipboard_keep { self.clips.iter().map(|(t, c)| format!("{t}\t{}", esc(c))).collect::<Vec<_>>().join("\n") } else { String::new() };
        write("clipboard.txt", &text);
    }
    fn save_focus(&self) {
        write("focus.txt", &self.focus_log.iter().map(|(d, m)| format!("{d} {m}")).collect::<Vec<_>>().join("\n"));
    }
    fn mode_len(&self, m: Mode) -> f64 {
        60.0 * match m { Mode::Focus => self.s.focus, Mode::Short => self.s.short_break, Mode::Long => self.s.long_break } as f64
    }
    /// Minute of day of the next water reminder not shown yet.
    fn next_water_slot(&self) -> Option<f32> {
        let p = self.s.water_plan();
        let now = self.now_min() as f32;
        (0..p.reminders.min(64))
            .map(|i| (i, self.s.from as f32 + (i as f32 + 0.5) * p.interval))
            .find(|&(i, t)| self.fired & (1 << i) == 0 && t + 2.0 > now)
            .map(|(_, t)| t)
    }
    /// Whether the prayer place is known: a city, or "My location" once found.
    fn prayer_placed(&self) -> bool {
        self.s.prayer_city != prayer::here_index() || (self.s.prayer_lat, self.s.prayer_lon) != (0.0, 0.0)
    }

    /// Prayer times with the per-prayer adjustments from Settings (0x1400863af).
    fn prayer_times(&self, day: NaiveDate) -> [u32; 5] {
        let asr = [prayer::Asr::Method, prayer::Asr::Shafii, prayer::Asr::Hanafi][self.s.prayer_asr.min(2)];
        let t = prayer::times_with(day, self.s.prayer_city, (self.s.prayer_lat, self.s.prayer_lon), self.s.prayer_method.checked_sub(1), asr);
        std::array::from_fn(|i| (t[i] as i32 + self.s.prayer_adj[i]).clamp(0, 1439) as u32)
    }
    fn focus_today(&self) -> u32 { self.focus_log.get(&self.today()).copied().unwrap_or(0) }
    fn done_today(&self) -> usize { self.tasks.iter().filter(|t| t.done_day == Some(self.today())).count() }

    fn drink(&mut self) {
        let each = self.s.water_plan().each;
        self.drink_ml(each);
    }

    /// Counts `ml` as drunk now, and remembers it for Undo.
    fn drink_ml(&mut self, ml: u32) {
        let hour = self.now.hour() as usize;
        let from_bottle = self.in_bottle.min(ml);
        self.drank += ml;
        self.hours[hour] += ml;
        self.in_bottle -= from_bottle;
        self.last_drink = Some((ml, hour, from_bottle, Instant::now()));
        self.save_water();
    }

    /// Millilitres drunk on each of the last 7 days, oldest first; today's is the live count.
    fn water_week(&self) -> [u32; 7] {
        let today = self.today();
        std::array::from_fn(|k| {
            let day = today - Duration::days(6 - k as i64);
            if day == today { self.drank } else { self.water_log.get(&day).copied().unwrap_or(0) }
        })
    }

    /// The water reminder slots whose time has come today, as `fired` bits.
    fn water_slots_past(&self) -> u64 {
        let p = self.s.water_plan();
        let now = self.now_min() as f32 + self.now.second() as f32 / 60.0;
        (0..p.reminders.min(64))
            .filter(|&i| now >= self.s.from as f32 + (i as f32 + 0.5) * p.interval)
            .fold(0, |bits, i| bits | 1 << i)
    }

    /// Takes the last drink back (the Undo shown for a few seconds after it).
    fn undo_drink(&mut self) {
        if let Some((ml, hour, from_bottle, _)) = self.last_drink.take() {
            self.drank = self.drank.saturating_sub(ml);
            self.hours[hour] = self.hours[hour].saturating_sub(ml);
            self.in_bottle = (self.in_bottle + from_bottle).min(self.s.bottle);
            self.save_water();
        }
    }

    /// Moves Pomodoro on to its next part: a break after focus (the long one every 4th round),
    /// focus after a break. Run out, it stops and the pop-up waits for its button; skipped, the
    /// next part goes on as the skipped one did (running or not), with no pop-up.
    fn next_part(&mut self, skipped: bool) {
        let was_running = self.running;
        self.running = false;
        let (title, body) = if self.mode == Mode::Focus {
            self.rounds += 1;
            self.rounds_day = self.today();
            // Texts and order from 0x14000143f: rounds 1-3 "Focus done", the 4th "Round complete".
            if self.rounds.is_multiple_of(LONG_BREAK_EVERY) {
                self.mode = Mode::Long;
                (t("Round complete"), tr("Time for a {} min break", &[&self.s.long_break]))
            } else {
                self.mode = Mode::Short;
                (t("Focus done"), tr("Time for a {} min break", &[&self.s.short_break]))
            }
        } else {
            self.mode = Mode::Focus;
            (t("Break over"), tr("Back to focus, {} min", &[&self.s.focus]))
        };
        self.left = self.mode_len(self.mode);
        if skipped {
            self.running = was_running;
            self.last_tick = Instant::now();
        } else if self.s.auto_next {
            // Goes on by itself: the pop-up only says so, and closes on its own.
            self.running = true;
            self.last_tick = Instant::now();
            self.notify(title, &body, Kind::About);
        } else {
            self.notify(title, &body, Kind::Info);
        }
    }

    /// Everything that runs on the clock: timer, reminders, clipboard, battery.
    fn tick(&mut self) {
        self.now = Local::now().naive_local();
        let dt = self.last_tick.elapsed().as_secs_f64();
        self.last_tick = Instant::now();
        let today = self.today();
        let m = self.now_min();
        let now_f = m as f32 + self.now.second() as f32 / 60.0; // minutes, with the seconds
        self.tick_pomodoro(today, dt);
        self.tick_water(today, now_f);
        self.tick_tasks(today, m);
        self.tick_calendar(today, m);
        self.tick_prayer(today, now_f);
        self.tick_media_resume();
        self.tick_events();
        self.tick_battery_news();
        self.watch_indicators();
        self.close_finished_popup();
    }

    fn tick_pomodoro(&mut self, today: NaiveDate, dt: f64) {
        // A new day starts the 4 rounds over, unless a part is under way.
        if self.rounds_day != today && !self.running && self.left >= self.mode_len(self.mode) {
            self.rounds_day = today;
            self.rounds = 0;
            self.mode = Mode::Focus;
            self.left = self.mode_len(Mode::Focus);
        }
        // While it runs the notch looks in at least every second, so a longer gap means the PC
        // slept (or the app was stopped). That time isn't focus: pause, count none of it, and say so.
        if self.running && dt > SLEEP_GAP_SECS {
            self.running = false;
            self.notify(t("Pomodoro paused"), t("The PC was asleep — press Resume to go on"), Kind::Hint);
        }
        if self.running {
            self.left -= dt;
            if self.mode == Mode::Focus {
                self.focus_acc += dt;
                if self.focus_acc >= 60.0 {
                    self.focus_acc -= 60.0;
                    *self.focus_log.entry(today).or_insert(0) += 1;
                    self.save_focus();
                }
            }
            if self.left <= 0.0 {
                self.next_part(false);
            }
        }
    }

    fn tick_water(&mut self, today: NaiveDate, now: f32) {
        if self.water_day != today {
            // The day that ended goes into the history before today starts from 0.
            self.water_log.insert(self.water_day, self.drank);
            self.water_day = today;
            self.drank = 0;
            self.fired = 0;
            self.hours = [0; 24];
            self.last_drink = None;
            self.save_water();
        }
        // Water reminders as the original schedules them (0x140001fe5): slot i is due at
        // from + (i + 0.5) * interval and shows if seen within 2 minutes; a snooze within 3.
        if !self.s.water || self.alert.is_some() || self.drank >= self.s.goal {
            return;
        }
        let p = self.s.water_plan();
        let mut due = false;
        if let Some(at) = self.water_snooze.filter(|&at| now >= at as f32) {
            self.water_snooze = None;
            due = now < at as f32 + 3.0;
        }
        // A slot missed while something else was up, the PC slept or a game filled the screen
        // still shows, as long as the next one isn't due yet (so at most one catch-up).
        let window = p.interval.max(2.0);
        for i in 0..p.reminders.min(64) {
            let t = self.s.from as f32 + (i as f32 + 0.5) * p.interval;
            if self.fired & (1 << i) == 0 && now >= t {
                self.fired |= 1 << i;
                due |= now < t + window;
            }
        }
        if due {
            self.save_water();
            if (self.in_bottle as f32) < REFILL_FRACTION * p.each as f32 {
                self.notify(t("Time to Refill"), t("Your bottle is probably empty"), Kind::Refill);
            } else {
                self.notify(t("Time to Drink"), &format!("{} ml", p.each), Kind::Drink);
            }
        }
    }

    fn tick_tasks(&mut self, today: NaiveDate, m: u32) {
        // Tasks come back every day: yesterday's reminders and "Later" don't carry over.
        for task in self.tasks.iter_mut().filter(|t| t.fired_day != Some(today)) {
            (task.fired_start, task.fired_end, task.snooze_until, task.fired_day) = (false, false, 0, Some(today));
        }
        if self.alert.is_some() {
            return;
        }
        let hit = self.tasks.iter().enumerate().find_map(|(i, t)| {
            let st = t.start? as u32;
            if t.done_day == Some(today) || m < t.snooze_until {
                return None;
            }
            if self.s.todo_start && !t.fired_start && m >= st && m < st + 60 {
                return Some((i, true));
            }
            (self.s.todo_end && t.dur > 0 && !t.fired_end && m >= st + t.dur as u32).then_some((i, false))
        });
        if let Some((i, start)) = hit {
            let title = self.tasks[i].title.clone();
            if start {
                self.tasks[i].fired_start = true;
                self.notify(t("Task Time!"), &title, Kind::TaskStart(i));
            } else {
                self.tasks[i].fired_end = true;
                self.notify(t("Time's Up!"), &title, Kind::TaskEnd(i));
            }
            // Kept, so opening the app again doesn't remind of the same thing twice.
            data::save_tasks(&self.tasks);
        }
    }

    /// Calendar events remind as tasks do: at the start, within the first hour, once; and
    /// (cal_early) 10 minutes before, as meetings want.
    fn tick_calendar(&mut self, today: NaiveDate, m: u32) {
        if self.alert.is_some() || !self.s.todo_start {
            return;
        }
        let events = self.calendar.lock().unwrap().events.clone();
        let due = events.into_iter().find_map(|e| {
            let st = e.start?;
            let early = self.s.cal_early && st >= CAL_EARLY_MIN && m >= st - CAL_EARLY_MIN && m < st;
            let now = m >= st && m < st + 60;
            let key = (today, st, e.title.clone(), early);
            ((early || now) && !self.cal_fired.contains(&key)).then_some((e, key))
        });
        if let Some((e, key)) = due {
            let early = key.3;
            self.cal_fired.insert(key);
            let title = if e.title.is_empty() { t("(No title)").to_string() } else { e.title };
            let head = if early { tr("Calendar · in {} min", &[&CAL_EARLY_MIN]) } else { t("Calendar").to_string() };
            self.notify(&head, &title, Kind::Event(e.start.unwrap_or(0)));
        }
    }

    fn tick_prayer(&mut self, today: NaiveDate, now_f: f32) {
        // "My location" not found yet would time prayers for 0°, 0°: no reminders until it is.
        if !(self.s.prayer && self.prayer_placed() && (self.s.prayer_reminder || self.s.prayer_pause_media) && self.alert.is_none()) {
            return;
        }
        for (i, &pt) in self.prayer_times(today).iter().enumerate() {
            for early in [false, true] {
                if early && !(self.s.prayer_early && self.s.prayer_reminder) {
                    continue;
                }
                let at = if early { pt.saturating_sub(PRAYER_EARLY_MIN) } else { pt };
                // Due from 3 s before (0x140002345: window -0.05 .. +2.0 minutes). One missed while
                // something else was up, the PC slept or a game filled the screen still shows later:
                // the early one until prayer time, the prayer itself for PRAYER_CATCHUP_MIN.
                let until = if early { pt as f32 } else { pt as f32 + PRAYER_CATCHUP_MIN };
                if now_f >= at as f32 + PRAYER_WINDOW.0 && now_f < until && self.prayer_fired.insert((today, i, early)) {
                    let body = if early { tr("In {} minutes", &[&PRAYER_EARLY_MIN]) } else { i18n::t("It's time to pray").to_string() };
                    if self.s.prayer_reminder {
                        self.notify(i18n::t(prayer::NAMES[i]), &body, Kind::Prayer(pt, early));
                    }
                    // "Pause media at prayer time": only at the time itself (not on a late catch-up),
                    // and it does not resume.
                    if !early && self.s.prayer_pause_media && now_f < at as f32 + PRAYER_WINDOW.1 {
                        // Remembered only when something was playing, for "Play again after".
                        if self.media.now.lock().is_ok_and(|n| n.playing) {
                            self.prayer_paused = Some(Instant::now());
                        }
                        self.media.send(media::Cmd::Pause);
                    }
                    return;
                }
            }
        }
    }

    /// Music paused at prayer time plays again later, if asked; not if someone played it meanwhile.
    fn tick_media_resume(&mut self) {
        if let Some(at) = self.prayer_paused {
            // A few seconds' grace: the player takes a moment to report the pause itself.
            let playing = at.elapsed().as_secs() > 5 && self.media.now.lock().is_ok_and(|n| n.playing);
            if playing || !self.s.prayer_resume_media {
                self.prayer_paused = None;
            } else if at.elapsed().as_secs() >= PRAYER_RESUME_MIN * 60 {
                self.prayer_paused = None;
                self.media.send(media::Cmd::Play);
            }
        }
    }

    /// Reads again only what Windows says changed (screenshots get no events and need none).
    fn tick_events(&mut self) {
        let ev = if self.shot.is_none() { tray::take_events() } else { 0 };
        // While hidden it looks each time (see idle_wait), so it comes back even if an event is missed.
        if ev & tray::EV_WINDOW != 0 || self.hidden || !self.s.hide_fullscreen {
            self.hidden = self.shot.is_none() && self.s.hide_fullscreen && fullscreen_in_front();
        }
        if ev & tray::EV_CLIP != 0 && self.s.clipboard {
            if let Some(t) = self.clipboard.as_mut().and_then(|c| c.get_text().ok()) {
                if t != self.last_clip {
                    self.last_clip = t.clone();
                    // Passwords and other text marked "don't keep" never go into the history.
                    if !t.trim().is_empty() && !tray::clipboard_private() {
                        self.clips.retain(|c| c.1 != t);
                        self.clips.insert(0, (Utc::now().timestamp(), t));
                        self.clips.truncate(self.s.clipboard_max);
                        self.save_clips();
                    }
                }
            }
        }
        if ev & tray::EV_POWER != 0 {
            let old = self.battery;
            self.battery = battery();
            // The original's rules (0x140056c76): a charger change first; otherwise a warning only when
            // the level crosses the chosen low level (the original's 21 %) / 11 % going down on battery, or reaches 100 % while charging.
            if let (true, Some((p, ac)), Some((op, oac))) = (self.s.popup_battery, self.battery, old) {
                let say = if ac != oac {
                    Some(if ac { (t("Charging").to_string(), t("Charger connected").to_string()) } else { (t("Charger unplugged").to_string(), format!("{p}%")) })
                } else if !ac && p < BATTERY_CRITICAL && op >= BATTERY_CRITICAL {
                    Some((t("Battery critical").to_string(), t("Plug in now").to_string()))
                } else if !ac && p <= self.s.battery_low && op > self.s.battery_low {
                    Some((t("Low battery").to_string(), t("Plug in soon").to_string()))
                } else if ac && p >= 100 && op < 100 {
                    Some((t("Fully charged").to_string(), t("You can unplug").to_string()))
                } else {
                    None
                };
                if let Some((title, body)) = say {
                    self.battery_news = Some(Alert { title, body, kind: Kind::Battery(p, ac), at: Instant::now() });
                }
            }
        }
    }

    /// Battery news isn't lost behind another pop-up: it takes the place of one that needs no
    /// answer (a volume bar, a notification…), or waits for one that does (Drank / Snooze…).
    fn tick_battery_news(&mut self) {
        let passing = self.alert.as_ref().is_none_or(|a| matches!(a.kind, Kind::Hud(..) | Kind::Notice(..) | Kind::Network(_) | Kind::Bluetooth(_) | Kind::Battery(..) | Kind::About | Kind::Hint | Kind::Song));
        if passing {
            if let Some(mut news) = self.battery_news.take() {
                news.at = Instant::now();
                if self.s.sound {
                    beep();
                }
                self.alert = Some(news);
            }
        }
    }

    /// Information pop-ups close by themselves; the ones with buttons wait for an answer.
    fn close_finished_popup(&mut self) {
        let over = self.alert.as_ref().is_some_and(|a| {
            let age = a.at.elapsed().as_secs_f32();
            match a.kind {
                Kind::Network(_) => age > NET_POPUP_SECS,
                Kind::Hud(..) => age > HUD_SECS,
                Kind::Battery(..) => age > BATTERY_POPUP_SECS,
                Kind::Bluetooth(_) => age > BT_POPUP_SECS,
                // Longer messages stay longer, and none goes while the pointer rests on it.
                Kind::Notice(..) => age > notice_secs(&a.title, &a.body) && self.last_inside.elapsed().as_secs_f32() > NOTICE_AFTER_HOVER_SECS,
                Kind::About => age > ABOUT_SECS,
                Kind::Hint => age > HINT_SECS,
                Kind::Song => age > SONG_POPUP_SECS,
                Kind::Prayer(_, early) => age > if early { PRAYER_EARLY_SECS } else { PRAYER_POPUP_SECS },
                Kind::Event(_) => age > PRAYER_POPUP_SECS,
                _ => false,
            }
        });
        if over {
            self.alert = None;
        }
    }

    fn target_size(&self) -> Vec2 {
        if self.expanded && self.page == 0 && self.lyrics_open && !self.settings_open {
            LYRICS
        } else if self.expanded && self.settings_open {
            SETTINGS
        } else if self.expanded {
            EXPANDED
        } else if self.alert.as_ref().is_some_and(|a| matches!(a.kind, Kind::Hud(..))) {
            HUD
        } else if let Some(a) = self.alert.as_ref().filter(|a| matches!(a.kind, Kind::Notice(..))) {
            if a.body.is_empty() { NOTICE } else { NOTICE_TALL }
        } else if self.alert.is_some() {
            POPUP
        } else if self.s.minimal && self.idle() {
            PEEK
        } else if self.s.minimal && !self.running {
            MINIMAL
        } else {
            COLLAPSED
        }
    }

    /// Moves the springs and the window along with them.
    fn animate(&mut self, ctx: &egui::Context) -> bool {
        let dt = ctx.input(|i| i.stable_dt).min(1.0 / 30.0);
        let target = self.target_size();
        if target != self.last_target {
            self.spring = if self.last_target == POPUP && !self.expanded { SPRING_FROM_POPUP } else { SPRING };
            self.last_target = target;
        }
        let mut moving = self.w.step(target.x, dt, self.spring);
        moving |= self.h.step(target.y, dt, self.spring);
        // Content fades in once the shape has mostly arrived, and out at once when it leaves.
        let near = (self.w.x - target.x).abs() < 40.0 && (self.h.x - target.y).abs() < 20.0;
        moving |= self.content.step(if near { 1.0 } else { 0.0 }, dt * if near { 1.0 } else { 3.0 }, SPRING_TO_PEEK);
        self.content.x = self.content.x.clamp(0.0, 1.0);
        let page_target = self.pos() as f32;
        if (self.page_anim - page_target).abs() > 0.001 {
            self.page_anim += (page_target - self.page_anim) * (1.0 - (-dt * 14.0).exp());
            moving = true;
        } else {
            self.page_anim = page_target;
        }

        let zoom = SIZE_SCALES[self.s.size];
        if ctx.zoom_factor() != zoom {
            ctx.set_zoom_factor(zoom);
        }
        if self.shot.is_none() {
            let size = vec2(self.w.x.max(target.x) + 2.0 * EAR_MAX, self.h.x.max(target.y));
            let mon = ctx.input(|i| i.viewport().monitor_size).unwrap_or(vec2(1920.0, 1080.0));
            // Both are in points, which already include the zoom; dividing by it again pushed bigger sizes left.
            let x = ((mon.x - size.x) / 2.0).max(0.0);
            let want = [size.x.ceil() as i32, size.y.ceil() as i32, x.round() as i32];
            if self.applied != Some(want) {
                self.applied = Some(want);
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(want[0] as f32, want[1] as f32)));
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos2(x, 0.0)));
            }
        }
        moving
    }

    // ---------- shared pieces ----------

    fn header(&mut self, ui: &mut egui::Ui, r: Rect, title: (&str, &str, Color32)) {
        let p = ui.painter().clone();
        let y = r.top() + 24.0;
        let g = p.layout_no_wrap(title.0.into(), bold(17.0), WHITE);
        let w0 = g.size().x;
        p.galley(pos2(r.left() + 22.0, y - g.size().y / 2.0), g, WHITE);
        if !title.1.is_empty() {
            p.text(pos2(r.left() + 22.0 + w0, y), Align2::LEFT_CENTER, title.1, bold(17.0), title.2);
        }
        let date = i18n::date(self.now);
        let time = self.now.format("%H:%M").to_string();
        let dg = p.layout_no_wrap(date, reg(16.0), WHITE);
        let tg = p.layout_no_wrap(time, bold(16.0), WHITE);
        let total = 22.0 + 10.0 + dg.size().x + 8.0 + tg.size().x;
        let x0 = r.center().x - total / 2.0 + 30.0;
        self.status_ring(&p, pos2(x0 + 11.0, y), 9.0);
        let dx = x0 + 32.0;
        let dw = dg.size().x;
        if self.s.show_clock {
            p.galley(pos2(dx, y - dg.size().y / 2.0), dg, WHITE);
            p.galley(pos2(dx + dw + 8.0, y - tg.size().y / 2.0), tg, WHITE);
        }
        if self.page != SHELF_PAGE || self.settings_open {
            bars(&p, pos2(r.right() - 34.0, y), self.running, self.music_tint());
        }
    }

    fn status_ring(&self, p: &egui::Painter, c: Pos2, rad: f32) {
        let frac = match self.s.status_ring {
            0 => self.drank as f32 / self.s.goal.max(1) as f32,
            1 => self.done_today() as f32 / self.tasks.len().max(1) as f32,
            _ => return,
        };
        ring(p, c, rad, 3.0, frac, GREEN);
    }

    fn dots(&mut self, ui: &mut egui::Ui, r: Rect) {
        let y = r.bottom() - 12.0;
        let order = self.order();
        let n = order.len();
        let mut x = r.center().x - (n as f32 * 9.0 + 14.0) / 2.0;
        for i in 0..n {
            // The current page's dot stretches into a pill, following the page motion.
            let wgt = (1.0 - (self.page_anim - i as f32).abs()).clamp(0.0, 1.0);
            let w = 5.0 + 15.0 * wgt;
            let rect = Rect::from_min_size(pos2(x, y - 2.5), vec2(w, 5.0));
            let col = lerp_color(FAINT, WHITE, wgt);
            ui.painter().rect_filled(rect, 2.5, col);
            if ui.interact(rect.expand(3.0), ui.id().with(("dot", i)), Sense::click()).clicked() {
                self.page = order[i];
            }
            x += w + 4.0;
        }
    }

    fn arrows(&mut self, ui: &mut egui::Ui, r: Rect) {
        let y = r.top() + 98.0;
        let (order, at) = (self.order(), self.pos());
        if at > 0 && circle_button(ui, pos2(r.left() + 26.0, y), 13.0, ARROW, "left") {
            self.page = order[at - 1];
        }
        if at + 1 < order.len() && circle_button(ui, pos2(r.right() - 26.0, y), 13.0, ARROW, "right") {
            self.page = order[at + 1];
        }
    }

    /// The mouse wheel or a touchpad swipe turns pages (down or left = next), and so do the ← →
    /// keys while nothing is being typed. Over a list that scrolls itself, only sideways scroll turns
    /// (none at all over one that scrolls sideways).
    fn turn_by_scroll_and_keys(&mut self, ui: &mut egui::Ui) {
        let (order, at) = (self.order(), self.pos());
        let typing = ui.ctx().memory(|m| m.focused().is_some());
        let (left, right, d, over) = ui.input(|i| {
            let keys = !typing && i.modifiers.is_none();
            let over = i.pointer.hover_pos().zip(self.scroll_zone).filter(|(p, z)| z.0.contains(*p)).map(|(_, z)| z.1);
            (keys && i.key_pressed(egui::Key::ArrowLeft), keys && i.key_pressed(egui::Key::ArrowRight), i.raw_scroll_delta, over)
        });
        let step = match over {
            Some(true) => d.x,
            Some(false) => 0.0,
            None => d.x + d.y,
        };
        if step != 0.0 {
            // A pause ends the flick; what it gathered is dropped.
            if self.wheel_last.elapsed().as_secs_f32() > WHEEL_REST_SECS {
                self.wheel_acc = 0.0;
            }
            self.wheel_last = Instant::now();
            if self.wheel_rest > Instant::now() {
                self.wheel_rest = Instant::now() + std::time::Duration::from_secs_f32(WHEEL_REST_SECS);
            } else {
                self.wheel_acc -= step;
            }
        }
        let mut go = right as i32 - left as i32;
        if self.wheel_acc.abs() >= WHEEL_STEP {
            go = self.wheel_acc.signum() as i32;
            self.wheel_acc = 0.0;
            self.wheel_rest = Instant::now() + std::time::Duration::from_secs_f32(WHEEL_REST_SECS);
        }
        let to = at as i32 + go;
        if go != 0 && (0..order.len() as i32).contains(&to) {
            self.page = order[to as usize];
        }
    }

    /// The level bars' colour: the playing song's cover colour, else white.
    fn music_tint(&self) -> Color32 {
        let now = self.media.now.lock();
        match now.as_ref().ok().filter(|n| n.playing).and_then(|n| n.tint) {
            Some([r, g, b]) => Color32::from_rgb(r, g, b),
            None => WHITE,
        }
    }

    /// The playing song's cover as a texture, made once per song.
    fn art_texture(&mut self, ctx: &egui::Context, now: &media::Now) -> Option<egui::TextureHandle> {
        let art = now.art.as_ref()?;
        if let Some((key, tex)) = &self.art_tex {
            if *key == now.art_key {
                return Some(tex.clone());
            }
        }
        let img = egui::ColorImage::from_rgba_unmultiplied([art.0, art.1], &art.2);
        let tex = ctx.load_texture("cover", img, egui::TextureOptions::LINEAR);
        self.art_tex = Some((now.art_key.clone(), tex.clone()));
        Some(tex)
    }

    /// The notifying app's logo as a texture, made once per app from what the watcher read.
    fn logo_texture(&mut self, ctx: &egui::Context, id: &str) -> Option<egui::TextureHandle> {
        if let Some(t) = self.logos.get(id) {
            return Some(t.clone());
        }
        let logo = media::logo(id)?;
        let img = egui::ColorImage::from_rgba_unmultiplied([logo.0, logo.1], &logo.2);
        let tex = ctx.load_texture(format!("logo-{id}"), img, egui::TextureOptions::LINEAR);
        self.logos.insert(id.to_string(), tex.clone());
        Some(tex)
    }
}

impl eframe::App for App {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let pick = tray::take();
        if pick == tray::OPEN {
            self.expanded = true;
            self.settings_open = false;
            self.held = true;
        }
        // The tray menu's switches change the same settings as Settings does.
        if matches!(pick, tray::STARTUP | tray::MASCOT | tray::MINIMAL) {
            match pick {
                tray::STARTUP => {
                    if set_startup(!self.s.startup) {
                        self.s.startup = !self.s.startup;
                    } else {
                        self.alert = Some(Alert { title: t("Couldn't change Start with Windows").into(), body: t("Windows didn't allow it — try again").into(), kind: Kind::Hint, at: Instant::now() });
                    }
                }
                tray::MASCOT => self.s.mascot = !self.s.mascot,
                _ => self.s.minimal = !self.s.minimal,
            }
            self.s.save();
        }
        if pick == tray::OCR {
            self.expanded = false;
            ocr::start(self.ocr.clone());
        }
        // The weather is fetched only while a Weather widget is chosen, for its place.
        let shown = self.s.widget_left == WEATHER_WIDGET || self.s.widget_right == WEATHER_WIDGET;
        let here = (self.s.prayer_lat, self.s.prayer_lon);
        let unfixed = self.s.weather_city == prayer::here_index() && here == (0.0, 0.0);
        *self.weather_place.lock().unwrap() = (shown && !unfixed).then(|| prayer::place_coords(self.s.weather_city, here));
        // A location found for "My location" is kept even if Settings was closed meanwhile.
        let found = *self.locating.lock().unwrap();
        if let Locating::Got(lat, lon) = found {
            (self.s.prayer_lat, self.s.prayer_lon) = (lat, lon);
            *self.locating.lock().unwrap() = Locating::Idle;
            self.s.save();
        }
        let read = self.ocr.lock().unwrap().take();
        match read {
            Some(ocr::Outcome::Copied(n)) => self.alert = Some(Alert { title: t("Text copied").into(), body: tr("{} characters, ready to paste", &[&n]), kind: Kind::About, at: Instant::now() }),
            Some(ocr::Outcome::NoText) => self.alert = Some(Alert { title: t("No text found").into(), body: t("Try a larger area").into(), kind: Kind::About, at: Instant::now() }),
            Some(ocr::Outcome::ClipboardBusy) => self.alert = Some(Alert { title: t("Couldn't copy").into(), body: t("Another app is using the clipboard — try again").into(), kind: Kind::About, at: Instant::now() }),
            Some(ocr::Outcome::NoEngine) => self.alert = Some(Alert { title: t("No text found").into(), body: t("Add an OCR language in Windows Settings › Language").into(), kind: Kind::About, at: Instant::now() }),
            _ => {}
        }
        // A shortcut another app owns is said once, after any pop-up already up (e.g. the welcome).
        if !self.hotkeys_told && self.alert.is_none() {
            if let Some(bits) = tray::hotkeys_taken() {
                self.hotkeys_told = true;
                if bits != 0 {
                    self.alert = Some(taken_hint(bits));
                }
            }
        }
        if pick == tray::ABOUT {
            self.alert = Some(Alert { title: "My Notch".into(), body: tr("Version {}", &[&env!("CARGO_PKG_VERSION")]), kind: Kind::About, at: Instant::now() });
        }
        tray::set_checks(self.s.startup, self.s.mascot, self.s.minimal);
        if pick == tray::ASK {
            self.expanded = true;
            self.held = true;
            self.settings_open = false;
            self.ask_open = true;
            self.ask_focus = true;
            // The hotkey lets this app take the keyboard, so typing goes straight into the box.
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        if pick == tray::NOTE {
            self.expanded = true;
            self.held = true;
            self.settings_open = false;
            self.ask_open = false;
            // Asking for the page by its shortcut brings it back if it was hidden.
            if self.s.pages_hidden.contains(&NOTE_PAGE) {
                self.s.pages_hidden.retain(|&p| p != NOTE_PAGE);
                self.s.save();
            }
            self.page = NOTE_PAGE;
            self.note_focus = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        // Turning the Today page off while on it moves back to the last page left.
        // A page turned off or hidden while on it hands over to the first one shown.
        let order = self.order();
        if !order.contains(&self.page) {
            self.page = order[0];
        }
        let was_hidden = self.hidden;
        self.tick();
        if self.hidden != was_hidden {
            // Not Visible(false): eframe stops calling update for a hidden window, so the notch could
            // never come back, and its event loop spins a core. Drawing nothing and letting clicks
            // through looks the same and keeps the once-a-second look in idle_wait going.
            ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(self.hidden));
        }
        if self.hidden {
            ctx.request_repaint_after(self.idle_wait());
            return;
        }

        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if !dropped.is_empty() {
            for p in dropped {
                if !self.shelf.contains(&p) {
                    self.shelf.push(p);
                }
            }
            self.save_shelf();
            // Dropped files are kept either way; the Shelf page comes up only when it is shown.
            if self.order().contains(&SHELF_PAGE) {
                self.page = SHELF_PAGE;
            }
            self.shelf_view = ShelfView::Files;
        }

        let files_over = ctx.input(|i| !i.raw.hovered_files.is_empty());
        let inside = files_over || ctx.input(|i| i.pointer.hover_pos().is_some());
        if files_over && self.order().contains(&SHELF_PAGE) {
            self.page = SHELF_PAGE;
            self.settings_open = false;
        }
        let typing = ctx.memory(|m| m.focused().is_some());
        if (self.shot.is_some() && !self.resting) || files_over {
            self.expanded = true;
        } else if inside {
            self.last_inside = Instant::now();
            self.held = false;
            if !self.s.open_click && self.alert.is_none() {
                self.expanded = true;
            }
        } else if self.expanded && !typing && !self.held && self.last_inside.elapsed().as_secs_f32() > 0.5 {
            self.expanded = false;
            self.settings_open = false;
            self.ask_open = false;
        }
        let moving = self.animate(ctx);

        egui::CentralPanel::default().frame(egui::Frame::default()).show(ctx, |ui| {
            let full = ui.max_rect();
            let size = vec2(self.w.x, self.h.x);
            let body = Rect::from_min_size(pos2(full.center().x - size.x / 2.0, full.top()), size);
            // Corner radius follows the height, so the pill and the panel share one shape.
            let rad = (size.y * 0.2).clamp(12.0, 32.0);
            let ear = (rad * 0.55).min(EAR_MAX);
            notch_shape(ui.painter(), body, rad, ear);
            let resp = ui.interact(body, ui.id().with("notch"), Sense::click());
            if resp.secondary_clicked() {
                self.expanded = true;
                self.settings_open = !self.settings_open;
            }
            let notice_app = self.alert.as_ref().and_then(|a| match &a.kind {
                Kind::Notice(_, id, _) if can_open_app(id) => Some(id.clone()),
                _ => None,
            });
            if !self.expanded && resp.clicked() && notice_app.is_some() {
                // A notification opens the app that sent it, as a click on the toast would.
                let opened = notice_app.is_some_and(|id| open_app(&id));
                if opened {
                    self.alert = None;
                } else {
                    self.expanded = true;
                }
            } else if !self.expanded && resp.clicked() {
                if self.alert.as_ref().is_some_and(|a| matches!(a.kind, Kind::Info | Kind::Battery(..) | Kind::Prayer(..) | Kind::Event(_) | Kind::Hint)) {
                    self.alert = None;
                }
                self.expanded = true;
            }

            let alpha = self.content.x;
            if alpha < 0.02 {
                return;
            }
            let target = self.target_size();
            let r = Rect::from_min_size(pos2(full.center().x - target.x / 2.0, full.top()), target);
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(r), |ui| {
                ui.multiply_opacity(alpha);
                ui.set_clip_rect(body);
                if self.expanded {
                    if self.ask_open && !self.settings_open {
                        self.page_ask(ui, r);
                        return;
                    }
                    if self.settings_open {
                        ui.painter().text(pos2(r.left() + 22.0, r.top() + 24.0), Align2::LEFT_CENTER, t("Settings"), bold(17.0), WHITE);
                        // Close, or Esc as in the Ask box.
                        let esc = ui.input(|i| i.key_pressed(egui::Key::Escape));
                        if pill_button(ui, Rect::from_min_size(pos2(r.right() - 76.0, r.top() + 12.0), vec2(56.0, 24.0)), t("Close"), BTN, "sclose") || esc {
                            self.settings_open = false;
                        }
                        self.page_settings(ui, r);
                        return;
                    }
                    // Pages slide sideways and fade as the dots move (page motion).
                    // page_anim moves through positions in the user's order; each position shows its page.
                    let order = self.order();
                    let last = order.len() - 1;
                    let shown = [(self.page_anim.floor() as usize).min(last), (self.page_anim.ceil() as usize).min(last)];
                    let title = self.page_title(self.page);
                    let lyrics_view = self.page == 0 && self.lyrics_open;
                    if !lyrics_view {
                        self.header(ui, r, title);
                    }
                    self.scroll_zone = None;
                    for (k, i) in shown.into_iter().enumerate() {
                        if k == 1 && shown[0] == shown[1] {
                            break;
                        }
                        let off = i as f32 - self.page_anim;
                        let vis = (1.0 - off.abs()).clamp(0.0, 1.0);
                        if vis < 0.05 {
                            continue;
                        }
                        let pr = r.translate(vec2(off * 120.0, 0.0));
                        let page = order[i];
                        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(r).id_salt(("page", page)), |ui| {
                            ui.multiply_opacity(vis);
                            if (off).abs() > 0.5 {
                                ui.disable();
                            }
                            self.draw_page(ui, page, pr);
                        });
                    }
                    if !lyrics_view {
                        self.arrows(ui, r);
                        self.dots(ui, r);
                        self.turn_by_scroll_and_keys(ui);
                    }
                } else if self.alert.is_some() {
                    self.draw_popup(ui, r);
                } else {
                    self.draw_collapsed(ui, r);
                }
            });
        });

        if let Some(path) = self.shot.clone() {
            self.frames += 1;
            if self.frames == 120 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
            let img = ctx.input(|i| {
                i.raw.events.iter().find_map(|e| match e {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            });
            if let Some(img) = img {
                let bytes: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
                if let Some(dir) = path.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                let _ = image::save_buffer(&path, &bytes, img.size[0] as u32, img.size[1] as u32, image::ColorType::Rgba8);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ctx.request_repaint();
        } else if moving || self.running || self.alert.is_some() {
            ctx.request_repaint();
        } else {
            // Still: nothing is drawn until Windows reports a change (see tray.rs) or the next
            // thing the clock moves: the minute, the mascot's blink, a playing song's progress.
            ctx.request_repaint_after(self.idle_wait());
        }
    }
}

impl App {
    fn idle_wait(&self) -> std::time::Duration {
        let ms = Utc::now().timestamp_millis();
        // Just after the minute turns, so the clock and minute-based reminders are on time.
        let mut wait = 60_000 - ms.rem_euclid(60_000) + 20;
        let mascot_shown = !self.expanded && self.alert.is_none() && self.s.mascot && self.left >= self.mode_len(self.mode);
        if mascot_shown {
            let b = ms.rem_euclid(BLINK_EVERY_MS);
            // During a blink, draw every frame so the lids move; otherwise wake for the next one.
            wait = wait.min(if b < BLINK_MS { 16 } else { BLINK_EVERY_MS - b });
        }
        let song_shown = self.expanded && self.page == 0 && !self.settings_open;
        if song_shown && self.media.now.lock().is_ok_and(|n| n.playing) {
            wait = wait.min(250);
        }
        // Open, someone is using it: short-lived states ("Copied", closing 0.5 s after the pointer
        // leaves, an armed reset) need a look every half second. Closed and still, none of them apply.
        if self.expanded {
            wait = wait.min(500);
        }
        if self.hidden {
            wait = wait.min(1000);
        }
        std::time::Duration::from_millis(wait.max(1) as u64)
    }
}

// ---------- text, time and date helpers ----------

/// A notification icon's size as it arrives: from half size to a little over, then settling at 1
/// over NOTICE_POP_SECS (an ease-out with a small overshoot).
fn pop_in(since: f32) -> f32 {
    let u = (since / NOTICE_POP_SECS).clamp(0.0, 1.0) - 1.0;
    let back = 1.0 + 2.70158 * u * u * u + 1.70158 * u * u;
    0.5 + 0.5 * back
}

/// How long a notification shows: 4.5 s, plus a second for every 30 characters past the first
/// 40 (about reading speed), up to 8 s.
fn notice_secs(title: &str, body: &str) -> f32 {
    let chars = (title.chars().count() + body.chars().count()) as f32;
    (NOTICE_POPUP_SECS + (chars - 40.0).max(0.0) / 30.0).min(NOTICE_MAX_SECS)
}

/// Millilitres as litres: one decimal, or two when needed (1.5, 2.0, 1.75).
fn litres(ml: u32) -> String {
    if ml.is_multiple_of(100) { format!("{:.1}", ml as f32 / 1000.0) } else { format!("{:.2}", ml as f32 / 1000.0) }
}

/// `text` on one line no wider than `width`, ending in "…" when it is cut (cut by real width,
/// so Vietnamese and Chinese text fill the line too).
fn fit_line(p: &egui::Painter, text: &str, font: FontId, col: Color32, width: f32) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(text.replace('\n', " "), font, col);
    job.wrap = egui::text::TextWrapping { max_width: width.max(10.0), max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
    p.layout_job(job)
}

/// The default settings, keeping what isn't a preference to restore: the language, the water plan
/// (it lives with the day's water), whether Windows starts the app, the location found, the welcome.
fn default_settings_keeping(s: &Settings) -> Settings {
    Settings {
        language: s.language,
        water: s.water,
        goal: s.goal,
        bottle: s.bottle,
        from: s.from,
        until: s.until,
        startup: s.startup,
        prayer_lat: s.prayer_lat,
        prayer_lon: s.prayer_lon,
        welcomed: s.welcomed,
        ..Settings::from_text("", "")
    }
}

/// Asks Windows where this PC is, on its own thread (unless already asking); `update` applies it.
fn find_location(locating: &std::sync::Arc<std::sync::Mutex<Locating>>) {
    let mut state = locating.lock().unwrap();
    if *state == Locating::Busy {
        return;
    }
    *state = Locating::Busy;
    let out = locating.clone();
    std::thread::spawn(move || {
        *out.lock().unwrap() = media::locate().map_or(Locating::Failed, |(a, b)| Locating::Got(a, b));
        tray::wake();
    });
}

/// Moves label centres `xs` (left to right) apart to at least `gap`, as little as it can, staying
/// within `lo..hi` where there is room.
fn spread(xs: &[f32], gap: f32, lo: f32, hi: f32) -> Vec<f32> {
    let mut out = xs.to_vec();
    for i in 1..out.len() {
        out[i] = out[i].max(out[i - 1] + gap);
    }
    if let Some(last) = out.last_mut() {
        *last = last.min(hi);
    }
    for i in (0..out.len().saturating_sub(1)).rev() {
        out[i] = out[i].min(out[i + 1] - gap);
    }
    if let Some(first) = out.first_mut() {
        *first = first.max(lo);
    }
    out
}

/// A row inside `area` that scrolls sideways when its items don't fit; the mouse wheel scrolls it
/// too (not only Shift + wheel).
fn sideways(ui: &mut egui::Ui, area: Rect, id: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(area), |ui| {
        ui.style_mut().always_scroll_the_only_direction = true;
        egui::ScrollArea::horizontal().id_salt(id).auto_shrink(false).show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.horizontal_top(add);
        });
    });
}

/// A steady colour for an app without a logo, picked from its name, so each app keeps its own.
fn app_color(app: &str) -> Color32 {
    const COLORS: [Color32; 8] = [
        Color32::from_rgb(37, 99, 235), Color32::from_rgb(22, 163, 74), Color32::from_rgb(147, 51, 234), Color32::from_rgb(219, 39, 119),
        Color32::from_rgb(234, 88, 12), Color32::from_rgb(13, 148, 136), Color32::from_rgb(202, 138, 4), Color32::from_rgb(79, 70, 229),
    ];
    // FNV-1a: the same name gives the same colour on every run.
    let h = app.bytes().fold(0x811c9dc5u32, |h, b| (h ^ b as u32).wrapping_mul(0x01000193));
    COLORS[h as usize % COLORS.len()]
}

fn clip_text(text: &str, g: &egui::Galley, max_rows: usize, clipped: bool, just: bool) -> String {
    if just {
        return t("Copied").into();
    }
    if !clipped {
        return text.to_string();
    }
    let chars: usize = g.rows.iter().take(max_rows).map(|r| r.char_count_including_newline()).sum();
    let mut s: String = text.chars().take(chars.saturating_sub(3)).collect();
    s.push('…');
    s
}

fn ellipsize(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(n - 1).collect();
        t.push('…');
        t
    }
}

fn human_size(b: u64) -> String {
    match b {
        b if b >= 1 << 30 => format!("{:.1} GB", b as f64 / (1u64 << 30) as f64),
        b if b >= 1 << 20 => format!("{:.1} MB", b as f64 / (1u64 << 20) as f64),
        b if b >= 1 << 10 => format!("{} KB", b >> 10),
        b => format!("{b} B"),
    }
}

fn ago(secs: i64) -> String {
    match secs {
        s if s < 60 => t("Just now").into(),
        s if s < 3600 => tr("{} min ago", &[&(s / 60)]),
        s if s < 86400 => tr("{} h ago", &[&(s / 3600)]),
        s => tr("{} d ago", &[&(s / 86400)]),
    }
}

fn dur_text(min: u32) -> String {
    match (min / 60, min % 60) {
        (0, m) => tr("{} min", &[&m]),
        (h, 0) => format!("{h} h"),
        (h, m) => tr("{} h {} min", &[&h, &m]),
    }
}

fn hm(m: u32) -> String {
    format!("{:02}:{:02}", m / 60, m % 60)
}

fn mmss(secs: f64) -> String {
    let s = secs.max(0.0).ceil() as u32;
    format!("{:02}:{:02}", s / 60, s % 60)
}

fn parse_hm(s: &str) -> Option<u16> {
    let (h, m) = s.trim().split_once(':')?;
    let (h, m): (u16, u16) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

/// Moon phase index (0 = new) and lit fraction, from the mean synodic month.
fn moon(utc: NaiveDateTime) -> (usize, f64) {
    const SYNODIC: f64 = 29.530588853;
    let ref_new = NaiveDate::from_ymd_opt(2000, 1, 6).unwrap().and_hms_opt(18, 14, 0).unwrap();
    let age = ((utc - ref_new).num_seconds() as f64 / 86400.0).rem_euclid(SYNODIC);
    let idx = ((age / SYNODIC * 8.0) + 0.5) as usize % 8;
    (idx, (1.0 - (age / SYNODIC * std::f64::consts::TAU).cos()) / 2.0)
}

fn nth_sunday(y: i32, m: u32, n: u32) -> NaiveDate {
    let first = NaiveDate::from_ymd_opt(y, m, 1).unwrap();
    first + Duration::days(((7 - first.weekday().num_days_from_sunday()) % 7 + 7 * (n - 1)) as i64)
}

fn last_sunday(y: i32, m: u32) -> NaiveDate {
    let next = if m == 12 { NaiveDate::from_ymd_opt(y + 1, 1, 1) } else { NaiveDate::from_ymd_opt(y, m + 1, 1) }.unwrap();
    let last = next - Duration::days(1);
    last - Duration::days(last.weekday().num_days_from_sunday() as i64)
}

/// Local time in a world-clock city, with simple daylight-saving rules.
fn world_time(idx: usize) -> NaiveDateTime {
    let (_, std, rule) = WORLD[idx.min(WORLD.len() - 1)];
    let utc = Utc::now().naive_utc();
    utc + Duration::minutes(((std + if summer(rule, std, utc) { 1.0 } else { 0.0 }) * 60.0) as i64)
}

/// Whether summer time applies at that UTC moment, for a place `std` hours from UTC under `rule`.
/// Shared with prayer times for cities abroad.
fn summer(rule: Dst, std: f64, utc: NaiveDateTime) -> bool {
    let at = |d: NaiveDate, local_hour: f64| d.and_hms_opt(0, 0, 0).unwrap() + Duration::minutes(((local_hour - std) * 60.0) as i64);
    let y = utc.year();
    match rule {
        Dst::No => false,
        Dst::Eu => utc >= last_sunday(y, 3).and_hms_opt(1, 0, 0).unwrap() && utc < last_sunday(y, 10).and_hms_opt(1, 0, 0).unwrap(),
        Dst::Us => utc >= at(nth_sunday(y, 3, 2), 2.0) && utc < at(nth_sunday(y, 11, 1), 1.0),
        Dst::Au => utc >= at(nth_sunday(y, 10, 1), 2.0) || utc < at(nth_sunday(y, 4, 1), 2.0),
    }
}

/// What the Copy button copies: the answer without its "= " (a sum, a conversion), or Gemini's text.
fn answer_text(shown: &str) -> String {
    shown.strip_prefix("= ").unwrap_or(shown).trim().to_string()
}

/// One group per app (newest app first): (app name, app id, newest time, its (title, body), how many).
type AppGroup = (String, String, i64, (String, String), usize);

fn group_by_app(recent: &[(i64, media::Notice)]) -> Vec<AppGroup> {
    let mut out: Vec<AppGroup> = vec![];
    for (ts, (app, title, body, id)) in recent {
        match out.iter_mut().find(|g| g.0 == *app && g.1 == *id) {
            Some(g) => g.4 += 1, // newest first, so the one kept is the newest
            None => out.push((app.clone(), id.clone(), *ts, (title.clone(), body.clone()), 1)),
        }
    }
    out
}

/// Whether `id` (an app's AppUserModelId from a notification) is safe to hand to Explorer.
fn can_open_app(id: &str) -> bool {
    !id.is_empty() && !id.chars().any(|c| c == '"' || c.is_control())
}

/// Opens the app that sent a notification, as a click on its toast would; false if it couldn't.
fn open_app(id: &str) -> bool {
    can_open_app(id) && std::process::Command::new("explorer.exe").arg(format!("shell:AppsFolder\\{id}")).spawn().is_ok()
}

/// A clock's distance from this PC's, rounded to the quarter hour: "+1 h", "−5:30 h", "same time".
fn offset_text(minutes: i64) -> String {
    let q = ((minutes as f64 / 15.0).round() as i64) * 15;
    if q == 0 {
        return t("same time").to_string();
    }
    let sign = if q > 0 { "+" } else { "−" };
    let (h, m) = (q.abs() / 60, q.abs() % 60);
    if m == 0 { tr("{} h", &[&format!("{sign}{h}")]) } else { tr("{} h", &[&format!("{sign}{h}:{m:02}")]) }
}

/// The next day of "My own date": "DD/MM" comes every year, "DD/MM/YYYY" once (None once past,
/// or when it doesn't read as a date).
fn my_date(today: NaiveDate, text: &str) -> Option<NaiveDate> {
    let parts: Vec<u32> = text.trim().split(['/', '-', '.']).map(|p| p.trim().parse().ok()).collect::<Option<_>>()?;
    match parts[..] {
        // This year's if still to come, else the next year that has it (29/02: the next leap year).
        [d, m] => (0..=8).filter_map(|k| NaiveDate::from_ymd_opt(today.year() + k, m, d)).find(|&x| x >= today),
        [d, m, y] => NaiveDate::from_ymd_opt(y as i32, m, d).filter(|&x| x >= today),
        _ => None,
    }
}

fn next_event(today: NaiveDate, ev: usize) -> Option<NaiveDate> {
    let fixed = |m, d| {
        let this = NaiveDate::from_ymd_opt(today.year(), m, d)?;
        Some(if this >= today { this } else { NaiveDate::from_ymd_opt(today.year() + 1, m, d)? })
    };
    let table = |t: &[(i32, u32, u32)]| t.iter().filter_map(|&(y, m, d)| NaiveDate::from_ymd_opt(y, m, d)).find(|d| *d >= today);
    // Past the announced dates, the worked-out Hijri calendar (which can be a day out).
    let hijri = |month: u32, day: u32| (0..400).map(|k| today + Duration::days(k)).find(|&d| prayer::hijri(d).1 == month && prayer::hijri(d).2 == day);
    match ev {
        0 => fixed(1, 1),
        1 => table(&RAMADAN).or_else(|| hijri(9, 1)),
        2 => table(&EID_FITR).or_else(|| hijri(10, 1)),
        3 => table(&EID_ADHA).or_else(|| hijri(12, 10)),
        4 => fixed(8, 17),
        6 => table(&LUNAR_NEW_YEAR),
        7 => fixed(9, 2),
        _ => fixed(12, 25),
    }
}

/// The first start's hint: how to open the notch and reach Settings.
fn welcome_hint(open_click: bool) -> Alert {
    let body = if open_click { t("Click to open · right-click for Settings") } else { t("Hover to open · right-click for Settings") };
    Alert { title: t("Welcome to My Notch").into(), body: body.into(), kind: Kind::Hint, at: Instant::now() }
}

/// Says which shortcuts another app already owns (`bits` from tray::hotkeys_taken).
fn taken_hint(bits: u8) -> Alert {
    let keys: Vec<&str> = [(tray::TAKEN_ASK, "Ctrl+Alt+Space"), (tray::TAKEN_OCR, "Ctrl+Alt+T"), (tray::TAKEN_NOTE, "Ctrl+Alt+N")]
        .iter()
        .filter(|(b, _)| bits & b != 0)
        .map(|&(_, k)| k)
        .collect();
    Alert { title: t("Shortcut used by another app").into(), body: keys.join(" · "), kind: Kind::Hint, at: Instant::now() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn countdowns_go_on_past_the_table() {
        let d = |y, m, dd| NaiveDate::from_ymd_opt(y, m, dd).unwrap();
        // Inside the table: its dates.
        assert_eq!(next_event(d(2026, 11, 23), 1), Some(d(2027, 2, 8)));
        assert_eq!(next_event(d(2026, 11, 23), 6), Some(d(2027, 2, 6))); // Tết
        assert_eq!(next_event(d(2026, 11, 23), 7), Some(d(2027, 9, 2))); // 2/9
        assert_eq!(next_event(d(2041, 3, 1), 6), None); // past the Tết table: "—", not a wrong date
        // My own date: every year, once, a leap day, and nonsense.
        assert_eq!(my_date(d(2026, 11, 23), "15/03"), Some(d(2027, 3, 15)));
        assert_eq!(my_date(d(2026, 11, 23), "23/11"), Some(d(2026, 11, 23)));
        assert_eq!(my_date(d(2026, 11, 23), "01/06/2027"), Some(d(2027, 6, 1)));
        assert_eq!(my_date(d(2026, 11, 23), "01/06/2026"), None);
        assert_eq!(my_date(d(2026, 11, 23), "29/02"), Some(d(2028, 2, 29)));
        assert_eq!((my_date(d(2026, 11, 23), ""), my_date(d(2026, 11, 23), "31/02"), my_date(d(2026, 11, 23), "soon")), (None, None, None));
        // The worked-out calendar lands within a day of every date in the tables...
        let first = |from: NaiveDate, m: u32, dd: u32| (0..400).map(|k| from + Duration::days(k)).find(|&x| prayer::hijri(x).1 == m && prayer::hijri(x).2 == dd).unwrap();
        for (table, m, dd) in [(&RAMADAN, 9, 1), (&EID_FITR, 10, 1), (&EID_ADHA, 12, 10)] {
            for &(y, mo, da) in table.iter() {
                let want = d(y, mo, da);
                let got = first(want - Duration::days(5), m, dd);
                assert!((got - want).num_days().abs() <= 1, "{want} vs {got}");
            }
        }
        // ...and past them (2031) it still gives a date, the right Hijri day, within a year.
        for (ev, m, dd) in [(1, 9, 1), (2, 10, 1), (3, 12, 10)] {
            let got = next_event(d(2031, 1, 1), ev).unwrap();
            assert!(prayer::hijri(got).1 == m && prayer::hijri(got).2 == dd && got < d(2032, 1, 1), "{ev}: {got}");
        }
    }

    #[test]
    fn restoring_defaults_keeps_what_isnt_a_preference() {
        let mut s = Settings::from_text("", "");
        (s.language, s.goal, s.startup, s.prayer_lat, s.welcomed) = (1, 2500, true, 21.03, true);
        (s.mascot, s.size, s.focus, s.show_clock) = (!s.mascot, 3, 50, !s.show_clock);
        let d = default_settings_keeping(&s);
        let fresh = Settings::from_text("", "");
        assert_eq!((d.language, d.goal, d.startup, d.prayer_lat, d.welcomed), (1, 2500, true, 21.03, true));
        assert_eq!((d.mascot, d.size, d.focus, d.show_clock), (fresh.mascot, fresh.size, fresh.focus, fresh.show_clock));
    }

    #[test]
    fn startup_entries_read_back() {
        // Shaped like `reg query` output.
        let run = "\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Run\r\n    MyNotch    REG_SZ    \"C:\\Users\\x\\AppData\\Local\\Programs\\MyNotch\\MyNotch.exe\"\r\n\r\n";
        assert_eq!(startup_path(run).as_deref(), Some(r"C:\Users\x\AppData\Local\Programs\MyNotch\MyNotch.exe"));
        assert_eq!(startup_path("nothing here"), None);
        let off = "\r\nHKEY_CURRENT_USER\\...\\StartupApproved\\Run\r\n    MyNotch    REG_BINARY    030000004A3B2C1D00000000\r\n";
        let on = "    MyNotch    REG_BINARY    020000000000000000000000\r\n";
        assert!(!startup_approved(off) && startup_approved(on) && startup_approved(""));
    }

    #[test]
    fn copy_takes_the_answer_itself() {
        assert_eq!(answer_text("= 84"), "84");
        assert_eq!(answer_text("= 25,779 VND"), "25,779 VND");
        assert_eq!(answer_text("Hà Nội là thủ đô."), "Hà Nội là thủ đô.");
    }

    #[test]
    fn notifications_group_by_app() {
        let n = |app: &str, title: &str, ts: i64| (ts, (app.to_string(), title.to_string(), String::new(), format!("{app}.id")));
        let recent = vec![n("Zalo", "Minh", 30), n("Teams", "Standup", 20), n("Zalo", "Lan", 10), n("Zalo", "Hoa", 5)];
        let g = group_by_app(&recent);
        assert_eq!(g.len(), 2);
        assert_eq!((g[0].0.as_str(), g[0].2, g[0].3 .0.as_str(), g[0].4), ("Zalo", 30, "Minh", 3)); // the newest kept
        assert_eq!((g[1].0.as_str(), g[1].4), ("Teams", 1));
        assert!(!can_open_app("") && !can_open_app("a\"b") && can_open_app("Microsoft.Teams_8wekyb3d8bbwe!Teams"));
    }

    #[test]
    fn clock_offsets_read_plainly() {
        let _lock = i18n::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        i18n::set(0);
        assert_eq!(offset_text(0), "same time");
        assert_eq!(offset_text(59), "+1 h"); // a clock read a second apart still rounds right
        assert_eq!(offset_text(-360), "−6 h");
        assert_eq!(offset_text(-150), "−2:30 h");
        assert_eq!(offset_text(330), "+5:30 h");
    }

    #[test]
    fn arc_labels_spread_apart() {
        // Maghrib and Isha 10 points apart get pushed to the gap; well-spaced ones stay put.
        assert_eq!(spread(&[0.0, 100.0, 150.0, 200.0, 210.0], 42.0, -10.0, 300.0), vec![0.0, 100.0, 150.0, 200.0, 242.0]);
        // At the right edge they move left instead.
        assert_eq!(spread(&[0.0, 100.0, 250.0, 255.0], 42.0, -10.0, 260.0), vec![0.0, 100.0, 218.0, 260.0]);
    }

    #[test]
    fn litres_show_quarters() {
        assert_eq!((litres(1500), litres(2000), litres(1750), litres(1250)), ("1.5".into(), "2.0".into(), "1.75".into(), "1.25".into()));
    }

    #[test]
    fn notification_icon_pops_then_settles() {
        assert!((pop_in(0.0) - 0.5).abs() < 1e-4);
        assert!(pop_in(NOTICE_POP_SECS * 0.7) > 1.0); // the overshoot
        assert_eq!(pop_in(NOTICE_POP_SECS), 1.0);
        assert_eq!(pop_in(5.0), 1.0);
    }

    #[test]
    fn longer_notifications_stay_longer() {
        assert_eq!(notice_secs("Mai", "Ok"), NOTICE_POPUP_SECS);
        let long = notice_secs("Nhóm: TRI THỨC CHỈ ĐẠO CHUNG", "Anh Tám (Phòng KH): Mai họp lúc 9h nhé mọi người, nhớ mang laptop");
        assert!(long > NOTICE_POPUP_SECS + 1.0 && long < NOTICE_MAX_SECS, "{long}");
        assert_eq!(notice_secs(&"x".repeat(500), ""), NOTICE_MAX_SECS);
    }

    #[test]
    fn blink_closes_smoothly() {
        assert_eq!(blink_open(BLINK_MS), 1.0);
        assert_eq!(blink_open(BLINK_EVERY_MS - 1), 1.0);
        assert!((blink_open(BLINK_MS / 2) - 0.1).abs() < 0.01); // most closed halfway
        assert!(blink_open(BLINK_MS / 4) > blink_open(BLINK_MS / 2)); // closing, not a jump
        assert!(blink_open(BLINK_EVERY_MS + BLINK_MS / 2) < 0.2); // repeats every period
    }

    #[test]
    fn time_helpers() {
        let _english = i18n::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(parse_hm("9:05"), Some(545));
        assert_eq!(parse_hm("24:00"), None);
        assert_eq!(hm(1440), "24:00");
        assert_eq!(mmss(1500.0), "25:00");
        assert_eq!(dur_text(130), "2 h 10 min");
        assert_eq!(ago(30), "Just now");
        assert_eq!(lang_name(0x0409), "EN");
        assert_eq!(lang_name(0x042A), "VI");
    }
    #[test]
    fn dst_dates() {
        assert_eq!(last_sunday(2026, 3), NaiveDate::from_ymd_opt(2026, 3, 29).unwrap());
        assert_eq!(nth_sunday(2026, 3, 2), NaiveDate::from_ymd_opt(2026, 3, 8).unwrap());
        assert_eq!(nth_sunday(2026, 11, 1), NaiveDate::from_ymd_opt(2026, 11, 1).unwrap());
    }
    #[test]
    fn moon_full_2026() {
        // Full moon on 2026-01-03 ~10:03 UTC
        let (i, lit) = moon(NaiveDate::from_ymd_opt(2026, 1, 3).unwrap().and_hms_opt(10, 0, 0).unwrap());
        assert_eq!(i, 4);
        assert!(lit > 0.95);
    }
    #[test]
    fn spring_settles_on_target() {
        let mut s = Spring { x: COLLAPSED.x, v: 0.0 };
        let mut frames = 0;
        while s.step(EXPANDED.x, 1.0 / 60.0, SPRING) {
            frames += 1;
            assert!(frames < 240, "spring never settled");
        }
        assert_eq!(s.x, EXPANDED.x);
    }
}
