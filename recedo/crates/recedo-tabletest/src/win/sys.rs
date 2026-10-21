//! Small Win32 helpers: strings, clipboard, monitors, folders, time, OS version.

use std::collections::HashMap;
use std::path::PathBuf;

use windows_sys::Win32::Devices::Display::{
    DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO,
    DISPLAYCONFIG_SOURCE_DEVICE_NAME, DISPLAYCONFIG_TARGET_DEVICE_NAME, DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes,
    QDC_ONLY_ACTIVE_PATHS, QueryDisplayConfig,
};
use windows_sys::Win32::Foundation::{GlobalFree, HWND, LPARAM, RECT};
use windows_sys::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW};
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData};
use windows_sys::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows_sys::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegGetValueW};
use windows_sys::Win32::System::SystemInformation::GetLocalTime;
use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows_sys::Win32::UI::Shell::{FOLDERID_Documents, FOLDERID_RoamingAppData, KF_FLAG_CREATE, SHGetKnownFolderPath, ShellExecuteW};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

const CF_UNICODETEXT: u32 = 13;
const MONITORINFOF_PRIMARY: u32 = 1;

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn from_wide(s: &[u16]) -> String {
    recedo_tablet::wintab_abi::utf16_until_nul(s)
}

pub fn set_clipboard_text(hwnd: HWND, text: &str) -> Result<(), String> {
    let w = wide(text);
    // SAFETY: standard clipboard sequence; the global memory is handed to the system on success.
    unsafe {
        if OpenClipboard(hwnd) == 0 {
            return Err("the clipboard is busy".into());
        }
        EmptyClipboard();
        let h = GlobalAlloc(GMEM_MOVEABLE, w.len() * 2);
        if h.is_null() {
            CloseClipboard();
            return Err("out of memory".into());
        }
        let p = GlobalLock(h) as *mut u16;
        std::ptr::copy_nonoverlapping(w.as_ptr(), p, w.len());
        GlobalUnlock(h);
        if SetClipboardData(CF_UNICODETEXT, h as _).is_null() {
            GlobalFree(h);
            CloseClipboard();
            return Err("could not set clipboard data".into());
        }
        CloseClipboard();
    }
    Ok(())
}

pub fn get_clipboard_text(hwnd: HWND) -> Option<String> {
    // SAFETY: standard clipboard sequence; the data stays owned by the clipboard.
    unsafe {
        if OpenClipboard(hwnd) == 0 {
            return None;
        }
        let h = GetClipboardData(CF_UNICODETEXT);
        let mut out = None;
        if !h.is_null() {
            let p = GlobalLock(h as _) as *const u16;
            if !p.is_null() {
                let mut len = 0;
                while *p.add(len) != 0 {
                    len += 1;
                }
                out = Some(String::from_utf16_lossy(std::slice::from_raw_parts(p, len)));
                GlobalUnlock(h as _);
            }
        }
        CloseClipboard();
        out
    }
}

#[derive(Clone)]
pub struct Monitor {
    pub handle: HMONITOR,
    pub rect: RECT,
    pub work: RECT,
    pub gdi_name: String,
    pub friendly: String,
    pub dpi: u32,
    pub primary: bool,
}

/// Maps GDI device names (`\\.\DISPLAY2`) to the monitor's own name from its EDID
/// (e.g. "Artist 12").
fn friendly_names() -> HashMap<String, String> {
    let mut map = HashMap::new();
    // SAFETY: sizes come from GetDisplayConfigBufferSizes; structs are POD.
    unsafe {
        let (mut np, mut nm) = (0u32, 0u32);
        if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut np, &mut nm) != 0 {
            return map;
        }
        let mut paths: Vec<DISPLAYCONFIG_PATH_INFO> = vec![std::mem::zeroed(); np as usize];
        let mut modes: Vec<DISPLAYCONFIG_MODE_INFO> = vec![std::mem::zeroed(); nm as usize];
        if QueryDisplayConfig(QDC_ONLY_ACTIVE_PATHS, &mut np, paths.as_mut_ptr(), &mut nm, modes.as_mut_ptr(), std::ptr::null_mut()) != 0 {
            return map;
        }
        for p in &paths[..np as usize] {
            let mut src: DISPLAYCONFIG_SOURCE_DEVICE_NAME = std::mem::zeroed();
            src.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
            src.header.size = std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32;
            src.header.adapterId = p.sourceInfo.adapterId;
            src.header.id = p.sourceInfo.id;
            let mut tgt: DISPLAYCONFIG_TARGET_DEVICE_NAME = std::mem::zeroed();
            tgt.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
            tgt.header.size = std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
            tgt.header.adapterId = p.targetInfo.adapterId;
            tgt.header.id = p.targetInfo.id;
            if DisplayConfigGetDeviceInfo(&mut src.header) == 0 && DisplayConfigGetDeviceInfo(&mut tgt.header) == 0 {
                let name = from_wide(&tgt.monitorFriendlyDeviceName);
                if !name.is_empty() {
                    map.insert(from_wide(&src.viewGdiDeviceName), name);
                }
            }
        }
    }
    map
}

pub fn monitors() -> Vec<Monitor> {
    unsafe extern "system" fn cb(h: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> i32 {
        // SAFETY: data is the Vec passed below.
        let v = unsafe { &mut *(data as *mut Vec<HMONITOR>) };
        v.push(h);
        1
    }
    let mut handles: Vec<HMONITOR> = Vec::new();
    // SAFETY: callback only pushes into the vector.
    unsafe { EnumDisplayMonitors(std::ptr::null_mut(), std::ptr::null(), Some(cb), &mut handles as *mut _ as LPARAM) };
    let names = friendly_names();
    handles
        .into_iter()
        .filter_map(|h| {
            // SAFETY: MONITORINFOEXW with cbSize set.
            unsafe {
                let mut mi: MONITORINFOEXW = std::mem::zeroed();
                mi.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
                if GetMonitorInfoW(h, &mut mi as *mut MONITORINFOEXW as *mut _) == 0 {
                    return None;
                }
                let (mut dx, mut dy) = (96u32, 96u32);
                GetDpiForMonitor(h, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
                let gdi_name = from_wide(&mi.szDevice);
                Some(Monitor {
                    handle: h,
                    rect: mi.monitorInfo.rcMonitor,
                    work: mi.monitorInfo.rcWork,
                    friendly: names.get(&gdi_name).cloned().unwrap_or_default(),
                    gdi_name,
                    dpi: dx,
                    primary: mi.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
                })
            }
        })
        .collect()
}

fn known_folder(id: &windows_sys::core::GUID) -> Option<PathBuf> {
    // SAFETY: SHGetKnownFolderPath allocates the string; freed with CoTaskMemFree.
    unsafe {
        let mut p: *mut u16 = std::ptr::null_mut();
        if SHGetKnownFolderPath(id, KF_FLAG_CREATE as u32, std::ptr::null_mut(), &mut p) != 0 || p.is_null() {
            return None;
        }
        let mut len = 0;
        while *p.add(len) != 0 {
            len += 1;
        }
        let s = String::from_utf16_lossy(std::slice::from_raw_parts(p, len));
        CoTaskMemFree(p as _);
        Some(PathBuf::from(s))
    }
}

/// `Documents\Recedo` (where recordings and reports go).
pub fn output_dir() -> PathBuf {
    known_folder(&FOLDERID_Documents).unwrap_or_else(std::env::temp_dir).join("Recedo")
}

/// `%APPDATA%\Recedo\tablet-test.json`.
pub fn settings_path() -> PathBuf {
    known_folder(&FOLDERID_RoamingAppData).unwrap_or_else(std::env::temp_dir).join("Recedo").join("tablet-test.json")
}

/// Local time as `2026-10-21_14-03-55`.
pub fn timestamp() -> String {
    // SAFETY: out-parameter call.
    let t = unsafe {
        let mut t = std::mem::zeroed();
        GetLocalTime(&mut t);
        t
    };
    format!("{:04}-{:02}-{:02}_{:02}-{:02}-{:02}", t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond)
}

fn reg_string(name: &str) -> Option<String> {
    let key = wide(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion");
    let val = wide(name);
    let mut buf = [0u16; 256];
    let mut size = (buf.len() * 2) as u32;
    // SAFETY: buffer and size match.
    let r = unsafe {
        RegGetValueW(HKEY_LOCAL_MACHINE, key.as_ptr(), val.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), buf.as_mut_ptr().cast(), &mut size)
    };
    (r == 0).then(|| from_wide(&buf))
}

fn reg_dword(name: &str) -> Option<u32> {
    let key = wide(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion");
    let val = wide(name);
    let mut v = 0u32;
    let mut size = 4u32;
    // SAFETY: DWORD-sized buffer.
    let r = unsafe {
        RegGetValueW(HKEY_LOCAL_MACHINE, key.as_ptr(), val.as_ptr(), RRF_RT_REG_DWORD, std::ptr::null_mut(), (&mut v as *mut u32).cast(), &mut size)
    };
    (r == 0).then_some(v)
}

pub fn os_version() -> String {
    let build: u32 = reg_string("CurrentBuildNumber").and_then(|b| b.parse().ok()).unwrap_or(0);
    let ubr = reg_dword("UBR").unwrap_or(0);
    let display = reg_string("DisplayVersion").unwrap_or_default();
    // ProductName still says "Windows 10" on Windows 11; the build number tells them apart.
    let family = if build >= 22000 { "Windows 11" } else { "Windows 10" };
    let edition = reg_string("EditionID").unwrap_or_default();
    format!("{family} {edition} {display} (build {build}.{ubr})")
}

pub fn open_folder(path: &std::path::Path) {
    let w = wide(&path.to_string_lossy());
    let verb = wide("open");
    // SAFETY: valid wide strings.
    unsafe { ShellExecuteW(std::ptr::null_mut(), verb.as_ptr(), w.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL) };
}
