//! Smart OCR (Ctrl + Alt + T): freeze the screen, drag over any area, and the words in it are
//! read by Windows' own OCR (offline, in the user's languages) and copied. Nothing is saved.
use std::sync::{Arc, Mutex};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{Input::KeyboardAndMouse::{ReleaseCapture, SetCapture, VK_ESCAPE}, WindowsAndMessaging::*},
};

/// What a read ended with: the text copied, nothing found, or cancelled.
pub enum Outcome {
    Copied(usize),
    NoText,
    NoEngine, // Windows has no OCR language installed
    ClipboardBusy, // read, but another app held the clipboard
    Cancelled,
}

/// Clears BUSY however the read ends, a panic included, so the hotkey never stays dead.
struct Done;
impl Drop for Done {
    fn drop(&mut self) {
        BUSY.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

fn shot() -> std::sync::MutexGuard<'static, Option<Shot>> {
    SHOT.lock().unwrap_or_else(|e| e.into_inner())
}

/// One read at a time: a second Ctrl + Alt + T while the screen is frozen does nothing.
static BUSY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The frozen screen and the area being dragged, for the overlay window's thread only.
struct Shot {
    bitmap: HBITMAP,
    w: i32,
    h: i32,
    from: Option<(i32, i32)>,
    to: (i32, i32),
    done: Option<RECT>,
}
unsafe impl Send for Shot {}
static SHOT: Mutex<Option<Shot>> = Mutex::new(None);

/// Starts a read on its own thread; `out` gets the outcome and the notch is woken.
pub fn start(out: Arc<Mutex<Option<Outcome>>>) {
    use std::sync::atomic::Ordering;
    if BUSY.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        let _done = Done;
        let outcome = unsafe { pick_area() }.map_or(Outcome::Cancelled, |(pixels, w, h)| match read(&pixels, w, h) {
            Err(e) => e,
            Ok(text) if text.trim().is_empty() => Outcome::NoText,
            Ok(text) => {
                let n = text.chars().count();
                // Another app may hold the clipboard for a moment: try a few times.
                let copied = (0..3).any(|k| {
                    if k > 0 {
                        std::thread::sleep(std::time::Duration::from_millis(60));
                    }
                    arboard::Clipboard::new().and_then(|mut c| c.set_text(text.clone())).is_ok()
                });
                if copied { Outcome::Copied(n) } else { Outcome::ClipboardBusy }
            }
        });
        *out.lock().unwrap_or_else(|e| e.into_inner()) = Some(outcome);
        crate::tray::wake();
    });
}

/// Shows the frozen screen and waits for a drag; the dragged area's pixels (BGRA, top-down).
unsafe fn pick_area() -> Option<(Vec<u8>, i32, i32)> {
    let (x, y) = (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN));
    let (w, h) = (GetSystemMetrics(SM_CXVIRTUALSCREEN), GetSystemMetrics(SM_CYVIRTUALSCREEN));
    let screen = GetDC(std::ptr::null_mut());
    let mem = CreateCompatibleDC(screen);
    let bitmap = CreateCompatibleBitmap(screen, w, h);
    let old = SelectObject(mem, bitmap);
    BitBlt(mem, 0, 0, w, h, screen, x, y, SRCCOPY | CAPTUREBLT);
    SelectObject(mem, old);
    DeleteDC(mem);
    ReleaseDC(std::ptr::null_mut(), screen);
    *shot() = Some(Shot { bitmap, w, h, from: None, to: (0, 0), done: None });

    let class: Vec<u16> = "NotchOcr\0".encode_utf16().collect();
    let module = GetModuleHandleW(std::ptr::null());
    let wc = WNDCLASSW { lpfnWndProc: Some(proc_), hInstance: module, lpszClassName: class.as_ptr(), hCursor: LoadCursorW(std::ptr::null_mut(), IDC_CROSS), ..std::mem::zeroed() };
    RegisterClassW(&wc);
    let hwnd = CreateWindowExW(WS_EX_TOPMOST | WS_EX_TOOLWINDOW, class.as_ptr(), class.as_ptr(), WS_POPUP | WS_VISIBLE, x, y, w, h, std::ptr::null_mut(), std::ptr::null_mut(), module, std::ptr::null());
    if hwnd.is_null() {
        DeleteObject(shot().take()?.bitmap);
        return None;
    }
    SetForegroundWindow(hwnd);
    let mut msg: MSG = std::mem::zeroed();
    while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
    let shot = shot().take()?;
    let area = shot.done;
    let pixels = area.and_then(|r| crop(shot.bitmap, r));
    DeleteObject(shot.bitmap);
    pixels
}

/// The area's pixels from the frozen screen, as BGRA rows from the top.
unsafe fn crop(bitmap: HBITMAP, r: RECT) -> Option<(Vec<u8>, i32, i32)> {
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    if w < 4 || h < 4 {
        return None;
    }
    let screen = GetDC(std::ptr::null_mut());
    let (src, dst) = (CreateCompatibleDC(screen), CreateCompatibleDC(screen));
    let part = CreateCompatibleBitmap(screen, w, h);
    let (o1, o2) = (SelectObject(src, bitmap), SelectObject(dst, part));
    BitBlt(dst, 0, 0, w, h, src, r.left, r.top, SRCCOPY);
    SelectObject(src, o1);
    SelectObject(dst, o2);
    let mut info: BITMAPINFO = std::mem::zeroed();
    info.bmiHeader = BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32, biCompression: BI_RGB, ..std::mem::zeroed() };
    let mut pixels = vec![0u8; (w * h * 4) as usize];
    let rows = GetDIBits(dst, part, 0, h as u32, pixels.as_mut_ptr().cast(), &mut info, DIB_RGB_COLORS);
    DeleteObject(part);
    DeleteDC(src);
    DeleteDC(dst);
    ReleaseDC(std::ptr::null_mut(), screen);
    (rows == h).then_some((pixels, w, h))
}

/// Windows OCR on BGRA pixels, line by line. Areas bigger than Windows OCR takes are shrunk to fit.
fn read(pixels: &[u8], w: i32, h: i32) -> Result<String, Outcome> {
    // WinRT needs this once on each thread that uses it; again is harmless.
    let _ = unsafe { windows::Win32::System::WinRT::RoInitialize(windows::Win32::System::WinRT::RO_INIT_MULTITHREADED) };
    use windows::{Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap}, Media::Ocr::OcrEngine, Security::Cryptography::CryptographicBuffer};
    let engine = OcrEngine::TryCreateFromUserProfileLanguages().map_err(|_| Outcome::NoEngine)?;
    let most = OcrEngine::MaxImageDimension().unwrap_or(2600) as i32;
    let (pixels, w, h) = if w > most || h > most {
        let k = most as f32 / w.max(h) as f32;
        let (nw, nh) = (((w as f32 * k) as u32).max(1), ((h as f32 * k) as u32).max(1));
        let img = image::RgbaImage::from_raw(w as u32, h as u32, pixels.to_vec()).ok_or(Outcome::NoText)?;
        let small = image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle);
        (small.into_raw(), nw as i32, nh as i32)
    } else {
        (pixels.to_vec(), w, h)
    };
    let fail = |_| Outcome::NoText;
    let buffer = CryptographicBuffer::CreateFromByteArray(&pixels).map_err(fail)?;
    let bitmap = SoftwareBitmap::CreateCopyFromBuffer(&buffer, BitmapPixelFormat::Bgra8, w, h).map_err(fail)?;
    let result = engine.RecognizeAsync(&bitmap).and_then(|op| op.get()).map_err(fail)?;
    let lines: Vec<String> = result.Lines().map_err(fail)?.into_iter().filter_map(|l| l.Text().ok()).map(|t| t.to_string()).collect();
    Ok(lines.join("\n"))
}

fn rect(a: (i32, i32), b: (i32, i32)) -> RECT {
    RECT { left: a.0.min(b.0), top: a.1.min(b.1), right: a.0.max(b.0), bottom: a.1.max(b.1) }
}

fn point(l: LPARAM) -> (i32, i32) {
    ((l & 0xffff) as i16 as i32, ((l >> 16) & 0xffff) as i16 as i32)
}

unsafe extern "system" fn proc_(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_LBUTTONDOWN => {
            if let Some(s) = shot().as_mut() {
                s.from = Some(point(l));
                s.to = point(l);
            }
            SetCapture(hwnd);
            0
        }
        WM_MOUSEMOVE => {
            if let Some(s) = shot().as_mut().filter(|s| s.from.is_some()) {
                s.to = point(l);
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
            0
        }
        WM_LBUTTONUP => {
            ReleaseCapture();
            if let Some(s) = shot().as_mut() {
                s.done = s.from.map(|f| rect(f, point(l)));
            }
            DestroyWindow(hwnd);
            0
        }
        WM_KEYDOWN if w as u16 == VK_ESCAPE => {
            DestroyWindow(hwnd);
            0
        }
        WM_RBUTTONUP => {
            DestroyWindow(hwnd); // right-click cancels too
            0
        }
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let dc = BeginPaint(hwnd, &mut ps);
            if let Some(s) = shot().as_ref() {
                // Frozen screen, darkened, with the dragged area shown at full brightness.
                let back = CreateCompatibleDC(dc);
                let frame = CreateCompatibleBitmap(dc, s.w, s.h);
                let of = SelectObject(back, frame);
                let src = CreateCompatibleDC(dc);
                let os = SelectObject(src, s.bitmap);
                BitBlt(back, 0, 0, s.w, s.h, src, 0, 0, SRCCOPY);
                let shade = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 110, AlphaFormat: 0 };
                let black = CreateCompatibleDC(dc);
                let one = CreateCompatibleBitmap(dc, 1, 1);
                let ob = SelectObject(black, one);
                PatBlt(black, 0, 0, 1, 1, BLACKNESS);
                AlphaBlend(back, 0, 0, s.w, s.h, black, 0, 0, 1, 1, shade);
                if let Some(f) = s.from {
                    let r = rect(f, s.to);
                    BitBlt(back, r.left, r.top, r.right - r.left, r.bottom - r.top, src, r.left, r.top, SRCCOPY);
                    let pen = CreatePen(PS_SOLID, 2, 0x00FF_FFFF);
                    let op = SelectObject(back, pen);
                    let obr = SelectObject(back, GetStockObject(NULL_BRUSH));
                    Rectangle(back, r.left, r.top, r.right, r.bottom);
                    SelectObject(back, op);
                    SelectObject(back, obr);
                    DeleteObject(pen);
                }
                BitBlt(dc, 0, 0, s.w, s.h, back, 0, 0, SRCCOPY);
                SelectObject(black, ob);
                DeleteObject(one);
                DeleteDC(black);
                SelectObject(src, os);
                DeleteDC(src);
                SelectObject(back, of);
                DeleteObject(frame);
                DeleteDC(back);
            }
            EndPaint(hwnd, &ps);
            0
        }
        WM_ERASEBKGND => 1,
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, w, l),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Draws words with GDI and reads them back through Windows OCR (needs an OCR language, e.g. English).
    #[test]
    fn reads_words_drawn_on_a_bitmap() {
        unsafe {
            let (w, h) = (520, 120);
            let screen = GetDC(std::ptr::null_mut());
            let dc = CreateCompatibleDC(screen);
            let bmp = CreateCompatibleBitmap(screen, w, h);
            let old = SelectObject(dc, bmp);
            PatBlt(dc, 0, 0, w, h, WHITENESS);
            let face: Vec<u16> = "Segoe UI\0".encode_utf16().collect();
            let font = CreateFontW(64, 0, 0, 0, 600, 0, 0, 0, 0, 0, 0, 0, 0, face.as_ptr());
            let of = SelectObject(dc, font);
            let text: Vec<u16> = "HELLO 2026".encode_utf16().collect();
            TextOutW(dc, 20, 20, text.as_ptr(), text.len() as i32);
            SelectObject(dc, of);
            DeleteObject(font);
            SelectObject(dc, old);
            DeleteDC(dc);
            ReleaseDC(std::ptr::null_mut(), screen);
            let (pixels, pw, ph) = crop(bmp, RECT { left: 0, top: 0, right: w, bottom: h }).unwrap();
            DeleteObject(bmp);
            let got = read(&pixels, pw, ph).unwrap_or_default();
            assert!(got.contains("HELLO") && got.contains("2026"), "{got:?}");
        }
    }
}
