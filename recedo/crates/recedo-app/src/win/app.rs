//! Recedo on Windows: owns the window's GPU surface, the session (editor and interface) and
//! the tablet inputs, and turns window messages into pen samples and interface input.
//!
//! Where pen input goes:
//! * Windows Ink (`WM_POINTER*`) or Wintab packets, whichever the settings choose, paint.
//!   (If Wintab cannot be opened, Windows Ink is used.)
//! * The same pen also moves the system cursor, and the mouse messages Windows makes from
//!   it drive the interface (menus, sliders). Those never paint.
//! * A real mouse paints with the left button and pans with the middle button, but only
//!   while no pen is near the tablet.

use std::num::NonZeroIsize;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use recedo_render::gpu::Gpu;
use recedo_tablet::clock::{MsClockMapper, now_us};
use recedo_tablet::ink::{InkEvent, InkInput, disable_pen_gestures, pen_devices};
use recedo_tablet::mapping::MappingMode;
use recedo_tablet::wintab::WintabInput;
use recedo_tablet::wintab_abi::{WT_DEFBASE, WT_MAXOFFSET, WT_PACKET, WT_PROXIMITY};
use recedo_tablet::{Flags, PenSample, Source};
use recedo_win::egui_input::{EguiInput, system_cursor};
use recedo_win::sys;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{ScreenToClient, ValidateRect};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_MENU, VK_SHIFT, VK_SPACE};
use windows_sys::Win32::UI::Shell::{DragAcceptFiles, DragFinish, DragQueryFileW, HDROP};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::editor::{ExportFormat, Modifiers, Notice, NoticeKind, Tool};
use crate::session::{Renderer, Session, Target};
use crate::settings::{Settings, TabletApi};
use crate::ui::Request;

/// Things that run outside the message handler: Windows' file dialogs run their own message
/// loop, which sends messages back to this window while they are open.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Deferred {
    Open,
    SaveAs,
    Export(ExportFormat),
}

const TIMER_SIZEMOVE: usize = 1;
// Not in windows-sys's WindowsAndMessaging module.
const WM_MOUSELEAVE: u32 = 0x02A3;
const MK_LBUTTON: u32 = 0x0001;
const MK_MBUTTON: u32 = 0x0010;
/// How often the editor is checked while idle (autosave, background saves).
const IDLE_TICK: Duration = Duration::from_secs(1);
/// After a tablet packet, the mouse is ignored this long (the driver's own cursor moves
/// arrive as mouse messages).
const TABLET_QUIET: Duration = Duration::from_millis(300);
/// A pen said to be in range, with no word from it for this long, no longer blocks the mouse
/// (in case a "left range" message was lost).
const IN_RANGE_TIMEOUT: Duration = Duration::from_secs(2);

fn lparam_pos(lparam: isize) -> (i32, i32) {
    ((lparam & 0xFFFF) as i16 as i32, ((lparam >> 16) & 0xFFFF) as i16 as i32)
}

/// Mouse messages that Windows made from pen or touch input carry this signature.
fn mouse_from_pen_or_touch() -> bool {
    // SAFETY: plain call.
    let info = unsafe { GetMessageExtraInfo() } as usize;
    info & 0xFFFF_FF00 == 0xFF51_5700
}

fn key_down(vk: u16) -> bool {
    // SAFETY: plain key state query.
    unsafe { GetKeyState(vk as i32) as u16 & 0x8000 != 0 }
}

/// Keys held right now (as of the message being handled). Space is tracked by the app
/// itself (see `App::space_held`).
fn modifiers(space: bool) -> Modifiers {
    Modifiers { ctrl: key_down(VK_CONTROL), shift: key_down(VK_SHIFT), alt: key_down(VK_MENU), space }
}

pub fn client_size(hwnd: HWND) -> (u32, u32) {
    let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    // SAFETY: valid window.
    unsafe { GetClientRect(hwnd, &mut r) };
    ((r.right - r.left).max(0) as u32, (r.bottom - r.top).max(0) as u32)
}

/// The mouse pointer in client pixels.
fn cursor_in_client(hwnd: HWND) -> (f64, f64) {
    let mut p = POINT { x: 0, y: 0 };
    // SAFETY: out-parameter calls on a valid window.
    unsafe {
        GetCursorPos(&mut p);
        ScreenToClient(hwnd, &mut p);
    }
    (p.x as f64 + 0.5, p.y as f64 + 0.5)
}

/// `%APPDATA%\Recedo\recedo.json`.
pub fn settings_path() -> PathBuf {
    sys::roaming_app_data_dir().join("Recedo").join("recedo.json")
}

/// `%LOCALAPPDATA%\Recedo\recovery` (autosaved copies of unsaved paintings).
pub fn recovery_dir() -> PathBuf {
    sys::local_app_data_dir().join("Recedo").join("recovery")
}

/// Frame counters, for the self-test.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameStats {
    pub frames: u64,
    /// Time spent on the last frame before handing it to the GPU, in ms.
    pub last_work_ms: f64,
}

pub struct App {
    pub hwnd: HWND,
    pub gpu: Gpu,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    pub session: Session,
    pub renderer: Renderer,
    egui_in: EguiInput,
    pub wintab: Option<WintabInput>,
    pub ink: InkInput,
    /// The Ink pen is near the screen, and when it was last heard from.
    ink_in_range: Option<Instant>,
    last_wintab_packet: Option<Instant>,
    mouse_clock: MsClockMapper,
    /// The mouse button that is painting or panning (`MK_LBUTTON` / `MK_MBUTTON`).
    mouse_held: Option<u32>,
    /// Space is held as the pan key: from its own key messages, so Space typed into a text
    /// box never pans (and so it is right under Wine too, where `GetKeyState` lagged).
    pub space_held: bool,
    frame_wanted: bool,
    /// When egui asked to be drawn again (animations, tooltips).
    repaint_at: Option<Instant>,
    next_tick: Instant,
    acquired: Option<wgpu::SurfaceTexture>,
    ppp: f32,
    cursor: egui::CursorIcon,
    deferred: Vec<Deferred>,
    title: String,
    pub stats: FrameStats,
    /// The self-test asks for a copy of the next frame shown ([`App::enable_capture`]).
    pub capture_next: bool,
    pub captured: Option<Result<recedo_core::image::Rgba8, String>>,
    /// Save settings when closing (off for the self-test).
    pub persist: bool,
    /// The app should close. The loop drops the app (and its GPU surface) and only then
    /// destroys the window, so the surface never outlives its window.
    pub close_requested: bool,
    /// The self-test's trace of pen messages and of where strokes start and end (`None`:
    /// not recording). Printed when a pen check fails, to show why.
    pub pen_log: Option<PenLog>,
}

/// See [`App::pen_log`].
pub struct PenLog {
    start: Instant,
    pub lines: Vec<String>,
}

impl PenLog {
    pub fn new() -> PenLog {
        PenLog { start: Instant::now(), lines: Vec::new() }
    }

    fn add(&mut self, line: String) {
        if self.lines.len() < 400 {
            self.lines.push(format!("{:7.1} ms  {line}", self.start.elapsed().as_secs_f64() * 1000.0));
        }
    }
}

impl App {
    pub fn new(hwnd: HWND, settings: Settings, recovery: Option<PathBuf>, persist: bool) -> Result<App, String> {
        let instance = Gpu::new_instance();
        let mut wh = raw_window_handle::Win32WindowHandle::new(NonZeroIsize::new(hwnd as isize).ok_or("no window")?);
        // SAFETY: plain call.
        wh.hinstance = NonZeroIsize::new(unsafe { GetModuleHandleW(std::ptr::null()) } as isize);
        // SAFETY: the window outlives the surface: on close the app (and this surface) is
        // dropped before the window is destroyed (see `close_requested`).
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: Some(raw_window_handle::RawDisplayHandle::Windows(raw_window_handle::WindowsDisplayHandle::new())),
                raw_window_handle: raw_window_handle::RawWindowHandle::Win32(wh),
            })
        }
        .map_err(|e| format!("could not create a GPU surface: {e}"))?;
        let gpu = Gpu::new(instance, Some(&surface))?;
        let caps = surface.get_capabilities(&gpu.adapter);
        let format = egui_wgpu::preferred_framebuffer_format(&caps.formats).map_err(|e| format!("{e:?}"))?;
        let (w, h) = client_size(hwnd);
        let mut config = surface.get_default_config(&gpu.adapter, w.max(1), h.max(1)).ok_or("the GPU cannot draw to this window")?;
        config.format = format;
        // Vsync with one frame queued: frames are paced by the display (Mailbox would redraw
        // as fast as the GPU can during a stroke, for no visible gain), and the frame waits
        // for its image before reading input, so the newest pen samples are shown.
        config.present_mode = wgpu::PresentMode::Fifo;
        config.desired_maximum_frame_latency = 1;
        surface.configure(&gpu.device, &config);
        let session = Session::new(settings, recovery);
        let renderer = Renderer::new(&gpu.device, format, &session.ed);
        disable_pen_gestures(hwnd);
        // SAFETY: valid window.
        unsafe { DragAcceptFiles(hwnd, 1) };
        let mut app = App {
            hwnd,
            gpu,
            surface,
            config,
            session,
            renderer,
            egui_in: EguiInput::new(),
            wintab: None,
            ink: InkInput::new(),
            ink_in_range: None,
            last_wintab_packet: None,
            mouse_clock: MsClockMapper::default(),
            mouse_held: None,
            space_held: false,
            frame_wanted: true,
            repaint_at: None,
            next_tick: Instant::now() + IDLE_TICK,
            acquired: None,
            ppp: 1.0,
            cursor: egui::CursorIcon::Default,
            deferred: Vec::new(),
            title: String::new(),
            stats: FrameStats::default(),
            capture_next: false,
            captured: None,
            persist,
            close_requested: false,
            pen_log: None,
        };
        let api = app.session.ui.settings.tablet;
        app.set_tablet_api(api);
        Ok(app)
    }

    /// Lets frames be copied back from the screen (self-test only: not every GPU allows it,
    /// and it is not needed for drawing). Returns false where it is not possible.
    pub fn enable_capture(&mut self) -> bool {
        if !self.surface.get_capabilities(&self.gpu.adapter).usages.contains(wgpu::TextureUsages::COPY_SRC) {
            return false;
        }
        self.config.usage |= wgpu::TextureUsages::COPY_SRC;
        self.acquired = None;
        self.surface.configure(&self.gpu.device, &self.config);
        true
    }

    pub fn describe_gpu(&self) -> String {
        let i = self.gpu.adapter.get_info();
        format!("{} ({:?}, {:?})", i.name, i.backend, i.device_type)
    }

    fn notice(&mut self, kind: NoticeKind, text: impl Into<String>) {
        self.session.ed.notices.push(Notice { kind, text: text.into() });
    }

    // ----- tablet ---------------------------------------------------------------------------

    /// Switches the tablet interface (from the settings, or at start-up).
    pub fn set_tablet_api(&mut self, api: TabletApi) {
        let now = now_us();
        self.session.ed.pen_gone(Source::Ink, now);
        self.session.ed.pen_gone(Source::Wintab, now);
        self.session.ui.settings.tablet = api;
        self.wintab = None;
        if api == TabletApi::Wintab {
            match WintabInput::open(self.hwnd, MappingMode::Auto) {
                Ok(w) => {
                    self.session.ui.tablet_status = format!("Wintab: {}", w.ctx.device.device_name);
                    self.wintab = Some(w);
                }
                Err(e) => {
                    self.session.ui.tablet_status = "Wintab did not open; using Windows Ink".into();
                    self.notice(NoticeKind::Error, format!("Wintab could not be opened ({e}). The pen works through Windows Ink instead."));
                }
            }
        } else {
            let pens: Vec<String> = pen_devices().into_iter().map(|d| if d.product.is_empty() { "pen".to_string() } else { d.product }).collect();
            self.session.ui.tablet_status =
                if pens.is_empty() { "Windows Ink (no pen found yet)".into() } else { format!("Windows Ink: {}", pens.join(", ")) };
        }
        self.request_frame();
    }

    /// The source that paints.
    fn painting_source(&self) -> Source {
        if self.wintab.is_some() { Source::Wintab } else { Source::Ink }
    }

    /// Is a pen near the tablet (so mouse messages are the pen's, not a real mouse's)?
    ///
    /// Wintab: for as long as the driver says the pen is in range, like Krita. Its mouse
    /// messages look like a real mouse's, and one arriving before the pen's own packet would
    /// otherwise start a stroke without pressure. (Windows Ink marks its mouse messages, so
    /// there the range only matters for a real mouse used while the pen is near.)
    fn pen_nearby(&self) -> bool {
        let now = Instant::now();
        let recent = |t: Option<Instant>, d: Duration| t.is_some_and(|t| now.duration_since(t) < d);
        recent(self.ink_in_range, IN_RANGE_TIMEOUT) || recent(self.last_wintab_packet, TABLET_QUIET) || self.wintab.as_ref().is_some_and(|w| w.in_proximity)
    }

    /// Hands samples to the editor.
    pub fn feed(&mut self, samples: &[PenSample], mods: Modifiers) {
        for s in samples {
            let over = self.session.over_canvas(s.x, s.y);
            let was = self.session.ed.stroke_active();
            self.session.ed.pen(s, over, mods);
            let now = self.session.ed.stroke_active();
            if let Some(log) = &mut self.pen_log
                && was != now
            {
                let what = if now { "starts" } else { "ends" };
                log.add(format!("  stroke {what} at {:?} sample {} (contact {}, over the canvas {over})", s.source, s.serial, s.contact()));
            }
        }
        self.request_frame();
    }

    fn on_mouse(&mut self, msg: u32, wparam: WPARAM, lparam: LPARAM) {
        if mouse_from_pen_or_touch() || (self.mouse_held.is_none() && self.pen_nearby()) {
            return;
        }
        let mut mods = modifiers(self.space_held);
        let held_before = self.mouse_held;
        match msg {
            WM_LBUTTONDOWN | WM_LBUTTONDBLCLK if self.mouse_held.is_none() => self.mouse_held = Some(MK_LBUTTON),
            WM_MBUTTONDOWN | WM_MBUTTONDBLCLK if self.mouse_held.is_none() => self.mouse_held = Some(MK_MBUTTON),
            WM_LBUTTONUP if self.mouse_held == Some(MK_LBUTTON) => self.mouse_held = None,
            WM_MBUTTONUP if self.mouse_held == Some(MK_MBUTTON) => self.mouse_held = None,
            WM_MOUSEMOVE => {
                // A button-up lost elsewhere (e.g. released over another window).
                if let Some(b) = self.mouse_held
                    && wparam as u32 & b == 0
                {
                    self.mouse_held = None;
                }
            }
            _ => return,
        }
        // The middle button drags the view, like holding Space (Krita's default).
        if self.mouse_held.or(held_before) == Some(MK_MBUTTON) {
            mods.space = true;
            mods.shift = false;
            mods.ctrl = false;
        }
        let (x, y) = lparam_pos(lparam);
        let recv = now_us();
        // SAFETY: plain call.
        let t = self.mouse_clock.map(unsafe { GetMessageTime() } as u32, recv);
        let contact = self.mouse_held.is_some();
        let mut flags = Flags(0);
        flags.set(Flags::CONTACT, contact);
        flags.set(Flags::COARSE_POSITION, true);
        let s = PenSample {
            source: Source::Mouse,
            t_us: t,
            recv_us: recv,
            x: x as f64 + 0.5,
            y: y as f64 + 0.5,
            // A mouse paints at full pressure.
            pressure: if contact { 1.0 } else { 0.0 },
            tilt_x: 0.0,
            tilt_y: 0.0,
            twist: 0.0,
            flags,
            raw_pressure: 0,
            raw_buttons: wparam as u32,
            serial: 0,
            cursor: 0,
        };
        self.feed(&[s], mods);
    }

    // ----- messages -------------------------------------------------------------------------

    /// Handles a window message. `None` means "pass to DefWindowProc" (done by the caller
    /// once the app is no longer borrowed).
    pub fn handle(&mut self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
        if self.pen_log.is_none() {
            return self.handle_message(hwnd, msg, wparam, lparam);
        }
        let was = self.session.ed.stroke_active();
        let result = self.handle_message(hwnd, msg, wparam, lparam);
        let now = self.session.ed.stroke_active();
        if let Some(log) = &mut self.pen_log
            && was != now
        {
            log.add(format!("stroke {} while handling message 0x{msg:04X}", if now { "started" } else { "ended" }));
        }
        result
    }

    fn handle_message(&mut self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
        match msg {
            WM_POINTERDOWN | WM_POINTERUPDATE | WM_POINTERUP | WM_POINTERENTER | WM_POINTERLEAVE => {
                let event = self.ink.handle(hwnd, msg, wparam);
                if let Some(log) = &mut self.pen_log {
                    let name = match msg {
                        WM_POINTERDOWN => "DOWN",
                        WM_POINTERUPDATE => "UPDATE",
                        WM_POINTERUP => "UP",
                        WM_POINTERENTER => "ENTER",
                        _ => "LEAVE",
                    };
                    let what = match &event {
                        Some(InkEvent::Samples(v)) => {
                            let frames: Vec<String> = v.iter().map(|s| format!("{}{}", s.serial, if s.contact() { "c" } else { "h" })).collect();
                            format!("{} samples [{}]", v.len(), frames.join(" "))
                        }
                        Some(InkEvent::Leave { .. }) => "left".into(),
                        None => "not a pen".into(),
                    };
                    log.add(format!("WM_POINTER{name} id {}: {what}", wparam & 0xFFFF));
                }
                match event {
                    Some(InkEvent::Samples(v)) => {
                        self.ink_in_range = Some(Instant::now());
                        if self.painting_source() == Source::Ink {
                            self.feed(&v, modifiers(self.space_held));
                        }
                    }
                    Some(InkEvent::Leave { .. }) => {
                        self.ink_in_range = None;
                        self.session.ed.pen_gone(Source::Ink, now_us());
                        self.request_frame();
                    }
                    None => {}
                }
                // DefWindowProc turns pen pointer messages into mouse messages for the interface.
                None
            }
            WM_POINTERCAPTURECHANGED => {
                if let Some(log) = &mut self.pen_log {
                    log.add(format!("WM_POINTERCAPTURECHANGED id {}", wparam & 0xFFFF));
                }
                self.session.ed.pen_gone(Source::Ink, now_us());
                self.request_frame();
                None
            }
            m if (WT_DEFBASE..=WT_DEFBASE + WT_MAXOFFSET).contains(&m) => {
                let off = m - WT_DEFBASE;
                let mut samples = Vec::new();
                let mut left = false;
                if let Some(w) = &mut self.wintab {
                    match off {
                        WT_PACKET => samples = w.process(hwnd),
                        WT_PROXIMITY => {
                            w.on_proximity(lparam);
                            left = !w.in_proximity;
                        }
                        _ => {}
                    }
                }
                if !samples.is_empty() {
                    self.last_wintab_packet = Some(Instant::now());
                    self.feed(&samples, modifiers(self.space_held));
                }
                if left {
                    self.session.ed.pen_gone(Source::Wintab, now_us());
                    self.request_frame();
                }
                Some(0)
            }
            WM_ACTIVATE => {
                if (wparam & 0xFFFF) as u32 == WA_INACTIVE {
                    // The key-up would go to another window.
                    self.space_held = false;
                }
                if let Some(w) = &self.wintab {
                    w.ctx.on_activate((wparam & 0xFFFF) as u32 != WA_INACTIVE);
                }
                None
            }
            WM_SIZE => {
                self.request_frame();
                None
            }
            WM_DPICHANGED => {
                // Apply Windows' suggested rectangle right away, as Microsoft recommends. The
                // WM_SIZE etc. this sends back arrive while the app is borrowed and get default
                // handling; the new size is picked up at the next frame.
                // SAFETY: lparam points to the suggested window rectangle; valid window.
                unsafe {
                    let r = *(lparam as *const RECT);
                    SetWindowPos(hwnd, std::ptr::null_mut(), r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOZORDER | SWP_NOACTIVATE);
                }
                self.request_frame();
                Some(0)
            }
            WM_DISPLAYCHANGE => {
                self.ink.reset_devices();
                if let Some(w) = &mut self.wintab {
                    w.calibrator.reset();
                }
                None
            }
            WM_SETCURSOR => {
                if (lparam & 0xFFFF) as u32 == HTCLIENT {
                    self.apply_cursor();
                    Some(1)
                } else {
                    None
                }
            }
            WM_PAINT => {
                // SAFETY: valid window.
                unsafe { ValidateRect(hwnd, std::ptr::null()) };
                self.request_frame();
                Some(0)
            }
            WM_ERASEBKGND => Some(1),
            WM_ENTERSIZEMOVE => {
                // The system runs its own message loop while the window is dragged or
                // resized; a timer keeps the window drawing during it.
                // SAFETY: valid window.
                unsafe { SetTimer(hwnd, TIMER_SIZEMOVE, 16, None) };
                None
            }
            WM_EXITSIZEMOVE => {
                // SAFETY: valid window.
                unsafe { KillTimer(hwnd, TIMER_SIZEMOVE) };
                None
            }
            WM_TIMER if wparam == TIMER_SIZEMOVE => {
                self.frame();
                Some(0)
            }
            WM_GETMINMAXINFO => {
                // SAFETY: lparam points to MINMAXINFO.
                let mmi = unsafe { &mut *(lparam as *mut MINMAXINFO) };
                mmi.ptMinTrackSize.x = 800;
                mmi.ptMinTrackSize.y = 560;
                Some(0)
            }
            WM_SYSCOMMAND if (wparam & 0xFFF0) as u32 == SC_KEYMENU && lparam != b' ' as isize => {
                // Alt or F10 alone would put the window into menu mode (the next key goes to
                // the invisible system menu). Alt+Space still opens the window menu.
                Some(0)
            }
            WM_DROPFILES => {
                let drop = wparam as HDROP;
                let mut buf = vec![0u16; 32_768];
                // SAFETY: valid drop handle from the system; the buffer length is passed.
                let len = unsafe { DragQueryFileW(drop, 0, buf.as_mut_ptr(), buf.len() as u32) } as usize;
                // SAFETY: the handle is ours to release.
                unsafe { DragFinish(drop) };
                if len > 0 {
                    use std::os::windows::ffi::OsStringExt;
                    let path = PathBuf::from(std::ffi::OsString::from_wide(&buf[..len.min(buf.len())]));
                    self.open_path(path);
                }
                Some(0)
            }
            WM_QUERYENDSESSION => {
                // Windows is shutting down or the user signs out: keep a recovery copy.
                self.session.ed.emergency_autosave();
                Some(1)
            }
            WM_CLOSE => {
                if self.session.ui.request_quit(&self.session.ed) {
                    self.close_requested = true;
                }
                self.request_frame();
                Some(0)
            }
            WM_DESTROY => {
                // SAFETY: plain call.
                unsafe { PostQuitMessage(0) };
                Some(0)
            }
            WM_KEYDOWN | WM_KEYUP if wparam as u16 == VK_SPACE && (msg == WM_KEYUP || !self.session.egui.egui_wants_keyboard_input()) => {
                // Space is the pan key, not a key for the interface (where it would press
                // whichever button has keyboard focus).
                self.space_held = msg == WM_KEYDOWN;
                self.apply_cursor();
                Some(0)
            }
            WM_MOUSEWHEEL => {
                // Over the canvas the wheel zooms about the pointer; elsewhere it scrolls panels.
                let (sx, sy) = lparam_pos(lparam);
                let mut p = POINT { x: sx, y: sy };
                // SAFETY: valid window.
                unsafe { ScreenToClient(hwnd, &mut p) };
                let at = (p.x as f64 + 0.5, p.y as f64 + 0.5);
                if self.session.over_canvas(at.0, at.1) {
                    let steps = ((wparam >> 16) & 0xFFFF) as i16 as f64 / WHEEL_DELTA as f64;
                    self.session.ed.wheel(steps, at);
                    self.request_frame();
                    return Some(0);
                }
                let consumed = self.egui_in.on_message(hwnd, msg, wparam, lparam, self.ppp);
                self.request_frame();
                consumed.then_some(0)
            }
            WM_MOUSEMOVE | WM_LBUTTONDOWN | WM_LBUTTONUP | WM_LBUTTONDBLCLK | WM_RBUTTONDOWN | WM_RBUTTONUP | WM_RBUTTONDBLCLK | WM_MBUTTONDOWN
            | WM_MBUTTONUP | WM_MBUTTONDBLCLK | WM_MOUSEHWHEEL | WM_MOUSELEAVE | WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP | WM_CHAR
            | WM_SETFOCUS | WM_KILLFOCUS | WM_CAPTURECHANGED => {
                if matches!(msg, WM_MOUSEMOVE | WM_LBUTTONDOWN | WM_LBUTTONUP | WM_LBUTTONDBLCLK | WM_MBUTTONDOWN | WM_MBUTTONUP | WM_MBUTTONDBLCLK) {
                    self.on_mouse(msg, wparam, lparam);
                }
                if msg == WM_KILLFOCUS {
                    self.space_held = false;
                }
                if matches!(msg, WM_KILLFOCUS | WM_CAPTURECHANGED) && self.mouse_held.take().is_some() {
                    // The button-up will not come to this window.
                    self.session.ed.pen_gone(Source::Mouse, now_us());
                }
                if msg == WM_MOUSELEAVE {
                    self.session.ed.hover = None;
                }
                let consumed = self.egui_in.on_message(hwnd, msg, wparam, lparam, self.ppp);
                self.request_frame();
                consumed.then_some(0)
            }
            _ => None,
        }
    }

    /// Sets the mouse cursor for where it is: a crosshair or move cursor over the canvas, the
    /// interface's own cursor elsewhere.
    fn apply_cursor(&mut self) {
        let (x, y) = cursor_in_client(self.hwnd);
        let id = if self.session.over_canvas(x, y) {
            if self.space_held || self.session.ed.tool == Tool::Pan { Some(IDC_SIZEALL) } else { Some(IDC_CROSS) }
        } else {
            system_cursor(self.cursor)
        };
        // SAFETY: system cursor ids.
        unsafe {
            match id {
                Some(id) => SetCursor(LoadCursorW(std::ptr::null_mut(), id)),
                None => SetCursor(std::ptr::null_mut()),
            };
        }
    }

    // ----- files ----------------------------------------------------------------------------

    /// Opens a file (from the Open dialog, a drop on the window or the command line), asking
    /// about unsaved changes first.
    pub fn open_path(&mut self, path: PathBuf) {
        let (ui, ed) = (&mut self.session.ui, &mut self.session.ed);
        ui.open_path(ed, path);
        self.save_settings();
        self.request_frame();
    }

    pub fn save_as(&mut self, path: PathBuf) {
        let (ui, ed) = (&mut self.session.ui, &mut self.session.ed);
        ui.save_as_path(ed, path);
        self.save_settings();
        self.request_frame();
    }

    pub fn export(&mut self, path: PathBuf, format: ExportFormat) {
        if let Some(dir) = path.parent() {
            self.session.ui.settings.last_folder = Some(dir.to_path_buf());
        }
        self.session.ed.export(&path, format);
        self.request_frame();
    }

    pub fn save_cancelled(&mut self) {
        self.session.ui.save_cancelled();
        self.request_frame();
    }

    pub fn take_deferred(&mut self) -> Vec<Deferred> {
        std::mem::take(&mut self.deferred)
    }

    fn handle_request(&mut self, r: Request) {
        match r {
            Request::OpenDialog => self.deferred.push(Deferred::Open),
            Request::SaveAsDialog => self.deferred.push(Deferred::SaveAs),
            Request::ExportDialog(f) => self.deferred.push(Deferred::Export(f)),
            Request::Quit => self.close_requested = true,
            Request::SetTabletApi(api) => {
                self.set_tablet_api(api);
                self.save_settings();
            }
        }
    }

    pub fn save_settings(&mut self) {
        if !self.persist {
            return;
        }
        let mut s = self.session.settings();
        let mut wp: WINDOWPLACEMENT = WINDOWPLACEMENT { length: std::mem::size_of::<WINDOWPLACEMENT>() as u32, ..Default::default() };
        // SAFETY: valid window and struct.
        if unsafe { GetWindowPlacement(self.hwnd, &mut wp) } != 0 {
            let r = wp.rcNormalPosition;
            s.window = Some((r.left, r.top, r.right, r.bottom, wp.showCmd == SW_SHOWMAXIMIZED as u32));
        }
        self.session.ui.settings.window = s.window;
        if let Err(e) = s.save(&settings_path()) {
            self.notice(NoticeKind::Warning, format!("Could not save the settings: {e}"));
        }
    }

    /// Before the window closes: waits for saves, removes the recovery copy (the user saved
    /// or chose not to), keeps the settings.
    pub fn closing(&mut self) {
        self.session.ed.closing();
        self.save_settings();
    }

    // ----- frames ---------------------------------------------------------------------------

    pub fn request_frame(&mut self) {
        self.frame_wanted = true;
    }

    pub fn wants_frame(&self) -> bool {
        self.frame_wanted || self.session.ed.needs_ticks() || self.repaint_at.is_some_and(|t| Instant::now() >= t)
    }

    /// Time until something is due if no input arrives.
    pub fn idle_timeout(&self) -> Duration {
        if self.wants_frame() {
            return Duration::ZERO;
        }
        let next = self.repaint_at.map_or(self.next_tick, |r| r.min(self.next_tick));
        next.saturating_duration_since(Instant::now())
    }

    /// While idle: lets autosave and background saves run without drawing frames.
    pub fn idle_tick(&mut self) {
        let now = Instant::now();
        if now < self.next_tick {
            return;
        }
        self.next_tick = now + IDLE_TICK;
        let ed = &mut self.session.ed;
        ed.tick(now_us(), now);
        if !ed.notices.is_empty() || ed.needs_ticks() {
            self.frame_wanted = true;
        }
    }

    fn reconfigure_if_resized(&mut self) -> bool {
        let (w, h) = client_size(self.hwnd);
        if w == 0 || h == 0 {
            return false;
        }
        if (w, h) != (self.config.width, self.config.height) {
            self.acquired = None;
            self.config.width = w;
            self.config.height = h;
            self.surface.configure(&self.gpu.device, &self.config);
        }
        true
    }

    /// Step 1 of a frame: wait for the GPU to have a free image. Done before reading input,
    /// so the frame shows the newest pen samples.
    pub fn acquire(&mut self) {
        if self.acquired.is_some() || !self.reconfigure_if_resized() {
            return;
        }
        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => self.acquired = Some(t),
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.gpu.device, &self.config);
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Validation => {}
        }
    }

    /// Acquires (if needed), runs the interface, draws and presents one frame.
    pub fn frame(&mut self) {
        if self.pen_log.is_none() {
            return self.draw_frame();
        }
        let (was, start) = (self.session.ed.stroke_active(), Instant::now());
        self.draw_frame();
        let now = self.session.ed.stroke_active();
        if let Some(log) = &mut self.pen_log {
            let change = if was == now { "" } else if now { "; stroke started" } else { "; stroke ended" };
            log.add(format!("frame took {:.1} ms{change}", start.elapsed().as_secs_f64() * 1000.0));
        }
    }

    fn draw_frame(&mut self) {
        // Lets the self-test check the crash report (never set in normal use).
        if std::env::var_os("RECEDO_TEST_CRASH").is_some() {
            panic!("crash requested by RECEDO_TEST_CRASH");
        }
        self.acquire();
        let Some(frame) = self.acquired.take() else {
            // Minimised or not ready: try again a little later.
            self.frame_wanted = false;
            self.repaint_at = Some(Instant::now() + Duration::from_millis(250));
            return;
        };
        let start = Instant::now();
        self.frame_wanted = false;
        self.repaint_at = None;
        self.session.ed.tick(now_us(), start);
        self.next_tick = start + IDLE_TICK;
        let (w, h) = (self.config.width, self.config.height);
        // SAFETY: valid window.
        let native_ppp = unsafe { GetDpiForWindow(self.hwnd) }.max(96) as f32 / 96.0;
        // This frame's scale, not last frame's: the first frame (and the first after a DPI
        // change) must already be laid out at the right size.
        self.ppp = native_ppp * self.session.ui.settings.ui_scale.clamp(0.5, 3.0);
        let raw = self.egui_in.take((w, h), native_ppp, self.ppp);
        let (out, ui_out) = self.session.ui_frame(raw);
        self.ppp = out.pixels_per_point;
        self.cursor = out.platform_output.cursor_icon;
        for cmd in &out.platform_output.commands {
            if let egui::OutputCommand::CopyText(t) = cmd {
                let _ = sys::set_clipboard_text(self.hwnd, t);
            }
        }
        if let Some(v) = out.viewport_output.get(&egui::ViewportId::ROOT) {
            if v.repaint_delay.is_zero() {
                self.frame_wanted = true;
            } else if v.repaint_delay < Duration::from_secs(3600) {
                self.repaint_at = Some(start + v.repaint_delay);
            }
        }
        let gpu = &self.gpu;
        self.renderer.sync(&gpu.device, &gpu.queue, &mut self.session.ed);
        let view = frame.texture.create_view(&Default::default());
        self.renderer.render(&gpu.device, &gpu.queue, Target { view: &view, size: [w, h] }, &self.session.ed, &self.session.egui, out);
        self.stats.last_work_ms = start.elapsed().as_secs_f64() * 1000.0;
        if std::mem::take(&mut self.capture_next) {
            self.captured = Some(crate::session::read_texture(&gpu.device, &gpu.queue, &frame.texture));
        }
        gpu.queue.present(frame);
        self.stats.frames += 1;
        for r in ui_out.requests {
            self.handle_request(r);
        }
        self.update_title();
    }

    fn update_title(&mut self) {
        let ed = &self.session.ed;
        let title = format!("{}{} - Recedo", ed.title(), if ed.has_unsaved_changes() { "*" } else { "" });
        if title != self.title {
            let w = sys::wide(&title);
            // SAFETY: valid window and string.
            unsafe { SetWindowTextW(self.hwnd, w.as_ptr()) };
            self.title = title;
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.acquired = None;
        self.wintab = None;
    }
}
