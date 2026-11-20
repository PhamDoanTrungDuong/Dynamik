//! Taskbar tray icon with a Settings / Quit menu, so the notch can be set up or closed from there.
//! Its hidden window also hears what Windows announces, as the original does, so the notch waits
//! instead of checking: brightness and power changes, clipboard copies, the window in front
//! changing or moving (full screen), and Caps Lock / language keys.
use eframe::egui;
use std::{ptr::{null, null_mut}, sync::{atomic::{AtomicPtr, AtomicU32, AtomicU8, Ordering}, OnceLock}};
use windows_sys::{
    core::GUID,
    Win32::{
        Foundation::*,
        System::{DataExchange::AddClipboardFormatListener, LibraryLoader::GetModuleHandleW, Power::*},
        UI::{Accessibility::*, Input::KeyboardAndMouse::*, Shell::*, WindowsAndMessaging::*},
    },
};

/// What changed since the last look: the clipboard, the power source or battery level, the window in front.
pub const EV_CLIP: u8 = 1;
pub const EV_POWER: u8 = 2;
pub const EV_WINDOW: u8 = 4;
// All set at start, so the first frame reads everything once.
static EVENTS: AtomicU8 = AtomicU8::new(EV_CLIP | EV_POWER | EV_WINDOW);
static HWND_: AtomicPtr<core::ffi::c_void> = AtomicPtr::new(null_mut());
const ACDC: GUID = GUID::from_u128(0x5d3e9a59_e9d5_4b00_a6bd_ff34ff516548); // GUID_ACDC_POWER_SOURCE
const BATTERY_LEVEL: GUID = GUID::from_u128(0xa7ad8041_b45a_4cae_87a3_eecbb468a9e1); // GUID_BATTERY_PERCENTAGE_REMAINING
// A language switch lands a moment after its keys; a moving window settles before it's checked.
const LANG_TIMER: usize = 2;
const MOVE_TIMER: usize = 3;

pub const SETTINGS: u8 = 1;
const QUIT: u8 = 2;
const WM_TRAY: u32 = WM_APP + 1;
static PENDING: AtomicU8 = AtomicU8::new(0);
static CTX: OnceLock<egui::Context> = OnceLock::new();
// GUID_DEVICE_POWER_POLICY_VIDEO_BRIGHTNESS, the GUID the original registers for.
const BRIGHTNESS: GUID = GUID::from_u128(0xaded5e82_b909_4619_9949_f5d71dac0bcb);
// Latest brightness in percent, and a count of changes so the app sees each one.
const NO_VALUE: u32 = u32::MAX;
static BRIGHT: AtomicU32 = AtomicU32::new(NO_VALUE);
static BRIGHT_SEQ: AtomicU32 = AtomicU32::new(0);

/// The menu choice made since the last call (0 when none).
pub fn take() -> u8 {
    PENDING.swap(0, Ordering::Relaxed)
}

/// The EV_* changes since the last call.
pub fn take_events() -> u8 {
    EVENTS.swap(0, Ordering::Relaxed)
}

/// Asks the notch to draw a frame, for a background watcher whose value just changed.
pub fn wake() {
    if let Some(ctx) = CTX.get() {
        ctx.request_repaint();
    }
}

fn signal(ev: u8) {
    EVENTS.fetch_or(ev, Ordering::Relaxed);
    wake();
}

/// (change count, brightness percent); the count is 0 until Windows reports a change.
pub fn brightness() -> (u32, f32) {
    (BRIGHT_SEQ.load(Ordering::Relaxed), BRIGHT.load(Ordering::Relaxed) as f32 / 100.0)
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

fn data(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid
}

// Explorer broadcasts this when the taskbar is (re)created; the icon must be added again then.
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);
const RETRY_TIMER: usize = 1;

/// Adds the icon; false while the taskbar isn't there yet (e.g. at sign-in, before Explorer).
unsafe fn add_icon(hwnd: HWND) -> bool {
    let mut nid = data(hwnd);
    nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    nid.uCallbackMessage = WM_TRAY;
    nid.hIcon = icon();
    for (d, c) in nid.szTip.iter_mut().zip("My Notch".encode_utf16()) {
        *d = c;
    }
    Shell_NotifyIconW(NIM_ADD, &nid) != 0
}

pub fn start(ctx: &egui::Context) {
    if CTX.set(ctx.clone()).is_err() {
        return;
    }
    std::thread::spawn(|| unsafe {
        let class = wide("NotchTray");
        let wc = WNDCLASSW { lpfnWndProc: Some(proc_), lpszClassName: class.as_ptr(), ..std::mem::zeroed() };
        RegisterClassW(&wc);
        // A hidden top-level window (not message-only) so the menu can take the foreground and close on click-away.
        let hwnd = CreateWindowExW(0, class.as_ptr(), class.as_ptr(), 0, 0, 0, 0, 0, null_mut(), null_mut(), null_mut(), null());
        if hwnd.is_null() {
            return;
        }
        TASKBAR_CREATED.store(RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()), Ordering::Relaxed);
        // Windows answers the registration with the current value (the starting point, not a change).
        RegisterPowerSettingNotification(hwnd, &BRIGHTNESS, DEVICE_NOTIFY_WINDOW_HANDLE);
        RegisterPowerSettingNotification(hwnd, &ACDC, DEVICE_NOTIFY_WINDOW_HANDLE);
        RegisterPowerSettingNotification(hwnd, &BATTERY_LEVEL, DEVICE_NOTIFY_WINDOW_HANDLE);
        HWND_.store(hwnd, Ordering::Relaxed);
        AddClipboardFormatListener(hwnd);
        let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;
        SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, null_mut(), Some(win_event), 0, 0, flags);
        SetWinEventHook(EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_LOCATIONCHANGE, null_mut(), Some(win_event), 0, 0, flags);
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(keys), GetModuleHandleW(null()), 0);
        if !add_icon(hwnd) {
            SetTimer(hwnd, RETRY_TIMER, 2000, None);
        }
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            DispatchMessageW(&msg);
        }
    });
}

unsafe extern "system" fn proc_(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if msg == WM_POWERBROADCAST && w as u32 == PBT_POWERSETTINGCHANGE && l != 0 {
        let set = &*(l as *const POWERBROADCAST_SETTING);
        let g = &set.PowerSetting;
        let same = (g.data1, g.data2, g.data3, g.data4) == (BRIGHTNESS.data1, BRIGHTNESS.data2, BRIGHTNESS.data3, BRIGHTNESS.data4);
        if same && set.DataLength >= 4 {
            let data = (l as *const u8).add(std::mem::offset_of!(POWERBROADCAST_SETTING, Data));
            let value = std::ptr::read_unaligned(data as *const u32).min(100);
            // The first message (sent on registering) only sets the starting value.
            let old = BRIGHT.swap(value, Ordering::Relaxed);
            if old != NO_VALUE && old != value {
                BRIGHT_SEQ.fetch_add(1, Ordering::Relaxed);
                wake();
            }
        } else if [ACDC, BATTERY_LEVEL].iter().any(|b| (g.data1, g.data2, g.data3, g.data4) == (b.data1, b.data2, b.data3, b.data4)) {
            signal(EV_POWER);
        }
        return 1;
    }
    if msg == WM_CLIPBOARDUPDATE {
        signal(EV_CLIP);
        return 0;
    }
    if msg == WM_TIMER && (w == LANG_TIMER || w == MOVE_TIMER) {
        KillTimer(hwnd, w);
        signal(if w == MOVE_TIMER { EV_WINDOW } else { 0 });
        return 0;
    }
    if (msg == WM_TIMER && w == RETRY_TIMER) || (msg != 0 && msg == TASKBAR_CREATED.load(Ordering::Relaxed)) {
        if add_icon(hwnd) {
            KillTimer(hwnd, RETRY_TIMER);
        } else {
            SetTimer(hwnd, RETRY_TIMER, 2000, None);
        }
        return 0;
    }
    if msg == WM_TRAY && w == 1 && matches!(l as u32 & 0xFFFF, WM_LBUTTONUP | WM_RBUTTONUP) {
        let menu = CreatePopupMenu();
        AppendMenuW(menu, MF_STRING, SETTINGS as usize, wide("Settings").as_ptr());
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        AppendMenuW(menu, MF_STRING, QUIT as usize, wide("Quit").as_ptr());
        let mut pt = POINT { x: 0, y: 0 };
        GetCursorPos(&mut pt);
        SetForegroundWindow(hwnd);
        let pick = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON, pt.x, pt.y, 0, hwnd, null()) as u8;
        PostMessageW(hwnd, WM_NULL, 0, 0);
        DestroyMenu(menu);
        if pick == QUIT {
            // Quit from here: the notch window may be hidden (full screen) and not redrawing.
            // Everything is saved when it changes, so nothing is lost by leaving at once.
            Shell_NotifyIconW(NIM_DELETE, &data(hwnd));
            std::process::exit(0);
        }
        if pick != 0 {
            PENDING.store(pick, Ordering::Relaxed);
            if let Some(ctx) = CTX.get() {
                ctx.request_repaint();
            }
        }
        return 0;
    }
    DefWindowProcW(hwnd, msg, w, l)
}

/// The front window changed (checked at once) or moved (checked once it has been still for 0.2 s,
/// so dragging a window around doesn't redraw the notch on every step).
unsafe extern "system" fn win_event(_: HWINEVENTHOOK, event: u32, hwnd: HWND, object: i32, child: i32, _: u32, _: u32) {
    if event == EVENT_SYSTEM_FOREGROUND {
        signal(EV_WINDOW);
    } else if object == OBJID_WINDOW && child == CHILDID_SELF as i32 && hwnd == GetForegroundWindow() {
        SetTimer(HWND_.load(Ordering::Relaxed), MOVE_TIMER, 200, None);
    }
}

/// Caps Lock and the keys that switch the language (Win+Space, Alt+Shift, Ctrl+Shift): the notch
/// looks again now and once more just after, when the new language has landed. Keys pass on unchanged.
unsafe extern "system" fn keys(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && matches!(w as u32, WM_KEYUP | WM_SYSKEYUP) {
        let vk = (*(l as *const KBDLLHOOKSTRUCT)).vkCode as u16;
        if matches!(vk, VK_CAPITAL | VK_SPACE | VK_SHIFT | VK_LSHIFT | VK_RSHIFT | VK_CONTROL | VK_LCONTROL | VK_RCONTROL | VK_MENU | VK_LMENU | VK_RMENU | VK_LWIN | VK_RWIN) {
            wake();
            SetTimer(HWND_.load(Ordering::Relaxed), LANG_TIMER, 300, None);
        }
    }
    CallNextHookEx(null_mut(), code, w, l)
}

/// A 32 px black notch pill with a white rim, readable on light and dark taskbars.
unsafe fn icon() -> HICON {
    const N: usize = 32;
    let mut bgra = vec![0u8; N * N * 4];
    for y in 0..N {
        for x in 0..N {
            let (dx, dy) = (x as f32 + 0.5 - 16.0, y as f32 + 0.5 - 16.0);
            let d = (dx.abs() - 7.0).max(0.0).hypot(dy) - 8.0; // distance to a 30 x 16 capsule
            let outer = (0.5 - d).clamp(0.0, 1.0);
            let inner = (0.5 - (d + 2.5)).clamp(0.0, 1.0);
            let v = (255.0 * (1.0 - inner)) as u8;
            bgra[(y * N + x) * 4..][..4].copy_from_slice(&[v, v, v, (255.0 * outer) as u8]);
        }
    }
    let mask = [0u8; N * N / 8];
    CreateIcon(null_mut(), N as i32, N as i32, 1, 32, mask.as_ptr(), bgra.as_ptr())
}
