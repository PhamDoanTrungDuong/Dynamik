//! Music: what Windows is playing (the same media session API the original uses), play/pause and
//! skip, and synced lyrics from LRCLIB, all on a background thread.
use std::{
    sync::{mpsc, Arc, Mutex},
    time::{Duration, Instant},
};
use windows::{
    core::HSTRING,
    Foundation::Uri,
    Media::Control::{GlobalSystemMediaTransportControlsSessionManager as Manager, GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status},
    Web::Http::HttpClient,
};

#[derive(Clone, Default)]
pub struct Now {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub playing: bool,
    pub position: f64, // seconds, at `read_at`
    pub duration: f64,
    pub read_at: Option<Instant>,
    pub lyrics: Vec<(f64, String)>, // (seconds, line), empty when none were found
    pub lyrics_state: LyricsState,
    pub art: Option<Logo>, // the cover, when the player gives one
    pub art_key: String,   // which song `art` belongs to
    pub tint: Option<[u8; 3]>, // the cover's main colour, made bright enough for a dark notch
    pub app_id: String,    // the player (its logo comes from `logo`)
    pub app_name: String,  // the player's name, e.g. "Spotify"
    pub players: usize,    // how many players Windows knows now (more than one: they can be switched)
    // What the player allows: a single browser video, say, has no next or previous track.
    pub can_next: bool,
    pub can_prev: bool,
    pub can_seek: bool,
}

/// A player's name: what Windows calls the app, else the readable part of its id
/// ("SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify" or "Spotify.exe" both give "Spotify").
fn app_name(id: &str) -> String {
    let known = windows::ApplicationModel::AppInfo::GetFromAppUserModelId(&id.into())
        .and_then(|a| a.DisplayInfo())
        .and_then(|d| d.DisplayName())
        .map(|n| n.to_string())
        .unwrap_or_default();
    if !known.is_empty() {
        return known;
    }
    let tail = id.rsplit('!').next().unwrap_or(id);
    let tail = tail.strip_suffix(".exe").or_else(|| tail.strip_suffix(".EXE")).unwrap_or(tail);
    tail.rsplit(['\\', '/']).next().unwrap_or(tail).to_string()
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum LyricsState { #[default] None, Loading, Found, Missing }

impl Now {
    pub fn active(&self) -> bool {
        !self.title.is_empty()
    }
    /// Position now, moving on while it plays.
    pub fn pos(&self) -> f64 {
        let moved = if self.playing { self.read_at.map(|t| t.elapsed().as_secs_f64()).unwrap_or(0.0) } else { 0.0 };
        (self.position + moved).min(self.duration.max(self.position))
    }
    /// Index of the lyric line being sung.
    pub fn line(&self) -> Option<usize> {
        let p = self.pos();
        self.lyrics.iter().rposition(|(t, _)| *t <= p)
    }
}

pub enum Cmd { Toggle, Next, Prev, Pause, Play, Seek(f64), NextPlayer }

const ART_TRIES: u32 = 4; // more reads of a song's cover while the player hasn't sent it yet

/// What wakes the media thread: a button press, or Windows saying the session or song changed.
enum Msg { Cmd(Cmd), Changed }

pub struct Media {
    pub now: Arc<Mutex<Now>>,
    tx: mpsc::Sender<Msg>,
}

impl Media {
    /// `live` false keeps it empty (screenshots shouldn't show what the person is playing).
    pub fn start(want_lyrics: bool, live: bool) -> Self {
        let now = Arc::new(Mutex::new(Now::default()));
        let (tx, rx) = mpsc::channel();
        let shared = now.clone();
        if !live {
            return Media { now, tx };
        }
        let events = tx.clone();
        std::thread::spawn(move || {
            use windows::Foundation::TypedEventHandler;
            unsafe {
                let _ = windows::Win32::System::WinRT::RoInitialize(windows::Win32::System::WinRT::RO_INIT_MULTITHREADED);
            }
            let Ok(manager) = Manager::RequestAsync().and_then(|op| op.get()) else { return };
            // Waits instead of checking: Windows says when the session, the song, play/pause or the
            // position changes. Each event (or button press) wakes this thread for one read.
            let e = events.clone();
            let _ = manager.CurrentSessionChanged(&TypedEventHandler::new(move |_, _| {
                let _ = e.send(Msg::Changed);
                Ok(())
            }));
            let e = events.clone();
            let _ = manager.SessionsChanged(&TypedEventHandler::new(move |_, _| {
                let _ = e.send(Msg::Changed);
                Ok(())
            }));
            // The player picked with "switch player", until it goes away (or stays paused while
            // another one plays); None follows Windows' own choice.
            let mut chosen: Option<String> = None;
            let mut watched: Option<(String, windows::Media::Control::GlobalSystemMediaTransportControlsSession, [windows::Foundation::EventRegistrationToken; 3])> = None;
            let mut last_track = String::new();
            let mut art_tries = 0;
            let mut player = (String::new(), String::new()); // (app id, name) of the last player seen
            let mut pending: Option<Msg> = None;
            loop {
                let cmds: Vec<Cmd> = pending.take().into_iter().chain(rx.try_iter()).filter_map(|m| if let Msg::Cmd(c) = m { Some(c) } else { None }).collect();
                let sessions: Vec<_> = manager.GetSessions().map(|l| l.into_iter().collect()).unwrap_or_default();
                let id_of = |s: &windows::Media::Control::GlobalSystemMediaTransportControlsSession| s.SourceAppUserModelId().map(|a| a.to_string()).unwrap_or_default();
                let playing = |s: &windows::Media::Control::GlobalSystemMediaTransportControlsSession| {
                    s.GetPlaybackInfo().and_then(|i| i.PlaybackStatus()).is_ok_and(|st| st == Status::Playing)
                };
                let windows_pick = manager.GetCurrentSession().ok();
                let mut current = chosen.as_ref().and_then(|c| sessions.iter().find(|s| id_of(s) == *c).cloned());
                // Drop the pick when it's gone, or paused while Windows' pick plays something else.
                if current.is_none() || current.as_ref().is_some_and(|c| !playing(c) && windows_pick.as_ref().is_some_and(|w| id_of(w) != id_of(c) && playing(w))) {
                    chosen = None;
                    current = windows_pick;
                }
                if cmds.iter().any(|c| matches!(c, Cmd::NextPlayer)) && sessions.len() > 1 {
                    // The next player after the one shown, in Windows' order.
                    let at = current.as_ref().and_then(|c| sessions.iter().position(|s| id_of(s) == id_of(c))).unwrap_or(0);
                    let next = sessions[(at + 1) % sessions.len()].clone();
                    chosen = Some(id_of(&next));
                    current = Some(next);
                }
                let app = current.as_ref().and_then(|s| s.SourceAppUserModelId().ok()).map(|a| a.to_string());
                if app != watched.as_ref().map(|w| w.0.clone()) {
                    if let Some((_, old, [a, b, c])) = watched.take() {
                        let _ = (old.RemoveMediaPropertiesChanged(a), old.RemovePlaybackInfoChanged(b), old.RemoveTimelinePropertiesChanged(c));
                    }
                    if let (Some(app), Some(s)) = (app, current.as_ref()) {
                        let (e1, e2, e3) = (events.clone(), events.clone(), events.clone());
                        let tokens = (
                            s.MediaPropertiesChanged(&TypedEventHandler::new(move |_, _| { let _ = e1.send(Msg::Changed); Ok(()) })),
                            s.PlaybackInfoChanged(&TypedEventHandler::new(move |_, _| { let _ = e2.send(Msg::Changed); Ok(()) })),
                            s.TimelinePropertiesChanged(&TypedEventHandler::new(move |_, _| { let _ = e3.send(Msg::Changed); Ok(()) })),
                        );
                        if let (Ok(a), Ok(b), Ok(c)) = tokens {
                            watched = Some((app, s.clone(), [a, b, c]));
                        }
                    }
                }
                if let Some(session) = current {
                    for cmd in cmds {
                        let _ = match cmd {
                            Cmd::Toggle => session.TryTogglePlayPauseAsync().and_then(|o| o.get()),
                            Cmd::Next => session.TrySkipNextAsync().and_then(|o| o.get()),
                            Cmd::Prev => session.TrySkipPreviousAsync().and_then(|o| o.get()),
                            Cmd::Pause => session.TryPauseAsync().and_then(|o| o.get()),
                            Cmd::Play => session.TryPlayAsync().and_then(|o| o.get()),
                            // The position is in 100 ns ticks.
                            Cmd::Seek(s) => session.TryChangePlaybackPositionAsync((s * 1e7) as i64).and_then(|o| o.get()),
                            Cmd::NextPlayer => Ok(true), // done above, before choosing the session
                        };
                    }
                    let mut n = read(&session).unwrap_or_default();
                    // Which app plays it: its name and logo, looked up once per app.
                    n.app_id = session.SourceAppUserModelId().map(|a| a.to_string()).unwrap_or_default();
                    if player.0 != n.app_id {
                        player = (n.app_id.clone(), app_name(&n.app_id));
                        let known = LOGOS.lock().unwrap().as_ref().is_some_and(|m| m.contains_key(&n.app_id));
                        if !known && !n.app_id.is_empty() {
                            preload_logo(&n.app_id);
                        }
                    }
                    n.app_name = player.1.clone();
                    n.players = sessions.len();
                    let key = format!("{}\u{1}{}", n.title, n.artist);
                    let mut fetch = None;
                    // The cover: read once per song. Some players (Spotify) send it a moment after
                    // the title, so a song without one tries again on its next few changes.
                    let had = shared.lock().unwrap().art.clone().filter(|_| key == last_track);
                    if had.is_some() {
                        n.art = had;
                    } else if key != last_track || art_tries < ART_TRIES {
                        art_tries = if key == last_track { art_tries + 1 } else { 0 };
                        n.art = read_art(&session);
                    }
                    n.art_key = key.clone();
                    n.tint = n.art.as_deref().and_then(|a| cover_tint(&a.2));
                    {
                        let mut s = shared.lock().unwrap();
                        let art_came = s.art.is_none() && n.art.is_some();
                        if key == last_track {
                            n.lyrics = std::mem::take(&mut s.lyrics);
                            n.lyrics_state = s.lyrics_state;
                            if art_came {
                                crate::tray::wake();
                            }
                        } else {
                            last_track = key;
                            if want_lyrics && n.active() {
                                n.lyrics_state = LyricsState::Loading;
                                fetch = Some((n.title.clone(), n.artist.clone(), n.album.clone(), n.duration));
                            }
                        }
                        // A new song, play/pause or a jump in position (seek from the player) is worth a frame.
                        let jumped = (n.pos() - s.pos()).abs() > 2.0;
                        let changed = s.title != n.title || s.artist != n.artist || s.playing != n.playing || s.lyrics_state != n.lyrics_state || jumped;
                        *s = n;
                        if changed {
                            crate::tray::wake();
                        }
                    }
                    if let Some((t, a, al, d)) = fetch {
                        let found = lrclib(&t, &a, &al, d).unwrap_or_default();
                        let mut s = shared.lock().unwrap();
                        if s.title == t {
                            s.lyrics_state = if found.is_empty() { LyricsState::Missing } else { LyricsState::Found };
                            s.lyrics = found;
                            crate::tray::wake();
                        }
                    }
                } else {
                    // Nothing to send them to: dropped (cmds above), so a pause from prayer time can't hit music started later.
                    let mut s = shared.lock().unwrap();
                    if s.active() {
                        crate::tray::wake();
                    }
                    *s = Now::default();
                    last_track.clear();
                }
                // shortcut: a missed event is caught by this 30 s look; Windows has not been seen to miss one.
                pending = match rx.recv_timeout(Duration::from_secs(30)) {
                    Ok(m) => Some(m),
                    Err(mpsc::RecvTimeoutError::Timeout) => None,
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                };
            }
        });
        Media { now, tx }
    }

    pub fn send(&self, c: Cmd) {
        if let Cmd::Seek(s) = c {
            // Show the new spot at once; the read that follows confirms it or puts it back.
            let mut n = self.now.lock().unwrap();
            n.position = if n.duration > 0.0 { s.clamp(0.0, n.duration) } else { s.max(0.0) };
            n.read_at = Some(Instant::now());
        }
        let _ = self.tx.send(Msg::Cmd(c));
    }
}

/// Keeps a watcher's new value and asks for a frame only when it differs from the last.
fn store<T: PartialEq>(shared: &Mutex<T>, value: T) {
    let mut v = shared.lock().unwrap();
    if *v != value {
        *v = value;
        crate::tray::wake();
    }
}

type Volume = Arc<Mutex<Option<(f32, bool)>>>;

/// Windows calls this on every volume or mute change of the speakers it is registered on.
#[windows::core::implement(windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolumeCallback)]
struct VolumeChanged(Volume);

impl windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolumeCallback_Impl for VolumeChanged_Impl {
    fn OnNotify(&self, data: *mut windows::Win32::Media::Audio::AUDIO_VOLUME_NOTIFICATION_DATA) -> windows::core::Result<()> {
        if let Some(d) = unsafe { data.as_ref() } {
            store(&self.0, Some((d.fMasterVolume, d.bMuted.as_bool())));
        }
        Ok(())
    }
}

/// The speakers' volume (0..1) and mute. Windows reports changes (no polling); the default
/// speakers are looked up again every 5 s, as the person may switch them.
pub fn volume_watch() -> Volume {
    use windows::Win32::{
        Media::Audio::{eConsole, eRender, Endpoints::{IAudioEndpointVolume, IAudioEndpointVolumeCallback}, IMMDeviceEnumerator, MMDeviceEnumerator},
        System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED},
    };
    let shared: Volume = Arc::new(Mutex::new(None));
    let out = shared.clone();
    std::thread::spawn(move || unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let Ok(devices) = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL) else { return };
        let callback: IAudioEndpointVolumeCallback = VolumeChanged(shared.clone()).into();
        let mut current: Option<(String, IAudioEndpointVolume)> = None;
        loop {
            let device = devices.GetDefaultAudioEndpoint(eRender, eConsole).ok();
            let id = device.as_ref().and_then(|d| d.GetId().ok()).and_then(|p| p.to_string().ok());
            if id != current.as_ref().map(|c| c.0.clone()) {
                if let Some((_, old)) = current.take() {
                    let _ = old.UnregisterControlChangeNotify(&callback);
                }
                let ep = device.and_then(|d| d.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None).ok());
                // The value now, then changes as Windows reports them.
                store(&shared, ep.as_ref().and_then(|e| Some((e.GetMasterVolumeLevelScalar().ok()?, e.GetMute().ok()?.as_bool()))));
                if let (Some(id), Some(ep)) = (id, ep) {
                    if ep.RegisterControlChangeNotify(&callback).is_ok() {
                        current = Some((id, ep));
                    }
                }
            }
            std::thread::sleep(Duration::from_secs(5));
        }
    });
    out
}

/// The network: ("Wi-Fi" or "Ethernet", 0 none / 1 no internet / 2 connected), read again
/// each time Windows reports a network status change.
pub fn network_watch() -> Arc<Mutex<Option<(String, u8)>>> {
    use windows::Networking::Connectivity::{NetworkConnectivityLevel as Level, NetworkInformation, NetworkStatusChangedEventHandler};
    let shared = Arc::new(Mutex::new(None));
    let out = shared.clone();
    let (tx, rx) = mpsc::channel::<()>();
    let _ = NetworkInformation::NetworkStatusChanged(&NetworkStatusChangedEventHandler::new(move |_| {
        let _ = tx.send(());
        Ok(())
    }));
    std::thread::spawn(move || loop {
        let now = match NetworkInformation::GetInternetConnectionProfile() {
            Ok(p) => {
                let kind = if p.IsWlanConnectionProfile().unwrap_or(false) { "Wi-Fi" } else { "Ethernet" };
                let level = match p.GetNetworkConnectivityLevel() {
                    Ok(Level::InternetAccess) => 2,
                    Ok(Level::None) | Err(_) => 0,
                    Ok(_) => 1,
                };
                (kind.to_string(), level)
            }
            Err(_) => ("Wi-Fi".to_string(), 0),
        };
        store(&shared, Some(now));
        // shortcut: also looks every 60 s in case an event is missed; none has been seen missing.
        let _ = rx.recv_timeout(Duration::from_secs(60));
        while rx.try_recv().is_ok() {}
    });
    out
}

fn read(s: &windows::Media::Control::GlobalSystemMediaTransportControlsSession) -> windows::core::Result<Now> {
    let props = s.TryGetMediaPropertiesAsync()?.get()?;
    let tl = s.GetTimelineProperties()?;
    let info = s.GetPlaybackInfo()?;
    let playing = info.PlaybackStatus()? == Status::Playing;
    let controls = info.Controls()?;
    let mut position = tl.Position()?.Duration as f64 / 1e7;
    // Windows reports the position as of its last update; add the time since then while playing.
    if playing {
        let updated = tl.LastUpdatedTime()?.UniversalTime; // 100 ns since 1601
        let now = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs_f64() + 11_644_473_600.0) * 1e7;
        if updated > 0 {
            position += ((now - updated as f64) / 1e7).clamp(0.0, 3600.0);
        }
    }
    Ok(Now {
        title: props.Title()?.to_string(),
        artist: props.Artist()?.to_string(),
        album: props.AlbumTitle()?.to_string(),
        playing,
        position,
        duration: tl.EndTime()?.Duration as f64 / 1e7,
        read_at: Some(Instant::now()),
        can_next: controls.IsNextEnabled().unwrap_or(true),
        can_prev: controls.IsPreviousEnabled().unwrap_or(true),
        can_seek: controls.IsPlaybackPositionEnabled().unwrap_or(true),
        ..Default::default()
    })
}

/// LRCLIB lookup as the original does it (0x14000de39): search by track and artist, else by
/// "title artist"; keep answers with synced lyrics and take the one closest in duration.
fn lrclib(title: &str, artist: &str, _album: &str, duration: f64) -> Option<Vec<(f64, String)>> {
    let title = clean_title(title);
    let client = HttpClient::new().ok()?;
    let _ = client.DefaultRequestHeaders().ok()?.UserAgent().ok()?.TryParseAdd(&HSTRING::from("MyNotch/0.1 (personal notch app)"));
    let get = |query: String| -> Option<String> {
        let uri = Uri::CreateUri(&HSTRING::from(format!("https://lrclib.net/api/search?{query}"))).ok()?;
        Some(client.GetStringAsync(&uri).ok()?.get().ok()?.to_string())
    };
    let queries = [
        format!("track_name={}&artist_name={}", enc(title), enc(artist)),
        format!("q={}", enc(&format!("{title} {artist}"))),
    ];
    queries.into_iter().find_map(|q| best_synced(&get(q)?, duration)).map(|lrc| parse_lrc(&lrc))
}

/// The title without " (…)", " […]" or " - …" tails (the cut marks in the original's data at 0x1400e1d9c).
pub fn clean_title(t: &str) -> &str {
    let cut = [" (", " [", " - "].iter().filter_map(|m| t.find(m)).min().unwrap_or(t.len());
    t[..cut].trim()
}

/// From a search answer (a list of objects), the synced lyrics whose "duration" is nearest the
/// song's; with no known song length the first one wins.
pub fn best_synced(body: &str, duration: f64) -> Option<String> {
    body.split("{\"id\":")
        .skip(1)
        .filter_map(|obj| {
            let lyrics = json_string(obj, "syncedLyrics")?;
            let d = json_number(obj, "duration").unwrap_or(0.0);
            let miss = if duration > 0.0 { (d - duration).abs() } else { 0.0 };
            Some((miss, lyrics))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, l)| l)
}

pub fn json_number(body: &str, key: &str) -> Option<f64> {
    let start = body.find(&format!("\"{key}\""))?;
    let rest = body[start + key.len() + 2..].trim_start().strip_prefix(':')?.trim_start();
    let end = rest.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-')).unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// The value of a string field in a JSON object (enough for LRCLIB's flat answer).
pub fn json_string(body: &str, key: &str) -> Option<String> {
    let start = body.find(&format!("\"{key}\""))?;
    let rest = body[start + key.len() + 2..].trim_start().strip_prefix(':')?.trim_start();
    let mut chars = rest.strip_prefix('"')?.chars();
    let mut out = String::new();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => {}
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    out.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?).unwrap_or('?'));
                }
                o => out.push(o),
            },
            c => out.push(c),
        }
    }
    None
}

/// "[mm:ss.xx] words" lines into (seconds, words).
/// An [offset:ms] tag moves every line (the original reads it, 0x14000fa88); positive = earlier,
/// as the LRC format defines it.
pub fn parse_lrc(text: &str) -> Vec<(f64, String)> {
    let mut offset = 0.0;
    let mut out = vec![];
    for l in text.lines() {
        let Some((stamp, words)) = l.trim().strip_prefix('[').and_then(|l| l.split_once(']')) else { continue };
        let Some((m, s)) = stamp.split_once(':') else { continue };
        if m.eq_ignore_ascii_case("offset") {
            offset = s.trim().parse::<f64>().unwrap_or(0.0) / 1000.0;
        } else if let (Ok(m), Ok(s)) = (m.parse::<f64>(), s.parse::<f64>()) {
            out.push((m * 60.0 + s, words.trim().to_string()));
        }
    }
    out.iter_mut().for_each(|l| l.0 = (l.0 - offset).max(0.0));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lyrics_parse() {
        let body = r#"{"id":1,"syncedLyrics":"[00:01.50] hello\n[01:02.00] wor\"ld","plainLyrics":null}"#;
        let l = parse_lrc(&json_string(body, "syncedLyrics").unwrap());
        assert_eq!(l, vec![(1.5, "hello".to_string()), (62.0, "wor\"ld".to_string())]);
        assert_eq!(enc("a b&c"), "a%20b%26c");
        assert_eq!(parse_lrc("[offset:+500]\n[00:02.00] x"), vec![(1.5, "x".to_string())]);
    }
    #[test]
    fn search_picks_closest_duration() {
        let body = r#"[{"id":1,"duration":200.0,"syncedLyrics":"[00:01.00] far"},{"id":2,"duration":181,"syncedLyrics":"[00:01.00] near"},{"id":3,"duration":180,"syncedLyrics":null}]"#;
        assert_eq!(best_synced(body, 180.0).as_deref(), Some("[00:01.00] near"));
        assert_eq!(clean_title("Paper Sky (Remastered 2020) - Live"), "Paper Sky");
    }
}

/// A connected Bluetooth headset: address, name and battery percent when the device reports one.
/// A connected device with a battery or a kind worth showing: (id, name, battery %, kind).
pub type Device = (u64, String, Option<u8>, Gadget);

/// What a device is, for its icon and words.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Gadget {
    Headset,
    Mouse,
    Keyboard,
    Controller,
    Other,
}

/// The kind from a Bluetooth class of device: major class 4 is audio/video, 5 a peripheral whose
/// minor bits say keyboard (01), pointer (10) or both (11), with joystick / gamepad subtypes 1 / 2.
pub fn gadget_of(cod: u32) -> Gadget {
    let minor = (cod >> 2) & 0x3f;
    match (cod >> 8) & 0x1f {
        4 => Gadget::Headset,
        5 if matches!(minor & 0x0f, 1 | 2) => Gadget::Controller,
        5 if minor & 0x30 == 0x20 => Gadget::Mouse,
        5 if minor & 0x30 != 0 => Gadget::Keyboard,
        _ => Gadget::Other,
    }
}

/// Ids for Xbox controllers (slots 0-3), above any 48-bit Bluetooth address.
const XINPUT_ID: u64 = 1 << 56;

/// An Xbox controller's battery in slot 0-3, from XInput's four levels; None for no controller or
/// a wired one (which reports no battery).
fn xinput_battery(slot: u32) -> Option<u8> {
    use windows_sys::Win32::UI::Input::XboxController::{XInputGetBatteryInformation, BATTERY_DEVTYPE_GAMEPAD, BATTERY_TYPE_ALKALINE, BATTERY_TYPE_NIMH, XINPUT_BATTERY_INFORMATION};
    let mut info: XINPUT_BATTERY_INFORMATION = unsafe { std::mem::zeroed() };
    if unsafe { XInputGetBatteryInformation(slot, BATTERY_DEVTYPE_GAMEPAD, &mut info) } != 0 {
        return None;
    }
    matches!(info.BatteryType, BATTERY_TYPE_ALKALINE | BATTERY_TYPE_NIMH).then(|| xinput_percent(info.BatteryLevel))
}

/// XInput's Empty / Low / Medium / Full as a percentage.
fn xinput_percent(level: u8) -> u8 {
    [10, 40, 70, 100][level.min(3) as usize]
}

/// Connected Bluetooth devices (headsets, mice, keyboards, controllers, and anything else that
/// reports a battery) and Xbox controllers. Like the original: device ids from cfgmgr32 under
/// BTHENUM, the battery from DEVPKEY {104EA319-...} 2, and the connection state, name and class from
/// Windows.Devices.Bluetooth.BluetoothDevice, whose ConnectionStatusChanged wakes the look.
pub fn bluetooth_watch() -> Arc<Mutex<Option<Vec<Device>>>> {
    use windows::{Devices::Bluetooth::{BluetoothConnectionStatus, BluetoothDevice}, Foundation::TypedEventHandler};
    let shared = Arc::new(Mutex::new(None));
    let out = shared.clone();
    let (tx, rx) = mpsc::channel::<()>();
    std::thread::spawn(move || {
        let mut devices: std::collections::HashMap<u64, BluetoothDevice> = Default::default();
        // When each headset was first seen connected: its battery level often appears a few seconds
        // later, so a new headset is reported once it has one, or after 5 s without.
        let mut since: std::collections::HashMap<u64, Instant> = Default::default();
        loop {
            let Some(nodes) = bth_nodes() else {
                let _ = rx.recv_timeout(Duration::from_secs(5));
                continue;
            };
            let mut now = vec![];
            for addr in nodes.iter().map(|n| n.0).collect::<std::collections::BTreeSet<_>>() {
                if !devices.contains_key(&addr) {
                    // Not cached on failure (radio off, stack not ready): tried again next round.
                    let Ok(dev) = BluetoothDevice::FromBluetoothAddressAsync(addr).and_then(|op| op.get()) else { continue };
                    let t = tx.clone();
                    let _ = dev.ConnectionStatusChanged(&TypedEventHandler::new(move |_, _| {
                        let _ = t.send(());
                        Ok(())
                    }));
                    devices.insert(addr, dev);
                }
                let dev = &devices[&addr];
                let kind = dev.ClassOfDevice().and_then(|c| c.RawValue()).map_or(Gadget::Other, gadget_of);
                let battery = nodes.iter().filter(|n| n.0 == addr).find_map(|n| n.1);
                // Phones, PCs and the like only when they report a battery.
                let shown = kind != Gadget::Other || battery.is_some();
                if !(shown && dev.ConnectionStatus().is_ok_and(|s| s == BluetoothConnectionStatus::Connected)) {
                    since.remove(&addr);
                    continue;
                }
                if battery.is_some() || since.entry(addr).or_insert_with(Instant::now).elapsed() >= Duration::from_secs(5) {
                    let name = dev.Name().map(|n| n.to_string()).unwrap_or_else(|_| "Bluetooth".into());
                    now.push((addr, name, battery, kind));
                }
            }
            // shortcut: Xbox controllers are looked at with the Bluetooth list (a change, or every
            // 60 s), so one switched on can take up to a minute to show; XInput sends no events.
            for slot in 0..4u32 {
                if let Some(b) = xinput_battery(slot) {
                    now.push((XINPUT_ID + slot as u64, crate::i18n::tr("Xbox controller {}", &[&(slot + 1)]), Some(b), Gadget::Controller));
                }
            }
            let waiting = since.keys().any(|a| !now.iter().any(|h| h.0 == *a));
            store(&shared, Some(now));
            // A headset that just connected is looked at each second until its battery shows (up to 5 s);
            // otherwise the thread waits for a connection change. shortcut: newly paired devices and
            // battery levels are picked up by a look every 60 s.
            let _ = rx.recv_timeout(Duration::from_secs(if waiting { 1 } else { 60 }));
            while rx.try_recv().is_ok() {}
        }
    });
    out
}

/// Present BTHENUM device nodes as (Bluetooth address, battery percent if the node has one).
/// None when the list can't be read, so a failed look isn't taken for "nothing connected".
fn bth_nodes() -> Option<Vec<(u64, Option<u8>)>> {
    use windows_sys::Win32::Devices::{DeviceAndDriverInstallation::*, Properties::DEVPROPKEY};
    const BATTERY: DEVPROPKEY = DEVPROPKEY {
        fmtid: windows_sys::core::GUID::from_u128(0x104ea319_6ee2_4701_bd47_8ddbf425bbe5),
        pid: 2,
    };
    let filter: Vec<u16> = "BTHENUM\0".encode_utf16().collect();
    let flags = CM_GETIDLIST_FILTER_ENUMERATOR | CM_GETIDLIST_FILTER_PRESENT;
    let mut len = 0u32;
    if unsafe { CM_Get_Device_ID_List_SizeW(&mut len, filter.as_ptr(), flags) } != CR_SUCCESS {
        return None;
    }
    if len <= 1 {
        return Some(vec![]); // just the closing NUL: no Bluetooth devices
    }
    let mut buf = vec![0u16; len as usize];
    if unsafe { CM_Get_Device_ID_ListW(filter.as_ptr(), buf.as_mut_ptr(), len, flags) } != CR_SUCCESS {
        return None;
    }
    Some(buf.split(|&c| c == 0).filter(|id| !id.is_empty()).filter_map(|id| {
        let addr = bth_address(&String::from_utf16_lossy(id))?;
        let mut wid = id.to_vec();
        wid.push(0);
        let mut node = 0u32;
        let mut battery = None;
        if unsafe { CM_Locate_DevNodeW(&mut node, wid.as_ptr(), CM_LOCATE_DEVNODE_NORMAL) } == CR_SUCCESS {
            let (mut ty, mut b, mut size) = (0u32, 0u8, 1u32);
            if unsafe { CM_Get_DevNode_PropertyW(node, &BATTERY, &mut ty, &mut b, &mut size, 0) } == CR_SUCCESS && b <= 100 {
                battery = Some(b);
            }
        }
        Some((addr, battery))
    }).collect())
}

/// The device address in a BTHENUM instance id: `BTHENUM\{uuid}_...\7&..&0&AABBCCDDEEFF_C00000000`
/// or `BTHENUM\DEV_AABBCCDDEEFF\...`.
fn bth_address(id: &str) -> Option<u64> {
    let parts: Vec<&str> = id.split('\\').collect();
    let hex = if let Some(d) = parts.get(1).filter(|p| p.len() == 16 && p.get(..4).is_some_and(|s| s.eq_ignore_ascii_case("dev_"))) {
        &d[4..]
    } else {
        let last = parts.last()?;
        last.rsplit('&').next()?.split('_').next()?
    };
    (hex.len() == 12).then(|| u64::from_str_radix(hex, 16).ok()).flatten().filter(|&a| a != 0)
}

/// Where this PC is, from Windows' location service; None when location is off or denied.
/// Blocks for a few seconds, so call it from its own thread.
pub fn locate() -> Option<(f64, f64)> {
    // WinRT needs this once on each thread that uses it; again is harmless.
    let _ = unsafe { windows::Win32::System::WinRT::RoInitialize(windows::Win32::System::WinRT::RO_INIT_MULTITHREADED) };
    use windows::Devices::Geolocation::{GeolocationAccessStatus, Geolocator};
    let access = Geolocator::RequestAccessAsync().ok()?.get().ok()?;
    if access != GeolocationAccessStatus::Allowed {
        return None;
    }
    // A fix up to 10 minutes old is fine; give up after 15 s so the button never stays stuck.
    let ticks = |s: i64| windows::Foundation::TimeSpan { Duration: s * 10_000_000 };
    let pos = Geolocator::new().ok()?.GetGeopositionAsyncWithAgeAndTimeout(ticks(600), ticks(15)).ok()?.get().ok()?.Coordinate().ok()?.Point().ok()?.Position().ok()?;
    Some((pos.Latitude, pos.Longitude))
}

/// A new Windows notification: app name, title, body text and the app's id (to open it).
pub type Notice = (String, String, String, String);

/// An app's logo for its notifications: (width, height, RGBA pixels).
pub type Logo = Arc<(usize, usize, Vec<u8>)>;
pub const LOGO_PX: u32 = 48; // twice the drawn size, so it stays sharp at 150% scaling

// Logos by app id, read once per app; None = the app has no logo (or it couldn't be read).
static LOGOS: Mutex<Option<std::collections::HashMap<String, Option<Logo>>>> = Mutex::new(None);

/// The app's logo, once the notification watcher has read it.
pub fn logo(id: &str) -> Option<Logo> {
    LOGOS.lock().unwrap().as_ref()?.get(id).cloned().flatten()
}

/// Reads an installed app's logo by its id ahead of time (for screenshots, where no watcher runs).
pub fn preload_logo(id: &str) {
    let logo = windows::ApplicationModel::AppInfo::GetFromAppUserModelId(&id.into()).ok().and_then(|a| read_logo(&a));
    LOGOS.lock().unwrap().get_or_insert_with(Default::default).insert(id.to_string(), logo);
}

/// A picture Windows hands over as a stream (an app logo, a song's cover), decoded.
fn read_image(r: &impl windows::core::Interface) -> Option<image::RgbaImage> {
    use windows::Storage::Streams::{DataReader, IRandomAccessStreamReference};
    let stream = r.cast::<IRandomAccessStreamReference>().ok()?.OpenReadAsync().ok()?.get().ok()?;
    let len = stream.Size().ok()? as u32;
    if len == 0 || len > 8 << 20 {
        return None;
    }
    let reader = DataReader::CreateDataReader(&stream).ok()?;
    reader.LoadAsync(len).ok()?.get().ok()?;
    let mut bytes = vec![0u8; len as usize];
    reader.ReadBytes(&mut bytes).ok()?;
    Some(image::load_from_memory(&bytes).ok()?.to_rgba8())
}

/// A cover's main colour: the average of its colourful pixels (grey, near-black and near-white
/// ones count little), lifted so its brightest part is 230. None for a grey cover.
pub fn cover_tint(rgba: &[u8]) -> Option<[u8; 3]> {
    let (mut sum, mut weight) = ([0.0f32; 3], 0.0f32);
    for p in rgba.chunks_exact(4) {
        let (r, g, b) = (p[0] as f32, p[1] as f32, p[2] as f32);
        let (hi, lo) = (r.max(g).max(b), r.min(g).min(b));
        if hi < 40.0 || p[3] < 128 {
            continue;
        }
        let sat = (hi - lo) / hi;
        let w = sat * sat;
        sum = [sum[0] + r * w, sum[1] + g * w, sum[2] + b * w];
        weight += w;
    }
    if weight < rgba.len() as f32 / 4.0 * 0.02 {
        return None; // mostly grey
    }
    let avg = sum.map(|c| c / weight);
    let lift = 230.0 / avg[0].max(avg[1]).max(avg[2]).max(1.0);
    Some(avg.map(|c| (c * lift).min(255.0) as u8))
}

pub const ART_PX: u32 = 156; // the cover is drawn at 78 points; twice that stays sharp at 150-200%

/// The playing song's cover, cut to a centred square. None when the player gives none.
fn read_art(s: &windows::Media::Control::GlobalSystemMediaTransportControlsSession) -> Option<Logo> {
    let thumb = s.TryGetMediaPropertiesAsync().ok()?.get().ok()?.Thumbnail().ok()?;
    let img = read_image(&thumb)?;
    let (w, h) = img.dimensions();
    let side = w.min(h);
    let square = image::imageops::crop_imm(&img, (w - side) / 2, (h - side) / 2, side, side).to_image();
    let img = image::imageops::resize(&square, ART_PX, ART_PX, image::imageops::FilterType::Triangle);
    Some(Arc::new((ART_PX as usize, ART_PX as usize, img.into_raw())))
}

fn read_logo(info: &windows::ApplicationModel::AppInfo) -> Option<Logo> {
    use windows::Foundation::Size;
    let px = LOGO_PX as f32;
    let mut img = read_image(&info.DisplayInfo().ok()?.GetLogo(Size { Width: px, Height: px }).ok()?)?;
    // Many logos sit small inside a large transparent square; keep just the drawn part (centred).
    let (w, h) = img.dimensions();
    let solid = |x: u32, y: u32| img.get_pixel(x, y)[3] > 24;
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for y in 0..h {
        for x in 0..w {
            if solid(x, y) {
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
            }
        }
    }
    if x1 < x0 {
        return None; // nothing drawn
    }
    let side = (x1 - x0).max(y1 - y0) + 1;
    let (cx, cy) = ((x0 + x1).div_ceil(2), (y0 + y1).div_ceil(2));
    let mut square = image::RgbaImage::new(side, side);
    image::imageops::overlay(&mut square, &img, side as i64 / 2 - cx as i64, side as i64 / 2 - cy as i64);
    img = square;
    let img = image::imageops::resize(&img, LOGO_PX, LOGO_PX, image::imageops::FilterType::Triangle);
    Some(Arc::new((LOGO_PX as usize, LOGO_PX as usize, img.into_raw())))
}

/// New toast notifications, read every 3 s with UserNotificationListener like the original
/// (0x14000d049): it asks for access once, waits while access is off, and treats ids it has not
/// seen before as new. Ones already there at start are not shown.
pub fn notification_watch() -> Arc<Mutex<Vec<Notice>>> {
    use windows::UI::Notifications::{Management::{UserNotificationListener, UserNotificationListenerAccessStatus as Access}, NotificationKinds};
    let shared = Arc::new(Mutex::new(vec![]));
    let out = shared.clone();
    std::thread::spawn(move || {
        let Ok(listener) = UserNotificationListener::Current() else { return };
        if listener.GetAccessStatus().is_ok_and(|a| a == Access::Unspecified) {
            let _ = listener.RequestAccessAsync().and_then(|op| op.get());
        }
        let mut seen: Option<std::collections::HashSet<u32>> = None;
        loop {
            std::thread::sleep(Duration::from_secs(3));
            if !listener.GetAccessStatus().is_ok_and(|a| a == Access::Allowed) {
                // Start over when access comes back, so toasts from the gap are not shown as new.
                seen = None;
                continue;
            }
            let Ok(list) = listener.GetNotificationsAsync(NotificationKinds::Toast).and_then(|op| op.get()) else { continue };
            let ids: std::collections::HashSet<u32> = (&list).into_iter().filter_map(|n| n.Id().ok()).collect();
            if let Some(old) = &seen {
                for n in list.into_iter().filter(|n| n.Id().is_ok_and(|id| !old.contains(&id))) {
                    let app = n.AppInfo().and_then(|a| a.DisplayInfo()).and_then(|d| d.DisplayName()).map(|s| s.to_string()).unwrap_or_default();
                    let texts: Vec<String> = n.Notification().and_then(|t| t.Visual()).and_then(|v| v.Bindings())
                        .map(|b| b.into_iter().flat_map(|b| b.GetTextElements().into_iter().flatten()).filter_map(|t| t.Text().ok()).map(|t| t.to_string()).collect())
                        .unwrap_or_default();
                    if app.is_empty() && texts.is_empty() {
                        continue;
                    }
                    let title = texts.first().cloned().unwrap_or_else(|| app.clone());
                    let id = n.AppInfo().and_then(|a| a.AppUserModelId()).map(|s| s.to_string()).unwrap_or_default();
                    // Each app's logo is read the first time it sends something, before the notice is shown.
                    let known = LOGOS.lock().unwrap().as_ref().is_some_and(|m| m.contains_key(&id));
                    if !known && !id.is_empty() {
                        let logo = n.AppInfo().ok().and_then(|a| read_logo(&a));
                        LOGOS.lock().unwrap().get_or_insert_with(Default::default).insert(id.clone(), logo);
                    }
                    let mut q = shared.lock().unwrap();
                    if q.len() >= 16 {
                        q.remove(0); // only the latest is shown; don't keep message text piling up
                    }
                    q.push((app, title, texts.get(1).cloned().unwrap_or_default(), id));
                }
            }
            if !shared.lock().unwrap().is_empty() {
                crate::tray::wake();
            }
            seen = Some(ids);
        }
    });
    out
}

#[cfg(test)]
mod player_name_tests {
    #[test]
    fn readable_names_from_player_ids() {
        // Made-up ids (no installed app answers them), so only the id's own text is used.
        assert_eq!(super::app_name("ExampleAB.ExampleMusic_abc123!Example"), "Example");
        assert_eq!(super::app_name("Example.exe"), "Example");
        assert_eq!(super::app_name(r"C:\Apps\Player\Example.exe"), "Example");
        assert_eq!(super::app_name("ExamplePlayer"), "ExamplePlayer");
    }

    #[test]
    fn cover_colour() {
        // A dark-red cover with a grey border: the red wins, lifted to stay visible.
        let mut px = vec![];
        for i in 0..100 {
            px.extend_from_slice(if i < 30 { &[128, 128, 128, 255] } else { &[120, 20, 30, 255] });
        }
        let t = super::cover_tint(&px).unwrap();
        assert!(t[0] == 230 && t[1] < 60 && t[2] < 70, "{t:?}");
        assert_eq!(super::cover_tint(&[128, 128, 128, 255].repeat(50)), None);
    }
}

#[cfg(test)]
mod controls_tests {
    /// What the playing app lets us do (skipped when nothing plays): cargo test live_controls -- --ignored
    /// Only the yes/no of each button is printed, never what is playing.
    #[test]
    #[ignore]
    fn live_controls_of_the_playing_app() {
        let manager = super::Manager::RequestAsync().unwrap().get().unwrap();
        let Ok(session) = manager.GetCurrentSession() else { return println!("nothing playing") };
        let c = session.GetPlaybackInfo().unwrap().Controls().unwrap();
        println!(
            "play {:?} pause {:?} next {:?} previous {:?} seek {:?}",
            c.IsPlayEnabled(), c.IsPauseEnabled(), c.IsNextEnabled(), c.IsPreviousEnabled(), c.IsPlaybackPositionEnabled()
        );
    }
}

#[cfg(test)]
mod art_tests {
    /// The cover of whatever plays now (skipped when nothing does): cargo test live -- --ignored
    /// Only the size is checked and printed, never what is playing.
    #[test]
    #[ignore]
    fn live_cover_of_the_playing_song() {
        let manager = super::Manager::RequestAsync().unwrap().get().unwrap();
        let Ok(session) = manager.GetCurrentSession() else { return println!("nothing playing") };
        match super::read_art(&session) {
            Some(a) => {
                assert_eq!((a.0, a.1, a.2.len()), (156, 156, 156 * 156 * 4));
                println!("cover read: {}x{}", a.0, a.1);
            }
            None => println!("this player gives no cover"),
        }
    }
}

#[cfg(test)]
mod logo_tests {
    /// A real app's logo, read the way notifications read theirs: cargo test live -- --ignored
    #[test]
    #[ignore]
    fn live_logo_of_an_installed_app() {
        let info = windows::ApplicationModel::AppInfo::GetFromAppUserModelId(&"windows.immersivecontrolpanel_cw5n1h2txyewy!microsoft.windows.immersivecontrolpanel".into()).expect("Settings is installed");
        let logo = super::read_logo(&info).expect("logo read");
        assert_eq!((logo.0, logo.1, logo.2.len()), (48, 48, 48 * 48 * 4));
        assert!(logo.2.chunks(4).any(|p| p[3] > 0), "logo is not blank");
    }
}

#[cfg(test)]
mod bluetooth_tests {
    use super::{bth_address, gadget_of, xinput_percent, Gadget};

    #[test]
    fn device_kinds_from_class_and_xinput_levels() {
        assert_eq!(gadget_of(0x240404), Gadget::Headset); // audio/video, wearable headset
        assert_eq!(gadget_of(0x002580), Gadget::Mouse); // peripheral, pointing
        assert_eq!(gadget_of(0x002540), Gadget::Keyboard); // peripheral, keyboard
        assert_eq!(gadget_of(0x0025c0), Gadget::Keyboard); // keyboard + pointer combo
        assert_eq!(gadget_of(0x002508), Gadget::Controller); // peripheral, gamepad
        assert_eq!(gadget_of(0x5a020c), Gadget::Other); // a phone
        assert_eq!([0u8, 1, 2, 3, 9].map(xinput_percent), [10, 40, 70, 100, 100]);
    }

    #[test]
    fn bluetooth_address_from_instance_id() {
        let hfp = r"BTHENUM\{0000111E-0000-1000-8000-00805F9B34FB}_LOCALMFG&0002\7&2D7D5D6F&0&A0B1C2D3E4F5_C00000000";
        assert_eq!(bth_address(hfp), Some(0xA0B1C2D3E4F5));
        assert_eq!(bth_address(r"BTHENUM\DEV_A0B1C2D3E4F5\7&8E4D5B4&0&BLUETOOTHDEVICE_A0B1C2D3E4F5"), Some(0xA0B1C2D3E4F5));
        // Local services carry a zero address.
        assert_eq!(bth_address(r"BTHENUM\{00001105-0000-1000-8000-00805F9B34FB}_LOCALMFG&0000\7&1&0&000000000000_00000000"), None);
    }
}
