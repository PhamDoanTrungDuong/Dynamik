//! Dragging Shelf files and clipboard text out of the notch into Explorer, a chat or an email,
//! with OLE drag and drop as the original does (it imports DoDragDrop). The Shell's own data
//! object holds the data and SHDoDragDrop supplies the drop source and drag image.
use std::{mem::ManuallyDrop, os::windows::ffi::OsStrExt, path::PathBuf};
use windows::{
    core::Result,
    Win32::{
        Foundation::{GlobalFree, BOOL, LPARAM, POINT, WPARAM},
        System::{
            Com::{IDataObject, DVASPECT_CONTENT, FORMATETC, STGMEDIUM, STGMEDIUM_0, TYMED_HGLOBAL},
            Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
            Threading::GetCurrentProcessId,
            Ole::{OleInitialize, CF_HDROP, CF_UNICODETEXT, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK},
        },
        UI::{
            Shell::{SHCreateDataObject, SHDoDragDrop, DROPFILES},
            WindowsAndMessaging::{GetAncestor, GetCursorPos, GetWindowThreadProcessId, PostMessageW, WindowFromPoint, GA_ROOT, WM_LBUTTONUP},
        },
    },
};

/// Drags these files out; blocks until they are dropped or the drag is cancelled.
/// Copy or link only, never move, so a drop can't take a file away from where it lives.
pub fn files(paths: &[PathBuf]) {
    let _ = drag(CF_HDROP.0, &hdrop(paths), DROPEFFECT_COPY | DROPEFFECT_LINK);
}

/// Drags this text out, as if it were selected text.
pub fn text(s: &str) {
    let bytes: Vec<u8> = s.encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect();
    let _ = drag(CF_UNICODETEXT.0, &bytes, DROPEFFECT_COPY);
}

/// CF_HDROP: a DROPFILES header, then the wide paths, each ending in 0, and one more 0.
fn hdrop(paths: &[PathBuf]) -> Vec<u8> {
    let head = std::mem::size_of::<DROPFILES>();
    let mut out = vec![0u8; head];
    out[..4].copy_from_slice(&(head as u32).to_le_bytes()); // pFiles
    out[head - 4..].copy_from_slice(&1u32.to_le_bytes()); // fWide
    for p in paths {
        out.extend(p.as_os_str().encode_wide().chain([0]).flat_map(u16::to_le_bytes));
    }
    out.extend([0, 0]);
    out
}

fn drag(format: u16, data: &[u8], allowed: DROPEFFECT) -> Result<DROPEFFECT> {
    unsafe {
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let hwnd = GetAncestor(WindowFromPoint(pt), GA_ROOT);
        // The drag starts on the notch, so the window under the pointer is ours; if not, don't drag
        // (and don't post the release below to another app).
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if hwnd.is_invalid() || pid != GetCurrentProcessId() {
            return Ok(DROPEFFECT(0));
        }
        // winit already set up OLE on this thread (drag and drop on); repeating it is harmless.
        let _ = OleInitialize(None);
        let obj = data_object(format, data)?;
        let done = SHDoDragDrop(hwnd, &obj, None, allowed);
        // The drag loop swallows the button release; hand one back so the notch doesn't think it's still held.
        let _ = PostMessageW(hwnd, WM_LBUTTONUP, WPARAM(0), LPARAM(0));
        done
    }
}

/// A Shell data object holding `data` in `format`.
fn data_object(format: u16, data: &[u8]) -> Result<IDataObject> {
    unsafe {
        let obj: IDataObject = SHCreateDataObject(None, None, None)?;
        let mem = GlobalAlloc(GMEM_MOVEABLE, data.len())?;
        let dst = GlobalLock(mem) as *mut u8;
        if dst.is_null() {
            let _ = GlobalFree(mem);
            return Err(windows::core::Error::from_win32());
        }
        std::ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len());
        let _ = GlobalUnlock(mem);
        let fmt = FORMATETC { cfFormat: format, ptd: std::ptr::null_mut(), dwAspect: DVASPECT_CONTENT.0, lindex: -1, tymed: TYMED_HGLOBAL.0 as u32 };
        let med = STGMEDIUM { tymed: TYMED_HGLOBAL.0 as u32, u: STGMEDIUM_0 { hGlobal: mem }, pUnkForRelease: ManuallyDrop::new(None) };
        // With release = true the data object owns the memory from here on.
        if let Err(e) = obj.SetData(&fmt, &med, BOOL(1)) {
            let _ = GlobalFree(mem);
            return Err(e);
        }
        Ok(obj)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn hdrop_lists_wide_paths_after_the_header() {
        let b = super::hdrop(&["C:\\a.txt".into(), "D:\\b".into()]);
        let head = std::mem::size_of::<super::DROPFILES>();
        assert_eq!(u32::from_le_bytes(b[..4].try_into().unwrap()) as usize, head);
        assert_eq!(u32::from_le_bytes(b[head - 4..head].try_into().unwrap()), 1);
        let wide: Vec<u16> = b[head..].chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let want: Vec<u16> = "C:\\a.txt\0D:\\b\0\0".encode_utf16().collect();
        assert_eq!(wide, want);
    }

    #[test]
    fn shell_data_object_hands_back_the_text() {
        use windows::Win32::System::{Com::*, Memory::GlobalSize, Ole::{OleInitialize, ReleaseStgMedium, CF_UNICODETEXT}};
        unsafe {
            let _ = OleInitialize(None);
            let bytes: Vec<u8> = "xin chào\0".encode_utf16().flat_map(u16::to_le_bytes).collect();
            let obj = super::data_object(CF_UNICODETEXT.0, &bytes).unwrap();
            let fmt = FORMATETC { cfFormat: CF_UNICODETEXT.0, ptd: std::ptr::null_mut(), dwAspect: DVASPECT_CONTENT.0, lindex: -1, tymed: TYMED_HGLOBAL.0 as u32 };
            let mut med = obj.GetData(&fmt).unwrap();
            let h = med.u.hGlobal;
            let got = std::slice::from_raw_parts(windows::Win32::System::Memory::GlobalLock(h) as *const u8, bytes.len().min(GlobalSize(h)));
            assert_eq!(got, &bytes[..]);
            let _ = windows::Win32::System::Memory::GlobalUnlock(h);
            ReleaseStgMedium(&mut med);
        }
    }
}
