//! Asking Windows: full screen in front, Caps Lock, keyboard language, battery, the chime, Start with Windows.

/// The original's full-screen rule (0x140051a46): hide when Windows says a full-screen game or
/// presentation runs, or when the front window (not ours, not the desktop or taskbar, not a
/// maximised window with a title bar) covers its whole monitor.
pub(crate) fn fullscreen_in_front() -> bool {
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

pub(crate) fn caps_lock() -> bool {
    unsafe { windows_sys::Win32::UI::Input::KeyboardAndMouse::GetKeyState(0x14) & 1 != 0 } // VK_CAPITAL
}

/// The keyboard language of the window in front (its language id).
pub(crate) fn input_lang() -> u16 {
    use windows_sys::Win32::UI::{Input::KeyboardAndMouse::GetKeyboardLayout, WindowsAndMessaging::*};
    unsafe {
        let thread = GetWindowThreadProcessId(GetForegroundWindow(), std::ptr::null_mut());
        (GetKeyboardLayout(thread) as usize & 0xffff) as u16
    }
}

/// "EN", "VI"… for a language id.
pub(crate) fn lang_name(lang: u16) -> String {
    use windows_sys::Win32::Globalization::{GetLocaleInfoW, LOCALE_SISO639LANGNAME};
    let mut buf = [0u16; 9];
    let n = unsafe { GetLocaleInfoW(lang as u32, LOCALE_SISO639LANGNAME, buf.as_mut_ptr(), 9) };
    let s = String::from_utf16_lossy(&buf[..(n.max(1) - 1) as usize]).to_uppercase();
    if s.is_empty() { format!("{lang:04X}") } else { s }
}

pub(crate) fn battery() -> Option<(u8, bool)> {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut s: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    // Flag 128 = no system battery (desktop PCs).
    if unsafe { GetSystemPowerStatus(&mut s) } == 0 || s.BatteryLifePercent == 255 || s.BatteryFlag & 128 != 0 {
        return None;
    }
    Some((s.BatteryLifePercent, s.ACLineStatus == 1))
}

pub(crate) fn beep() {
    unsafe { windows_sys::Win32::System::Diagnostics::Debug::MessageBeep(0) };
}

pub(crate) const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
// Where Task Manager › Startup keeps its own on/off for the same entry.
pub(crate) const APPROVED_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

/// Runs `reg` with no window; its output when it worked.
pub(crate) fn run_reg(args: &[&str]) -> Option<String> {
    use std::os::windows::process::CommandExt;
    let out = std::process::Command::new("reg").args(args).creation_flags(0x0800_0000).output().ok()?; // CREATE_NO_WINDOW
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Turns starting with Windows on (for this exe) or off; false when Windows wouldn't take it.
pub(crate) fn set_startup(on: bool) -> bool {
    if on {
        let exe = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default();
        // An earlier "off" from Task Manager would keep it from starting, so that goes too.
        let _ = run_reg(&["delete", APPROVED_KEY, "/v", "MyNotch", "/f"]);
        run_reg(&["add", RUN_KEY, "/v", "MyNotch", "/t", "REG_SZ", "/d", &format!("\"{exe}\""), "/f"]).is_some()
    } else {
        run_reg(&["delete", RUN_KEY, "/v", "MyNotch", "/f"]).is_some() || startup_state().is_none()
    }
}

/// What Windows will really do at sign-in: the exe it starts, and whether Task Manager lets it.
/// None when it isn't set to start at all.
pub(crate) fn startup_state() -> Option<(String, bool)> {
    let run = run_reg(&["query", RUN_KEY, "/v", "MyNotch"])?;
    let path = startup_path(&run)?;
    let approved = run_reg(&["query", APPROVED_KEY, "/v", "MyNotch"]).is_none_or(|q| startup_approved(&q));
    Some((path, approved))
}

/// The exe path out of `reg query` for the Run value ("    MyNotch    REG_SZ    "C:\…\notch.exe"").
pub(crate) fn startup_path(query: &str) -> Option<String> {
    let line = query.lines().find(|l| l.contains("REG_SZ"))?;
    Some(line.split_once("REG_SZ")?.1.trim().trim_matches('"').to_string())
}

/// Task Manager's own switch, out of `reg query`: an entry whose first byte is 03 is turned off.
pub(crate) fn startup_approved(query: &str) -> bool {
    let Some(line) = query.lines().find(|l| l.contains("REG_BINARY")) else { return true };
    !line.split_once("REG_BINARY").is_some_and(|(_, hex)| hex.trim().to_ascii_uppercase().starts_with("03"))
}
