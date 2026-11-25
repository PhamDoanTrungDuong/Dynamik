//! Made-up content for the --demo screenshots (never shown in normal use).
use super::*;

/// `--page 0 --demo` on a screenshot: a made-up song with a made-up cover (a violet-to-orange
/// square), to check the Music page without anyone's real music.
pub(crate) fn demo_music(m: media::Media, shot_music: bool) -> media::Media {
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
            can_next: true,
            can_prev: true,
            can_seek: true,
            ..Default::default()
        };
    }
    m
}

pub(crate) const DEMO_ANSWER: &str = "Sunlight is made of every colour. Air scatters short blue waves far more than red ones, \
    so blue light reaches your eyes from all over the sky. At sunset the light crosses more air, and the reds win.";

/// Made-up notifications for checking the Today page's look (`--page 7 --demo`).
pub(crate) fn demo_recent() -> Vec<(i64, media::Notice)> {
    let now = Utc::now().timestamp();
    [
        ("Messenger", "Lan", "See you at 7"),
        ("Zalo", "Nhóm: TRI THỨC CHỈ ĐẠO CHUNG VÀ PHỐI HỢP", "Anh Tám: Mai họp lúc 9h nhé mọi người"),
        ("Outlook", "Weekly report", "Attached is this week's report"),
        ("Zalo", "Minh", "Ok"),
        ("Zalo", "Nhóm: TRI THỨC CHỈ ĐẠO CHUNG VÀ PHỐI HỢP", "Chị Hoa: Đã gửi file"),
        ("Teams", "Standup", "Starting in 5 minutes"),
    ]
        .iter()
        .enumerate()
        .map(|(k, (a, t, b))| (now - k as i64 * 400, (a.to_string(), t.to_string(), b.to_string(), String::new())))
        .collect()
}

/// `--popup bluetooth|notice|brightness` with --screenshot --resting shows that pop-up, for checking its look.
/// Notes for the --page 8 --demo screenshot: one pinned, two open, one done.
pub(crate) fn demo_notes() -> Vec<Note> {
    let now = Utc::now().timestamp();
    let today = Local::now().date_naive();
    let mut n = vec![
        Note { created: now - 7200, text: "Gọi lại anh Tám về hợp đồng bản 2.0".into(), pinned: true, done: None },
        Note { created: now - 600, text: "Họp 9h mai: chốt giá và lịch giao hàng".into(), pinned: false, done: None },
        Note { created: now - 3600, text: "Link tài liệu: drive.google.com/notch-spec".into(), pinned: false, done: None },
        Note { created: now - 9000, text: "Gửi báo cáo tuần cho chị Lan".into(), pinned: false, done: Some(today) },
    ];
    tidy_notes(&mut n, today, 3);
    n
}

pub(crate) fn demo_alert() -> Option<Alert> {
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
        // With --demo, the made-up song's cover (demo_music) shows too.
        "song" => ("Nơi này có anh (Lofi Version) — bản phối dài cho buổi chiều", "Sơn Tùng M-TP", Kind::Song),
        "taken" => return Some(taken_hint(tray::TAKEN_ASK | tray::TAKEN_OCR | tray::TAKEN_NOTE)),
        _ => return None,
    };
    Some(Alert { title: title.into(), body: body.into(), kind, at: Instant::now() })
}
