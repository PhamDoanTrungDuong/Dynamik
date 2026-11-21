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

pub enum Cmd { Toggle, Next, Prev, Pause, Seek(f64) }

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
            let mut watched: Option<(String, windows::Media::Control::GlobalSystemMediaTransportControlsSession, [windows::Foundation::EventRegistrationToken; 3])> = None;
            let mut last_track = String::new();
            let mut pending: Option<Msg> = None;
            loop {
                let cmds: Vec<Cmd> = pending.take().into_iter().chain(rx.try_iter()).filter_map(|m| if let Msg::Cmd(c) = m { Some(c) } else { None }).collect();
                let current = manager.GetCurrentSession().ok();
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
                            // The position is in 100 ns ticks.
                            Cmd::Seek(s) => session.TryChangePlaybackPositionAsync((s * 1e7) as i64).and_then(|o| o.get()),
                        };
                    }
                    let mut n = read(&session).unwrap_or_default();
                    let key = format!("{}\u{1}{}", n.title, n.artist);
                    let mut fetch = None;
                    {
                        let mut s = shared.lock().unwrap();
                        if key == last_track {
                            n.lyrics = std::mem::take(&mut s.lyrics);
                            n.lyrics_state = s.lyrics_state;
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
    let playing = s.GetPlaybackInfo()?.PlaybackStatus()? == Status::Playing;
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
pub type Headset = (u64, String, Option<u8>);

/// Connected Bluetooth audio devices. Like the original: device ids from cfgmgr32 under BTHENUM,
/// the battery from DEVPKEY {104EA319-...} 2, and the connection state and name from
/// Windows.Devices.Bluetooth.BluetoothDevice, whose ConnectionStatusChanged wakes the look.
pub fn bluetooth_watch() -> Arc<Mutex<Option<Vec<Headset>>>> {
    use windows::{Devices::Bluetooth::{BluetoothConnectionStatus, BluetoothDevice, BluetoothMajorClass}, Foundation::TypedEventHandler};
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
                let audio = dev.ClassOfDevice().and_then(|c| c.MajorClass()).is_ok_and(|m| m == BluetoothMajorClass::AudioVideo);
                if !(audio && dev.ConnectionStatus().is_ok_and(|s| s == BluetoothConnectionStatus::Connected)) {
                    since.remove(&addr);
                    continue;
                }
                let battery = nodes.iter().filter(|n| n.0 == addr).find_map(|n| n.1);
                if battery.is_some() || since.entry(addr).or_insert_with(Instant::now).elapsed() >= Duration::from_secs(5) {
                    let name = dev.Name().map(|n| n.to_string()).unwrap_or_else(|_| "Bluetooth".into());
                    now.push((addr, name, battery));
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
mod bluetooth_tests {
    use super::bth_address;

    #[test]
    fn bluetooth_address_from_instance_id() {
        let hfp = r"BTHENUM\{0000111E-0000-1000-8000-00805F9B34FB}_LOCALMFG&0002\7&2D7D5D6F&0&A0B1C2D3E4F5_C00000000";
        assert_eq!(bth_address(hfp), Some(0xA0B1C2D3E4F5));
        assert_eq!(bth_address(r"BTHENUM\DEV_A0B1C2D3E4F5\7&8E4D5B4&0&BLUETOOTHDEVICE_A0B1C2D3E4F5"), Some(0xA0B1C2D3E4F5));
        // Local services carry a zero address.
        assert_eq!(bth_address(r"BTHENUM\{00001105-0000-1000-8000-00805F9B34FB}_LOCALMFG&0000\7&1&0&000000000000_00000000"), None);
    }
}
