//! Windows' own Open and Save dialogs.

use std::os::windows::ffi::OsStringExt;
use std::path::{Path, PathBuf};

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, GetSaveFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_HIDEREADONLY, OFN_NOCHANGEDIR, OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};

use recedo_win::sys::wide;

/// Long enough for any path Windows allows (32 767 UTF-16 units).
const BUF: usize = 32_768;

/// What a dialog is for: its file types and default extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Open,
    Kra,
    Png,
    Jpeg,
}

impl Kind {
    /// The "Save as type" / "Files of type" list: pairs of (label, patterns).
    fn filters(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Kind::Open => &[
                ("Pictures (*.kra, *.png, *.jpg)", "*.kra;*.png;*.jpg;*.jpeg"),
                ("Krita documents (*.kra)", "*.kra"),
                ("PNG pictures (*.png)", "*.png"),
                ("JPEG pictures (*.jpg)", "*.jpg;*.jpeg"),
                ("All files", "*.*"),
            ],
            Kind::Kra => &[("Krita document (*.kra)", "*.kra")],
            Kind::Png => &[("PNG picture (*.png)", "*.png")],
            Kind::Jpeg => &[("JPEG picture (*.jpg)", "*.jpg;*.jpeg")],
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Kind::Open => "",
            Kind::Kra => "kra",
            Kind::Png => "png",
            Kind::Jpeg => "jpg",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Kind::Open => "Open",
            Kind::Kra => "Save as",
            Kind::Png => "Export as PNG",
            Kind::Jpeg => "Export as JPEG",
        }
    }
}

/// "label\0pattern\0...\0\0".
fn filter_string(kind: Kind) -> Vec<u16> {
    let mut s = String::new();
    for (label, pattern) in kind.filters() {
        s.push_str(label);
        s.push('\0');
        s.push_str(pattern);
        s.push('\0');
    }
    s.push('\0');
    s.encode_utf16().collect()
}

/// Shows the dialog; `None` if it was cancelled. `name` is the suggested file name (Save
/// dialogs), `folder` where it starts.
pub fn show(owner: HWND, kind: Kind, folder: Option<&Path>, name: &str) -> Option<PathBuf> {
    let mut buf = vec![0u16; BUF];
    // A name with characters Windows forbids would make the dialog refuse to open.
    let clean: String = name.chars().map(|c| if "<>:\"/\\|?*".contains(c) || c.is_control() { '_' } else { c }).collect();
    for (i, u) in clean.encode_utf16().take(BUF - 1).enumerate() {
        buf[i] = u;
    }
    let filter = filter_string(kind);
    let dir = folder.filter(|d| d.is_dir()).map(|d| wide(&d.to_string_lossy()));
    let ext = wide(kind.extension());
    let title = wide(kind.title());
    // SAFETY: zeroed POD, filled in below.
    let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    ofn.hwndOwner = owner;
    ofn.lpstrFilter = filter.as_ptr();
    ofn.nFilterIndex = 1;
    ofn.lpstrFile = buf.as_mut_ptr();
    ofn.nMaxFile = BUF as u32;
    ofn.lpstrInitialDir = dir.as_ref().map_or(std::ptr::null(), |d| d.as_ptr());
    ofn.lpstrTitle = title.as_ptr();
    ofn.lpstrDefExt = if kind == Kind::Open { std::ptr::null() } else { ext.as_ptr() };
    ofn.Flags = OFN_EXPLORER | OFN_NOCHANGEDIR | OFN_HIDEREADONLY | OFN_PATHMUSTEXIST;
    // SAFETY: every pointer in `ofn` points at a buffer that lives until the call returns.
    let ok = unsafe {
        if kind == Kind::Open {
            ofn.Flags |= OFN_FILEMUSTEXIST;
            GetOpenFileNameW(&mut ofn)
        } else {
            ofn.Flags |= OFN_OVERWRITEPROMPT;
            GetSaveFileNameW(&mut ofn)
        }
    };
    if ok == 0 {
        return None;
    }
    let len = buf.iter().position(|&u| u == 0).unwrap_or(BUF);
    // Exactly as Windows gave it (a lossy conversion could change unusual file names).
    Some(PathBuf::from(std::ffi::OsString::from_wide(&buf[..len])))
}
