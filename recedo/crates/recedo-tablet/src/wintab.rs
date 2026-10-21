//! Wintab: the tablet driver's `Wintab32.dll`, loaded at run time.
//!
//! The context is opened the way Qt and Krita open theirs: a copy of the driver's default
//! *system* context (so the driver keeps moving the cursor), with messages turned on,
//! absolute packets, and the output Y axis flipped so Y grows downward. Each `WT_PACKET`
//! message drains the whole queue with `WTPacketsGet`, so every packet the driver queued
//! is used, not one per message or frame.

use std::collections::HashMap;
use std::ffi::c_void;

use windows_sys::Win32::Foundation::{FreeLibrary, HMODULE, HWND, POINT};
use windows_sys::Win32::Graphics::Gdi::{ClientToScreen, GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint};
use windows_sys::Win32::System::LibraryLoader::{GetModuleFileNameW, GetProcAddress, LoadLibraryW};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

use crate::clock::{MsClockMapper, now_us};
use crate::mapping::{Calibrator, MappingMode, Norm, OutputSpace, Rect, Targets};
use crate::orientation::{decode_orientation, has_tilt, azimuth_altitude_to_tilt};
use crate::sample::{Flags, PenSample, Source};
use crate::wintab_abi::*;

pub type HCTX = *mut c_void;

type FnInfo = unsafe extern "system" fn(u32, u32, *mut c_void) -> u32;
type FnOpen = unsafe extern "system" fn(HWND, *mut LOGCONTEXTW, i32) -> HCTX;
type FnClose = unsafe extern "system" fn(HCTX) -> i32;
type FnPacketsGet = unsafe extern "system" fn(HCTX, i32, *mut c_void) -> i32;
type FnEnable = unsafe extern "system" fn(HCTX, i32) -> i32;
type FnOverlap = unsafe extern "system" fn(HCTX, i32) -> i32;
type FnQueueSizeGet = unsafe extern "system" fn(HCTX) -> i32;
type FnQueueSizeSet = unsafe extern "system" fn(HCTX, i32) -> i32;
type FnGet = unsafe extern "system" fn(HCTX, *mut LOGCONTEXTW) -> i32;

/// Environment variable that points Recedo at a different Wintab DLL (used by tests).
pub const DLL_OVERRIDE_ENV: &str = "RECEDO_WINTAB_DLL";

/// The loaded `Wintab32.dll`.
pub struct WintabLib {
    module: HMODULE,
    pub path: String,
    info: FnInfo,
    open: FnOpen,
    close: FnClose,
    packets_get: FnPacketsGet,
    enable: FnEnable,
    overlap: FnOverlap,
    queue_get: FnQueueSizeGet,
    queue_set: FnQueueSizeSet,
    get: Option<FnGet>,
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

impl WintabLib {
    /// Loads Wintab from `RECEDO_WINTAB_DLL` if set, else the system's `Wintab32.dll`.
    pub fn load() -> Result<WintabLib, String> {
        let name = std::env::var(DLL_OVERRIDE_ENV).unwrap_or_else(|_| "Wintab32.dll".to_string());
        let w = wide(&name);
        // SAFETY: valid NUL-terminated wide string.
        let module = unsafe { LoadLibraryW(w.as_ptr()) };
        if module.is_null() {
            return Err(format!(
                "{name} is not installed. Wintab comes with the tablet driver; reinstall the XP-Pen driver to get it."
            ));
        }
        let mut path_buf = [0u16; 1024];
        // SAFETY: buffer and length match.
        let n = unsafe { GetModuleFileNameW(module, path_buf.as_mut_ptr(), path_buf.len() as u32) } as usize;
        let path = String::from_utf16_lossy(&path_buf[..n.min(path_buf.len())]);

        macro_rules! sym {
            ($name:literal, $ty:ty) => {{
                // SAFETY: module is a loaded library and the name is NUL-terminated.
                let p = unsafe { GetProcAddress(module, concat!($name, "\0").as_ptr()) };
                match p {
                    // SAFETY: the Wintab spec fixes these signatures.
                    Some(f) => unsafe { std::mem::transmute::<unsafe extern "system" fn() -> isize, $ty>(f) },
                    None => {
                        // SAFETY: module came from LoadLibraryW.
                        unsafe { FreeLibrary(module) };
                        return Err(format!("{path} does not export {}", $name));
                    }
                }
            }};
        }
        let info = sym!("WTInfoW", FnInfo);
        let open = sym!("WTOpenW", FnOpen);
        let close = sym!("WTClose", FnClose);
        let packets_get = sym!("WTPacketsGet", FnPacketsGet);
        let enable = sym!("WTEnable", FnEnable);
        let overlap = sym!("WTOverlap", FnOverlap);
        let queue_get = sym!("WTQueueSizeGet", FnQueueSizeGet);
        let queue_set = sym!("WTQueueSizeSet", FnQueueSizeSet);
        // SAFETY: as above; WTGetW is optional.
        let get = unsafe { GetProcAddress(module, c"WTGetW".as_ptr().cast()) }
            .map(|f| unsafe { std::mem::transmute::<unsafe extern "system" fn() -> isize, FnGet>(f) });
        Ok(WintabLib { module, path, info, open, close, packets_get, enable, overlap, queue_get, queue_set, get })
    }

    /// Raw `WTInfoW` into a byte buffer sized by the driver.
    pub fn info_bytes(&self, category: u32, index: u32) -> Option<Vec<u8>> {
        // SAFETY: a null output pointer asks only for the size.
        let size = unsafe { (self.info)(category, index, std::ptr::null_mut()) } as usize;
        if size == 0 {
            return None;
        }
        // Some drivers write a little more than they report; leave headroom.
        let mut buf = vec![0u8; size + 64];
        // SAFETY: buffer is at least `size` bytes.
        let got = unsafe { (self.info)(category, index, buf.as_mut_ptr().cast()) } as usize;
        if got == 0 {
            return None;
        }
        buf.truncate(got.max(size));
        Some(buf)
    }

    pub fn info_u32(&self, category: u32, index: u32) -> Option<u32> {
        let b = self.info_bytes(category, index)?;
        Some(match b.len() {
            0 => return None,
            1 => b[0] as u32,
            2 | 3 => u16::from_le_bytes([b[0], b[1]]) as u32,
            _ => u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        })
    }

    pub fn info_string(&self, category: u32, index: u32) -> Option<String> {
        let b = self.info_bytes(category, index)?;
        let wide: Vec<u16> = b.as_chunks::<2>().0.iter().map(|&c| u16::from_le_bytes(c)).collect();
        Some(utf16_until_nul(&wide)).filter(|s| !s.is_empty())
    }

    fn info_struct<T: Copy + Default>(&self, category: u32, index: u32) -> Option<T> {
        let b = self.info_bytes(category, index)?;
        if b.len() < std::mem::size_of::<T>() {
            return None;
        }
        // SAFETY: T is a plain repr(C) Wintab struct and the buffer is large enough.
        Some(unsafe { std::ptr::read_unaligned(b.as_ptr().cast::<T>()) })
    }

    pub fn info_axis(&self, category: u32, index: u32) -> Option<AXIS> {
        self.info_struct(category, index)
    }

    /// True if the Wintab service answers at all.
    pub fn available(&self) -> bool {
        // SAFETY: category 0 returns non-zero if Wintab is available.
        unsafe { (self.info)(0, 0, std::ptr::null_mut()) != 0 }
    }
}

impl Drop for WintabLib {
    fn drop(&mut self) {
        // SAFETY: module came from LoadLibraryW and no context outlives the library
        // (WintabContext owns its WintabLib).
        unsafe { FreeLibrary(self.module) };
    }
}

/// What the driver says about itself.
#[derive(Clone, Debug, Default)]
pub struct DeviceInfo {
    pub dll_path: String,
    pub wintab_id: String,
    pub spec_version: u32,
    pub impl_version: u32,
    pub n_devices: u32,
    pub n_cursors: u32,
    pub device_name: String,
    pub pnp_id: String,
    pub pkt_rate: u32,
    pub pkt_data: u32,
    pub csr_data: u32,
    pub x: AXIS,
    pub y: AXIS,
    pub pressure: AXIS,
    pub orientation: [AXIS; 3],
    pub has_tilt: bool,
}

impl DeviceInfo {
    pub fn query(lib: &WintabLib) -> DeviceInfo {
        let dev = WTI_DEVICES;
        let orientation = lib
            .info_bytes(dev, DVC_ORIENTATION)
            .filter(|b| b.len() >= 48)
            .map(|b| {
                let mut axes = [AXIS::default(); 3];
                for (i, a) in axes.iter_mut().enumerate() {
                    // SAFETY: 3 AXIS structs fit in 48 bytes.
                    *a = unsafe { std::ptr::read_unaligned(b.as_ptr().add(i * 16).cast::<AXIS>()) };
                }
                axes
            })
            .unwrap_or_default();
        DeviceInfo {
            dll_path: lib.path.clone(),
            wintab_id: lib.info_string(WTI_INTERFACE, IFC_WINTABID).unwrap_or_default(),
            spec_version: lib.info_u32(WTI_INTERFACE, IFC_SPECVERSION).unwrap_or(0),
            impl_version: lib.info_u32(WTI_INTERFACE, IFC_IMPLVERSION).unwrap_or(0),
            n_devices: lib.info_u32(WTI_INTERFACE, IFC_NDEVICES).unwrap_or(0),
            n_cursors: lib.info_u32(WTI_INTERFACE, IFC_NCURSORS).unwrap_or(0),
            device_name: lib.info_string(dev, DVC_NAME).unwrap_or_default(),
            pnp_id: lib.info_string(dev, DVC_PNPID).unwrap_or_default(),
            pkt_rate: lib.info_u32(dev, DVC_PKTRATE).unwrap_or(0),
            pkt_data: lib.info_u32(dev, DVC_PKTDATA).unwrap_or(0),
            csr_data: lib.info_u32(dev, DVC_CSRDATA).unwrap_or(0),
            x: lib.info_axis(dev, DVC_X).unwrap_or_default(),
            y: lib.info_axis(dev, DVC_Y).unwrap_or_default(),
            pressure: lib.info_axis(dev, DVC_NPRESSURE).unwrap_or_default(),
            has_tilt: has_tilt(&orientation),
            orientation,
        }
    }

    pub fn version_string(v: u32) -> String {
        format!("{}.{}", (v >> 8) & 0xFF, v & 0xFF)
    }
}

/// A cursor (pen tip, eraser, puck...) as described by the driver.
#[derive(Clone, Debug, Default)]
pub struct CursorInfo {
    pub index: u32,
    pub name: String,
    pub csr_type: u32,
    pub physid: u32,
    pub buttons: u32,
}

/// An open Wintab context.
pub struct WintabContext {
    pub hctx: HCTX,
    pub ctx: LOGCONTEXTW,
    pub layout: PacketLayout,
    pub queue_size: i32,
    pub device: DeviceInfo,
    /// Masks tried before one opened, with the result of each.
    pub open_log: Vec<String>,
    buf: Vec<u8>,
    cursors: HashMap<u32, CursorInfo>,
    /// Kept loaded while the context is open; `Drop` closes the context first.
    lib: WintabLib,
}

impl WintabContext {
    pub fn open(lib: WintabLib, hwnd: HWND) -> Result<WintabContext, String> {
        if !lib.available() {
            return Err(format!("{} is installed but the Wintab service does not respond (is the tablet driver running?)", lib.path));
        }
        let device = DeviceInfo::query(&lib);
        let base: LOGCONTEXTW = lib
            .info_struct(WTI_DEFSYSCTX, 0)
            .ok_or_else(|| "the driver has no default system context (WTI_DEFSYSCTX)".to_string())?;
        let mut open_log = Vec::new();
        let mut opened = None;
        for mask in [PACKETDATA_FULL, PACKETDATA_QT, PACKETDATA_MIN] {
            let mut ctx = base;
            ctx.set_name("Recedo");
            ctx.lcOptions |= CXO_MESSAGES | CXO_CSRMESSAGES;
            ctx.lcMsgBase = WT_DEFBASE;
            ctx.lcPktData = mask;
            ctx.lcPktMode = 0; // absolute values for everything
            ctx.lcMoveMask = mask;
            ctx.lcBtnUpMask = ctx.lcBtnDnMask;
            ctx.lcOutOrgX = 0;
            ctx.lcOutExtX = ctx.lcInExtX;
            ctx.lcOutOrgY = 0;
            ctx.lcOutExtY = -ctx.lcInExtY;
            // SAFETY: valid window and context struct.
            let h = unsafe { (lib.open)(hwnd, &mut ctx, 1) };
            if h.is_null() {
                open_log.push(format!("WTOpen with {} failed", describe_mask(mask)));
                continue;
            }
            let mut actual = ctx;
            if let Some(get) = lib.get {
                // SAFETY: h is an open context.
                if unsafe { get(h, &mut actual) } == 0 {
                    actual = ctx;
                }
            }
            open_log.push(format!("WTOpen with {} succeeded; context reports {}", describe_mask(mask), describe_mask(actual.lcPktData)));
            opened = Some((h, actual));
            break;
        }
        let (hctx, ctx) = opened.ok_or_else(|| format!("the driver refused to open a context: {}", open_log.join("; ")))?;

        // A failed WTQueueSizeSet deletes the queue, so keep trying smaller sizes.
        let mut queue_size = 0;
        for q in [1024, 512, 256, 128, 64, 32, 16, 8] {
            // SAFETY: open context.
            if unsafe { (lib.queue_set)(hctx, q) } != 0 {
                queue_size = q;
                break;
            }
        }
        if queue_size == 0 {
            // SAFETY: open context.
            unsafe { (lib.close)(hctx) };
            return Err("the driver would not create a packet queue".to_string());
        }
        // SAFETY: open context.
        let reported = unsafe { (lib.queue_get)(hctx) };
        if reported > 0 {
            queue_size = reported;
        }
        let layout = PacketLayout::new(ctx.lcPktData);
        // SAFETY: open context.
        unsafe {
            (lib.enable)(hctx, 1);
            (lib.overlap)(hctx, 1);
        }
        Ok(WintabContext {
            hctx,
            ctx,
            layout,
            queue_size,
            device,
            open_log,
            buf: vec![0u8; layout.size * queue_size as usize],
            cursors: HashMap::new(),
            lib,
        })
    }

    pub fn dll_path(&self) -> &str {
        &self.lib.path
    }

    /// Raw access to the loaded DLL (the self-test talks to the mock driver through it).
    pub fn proc_address(&self, name: &std::ffi::CStr) -> Option<unsafe extern "system" fn() -> isize> {
        // SAFETY: module is loaded for the lifetime of self.
        unsafe { GetProcAddress(self.lib.module, name.as_ptr().cast()) }
    }

    /// Reads every queued packet.
    pub fn drain(&mut self) -> Vec<RawPacket> {
        let mut out = Vec::new();
        loop {
            // SAFETY: buffer holds queue_size packets of layout.size bytes.
            let n = unsafe { (self.lib.packets_get)(self.hctx, self.queue_size, self.buf.as_mut_ptr().cast()) };
            if n <= 0 {
                break;
            }
            let n = (n as usize).min(self.queue_size as usize);
            for i in 0..n {
                out.push(self.layout.decode(&self.buf[i * self.layout.size..(i + 1) * self.layout.size]));
            }
            if n < self.queue_size as usize {
                break;
            }
        }
        out
    }

    /// Call on `WM_ACTIVATE`: brings the context on top when the window is active.
    pub fn on_activate(&self, active: bool) {
        // SAFETY: open context.
        unsafe {
            (self.lib.enable)(self.hctx, active as i32);
            if active {
                (self.lib.overlap)(self.hctx, 1);
            }
        }
    }

    pub fn cursor(&mut self, index: u32) -> &CursorInfo {
        let lib = &self.lib;
        self.cursors.entry(index).or_insert_with(|| CursorInfo {
            index,
            name: lib.info_string(WTI_CURSORS + index, CSR_NAME).unwrap_or_default(),
            csr_type: lib.info_u32(WTI_CURSORS + index, CSR_TYPE).unwrap_or(0),
            physid: lib.info_u32(WTI_CURSORS + index, CSR_PHYSID).unwrap_or(0),
            buttons: lib.info_u32(WTI_CURSORS + index, CSR_BUTTONS).unwrap_or(0),
        })
    }

    pub fn known_cursors(&self) -> impl Iterator<Item = &CursorInfo> {
        self.cursors.values()
    }

    /// The `lcSys*` rectangle in screen pixels.
    pub fn system_rect(&self) -> Rect {
        let c = &self.ctx;
        let (mut x, mut w) = (c.lcSysOrgX as f64, c.lcSysExtX as f64);
        let (mut y, mut h) = (c.lcSysOrgY as f64, c.lcSysExtY as f64);
        if w < 0.0 {
            x += w;
            w = -w;
        }
        if h < 0.0 {
            y += h;
            h = -h;
        }
        Rect::new(x, y, w, h)
    }

    pub fn output_space(&self) -> OutputSpace {
        let c = &self.ctx;
        OutputSpace { org_x: c.lcOutOrgX, org_y: c.lcOutOrgY, ext_x: c.lcOutExtX, ext_y: c.lcOutExtY }
    }
}

impl Drop for WintabContext {
    fn drop(&mut self) {
        // SAFETY: open context; the library is still loaded (dropped after this).
        unsafe { (self.lib.close)(self.hctx) };
    }
}

/// Screen rectangles for the mapping candidates, read from Win32.
pub fn current_targets(system_context: Rect, cursor: POINT) -> Targets {
    // SAFETY: plain metric queries.
    let virtual_desktop = unsafe {
        Rect::new(
            GetSystemMetrics(SM_XVIRTUALSCREEN) as f64,
            GetSystemMetrics(SM_YVIRTUALSCREEN) as f64,
            GetSystemMetrics(SM_CXVIRTUALSCREEN) as f64,
            GetSystemMetrics(SM_CYVIRTUALSCREEN) as f64,
        )
    };
    // SAFETY: MONITORINFO with cbSize set; MonitorFromPoint never fails with DEFAULTTONEAREST.
    let monitor = unsafe {
        let hmon = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        (GetMonitorInfoW(hmon, &mut mi) != 0).then(|| {
            let r = mi.rcMonitor;
            Rect::new(r.left as f64, r.top as f64, (r.right - r.left) as f64, (r.bottom - r.top) as f64)
        })
    };
    Targets { system_context, virtual_desktop, monitor }
}

pub fn client_origin(hwnd: HWND) -> (f64, f64) {
    let mut p = POINT { x: 0, y: 0 };
    // SAFETY: valid window.
    unsafe { ClientToScreen(hwnd, &mut p) };
    (p.x as f64, p.y as f64)
}

pub fn cursor_pos() -> POINT {
    let mut p = POINT { x: 0, y: 0 };
    // SAFETY: out-parameter call.
    unsafe { GetCursorPos(&mut p) };
    p
}

/// Wintab packets turned into [`PenSample`]s.
pub struct WintabInput {
    pub ctx: WintabContext,
    pub calibrator: Calibrator,
    pub targets: Targets,
    clock: MsClockMapper,
    proximity_pending: bool,
    pub in_proximity: bool,
    /// The newest packet, for the diagnostics panel.
    pub last_packet: Option<RawPacket>,
    pub last_norm: Option<Norm>,
}

impl WintabInput {
    pub fn open(hwnd: HWND, mode: MappingMode) -> Result<WintabInput, String> {
        let lib = WintabLib::load()?;
        let ctx = WintabContext::open(lib, hwnd)?;
        let targets = current_targets(ctx.system_rect(), cursor_pos());
        Ok(WintabInput {
            ctx,
            calibrator: Calibrator::new(mode),
            targets,
            clock: MsClockMapper::default(),
            proximity_pending: true,
            in_proximity: false,
            last_packet: None,
            last_norm: None,
        })
    }

    /// `WT_PROXIMITY`: `lparam` low word non-zero = entering the context.
    pub fn on_proximity(&mut self, lparam: isize) {
        let entering = (lparam & 0xFFFF) != 0;
        self.in_proximity = entering;
        if entering {
            self.proximity_pending = true;
        }
    }

    /// Drains the queue (call on `WT_PACKET`).
    pub fn process(&mut self, hwnd: HWND) -> Vec<PenSample> {
        let packets = self.ctx.drain();
        if packets.is_empty() {
            return Vec::new();
        }
        let recv = now_us();
        let cursor = cursor_pos();
        self.targets = current_targets(self.ctx.system_rect(), cursor);
        let origin = client_origin(hwnd);
        let space = self.ctx.output_space();
        let layout = self.ctx.layout;
        let dev = self.ctx.device.clone();
        let p_min = dev.pressure.axMin as f64;
        let p_span = (dev.pressure.axMax - dev.pressure.axMin).max(1) as f64;
        let has_pressure = layout.has(PK_NORMAL_PRESSURE) && dev.pressure.axMax > dev.pressure.axMin;
        let has_orientation = layout.has(PK_ORIENTATION) && dev.has_tilt;
        let has_twist = layout.has(PK_ORIENTATION) && dev.orientation[2].axMax > dev.orientation[2].axMin;
        let eraser_by_index = dev.n_cursors >= 3;

        let mut out = Vec::with_capacity(packets.len());
        for p in &packets {
            let norm = space.normalize(p.x, p.y);
            let (sx, sy) = self.calibrator.map(norm, &self.targets);
            let pressure = if has_pressure { ((p.normal_pressure as f64 - p_min) / p_span).clamp(0.0, 1.0) as f32 } else { 0.0 };
            let tip = p.buttons & 1 != 0;
            let contact = tip || (has_pressure && p.normal_pressure as f64 > p_min);
            let mut flags = Flags(0);
            flags.set(Flags::CONTACT, contact);
            flags.set(Flags::HAS_PRESSURE, has_pressure);
            flags.set(Flags::BARREL1, p.buttons & 2 != 0);
            flags.set(Flags::BARREL2, p.buttons & 4 != 0);
            flags.set(Flags::QUEUE_ERROR, layout.has(PK_STATUS) && p.status & TPS_QUEUE_ERR != 0);
            let inverted = layout.has(PK_STATUS) && p.status & TPS_INVERT != 0;
            flags.set(Flags::ERASER, inverted || (eraser_by_index && p.cursor % 3 == 2));
            let (mut tilt_x, mut tilt_y, mut twist) = (0.0, 0.0, 0.0);
            if has_orientation {
                let o = decode_orientation([p.orientation.orAzimuth, p.orientation.orAltitude, p.orientation.orTwist], &dev.orientation);
                let (tx, ty) = azimuth_altitude_to_tilt(o.azimuth, o.altitude);
                tilt_x = tx as f32;
                tilt_y = ty as f32;
                flags.set(Flags::HAS_TILT, true);
                if has_twist {
                    twist = o.twist as f32;
                    flags.set(Flags::HAS_TWIST, true);
                }
            }
            if self.proximity_pending {
                flags.set(Flags::PROXIMITY_IN, true);
                self.proximity_pending = false;
                self.clock.reset();
            }
            let t_us = if layout.has(PK_TIME) { self.clock.map(p.time, recv) } else { recv };
            out.push(PenSample {
                source: Source::Wintab,
                t_us,
                recv_us: recv,
                x: sx - origin.0,
                y: sy - origin.1,
                pressure,
                tilt_x,
                tilt_y,
                twist,
                flags,
                raw_pressure: p.normal_pressure,
                raw_buttons: p.buttons,
                serial: p.serial,
                cursor: p.cursor,
            });
        }
        let last = *packets.last().unwrap();
        let norm = space.normalize(last.x, last.y);
        self.calibrator.observe(norm, (cursor.x as f64, cursor.y as f64), &self.targets);
        self.last_packet = Some(last);
        self.last_norm = Some(norm);
        self.ctx.cursor(last.cursor);
        self.in_proximity = true;
        out
    }
}
