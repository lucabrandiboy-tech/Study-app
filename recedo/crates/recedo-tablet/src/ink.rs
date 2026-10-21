//! Windows Ink: pen input from `WM_POINTER*` messages.
//!
//! Each message carries a history of every frame coalesced since the previous one;
//! [`InkInput::handle`] reads all of them with `GetPointerPenInfoHistory` (oldest first).
//! Positions come from the HIMETRIC location mapped through `GetPointerDeviceRects`, which
//! has sub-pixel precision; the integer pixel location is the fallback.
//!
//! Windows normalises Ink pressure to 0..=1024, so a pen with 8192 or 16384 levels gives at
//! most 1025 distinct values through this API (Krita has the same limit in Ink mode).

use std::collections::HashMap;

use windows_sys::Win32::Foundation::{HANDLE, HWND, RECT};
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows_sys::Win32::UI::Controls::{
    FEEDBACK_PEN_BARRELVISUALIZATION, FEEDBACK_PEN_DOUBLETAP, FEEDBACK_PEN_PRESSANDHOLD, FEEDBACK_PEN_RIGHTTAP, FEEDBACK_PEN_TAP,
    POINTER_DEVICE_INFO, POINTER_DEVICE_TYPE_EXTERNAL_PEN, POINTER_DEVICE_TYPE_INTEGRATED_PEN,
};
use windows_sys::Win32::UI::Input::Pointer::{
    GetPointerDeviceRects, GetPointerInfo, GetPointerPenInfo, GetPointerPenInfoHistory, GetPointerType, POINTER_FLAG_INCONTACT,
    POINTER_FLAG_NEW, POINTER_FLAG_SECONDBUTTON, POINTER_FLAG_THIRDBUTTON, POINTER_INFO, POINTER_PEN_INFO,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    PEN_FLAG_BARREL, PEN_FLAG_ERASER, PEN_FLAG_INVERTED, PEN_MASK_PRESSURE, PEN_MASK_ROTATION, PEN_MASK_TILT_X, PEN_MASK_TILT_Y,
    PT_PEN, SetPropW, WM_POINTERDOWN, WM_POINTERENTER, WM_POINTERLEAVE, WM_POINTERUP, WM_POINTERUPDATE,
};

use crate::clock::{now_us, qpc_to_us};
use crate::sample::{Flags, FrameFilter, PenSample, Source};
use crate::wintab::client_origin;

/// What a pointer message turned into.
#[derive(Clone, Debug, PartialEq)]
pub enum InkEvent {
    /// All frames carried by the message, oldest first.
    Samples(Vec<PenSample>),
    /// The pen left the window or went out of range.
    Leave { pointer_id: u32 },
}

/// Largest history read per message. Windows keeps far fewer frames than this.
const MAX_HISTORY: u32 = 1024;

#[derive(Default)]
pub struct InkInput {
    rects: HashMap<isize, Option<(RECT, RECT)>>,
    buf: Vec<POINTER_PEN_INFO>,
    /// Largest number of frames one message carried.
    pub max_history: u32,
    /// Samples that used the integer pixel position because the HIMETRIC one disagreed.
    pub coarse_fallbacks: u64,
    /// Frames already delivered (Windows can send the same frame in two messages).
    frames: FrameFilter,
    /// Repeated frames that were dropped.
    pub repeats_dropped: u64,
}

fn device_rects(device: HANDLE) -> Option<(RECT, RECT)> {
    let mut dev = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    let mut disp = dev;
    // SAFETY: out-parameter call with a device handle from the system.
    let ok = unsafe { GetPointerDeviceRects(device, &mut dev, &mut disp) } != 0;
    (ok && dev.right > dev.left && dev.bottom > dev.top && disp.right > disp.left && disp.bottom > disp.top).then_some((dev, disp))
}

impl InkInput {
    pub fn new() -> InkInput {
        InkInput::default()
    }

    /// Forget cached device rectangles (call after display changes).
    pub fn reset_devices(&mut self) {
        self.rects.clear();
    }

    /// Handles a `WM_POINTER*` message. Returns `None` for anything that is not a pen.
    pub fn handle(&mut self, hwnd: HWND, msg: u32, wparam: usize) -> Option<InkEvent> {
        let id = (wparam & 0xFFFF) as u32;
        let mut ty = 0;
        // SAFETY: out-parameter call.
        if unsafe { GetPointerType(id, &mut ty) } == 0 || ty != PT_PEN {
            return None;
        }
        if msg == WM_POINTERLEAVE {
            self.frames.forget(id);
            return Some(InkEvent::Leave { pointer_id: id });
        }
        if !matches!(msg, WM_POINTERDOWN | WM_POINTERUPDATE | WM_POINTERUP | WM_POINTERENTER) {
            return None;
        }
        let recv = now_us();
        // SAFETY: zeroed POD out-parameter.
        let mut info: POINTER_INFO = unsafe { std::mem::zeroed() };
        // SAFETY: out-parameter call.
        let history = if unsafe { GetPointerInfo(id, &mut info) } != 0 { info.historyCount.clamp(1, MAX_HISTORY) } else { 1 };
        // SAFETY: zeroed POD.
        self.buf.resize(history as usize, unsafe { std::mem::zeroed() });
        let mut count = history;
        // SAFETY: buffer holds `count` entries.
        let mut n = if unsafe { GetPointerPenInfoHistory(id, &mut count, self.buf.as_mut_ptr()) } != 0 {
            count.min(history) as usize
        } else {
            0
        };
        if n == 0 {
            // SAFETY: buffer holds at least one entry.
            if unsafe { GetPointerPenInfo(id, self.buf.as_mut_ptr()) } == 0 {
                return None;
            }
            n = 1;
        }
        self.max_history = self.max_history.max(n as u32);
        let origin = client_origin(hwnd);
        let mut out = Vec::with_capacity(n);
        // History is newest first.
        for i in (0..n).rev() {
            let pen = self.buf[i];
            if !self.frames.is_new(pen.pointerInfo.pointerId, pen.pointerInfo.frameId) {
                self.repeats_dropped += 1;
                continue;
            }
            out.push(self.convert(&pen, origin, recv, msg == WM_POINTERENTER && i == n - 1));
        }
        Some(InkEvent::Samples(out))
    }

    fn convert(&mut self, pen: &POINTER_PEN_INFO, origin: (f64, f64), recv: u64, entering: bool) -> PenSample {
        let pi = &pen.pointerInfo;
        let px = (pi.ptPixelLocation.x as f64, pi.ptPixelLocation.y as f64);
        let rects = *self.rects.entry(pi.sourceDevice as isize).or_insert_with(|| device_rects(pi.sourceDevice));
        let mut flags = Flags(0);
        let (sx, sy) = match rects {
            Some((dev, disp)) => {
                let fx = (pi.ptHimetricLocation.x - dev.left) as f64 / (dev.right - dev.left) as f64;
                let fy = (pi.ptHimetricLocation.y - dev.top) as f64 / (dev.bottom - dev.top) as f64;
                let sub = (disp.left as f64 + fx * (disp.right - disp.left) as f64, disp.top as f64 + fy * (disp.bottom - disp.top) as f64);
                // The pixel location is the sub-pixel one rounded; anything further off means
                // the HIMETRIC mapping does not describe this device correctly.
                if (sub.0 - px.0).abs() <= 1.5 && (sub.1 - px.1).abs() <= 1.5 {
                    sub
                } else {
                    self.coarse_fallbacks += 1;
                    flags.set(Flags::COARSE_POSITION, true);
                    px
                }
            }
            None => {
                self.coarse_fallbacks += 1;
                flags.set(Flags::COARSE_POSITION, true);
                px
            }
        };
        let contact = pi.pointerFlags & POINTER_FLAG_INCONTACT != 0;
        flags.set(Flags::CONTACT, contact);
        let has_pressure = pen.penMask & PEN_MASK_PRESSURE != 0;
        flags.set(Flags::HAS_PRESSURE, has_pressure);
        let pressure = if has_pressure { (pen.pressure.min(1024) as f32) / 1024.0 } else if contact { 1.0 } else { 0.0 };
        let has_tilt = pen.penMask & (PEN_MASK_TILT_X | PEN_MASK_TILT_Y) != 0;
        flags.set(Flags::HAS_TILT, has_tilt);
        let has_twist = pen.penMask & PEN_MASK_ROTATION != 0;
        flags.set(Flags::HAS_TWIST, has_twist);
        flags.set(Flags::ERASER, pen.penFlags & (PEN_FLAG_INVERTED | PEN_FLAG_ERASER) != 0);
        flags.set(Flags::BARREL1, pen.penFlags & PEN_FLAG_BARREL != 0 || pi.pointerFlags & POINTER_FLAG_SECONDBUTTON != 0);
        flags.set(Flags::BARREL2, pi.pointerFlags & POINTER_FLAG_THIRDBUTTON != 0);
        flags.set(Flags::PROXIMITY_IN, entering || pi.pointerFlags & POINTER_FLAG_NEW != 0);
        let t_us = if pi.PerformanceCount != 0 { qpc_to_us(pi.PerformanceCount) } else { recv };
        PenSample {
            source: Source::Ink,
            t_us,
            recv_us: recv,
            x: sx - origin.0,
            y: sy - origin.1,
            pressure,
            tilt_x: if pen.penMask & PEN_MASK_TILT_X != 0 { pen.tiltX as f32 } else { 0.0 },
            tilt_y: if pen.penMask & PEN_MASK_TILT_Y != 0 { pen.tiltY as f32 } else { 0.0 },
            twist: if has_twist { pen.rotation as f32 } else { 0.0 },
            flags,
            raw_pressure: pen.pressure,
            raw_buttons: pen.penFlags,
            serial: pi.frameId,
            cursor: pi.pointerId,
        }
    }
}

/// Looks up an optional `user32.dll` function at run time, so a Windows build (or Wine)
/// without it reports it missing instead of crashing.
pub fn user32_proc(name: &std::ffi::CStr) -> Option<unsafe extern "system" fn() -> isize> {
    let module: Vec<u16> = "user32.dll\0".encode_utf16().collect();
    // SAFETY: user32 is always loaded in a GUI process; the name is NUL-terminated.
    unsafe {
        let h = GetModuleHandleW(module.as_ptr());
        if h.is_null() {
            return None;
        }
        GetProcAddress(h, name.as_ptr().cast())
    }
}

type FnSetWindowFeedbackSetting = unsafe extern "system" fn(HWND, i32, u32, u32, *const std::ffi::c_void) -> i32;
type FnGetPointerDevices = unsafe extern "system" fn(*mut u32, *mut POINTER_DEVICE_INFO) -> i32;

/// Turns off Windows' pen feedback and gestures for the window: tap ripples, the
/// press-and-hold right-click ring, barrel-button visuals and flicks. A painting app needs
/// the raw pen; these add delays and pop-ups mid-stroke.
pub fn disable_pen_gestures(hwnd: HWND) {
    let off: i32 = 0;
    if let Some(f) = user32_proc(c"SetWindowFeedbackSetting") {
        // SAFETY: the documented signature of SetWindowFeedbackSetting.
        let set = unsafe { std::mem::transmute::<unsafe extern "system" fn() -> isize, FnSetWindowFeedbackSetting>(f) };
        for f in [FEEDBACK_PEN_BARRELVISUALIZATION, FEEDBACK_PEN_TAP, FEEDBACK_PEN_DOUBLETAP, FEEDBACK_PEN_PRESSANDHOLD, FEEDBACK_PEN_RIGHTTAP] {
            // SAFETY: valid window and a BOOL-sized value.
            unsafe { set(hwnd, f, 0, std::mem::size_of::<i32>() as u32, (&off as *const i32).cast()) };
        }
    }
    const TABLET_DISABLE_PRESSANDHOLD: usize = 0x0000_0001;
    const TABLET_DISABLE_PENTAPFEEDBACK: usize = 0x0000_0008;
    const TABLET_DISABLE_PENBARRELFEEDBACK: usize = 0x0000_0010;
    const TABLET_DISABLE_FLICKS: usize = 0x0001_0000;
    const TABLET_DISABLE_FLICKFALLBACKKEYS: usize = 0x0010_0000;
    let value = TABLET_DISABLE_PRESSANDHOLD
        | TABLET_DISABLE_PENTAPFEEDBACK
        | TABLET_DISABLE_PENBARRELFEEDBACK
        | TABLET_DISABLE_FLICKS
        | TABLET_DISABLE_FLICKFALLBACKKEYS;
    let name: Vec<u16> = "MicrosoftTabletPenServiceProperty\0".encode_utf16().collect();
    // SAFETY: valid window and NUL-terminated name; the "handle" is a flag value by design.
    unsafe { SetPropW(hwnd, name.as_ptr(), value as HANDLE) };
}

/// A pen digitizer Windows knows about.
#[derive(Clone, Debug)]
pub struct PenDevice {
    pub product: String,
    pub integrated: bool,
    pub himetric_rect: Option<(i32, i32, i32, i32)>,
    pub display_rect: Option<(i32, i32, i32, i32)>,
}

/// Lists pen digitizers (for the diagnostics panel).
pub fn pen_devices() -> Vec<PenDevice> {
    let Some(f) = user32_proc(c"GetPointerDevices") else { return Vec::new() };
    // SAFETY: the documented signature of GetPointerDevices.
    let get_devices = unsafe { std::mem::transmute::<unsafe extern "system" fn() -> isize, FnGetPointerDevices>(f) };
    let mut count = 0u32;
    // SAFETY: null buffer asks for the count.
    if unsafe { get_devices(&mut count, std::ptr::null_mut()) } == 0 || count == 0 {
        return Vec::new();
    }
    // SAFETY: zeroed POD.
    let mut v: Vec<POINTER_DEVICE_INFO> = vec![unsafe { std::mem::zeroed() }; count as usize];
    // SAFETY: buffer holds `count` entries.
    if unsafe { get_devices(&mut count, v.as_mut_ptr()) } == 0 {
        return Vec::new();
    }
    v.truncate(count as usize);
    v.iter()
        .filter(|d| d.pointerDeviceType == POINTER_DEVICE_TYPE_INTEGRATED_PEN || d.pointerDeviceType == POINTER_DEVICE_TYPE_EXTERNAL_PEN)
        .map(|d| {
            let r = device_rects(d.device);
            PenDevice {
                product: crate::wintab_abi::utf16_until_nul(&d.productString),
                integrated: d.pointerDeviceType == POINTER_DEVICE_TYPE_INTEGRATED_PEN,
                himetric_rect: r.map(|(a, _)| (a.left, a.top, a.right, a.bottom)),
                display_rect: r.map(|(_, b)| (b.left, b.top, b.right, b.bottom)),
            }
        })
        .collect()
}
