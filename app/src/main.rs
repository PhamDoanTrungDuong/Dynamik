#![windows_subsystem = "windows"]
//! A notch at the top of the screen: Pomodoro, Water, Shelf, To-do, Prayer and Widgets pages,
//! drawn to match the original's black notch with page dots and spring motion.
mod calc;
mod calendar;
mod data;
mod ocr;
mod online;
mod i18n;
mod dragout;
mod media;
mod prayer;
mod tray;

use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, Timelike, Utc, Weekday};
use data::*;
use i18n::{t, tr};
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontFamily, FontId, Pos2, Rect, Sense, Shape, Stroke, Vec2};
use std::{collections::{BTreeMap, HashSet}, f32::consts::PI, path::PathBuf, time::Instant};

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
const SETTINGS: Vec2 = vec2(548.0, 430.0); // tall enough to read a section at a time
const LYRICS: Vec2 = vec2(676.0, 184.0); // Music page with lyrics beside it (reference picture 12)
const WIN: Vec2 = vec2(LYRICS.x + 2.0 * EAR_MAX, SETTINGS.y);
const EAR_MAX: f32 = 14.0; // side margin of the pill states in 0x140056ff1
// Springs set on every state change in 0x14007dae7: (stiffness, damping).
const SPRING: (f32, f32) = (230.0, 19.0); // most changes: open, close, pop-up (a little bounce)
const SPRING_FROM_POPUP: (f32, f32) = (110.0, 15.5); // leaving a pop-up for the pill
const SPRING_TO_PEEK: (f32, f32) = (200.0, 29.0); // into the small peek state (no bounce)
// Battery pop-ups (0x140056c76): warnings below 21 % and 11 %; they stay 4.5 s (timer 4 in 0x14007f665).
const BATTERY_LOW: u8 = 21;
const BATTERY_CRITICAL: u8 = 11;
const BATTERY_POPUP_SECS: f32 = 4.5;
// Prayer reminders (0x140002345): at prayer time 10 s (timer 4, 10000 ms), the early one 6 s (6000 ms).
const PRAYER_POPUP_SECS: f32 = 10.0;
const PRAYER_EARLY_SECS: f32 = 6.0;
const PRAYER_WINDOW: (f32, f32) = (-0.05, 2.0); // minutes around the due time
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
const PAGES: [&str; 8] = ["Music", "Pomodoro", "Water", "Shelf", "To-do", "Prayer", "Widgets", "Today"];
const SHELF_PAGE: usize = 3;
const TODAY_PAGE: usize = 7;
// How many notifications the Today page remembers (estimated; not found in the original).
const RECENT_MAX: usize = 20;
// The mascot blinks every 4.2 s; the lids close and open again over 180 ms (estimated).
const BLINK_EVERY_MS: i64 = 4200;
const BLINK_MS: i64 = 180;
const LONG_BREAK_EVERY: u32 = 4;
const RESET_CONFIRM_SECS: f32 = 3.0;
const MOON: [&str; 8] = ["New Moon", "Waxing Crescent", "First Quarter", "Waxing Gibbous", "Full Moon", "Waning Gibbous", "Last Quarter", "Waning Crescent"];
const EVENTS: [&str; 6] = ["New Year", "Ramadan", "Eid al-Fitr", "Eid al-Adha", "Independence Day", "Christmas"];
const WIDGETS: [&str; 9] = ["This week", "Battery", "Next prayer", "Focus today", "World clock", "Moon", "Countdown", "Water", "Weather"];
const WEATHER_WIDGET: usize = 8;
// Estimated Islamic dates (official announcements can differ by a day).
const RAMADAN: [(i32, u32, u32); 5] = [(2026, 2, 18), (2027, 2, 8), (2028, 1, 28), (2029, 1, 16), (2030, 1, 5)];
const EID_FITR: [(i32, u32, u32); 5] = [(2026, 3, 20), (2027, 3, 10), (2028, 2, 27), (2029, 2, 14), (2030, 2, 4)];
const EID_ADHA: [(i32, u32, u32); 5] = [(2026, 5, 27), (2027, 5, 16), (2028, 5, 5), (2029, 4, 24), (2030, 4, 13)];

#[derive(Clone, Copy)]
enum Dst { No, Eu, Us, Au }
// Order and defaults (Tokyo, London) from the original's table at 0x1400e40d8.
const WORLD: [(&str, f64, Dst); 19] = [
    ("Jakarta", 7.0, Dst::No), ("Makassar", 8.0, Dst::No), ("Jayapura", 9.0, Dst::No), ("Singapore", 8.0, Dst::No),
    ("Kuala Lumpur", 8.0, Dst::No), ("Tokyo", 9.0, Dst::No), ("Seoul", 9.0, Dst::No), ("Beijing", 8.0, Dst::No),
    ("Mumbai", 5.5, Dst::No), ("Dubai", 4.0, Dst::No), ("Mecca", 3.0, Dst::No), ("Istanbul", 3.0, Dst::No),
    ("Cairo", 2.0, Dst::Eu), ("London", 0.0, Dst::Eu), ("Paris", 1.0, Dst::Eu), ("Berlin", 1.0, Dst::Eu),
    ("New York", -5.0, Dst::Us), ("Los Angeles", -8.0, Dst::Us), ("Sydney", 10.0, Dst::Au),
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
enum Kind { Drink, Refill, TaskStart(usize), TaskEnd(usize), Event(u32), Hint, Prayer(u32, bool), Battery(u8, bool), Hud(HudKind, f32), Network(u8), Bluetooth(Option<u8>), Notice(String, String, u32), Info, About }
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
    ask_reply: std::sync::Arc<std::sync::Mutex<(u64, String)>>,
    // Calendar: the secret iCal address (sealed too), what the watcher read, and reminders shown.
    ical_text: String,
    ical_url: std::sync::Arc<std::sync::Mutex<String>>,
    calendar: std::sync::Arc<std::sync::Mutex<calendar::Today>>,
    cal_fired: HashSet<(NaiveDate, u32, String)>,
    // Shortcuts: whether a taken one still has to be told. Page turning by scroll: the scroll
    // gathered so far, when it last moved, and until when a flick is ignored after a turn;
    // `scroll_zone` is a list on the page that scrolls itself (no page turns over it).
    hotkeys_told: bool,
    wheel_acc: f32,
    wheel_last: Instant,
    wheel_rest: Instant,
    scroll_zone: Option<Rect>,
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
    // focus
    mode: Mode,
    left: f64,
    running: bool,
    rounds: u32,
    focus_log: BTreeMap<NaiveDate, u32>,
    rounds_day: NaiveDate, // the day `rounds` counts for
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
        // Screenshots never read the secrets.
        let secret = |name: &str| if live { read_secret(name) } else { String::new() };
        let ical_text = secret(ICAL_FILE);
        let ical_url = std::sync::Arc::new(std::sync::Mutex::new(ical_text.clone()));
        App {
            gemini_key: secret(GEMINI_FILE), ask_gen: 0, ask_waiting: false, ask_reply: Default::default(),
            calendar: if live {
                calendar::watch(ical_url.clone())
            } else {
                // --demo on a screenshot of To-do shows made-up events.
                let demo = std::env::args().any(|a| a == "--demo") && page == 4;
                let ev = |title: &str, start, dur| calendar::Event { title: title.into(), start, dur };
                let events = if demo { vec![ev("Team trip", None, 0), ev("Standup", Some(540), 30)] } else { vec![] };
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
            water_day, drank, in_bottle, hours, fired, water_snooze: None, water_settings: false, reset_armed: None,
            mode: Mode::Focus, left, running: false, rounds: 0, rounds_day: now.date(), focus_log, focus_acc: 0.0, last_tick: Instant::now(),
            shelf, shelf_view: ShelfView::Files, clips, clipboard, last_clip, copied: None,
            battery: battery(), prayer_fired: HashSet::new(), hidden: false, media: demo_music(media::Media::start(s.lyrics, live), !live && page == 0), lyrics_open: false, volume: media::volume_watch(), last_volume: None, last_caps: caps_lock(), last_lang: input_lang(), network: media::network_watch(), last_network: None,
            bluetooth: if live { media::bluetooth_watch() } else { Default::default() }, last_headsets: None,
            notices: if live && s.notifications { media::notification_watch() } else { Default::default() },
            weather_place: weather_place.clone(),
            // --weather SYMBOL fills a screenshot's Weather widget (no network).
            weather: if live {
                online::weather_watch(weather_place)
            } else {
                let demo = std::env::args().skip_while(|a| a != "--weather").nth(1).map(|symbol| online::Weather { temp: 28.0, symbol });
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
    /// Prayer times with the per-prayer adjustments from Settings (0x1400863af).
    fn prayer_times(&self, day: NaiveDate) -> [u32; 5] {
        let t = prayer::times_for(day, self.s.prayer_city, (self.s.prayer_lat, self.s.prayer_lon), self.s.prayer_method.checked_sub(1));
        std::array::from_fn(|i| (t[i] as i32 + self.s.prayer_adj[i]).clamp(0, 1439) as u32)
    }
    fn focus_today(&self) -> u32 { self.focus_log.get(&self.today()).copied().unwrap_or(0) }
    fn done_today(&self) -> usize { self.tasks.iter().filter(|t| t.done_day == Some(self.today())).count() }

    fn drink(&mut self) {
        let each = self.s.water_plan().each;
        self.drank += each;
        self.hours[self.now.hour() as usize] += each;
        self.in_bottle = self.in_bottle.saturating_sub(each);
        self.save_water();
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
            if self.rounds % LONG_BREAK_EVERY == 0 {
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

        // A new day starts the 4 rounds over, unless a part is under way.
        if self.rounds_day != today && !self.running && self.left >= self.mode_len(self.mode) {
            self.rounds_day = today;
            self.rounds = 0;
            self.mode = Mode::Focus;
            self.left = self.mode_len(Mode::Focus);
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

        if self.water_day != today {
            self.water_day = today;
            self.drank = 0;
            self.fired = 0;
            self.hours = [0; 24];
            self.save_water();
        }
        // Water reminders as the original schedules them (0x140001fe5): slot i is due at
        // from + (i + 0.5) * interval and shows if seen within 2 minutes; a snooze within 3.
        if self.s.water && self.alert.is_none() && self.drank < self.s.goal {
            let now = m as f32 + self.now.second() as f32 / 60.0;
            let p = self.s.water_plan();
            let mut due = false;
            if let Some(at) = self.water_snooze.filter(|&at| now >= at as f32) {
                self.water_snooze = None;
                due = now < at as f32 + 3.0;
            }
            for i in 0..p.reminders.min(64) {
                let t = self.s.from as f32 + (i as f32 + 0.5) * p.interval;
                if self.fired & (1 << i) == 0 && now >= t && now < t + 2.0 {
                    self.fired |= 1 << i;
                    due = true;
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

        if self.alert.is_none() {
            let hit = self.tasks.iter().enumerate().find_map(|(i, t)| {
                let st = t.start? as u32;
                if t.done_day.is_some() || m < t.snooze_until {
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
            }
        }

        // Calendar events remind as tasks do: at the start, within the first hour, once.
        if self.alert.is_none() && self.s.todo_start {
            let today = self.today();
            let events = self.calendar.lock().unwrap().events.clone();
            let due = events.into_iter().find(|e| {
                e.start.is_some_and(|st| m >= st && m < st + 60) && !self.cal_fired.contains(&(today, e.start.unwrap(), e.title.clone()))
            });
            if let Some(e) = due {
                let st = e.start.unwrap();
                self.cal_fired.insert((today, st, e.title.clone()));
                let title = if e.title.is_empty() { t("(No title)").to_string() } else { e.title };
                self.notify(t("Calendar"), &title, Kind::Event(st));
            }
        }

        // "My location" not found yet would time prayers for 0°, 0°: no reminders until it is.
        let placed = self.s.prayer_city != prayer::here_index() || (self.s.prayer_lat, self.s.prayer_lon) != (0.0, 0.0);
        if self.s.prayer && placed && (self.s.prayer_reminder || self.s.prayer_pause_media) && self.alert.is_none() {
            let t = self.prayer_times(today);
            'outer: for (i, &pt) in t.iter().enumerate() {
                for early in [false, true] {
                    if early && !(self.s.prayer_early && self.s.prayer_reminder) {
                        continue;
                    }
                    let at = if early { pt.saturating_sub(PRAYER_EARLY_MIN) } else { pt };
                    // Due from 3 s before until 2 min after (0x140002345: window -0.05 .. +2.0 minutes).
                    let now_f = m as f32 + self.now.second() as f32 / 60.0;
                    if now_f >= at as f32 + PRAYER_WINDOW.0 && now_f < at as f32 + PRAYER_WINDOW.1 && self.prayer_fired.insert((today, i, early)) {
                        let body = if early { tr("In {} minutes", &[&PRAYER_EARLY_MIN]) } else { i18n::t("It's time to pray").to_string() };
                        if self.s.prayer_reminder {
                            self.notify(prayer::NAMES[i], &body, Kind::Prayer(pt, early));
                        }
                        // "Pause media at prayer time": only at the time itself, and it does not resume.
                        if !early && self.s.prayer_pause_media {
                            self.media.send(media::Cmd::Pause);
                        }
                        break 'outer;
                    }
                }
            }
        }

        // Read again only what Windows says changed (screenshots get no events and need none).
        let ev = if self.shot.is_none() { tray::take_events() } else { 0 };
        // While hidden it looks each time (see idle_wait), so it comes back even if an event is missed.
        if ev & tray::EV_WINDOW != 0 || self.hidden || !self.s.hide_fullscreen {
            self.hidden = self.shot.is_none() && self.s.hide_fullscreen && fullscreen_in_front();
        }
        if ev & tray::EV_CLIP != 0 {
            if self.s.clipboard {
                if let Some(t) = self.clipboard.as_mut().and_then(|c| c.get_text().ok()) {
                    if t != self.last_clip {
                        self.last_clip = t.clone();
                        if !t.trim().is_empty() {
                            self.clips.retain(|c| c.1 != t);
                            self.clips.insert(0, (Utc::now().timestamp(), t));
                            self.clips.truncate(self.s.clipboard_max);
                            self.save_clips();
                        }
                    }
                }
            }
        }

        if ev & tray::EV_POWER != 0 {
            let old = self.battery;
            self.battery = battery();
            // The original's rules (0x140056c76): a charger change first; otherwise a warning only when
            // the level crosses 21 % / 11 % going down on battery, or reaches 100 % while charging.
            if let (true, Some((p, ac)), Some((op, oac))) = (self.s.popup_battery && self.alert.is_none(), self.battery, old) {
                if ac != oac {
                    if ac {
                        self.notify(t("Charging"), t("Charger connected"), Kind::Battery(p, ac));
                    } else {
                        self.notify(t("Charger unplugged"), &format!("{p}%"), Kind::Battery(p, ac));
                    }
                } else if !ac && p < BATTERY_CRITICAL && op >= BATTERY_CRITICAL {
                    self.notify(t("Battery critical"), t("Plug in now"), Kind::Battery(p, ac));
                } else if !ac && p < BATTERY_LOW && op >= BATTERY_LOW {
                    self.notify(t("Low battery"), t("Plug in soon"), Kind::Battery(p, ac));
                } else if ac && p >= 100 && op < 100 {
                    self.notify(t("Fully charged"), t("You can unplug"), Kind::Battery(p, ac));
                }
            }
        }

        self.watch_indicators();

        // Information pop-ups close by themselves; the ones with buttons wait for an answer.
        if self.alert.as_ref().is_some_and(|a| matches!(a.kind, Kind::Network(_)) && a.at.elapsed().as_secs_f32() > NET_POPUP_SECS) {
            self.alert = None;
        }
        if self.alert.as_ref().is_some_and(|a| matches!(a.kind, Kind::Hud(..)) && a.at.elapsed().as_secs_f32() > HUD_SECS) {
            self.alert = None;
        }
        if self.alert.as_ref().is_some_and(|a| match a.kind {
            Kind::Battery(..) => a.at.elapsed().as_secs_f32() > BATTERY_POPUP_SECS,
            Kind::Bluetooth(_) => a.at.elapsed().as_secs_f32() > BT_POPUP_SECS,
            // Longer messages stay longer, and none goes while the pointer rests on it.
            Kind::Notice(..) => a.at.elapsed().as_secs_f32() > notice_secs(&a.title, &a.body) && self.last_inside.elapsed().as_secs_f32() > NOTICE_AFTER_HOVER_SECS,
            Kind::About => a.at.elapsed().as_secs_f32() > ABOUT_SECS,
            Kind::Hint => a.at.elapsed().as_secs_f32() > HINT_SECS,
            Kind::Prayer(_, early) => a.at.elapsed().as_secs_f32() > if early { PRAYER_EARLY_SECS } else { PRAYER_POPUP_SECS },
            Kind::Event(_) => a.at.elapsed().as_secs_f32() > PRAYER_POPUP_SECS,
            _ => false,
        }) {
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
    /// keys while nothing is being typed. Over a list that scrolls itself, only sideways scroll turns.
    fn turn_by_scroll_and_keys(&mut self, ui: &mut egui::Ui) {
        let (order, at) = (self.order(), self.pos());
        let typing = ui.ctx().memory(|m| m.focused().is_some());
        let (left, right, d, over) = ui.input(|i| {
            let keys = !typing && i.modifiers.is_none();
            let over = i.pointer.hover_pos().zip(self.scroll_zone).is_some_and(|(p, z)| z.contains(p));
            (keys && i.key_pressed(egui::Key::ArrowLeft), keys && i.key_pressed(egui::Key::ArrowRight), i.raw_scroll_delta, over)
        });
        let step = if over { d.x } else { d.x + d.y };
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
        let mut go = left as i32 * -1 + right as i32;
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

    // ---------- collapsed and pop-up ----------

    /// Volume, Caps Lock and keyboard-language changes show a short indicator, unless a pop-up
    /// that needs an answer is up.
    fn watch_indicators(&mut self) {
        let free = self.alert.as_ref().is_none_or(|a| matches!(a.kind, Kind::Hud(..) | Kind::Info | Kind::Battery(..) | Kind::Bluetooth(_) | Kind::Notice(..)));
        let vol = *self.volume.lock().unwrap();
        if let (Some((v, m)), Some((ov, om))) = (vol, self.last_volume) {
            if self.s.hud_volume && free && (m != om || (v - ov).abs() > 0.004) {
                let kind = if m { HudKind::Muted } else { HudKind::Volume };
                self.alert = Some(Alert { title: t("Volume").into(), body: String::new(), kind: Kind::Hud(kind, v), at: Instant::now() });
            }
        }
        if vol.is_some() {
            self.last_volume = vol;
        }
        let (seq, level) = tray::brightness();
        if seq != self.last_bright {
            self.last_bright = seq;
            if self.s.hud_brightness && free {
                self.alert = Some(Alert { title: t("Brightness").into(), body: String::new(), kind: Kind::Hud(HudKind::Brightness, level), at: Instant::now() });
            }
        }
        // A headset that was not connected at the last look shows once; those connected at start do not.
        let heads = self.bluetooth.lock().unwrap().clone();
        if let Some(heads) = heads {
            let now: HashSet<u64> = heads.iter().map(|h| h.0).collect();
            let mut wait = false;
            if let Some(old) = &self.last_headsets {
                if let Some((_, name, battery)) = heads.iter().find(|h| !old.contains(&h.0)) {
                    if self.s.popup_bluetooth && free {
                        let body = battery.map_or(t("Connected").to_string(), |b| tr("Connected · {}%", &[&b]));
                        self.alert = Some(Alert { title: name.clone(), body, kind: Kind::Bluetooth(*battery), at: Instant::now() });
                    }
                    // A pop-up waiting for an answer is up: show this one after it, not never.
                    wait = self.s.popup_bluetooth && !free;
                }
            }
            if !wait {
                self.last_headsets = Some(now);
            }
        }
        let new: Vec<media::Notice> = self.notices.lock().unwrap().drain(..).collect();
        // The Today page lists them, newest first; kept in memory only, so nothing private is written to disk.
        let ts = Utc::now().timestamp();
        if self.s.notifications {
            self.recent.splice(0..0, new.iter().rev().map(|n| (ts, n.clone())));
        } else {
            // Turned off while running (the watcher keeps going until restart): keep nothing.
            self.recent.clear();
        }
        self.recent.truncate(RECENT_MAX);
        let arrived = new.len() as u32;
        if let Some((app, title, body, id)) = new.into_iter().last() {
            if self.s.notifications && free {
                let body = if self.s.notification_preview { body } else { String::new() };
                // The newest shows; "+N" counts the ones it covers (all still on the Today page).
                let covered = match &self.alert {
                    Some(Alert { kind: Kind::Notice(_, _, more), .. }) => more + 1,
                    _ => 0,
                };
                self.alert = Some(Alert { title, body, kind: Kind::Notice(app, id, covered + arrived - 1), at: Instant::now() });
            }
        }
        // A network change shows once it has held for 1.5 s, like the original's debounce.
        let net = self.network.lock().unwrap().clone();
        if let Some(net) = net {
            match &self.last_network {
                None => self.last_network = Some(net),
                Some(last) if *last == net => self.network_change = None,
                Some(_) => match &self.network_change {
                    Some((seen, at)) if *seen == net => {
                        if at.elapsed().as_secs_f32() >= NET_SETTLE_SECS {
                            self.network_change = None;
                            self.last_network = Some(net.clone());
                            if self.s.popup_wifi && free {
                                let body = [t("Disconnected"), t("No internet access"), t("Connected")][net.1.min(2) as usize];
                                self.alert = Some(Alert { title: net.0, body: body.into(), kind: Kind::Network(net.1), at: Instant::now() });
                            }
                        }
                    }
                    _ => self.network_change = Some((net, Instant::now())),
                },
            }
        }
        let caps = caps_lock();
        if caps != self.last_caps {
            self.last_caps = caps;
            if self.s.hud_caps && free {
                self.alert = Some(Alert { title: t("Caps Lock").into(), body: if caps { t("On") } else { t("Off") }.into(), kind: Kind::Hud(HudKind::Caps, caps as u8 as f32), at: Instant::now() });
            }
        }
        let lang = input_lang();
        if lang != self.last_lang && lang != 0 {
            self.last_lang = lang;
            if self.s.hud_language && free {
                self.alert = Some(Alert { title: t("Input language").into(), body: lang_name(lang), kind: Kind::Hud(HudKind::Lang, 0.0), at: Instant::now() });
            }
        }
    }

    /// Nothing to show: no session running or paused, no song playing, no mascot to look at.
    fn idle(&self) -> bool {
        !self.running && self.left >= self.mode_len(self.mode) && !self.s.mascot && !self.media.now.lock().is_ok_and(|n| n.playing)
    }

    fn draw_collapsed(&mut self, ui: &mut egui::Ui, r: Rect) {
        if self.target_size() == PEEK {
            return; // the bar itself is all there is
        }
        let y = r.center().y;
        let timer = self.running || self.left < self.mode_len(self.mode);
        // A song playing (and no session counting down): its cover on the left, the level bars in
        // its colour on the right, the clock between. It takes the mascot's place while it plays.
        let now = self.media.now.lock().map(|n| n.clone()).unwrap_or_default();
        if now.playing && !timer {
            let c = pos2(r.left() + 24.0, y);
            match self.art_texture(ui.ctx(), &now) {
                Some(tex) => {
                    egui::Image::new(&tex).corner_radius(6).paint_at(ui, Rect::from_center_size(c, vec2(22.0, 22.0)));
                }
                None => album_art(ui.painter(), Rect::from_center_size(c, vec2(22.0, 22.0))),
            }
            let p = ui.painter();
            if self.s.show_clock {
                p.text(r.center(), Align2::CENTER_CENTER, self.now.format("%H:%M").to_string(), bold(18.0), WHITE);
            }
            bars(p, pos2(r.right() - 24.0, y), false, self.music_tint());
            return;
        }
        let p = ui.painter();
        if self.s.mascot && !self.running && self.left >= self.mode_len(self.mode) {
            mascot(p, r);
            return;
        }
        if self.s.status_ring < 2 {
            self.status_ring(p, pos2(r.left() + 26.0, y), 9.0);
        }
        // A session's countdown always shows; the clock only with the clock switch on.
        if timer {
            p.text(r.center(), Align2::CENTER_CENTER, mmss(self.left), bold(18.0), PURPLE);
        } else if self.s.show_clock {
            p.text(r.center(), Align2::CENTER_CENTER, self.now.format("%H:%M").to_string(), bold(18.0), WHITE);
        }
        if !self.s.minimal {
            bars(p, pos2(r.right() - 26.0, y), self.running, self.music_tint());
        }
    }

    fn draw_popup(&mut self, ui: &mut egui::Ui, r: Rect) {
        let Some(a) = &self.alert else { return };
        let (title, body) = (a.title.clone(), a.body.clone());
        let since = a.at.elapsed().as_secs_f32();
        let p = ui.painter().clone();
        let icon = pos2(r.left() + 46.0, r.center().y);
        let tx = r.left() + 84.0;
        if !matches!(a.kind, Kind::Hud(..) | Kind::Notice(..)) {
            p.text(pos2(tx, r.center().y - 9.0), Align2::LEFT_CENTER, &title, bold(17.0), WHITE);
            p.text(pos2(tx, r.center().y + 13.0), Align2::LEFT_CENTER, &body, reg(14.0), DIM);
        }
        let right = r.right() - 28.0;
        let mut close = false;
        match a.kind {
            Kind::Battery(pct, ac) => {
                ring(&p, icon, 14.0, 4.0, pct as f32 / 100.0, if ac || pct > 20 { GREEN } else { RED });
                bolt(&p, icon, 6.0, WHITE);
                p.text(pos2(right, r.center().y), Align2::RIGHT_CENTER, format!("{pct}%"), reg(28.0), WHITE);
            }
            Kind::Prayer(at, _) => {
                moon_icon(&p, icon, 11.0);
                p.text(pos2(right, r.center().y - 4.0), Align2::RIGHT_CENTER, hm(at), bold(17.0), WHITE);
                let bar = Rect::from_min_max(pos2(r.left() + 32.0, r.bottom() - 12.0), pos2(r.right() - 32.0, r.bottom() - 7.0));
                gradient_bar(&p, bar, 1.0, BLUE, PURPLE);
            }
            Kind::Event(at) => {
                calendar_icon(&p, icon, BLUE);
                p.text(pos2(right, r.center().y), Align2::RIGHT_CENTER, hm(at), bold(17.0), WHITE);
            }
            Kind::Drink | Kind::Refill => {
                drop_icon(&p, icon, 12.0, BLUE);
                let refill = matches!(a.kind, Kind::Refill);
                let first = if refill { t("Refilled") } else { t("Drank") };
                if pill_button(ui, Rect::from_min_size(pos2(right - 168.0, r.center().y - 15.0), vec2(84.0, 30.0)), first, BLUE_DARK, "p1") {
                    if refill {
                        // Refilled fills the bottle, then asks for the drink again (0x1400583ab).
                        self.in_bottle = self.s.bottle;
                        self.save_water();
                        let each = self.s.water_plan().each;
                        self.alert = Some(Alert { title: t("Time to Drink").into(), body: format!("{each} ml"), kind: Kind::Drink, at: Instant::now() });
                        return;
                    }
                    self.drink();
                    close = true;
                }
                if pill_button(ui, Rect::from_min_size(pos2(right - 78.0, r.center().y - 15.0), vec2(78.0, 30.0)), t("Snooze"), BTN, "p2") {
                    self.water_snooze = Some((self.now_min() + WATER_SNOOZE_MIN).min(1439));
                    close = true;
                }
            }
            Kind::TaskStart(i) | Kind::TaskEnd(i) => {
                let end = matches!(a.kind, Kind::TaskEnd(_));
                check_circle(&p, icon, 13.0, false);
                if pill_button(ui, Rect::from_min_size(pos2(right - 152.0, r.center().y - 15.0), vec2(70.0, 30.0)), t("Done"), GREEN_DARK, "p1") {
                    if let Some(t) = self.tasks.get_mut(i) {
                        t.done_day = Some(self.now.date());
                    }
                    data::save_tasks(&self.tasks);
                    close = true;
                }
                if pill_button(ui, Rect::from_min_size(pos2(right - 76.0, r.center().y - 15.0), vec2(76.0, 30.0)), t("Later"), BTN, "p2") {
                    let m = self.now_min();
                    if let Some(t) = self.tasks.get_mut(i) {
                        t.snooze_until = m + self.s.snooze;
                        if end { t.fired_end = false } else { t.fired_start = false }
                    }
                    close = true;
                }
            }
            Kind::Hud(kind, level) => {
                let c = pos2(r.left() + 40.0, r.center().y);
                match kind {
                    HudKind::Volume | HudKind::Muted | HudKind::Brightness => {
                        if matches!(kind, HudKind::Brightness) {
                            sun_icon(&p, c, WHITE);
                        } else {
                            speaker_icon(&p, c, WHITE, matches!(kind, HudKind::Muted));
                        }
                        let bar = Rect::from_min_max(pos2(r.left() + 72.0, r.center().y - 3.0), pos2(r.right() - 74.0, r.center().y + 3.0));
                        let lv = if matches!(kind, HudKind::Muted) { 0.0 } else { level };
                        gradient_bar(&p, bar, lv, WHITE, WHITE);
                        p.text(pos2(r.right() - 26.0, r.center().y), Align2::RIGHT_CENTER, format!("{}", (lv * 100.0).round()), bold(17.0), WHITE);
                    }
                    HudKind::Caps | HudKind::Lang => {
                        let tag = if matches!(kind, HudKind::Caps) { "⇪".to_string() } else { body.clone() };
                        let box_ = Rect::from_center_size(c, vec2(34.0, 26.0));
                        p.rect_stroke(box_, 6.0, Stroke::new(1.6_f32, WHITE), egui::StrokeKind::Inside);
                        if matches!(kind, HudKind::Caps) {
                            p.add(Shape::convex_polygon(vec![c + vec2(0.0, -8.0), c + vec2(7.0, 0.0), c + vec2(-7.0, 0.0)], WHITE, Stroke::NONE));
                            p.rect_filled(Rect::from_center_size(c + vec2(0.0, 4.5), vec2(6.0, 7.0)), 1.0, WHITE);
                        } else {
                            p.text(c, Align2::CENTER_CENTER, &tag, bold(12.0), WHITE);
                        }
                        p.text(pos2(r.left() + 76.0, r.center().y), Align2::LEFT_CENTER, &title, bold(16.0), WHITE);
                        let col = if matches!(kind, HudKind::Caps) && level > 0.5 { GREEN } else { DIM };
                        p.text(pos2(r.right() - 26.0, r.center().y), Align2::RIGHT_CENTER, &body, bold(16.0), col);
                    }
                }
            }
            Kind::Network(level) => {
                // Signal arcs, grey when there's no connection.
                let col = if level == 2 { WHITE } else { DIM };
                for (k, rad) in [5.0f32, 10.0, 15.0].iter().enumerate() {
                    let pts: Vec<Pos2> = (0..=10).map(|i| {
                        let a = -PI * 0.75 + PI * 0.5 * i as f32 / 10.0;
                        icon + vec2(0.0, 8.0) + *rad * vec2(a.cos(), a.sin())
                    }).collect();
                    p.add(Shape::line(pts, Stroke::new(2.2_f32, if k as u8 + 1 <= level.max(1) * 2 { col } else { FAINT })));
                }
                p.circle_filled(icon + vec2(0.0, 8.0), 2.0, col);
            }
            Kind::Bluetooth(battery) => {
                // Blue ring holding the battery level around the Bluetooth rune; a headphone badge on the right.
                ring(&p, icon, 14.0, 4.0, battery.map_or(1.0, |b| b as f32 / 100.0), BLUE);
                bluetooth_rune(&p, icon, 6.0, WHITE);
                let badge = pos2(right - 16.0, r.center().y);
                p.circle_filled(badge, 20.0, Color32::from_rgb(12, 30, 44));
                p.circle_stroke(badge, 20.0, Stroke::new(1.5_f32, BLUE_DARK));
                headphone_icon(&p, badge, 8.0, WHITE);
            }
            Kind::Notice(ref app, ref id, more) => {
                // One calm line: the app's logo (or its initial in a dot of its own colour), who sent
                // it, then (preview on) what they said.
                let (app, id) = (app.clone(), id.clone());
                let y = r.center().y;
                let dot = pos2(r.left() + 30.0, y);
                // On arrival the icon pops in, and a cyan-to-violet light runs once along the bottom.
                let s = pop_in(since);
                if let Some(tex) = self.logo_texture(ui.ctx(), &id) {
                    // On a dark plate, so logos drawn for light backgrounds still show.
                    p.rect_filled(Rect::from_center_size(dot, vec2(26.0, 26.0) * s), 7.0 * s, Color32::from_gray(38));
                    egui::Image::new(&tex).corner_radius(5).paint_at(ui, Rect::from_center_size(dot, vec2(20.0, 20.0) * s));
                } else {
                    p.circle_filled(dot, 12.0 * s, app_color(&app));
                    let initial: String = app.chars().next().map(|c| c.to_uppercase().collect()).unwrap_or_default();
                    p.text(dot, Align2::CENTER_CENTER, initial, bold(13.0 * s), WHITE);
                }
                let u = since / NOTICE_SWEEP_SECS;
                if u < 1.0 {
                    let fade = (1.0 - u) * (u * 6.0).min(1.0);
                    let col = lerp_color(Color32::from_rgb(34, 211, 238), Color32::from_rgb(110, 50, 200), u).gamma_multiply(fade);
                    let cx = r.left() + 30.0 + (r.width() - 60.0) * u;
                    p.line_segment([pos2(cx - 50.0, r.bottom() - 2.5), pos2(cx + 50.0, r.bottom() - 2.5)], Stroke::new(2.5_f32, col));
                }
                // Line 1: who sent it, with the app's name dim on the right (a group's name alone
                // doesn't say which app). Line 2, when there is one: what they said, full width.
                let x = dot.x + 22.0;
                let mut right = r.right() - 20.0;
                let y1 = if body.is_empty() { y } else { y - 10.0 };
                if more > 0 {
                    // How many more came in while this one showed: a small pill at line 1's end.
                    let tag = p.layout_no_wrap(format!("+{more}"), bold(10.0), WHITE);
                    let badge = Rect::from_min_size(pos2(right - tag.size().x - 10.0, y1 - 7.5), vec2(tag.size().x + 10.0, 15.0));
                    p.rect_filled(badge, 7.5, BLUE);
                    p.galley(badge.center() - tag.size() / 2.0, tag, WHITE);
                    right = badge.left() - 8.0;
                }
                let name = p.layout_no_wrap(app.clone(), reg(11.0), DIM);
                let nw = if app.is_empty() || app == title { 0.0 } else { name.size().x + 10.0 };
                let two = !body.is_empty();
                let who = fit_line(&p, &title, bold(14.0), WHITE, right - x - nw);
                p.galley(pos2(x, y1 - who.size().y / 2.0), who, WHITE);
                if nw > 0.0 {
                    p.galley(pos2(right - name.size().x, y1 - name.size().y / 2.0), name, DIM);
                }
                if two {
                    let said = fit_line(&p, &body, reg(13.0), DIM, r.right() - 20.0 - x);
                    p.galley(pos2(x, y + 10.0 - said.size().y / 2.0), said, DIM);
                }
            }
            Kind::About | Kind::Hint => {
                // A violet dot beside the name and version (title and body are drawn above).
                ring(&p, icon, 14.0, 4.0, 1.0, PURPLE);
            }
            Kind::Info => {
                // The original waits here with one button that starts the next part (0x14000143f).
                ring(&p, icon, 14.0, 4.0, 1.0, PURPLE);
                let label = if self.mode == Mode::Focus { t("Focus") } else { t("Break") };
                if pill_button(ui, Rect::from_min_size(pos2(right - 84.0, r.center().y - 15.0), vec2(84.0, 30.0)), label, Color32::from_rgb(76, 29, 149), "phase") {
                    self.running = true;
                    self.last_tick = Instant::now();
                    close = true;
                }
            }
        }
        if close {
            self.alert = None;
        }
    }

    // ---------- pages ----------

    fn page_title(&self, i: usize) -> (&'static str, &'static str, Color32) {
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

    fn draw_page(&mut self, ui: &mut egui::Ui, i: usize, r: Rect) {
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

    /// Music, laid out like reference pictures 13 (plain) and 12 (lyrics open beside it).
    fn page_music(&mut self, ui: &mut egui::Ui, r: Rect) {
        let now = self.media.now.lock().map(|n| n.clone()).unwrap_or_default();
        let p = ui.painter().clone();
        let lyr = self.lyrics_open;
        let art = Rect::from_min_size(r.min + if lyr { vec2(28.0, 16.0) } else { vec2(30.0, 18.0) }, vec2(78.0, 78.0));
        // The song's real cover; the drawn one only while the player gives none.
        match self.art_texture(ui.ctx(), &now) {
            Some(tex) => {
                egui::Image::new(&tex).corner_radius(10).paint_at(ui, art);
            }
            None => album_art(&p, art),
        }
        // The player's icon on the cover's corner, so it's clear where the music comes from.
        if now.active() && !now.app_id.is_empty() {
            let c = art.right_bottom() - vec2(4.0, 4.0);
            p.circle_filled(c, 13.0, BG);
            // With several players open (Spotify and a browser, say), the icon switches between them.
            if now.players > 1 {
                let name = tr("Switch player ({} open)", &[&now.players]);
                let resp = ui.interact(Rect::from_center_size(c, vec2(26.0, 26.0)), ui.id().with("player"), Sense::click()).on_hover_text(&name);
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
                p.circle_stroke(c, 12.5, Stroke::new(1.5_f32, if resp.hovered() { WHITE } else { DIM }));
                if resp.clicked() {
                    self.media.send(media::Cmd::NextPlayer);
                }
            }
            match self.logo_texture(ui.ctx(), &now.app_id) {
                Some(tex) => {
                    p.circle_filled(c, 11.0, Color32::from_gray(38));
                    egui::Image::new(&tex).corner_radius(4).paint_at(ui, Rect::from_center_size(c, vec2(15.0, 15.0)));
                }
                None => {
                    p.circle_filled(c, 11.0, app_color(&now.app_name));
                    let initial: String = now.app_name.chars().next().map(|ch| ch.to_uppercase().collect()).unwrap_or_default();
                    p.text(c, Align2::CENTER_CENTER, initial, bold(11.0), WHITE);
                }
            }
        }
        let tx = art.right() + 22.0;
        let (title, sub) = if now.active() {
            let sub = if now.album.is_empty() { now.artist.clone() } else { format!("{} - {}", now.artist, now.album) };
            (now.title.clone(), sub)
        } else {
            (t("Nothing playing").to_string(), t("Play music in any app").to_string())
        };
        // Cut by width, not letters: up to the lyrics column, or short of the page arrow.
        let room = if lyr { r.left() + 322.0 } else { r.right() - 48.0 } - tx;
        let line = |text: &str, font: FontId, col: Color32, y: f32| {
            let g = fit_line(&p, text, font, col, room);
            p.galley(pos2(tx, y - g.size().y / 2.0), g, col);
        };
        line(&title, bold(18.0), WHITE, art.top() + 44.0);
        line(&sub, reg(14.0), DIM, art.top() + 65.0);

        // Progress row: elapsed, bar, remaining.
        let row_y = r.top() + if lyr { 116.0 } else { 120.0 };
        let (bar_l, bar_r, rem_x) = if lyr { (r.left() + 98.0, r.left() + 238.0, r.left() + 306.0) } else { (r.left() + 100.0, r.right() - 106.0, r.right() - 34.0) };
        let (pos, dur) = (now.pos(), now.duration);
        p.text(pos2(r.left() + 32.0, row_y), Align2::LEFT_CENTER, mmss(pos), reg(15.0), WHITE);
        let bar = Rect::from_min_max(pos2(bar_l, row_y - 2.0), pos2(bar_r, row_y + 2.0));
        p.rect_filled(bar, 2.0, Color32::from_gray(55));
        if dur > 0.0 {
            // Click or drag along the bar to seek; while dragging it follows the pointer.
            let resp = ui.interact(bar.expand2(vec2(4.0, 8.0)), ui.id().with("seek"), Sense::click_and_drag());
            let at = resp.interact_pointer_pos().map(|q| ((q.x - bar.left()) / bar.width()).clamp(0.0, 1.0));
            if let (Some(u), true) = (at, resp.clicked() || resp.drag_stopped()) {
                self.media.send(media::Cmd::Seek(u as f64 * dur));
            }
            let f = at.filter(|_| resp.dragged()).unwrap_or((pos / dur).clamp(0.0, 1.0) as f32);
            p.rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * f, bar.height())), 2.0, now.tint.map_or(WHITE, |[r, g, b]| Color32::from_rgb(r, g, b)));
        }
        let rem = if dur > 0.0 { format!("-{}", mmss(dur - pos)) } else { "-00:00".into() };
        p.text(pos2(rem_x, row_y), Align2::RIGHT_CENTER, rem, reg(15.0), WHITE);

        // Controls.
        let cy = r.top() + if lyr { 150.0 } else { 155.0 };
        let (lx, mid, gap) = if lyr { (r.left() + 68.0, r.left() + 170.0, 52.0) } else { (r.left() + 68.0, r.center().x, 62.0) };
        let on = if now.active() { WHITE } else { FAINT };
        if glyph_button(ui, pos2(lx, cy), "lyrics", if self.s.lyrics { WHITE } else { FAINT }) && self.s.lyrics {
            self.lyrics_open = !self.lyrics_open;
        }
        if glyph_button(ui, pos2(mid - gap, cy), "prev", on) {
            self.media.send(media::Cmd::Prev);
        }
        if glyph_button(ui, pos2(mid, cy), if now.playing { "pause" } else { "play" }, on) {
            self.media.send(media::Cmd::Toggle);
        }
        if glyph_button(ui, pos2(mid + gap, cy), "next", on) {
            self.media.send(media::Cmd::Next);
        }

        if lyr {
            let col = Rect::from_min_max(pos2(r.left() + 344.0, r.top() + 14.0), pos2(r.right() - 30.0, r.bottom() - 8.0));
            p.line_segment([pos2(col.left() - 16.0, r.top() + 30.0), pos2(col.left() - 16.0, r.bottom() - 30.0)], Stroke::new(1.0_f32, Color32::from_gray(40)));
            let clip = p.with_clip_rect(col);
            match now.lyrics_state {
                media::LyricsState::Found => {
                    // The sung line sits in the middle in bold white; neighbours fade with distance.
                    let cur = now.line().unwrap_or(0);
                    let mut jump = None;
                    for (k, (t, line)) in now.lyrics.iter().enumerate() {
                        let d = k as f32 - cur as f32;
                        if d.abs() > 3.0 {
                            continue;
                        }
                        let y = col.center().y - 6.0 + d * 38.0;
                        // Click a line to jump there; it brightens under the pointer.
                        let hit = Rect::from_min_max(pos2(col.left(), y - 18.0), pos2(col.right(), y + 18.0)).intersect(col);
                        let resp = ui.interact(hit, ui.id().with(("lyric", k)), Sense::click());
                        if resp.clicked() {
                            jump = Some(*t);
                        }
                        let (font, mut color) = if k == cur { (bold(19.0), WHITE) } else { (bold(17.0), Color32::from_gray((120.0 - d.abs() * 25.0).max(40.0) as u8)) };
                        if resp.hovered() && k != cur {
                            color = Color32::from_gray(200);
                        }
                        let g = clip.layout(line.clone(), font, color, col.width());
                        clip.galley(pos2(col.left(), y - g.size().y / 2.0), g, color);
                    }
                    if let Some(t) = jump {
                        self.media.send(media::Cmd::Seek(t));
                    }
                }
                state => {
                    let msg = match state {
                        media::LyricsState::Loading => t("Loading lyrics…"),
                        _ if !now.active() => t("Lyrics show here while music plays"),
                        _ => t("No lyrics found for this song"),
                    };
                    clip.text(pos2(col.left(), col.center().y), Align2::LEFT_CENTER, msg, bold(16.0), DIM);
                    clip.text(pos2(col.left(), col.center().y + 24.0), Align2::LEFT_CENTER, t("From LRCLIB, a community-contributed lyrics database"), reg(11.0), FAINT);
                }
            }
        }
    }

    fn page_pomodoro(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = ui.painter().clone();
        let x = r.left() + 60.0;
        let label = match self.mode {
            Mode::Focus => tr("Focus · {} of {}", &[&(self.rounds % LONG_BREAK_EVERY + 1).min(LONG_BREAK_EVERY), &LONG_BREAK_EVERY]),
            Mode::Short => t("Short break").into(),
            Mode::Long => t("Long break").into(),
        };
        p.text(pos2(x, r.top() + 50.0), Align2::LEFT_CENTER, label, reg(13.0), DIM);
        p.text(pos2(x - 2.0, r.top() + 80.0), Align2::LEFT_CENTER, mmss(self.left), bold(36.0), WHITE);
        let done_at = self.now + Duration::seconds(self.left.max(0.0) as i64);
        let tg = p.layout_no_wrap(done_at.format("%H:%M").to_string(), bold(16.0), WHITE);
        let tw = tg.size().x;
        p.galley(pos2(x, r.top() + 112.0 - tg.size().y / 2.0), tg, WHITE);
        p.text(pos2(x + tw + 18.0, r.top() + 112.0), Align2::LEFT_CENTER, t("until done"), bold(15.0), PURPLE);
        let frac = 1.0 - (self.left / self.mode_len(self.mode)).clamp(0.0, 1.0) as f32;
        gradient_bar(&p, Rect::from_min_size(pos2(x, r.top() + 135.0), vec2(166.0, 5.0)), frac, PURPLE, Color32::from_rgb(99, 102, 241));
        p.line_segment([pos2(r.left() + 250.0, r.top() + 46.0), pos2(r.left() + 250.0, r.top() + 158.0)], Stroke::new(1.0_f32, Color32::from_gray(40)));

        let rx = r.left() + 268.0;
        p.text(pos2(rx, r.top() + 52.0), Align2::LEFT_CENTER, t("This round"), reg(13.0), DIM);
        let full = self.left >= self.mode_len(self.mode);
        let text = if self.running { t("Pause") } else if full { t("Start") } else { t("Resume") };
        if pill_button(ui, Rect::from_min_size(pos2(r.left() + 354.0, r.top() + 40.0), vec2(78.0, 26.0)), text, BTN, "pp") {
            self.running = !self.running;
            self.last_tick = Instant::now();
        }
        // ↺ starts this part over; on a part not started yet, it starts the 4 rounds over.
        let untouched = !self.running && full;
        let all = untouched && (self.rounds % LONG_BREAK_EVERY != 0 || self.mode != Mode::Focus);
        let name = if all { t("Start the 4 rounds over") } else { t("Start this part over") };
        if icon_button(ui, pos2(r.left() + 450.0, r.top() + 53.0), 11.0, "reset", name) {
            self.running = false;
            if all {
                self.rounds = 0;
                self.mode = Mode::Focus;
            }
            self.left = self.mode_len(self.mode);
        }
        if icon_button(ui, pos2(r.left() + 478.0, r.top() + 53.0), 11.0, "skip", t("Skip to the next part")) {
            self.next_part(true);
        }
        // Round track: 4 focus rounds then the long break.
        let y = r.top() + 96.0;
        let seg = 34.0;
        let cur = self.rounds % LONG_BREAK_EVERY;
        for k in 0..=LONG_BREAK_EVERY {
            let sx = rx + k as f32 * (seg + 12.0);
            let done = k < cur || (k == LONG_BREAK_EVERY && false);
            let active = (k == cur && self.mode == Mode::Focus) || (k == LONG_BREAK_EVERY && self.mode == Mode::Long);
            let w = if k == LONG_BREAK_EVERY { 22.0 } else { seg - 4.0 };
            let col = if done { PURPLE } else { Color32::from_gray(38) };
            p.rect_filled(Rect::from_min_size(pos2(sx, y - 3.0), vec2(w, 6.0)), 3.0, col);
            if active {
                let fx = sx + w * frac;
                p.rect_filled(Rect::from_min_size(pos2(sx, y - 3.0), vec2(w * frac, 6.0)), 3.0, PURPLE);
                p.circle_filled(pos2(fx, y), 7.0, Color32::from_rgba_unmultiplied(139, 92, 246, 70));
                p.circle_filled(pos2(fx, y), 4.0, WHITE);
            }
            if k < LONG_BREAK_EVERY {
                p.circle_filled(pos2(sx + seg + 2.0, y), 3.0, if k < cur { Color32::from_rgb(45, 212, 191) } else { Color32::from_gray(38) });
            }
            let lbl = if k == LONG_BREAK_EVERY { t("Long").to_string() } else { (k + 1).to_string() };
            let lc = if active { WHITE } else { DIM };
            p.text(pos2(sx + w / 2.0, y + 22.0), Align2::CENTER_CENTER, lbl, bold(11.0), lc);
        }
        let total = LONG_BREAK_EVERY * self.s.focus + (LONG_BREAK_EVERY - 1) * self.s.short_break + self.s.long_break;
        p.text(pos2(rx, r.top() + 143.0), Align2::LEFT_CENTER, tr("{} · {} sessions", &[&dur_text(total), &LONG_BREAK_EVERY]), reg(12.0), DIM);
    }

    fn page_water(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = ui.painter().clone();
        let x = r.left() + 58.0;
        let liters = self.drank as f32 / 1000.0;
        let big = p.layout_no_wrap(format!("{liters:.1}"), bold(36.0), WHITE);
        let bw = big.size().x;
        p.galley(pos2(x, r.top() + 64.0 - big.size().y / 2.0), big, WHITE);
        p.text(pos2(x + bw + 6.0, r.top() + 72.0), Align2::LEFT_CENTER, format!("/ {:.1} L", self.s.goal as f32 / 1000.0), reg(15.0), DIM);
        if toggle(ui, pos2(r.left() + 210.0, r.top() + 50.0), self.s.water) {
            self.s.water = !self.s.water;
            self.save_water();
        }
        let pct = (self.drank * 100 / self.s.goal.max(1)).min(999);
        let status = if self.drank >= self.s.goal {
            t("Goal reached").to_string()
        } else if !self.s.water {
            t("Reminders off").to_string()
        } else {
            self.next_water_slot().map(|at| tr("next {}", &[&hm(at.ceil() as u32)])).unwrap_or_else(|| t("of your goal").into())
        };
        let a = p.layout_no_wrap(t("Today · ").into(), reg(14.0), DIM);
        let b = p.layout_no_wrap(format!("{pct}%"), bold(14.0), BLUE);
        let (aw, bwid) = (a.size().x, b.size().x);
        let ty = r.top() + 98.0;
        p.galley(pos2(x, ty - a.size().y / 2.0), a, DIM);
        p.galley(pos2(x + aw, ty - b.size().y / 2.0), b, BLUE);
        p.text(pos2(x + aw + bwid, ty), Align2::LEFT_CENTER, format!(" · {status}"), reg(14.0), DIM);
        gradient_bar(&p, Rect::from_min_size(pos2(x, r.top() + 112.0), vec2(196.0, 6.0)), self.drank as f32 / self.s.goal.max(1) as f32, Color32::from_rgb(56, 189, 248), BLUE);
        let each = self.s.water_plan().each;
        if pill_button(ui, Rect::from_min_size(pos2(x, r.top() + 132.0), vec2(86.0, 32.0)), t("Drink"), BLUE_DARK, "drink") {
            self.drink();
        }
        let set_label = if self.water_settings { t("Done") } else { t("Settings") };
        if pill_button(ui, Rect::from_min_size(pos2(x + 90.0, r.top() + 132.0), vec2(104.0, 32.0)), set_label, BTN, "wset") {
            self.water_settings = !self.water_settings;
        }
        p.line_segment([pos2(r.left() + 272.0, r.top() + 44.0), pos2(r.left() + 272.0, r.top() + 166.0)], Stroke::new(1.0_f32, Color32::from_gray(40)));
        let right = Rect::from_min_max(pos2(r.left() + 286.0, r.top() + 44.0), pos2(r.right() - 52.0, r.top() + 170.0));
        if self.water_settings {
            self.water_settings_ui(ui, right);
        } else {
            self.water_chart(&p, right);
        }
        let _ = each;
    }

    /// Hourly bars over the active hours, scaled like the original (0x140048333):
    /// top = ceil(max(2 x goal / active hours, biggest hour) / 100) x 100.
    fn water_chart(&self, p: &egui::Painter, r: Rect) {
        let h0 = (self.s.from / 60) as usize;
        let h1 = (self.s.until.div_ceil(60) as usize).clamp(h0 + 1, 24);
        let span_h = ((self.s.until - self.s.from) as f32 / 60.0).max(1.0);
        let biggest = self.hours[h0..h1].iter().copied().max().unwrap_or(0) as f32;
        let top = ((2.0 * self.s.goal as f32 / span_h).max(biggest) / 100.0).ceil().max(1.0) * 100.0;
        let plot = Rect::from_min_max(pos2(r.left() + 6.0, r.top() + 10.0), pos2(r.right() - 44.0, r.bottom() - 26.0));
        for (k, v) in [(0.0, "0 ml".to_string()), (0.5, format!("{}", top / 2.0)), (1.0, format!("{top}"))] {
            let y = plot.bottom() - k * plot.height();
            dashed(p, plot.left(), plot.right(), y);
            p.text(pos2(plot.right() + 8.0, y), Align2::LEFT_CENTER, v, reg(11.0), DIM);
        }
        p.line_segment([pos2(plot.left(), plot.bottom()), pos2(plot.right(), plot.bottom())], Stroke::new(1.0_f32, BLUE_DARK));
        let n = h1 - h0;
        let slot = plot.width() / n as f32;
        let bw = (slot * 0.5).min(9.0);
        for (k, hour) in (h0..h1).enumerate() {
            let cx = plot.left() + slot * (k as f32 + 0.5);
            let track = Rect::from_center_size(pos2(cx, plot.center().y), vec2(bw, plot.height()));
            p.rect_filled(track, bw / 2.0, Color32::from_gray(22));
            let v = self.hours[hour] as f32 / top;
            if v > 0.0 {
                let bar = Rect::from_min_max(pos2(cx - bw / 2.0, plot.bottom() - v.min(1.0) * plot.height()), pos2(cx + bw / 2.0, plot.bottom()));
                vertical_gradient(p, bar, bw / 2.0, Color32::from_rgb(56, 152, 255), Color32::from_rgb(29, 78, 216));
            }
            if (hour - h0) % 3 == 0 {
                p.text(pos2(cx, r.bottom() - 10.0), Align2::CENTER_CENTER, format!("{hour:02}"), reg(11.0), DIM);
            }
        }
    }

    fn water_settings_ui(&mut self, ui: &mut egui::Ui, r: Rect) {
        let before = self.s.clone();
        let p = ui.painter().clone();
        let lx = r.left();
        let vx = r.left() + 150.0;
        let rows = [(t("Goal"), 0.0), (t("Bottle"), 30.0), (t("Hours"), 60.0)];
        for (name, dy) in rows {
            p.text(pos2(lx, r.top() + 14.0 + dy), Align2::LEFT_CENTER, name, reg(13.0), DIM);
        }
        let y0 = r.top() + 14.0;
        // Goal: 1.5 / 2.0 / 2.5 / 3.0 L.
        let gi = GOALS.iter().position(|&g| g == self.s.goal).unwrap_or(0);
        let mut gnext = gi;
        if small_step(ui, pos2(vx - 70.0, y0), "-", "g-") { gnext = gi.saturating_sub(1); }
        p.text(pos2(vx - 30.0, y0), Align2::CENTER_CENTER, format!("{:.1} L", self.s.goal as f32 / 1000.0), bold(13.0), WHITE);
        if small_step(ui, pos2(vx + 10.0, y0), "+", "g+") { gnext = (gi + 1).min(GOALS.len() - 1); }
        self.s.goal = GOALS[gnext];
        let y1 = y0 + 30.0;
        if small_step(ui, pos2(vx - 70.0, y1), "-", "b-") { self.s.bottle = self.s.bottle.saturating_sub(BOTTLE_STEP).max(BOTTLE_MIN); }
        p.text(pos2(vx - 30.0, y1), Align2::CENTER_CENTER, format!("{} ml", self.s.bottle), bold(13.0), WHITE);
        if small_step(ui, pos2(vx + 10.0, y1), "+", "b+") { self.s.bottle = (self.s.bottle + BOTTLE_STEP).min(BOTTLE_MAX); }
        let y2 = y0 + 60.0;
        let (mut from, mut until) = (self.s.from, self.s.until);
        if small_step(ui, pos2(lx + 50.0, y2), "-", "f-") { from = from.saturating_sub(HOURS_STEP); }
        p.text(pos2(lx + 82.0, y2), Align2::CENTER_CENTER, hm(from as u32), bold(13.0), WHITE);
        if small_step(ui, pos2(lx + 114.0, y2), "+", "f+") { from = (from + HOURS_STEP).min(FROM_MAX); }
        p.text(pos2(lx + 132.0, y2), Align2::CENTER_CENTER, "to", reg(12.0), DIM);
        if small_step(ui, pos2(lx + 150.0, y2), "-", "u-") { until = until.saturating_sub(HOURS_STEP); }
        p.text(pos2(lx + 182.0, y2), Align2::CENTER_CENTER, hm(until as u32), bold(13.0), WHITE);
        if small_step(ui, pos2(lx + 214.0, y2), "+", "u+") { until = (until + HOURS_STEP).min(UNTIL_MAX); }
        self.s.from = from;
        // "until" always stays at least an hour after "from" (original rule).
        self.s.until = until.max(from + MIN_SPAN).min(UNTIL_MAX);
        let plan = self.s.water_plan();
        p.text(pos2(lx, y0 + 90.0), Align2::LEFT_CENTER, tr("{} reminders · {} refills · {} ml each", &[&plan.reminders, &plan.refills, &plan.each]), reg(12.0), DIM);
        let armed = self.reset_armed.is_some_and(|t| t.elapsed().as_secs_f32() < RESET_CONFIRM_SECS);
        if pill_button(ui, Rect::from_min_size(pos2(r.right() - 64.0, y0 + 78.0), vec2(64.0, 24.0)), if armed { t("Sure?") } else { t("Reset") }, if armed { RED } else { BTN }, "reset") {
            if armed {
                // Reset (0x140053982): today's total, the shown reminders, the hourly amounts, a full bottle.
                self.drank = 0;
                self.fired = 0;
                self.hours = [0; 24];
                self.in_bottle = self.s.bottle;
                self.reset_armed = None;
                self.save_water();
            } else {
                self.reset_armed = Some(Instant::now());
            }
        }
        self.in_bottle = self.in_bottle.min(self.s.bottle);
        if self.s != before {
            // Any water setting change clears today's shown reminders (0x140053c97).
            self.fired = 0;
            self.save_water();
        }
    }

    fn page_shelf(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = ui.painter().clone();
        // Files / Clipboard switch at the top right.
        let sw = Rect::from_min_size(pos2(r.right() - 112.0, r.top() + 13.0), vec2(82.0, 22.0));
        p.rect_filled(sw, 11.0, BTN);
        for (k, v) in [ShelfView::Files, ShelfView::Clipboard].into_iter().enumerate() {
            let cell = Rect::from_min_size(pos2(sw.left() + 2.0 + k as f32 * 40.0, sw.top() + 2.0), vec2(38.0, 18.0));
            if self.shelf_view == v {
                p.rect_filled(cell, 9.0, Color32::from_rgb(99, 102, 241));
            }
            if k == 0 { folder_icon(&p, cell.center(), WHITE) } else { clip_icon(&p, cell.center(), WHITE) }
            if ui.interact(cell, ui.id().with(("sv", k)), Sense::click()).clicked() {
                self.shelf_view = v;
            }
        }
        let area = Rect::from_min_max(pos2(r.left() + 56.0, r.top() + 46.0), pos2(r.right() - 56.0, r.top() + 166.0));
        let files_over = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
        if self.shelf_view == ShelfView::Files || files_over {
            if !self.shelf.is_empty()
                && pill_button(ui, Rect::from_min_size(pos2(r.left() + 80.0, r.top() + 13.0), vec2(50.0, 22.0)), t("Clear"), BTN, "clear")
            {
                self.shelf.clear();
                self.save_shelf();
            }
            if self.shelf.is_empty() || files_over {
                let col = if files_over { BLUE } else { Color32::from_gray(60) };
                dashed_rect(&p, area.shrink2(vec2(4.0, 6.0)), col);
                let t = if files_over { t("Release to add") } else { t("Drop files here") };
                p.text(area.center() - vec2(0.0, 8.0), Align2::CENTER_CENTER, t, bold(15.0), WHITE);
                p.text(area.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, i18n::t("Drag out anytime · click to open"), reg(12.0), DIM);
                return;
            }
            let (mut remove, mut drag) = (None, None);
            for (i, path) in self.shelf.iter().enumerate().take(5) {
                let cx = area.left() + 44.0 + i as f32 * 88.0;
                let icon = Rect::from_center_size(pos2(cx, area.top() + 34.0), vec2(56.0, 56.0));
                file_icon(&p, icon, path);
                let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                p.text(pos2(cx, area.top() + 76.0), Align2::CENTER_CENTER, ellipsize(&name, 12), bold(12.0), WHITE);
                let size = std::fs::metadata(path).map(|m| human_size(m.len())).unwrap_or_default();
                p.text(pos2(cx, area.top() + 92.0), Align2::CENTER_CENTER, size, reg(11.0), DIM);
                let resp = ui.interact(icon.union(Rect::from_center_size(pos2(cx, area.top() + 80.0), vec2(80.0, 30.0))), ui.id().with(("file", i)), Sense::click_and_drag());
                if resp.clicked() {
                    let _ = std::process::Command::new("explorer").arg(path).spawn();
                }
                if resp.drag_started() && path.exists() {
                    drag = Some(path.clone());
                }
                if resp.secondary_clicked() {
                    remove = Some(i);
                }
                resp.on_hover_text(t("Click to open · drag out · right-click to remove"));
            }
            if let Some(path) = drag {
                dragout::files(&[path]);
            }
            if let Some(i) = remove {
                self.shelf.remove(i);
                self.save_shelf();
            }
        } else {
            if !self.s.clipboard {
                p.text(area.center() - vec2(0.0, 8.0), Align2::CENTER_CENTER, t("Clipboard history is off"), bold(15.0), WHITE);
                p.text(area.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, t("Turn it on in Settings (right-click the notch)"), reg(12.0), DIM);
                return;
            }
            if self.clips.is_empty() {
                p.text(area.center() - vec2(0.0, 8.0), Align2::CENTER_CENTER, t("Copy some text"), bold(15.0), WHITE);
                p.text(area.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, t("It will wait here, ready to paste again"), reg(12.0), DIM);
                return;
            }
            let (mut copy, mut drag) = (None, None);
            let now = Utc::now().timestamp();
            for (i, (ts, text)) in self.clips.iter().enumerate().take(4) {
                let card = Rect::from_min_size(pos2(area.left() + i as f32 * 106.0, area.top() - 2.0), vec2(98.0, 104.0));
                let resp = ui.interact(card, ui.id().with(("clip", i)), Sense::click_and_drag());
                if resp.drag_started() {
                    drag = Some(text.clone());
                }
                p.rect_filled(card, 12.0, if resp.hovered() { Color32::from_gray(30) } else { CARD });
                if i == 0 {
                    p.rect_stroke(card, 12.0, Stroke::new(1.5_f32, Color32::from_rgb(99, 102, 241)), egui::StrokeKind::Inside);
                }
                let just = self.copied.as_ref().is_some_and(|(c, at)| c == text && at.elapsed().as_secs() < 2);
                let body = if just { t("Copied").to_string() } else { text.clone() };
                let g = p.layout(body, bold(12.0), WHITE, card.width() - 22.0);
                let rows: Vec<_> = g.rows.iter().take(3).collect();
                let clipped = rows.len() < g.rows.len();
                let shown: String = clip_text(text, &g, 3, clipped, just);
                let g2 = p.layout(shown, bold(12.0), WHITE, card.width() - 22.0);
                p.galley(card.min + vec2(11.0, 12.0), g2, WHITE);
                p.text(pos2(card.left() + 11.0, card.bottom() - 16.0), Align2::LEFT_CENTER, ago(now - ts), reg(11.0), DIM);
                if resp.clicked() {
                    copy = Some(text.clone());
                }
            }
            p.text(pos2(area.left() + 4.0, r.top() + 168.0), Align2::LEFT_CENTER, tr("{} items · click to copy · drag into a chat", &[&self.clips.len()]), reg(12.0), DIM);
            if let Some(t) = drag {
                dragout::text(&t);
            }
            if let Some(t) = copy {
                if let Some(c) = self.clipboard.as_mut() {
                    let _ = c.set_text(t.clone());
                }
                self.last_clip = t.clone();
                self.copied = Some((t, Instant::now()));
            }
        }
    }

    fn page_todo(&mut self, ui: &mut egui::Ui, r: Rect) {
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
        self.scroll_zone = Some(list); // the wheel scrolls this list, not the pages
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
                    let when = e.start.map_or_else(|| t("All day").to_string(), |s| hm(s));
                    pp.text(pos2(row.right() - 10.0, row.center().y), Align2::RIGHT_CENTER, when, reg(12.0), DIM);
                }
                for (i, t) in self.tasks.iter().enumerate() {
                    let (row, resp) = ui.allocate_exact_size(vec2(list.width() - 12.0, 34.0), Sense::click());
                    let is_done = t.done_day.is_some();
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
            t.done_day = if t.done_day.is_some() { None } else { Some(today) };
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

    fn page_prayer(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = ui.painter().clone();
        let x = r.left() + 60.0;
        if !self.s.prayer {
            p.text(pos2(x, r.top() + 70.0), Align2::LEFT_CENTER, t("Prayer times"), bold(26.0), WHITE);
            p.text(pos2(x, r.top() + 100.0), Align2::LEFT_CENTER, t("are off"), reg(15.0), DIM);
            if pill_button(ui, Rect::from_min_size(pos2(x, r.top() + 122.0), vec2(150.0, 30.0)), t("Turn on"), BLUE_DARK, "pon") {
                self.s.prayer = true;
                self.s.save();
            }
            return;
        }
        let today = self.today();
        let city = self.city_name();
        let city = city.as_str();
        p.text(pos2(r.left() + 100.0, r.top() + 24.0), Align2::LEFT_CENTER, city, reg(13.0), DIM);
        let t = self.prayer_times(today);
        let m = self.now_min();
        let now_f = m as f32 + self.now.second() as f32 / 60.0;
        let next = t.iter().position(|&x| x > m);
        let (name, at, label) = match next {
            Some(i) => (prayer::NAMES[i], t[i], i18n::t("Next")),
            None => (prayer::NAMES[0], self.prayer_times(today + Duration::days(1))[0], i18n::t("Next, tomorrow")),
        };
        p.text(pos2(x, r.top() + 46.0), Align2::LEFT_CENTER, label, reg(13.0), DIM);
        p.text(pos2(x - 1.0, r.top() + 76.0), Align2::LEFT_CENTER, name, bold(32.0), WHITE);
        let until = (at as i64 - m as i64).rem_euclid(1440);
        let ag = p.layout_no_wrap(hm(at), bold(17.0), WHITE);
        let aw = ag.size().x;
        p.galley(pos2(x, r.top() + 112.0 - ag.size().y / 2.0), ag, WHITE);
        p.text(pos2(x + aw + 18.0, r.top() + 112.0), Align2::LEFT_CENTER, tr("in {}", &[&dur_text(until as u32)]), bold(15.0), BLUE);
        let prev = next.and_then(|i| i.checked_sub(1)).map(|i| t[i]).unwrap_or(t[4]);
        let gap = (at as i64 - prev as i64).rem_euclid(1440).max(1) as f32;
        let frac = 1.0 - until as f32 / gap;
        gradient_bar(&p, Rect::from_min_size(pos2(x, r.top() + 136.0), vec2(166.0, 5.0)), frac, BLUE, PURPLE);
        p.line_segment([pos2(r.left() + 250.0, r.top() + 46.0), pos2(r.left() + 250.0, r.top() + 162.0)], Stroke::new(1.0_f32, Color32::from_gray(40)));

        // Day arc: the sun travels from Subuh to Isya, past part in blue.
        let ax0 = r.left() + 276.0;
        let ax1 = r.right() - 52.0;
        let base = r.top() + 124.0;
        let height = 62.0;
        let (t0, t1) = (t[0] as f32, t[4] as f32);
        let xy = |tm: f32| {
            let u = ((tm - t0) / (t1 - t0)).clamp(0.0, 1.0);
            pos2(ax0 + u * (ax1 - ax0), base - height * (PI * (u * 0.82 + 0.02)).sin())
        };
        p.line_segment([pos2(ax0, base + 2.0), pos2(ax1, base + 2.0)], Stroke::new(1.0_f32, Color32::from_gray(45)));
        let steps = 60;
        let pts: Vec<Pos2> = (0..=steps).map(|k| xy(t0 + (t1 - t0) * k as f32 / steps as f32)).collect();
        let cut = (((now_f - t0) / (t1 - t0)).clamp(0.0, 1.0) * steps as f32) as usize;
        p.add(Shape::line(pts[cut..].to_vec(), Stroke::new(1.5_f32, Color32::from_gray(70))));
        if cut > 0 {
            p.add(Shape::line(pts[..=cut].to_vec(), Stroke::new(2.5_f32, BLUE)));
        }
        for (i, &pt) in t.iter().enumerate() {
            let pos = xy(pt as f32);
            p.circle_filled(pos, 3.0, if (pt as f32) <= now_f { Color32::from_rgb(147, 197, 253) } else { Color32::from_gray(150) });
            let is_next = next == Some(i);
            let col = if is_next { WHITE } else { DIM };
            let f = if is_next { bold(11.0) } else { reg(11.0) };
            p.text(pos2(pos.x, base + 16.0), Align2::CENTER_CENTER, prayer::NAMES[i], f.clone(), col);
            p.text(pos2(pos.x, base + 30.0), Align2::CENTER_CENTER, hm(pt), reg(10.0), DIM);
        }
        if now_f >= t0 && now_f <= t1 {
            let sun = xy(now_f);
            for (rad, a) in [(14.0, 30), (10.0, 70), (6.5, 255)] {
                p.circle_filled(sun, rad, Color32::from_rgba_unmultiplied(253, 230, 160, a));
            }
        }
    }

    fn page_widgets(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = ui.painter().clone();
        let a = Rect::from_min_size(pos2(r.left() + 56.0, r.top() + 42.0), vec2(212.0, 126.0));
        let b = Rect::from_min_size(pos2(r.left() + 280.0, r.top() + 42.0), vec2(212.0, 126.0));
        for (rect, kind) in [(a, self.s.widget_left), (b, self.s.widget_right)] {
            p.rect_filled(rect, 14.0, CARD);
            self.widget(&p, rect, kind);
        }
    }

    /// Recent notifications on the left, the week (the "This week" widget) on the right.
    fn page_today(&mut self, ui: &mut egui::Ui, r: Rect) {
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
            for (k, (ts, (app, title, body, _))) in self.recent.iter().take(3).enumerate() {
                let y = a.top() + 40.0 + k as f32 * 30.0;
                p.text(pos2(a.left() + 14.0, y - 7.0), Align2::LEFT_CENTER, ellipsize(app, 20), reg(10.0), DIM);
                p.text(pos2(a.right() - 14.0, y - 7.0), Align2::RIGHT_CENTER, ago(now - ts), reg(10.0), DIM);
                let line = if self.s.notification_preview && !body.is_empty() { format!("{title}: {body}") } else { title.clone() };
                p.text(pos2(a.left() + 14.0, y + 7.0), Align2::LEFT_CENTER, ellipsize(&line, 26), bold(12.0), WHITE);
            }
        }
        p.rect_filled(b, 14.0, CARD);
        self.widget(&p, b, 0);
    }

    fn widget(&self, p: &egui::Painter, r: Rect, kind: usize) {
        let title = |t: &str| p.text(r.min + vec2(14.0, 15.0), Align2::LEFT_CENTER, t, bold(11.0), DIM);
        let today = self.today();
        match kind {
            0 => {
                title(t("THIS WEEK"));
                let ws = if self.s.week_monday { Weekday::Mon } else { Weekday::Sun };
                let start = today.week(ws).first_day();
                for k in 0..7 {
                    let d = start + Duration::days(k);
                    let cx = r.left() + 23.0 + k as f32 * 27.5;
                    let letter = i18n::day_letter(d.weekday());
                    p.text(pos2(cx, r.top() + 36.0), Align2::CENTER_CENTER, letter, bold(10.0), DIM);
                    let col = if d < today { DIM } else { WHITE };
                    if d == today {
                        p.circle_filled(pos2(cx, r.top() + 58.0), 11.0, BLUE);
                    }
                    p.text(pos2(cx, r.top() + 58.0), Align2::CENTER_CENTER, d.day().to_string(), bold(13.0), col);
                }
                let open = self.tasks.iter().filter(|t| t.done_day.is_none()).count();
                p.circle_filled(pos2(r.left() + 18.0, r.top() + 95.0), 4.0, GREEN);
                p.text(pos2(r.left() + 28.0, r.top() + 95.0), Align2::LEFT_CENTER, tr("{} tasks today", &[&open]), bold(12.0), WHITE);
                let next = self.tasks.iter().filter(|t| t.done_day.is_none() && t.start.is_some_and(|s| s as u32 >= self.now_min())).min_by_key(|t| t.start);
                let line = next.map(|task| tr("Next: {}, {}", &[&ellipsize(&task.title, 16), &hm(task.start.unwrap() as u32)])).unwrap_or_else(|| t("Nothing scheduled").into());
                p.text(pos2(r.left() + 28.0, r.top() + 111.0), Align2::LEFT_CENTER, line, reg(11.0), DIM);
            }
            1 => {
                title(t("BATTERY"));
                let c = pos2(r.left() + 50.0, r.top() + 72.0);
                match self.battery {
                    Some((pct, ac)) => {
                        let col = if pct <= 20 && !ac { RED } else { GREEN };
                        ring(p, c, 28.0, 6.0, pct as f32 / 100.0, col);
                        if ac { bolt(p, c, 11.0, WHITE) } else { p.text(c, Align2::CENTER_CENTER, format!("{pct}"), bold(14.0), WHITE); }
                        p.text(pos2(r.left() + 92.0, r.top() + 50.0), Align2::LEFT_CENTER, format!("{pct}%"), bold(24.0), WHITE);
                        p.text(pos2(r.left() + 92.0, r.top() + 76.0), Align2::LEFT_CENTER, if ac { t("Charging") } else { t("On battery") }, bold(12.0), if ac { GREEN } else { WHITE });
                        p.text(pos2(r.left() + 92.0, r.top() + 93.0), Align2::LEFT_CENTER, if ac { t("Plugged in") } else { t("Unplugged") }, reg(11.0), DIM);
                    }
                    None => {
                        ring(p, c, 28.0, 6.0, 1.0, GREEN);
                        bolt(p, c, 11.0, WHITE);
                        p.text(pos2(r.left() + 92.0, r.top() + 62.0), Align2::LEFT_CENTER, t("Plugged in"), bold(14.0), WHITE);
                        p.text(pos2(r.left() + 92.0, r.top() + 82.0), Align2::LEFT_CENTER, t("No battery"), reg(11.0), DIM);
                    }
                }
            }
            2 => {
                title(t("NEXT PRAYER"));
                if self.s.prayer {
                    let t = self.prayer_times(today);
                    let m = self.now_min();
                    let (name, at) = match t.iter().position(|&x| x > m) {
                        Some(i) => (prayer::NAMES[i], t[i]),
                        None => (prayer::NAMES[0], self.prayer_times(today + Duration::days(1))[0]),
                    };
                    p.text(pos2(r.left() + 14.0, r.top() + 56.0), Align2::LEFT_CENTER, name, bold(26.0), WHITE);
                    p.text(pos2(r.left() + 14.0, r.top() + 88.0), Align2::LEFT_CENTER, hm(at), bold(16.0), BLUE);
                    p.text(pos2(r.left() + 14.0, r.top() + 108.0), Align2::LEFT_CENTER, self.city_name(), reg(11.0), DIM);
                } else {
                    p.text(pos2(r.left() + 14.0, r.top() + 60.0), Align2::LEFT_CENTER, t("Prayer times"), bold(16.0), WHITE);
                    p.text(pos2(r.left() + 14.0, r.top() + 80.0), Align2::LEFT_CENTER, t("are off"), reg(12.0), DIM);
                }
            }
            3 => {
                title(t("FOCUS TODAY"));
                p.text(pos2(r.left() + 14.0, r.top() + 58.0), Align2::LEFT_CENTER, dur_text(self.focus_today()), bold(24.0), WHITE);
                let days: Vec<u32> = (0..7).rev().map(|i| self.focus_log.get(&(today - Duration::days(i))).copied().unwrap_or(0)).collect();
                let max = days.iter().copied().max().unwrap_or(0).max(1) as f32;
                for (k, v) in days.iter().enumerate() {
                    let h = 30.0 * *v as f32 / max;
                    let x = r.left() + 16.0 + k as f32 * 26.0;
                    p.rect_filled(Rect::from_min_max(pos2(x, r.bottom() - 16.0 - h.max(2.0)), pos2(x + 14.0, r.bottom() - 16.0)), 3.0, if k == 6 { PURPLE } else { Color32::from_gray(50) });
                }
            }
            4 => {
                title(t("WORLD CLOCK"));
                for (k, idx) in [self.s.widget_city1, self.s.widget_city2].into_iter().enumerate() {
                    let (name, _, _) = WORLD[idx.min(WORLD.len() - 1)];
                    let y = r.top() + 50.0 + k as f32 * 40.0;
                    p.text(pos2(r.left() + 14.0, y), Align2::LEFT_CENTER, name, reg(13.0), DIM);
                    p.text(pos2(r.right() - 14.0, y), Align2::RIGHT_CENTER, world_time(idx).format("%H:%M").to_string(), bold(22.0), WHITE);
                }
            }
            5 => {
                title(t("MOON"));
                let (phase, lit) = moon(Utc::now().naive_utc());
                let c = pos2(r.left() + 46.0, r.top() + 72.0);
                p.circle_filled(c, 24.0, Color32::from_gray(45));
                p.circle_filled(c + vec2(48.0 * (1.0 - lit as f32) * if phase < 4 { -1.0 } else { 1.0 }, 0.0), 24.0, CARD);
                p.circle_stroke(c, 24.0, Stroke::new(1.0_f32, Color32::from_gray(70)));
                p.text(pos2(r.left() + 84.0, r.top() + 62.0), Align2::LEFT_CENTER, t(MOON[phase]), bold(13.0), WHITE);
                p.text(pos2(r.left() + 84.0, r.top() + 82.0), Align2::LEFT_CENTER, tr("{}% lit", &[&format!("{:.0}", lit * 100.0)]), reg(12.0), DIM);
            }
            6 => {
                title(t("COUNTDOWN"));
                let ev = self.s.widget_event.min(EVENTS.len() - 1);
                let (big, small) = match next_event(today, ev) {
                    Some(d) if d == today => (t("Today").to_string(), t(EVENTS[ev]).to_string()),
                    Some(d) => ((d - today).num_days().to_string(), tr("days to {}", &[&t(EVENTS[ev])])),
                    None => ("—".into(), t(EVENTS[ev]).to_string()),
                };
                p.text(pos2(r.left() + 14.0, r.top() + 60.0), Align2::LEFT_CENTER, big, bold(32.0), WHITE);
                p.text(pos2(r.left() + 14.0, r.top() + 96.0), Align2::LEFT_CENTER, small, reg(12.0), DIM);
            }
            WEATHER_WIDGET => self.weather_widget(p, r),
            _ => {
                title(t("WATER"));
                let c = pos2(r.left() + 50.0, r.top() + 72.0);
                ring(p, c, 28.0, 6.0, self.drank as f32 / self.s.goal.max(1) as f32, BLUE);
                drop_icon(p, c, 10.0, BLUE);
                p.text(pos2(r.left() + 92.0, r.top() + 60.0), Align2::LEFT_CENTER, format!("{:.1} L", self.drank as f32 / 1000.0), bold(22.0), WHITE);
                p.text(pos2(r.left() + 92.0, r.top() + 84.0), Align2::LEFT_CENTER, tr("of {} L", &[&format!("{:.1}", self.s.goal as f32 / 1000.0)]), reg(12.0), DIM);
            }
        }
    }

    /// The date, the weather and a soft light in the sky's colour (MET Norway's next-hour symbol).
    fn weather_widget(&self, p: &egui::Painter, r: Rect) {
        let w = self.weather.lock().unwrap().clone();
        let (sky, label) = w.as_ref().map_or((Color32::from_gray(70), ""), |w| sky(&w.symbol));
        // The light: soft rings fading out from the top right, kept inside the card.
        let glow = p.with_clip_rect(r.intersect(p.clip_rect()));
        let c = r.right_top() + vec2(-30.0, 26.0);
        for k in 0..32 {
            let f = k as f32 / 32.0;
            glow.circle_filled(c, 130.0 * (1.0 - f) + 6.0, sky.gamma_multiply(0.025 + 0.02 * f));
        }
        p.text(r.min + vec2(14.0, 15.0), Align2::LEFT_CENTER, t("WEATHER"), bold(11.0), DIM);
        p.text(pos2(r.right() - 14.0, r.top() + 15.0), Align2::RIGHT_CENTER, i18n::date(self.now).rsplit_once(' ').map_or(String::new(), |d| d.0.to_string()), reg(11.0), DIM);
        let place = if self.s.weather_city == prayer::here_index() { t("My location").to_string() } else { prayer::place_name(self.s.weather_city).1.to_string() };
        match w {
            Some(w) => {
                weather_icon(p, pos2(r.left() + 40.0, r.top() + 66.0), &w.symbol, sky);
                p.text(pos2(r.left() + 78.0, r.top() + 58.0), Align2::LEFT_CENTER, format!("{:.0}°", w.temp), bold(32.0), WHITE);
                p.text(pos2(r.left() + 78.0, r.top() + 86.0), Align2::LEFT_CENTER, t(label), reg(13.0), WHITE);
            }
            None => {
                let why = if self.s.weather_city == prayer::here_index() && (self.s.prayer_lat, self.s.prayer_lon) == (0.0, 0.0) { t("Find my location in Settings") } else { t("Getting the weather…") };
                p.text(pos2(r.left() + 14.0, r.top() + 66.0), Align2::LEFT_CENTER, why, reg(13.0), DIM);
            }
        }
        p.text(pos2(r.left() + 14.0, r.bottom() - 14.0), Align2::LEFT_CENTER, ellipsize(&place, 24), reg(11.0), DIM);
        p.text(pos2(r.right() - 14.0, r.bottom() - 14.0), Align2::RIGHT_CENTER, "MET Norway", reg(9.0), FAINT);
    }

    /// Ask Nusa: a sum ending in "=" answers as you type; questions go to Gemini (needs a key).
    fn page_ask(&mut self, ui: &mut egui::Ui, r: Rect) {
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
        }
        // Gemini's answer to the latest question.
        if self.ask_waiting {
            let reply = self.ask_reply.lock().unwrap().clone();
            if reply.0 == self.ask_gen {
                self.ask_waiting = false;
                self.ask_answer = reply.1;
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
        }
        if !self.ask_answer.is_empty() {
            if self.ask_answer.starts_with('=') {
                p.text(pos2(r.left() + 30.0, r.top() + 116.0), Align2::LEFT_CENTER, &self.ask_answer, bold(30.0), WHITE);
            } else {
                // Gemini's answers wrap and scroll, and can be selected to copy.
                let area = Rect::from_min_max(pos2(r.left() + 30.0, r.top() + 92.0), pos2(r.right() - 30.0, r.bottom() - 28.0));
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
    fn ask_gemini(&mut self, rule: &str, question: String) {
        if self.gemini_key.trim().is_empty() {
            self.ask_answer = t("Ask Nusa needs a Gemini key — add one in Settings").to_string();
            return;
        }
        self.ask_gen += 1;
        self.ask_waiting = true;
        self.ask_answer = t("Thinking…").to_string();
        let (n, key, rule, out) = (self.ask_gen, self.gemini_key.trim().to_string(), rule.to_string(), self.ask_reply.clone());
        std::thread::spawn(move || {
            let answer = match online::gemini(&key, &rule, &question) {
                Ok(a) => a,
                Err(online::AskError::Key) => t("Gemini turned the key down — check it in Settings").to_string(),
                Err(online::AskError::Busy) => t("Too many questions just now — try again in a minute").to_string(),
                Err(online::AskError::Offline) => t("Couldn't reach Gemini — check the internet").to_string(),
                Err(online::AskError::Empty) => t("Gemini gave no answer to that").to_string(),
            };
            *out.lock().unwrap() = (n, answer);
            tray::wake();
        });
    }

    fn page_settings(&mut self, ui: &mut egui::Ui, r: Rect) {
        let before = self.s.clone();
        let locating = self.locating.clone();
        let (gemini, ical, ical_url) = (&mut self.gemini_key, &mut self.ical_text, self.ical_url.clone());
        let cal = self.calendar.lock().unwrap().clone();
        let inner = Rect::from_min_max(pos2(r.left() + 18.0, r.top() + 44.0), pos2(r.right() - 18.0, r.bottom() - 10.0));
        settings_style(ui);
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(inner), |ui| {
            let s = &mut self.s;
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.set_width(ui.available_width() - 8.0); // room for the scroll bar
                card(ui, "General", |ui| {
                    combo(ui, "Language", &mut s.language, &i18n::NAMES);
                    switch_row(ui, "Start with Windows", &mut s.startup);
                    combo(ui, "Size", &mut s.size, &SIZE_NAMES);
                    switch_row(ui, "Show the date and time — the clock in the notch and on each page", &mut s.show_clock);
                    combo(ui, "Tray icon", &mut s.app_icon, &tray::ICON_NAMES);
                    switch_row(ui, "Minimal notch — shrinks to a thin bar when nothing is going on", &mut s.minimal);
                    switch_row(ui, "Mascot — a little friend that lives in the notch", &mut s.mascot);
                    let mut oc = s.open_click as usize;
                    if combo(ui, "Open the notch on", &mut oc, &["Hover", "Click"]) {
                        s.open_click = oc == 1;
                    }
                    // The shortcuts, and whether another app has taken one.
                    let taken = tray::hotkeys_taken();
                    row(ui, "Shortcuts — Ctrl+Alt+Space asks Nusa, Ctrl+Alt+T copies text from the screen", |ui| {
                        let (text, col) = match taken {
                            None => (String::new(), DIM), // not registered yet
                            Some(0) => (t("Ready").to_string(), GREEN),
                            Some(b) => (tr("Taken: {}", &[&taken_hint(b).body]), RED),
                        };
                        ui.add(egui::Label::new(egui::RichText::new(text).font(reg(12.0)).color(col)).wrap());
                    });
                    combo(ui, "Status ring", &mut s.status_ring, &["Water", "Tasks done today", "No ring"]);
                    switch_row(ui, "Reminder sounds — chimes for Pomodoro, To-do and Water", &mut s.sound);
                });
                card(ui, "Pomodoro", |ui| {
                    number(ui, "Focus", &mut s.focus, 1..=120, " min");
                    number(ui, "Short break", &mut s.short_break, 1..=60, " min");
                    number(ui, "Long break", &mut s.long_break, 1..=60, " min");
                    switch_row(ui, "Go on by itself — the next break or focus starts without a click", &mut s.auto_next);
                });
                card(ui, "To-do reminders", |ui| {
                    switch_row(ui, "When a task starts", &mut s.todo_start);
                    switch_row(ui, "When time is up", &mut s.todo_end);
                    let mut sn = SNOOZES.iter().position(|&l| l == s.snooze).unwrap_or(1);
                    if combo(ui, "\"Later\" waits", &mut sn, &["5 min", "10 min", "15 min"]) {
                        s.snooze = SNOOZES[sn];
                    }
                });
                card(ui, "Prayer times", |ui| {
                    switch_row(ui, "Show prayer times — worked out on this PC, for any city", &mut s.prayer);
                    ui.add_enabled_ui(s.prayer, |ui| {
                        let names: Vec<String> = (0..prayer::place_count())
                            .map(|i| if i == prayer::here_index() { t("My location").into() } else { let (r, c) = prayer::place_name(i); format!("{c} — {r}") })
                            .collect();
                        let refs: Vec<&str> = names.iter().map(|n| n.as_str()).collect();
                        combo(ui, "City", &mut s.prayer_city, &refs);
                        if s.prayer_city == prayer::here_index() {
                            let state = *locating.lock().unwrap();
                            let label = match state {
                                Locating::Busy => t("Finding where you are…").to_string(),
                                Locating::Failed => t("Couldn't find your location — check Windows Settings › Privacy › Location, or pick a city").to_string(),
                                _ if s.prayer_lat == 0.0 && s.prayer_lon == 0.0 => t("Not found yet").to_string(),
                                _ => format!("{:.2}, {:.2}", s.prayer_lat, s.prayer_lon),
                            };
                            row(ui, "Find my location", |ui| {
                                if move_button(ui, "⌖", t("Find my location")) && state != Locating::Busy {
                                    *locating.lock().unwrap() = Locating::Busy;
                                    let out = locating.clone();
                                    std::thread::spawn(move || {
                                        *out.lock().unwrap() = media::locate().map_or(Locating::Failed, |(a, b)| Locating::Got(a, b));
                                        tray::wake();
                                    });
                                }
                                ui.add_space(8.0);
                                ui.label(egui::RichText::new(label).font(reg(12.0)).color(DIM));
                            });
                        }
                        let mut methods: Vec<&str> = vec![t("Usual for the city")];
                        methods.extend(prayer::METHODS.iter().map(|m| m.name));
                        combo(ui, "Method", &mut s.prayer_method, &methods);
                        switch_row(ui, "Remind me at prayer time", &mut s.prayer_reminder);
                        ui.add_enabled_ui(s.prayer_reminder, |ui| switch_row(ui, "Also 10 minutes before", &mut s.prayer_early));
                        switch_row(ui, "Pause media at prayer time", &mut s.prayer_pause_media);
                        row_label(ui, "Adjust — minutes added to each time", 40.0);
                        ui.columns(prayer::NAMES.len(), |cols| {
                            for (i, c) in cols.iter_mut().enumerate() {
                                c.vertical_centered(|c| {
                                    c.label(egui::RichText::new(prayer::NAMES[i]).font(reg(12.0)).color(DIM));
                                    c.add(egui::DragValue::new(&mut s.prayer_adj[i]).range(-10..=10));
                                });
                            }
                        });
                        ui.add_space(8.0);
                    });
                });
                card(ui, "Shelf", |ui| {
                    switch_row(ui, "Start with an empty Shelf", &mut s.shelf_clear);
                    switch_row(ui, "Clipboard history — text you copy, ready to paste again", &mut s.clipboard);
                    ui.add_enabled_ui(s.clipboard, |ui| {
                        number(ui, "Keep up to", &mut s.clipboard_max, 1..=CLIP_MAX_FREE, " items");
                        switch_row(ui, "Remember after restart", &mut s.clipboard_keep);
                    });
                });
                card(ui, "Popups", |ui| {
                    switch_row(ui, "Volume", &mut s.hud_volume);
                    switch_row(ui, "Brightness", &mut s.hud_brightness);
                    switch_row(ui, "Caps Lock", &mut s.hud_caps);
                    switch_row(ui, "Input language", &mut s.hud_language);
                    switch_row(ui, "Battery", &mut s.popup_battery);
                    switch_row(ui, "Wi-Fi", &mut s.popup_wifi);
                    switch_row(ui, "Bluetooth headset", &mut s.popup_bluetooth);
                    switch_row(ui, "Hide in full screen — games, videos and slides that fill the screen", &mut s.hide_fullscreen);
                });
                card(ui, "Notifications", |ui| {
                    switch_row(ui, "Show notifications — new Windows notifications appear here (after restart)", &mut s.notifications);
                    ui.add_enabled_ui(s.notifications, |ui| switch_row(ui, "Show the message text", &mut s.notification_preview));
                    switch_row(ui, "Today page — recent notifications and your week", &mut s.today_page);
                });
                card(ui, "Music", |ui| {
                    switch_row(ui, "Show lyrics — from LRCLIB, a community lyrics database (after restart)", &mut s.lyrics);
                });
                card(ui, "Ask Nusa", |ui| {
                    // Kept sealed with Windows DPAPI, never in settings.txt.
                    if secret_row(ui, "Gemini key — free at aistudio.google.com/apikey; kept sealed on this PC", gemini) {
                        write_secret(GEMINI_FILE, gemini.trim());
                    }
                });
                card(ui, "Google Calendar", |ui| {
                    if secret_row(ui, "Secret iCal address — Google Calendar › Settings › your calendar › Integrate calendar", ical) {
                        write_secret(ICAL_FILE, ical.trim());
                        *ical_url.lock().unwrap() = ical.trim().to_string();
                    }
                    let state = match cal.status {
                        _ if ical.trim().is_empty() => t("Today's events show in To-do and remind you when they start").to_string(),
                        _ if !ical.trim().starts_with("https://") && !ical.trim().starts_with("webcal://") => t("The address should start with https://").to_string(),
                        calendar::Status::Ready => tr("{} events today", &[&cal.events.len()]),
                        calendar::Status::Failed => t("Couldn't read the calendar — check the address and the internet").to_string(),
                        _ => t("Reading the calendar…").to_string(),
                    };
                    ui.label(egui::RichText::new(state).font(reg(12.0)).color(DIM));
                    ui.add_space(8.0);
                });
                card(ui, "Pages", |ui| {
                    // Reorder with the arrows, leave pages out with the switch; one always stays.
                    let order = s.page_order.clone();
                    let shown = order.iter().filter(|&&p| !s.pages_hidden.contains(&p) && (p != TODAY_PAGE || s.today_page)).count();
                    for (i, &pg) in order.iter().enumerate() {
                        row(ui, PAGES[pg], |ui| {
                            let mut on = if pg == TODAY_PAGE { s.today_page } else { !s.pages_hidden.contains(&pg) };
                            let was = on;
                            if switch(ui, &mut on, PAGES[pg]) && !(was && shown == 1) {
                                if pg == TODAY_PAGE {
                                    s.today_page = on;
                                } else if on {
                                    s.pages_hidden.retain(|&p| p != pg);
                                } else {
                                    s.pages_hidden.push(pg);
                                }
                            }
                            ui.add_space(8.0);
                            if i + 1 < order.len() && move_button(ui, "↓", t("Move down")) {
                                s.page_order.swap(i, i + 1);
                            }
                            if i > 0 && move_button(ui, "↑", t("Move up")) {
                                s.page_order.swap(i, i - 1);
                            }
                        });
                    }
                });
                card(ui, "Widgets", |ui| {
                    combo(ui, "Left widget", &mut s.widget_left, &WIDGETS);
                    combo(ui, "Right widget", &mut s.widget_right, &WIDGETS);
                    if s.widget_left == WEATHER_WIDGET || s.widget_right == WEATHER_WIDGET {
                        let names: Vec<String> = (0..prayer::place_count())
                            .map(|i| if i == prayer::here_index() { t("My location").into() } else { let (r, c) = prayer::place_name(i); format!("{c} — {r}") })
                            .collect();
                        let refs: Vec<&str> = names.iter().map(|n| n.as_str()).collect();
                        combo(ui, "Weather for", &mut s.weather_city, &refs);
                    }
                    let cities: Vec<&str> = WORLD.iter().map(|w| w.0).collect();
                    combo(ui, "World clock, first city", &mut s.widget_city1, &cities);
                    combo(ui, "World clock, second city", &mut s.widget_city2, &cities);
                    combo(ui, "Countdown to", &mut s.widget_event, &EVENTS);
                    let mut wm = s.week_monday as usize;
                    if combo(ui, "Week starts on", &mut wm, &["Sunday", "Monday"]) {
                        s.week_monday = wm == 1;
                    }
                });
                ui.add_space(8.0);
            });
        });
        if self.s != before {
            if self.s.startup != before.startup {
                set_startup(self.s.startup);
            }
            if self.s.language != before.language {
                i18n::set(self.s.language);
                setup_fonts(ui.ctx());
                self.s.language = i18n::lang();
            }
            if self.s.app_icon != before.app_icon && self.shot.is_none() {
                tray::set_icon(self.s.app_icon);
            }
            self.clips.truncate(self.s.clipboard_max);
            self.save_clips();
            if self.mode == Mode::Focus && !self.running && self.s.focus != before.focus {
                self.left = self.mode_len(Mode::Focus);
            }
            self.s.save();
        }
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
                    self.s.startup = !self.s.startup;
                    set_startup(self.s.startup);
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
                Kind::Notice(_, id, _) if !id.is_empty() && !id.chars().any(|c| c == '"' || c.is_control()) => Some(id.clone()),
                _ => None,
            });
            if !self.expanded && resp.clicked() && notice_app.is_some() {
                // A notification opens the app that sent it, as a click on the toast would.
                let opened = notice_app.is_some_and(|id| std::process::Command::new("explorer.exe").arg(format!("shell:AppsFolder\\{id}")).spawn().is_ok());
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
                        if pill_button(ui, Rect::from_min_size(pos2(r.right() - 76.0, r.top() + 12.0), vec2(56.0, 24.0)), t("Close"), BTN, "sclose") {
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

// ---------- drawing helpers ----------

/// The notch: a black body hanging from the top edge with rounded bottom corners, plus two
/// concave "ears" where it meets the top edge.
fn notch_shape(p: &egui::Painter, body: Rect, rad: f32, ear: f32) {
    p.rect_filled(body, egui::CornerRadius { nw: 0, ne: 0, sw: rad as u8, se: rad as u8 }, BG);
    let mut mesh = egui::epaint::Mesh::default();
    for (corner, dir) in [(body.left_top(), -1.0f32), (body.right_top(), 1.0)] {
        // The ear is the square beside the top corner minus a quarter circle centred one ear
        // out and one ear down; it is a fan seen from the body's top corner.
        let center = corner + vec2(dir * ear, ear);
        let base = mesh.vertices.len() as u32;
        mesh.colored_vertex(corner, BG);
        let n = 10;
        for k in 0..=n {
            let a = (k as f32 / n as f32) * PI / 2.0; // 0: beside the body, PI/2: on the top edge
            mesh.colored_vertex(center + vec2(-dir * ear * a.cos(), -ear * a.sin()), BG);
        }
        for k in 0..n {
            mesh.add_triangle(base, base + 1 + k, base + 2 + k);
        }
    }
    p.add(Shape::mesh(mesh));
}

/// The sky's colour and a short label for a MET Norway symbol ("rainshowers_day", "clearsky_night"…).
fn sky(symbol: &str) -> (Color32, &'static str) {
    let night = symbol.ends_with("_night") || symbol.ends_with("_polartwilight");
    let kind = symbol.split('_').next().unwrap_or("");
    match kind {
        _ if kind.contains("thunder") => (Color32::from_rgb(98, 72, 160), "Thunder"),
        _ if kind.contains("snow") => (Color32::from_rgb(170, 196, 224), "Snow"),
        _ if kind.contains("sleet") => (Color32::from_rgb(120, 150, 180), "Sleet"),
        _ if kind.contains("rain") => (Color32::from_rgb(70, 110, 160), if kind.contains("showers") { "Showers" } else { "Rain" }),
        "fog" => (Color32::from_gray(140), "Fog"),
        "cloudy" => (Color32::from_rgb(120, 132, 150), "Cloudy"),
        "partlycloudy" | "fair" if night => (Color32::from_rgb(60, 70, 140), if kind == "fair" { "Fair" } else { "Partly cloudy" }),
        "partlycloudy" => (Color32::from_rgb(90, 160, 230), "Partly cloudy"),
        "fair" => (Color32::from_rgb(70, 165, 255), "Fair"),
        _ if night => (Color32::from_rgb(52, 58, 130), "Clear sky"),
        _ => (Color32::from_rgb(56, 170, 255), "Clear sky"),
    }
}

/// A small sun, moon, cloud, rain, snow or lightning picture for the symbol.
fn weather_icon(p: &egui::Painter, c: Pos2, symbol: &str, sky: Color32) {
    let night = symbol.ends_with("_night") || symbol.ends_with("_polartwilight");
    let clear = symbol.starts_with("clearsky") || symbol.starts_with("fair") || symbol.starts_with("partlycloudy");
    if clear {
        let sun = c + vec2(-4.0, -5.0);
        if night {
            p.circle_filled(sun, 11.0, Color32::from_rgb(235, 235, 210));
            p.circle_filled(sun + vec2(5.0, -4.0), 9.5, lerp_color(CARD, sky, 0.35)); // a crescent
        } else {
            p.circle_filled(sun, 10.0, Color32::from_rgb(255, 205, 64));
            for k in 0..8 {
                let a = k as f32 * PI / 4.0;
                let d = vec2(a.cos(), a.sin());
                p.line_segment([sun + d * 13.0, sun + d * 17.0], Stroke::new(2.0_f32, Color32::from_rgb(255, 205, 64)));
            }
        }
    }
    if !symbol.starts_with("clearsky") {
        let cloud = Color32::from_gray(if symbol.contains("rain") || symbol.contains("thunder") { 185 } else { 225 });
        let base = c + vec2(4.0, 6.0);
        p.circle_filled(base + vec2(-9.0, 2.0), 7.0, cloud);
        p.circle_filled(base + vec2(0.0, -3.0), 9.0, cloud);
        p.circle_filled(base + vec2(9.0, 2.0), 7.0, cloud);
        p.rect_filled(Rect::from_min_max(base + vec2(-9.0, 2.0), base + vec2(9.0, 9.0)), 0.0, cloud);
        let below = base + vec2(0.0, 14.0);
        if symbol.contains("thunder") {
            p.add(Shape::line(vec![below + vec2(2.0, -2.0), below + vec2(-3.0, 4.0), below + vec2(2.0, 4.0), below + vec2(-2.0, 10.0)], Stroke::new(2.0_f32, Color32::from_rgb(255, 214, 80))));
        } else if symbol.contains("snow") || symbol.contains("sleet") {
            for dx in [-7.0, 0.0, 7.0] {
                p.circle_filled(below + vec2(dx, 2.0), 2.0, WHITE);
            }
        } else if symbol.contains("rain") {
            for dx in [-7.0, 0.0, 7.0] {
                p.line_segment([below + vec2(dx, -1.0), below + vec2(dx - 2.0, 5.0)], Stroke::new(2.0_f32, Color32::from_rgb(120, 180, 255)));
            }
        }
    }
}

fn ring(p: &egui::Painter, c: Pos2, r: f32, w: f32, frac: f32, color: Color32) {
    p.circle_stroke(c, r, Stroke::new(w, color.gamma_multiply(0.22)));
    let frac = frac.clamp(0.0, 1.0);
    if frac > 0.0 {
        let n = (48.0 * frac).max(2.0) as usize;
        let pts: Vec<Pos2> = (0..=n)
            .map(|i| {
                let a = -PI / 2.0 + frac * 2.0 * PI * i as f32 / n as f32;
                c + r * vec2(a.cos(), a.sin())
            })
            .collect();
        p.add(Shape::line(pts, Stroke::new(w, color)));
    }
}

/// The original's little level bars at the right of the header.
fn bars(p: &egui::Painter, c: Pos2, lively: bool, col: Color32) {
    let t = if lively { (Utc::now().timestamp_millis() % 100_000) as f32 / 1000.0 } else { 0.0 };
    let base = [0.55, 1.0, 0.75, 1.0, 0.6];
    for (k, h) in base.iter().enumerate() {
        let wave = if lively { 0.6 + 0.4 * (t * 5.0 + k as f32 * 1.3).sin().abs() } else { 1.0 };
        let hh = 16.0 * h * wave;
        let x = c.x - 10.0 + k as f32 * 5.0;
        p.rect_filled(Rect::from_center_size(pos2(x, c.y), vec2(2.4, hh)), 1.2, col);
    }
}

fn gradient_bar(p: &egui::Painter, r: Rect, frac: f32, a: Color32, b: Color32) {
    p.rect_filled(r, r.height() / 2.0, Color32::from_gray(38));
    let frac = frac.clamp(0.0, 1.0);
    if frac <= 0.0 {
        return;
    }
    let fill = Rect::from_min_size(r.min, vec2(r.width() * frac, r.height()));
    let mut mesh = egui::epaint::Mesh::default();
    let segs = 16;
    for k in 0..=segs {
        let u = k as f32 / segs as f32;
        let x = fill.left() + fill.width() * u;
        let col = lerp_color(a, b, u);
        mesh.colored_vertex(pos2(x, fill.top()), col);
        mesh.colored_vertex(pos2(x, fill.bottom()), col);
        if k > 0 {
            let i = (k * 2) as u32;
            mesh.add_triangle(i - 2, i - 1, i);
            mesh.add_triangle(i - 1, i, i + 1);
        }
    }
    p.add(Shape::mesh(mesh));
    // Round the ends.
    p.circle_filled(pos2(fill.left() + r.height() / 2.0, fill.center().y), r.height() / 2.0, a);
    p.circle_filled(pos2(fill.right() - r.height() / 2.0, fill.center().y), r.height() / 2.0, lerp_color(a, b, 1.0));
}

fn vertical_gradient(p: &egui::Painter, r: Rect, rad: f32, top: Color32, bottom: Color32) {
    p.rect_filled(r, rad, bottom);
    p.rect_filled(Rect::from_min_size(r.min, vec2(r.width(), (r.height() * 0.5).max(rad))), rad, top.gamma_multiply(0.95));
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

fn dashed(p: &egui::Painter, x0: f32, x1: f32, y: f32) {
    let mut x = x0;
    while x < x1 {
        p.line_segment([pos2(x, y), pos2((x + 3.0).min(x1), y)], Stroke::new(1.0_f32, Color32::from_gray(55)));
        x += 6.0;
    }
}

fn dashed_rect(p: &egui::Painter, r: Rect, col: Color32) {
    p.rect_stroke(r, 14.0, Stroke::new(1.3_f32, col), egui::StrokeKind::Inside);
}

fn circle_button(ui: &mut egui::Ui, c: Pos2, rad: f32, fill: Color32, dir: &str) -> bool {
    let rect = Rect::from_center_size(c, vec2(rad * 2.0, rad * 2.0));
    let resp = ui.interact(rect, ui.id().with(("arrow", dir)), Sense::click());
    let p = ui.painter();
    p.circle_filled(c, rad, if resp.hovered() { Color32::from_gray(90) } else { fill });
    let s = if dir == "left" { -1.0 } else { 1.0 };
    let st = Stroke::new(2.0_f32, WHITE);
    p.line_segment([c + vec2(-2.5 * s, -4.5), c + vec2(2.5 * s, 0.0)], st);
    p.line_segment([c + vec2(2.5 * s, 0.0), c + vec2(-2.5 * s, 4.5)], st);
    resp.clicked()
}

/// A small round button with a "reset" arrow or a "skip" (▶|) mark; `name` is its tooltip and
/// what screen readers say.
fn icon_button(ui: &mut egui::Ui, c: Pos2, rad: f32, kind: &str, name: &str) -> bool {
    let rect = Rect::from_center_size(c, vec2(rad * 2.0, rad * 2.0));
    let resp = ui.interact(rect, ui.id().with(kind), Sense::click()).on_hover_text(name);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
    let p = ui.painter();
    p.circle_filled(c, rad, if resp.hovered() { Color32::from_gray(60) } else { BTN });
    if kind == "skip" {
        p.add(Shape::convex_polygon(vec![c + vec2(-4.0, -4.5), c + vec2(2.0, 0.0), c + vec2(-4.0, 4.5)], WHITE, Stroke::NONE));
        p.rect_filled(Rect::from_center_size(c + vec2(3.6, 0.0), vec2(1.8, 9.0)), 0.5, WHITE);
    } else {
        // A small circular "reset" arrow.
        let pts: Vec<Pos2> = (0..=14).map(|k| {
            let a = -PI * 0.35 + (k as f32 / 14.0) * PI * 1.6;
            c + 5.0 * vec2(a.cos(), a.sin())
        }).collect();
        let end = pts[0];
        p.add(Shape::line(pts, Stroke::new(1.4_f32, WHITE)));
        p.line_segment([end, end + vec2(3.0, -1.0)], Stroke::new(1.4_f32, WHITE));
    }
    resp.clicked()
}

/// A placeholder cover like the original's demo art: pale blue stripes and a blue disc.
fn album_art(p: &egui::Painter, r: Rect) {
    p.rect_filled(r, 10.0, Color32::from_rgb(222, 234, 250));
    let mut y = r.top() + 6.0;
    while y < r.bottom() - 4.0 {
        p.line_segment([pos2(r.left() + 4.0, y), pos2(r.right() - 4.0, y)], Stroke::new(1.5_f32, Color32::from_rgb(190, 212, 242)));
        y += 7.0;
    }
    let c = r.center() + vec2(-6.0, 6.0);
    p.circle_stroke(c, r.width() * 0.3, Stroke::new(2.5_f32, Color32::from_rgb(70, 140, 230)));
    p.circle_filled(c, r.width() * 0.22, Color32::from_rgb(45, 120, 220));
    p.line_segment([r.left_bottom() + vec2(r.width() * 0.25, -2.0), r.right_top() + vec2(-r.width() * 0.25, 2.0)], Stroke::new(1.5_f32, WHITE));
}

/// Media control glyphs: lyrics bubble, previous, play, pause and next.
fn glyph_button(ui: &mut egui::Ui, c: Pos2, kind: &str, col: Color32) -> bool {
    let rect = Rect::from_center_size(c, vec2(30.0, 26.0));
    let resp = ui.interact(rect, ui.id().with(("glyph", kind)), Sense::click());
    let p = ui.painter();
    if resp.hovered() {
        p.circle_filled(c, 15.0, Color32::from_gray(30));
    }
    let tri = |x: f32, dir: f32, h: f32| vec![pos2(x, c.y - h), pos2(x + dir * h * 1.1, c.y), pos2(x, c.y + h)];
    match kind {
        "play" => {
            p.add(Shape::convex_polygon(tri(c.x - 5.0, 1.0, 7.0), col, Stroke::NONE));
        }
        "pause" => {
            for dx in [-3.5, 3.5] {
                p.rect_filled(Rect::from_center_size(c + vec2(dx, 0.0), vec2(3.5, 13.0)), 1.0, col);
            }
        }
        "next" => {
            p.add(Shape::convex_polygon(tri(c.x - 8.0, 1.0, 5.5), col, Stroke::NONE));
            p.add(Shape::convex_polygon(tri(c.x, 1.0, 5.5), col, Stroke::NONE));
        }
        "prev" => {
            p.add(Shape::convex_polygon(tri(c.x + 8.0, -1.0, 5.5), col, Stroke::NONE));
            p.add(Shape::convex_polygon(tri(c.x, -1.0, 5.5), col, Stroke::NONE));
        }
        _ => {
            let b = Rect::from_center_size(c + vec2(0.0, -1.5), vec2(18.0, 13.0));
            // Outlined speech bubble with two dots, as in reference pictures 12 and 13.
            p.rect_stroke(b, 4.0, Stroke::new(1.6_f32, col), egui::StrokeKind::Inside);
            p.add(Shape::convex_polygon(vec![pos2(b.left() + 4.0, b.bottom() - 1.0), pos2(b.left() + 9.0, b.bottom() - 1.0), pos2(b.left() + 4.0, b.bottom() + 4.0)], col, Stroke::NONE));
            for dx in [-3.0, 3.0] {
                p.circle_filled(b.center() + vec2(dx, 0.0), 1.5, col);
            }
        }
    }
    resp.clicked()
}

fn pill_button(ui: &mut egui::Ui, r: Rect, text: &str, fill: Color32, id: &str) -> bool {
    let resp = ui.interact(r, ui.id().with(("pill", id)), Sense::click());
    let p = ui.painter();
    let f = if resp.hovered() { lerp_color(fill, WHITE, 0.12) } else { fill };
    p.rect_filled(r, r.height() / 2.0, f);
    // Translations can be longer than the English the button was sized for: shrink to fit.
    let wide = p.layout_no_wrap(text.to_string(), bold(14.0), WHITE).size().x;
    let size = if wide > 0.0 { (14.0 * (r.width() - 12.0) / wide).clamp(9.0, 14.0) } else { 14.0 };
    p.text(r.center(), Align2::CENTER_CENTER, text, bold(size), WHITE);
    resp.clicked()
}

fn small_step(ui: &mut egui::Ui, c: Pos2, text: &str, id: &str) -> bool {
    let r = Rect::from_center_size(c, vec2(20.0, 20.0));
    let resp = ui.interact(r, ui.id().with(("step", id)), Sense::click());
    ui.painter().circle_filled(c, 10.0, if resp.hovered() { Color32::from_gray(60) } else { BTN });
    ui.painter().text(c, Align2::CENTER_CENTER, text, bold(13.0), WHITE);
    resp.clicked()
}

fn toggle(ui: &mut egui::Ui, at: Pos2, on: bool) -> bool {
    let r = Rect::from_min_size(at, vec2(44.0, 24.0));
    let resp = ui.interact(r, ui.id().with("toggle"), Sense::click());
    let t = ui.ctx().animate_bool(ui.id().with("toggle-anim"), on);
    let p = ui.painter();
    p.rect_filled(r, 12.0, lerp_color(Color32::from_gray(60), BLUE, t));
    let knob = pos2(r.left() + 12.0 + t * 20.0, r.center().y);
    p.circle_filled(knob, 9.0, WHITE);
    if on {
        p.circle_filled(knob, 3.0, BLUE);
    }
    resp.clicked()
}

fn check_circle(p: &egui::Painter, c: Pos2, r: f32, done: bool) {
    if done {
        p.circle_filled(c, r, GREEN);
        let st = Stroke::new(2.0_f32, Color32::BLACK);
        p.line_segment([c + vec2(-r * 0.45, 0.0), c + vec2(-r * 0.1, r * 0.35)], st);
        p.line_segment([c + vec2(-r * 0.1, r * 0.35), c + vec2(r * 0.45, -r * 0.35)], st);
    } else {
        p.circle_stroke(c, r - 1.0, Stroke::new(1.5_f32, Color32::from_gray(110)));
    }
}

fn bolt(p: &egui::Painter, c: Pos2, s: f32, col: Color32) {
    let pts = vec![
        c + vec2(0.15 * s, -s),
        c + vec2(-0.55 * s, 0.15 * s),
        c + vec2(-0.05 * s, 0.15 * s),
        c + vec2(-0.15 * s, s),
        c + vec2(0.55 * s, -0.15 * s),
        c + vec2(0.05 * s, -0.15 * s),
    ];
    p.add(Shape::convex_polygon(pts[..3].to_vec(), col, Stroke::NONE));
    p.add(Shape::convex_polygon(pts[3..].to_vec(), col, Stroke::NONE));
    p.add(Shape::convex_polygon(vec![pts[1], pts[2], pts[4], pts[5]], col, Stroke::NONE));
}

fn moon_icon(p: &egui::Painter, c: Pos2, r: f32) {
    p.circle_filled(c, r, WHITE);
    p.circle_filled(c + vec2(r * 0.45, -r * 0.3), r * 0.9, BG);
}

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

/// `text` on one line no wider than `width`, ending in "…" when it is cut (cut by real width,
/// so Vietnamese and Chinese text fill the line too).
fn fit_line(p: &egui::Painter, text: &str, font: FontId, col: Color32, width: f32) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(text.replace('\n', " "), font, col);
    job.wrap = egui::text::TextWrapping { max_width: width.max(10.0), max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
    p.layout_job(job)
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

/// A small page of a calendar: a frame with a filled top band.
fn calendar_icon(p: &egui::Painter, c: Pos2, col: Color32) {
    let r = Rect::from_center_size(c, vec2(20.0, 18.0));
    p.rect_stroke(r, 3.0, Stroke::new(1.6_f32, col), egui::StrokeKind::Inside);
    p.rect_filled(Rect::from_min_max(r.min, pos2(r.right(), r.top() + 5.0)), egui::CornerRadius { nw: 3, ne: 3, sw: 0, se: 0 }, col);
    p.circle_filled(c + vec2(0.0, 3.0), 2.0, col);
}

fn drop_icon(p: &egui::Painter, c: Pos2, r: f32, col: Color32) {
    p.circle_filled(c + vec2(0.0, r * 0.25), r * 0.7, col);
    p.add(Shape::convex_polygon(vec![c + vec2(0.0, -r), c + vec2(r * 0.66, r * 0.05), c + vec2(-r * 0.66, r * 0.05)], col, Stroke::NONE));
}

fn folder_icon(p: &egui::Painter, c: Pos2, col: Color32) {
    let r = Rect::from_center_size(c + vec2(0.0, 1.0), vec2(14.0, 10.0));
    p.rect_stroke(r, 2.0, Stroke::new(1.3_f32, col), egui::StrokeKind::Inside);
    p.line_segment([r.left_top() + vec2(1.0, -2.0), r.left_top() + vec2(6.0, -2.0)], Stroke::new(1.3_f32, col));
}

fn clip_icon(p: &egui::Painter, c: Pos2, col: Color32) {
    let r = Rect::from_center_size(c + vec2(0.0, 1.0), vec2(11.0, 13.0));
    p.rect_stroke(r, 2.0, Stroke::new(1.3_f32, col), egui::StrokeKind::Inside);
    p.rect_filled(Rect::from_center_size(pos2(c.x, r.top()), vec2(6.0, 3.0)), 1.0, col);
}

/// A colourful tile per file type (pictures get a little landscape like the original).
fn file_icon(p: &egui::Painter, r: Rect, path: &std::path::Path) {
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let (top, label) = match ext.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" => (Color32::from_rgb(77, 182, 255), ""),
        "pdf" => (Color32::from_rgb(239, 68, 68), "PDF"),
        "doc" | "docx" | "txt" | "md" => (Color32::from_rgb(59, 130, 246), "DOC"),
        "xls" | "xlsx" | "csv" => (Color32::from_rgb(34, 197, 94), "XLS"),
        "zip" | "rar" | "7z" => (ORANGE, "ZIP"),
        "mp3" | "wav" | "flac" | "mp4" | "mkv" | "mov" => (PURPLE, "▶"),
        _ if path.is_dir() => (Color32::from_rgb(250, 204, 21), "DIR"),
        _ => (Color32::from_gray(110), "FILE"),
    };
    p.rect_filled(r, 12.0, top);
    if label.is_empty() {
        let sea = Rect::from_min_max(pos2(r.left(), r.center().y + 4.0), r.max);
        p.rect_filled(sea, egui::CornerRadius { nw: 0, ne: 0, sw: 12, se: 12 }, Color32::from_rgb(37, 150, 210));
        p.rect_filled(Rect::from_min_max(pos2(r.left(), r.center().y + 2.0), pos2(r.right(), r.center().y + 7.0)), 0.0, Color32::from_rgb(250, 214, 140));
        p.circle_filled(pos2(r.right() - 15.0, r.top() + 16.0), 7.0, Color32::from_rgb(253, 243, 180));
    } else {
        p.text(r.center(), Align2::CENTER_CENTER, label, bold(14.0), WHITE);
    }
}

/// Dark controls for Settings: combo boxes, number fields and their drop-down lists.
fn settings_style(ui: &mut egui::Ui) {
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

/// A titled group of rows on a dark rounded card.
fn card(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(10.0);
    ui.label(egui::RichText::new(t(title).to_uppercase()).font(bold(11.0)).color(DIM));
    ui.add_space(4.0);
    egui::Frame::new().fill(CARD).corner_radius(12).inner_margin(egui::Margin::symmetric(12, 2)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        add(ui);
    });
}

/// "Name — hint": the name, with the hint dimmer underneath.
fn row_text(ui: &mut egui::Ui, label: &str) {
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

fn row_label(ui: &mut egui::Ui, label: &str, h: f32) {
    ui.allocate_ui_with_layout(vec2(ui.available_width(), h), egui::Layout::left_to_right(egui::Align::Center), |ui| row_text(ui, label));
}

/// One row: text on the left, the control on the right, centred in a fixed height.
fn row(ui: &mut egui::Ui, label: &str, control: impl FnOnce(&mut egui::Ui)) {
    let h = if label.contains(" — ") { 46.0 } else { 34.0 };
    ui.allocate_ui_with_layout(vec2(ui.available_width(), h), egui::Layout::right_to_left(egui::Align::Center), |ui| {
        control(ui);
        ui.add_space(10.0);
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| row_text(ui, label));
    });
}

/// A switch like the Water page's, reported to screen readers as a checkbox with its label.
fn switch_row(ui: &mut egui::Ui, label: &str, v: &mut bool) {
    row(ui, label, |ui| {
        switch(ui, v, label);
    });
}

/// The switch on its own; true when it was flipped.
fn switch(ui: &mut egui::Ui, v: &mut bool, label: &str) -> bool {
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
fn move_button(ui: &mut egui::Ui, text: &str, what: &str) -> bool {
    let (r, resp) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), what));
    let p = ui.painter();
    p.circle_filled(r.center(), 11.0, if resp.hovered() { Color32::from_gray(60) } else { BTN });
    p.text(r.center(), Align2::CENTER_CENTER, text, bold(13.0), WHITE);
    resp.clicked()
}

/// A row with a hidden text box for a secret; true when it changed.
fn secret_row(ui: &mut egui::Ui, label: &str, v: &mut String) -> bool {
    let mut changed = false;
    row(ui, label, |ui| {
        let name = t(label).split(" — ").next().unwrap_or(label).to_string();
        let resp = ui.add(egui::TextEdit::singleline(v).password(true).hint_text(t("Paste here")).desired_width(170.0).background_color(BTN).margin(egui::Margin::symmetric(8, 5)));
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, &name));
        changed = resp.changed();
    });
    changed
}

fn number<T: egui::emath::Numeric>(ui: &mut egui::Ui, label: &str, v: &mut T, range: std::ops::RangeInclusive<T>, suffix: &str) {
    row(ui, label, |ui| {
        ui.add(egui::DragValue::new(v).range(range).suffix(t(suffix)));
    });
}

fn combo(ui: &mut egui::Ui, label: &str, v: &mut usize, items: &[&str]) -> bool {
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

fn next_event(today: NaiveDate, ev: usize) -> Option<NaiveDate> {
    let fixed = |m, d| {
        let this = NaiveDate::from_ymd_opt(today.year(), m, d)?;
        Some(if this >= today { this } else { NaiveDate::from_ymd_opt(today.year() + 1, m, d)? })
    };
    let table = |t: &[(i32, u32, u32)]| t.iter().filter_map(|&(y, m, d)| NaiveDate::from_ymd_opt(y, m, d)).find(|d| *d >= today);
    match ev {
        0 => fixed(1, 1),
        1 => table(&RAMADAN),
        2 => table(&EID_FITR),
        3 => table(&EID_ADHA),
        4 => fixed(8, 17),
        _ => fixed(12, 25),
    }
}

/// The original's full-screen rule (0x140051a46): hide when Windows says a full-screen game or
/// presentation runs, or when the front window (not ours, not the desktop or taskbar, not a
/// maximised window with a title bar) covers its whole monitor.
fn fullscreen_in_front() -> bool {
    use windows_sys::Win32::{
        Foundation::RECT,
        Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST},
        System::Threading::GetCurrentProcessId,
        UI::Shell::{SHQueryUserNotificationState, QUNS_RUNNING_D3D_FULL_SCREEN, QUNS_PRESENTATION_MODE},
        UI::WindowsAndMessaging::*,
    };
    unsafe {
        let mut state = 0;
        if SHQueryUserNotificationState(&mut state) >= 0 && (state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_PRESENTATION_MODE) {
            return true;
        }
        let w = GetForegroundWindow();
        if w.is_null() || IsWindowVisible(w) == 0 || IsIconic(w) != 0 {
            return false;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(w, &mut pid);
        if pid == GetCurrentProcessId() {
            return false;
        }
        let mut class = [0u16; 32];
        let n = GetClassNameW(w, class.as_mut_ptr(), 32).max(0) as usize;
        let class = String::from_utf16_lossy(&class[..n]);
        if matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd") {
            return false;
        }
        let style = GetWindowLongW(w, GWL_STYLE) as u32;
        // A maximised window with a title bar is an ordinary window, not full screen.
        if style & WS_CAPTION == WS_CAPTION && IsZoomed(w) != 0 {
            return false;
        }
        let mut r: RECT = std::mem::zeroed();
        GetWindowRect(w, &mut r);
        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        GetMonitorInfoW(MonitorFromWindow(w, MONITOR_DEFAULTTONEAREST), &mut mi);
        let m = mi.rcMonitor;
        r.left <= m.left && r.top <= m.top && r.right >= m.right && r.bottom >= m.bottom
    }
}

fn caps_lock() -> bool {
    unsafe { windows_sys::Win32::UI::Input::KeyboardAndMouse::GetKeyState(0x14) & 1 != 0 } // VK_CAPITAL
}

/// The keyboard language of the window in front (its language id).
fn input_lang() -> u16 {
    use windows_sys::Win32::UI::{Input::KeyboardAndMouse::GetKeyboardLayout, WindowsAndMessaging::*};
    unsafe {
        let thread = GetWindowThreadProcessId(GetForegroundWindow(), std::ptr::null_mut());
        (GetKeyboardLayout(thread) as usize & 0xffff) as u16
    }
}

/// "EN", "VI"… for a language id.
fn lang_name(lang: u16) -> String {
    use windows_sys::Win32::Globalization::{GetLocaleInfoW, LOCALE_SISO639LANGNAME};
    let mut buf = [0u16; 9];
    let n = unsafe { GetLocaleInfoW(lang as u32, LOCALE_SISO639LANGNAME, buf.as_mut_ptr(), 9) };
    let s = String::from_utf16_lossy(&buf[..(n.max(1) - 1) as usize]).to_uppercase();
    if s.is_empty() { format!("{lang:04X}") } else { s }
}

/// `--page 0 --demo` on a screenshot: a made-up song with a made-up cover (a violet-to-orange
/// square), to check the Music page without anyone's real music.
fn demo_music(m: media::Media, shot_music: bool) -> media::Media {
    if shot_music && std::env::args().any(|a| a == "--demo") {
        let px = media::ART_PX as usize;
        let art: Vec<u8> = (0..px * px)
            .flat_map(|i| {
                let (x, y) = ((i % px) as f32 / px as f32, (i / px) as f32 / px as f32);
                [(120.0 + 120.0 * x) as u8, (40.0 + 90.0 * y) as u8, (200.0 - 150.0 * x * y) as u8, 255]
            })
            .collect();
        *m.now.lock().unwrap() = media::Now {
            title: "Nơi này có anh (Lofi Version) — bản phối dài cho buổi chiều".into(),
            artist: "Sơn Tùng M-TP".into(),
            album: "Chill Việt 2026".into(),
            playing: true,
            position: 72.0,
            duration: 245.0,
            read_at: Some(Instant::now()),
            tint: media::cover_tint(&art),
            art: Some(std::sync::Arc::new((px, px, art))),
            art_key: "demo".into(),
            app_id: "demo.player".into(),
            app_name: "Spotify".into(),
            players: 2,
            ..Default::default()
        };
    }
    m
}

const DEMO_ANSWER: &str = "Sunlight is made of every colour. Air scatters short blue waves far more than red ones, \
    so blue light reaches your eyes from all over the sky. At sunset the light crosses more air, and the reds win.";

/// Made-up notifications for checking the Today page's look (`--page 7 --demo`).
fn demo_recent() -> Vec<(i64, media::Notice)> {
    let now = Utc::now().timestamp();
    [("Messenger", "Lan", "See you at 7"), ("Outlook", "Weekly report", "Attached is this week's report"), ("Zalo", "Minh", "Ok")]
        .iter()
        .enumerate()
        .map(|(k, (a, t, b))| (now - k as i64 * 400, (a.to_string(), t.to_string(), b.to_string(), String::new())))
        .collect()
}

/// `--popup bluetooth|notice|brightness` with --screenshot --resting shows that pop-up, for checking its look.
fn demo_alert() -> Option<Alert> {
    let args: Vec<String> = std::env::args().collect();
    let which = args.iter().position(|a| a == "--popup").and_then(|i| args.get(i + 1))?;
    let (title, body, kind) = match which.as_str() {
        "bluetooth" => ("AirBuds Pro", "Connected · 70%", Kind::Bluetooth(Some(70))),
        "notice" => ("Nusa", "Hello, Nusa 1.2!", Kind::Notice("Messages".into(), String::new(), 0)),
        // An installed app's real logo (Windows Settings), as a notification from it would show.
        "notice-logo" => {
            let id = "windows.immersivecontrolpanel_cw5n1h2txyewy!microsoft.windows.immersivecontrolpanel";
            media::preload_logo(id);
            ("Nhóm: TRI THỨC CHỈ ĐẠO CHUNG VÀ PHỐI HỢP", "Anh Tám (Phòng KH): Mai họp lúc 9h nhé mọi người, nhớ mang laptop", Kind::Notice("Zalo".into(), id.into(), 2))
        }
        "notice-long" => ("Nhóm: TRI THỨC CHỈ ĐẠO CHUNG VÀ PHỐI HỢP", "Anh Tám (Phòng KH): Mai họp lúc 9h nhé mọi người, nhớ mang laptop", Kind::Notice("Zalo".into(), String::new(), 0)),
        "brightness" => ("Brightness", "", Kind::Hud(HudKind::Brightness, 0.6)),
        "welcome" => return Some(welcome_hint(false)),
        "taken" => return Some(taken_hint(tray::TAKEN_ASK | tray::TAKEN_OCR)),
        _ => return None,
    };
    Some(Alert { title: title.into(), body: body.into(), kind, at: Instant::now() })
}

/// The first start's hint: how to open the notch and reach Settings.
fn welcome_hint(open_click: bool) -> Alert {
    let body = if open_click { t("Click to open · right-click for Settings") } else { t("Hover to open · right-click for Settings") };
    Alert { title: t("Welcome to My Notch").into(), body: body.into(), kind: Kind::Hint, at: Instant::now() }
}

/// Says which shortcuts another app already owns (`bits` from tray::hotkeys_taken).
fn taken_hint(bits: u8) -> Alert {
    let keys: Vec<&str> = [(tray::TAKEN_ASK, "Ctrl+Alt+Space"), (tray::TAKEN_OCR, "Ctrl+Alt+T")]
        .iter()
        .filter(|(b, _)| bits & b != 0)
        .map(|&(_, k)| k)
        .collect();
    Alert { title: t("Shortcut used by another app").into(), body: keys.join(" · "), kind: Kind::Hint, at: Instant::now() }
}

fn sun_icon(p: &egui::Painter, c: Pos2, col: Color32) {
    p.circle_filled(c, 4.5, col);
    for i in 0..8 {
        let a = i as f32 * PI / 4.0;
        let d = vec2(a.cos(), a.sin());
        p.line_segment([c + d * 7.5, c + d * 10.5], Stroke::new(1.8_f32, col));
    }
}

fn bluetooth_rune(p: &egui::Painter, c: Pos2, s: f32, col: Color32) {
    let st = Stroke::new(1.6_f32, col);
    let (top, bot) = (c + vec2(0.0, -s * 1.3), c + vec2(0.0, s * 1.3));
    p.line_segment([top, bot], st);
    p.add(Shape::line(vec![c + vec2(-s * 0.8, -s * 0.65), c + vec2(s * 0.8, s * 0.65), bot], st));
    p.add(Shape::line(vec![c + vec2(-s * 0.8, s * 0.65), c + vec2(s * 0.8, -s * 0.65), top], st));
}

fn headphone_icon(p: &egui::Painter, c: Pos2, s: f32, col: Color32) {
    let pts: Vec<Pos2> = (0..=16).map(|i| {
        let a = PI + PI * i as f32 / 16.0;
        c + vec2(0.0, 2.0) + s * vec2(a.cos(), a.sin())
    }).collect();
    p.add(Shape::line(pts, Stroke::new(1.6_f32, col)));
    for x in [-s, s] {
        p.rect_filled(Rect::from_center_size(c + vec2(x * 0.9, s * 0.55), vec2(3.5, 7.0)), 1.5, col);
    }
}

fn speaker_icon(p: &egui::Painter, c: Pos2, col: Color32, muted: bool) {
    p.rect_filled(Rect::from_center_size(c + vec2(-6.0, 0.0), vec2(5.0, 8.0)), 1.0, col);
    p.add(Shape::convex_polygon(vec![c + vec2(-4.0, -4.0), c + vec2(2.0, -9.0), c + vec2(2.0, 9.0), c + vec2(-4.0, 4.0)], col, Stroke::NONE));
    if muted {
        p.line_segment([c + vec2(6.0, -4.0), c + vec2(13.0, 4.0)], Stroke::new(2.0_f32, col));
        p.line_segment([c + vec2(13.0, -4.0), c + vec2(6.0, 4.0)], Stroke::new(2.0_f32, col));
    } else {
        for (k, r) in [6.0f32, 11.0].iter().enumerate() {
            let pts: Vec<Pos2> = (0..=8).map(|i| {
                let a = -0.8 + 1.6 * i as f32 / 8.0;
                c + vec2(2.0, 0.0) + *r * vec2(a.cos(), a.sin())
            }).collect();
            p.add(Shape::line(pts, Stroke::new(1.8_f32 - k as f32 * 0.3, col)));
        }
    }
}

/// The mascot (reference picture 15): two white eyes on the black pill with a cyan-to-purple glow
/// along the bottom; it blinks now and then (blink timing estimated).
fn mascot(p: &egui::Painter, r: Rect) {
    // Soft glow filling the lower half and following the pill's rounded bottom corners (same radius
    // as notch_shape): cyan on the left, bright blue in the middle, purple fading out on the right.
    // A mesh has no anti-aliasing, so the glow stops short of the shape's edge and fades out over
    // the last few pixels; the black body's smooth edge stays outermost.
    const FEATHER: f32 = 1.5;
    let rad = (r.height() * 0.2).clamp(12.0, 32.0);
    // Bottom edge at x of the shape inset by `inset`: straight, rising along the corner arcs.
    let edge = |x: f32, inset: f32| {
        let rr = rad - inset;
        let into = (rr - (x - r.left() - inset)).max(rr - (r.right() - inset - x)).clamp(0.0, rr);
        r.bottom() - inset - rr + (rr * rr - into * into).sqrt()
    };
    let top = r.top() + r.height() * 0.3;
    let mid = r.top() + r.height() * 0.68;
    // Glow strength depends on height alone (0 at `top`, 0.3 at `mid`, full at the very bottom);
    // the corners just cut it off. Peaking at each column's own edge drew a bright rim round them.
    let strength = |y: f32| {
        if y < mid {
            0.3 * ((y - top) / (mid - top)).max(0.0)
        } else {
            0.3 + 0.7 * ((y - mid) / (r.bottom() - mid)).min(1.0)
        }
    };
    let (x0, x1) = (r.left() + 0.5, r.right() - 0.5);
    let mut mesh = egui::epaint::Mesh::default();
    let segs = 96;
    for k in 0..=segs {
        let u = k as f32 / segs as f32;
        let x = x0 + (x1 - x0) * u;
        let col = if u < 0.5 {
            lerp_color(Color32::from_rgb(34, 211, 238), Color32::from_rgb(59, 110, 255), u * 2.0)
        } else {
            lerp_color(Color32::from_rgb(59, 110, 255), Color32::from_rgb(110, 50, 200), (u - 0.5) * 2.0)
        };
        // The right end fades towards the dark corner, as in the reference; both ends fade out
        // over FEATHER so the sides are as soft as the bottom.
        let side = ((x - x0).min(x1 - x) / FEATHER).clamp(0.0, 1.0);
        let col = col.gamma_multiply((1.0 - 0.45 * ((u - 0.6) / 0.4).clamp(0.0, 1.0)) * side);
        let (inner, outer) = (edge(x, FEATHER + 0.5), edge(x, 0.5));
        let m = mid.min(inner);
        mesh.colored_vertex(pos2(x, top), Color32::TRANSPARENT);
        mesh.colored_vertex(pos2(x, m), col.gamma_multiply(strength(m)));
        mesh.colored_vertex(pos2(x, inner), col.gamma_multiply(strength(inner)));
        mesh.colored_vertex(pos2(x, outer), Color32::TRANSPARENT);
        if k > 0 {
            let i = (k * 4) as u32;
            for row in 0..3 {
                let (a, b) = (i - 4 + row, i + row);
                mesh.add_triangle(a, a + 1, b);
                mesh.add_triangle(a + 1, b, b + 1);
            }
        }
    }
    p.add(Shape::mesh(mesh));
    let open = blink_open(Utc::now().timestamp_millis());
    for dx in [-16.0, 16.0] {
        let c = pos2(r.center().x + dx, r.center().y - 1.0);
        // Closed eyes flatten into a slightly wider line (reference picture 15, second frame).
        let size = vec2(5.0 + 2.0 * (1.0 - open), (8.0 * open).max(1.2));
        // Soft blue halo, fading outwards.
        for (grow, alpha) in [(6.0, 8), (3.5, 14), (1.8, 26)] {
            p.add(Shape::ellipse_filled(c, size + vec2(grow, grow), Color32::from_rgba_unmultiplied(120, 170, 255, alpha)));
        }
        p.add(Shape::ellipse_filled(c, size, WHITE));
    }
}

/// How open the mascot's eyes are (1 = open) at this time: a smooth close-and-open over BLINK_MS.
fn blink_open(ms: i64) -> f32 {
    let b = ms.rem_euclid(BLINK_EVERY_MS);
    if b >= BLINK_MS {
        return 1.0;
    }
    1.0 - (std::f32::consts::PI * b as f32 / BLINK_MS as f32).sin() * 0.9
}

fn battery() -> Option<(u8, bool)> {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut s: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    // Flag 128 = no system battery (desktop PCs).
    if unsafe { GetSystemPowerStatus(&mut s) } == 0 || s.BatteryLifePercent == 255 || s.BatteryFlag & 128 != 0 {
        return None;
    }
    Some((s.BatteryLifePercent, s.ACLineStatus == 1))
}

fn beep() {
    unsafe { windows_sys::Win32::System::Diagnostics::Debug::MessageBeep(0) };
}

fn set_startup(on: bool) {
    use std::os::windows::process::CommandExt;
    let key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    let mut c = std::process::Command::new("reg");
    if on {
        let exe = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default();
        c.args(["add", key, "/v", "MyNotch", "/t", "REG_SZ", "/d", &format!("\"{exe}\""), "/f"]);
    } else {
        c.args(["delete", key, "/v", "MyNotch", "/f"]);
    }
    let _ = c.creation_flags(0x0800_0000).status(); // CREATE_NO_WINDOW
}

#[cfg(test)]
mod tests {
    use super::*;
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
