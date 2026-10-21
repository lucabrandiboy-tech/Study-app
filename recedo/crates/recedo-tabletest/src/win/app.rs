//! The test app on Windows: owns the window's GPU surface, the canvas, the panel and the
//! tablet inputs, and handles window messages.

use std::collections::VecDeque;
use std::num::NonZeroIsize;
use std::time::{Duration, Instant};

use recedo_tablet::clock::{MsClockMapper, now_us};
use recedo_tablet::ink::{InkEvent, InkInput, disable_pen_gestures, pen_devices};
use recedo_tablet::mapping::Candidate;
use recedo_tablet::wintab::{DeviceInfo, WintabInput};
use recedo_tablet::wintab_abi::{PK_SERIAL_NUMBER, WT_CSRCHANGE, WT_CTXOVERLAP, WT_DEFBASE, WT_MAXOFFSET, WT_PACKET, WT_PROXIMITY, describe_mask};
use recedo_tablet::{Flags, PenSample, Source};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::ValidateRect;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::egui_input::{EguiInput, system_cursor};
use super::sys;
use crate::brush::{Dab, DabBatch, sample_dot};
use crate::gpu::{CanvasRenderer, Gpu, Layer};
use crate::model::{Action, Api, InkView, Model, MonitorView, Settings, WintabState};
use crate::strokes::StrokeTracker;
use crate::{report, ui};

/// Window operations that must run outside the message handler (they send messages back
/// to the window synchronously).
#[derive(Clone)]
pub enum Deferred {
    /// Restore the window, move it to the rectangle, then maximise it there.
    MoveAndMaximize(RECT),
}

const TIMER_SIZEMOVE: usize = 1;
// Not in windows-sys's WindowsAndMessaging module.
const WM_MOUSELEAVE: u32 = 0x02A3;
const MK_LBUTTON: usize = 0x0001;
/// How often the panel's numbers refresh when nothing else is happening.
const STATS_REFRESH: Duration = Duration::from_millis(250);

fn lparam_pos(lparam: isize) -> (i32, i32) {
    ((lparam & 0xFFFF) as i16 as i32, ((lparam >> 16) & 0xFFFF) as i16 as i32)
}

/// Mouse messages that Windows made from pen or touch input carry this signature.
fn mouse_from_pen_or_touch() -> bool {
    // SAFETY: plain call.
    let info = unsafe { GetMessageExtraInfo() } as usize;
    info & 0xFFFF_FF00 == 0xFF51_5700
}

pub fn client_size(hwnd: HWND) -> (u32, u32) {
    let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    // SAFETY: valid window.
    unsafe { GetClientRect(hwnd, &mut r) };
    ((r.right - r.left).max(0) as u32, (r.bottom - r.top).max(0) as u32)
}

pub struct App {
    pub hwnd: HWND,
    pub gpu: Gpu,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    present_modes: Vec<wgpu::PresentMode>,
    pub canvas: CanvasRenderer,
    egui_ctx: egui::Context,
    egui_renderer: egui_wgpu::Renderer,
    egui_in: EguiInput,
    pub model: Model,
    pub wintab: Option<WintabInput>,
    pub ink: InkInput,
    pub tracker: StrokeTracker,
    pending_dabs: Vec<DabBatch>,
    pending_dots: Vec<Dab>,
    clear_canvas: bool,
    clear_dots: bool,
    frame_wanted: bool,
    next_refresh: Instant,
    acquired: Option<wgpu::SurfaceTexture>,
    frame_times: VecDeque<Instant>,
    samples_since_frame: usize,
    newest_unpresented_recv: Option<u64>,
    /// Panel rectangle in points, from the last frame.
    panel_rect: egui::Rect,
    ppp: f32,
    cursor: egui::CursorIcon,
    mouse_clock: MsClockMapper,
    last_pen_us: u64,
    ink_in_range: bool,
    deferred: Vec<Deferred>,
    /// Save settings on exit (off for the self-test).
    pub persist: bool,
    /// The user closed the window. The loop drops the app (and its GPU surface) and only
    /// then destroys the window, so the surface never outlives its window.
    pub close_requested: bool,
    monitors: Vec<sys::Monitor>,
}

impl App {
    pub fn new(hwnd: HWND, settings: Settings, persist: bool) -> Result<App, String> {
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
        let mut config = surface
            .get_default_config(&gpu.adapter, w.max(1), h.max(1))
            .ok_or("the GPU cannot draw to this window")?;
        config.format = format;
        config.desired_maximum_frame_latency = 1;
        let canvas = CanvasRenderer::new(&gpu, w.max(1), h.max(1), Some(format));
        let egui_renderer = egui_wgpu::Renderer::new(&gpu.device, format, egui_wgpu::RendererOptions::default());
        let mut model = Model { settings, gpu: gpu.describe(), os: sys::os_version(), ..Default::default() };
        model.log(format!("Recedo tablet test {} ({})", report::VERSION, report::GIT_HASH));
        model.log(format!("GPU: {}", model.gpu));
        disable_pen_gestures(hwnd);
        let mut app = App {
            hwnd,
            gpu,
            surface,
            config,
            present_modes: caps.present_modes,
            canvas,
            egui_ctx: egui::Context::default(),
            egui_renderer,
            egui_in: EguiInput::new(),
            model,
            wintab: None,
            ink: InkInput::new(),
            tracker: StrokeTracker::default(),
            pending_dabs: Vec::new(),
            pending_dots: Vec::new(),
            clear_canvas: false,
            clear_dots: false,
            frame_wanted: true,
            next_refresh: Instant::now(),
            acquired: None,
            frame_times: VecDeque::new(),
            samples_since_frame: 0,
            newest_unpresented_recv: None,
            panel_rect: egui::Rect::NOTHING,
            ppp: 1.0,
            cursor: egui::CursorIcon::Default,
            mouse_clock: MsClockMapper::default(),
            last_pen_us: 0,
            ink_in_range: false,
            deferred: Vec::new(),
            persist,
            close_requested: false,
            monitors: Vec::new(),
        };
        app.apply_present_mode();
        app.refresh_monitors();
        app.refresh_ink_devices();
        if app.model.settings.api == Api::Wintab {
            app.open_wintab();
        }
        Ok(app)
    }

    fn apply_present_mode(&mut self) {
        let want: &[wgpu::PresentMode] = if self.model.settings.low_latency {
            &[wgpu::PresentMode::Mailbox, wgpu::PresentMode::Immediate, wgpu::PresentMode::Fifo]
        } else {
            &[wgpu::PresentMode::Fifo]
        };
        self.config.present_mode = want.iter().copied().find(|m| self.present_modes.contains(m)).unwrap_or(wgpu::PresentMode::Fifo);
        self.acquired = None;
        self.surface.configure(&self.gpu.device, &self.config);
        self.model.frame.present_mode = format!("{:?}, max frame latency {}", self.config.present_mode, self.config.desired_maximum_frame_latency);
    }

    fn refresh_monitors(&mut self) {
        self.monitors = sys::monitors();
        // SAFETY: valid window.
        let mine = unsafe { windows_sys::Win32::Graphics::Gdi::MonitorFromWindow(self.hwnd, windows_sys::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST) };
        self.model.monitors = self
            .monitors
            .iter()
            .map(|m| MonitorView {
                name: if m.friendly.is_empty() { m.gdi_name.clone() } else { m.friendly.clone() },
                rect: (m.rect.left, m.rect.top, m.rect.right, m.rect.bottom),
                dpi: m.dpi,
                primary: m.primary,
                has_window: m.handle == mine,
            })
            .collect();
    }

    fn refresh_ink_devices(&mut self) {
        self.model.ink.devices = pen_devices()
            .into_iter()
            .map(|d| {
                let mut s = if d.product.is_empty() { "(unnamed pen)".to_string() } else { d.product };
                if let Some((l, t, r, b)) = d.display_rect {
                    s.push_str(&format!(", mapped to screen {}×{} at ({l},{t})", r - l, b - t));
                }
                s
            })
            .collect();
    }

    pub fn open_wintab(&mut self) {
        self.wintab = None;
        match WintabInput::open(self.hwnd, self.model.settings.mapping_mode()) {
            Ok(w) => {
                self.model.wintab.state = WintabState::Open;
                self.model.wintab.device_name = w.ctx.device.device_name.clone();
                for l in &w.ctx.open_log {
                    self.model.log(format!("Wintab: {l}"));
                }
                self.model.log(format!("Wintab: opened {} from {}", w.ctx.device.device_name, w.ctx.dll_path()));
                self.wintab = Some(w);
            }
            Err(e) => {
                self.model.log(format!("Wintab: {e}"));
                self.model.wintab = Default::default();
                self.model.wintab.state = WintabState::Failed(e);
            }
        }
        self.refresh_wintab_view();
    }

    pub fn close_wintab(&mut self) {
        if self.wintab.take().is_some() {
            self.model.log("Wintab: closed");
        }
        self.model.wintab = Default::default();
    }

    fn refresh_wintab_view(&mut self) {
        let Some(w) = &self.wintab else { return };
        let d: &DeviceInfo = &w.ctx.device;
        let c = &w.ctx.ctx;
        let v = &mut self.model.wintab;
        let levels = (d.pressure.axMax - d.pressure.axMin + 1).max(0);
        let o = &d.orientation;
        v.info = vec![
            ("DLL".into(), d.dll_path.clone()),
            ("Wintab ID".into(), d.wintab_id.clone()),
            ("Spec / driver version".into(), format!("{} / {}", DeviceInfo::version_string(d.spec_version), DeviceInfo::version_string(d.impl_version))),
            ("Devices / cursors".into(), format!("{} / {}", d.n_devices, d.n_cursors)),
            ("Device".into(), d.device_name.clone()),
            ("PnP id".into(), d.pnp_id.clone()),
            ("Advertised packet rate".into(), format!("{}/s", d.pkt_rate)),
            ("Position range".into(), format!("X {}..{}, Y {}..{}", d.x.axMin, d.x.axMax, d.y.axMin, d.y.axMax)),
            ("Pressure range".into(), format!("{}..{} ({levels} levels)", d.pressure.axMin, d.pressure.axMax)),
            (
                "Tilt (orientation)".into(),
                if d.has_tilt {
                    format!(
                        "azimuth {}..{} res {:.0}, altitude {}..{} res {:.0}",
                        o[0].axMin, o[0].axMax, recedo_tablet::wintab_abi::fix32_to_f64(o[0].axResolution),
                        o[1].axMin, o[1].axMax, recedo_tablet::wintab_abi::fix32_to_f64(o[1].axResolution)
                    )
                } else {
                    "not reported by the driver".into()
                },
            ),
            ("Packet fields".into(), describe_mask(w.ctx.layout.mask)),
            ("Queue size".into(), format!("{} packets", w.ctx.queue_size)),
            ("Context options".into(), format!("0x{:X}", c.lcOptions)),
            ("Tablet-provided area".into(), format!("{}×{} at ({}, {})", c.lcSysExtX, c.lcSysExtY, c.lcSysOrgX, c.lcSysOrgY)),
            ("Output extent".into(), format!("{} × {}", c.lcOutExtX, c.lcOutExtY)),
        ];
        let cal = &w.calibrator;
        let active = cal.active();
        v.mapping_active = active.label();
        v.mapping_rows = Candidate::FIXED.iter().map(|&c| (c.label(), cal.score(c))).collect();
        v.fit = cal.fit();
        v.mismatch = cal.mismatch();
        v.in_proximity = w.in_proximity;
        v.cursors = w.ctx.known_cursors().map(|c| format!("#{} {} (type 0x{:X}, id 0x{:X})", c.index, c.name, c.csr_type, c.physid)).collect();
        if let Some(p) = &w.last_packet {
            v.last_raw = format!(
                "x {} y {} p {} buttons 0x{:X} status 0x{:X} cursor {} az {} alt {} serial {}",
                p.x, p.y, p.normal_pressure, p.buttons, p.status, p.cursor, p.orientation.orAzimuth, p.orientation.orAltitude, p.serial
            );
        }
    }

    fn refresh_views(&mut self) {
        self.refresh_wintab_view();
        self.model.ink = InkView {
            devices: std::mem::take(&mut self.model.ink.devices),
            max_history: self.ink.max_history,
            coarse_fallbacks: self.ink.coarse_fallbacks,
            in_range: self.ink_in_range,
        };
        // SAFETY: valid window.
        self.model.window_dpi = unsafe { GetDpiForWindow(self.hwnd) }.max(96);
    }

    pub fn request_frame(&mut self) {
        self.frame_wanted = true;
    }

    fn pen_recently_active(&self) -> bool {
        self.ink_in_range || self.wintab.as_ref().is_some_and(|w| w.in_proximity) || now_us().saturating_sub(self.last_pen_us) < 2_000_000
    }

    pub fn on_samples(&mut self, source: Source, samples: Vec<PenSample>) {
        if samples.is_empty() {
            return;
        }
        let now = now_us();
        let serials = source == Source::Wintab && self.wintab.as_ref().is_some_and(|w| w.ctx.layout.has(PK_SERIAL_NUMBER));
        self.model.stats.get_mut(source).record_batch(&samples, now, serials);
        self.model.record(&samples);
        if source != Source::Mouse {
            self.last_pen_us = now;
        }
        let api = self.model.settings.api;
        let draws = source == api.source() || (source == Source::Mouse && (api == Api::MouseOnly || !self.pen_recently_active()));
        if !draws {
            return;
        }
        let brush = self.model.settings.brush;
        let (show_dots, show_hover) = (self.model.settings.show_dots, self.model.settings.show_hover_dots);
        // The closure copies what it needs so the tracker can be borrowed mutably.
        let accept = {
            let (size, ppp, panel) = ((self.config.width as f64, self.config.height as f64), self.ppp, self.panel_rect);
            let ctx = self.egui_ctx.clone();
            move |x: f64, y: f64| ui::stroke_may_start(&ctx, panel, ppp, size, x, y)
        };
        let before = self.tracker.strokes_started;
        for s in &samples {
            if show_dots && (s.contact() || show_hover) {
                self.pending_dots.push(sample_dot(s));
            }
            self.tracker.feed(s, &brush, &accept, &mut self.pending_dabs);
        }
        self.model.strokes += self.tracker.strokes_started - before;
        self.model.latest = samples.last().copied();
        self.samples_since_frame += samples.len();
        self.newest_unpresented_recv = Some(samples.last().unwrap().recv_us);
        self.request_frame();
    }

    fn on_mouse(&mut self, msg: u32, wparam: WPARAM, lparam: LPARAM) {
        if mouse_from_pen_or_touch() {
            return;
        }
        let contact = match msg {
            WM_LBUTTONDOWN => true,
            WM_LBUTTONUP => false,
            WM_MOUSEMOVE => wparam & MK_LBUTTON != 0,
            _ => return,
        };
        let (x, y) = lparam_pos(lparam);
        let recv = now_us();
        // SAFETY: plain call.
        let t = self.mouse_clock.map(unsafe { GetMessageTime() } as u32, recv);
        let mut flags = Flags(0);
        flags.set(Flags::CONTACT, contact);
        flags.set(Flags::COARSE_POSITION, true);
        let s = PenSample {
            source: Source::Mouse,
            t_us: t,
            recv_us: recv,
            x: x as f64 + 0.5,
            y: y as f64 + 0.5,
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
        self.on_samples(Source::Mouse, vec![s]);
    }

    /// Handles a window message. `None` means "pass to DefWindowProc" (done by the caller
    /// after the app is no longer borrowed).
    pub fn handle(&mut self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
        match msg {
            WM_POINTERDOWN | WM_POINTERUPDATE | WM_POINTERUP | WM_POINTERENTER | WM_POINTERLEAVE => {
                match self.ink.handle(hwnd, msg, wparam) {
                    Some(InkEvent::Samples(v)) => {
                        self.ink_in_range = true;
                        self.on_samples(Source::Ink, v);
                    }
                    Some(InkEvent::Leave { .. }) => {
                        self.ink_in_range = false;
                        self.tracker.end_source(Source::Ink);
                        self.request_frame();
                    }
                    None => {}
                }
                // DefWindowProc turns pen pointer messages into mouse messages for the panel.
                None
            }
            WM_POINTERCAPTURECHANGED => {
                self.tracker.end_source(Source::Ink);
                None
            }
            m if (WT_DEFBASE..=WT_DEFBASE + WT_MAXOFFSET).contains(&m) => {
                let off = m - WT_DEFBASE;
                if let Some(w) = &mut self.wintab {
                    match off {
                        WT_PACKET => {
                            let v = w.process(hwnd);
                            self.on_samples(Source::Wintab, v);
                        }
                        WT_PROXIMITY => {
                            w.on_proximity(lparam);
                            if !w.in_proximity {
                                self.tracker.end_source(Source::Wintab);
                            }
                            self.request_frame();
                        }
                        WT_CSRCHANGE => self.model.log("Wintab: cursor changed (pen tip / eraser)"),
                        WT_CTXOVERLAP => {}
                        _ => {}
                    }
                }
                Some(0)
            }
            WM_ACTIVATE => {
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
                self.refresh_monitors();
                self.ink.reset_devices();
                if let Some(w) = &mut self.wintab {
                    w.calibrator.reset();
                }
                self.model.log("Display layout changed: screen mapping reset");
                None
            }
            WM_SETCURSOR => {
                if (lparam & 0xFFFF) as u32 == HTCLIENT {
                    // SAFETY: system cursor ids.
                    unsafe {
                        match system_cursor(self.cursor) {
                            Some(id) => SetCursor(LoadCursorW(std::ptr::null_mut(), id)),
                            None => SetCursor(std::ptr::null_mut()),
                        };
                    }
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
                // resized; a timer keeps the canvas drawing during it.
                // SAFETY: valid window.
                unsafe { SetTimer(hwnd, TIMER_SIZEMOVE, 16, None) };
                None
            }
            WM_EXITSIZEMOVE => {
                // SAFETY: valid window.
                unsafe { KillTimer(hwnd, TIMER_SIZEMOVE) };
                self.refresh_monitors();
                None
            }
            WM_TIMER if wparam == TIMER_SIZEMOVE => {
                self.frame();
                Some(0)
            }
            WM_GETMINMAXINFO => {
                // SAFETY: lparam points to MINMAXINFO.
                let mmi = unsafe { &mut *(lparam as *mut MINMAXINFO) };
                mmi.ptMinTrackSize.x = 640;
                mmi.ptMinTrackSize.y = 480;
                Some(0)
            }
            WM_CLOSE => {
                self.save_settings();
                self.close_requested = true;
                Some(0)
            }
            WM_DESTROY => {
                // SAFETY: plain call.
                unsafe { PostQuitMessage(0) };
                Some(0)
            }
            WM_MOUSEMOVE | WM_LBUTTONDOWN | WM_LBUTTONUP | WM_LBUTTONDBLCLK | WM_RBUTTONDOWN | WM_RBUTTONUP | WM_MBUTTONDOWN | WM_MBUTTONUP
            | WM_MOUSEWHEEL | WM_MOUSEHWHEEL | WM_MOUSELEAVE | WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP | WM_CHAR | WM_SETFOCUS
            | WM_KILLFOCUS | WM_CAPTURECHANGED => {
                if matches!(msg, WM_MOUSEMOVE | WM_LBUTTONDOWN | WM_LBUTTONUP) {
                    self.on_mouse(msg, wparam, lparam);
                }
                let consumed = self.egui_in.on_message(hwnd, msg, wparam, lparam, self.ppp);
                self.request_frame();
                consumed.then_some(0)
            }
            _ => None,
        }
    }

    pub fn save_settings(&mut self) {
        if !self.persist {
            return;
        }
        let mut wp: WINDOWPLACEMENT = WINDOWPLACEMENT { length: std::mem::size_of::<WINDOWPLACEMENT>() as u32, ..Default::default() };
        // SAFETY: valid window and struct.
        if unsafe { GetWindowPlacement(self.hwnd, &mut wp) } != 0 {
            let r = wp.rcNormalPosition;
            self.model.settings.window = Some((r.left, r.top, r.right, r.bottom, wp.showCmd == SW_SHOWMAXIMIZED as u32));
        }
        if let Err(e) = self.model.settings.save(&sys::settings_path()) {
            self.model.log(format!("Could not save settings: {e}"));
        }
    }

    pub fn take_deferred(&mut self) -> Vec<Deferred> {
        std::mem::take(&mut self.deferred)
    }

    /// Time until the app next needs a frame, if nothing else happens.
    pub fn idle_timeout(&self) -> Duration {
        if self.frame_wanted {
            return Duration::ZERO;
        }
        self.next_refresh.saturating_duration_since(Instant::now())
    }

    pub fn wants_frame(&self) -> bool {
        self.frame_wanted || Instant::now() >= self.next_refresh
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
            self.canvas.ensure_size(&self.gpu, w, h);
        }
        true
    }

    /// Step 1 of a frame: wait for the GPU to have a free image. Done before reading
    /// input, so the frame shows the newest samples.
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

    /// Acquires (if needed), draws and presents one frame.
    pub fn frame(&mut self) {
        self.acquire();
        let Some(frame) = self.acquired.take() else {
            // Minimised or not ready: try again later.
            self.next_refresh = Instant::now() + STATS_REFRESH;
            self.frame_wanted = false;
            return;
        };
        self.frame_wanted = false;
        self.next_refresh = Instant::now() + STATS_REFRESH;
        self.refresh_views();
        let (w, h) = (self.config.width, self.config.height);
        let native_ppp = self.model.window_dpi as f32 / 96.0;
        let zoom = self.model.settings.ui_scale.clamp(0.5, 3.0);
        self.egui_ctx.set_zoom_factor(zoom);
        // This frame's scale, not last frame's: the first frame (and the first after a DPI
        // change) must already be laid out at the right size.
        self.ppp = native_ppp * zoom;
        let raw = self.egui_in.take((w, h), native_ppp, self.ppp);
        let now = now_us();
        let mut actions = Vec::new();
        let mut panel_rect = egui::Rect::NOTHING;
        let model = &mut self.model;
        let mut out = self.egui_ctx.run_ui(raw, |ui| {
            (actions, panel_rect) = ui::frame_ui(ui, model, now);
        });
        self.panel_rect = panel_rect;
        self.ppp = out.pixels_per_point;
        self.cursor = out.platform_output.cursor_icon;
        for cmd in &out.platform_output.commands {
            if let egui::OutputCommand::CopyText(t) = cmd {
                let _ = sys::set_clipboard_text(self.hwnd, t);
            }
        }
        if out.viewport_output.get(&egui::ViewportId::ROOT).is_some_and(|v| v.repaint_delay.is_zero()) {
            self.frame_wanted = true;
        }
        for a in actions {
            self.run_action(a);
        }

        let gpu = &self.gpu;
        let mut enc = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        if self.clear_canvas {
            self.canvas.clear(&mut enc);
            self.clear_canvas = false;
            self.pending_dabs.clear();
            self.pending_dots.clear();
        }
        if self.clear_dots {
            self.canvas.clear_dots(&mut enc);
            self.clear_dots = false;
        }
        let batches = std::mem::take(&mut self.pending_dabs);
        self.canvas.draw(gpu, &mut enc, Layer::Canvas, &batches);
        let dots = std::mem::take(&mut self.pending_dots);
        if !dots.is_empty() {
            self.canvas.draw(gpu, &mut enc, Layer::Dots, &[DabBatch { erase: false, dabs: dots }]);
        }
        self.model.dabs = self.canvas.dabs_drawn;
        let view = frame.texture.create_view(&Default::default());
        self.canvas.composite(gpu, &mut enc, &view, self.model.settings.show_dots);

        let prims = self.egui_ctx.tessellate(std::mem::take(&mut out.shapes), out.pixels_per_point);
        let mut textures = std::mem::take(&mut out.textures_delta);
        for (id, deltas) in textures.set.drain() {
            for delta in deltas {
                self.egui_renderer.update_texture(&gpu.device, &gpu.queue, id, &delta);
            }
        }
        let sd = egui_wgpu::ScreenDescriptor { size_in_pixels: [w, h], pixels_per_point: out.pixels_per_point };
        let extra = self.egui_renderer.update_buffers(&gpu.device, &gpu.queue, &mut enc, &prims, &sd);
        {
            let mut pass = enc
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                    })],
                    ..Default::default()
                })
                .forget_lifetime();
            self.egui_renderer.render(&mut pass, &prims, &sd);
        }
        for id in textures.free.drain() {
            self.egui_renderer.free_texture(&id);
        }
        gpu.queue.submit(extra.into_iter().chain(std::iter::once(enc.finish())));
        gpu.queue.present(frame);

        // Frame statistics. Only frames that drew new pen input count: when nothing moves
        // the app idles at a few frames per second on purpose, which says nothing about how
        // fast it can keep up with the pen.
        let t = Instant::now();
        let f = &mut self.model.frame;
        f.frames += 1;
        if self.samples_since_frame > 0 {
            self.frame_times.push_back(t);
            f.samples_per_frame = f.samples_per_frame * 0.9 + self.samples_since_frame as f64 * 0.1;
            self.samples_since_frame = 0;
        }
        while self.frame_times.front().is_some_and(|&ft| t - ft > Duration::from_secs(1)) {
            self.frame_times.pop_front();
        }
        f.fps = self.frame_times.len() as f64;
        // Frame times between consecutive input frames (gaps over 100 ms are pauses in drawing).
        let gaps: Vec<f64> = self
            .frame_times
            .iter()
            .zip(self.frame_times.iter().skip(1))
            .map(|(a, b)| (*b - *a).as_secs_f64() * 1000.0)
            .filter(|&g| g < 100.0)
            .collect();
        if !gaps.is_empty() {
            f.frame_ms_avg = gaps.iter().sum::<f64>() / gaps.len() as f64;
            f.frame_ms_max = gaps.iter().copied().fold(0.0, f64::max);
        }
        if let Some(recv) = self.newest_unpresented_recv.take() {
            f.input_age_ms = now_us().saturating_sub(recv) as f64 / 1000.0;
        }
    }

    fn run_action(&mut self, a: Action) {
        match a {
            Action::SetApi(api) => {
                self.tracker.end();
                self.model.log(format!("Input API: {}", api.label()));
                if api == Api::Wintab { self.open_wintab() } else { self.close_wintab() }
                self.save_settings();
            }
            Action::SetMapping(mode) => {
                if let Some(w) = &mut self.wintab {
                    w.calibrator.mode = mode;
                }
                self.model.log(format!("Wintab mapping: {}", mode.label()));
            }
            Action::ResetCalibration => {
                if let Some(w) = &mut self.wintab {
                    w.calibrator.reset();
                }
            }
            Action::Clear => {
                self.clear_canvas = true;
                self.model.strokes = 0;
                self.tracker.end();
            }
            Action::ClearDots => self.clear_dots = true,
            Action::SaveRecording => self.save_recording(),
            Action::CopyReport => {
                let text = report::build(&mut self.model, now_us());
                match sys::set_clipboard_text(self.hwnd, &text) {
                    Ok(()) => self.model.log("Report copied to the clipboard"),
                    Err(e) => self.model.log(format!("Could not copy the report: {e}")),
                }
            }
            Action::OpenFolder => sys::open_folder(&sys::output_dir()),
            Action::MoveToMonitor(i) => {
                if let Some(m) = self.monitors.get(i) {
                    self.deferred.push(Deferred::MoveAndMaximize(m.work));
                }
            }
            Action::ResetStats => {
                self.model.stats = Default::default();
                self.ink.max_history = 0;
                self.ink.coarse_fallbacks = 0;
            }
            Action::PresentModeChanged => self.apply_present_mode(),
        }
        self.request_frame();
    }

    pub fn save_recording(&mut self) {
        let dir = sys::output_dir();
        let stamp = sys::timestamp();
        let result = (|| -> Result<String, String> {
            std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            let report_path = dir.join(format!("tablet-report-{stamp}.txt"));
            std::fs::write(&report_path, report::build(&mut self.model, now_us())).map_err(|e| e.to_string())?;
            if self.model.recorded.is_empty() {
                return Ok(format!("{} (no pen recording: tick \"Record pen data\" first)", report_path.display()));
            }
            let csv_path = dir.join(format!("pen-recording-{stamp}.csv"));
            std::fs::write(&csv_path, recedo_tablet::record::to_csv(&self.model.recorded)).map_err(|e| e.to_string())?;
            Ok(format!("{} and {}", report_path.display(), csv_path.display()))
        })();
        match result {
            Ok(p) => {
                self.model.log(format!("Saved {p}"));
                self.model.saved_to = Some(p);
            }
            Err(e) => self.model.log(format!("Saving failed: {e}")),
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.acquired = None;
        self.wintab = None;
    }
}
