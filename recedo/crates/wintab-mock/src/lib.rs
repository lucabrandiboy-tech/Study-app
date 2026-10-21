//! A stand-in `Wintab32.dll` for automated tests. Never shipped.
//!
//! It implements the Wintab entry points Recedo uses and, when told to, plays a scripted
//! pen stroke into the window that opened the context: it queues packets, moves the system
//! cursor to the pen position (as a real driver does) and posts `WT_PACKET` messages.
//!
//! Packets are encoded here by hand, field by field in spec order, *not* with Recedo's
//! `PacketLayout`, so a layout mistake in the app cannot be hidden by the same mistake in
//! the mock.
//!
//! Point Recedo at it with `RECEDO_WINTAB_DLL=<path to wintab_mock.dll>`.

pub use recedo_tablet::mock_script as script;

#[cfg(windows)]
mod imp {
    use std::collections::VecDeque;
    use std::ffi::c_void;
    use std::sync::Mutex;

    use recedo_tablet::wintab_abi::*;
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, PostMessageW, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SetCursorPos,
    };

    use crate::script;

    const HANDLE_VALUE: usize = 0x5EC0;
    /// The packet fields this mock supports. PK_ROTATION is left out on purpose so the
    /// app's fallback for refused fields is exercised when asked for (it never is today).
    const SUPPORTED: WTPKT = PK_CONTEXT
        | PK_STATUS
        | PK_TIME
        | PK_CHANGED
        | PK_SERIAL_NUMBER
        | PK_CURSOR
        | PK_BUTTONS
        | PK_X
        | PK_Y
        | PK_Z
        | PK_NORMAL_PRESSURE
        | PK_TANGENT_PRESSURE
        | PK_ORIENTATION;

    struct State {
        open: Option<(HWND, LOGCONTEXTW)>,
        queue: VecDeque<Vec<u8>>,
        queue_size: usize,
        serial: u32,
        overflowed: bool,
        /// Refuse WTOpen for these masks (to test the app's fallback).
        refuse_mask: WTPKT,
    }

    // SAFETY: HWND is only used as an opaque value for PostMessageW.
    unsafe impl Send for State {}

    static STATE: Mutex<State> =
        Mutex::new(State { open: None, queue: VecDeque::new(), queue_size: 8, serial: 0, overflowed: false, refuse_mask: 0 });

    fn virtual_screen() -> (i32, i32, i32, i32) {
        // SAFETY: plain metric queries.
        unsafe {
            (
                GetSystemMetrics(SM_XVIRTUALSCREEN),
                GetSystemMetrics(SM_YVIRTUALSCREEN),
                GetSystemMetrics(SM_CXVIRTUALSCREEN),
                GetSystemMetrics(SM_CYVIRTUALSCREEN),
            )
        }
    }

    fn default_context() -> LOGCONTEXTW {
        let (vx, vy, vw, vh) = virtual_screen();
        let mut c = LOGCONTEXTW::default();
        c.set_name("Mock Default System Context");
        c.lcOptions = CXO_SYSTEM;
        c.lcMsgBase = WT_DEFBASE;
        c.lcPktRate = 200;
        c.lcPktData = PK_X | PK_Y | PK_BUTTONS;
        c.lcMoveMask = c.lcPktData;
        c.lcBtnDnMask = 0xFFFF_FFFF;
        c.lcBtnUpMask = 0xFFFF_FFFF;
        c.lcInExtX = script::X_MAX + 1;
        c.lcInExtY = script::Y_MAX + 1;
        c.lcInExtZ = 1024;
        c.lcOutExtX = c.lcInExtX;
        c.lcOutExtY = c.lcInExtY;
        c.lcOutExtZ = 1024;
        c.lcSensX = 1 << 16;
        c.lcSensY = 1 << 16;
        c.lcSensZ = 1 << 16;
        c.lcSysOrgX = vx;
        c.lcSysOrgY = vy;
        c.lcSysExtX = vw;
        c.lcSysExtY = vh;
        c.lcSysSensX = 1 << 16;
        c.lcSysSensY = 1 << 16;
        c
    }

    fn write<T: Copy>(out: *mut c_void, v: &T) -> u32 {
        let size = std::mem::size_of::<T>();
        if !out.is_null() {
            // SAFETY: the caller provides a buffer at least as large as the size we report.
            unsafe { std::ptr::copy_nonoverlapping((v as *const T).cast::<u8>(), out.cast::<u8>(), size) };
        }
        size as u32
    }

    fn write_str(out: *mut c_void, s: &str) -> u32 {
        let w: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
        let size = w.len() * 2;
        if !out.is_null() {
            // SAFETY: as above.
            unsafe { std::ptr::copy_nonoverlapping(w.as_ptr().cast::<u8>(), out.cast::<u8>(), size) };
        }
        size as u32
    }

    /// # Safety
    /// `out` must be null or point to a buffer large enough for the requested item.
    #[unsafe(no_mangle)]
    pub unsafe extern "system" fn WTInfoW(category: u32, index: u32, out: *mut c_void) -> u32 {
        let circle = |max: i32, min: i32| AXIS { axMin: min, axMax: max, axUnits: TU_CIRCLE, axResolution: 3600 << 16 };
        match (category, index) {
            (0, 0) => 1,
            (WTI_INTERFACE, IFC_WINTABID) => write_str(out, "Recedo Mock Wintab"),
            (WTI_INTERFACE, IFC_SPECVERSION) => write(out, &0x0104u16),
            (WTI_INTERFACE, IFC_IMPLVERSION) => write(out, &0x0001u16),
            (WTI_INTERFACE, IFC_NDEVICES) => write(out, &1u32),
            (WTI_INTERFACE, IFC_NCURSORS) => write(out, &3u32),
            (WTI_DEFSYSCTX, 0) | (WTI_DEFCONTEXT, 0) => write(out, &default_context()),
            (WTI_DEVICES, DVC_NAME) => write_str(out, "Recedo Mock Tablet"),
            (WTI_DEVICES, DVC_PKTRATE) => write(out, &200u32),
            (WTI_DEVICES, DVC_PKTDATA) => write(out, &SUPPORTED),
            (WTI_DEVICES, DVC_X) => write(out, &AXIS { axMin: 0, axMax: script::X_MAX, axUnits: TU_INCHES, axResolution: 5080 << 16 }),
            (WTI_DEVICES, DVC_Y) => write(out, &AXIS { axMin: 0, axMax: script::Y_MAX, axUnits: TU_INCHES, axResolution: 5080 << 16 }),
            (WTI_DEVICES, DVC_NPRESSURE) => write(out, &AXIS { axMin: 0, axMax: script::PRESSURE_MAX, axUnits: 0, axResolution: 0 }),
            (WTI_DEVICES, DVC_ORIENTATION) => write(out, &[circle(3599, 0), circle(900, -900), circle(3599, 0)]),
            (c, CSR_NAME) if (WTI_CURSORS..WTI_CURSORS + 3).contains(&c) => {
                write_str(out, ["Mock Puck", "Mock Pen", "Mock Eraser"][(c - WTI_CURSORS) as usize])
            }
            (c, CSR_TYPE) if (WTI_CURSORS..WTI_CURSORS + 3).contains(&c) => write(out, &[0u32, 0x0802, 0x080A][(c - WTI_CURSORS) as usize]),
            (c, CSR_PHYSID) if (WTI_CURSORS..WTI_CURSORS + 3).contains(&c) => write(out, &0x1234u32),
            (c, CSR_BUTTONS) if (WTI_CURSORS..WTI_CURSORS + 3).contains(&c) => write(out, &3u32),
            _ => 0,
        }
    }

    /// # Safety
    /// `ctx` must be null or point to a valid `LOGCONTEXTW`.
    #[unsafe(no_mangle)]
    pub unsafe extern "system" fn WTOpenW(hwnd: HWND, ctx: *mut LOGCONTEXTW, enable: i32) -> *mut c_void {
        let _ = enable;
        if ctx.is_null() {
            return std::ptr::null_mut();
        }
        // SAFETY: caller passes a valid context.
        let c = unsafe { *ctx };
        let mut st = STATE.lock().unwrap();
        if c.lcPktData & !SUPPORTED != 0 || (st.refuse_mask != 0 && c.lcPktData == st.refuse_mask) {
            return std::ptr::null_mut();
        }
        st.open = Some((hwnd, c));
        st.queue.clear();
        st.serial = 0;
        HANDLE_VALUE as *mut c_void
    }

    fn check(h: *mut c_void) -> bool {
        h as usize == HANDLE_VALUE
    }

    #[unsafe(no_mangle)]
    pub extern "system" fn WTClose(h: *mut c_void) -> i32 {
        if !check(h) {
            return 0;
        }
        let mut st = STATE.lock().unwrap();
        st.open = None;
        st.queue.clear();
        1
    }

    /// # Safety
    /// `out` must be null or point to a writable `LOGCONTEXTW`.
    #[unsafe(no_mangle)]
    pub unsafe extern "system" fn WTGetW(h: *mut c_void, out: *mut LOGCONTEXTW) -> i32 {
        let st = STATE.lock().unwrap();
        match (check(h), st.open) {
            (true, Some((_, c))) if !out.is_null() => {
                // SAFETY: caller passes a valid context buffer.
                unsafe { *out = c };
                1
            }
            _ => 0,
        }
    }

    /// # Safety
    /// `out` must hold `max` packets of the open context's packet size.
    #[unsafe(no_mangle)]
    pub unsafe extern "system" fn WTPacketsGet(h: *mut c_void, max: i32, out: *mut c_void) -> i32 {
        if !check(h) || max <= 0 || out.is_null() {
            return 0;
        }
        let mut st = STATE.lock().unwrap();
        let mut n = 0;
        let mut dst = out.cast::<u8>();
        while n < max {
            let Some(p) = st.queue.pop_front() else { break };
            // SAFETY: caller's buffer holds `max` packets of the context's packet size.
            unsafe {
                std::ptr::copy_nonoverlapping(p.as_ptr(), dst, p.len());
                dst = dst.add(p.len());
            }
            n += 1;
        }
        n
    }

    #[unsafe(no_mangle)]
    pub extern "system" fn WTPacket(_h: *mut c_void, _serial: u32, _out: *mut c_void) -> i32 {
        0
    }

    #[unsafe(no_mangle)]
    pub extern "system" fn WTEnable(h: *mut c_void, _enable: i32) -> i32 {
        check(h) as i32
    }

    #[unsafe(no_mangle)]
    pub extern "system" fn WTOverlap(h: *mut c_void, _top: i32) -> i32 {
        check(h) as i32
    }

    #[unsafe(no_mangle)]
    pub extern "system" fn WTQueueSizeGet(h: *mut c_void) -> i32 {
        if !check(h) {
            return 0;
        }
        STATE.lock().unwrap().queue_size as i32
    }

    #[unsafe(no_mangle)]
    pub extern "system" fn WTQueueSizeSet(h: *mut c_void, n: i32) -> i32 {
        // Like some real drivers, refuse very large queues.
        if !check(h) || !(1..=256).contains(&n) {
            return 0;
        }
        STATE.lock().unwrap().queue_size = n as usize;
        1
    }

    /// Encodes one packet for the opened context's mask, field by field in spec order.
    fn encode(mask: WTPKT, f: &Fields) -> Vec<u8> {
        let mut b = Vec::with_capacity(64);
        let u32le = |b: &mut Vec<u8>, v: u32| b.extend_from_slice(&v.to_le_bytes());
        if mask & PK_CONTEXT != 0 {
            b.extend_from_slice(&(HANDLE_VALUE as u64).to_le_bytes());
        }
        if mask & PK_STATUS != 0 {
            u32le(&mut b, f.status);
        }
        if mask & PK_TIME != 0 {
            u32le(&mut b, f.time);
        }
        if mask & PK_CHANGED != 0 {
            u32le(&mut b, mask);
        }
        if mask & PK_SERIAL_NUMBER != 0 {
            u32le(&mut b, f.serial);
        }
        if mask & PK_CURSOR != 0 {
            u32le(&mut b, f.cursor);
        }
        if mask & PK_BUTTONS != 0 {
            u32le(&mut b, f.buttons);
        }
        if mask & PK_X != 0 {
            u32le(&mut b, f.x as u32);
        }
        if mask & PK_Y != 0 {
            u32le(&mut b, f.y as u32);
        }
        if mask & PK_Z != 0 {
            u32le(&mut b, 0);
        }
        if mask & PK_NORMAL_PRESSURE != 0 {
            u32le(&mut b, f.pressure);
        }
        if mask & PK_TANGENT_PRESSURE != 0 {
            u32le(&mut b, 0);
        }
        if mask & PK_ORIENTATION != 0 {
            u32le(&mut b, f.azimuth as u32);
            u32le(&mut b, f.altitude as u32);
            u32le(&mut b, 0);
        }
        // Natural alignment: with a context handle the struct is padded to 8 bytes.
        if mask & PK_CONTEXT != 0 {
            while b.len() % 8 != 0 {
                b.push(0);
            }
        }
        b
    }

    struct Fields {
        status: u32,
        time: u32,
        serial: u32,
        cursor: u32,
        buttons: u32,
        x: i32,
        y: i32,
        pressure: u32,
        azimuth: i32,
        altitude: i32,
    }

    /// Test hook: refuse `WTOpen` for exactly this packet mask (0 = accept everything).
    #[unsafe(no_mangle)]
    pub extern "system" fn wtmock_refuse_mask(mask: u32) {
        STATE.lock().unwrap().refuse_mask = mask;
    }

    /// Test hook: plays the scripted stroke over the given screen rectangle on a
    /// background thread, ~200 packets per second. Returns the number of packets the
    /// script sends (lost packets not counted), or -1 if no context is open.
    #[unsafe(no_mangle)]
    pub extern "system" fn wtmock_play(left: i32, top: i32, width: i32, height: i32) -> i32 {
        let steps = script::stroke(left as f64, top as f64, width as f64, height as f64);
        let sent = steps.iter().filter(|s| !s.drop).count() as i32;
        if STATE.lock().unwrap().open.is_none() {
            return -1;
        }
        std::thread::spawn(move || {
            let mut prox_sent = false;
            for s in steps {
                let (hwnd, ctx) = {
                    let st = STATE.lock().unwrap();
                    match st.open {
                        Some(o) => o,
                        None => return,
                    }
                };
                let hwnd_val = hwnd as usize;
                if !prox_sent {
                    // SAFETY: posting to a window handle owned by the app.
                    unsafe { PostMessageW(hwnd_val as HWND, ctx.lcMsgBase + WT_PROXIMITY, HANDLE_VALUE, 0x0001_0001) };
                    prox_sent = true;
                }
                let sys = (ctx.lcSysOrgX as f64, ctx.lcSysOrgY as f64, ctx.lcSysExtX as f64, ctx.lcSysExtY as f64);
                let (x, y) = script::to_tablet(s.sx, s.sy, sys, (ctx.lcOutOrgX, ctx.lcOutOrgY, ctx.lcOutExtX, ctx.lcOutExtY));
                let (az, alt) = script::orientation(s.tilt_x, s.tilt_y);
                let mut st = STATE.lock().unwrap();
                st.serial = st.serial.wrapping_add(1);
                let serial = st.serial;
                if !s.drop {
                    let mut status = TPS_PROXIMITY;
                    if st.overflowed {
                        status |= TPS_QUEUE_ERR;
                        st.overflowed = false;
                    }
                    let f = Fields {
                        status,
                        // SAFETY: plain call.
                        time: unsafe { GetTickCount() },
                        serial,
                        cursor: 1,
                        buttons: s.tip as u32,
                        x,
                        y,
                        pressure: (s.pressure * script::PRESSURE_MAX as f64).round() as u32,
                        azimuth: az,
                        altitude: alt,
                    };
                    let p = encode(ctx.lcPktData, &f);
                    if st.queue.len() >= st.queue_size {
                        st.queue.pop_front();
                        st.overflowed = true;
                    }
                    st.queue.push_back(p);
                }
                drop(st);
                // A real driver keeps the cursor under the pen.
                // SAFETY: plain calls.
                unsafe {
                    SetCursorPos(s.sx.round() as i32, s.sy.round() as i32);
                    if !s.drop {
                        PostMessageW(hwnd_val as HWND, ctx.lcMsgBase + WT_PACKET, serial as usize, HANDLE_VALUE as isize);
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            let st = STATE.lock().unwrap();
            if let Some((hwnd, ctx)) = st.open {
                // SAFETY: as above.
                unsafe { PostMessageW(hwnd, ctx.lcMsgBase + WT_PROXIMITY, HANDLE_VALUE, 0) };
            }
        });
        sent
    }
}

#[cfg(windows)]
pub use imp::*;
