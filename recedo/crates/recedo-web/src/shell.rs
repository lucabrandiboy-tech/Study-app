//! The web shell: a full-window canvas drawn with WebGL2 through wgpu, browser pointer,
//! keyboard and wheel events turned into pen samples and interface input, files through a
//! file picker and downloads, settings in the browser's storage.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use recedo_tablet::clock::now_us;
use recedo_tablet::{Flags, PenSample, Source};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, KeyboardEvent, PointerEvent, WheelEvent};
use web_time::Instant;

use crate::editor::{ExportFormat, Modifiers, Tool};
use crate::session::{Renderer, Session, Target};
use crate::settings::Settings;
use crate::ui::Request;

const SETTINGS_KEY: &str = "recedo-settings";

struct App {
    canvas: HtmlCanvasElement,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    session: Session,
    renderer: Renderer,
    events: Vec<egui::Event>,
    mods: egui::Modifiers,
    space_held: bool,
    /// Points per pixel of the last frame (for converting event positions).
    ppp: f32,
    frame_wanted: bool,
    repaint_at: Option<Instant>,
    start: Instant,
    cursor: egui::CursorIcon,
    saved_settings: String,
    frames: u64,
    title: String,
    /// Fingers on the screen (pointer id, position in pixels). Fingers move the view: one
    /// pans, two pan and pinch-zoom. Only the pen and the mouse paint.
    touches: Vec<(i32, (f64, f64))>,
    /// Pointers pressed down (id, eraser end), from their down/up events.
    down: Vec<(i32, bool)>,
    /// The pen's last pressure while touching (for readings that report none).
    last_pressure: f32,
    /// Pen readings in the last second, and when that second started (shown in the status bar).
    pen_count: u32,
    pen_window: f64,
    /// Two fingers were down: the rest of this touch does nothing until all fingers lift.
    pinching: bool,
    last_autosave: Instant,
    /// The last frames' times in ms: interface, picture (recombining and uploading),
    /// drawing, the whole frame. `window.recedoStats()` reports them (for measuring).
    timings: Vec<[f64; 4]>,
    /// The browser took the graphics (WebGL) away: nothing is drawn until it gives them back
    /// and they are set up again. The painting itself is kept in memory.
    gpu_lost: bool,
}

/// What draws to the page: the WebGL2 surface on the canvas and its device.
struct Gpu {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}

async fn open_gpu(canvas: &HtmlCanvasElement) -> Result<Gpu, String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor { backends: wgpu::Backends::GL, ..wgpu::InstanceDescriptor::new_without_display_handle() });
    let surface = instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone())).map_err(|e| format!("this browser cannot draw with WebGL2 ({e})"))?;
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions { compatible_surface: Some(&surface), ..Default::default() })
        .await
        .map_err(|e| format!("no graphics adapter ({e})"))?;
    let desc = wgpu::DeviceDescriptor { required_limits: wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()), ..Default::default() };
    let (device, queue) = adapter.request_device(&desc).await.map_err(|e| format!("the graphics device did not open ({e})"))?;
    let caps = surface.get_capabilities(&adapter);
    let format = egui_wgpu::preferred_framebuffer_format(&caps.formats).map_err(|e| format!("{e:?}"))?;
    let (w, h) = canvas_size(canvas);
    let mut config = surface.get_default_config(&adapter, w, h).ok_or("the graphics device cannot draw to this page")?;
    config.format = format;
    surface.configure(&device, &config);
    Ok(Gpu { surface, device, queue, config })
}

/// How many frames' times are kept.
const TIMED_FRAMES: usize = 240;

fn window() -> web_sys::Window {
    web_sys::window().expect("a browser window")
}

/// A message on the page instead of the app (e.g. no WebGL2).
fn show_error(msg: &str) {
    if let Some(el) = window().document().and_then(|d| d.get_element_by_id("status")) {
        el.set_text_content(Some(msg));
        let _ = el.set_attribute("class", "error");
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    wasm_bindgen_futures::spawn_local(async {
        if let Err(e) = run().await {
            show_error(&format!("Recedo could not start: {e}"));
        }
    });
}

fn load_settings() -> Settings {
    let stored = window().local_storage().ok().flatten().and_then(|s| s.get_item(SETTINGS_KEY).ok().flatten());
    match stored.and_then(|t| serde_json::from_str::<Settings>(&t).ok()) {
        Some(mut s) => {
            s.sanitize();
            s
        }
        None => Settings::default(),
    }
}

async fn run() -> Result<(), String> {
    let document = window().document().ok_or("no document")?;
    let canvas: HtmlCanvasElement = document.get_element_by_id("recedo").ok_or("no canvas")?.dyn_into().map_err(|_| "not a canvas")?;
    let Gpu { surface, device, queue, config } = open_gpu(&canvas).await?;

    let settings = load_settings();
    let saved_settings = serde_json::to_string(&settings).unwrap_or_default();
    let mut session = Session::new(settings, None);
    session.ui.web = true;
    session.ui.tablet_status = "Pen: the browser's (pressure and tilt)".into();
    let renderer = Renderer::new(&device, config.format, &session.ed);
    let ppp = window().device_pixel_ratio() as f32 * session.ui.settings.ui_scale;
    let app = Rc::new(RefCell::new(App {
        canvas: canvas.clone(),
        surface,
        device,
        queue,
        config,
        session,
        renderer,
        events: Vec::new(),
        mods: egui::Modifiers::NONE,
        space_held: false,
        ppp,
        frame_wanted: true,
        repaint_at: None,
        start: Instant::now(),
        cursor: egui::CursorIcon::Default,
        saved_settings,
        frames: 0,
        title: String::new(),
        touches: Vec::new(),
        down: Vec::new(),
        last_pressure: 0.5,
        pen_count: 0,
        pen_window: 0.0,
        pinching: false,
        last_autosave: Instant::now(),
        timings: Vec::new(),
        gpu_lost: false,
    }));
    expose_stats(&app);
    restore_autosave(&app);
    listen(&app, &canvas)?;
    if let Some(el) = document.get_element_by_id("status") {
        el.remove();
    }
    animation_loop(app);
    Ok(())
}

/// `window.recedoStats()`: the last frames' times (ms) as `{frames, ui, picture, draw, total}`,
/// each `[median, worst]`, and `clear()` to start counting again. For measuring speed.
fn expose_stats(app: &Rc<RefCell<App>>) {
    let a = app.clone();
    let stats = Closure::<dyn FnMut(JsValue) -> JsValue>::new(move |clear: JsValue| {
        let Ok(mut app) = a.try_borrow_mut() else { return JsValue::NULL };
        if clear.is_truthy() {
            app.timings.clear();
            return JsValue::NULL;
        }
        let pick = |i: usize| {
            let mut v: Vec<f64> = app.timings.iter().map(|t| t[i]).collect();
            v.sort_by(f64::total_cmp);
            let median = v.get(v.len() / 2).copied().unwrap_or(0.0);
            format!("[{median:.2},{:.2}]", v.last().copied().unwrap_or(0.0))
        };
        let json = format!(r#"{{"frames":{},"ui":{},"picture":{},"draw":{},"total":{}}}"#, app.timings.len(), pick(0), pick(1), pick(2), pick(3));
        js_sys::JSON::parse(&json).unwrap_or(JsValue::NULL)
    });
    let _ = js_sys::Reflect::set(&window(), &JsValue::from_str("recedoStats"), stats.as_ref());
    stats.forget();
    // `window.recedoState()`: the tool, the undo steps (names, how many applied) and the
    // selection's bounds, for browser tests.
    let a = app.clone();
    let state = Closure::<dyn FnMut() -> JsValue>::new(move || {
        let Ok(app) = a.try_borrow() else { return JsValue::NULL };
        let ed = &app.session.ed;
        let (names, applied) = ed.history.entries();
        let selection = ed.doc.selection.as_ref().map(|m| m.bounds);
        let c = app.session.canvas_rect;
        let json = serde_json::json!({ "tool": ed.tool.id(), "history": names, "applied": applied, "selection": selection, "canvas": [c.min.x, c.min.y, c.max.x, c.max.y] });
        js_sys::JSON::parse(&json.to_string()).unwrap_or(JsValue::NULL)
    });
    let _ = js_sys::Reflect::set(&window(), &JsValue::from_str("recedoState"), state.as_ref());
    state.forget();
}

/// The canvas's size in device pixels (it fills the window).
fn canvas_size(canvas: &HtmlCanvasElement) -> (u32, u32) {
    let dpr = window().device_pixel_ratio();
    let w = ((canvas.client_width().max(1) as f64) * dpr).round() as u32;
    let h = ((canvas.client_height().max(1) as f64) * dpr).round() as u32;
    (w.max(1), h.max(1))
}

/// The frame callback, which schedules itself again each frame.
type FrameCallback = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

fn animation_loop(app: Rc<RefCell<App>>) {
    let f: FrameCallback = Rc::new(RefCell::new(None));
    let g = f.clone();
    *g.borrow_mut() = Some(Closure::new(move || {
        let requests = app.borrow_mut().tick();
        for r in requests {
            handle_request(&app, r);
        }
        let _ = window().request_animation_frame(f.borrow().as_ref().expect("set").as_ref().unchecked_ref());
    }));
    let _ = window().request_animation_frame(g.borrow().as_ref().expect("set").as_ref().unchecked_ref());
}

impl App {
    fn wants_frame(&self) -> bool {
        self.frame_wanted || self.session.ed.needs_ticks() || self.repaint_at.is_some_and(|t| Instant::now() >= t)
    }

    /// One animation frame: draws if something changed. Returns what the interface asked for.
    fn tick(&mut self) -> Vec<Request> {
        let (w, h) = canvas_size(&self.canvas);
        if (w, h) != (self.config.width, self.config.height) {
            self.canvas.set_width(w);
            self.canvas.set_height(h);
            self.config.width = w;
            self.config.height = h;
            self.surface.configure(&self.device, &self.config);
            self.frame_wanted = true;
        }
        // Also while nothing is drawn: a painting left alone is the one to keep.
        self.autosave(false);
        if self.gpu_lost || !self.wants_frame() {
            return Vec::new();
        }
        self.frame_wanted = false;
        self.repaint_at = None;
        let now = Instant::now();
        self.session.ed.tick(now_us(), now);
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            _ => {
                self.surface.configure(&self.device, &self.config);
                self.frame_wanted = true;
                return Vec::new();
            }
        };
        let native = window().device_pixel_ratio() as f32;
        let ppp = native * self.session.ui.settings.ui_scale.clamp(0.5, 3.0);
        let mut raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w as f32 / ppp, h as f32 / ppp))),
            time: Some((now - self.start).as_secs_f64()),
            events: std::mem::take(&mut self.events),
            focused: true,
            ..Default::default()
        };
        raw.viewports.entry(raw.viewport_id).or_default().native_pixels_per_point = Some(native);
        // The panel can also be closed from its own × button.
        self.session.ui.reference_open = reference_fn("isOpen").and_then(|f| f.call0(&JsValue::NULL).ok()).is_some_and(|v| v.is_truthy());
        let t_ui = Instant::now();
        let (out, ui_out) = self.session.ui_frame(raw);
        let ui_ms = t_ui.elapsed().as_secs_f64() * 1000.0;
        self.ppp = out.pixels_per_point;
        self.cursor = out.platform_output.cursor_icon;
        if let Some(v) = out.viewport_output.get(&egui::ViewportId::ROOT) {
            if v.repaint_delay.is_zero() {
                self.frame_wanted = true;
            } else if v.repaint_delay < Duration::from_secs(3600) {
                self.repaint_at = Some(now + v.repaint_delay);
            }
        }
        let t_pic = Instant::now();
        self.renderer.sync(&self.device, &self.queue, &mut self.session.ed);
        let pic_ms = t_pic.elapsed().as_secs_f64() * 1000.0;
        let t_draw = Instant::now();
        let view = frame.texture.create_view(&Default::default());
        self.renderer.render(&self.device, &self.queue, Target { view: &view, size: [w, h] }, &self.session.ed, &self.session.egui, out);
        self.queue.present(frame);
        let draw_ms = t_draw.elapsed().as_secs_f64() * 1000.0;
        if self.timings.len() == TIMED_FRAMES {
            self.timings.remove(0);
        }
        self.timings.push([ui_ms, pic_ms, draw_ms, now.elapsed().as_secs_f64() * 1000.0]);
        self.frames += 1;
        self.after_frame();
        ui_out.requests
    }

    fn after_frame(&mut self) {
        let ed = &self.session.ed;
        let title = format!("{}{} - Recedo", ed.title(), if ed.has_unsaved_changes() { "*" } else { "" });
        if title != self.title {
            if let Some(d) = window().document() {
                d.set_title(&title);
            }
            self.title = title;
        }
        self.update_cursor(None);
        // Keep the settings (panel layout, brush, colour) a few times a second at most.
        if self.frames.is_multiple_of(30) {
            let s = serde_json::to_string(&self.session.settings()).unwrap_or_default();
            if s != self.saved_settings {
                if let Some(st) = window().local_storage().ok().flatten() {
                    let _ = st.set_item(SETTINGS_KEY, &s);
                }
                self.saved_settings = s;
            }
        }
    }

    /// Crosshair (or move cursor) over the canvas, the interface's cursor elsewhere.
    fn update_cursor(&mut self, at: Option<(f64, f64)>) {
        let over = at.or(self.session.ed.hover).is_some_and(|(x, y)| self.session.over_canvas(x, y));
        let css = if over {
            if self.space_held || self.session.ed.tool == Tool::Pan { "move" } else { "crosshair" }
        } else {
            use egui::CursorIcon as C;
            match self.cursor {
                C::PointingHand => "pointer",
                C::Text => "text",
                C::ResizeHorizontal | C::ResizeColumn | C::ResizeEast | C::ResizeWest => "ew-resize",
                C::ResizeVertical | C::ResizeRow | C::ResizeNorth | C::ResizeSouth => "ns-resize",
                C::Grab => "grab",
                C::Grabbing => "grabbing",
                C::Move | C::AllScroll => "move",
                C::NotAllowed | C::NoDrop => "not-allowed",
                _ => "default",
            }
        };
        let _ = self.canvas.style().set_property("cursor", css);
    }

    /// Keeps a copy of a painting with unsaved changes in the browser's storage, every
    /// autosave interval (Settings), between strokes.
    /// `now`: without waiting for the interval (the page is being left, or lost its graphics).
    fn autosave(&mut self, now: bool) {
        let ed = &mut self.session.ed;
        let every = Duration::from_secs(self.session.ui.settings.autosave_minutes.max(1) as u64 * 60);
        if !ed.has_unsaved_changes() || !ed.autosaver.copy_is_behind() || ed.stroke_active() || (!now && self.last_autosave.elapsed() < every) {
            return;
        }
        self.last_autosave = Instant::now();
        let snapshot = Instant::now();
        if let Ok(bytes) = recedo_core::kra::save(&ed.doc) {
            let name = format!("{}.kra", stem(&ed.title()));
            store_call("put", &[JsValue::from_str(&name), js_sys::Uint8Array::from(&bytes[..]).into()]);
            ed.autosaver.autosaved(snapshot);
        }
    }

    /// The status bar's pen readout: readings per second and the pressure, so a pen that
    /// reports no pressure (or few readings) is easy to see.
    fn pen_stats(&mut self, samples: &[PenSample], now_ms: f64) {
        self.pen_count += samples.len() as u32;
        if now_ms - self.pen_window >= 1000.0 {
            let s = samples.last().expect("one sample");
            let what = if s.source == Source::Ink { "Pen" } else { "Mouse" };
            self.session.ui.tablet_status = if s.contact() {
                format!("{what}: {} readings/s, pressure {:.2}", self.pen_count, s.pressure)
            } else {
                format!("{what}: {} readings/s", self.pen_count)
            };
            self.pen_count = 0;
            self.pen_window = now_ms;
        }
    }

    /// The page lost the keyboard and pointer (another window or tab came to the front): a
    /// stroke in progress ends there, and no key or button stays held down, so nothing
    /// paints or pans by itself on coming back.
    fn let_go(&mut self) {
        let now = now_us();
        self.session.ed.pen_gone(Source::Ink, now);
        self.session.ed.pen_gone(Source::Mouse, now);
        self.down.clear();
        self.touches.clear();
        self.pinching = false;
        self.space_held = false;
        self.set_mods(egui::Modifiers::NONE);
        for button in [egui::PointerButton::Primary, egui::PointerButton::Middle, egui::PointerButton::Secondary] {
            self.events.push(egui::Event::PointerButton { pos: egui::Pos2::ZERO, button, pressed: false, modifiers: egui::Modifiers::NONE });
        }
        self.events.push(egui::Event::PointerGone);
        self.frame_wanted = true;
    }

    /// The browser gave the graphics back: everything on the GPU is made again (the whole
    /// painting, the interface's letters and images).
    fn gpu_back(&mut self, gpu: Gpu) {
        self.renderer = Renderer::new(&gpu.device, gpu.config.format, &self.session.ed);
        self.renderer.apply(&gpu.device, &gpu.queue, &self.session.ed, crate::editor::Refresh::All);
        let Gpu { surface, device, queue, config } = gpu;
        (self.surface, self.device, self.queue, self.config) = (surface, device, queue, config);
        // A new letter atlas (egui only sends changes to the old one otherwise); the app
        // adds no font providers, so this only makes egui load its fonts again.
        self.session.egui.set_font_providers(Vec::new());
        self.session.ui.previews.forget();
        self.gpu_lost = false;
        self.frame_wanted = true;
    }

    fn pos(&self, x: f64, y: f64) -> egui::Pos2 {
        egui::pos2(x as f32 / self.ppp, y as f32 / self.ppp)
    }

    fn modifiers(&self) -> Modifiers {
        Modifiers { ctrl: self.mods.ctrl || self.mods.mac_cmd, shift: self.mods.shift, alt: self.mods.alt, space: self.space_held }
    }

    /// Records the keys held, and tells the interface when they change (egui knows which
    /// are held only from these events; Ctrl+Enter in the text box needs it).
    fn set_mods(&mut self, m: egui::Modifiers) {
        if m != self.mods {
            self.events.push(egui::Event::ModifiersChanged(m));
        }
        self.mods = m;
    }
}

fn num(v: &JsValue, name: &str) -> f64 {
    js_sys::Reflect::get(v, &JsValue::from_str(name)).ok().and_then(|x| x.as_f64()).unwrap_or(0.0)
}

/// A pointer event as a pen sample, in canvas pixels. `contact` comes from the pointer's
/// down/up events, not from this event's buttons: the extra (coalesced) readings a browser
/// bundles with an event may not carry the button state, and a reading taken for "lifted"
/// in the middle of a stroke would split it into separate dabs. `parent_pressure` stands in
/// when such a reading reports no pressure while touching.
fn sample(e: &PointerEvent, dpr: f64, perf_now: f64, contact: bool, parent_pressure: f32, eraser: bool) -> PenSample {
    let pen = e.pointer_type() == "pen";
    let buttons = e.buttons() as u32;
    let mut flags = Flags(0);
    flags.set(Flags::CONTACT, contact);
    flags.set(Flags::HAS_PRESSURE, pen);
    flags.set(Flags::HAS_TILT, pen);
    flags.set(Flags::ERASER, pen && (eraser || buttons & 32 != 0));
    let own = e.pressure();
    let pressure = if !contact {
        0.0
    } else if pen {
        (if own > 0.0 && own.is_finite() { own } else { parent_pressure }).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let recv = now_us();
    // The event's own time, on the app clock.
    let age_us = ((perf_now - e.time_stamp()) * 1000.0).max(0.0) as u64;
    PenSample {
        source: if pen { Source::Ink } else { Source::Mouse },
        t_us: recv.saturating_sub(age_us),
        recv_us: recv,
        x: num(e, "clientX") * dpr,
        y: num(e, "clientY") * dpr,
        pressure,
        tilt_x: if pen { e.tilt_x() as f32 } else { 0.0 },
        tilt_y: if pen { e.tilt_y() as f32 } else { 0.0 },
        twist: if pen { e.twist() as f32 } else { 0.0 },
        flags,
        raw_pressure: (e.pressure() * 1024.0) as u32,
        raw_buttons: buttons,
        serial: 0,
        cursor: e.pointer_id() as u32,
    }
}

fn add<E: wasm_bindgen::convert::FromWasmAbi + 'static>(target: &web_sys::EventTarget, name: &str, passive: bool, f: impl FnMut(E) + 'static) -> Result<(), String> {
    let closure = Closure::<dyn FnMut(E)>::new(f);
    let opts = web_sys::AddEventListenerOptions::new();
    opts.set_passive(passive);
    target
        .add_event_listener_with_callback_and_add_event_listener_options(name, closure.as_ref().unchecked_ref(), &opts)
        .map_err(|_| format!("could not listen to {name}"))?;
    closure.forget();
    Ok(())
}

fn egui_button(b: i16) -> Option<egui::PointerButton> {
    match b {
        0 => Some(egui::PointerButton::Primary),
        1 => Some(egui::PointerButton::Middle),
        2 => Some(egui::PointerButton::Secondary),
        _ => None,
    }
}

fn listen(app: &Rc<RefCell<App>>, canvas: &HtmlCanvasElement) -> Result<(), String> {
    let c: &web_sys::EventTarget = canvas.as_ref();
    let perf = window().performance().ok_or("no performance clock")?;

    // Pointer: every sample (coalesced ones too) to the editor; the newest to the interface.
    for name in ["pointerdown", "pointermove", "pointerup", "pointercancel", "lostpointercapture"] {
        let (a, perf, cv) = (app.clone(), perf.clone(), canvas.clone());
        add(c, name, false, move |e: PointerEvent| {
            e.prevent_default();
            let dpr = window().device_pixel_ratio();
            let now = perf.now();
            let kind = e.type_();
            // A key pressed just before (B, then straight away a stroke) takes effect first:
            // the interface handles keys in its frame, the pen goes to the editor at once.
            let keys_waiting = a.try_borrow().is_ok_and(|app| app.events.iter().any(|e| matches!(e, egui::Event::Key { pressed: true, .. } | egui::Event::Text(_))));
            if kind == "pointerdown" && keys_waiting {
                let requests = a.borrow_mut().tick();
                for r in requests {
                    handle_request(&a, r);
                }
            }
            let mut app = a.borrow_mut();
            if kind == "pointerdown" {
                let _ = cv.set_pointer_capture(e.pointer_id());
            }
            if e.pointer_type() == "touch" {
                touch(&mut app, &e, &kind, dpr);
                return;
            }
            // Keys held, as the event reports them (also a key pressed before the page had focus).
            app.set_mods(egui::Modifiers { alt: e.alt_key(), ctrl: e.ctrl_key(), shift: e.shift_key(), mac_cmd: e.meta_key(), command: e.ctrl_key() || e.meta_key() });
            // Touching from pointerdown (pen tip or eraser, mouse left or middle button) to
            // pointerup or pointercancel.
            let id = e.pointer_id();
            let buttons = e.buttons() as u32;
            match kind.as_str() {
                "pointerdown" if buttons & (1 | 4 | 32) != 0 || e.button() == 0 || e.button() == 1 || e.button() == 5 => {
                    app.down.retain(|d| d.0 != id);
                    app.down.push((id, buttons & 32 != 0 || e.button() == 5));
                }
                "pointerup" | "pointercancel" | "lostpointercapture" => app.down.retain(|d| d.0 != id),
                _ => {}
            }
            let held = app.down.iter().find(|d| d.0 == id).copied();
            let contact = held.is_some();
            let eraser = held.is_some_and(|d| d.1);
            let parent_pressure = if e.pressure() > 0.0 { e.pressure() } else { app.last_pressure };
            if contact && e.pointer_type() == "pen" && e.pressure() > 0.0 {
                app.last_pressure = e.pressure();
            }
            let mut samples = Vec::new();
            if kind == "pointermove" {
                for ce in e.get_coalesced_events().iter() {
                    if let Ok(ce) = ce.dyn_into::<PointerEvent>() {
                        samples.push(sample(&ce, dpr, now, contact, parent_pressure, eraser));
                    }
                }
            }
            if samples.is_empty() {
                samples.push(sample(&e, dpr, now, contact, parent_pressure, eraser));
            }
            app.pen_stats(&samples, now);
            let mut mods = app.modifiers();
            // The middle mouse button drags the view, like holding Space.
            if e.pointer_type() == "mouse" && e.buttons() & 4 != 0 {
                mods = Modifiers { space: true, ..Modifiers::default() };
            }
            for s in &samples {
                let over = app.session.over_canvas(s.x, s.y);
                app.session.ed.pen(s, over, mods);
            }
            let last = *samples.last().expect("one sample");
            let pos = app.pos(last.x, last.y);
            let modifiers = app.mods;
            app.events.push(egui::Event::PointerMoved(pos));
            if (kind == "pointerdown" || kind == "pointerup" || kind == "pointercancel")
                && let Some(button) = egui_button(e.button())
            {
                app.events.push(egui::Event::PointerButton { pos, button, pressed: kind == "pointerdown", modifiers });
            }
            app.update_cursor(Some((last.x, last.y)));
            app.frame_wanted = true;
        })?;
    }
    {
        let a = app.clone();
        add(c, "pointerleave", true, move |_e: PointerEvent| {
            let mut app = a.borrow_mut();
            app.events.push(egui::Event::PointerGone);
            app.session.ed.hover = None;
            app.frame_wanted = true;
        })?;
    }
    add(c, "contextmenu", false, |e: web_sys::Event| e.prevent_default())?;

    // Wheel: zoom over the canvas, scroll panels elsewhere.
    {
        let a = app.clone();
        add(c, "wheel", false, move |e: WheelEvent| {
            e.prevent_default();
            let dpr = window().device_pixel_ratio();
            let mut app = a.borrow_mut();
            let at = (num(&e, "clientX") * dpr, num(&e, "clientY") * dpr);
            let scale = match e.delta_mode() {
                1 => 40.0,
                2 => 800.0,
                _ => 1.0,
            };
            let (dx, dy) = (e.delta_x() * scale, e.delta_y() * scale);
            if app.session.over_canvas(at.0, at.1) {
                app.session.ed.wheel(-dy / 100.0, at);
            } else {
                let modifiers = app.mods;
                app.events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(-dx as f32, -dy as f32),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Wheel,
                    modifiers,
                });
            }
            app.frame_wanted = true;
        })?;
    }

    // Keyboard, on the whole window.
    let w: web_sys::EventTarget = window().into();
    for name in ["keydown", "keyup"] {
        let a = app.clone();
        add(&w, name, false, move |e: KeyboardEvent| {
            // Typing in the page's own fields (the reference panel's search box) is not for
            // the app: no shortcuts, and Backspace must keep working there.
            let field = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).is_some_and(|el| {
                matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT") || el.closest("#reference").ok().flatten().is_some()
            });
            if field {
                return;
            }
            let mut app = a.borrow_mut();
            let pressed = e.type_() == "keydown";
            app.set_mods(egui::Modifiers { alt: e.alt_key(), ctrl: e.ctrl_key(), shift: e.shift_key(), mac_cmd: e.meta_key(), command: e.ctrl_key() || e.meta_key() });
            let typing = app.session.egui.egui_wants_keyboard_input();
            let key = e.key();
            app.frame_wanted = true;
            if key == " " && !typing {
                // Space is the pan key, never a key for the interface.
                e.prevent_default();
                app.space_held = pressed;
                app.update_cursor(None);
                return;
            }
            // Layout-independent letters and digits (Ctrl+Z is Ctrl+Z on any keyboard).
            let code = e.code();
            let name = code.strip_prefix("Key").or_else(|| code.strip_prefix("Digit")).map_or(key.clone(), str::to_string);
            let mods = app.mods;
            if let Some(k) = egui::Key::from_name(&name).or_else(|| egui::Key::from_name(&key)) {
                app.events.push(egui::Event::Key { key: k, physical_key: Some(k), pressed, repeat: pressed && e.repeat(), modifiers: mods });
            }
            if pressed && key.chars().count() == 1 && !mods.ctrl && !mods.mac_cmd {
                app.events.push(egui::Event::Text(key.clone()));
            }
            // Keep the browser's own shortcuts (Ctrl+S saves the page, Ctrl+O opens a file,
            // Ctrl+D bookmarks, Backspace goes back, Tab, …) from acting while Recedo uses
            // them. (A few, such as Ctrl+W, Ctrl+T and Ctrl+N, the browser keeps.)
            if mods.ctrl || mods.mac_cmd || key == "Tab" || (!typing && (key == "Backspace" || key.starts_with("Arrow") || key.starts_with('F') && key.len() > 1)) {
                e.prevent_default();
            }
        })?;
    }
    {
        let a = app.clone();
        add(&w, "blur", true, move |_e: web_sys::Event| a.borrow_mut().let_go())?;
    }
    // Leaving the tab: a copy of unsaved work right away (a hidden tab can be closed by the
    // browser to free memory, and the graphics can be taken away).
    {
        let a = app.clone();
        let d: web_sys::EventTarget = window().document().ok_or("no document")?.into();
        add(&d, "visibilitychange", true, move |_e: web_sys::Event| {
            if window().document().is_some_and(|d| d.hidden()) {
                let mut app = a.borrow_mut();
                app.let_go();
                app.autosave(true);
            }
        })?;
    }
    // The browser can take the graphics away (a hidden tab, the computer sleeping, the
    // graphics driver restarting). Saying "prevent default" asks for them back; then the
    // drawing side is set up again and the painting shown as it was.
    {
        let a = app.clone();
        add(c, "webglcontextlost", false, move |e: web_sys::Event| {
            e.prevent_default();
            let mut app = a.borrow_mut();
            app.gpu_lost = true;
            app.let_go();
            app.autosave(true);
        })?;
        let (a, cv) = (app.clone(), canvas.clone());
        add(c, "webglcontextrestored", true, move |_e: web_sys::Event| {
            let (a, cv) = (a.clone(), cv.clone());
            wasm_bindgen_futures::spawn_local(async move {
                match open_gpu(&cv).await {
                    Ok(gpu) => a.borrow_mut().gpu_back(gpu),
                    Err(e) => show_error(&format!("The graphics stopped and could not be started again ({e}). Reload the page: your painting was autosaved.")),
                }
            });
        })?;
    }
    // Leaving with unsaved changes: the browser asks first.
    {
        let a = app.clone();
        add(&w, "beforeunload", false, move |e: web_sys::BeforeUnloadEvent| {
            if a.try_borrow().is_ok_and(|app| app.session.ed.has_unsaved_changes()) {
                e.prevent_default();
                e.set_return_value("unsaved");
            }
        })?;
    }
    Ok(())
}

/// What the interface asked for that only the shell can do.
fn handle_request(app: &Rc<RefCell<App>>, r: Request) {
    match r {
        Request::OpenDialog => pick_file(app.clone()),
        Request::SaveAsDialog => {
            let mut a = app.borrow_mut();
            let snapshot = Instant::now();
            let name = format!("{}.kra", stem(&a.session.ed.title()));
            match a.session.ed.encode_kra() {
                Ok(bytes) => {
                    if download(&name, &bytes, "application/x-krita").is_ok() {
                        a.session.ed.saved_elsewhere(&name, snapshot);
                        store_call("clear", &[]);
                    }
                }
                Err(e) => a.session.ed.notices.push(crate::editor::Notice { kind: crate::editor::NoticeKind::Error, text: format!("Could not save: {e}") }),
            }
            a.frame_wanted = true;
        }
        Request::ExportDialog(format) => {
            let mut a = app.borrow_mut();
            let (ext, mime) = match format {
                ExportFormat::Png => ("png", "image/png"),
                ExportFormat::Jpeg(_) => ("jpg", "image/jpeg"),
            };
            let name = format!("{}.{ext}", stem(&a.session.ed.title()));
            match a.session.ed.encode_export(format) {
                Ok(bytes) => {
                    let _ = download(&name, &bytes, mime);
                }
                Err(e) => a.session.ed.notices.push(crate::editor::Notice { kind: crate::editor::NoticeKind::Error, text: format!("Could not export: {e}") }),
            }
            a.frame_wanted = true;
        }
        Request::CanvasOnly(on) => {
            if let Some(f) = reference_fn("setHidden") {
                let _ = f.call1(&JsValue::NULL, &JsValue::from_bool(on));
            }
            app.borrow_mut().frame_wanted = true;
        }
        Request::ToggleReference => {
            if let Some(f) = reference_fn("toggle") {
                let _ = f.call0(&JsValue::NULL);
            }
            app.borrow_mut().frame_wanted = true;
        }
        Request::Quit | Request::SetTabletApi(_) => {}
    }
}

fn stem(title: &str) -> String {
    std::path::Path::new(title).file_stem().map_or_else(|| title.to_string(), |s| s.to_string_lossy().into_owned())
}

/// Offers bytes as a downloaded file.
fn download(name: &str, bytes: &[u8], mime: &str) -> Result<(), JsValue> {
    let parts = js_sys::Array::of1(&js_sys::Uint8Array::from(bytes));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type(mime);
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &opts)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)?;
    let document = window().document().ok_or("no document")?;
    let a: web_sys::HtmlAnchorElement = document.create_element("a")?.dyn_into()?;
    a.set_href(&url);
    a.set_download(name);
    a.click();
    // Let the download start before the address is released.
    let release = Closure::once_into_js(move || {
        let _ = web_sys::Url::revoke_object_url(&url);
    });
    window().set_timeout_with_callback_and_timeout_and_arguments_0(release.unchecked_ref(), 60_000)?;
    Ok(())
}

/// The browser's file picker; the chosen file is opened from its bytes.
fn pick_file(app: Rc<RefCell<App>>) {
    let Some(document) = window().document() else { return };
    let Ok(input) = document.create_element("input").and_then(|e| e.dyn_into::<web_sys::HtmlInputElement>().map_err(Into::into)) else { return };
    input.set_type("file");
    input.set_accept(".kra,.png,.jpg,.jpeg,image/png,image/jpeg");
    let picker = input.clone();
    let on_change = Closure::once_into_js(move || {
        let Some(file) = picker.files().and_then(|f| f.get(0)) else { return };
        let name = file.name();
        wasm_bindgen_futures::spawn_local(async move {
            let Ok(buf) = wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await else { return };
            let bytes = js_sys::Uint8Array::new(&buf).to_vec();
            let mut a = app.borrow_mut();
            let App { session, .. } = &mut *a;
            session.ui.open_bytes(&mut session.ed, &name, &bytes);
            a.frame_wanted = true;
        });
    });
    input.set_onchange(Some(on_change.unchecked_ref()));
    input.click();
}

/// Fingers: one finger pans (through the editor's pan, like Space+drag), two pan and
/// pinch-zoom about the point between them. Fingers never paint.
fn touch(app: &mut App, e: &PointerEvent, kind: &str, dpr: f64) {
    let id = e.pointer_id();
    let p = (num(e, "clientX") * dpr, num(e, "clientY") * dpr);
    let before = app.touches.iter().find(|t| t.0 == id).map(|t| t.1);
    match kind {
        "pointerdown" => app.touches.push((id, p)),
        "pointermove" => {
            if let Some(t) = app.touches.iter_mut().find(|t| t.0 == id) {
                t.1 = p;
            }
        }
        _ => app.touches.retain(|t| t.0 != id),
    }
    let pos = app.pos(p.0, p.1);
    let modifiers = app.mods;
    // The interface gets fingers as clicks and drags (panels, menus, sliders).
    app.events.push(egui::Event::PointerMoved(pos));
    if kind != "pointermove" {
        app.events.push(egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: kind == "pointerdown", modifiers });
    }
    app.frame_wanted = true;
    if app.touches.len() >= 2 {
        if !app.pinching {
            app.pinching = true;
            app.session.ed.pen_gone(Source::Mouse, now_us());
        }
        // Move with the finger that moved, against the other one.
        if let (Some(old), Some(other)) = (before, app.touches.iter().find(|t| t.0 != id).map(|t| t.1)) {
            let d0 = (old.0 - other.0).hypot(old.1 - other.1);
            let d1 = (p.0 - other.0).hypot(p.1 - other.1);
            let view = &mut app.session.ed.view;
            view.pan((p.0 - old.0) / 2.0, (p.1 - old.1) / 2.0);
            if d0 > 10.0 && d1 > 10.0 {
                view.zoom_by(d1 / d0, ((p.0 + other.0) / 2.0, (p.1 + other.1) / 2.0));
            }
        }
        return;
    }
    if app.touches.is_empty() {
        app.pinching = false;
    }
    if app.pinching {
        return;
    }
    let mut s = PenSample::synthetic(now_us(), p.0, p.1, 1.0, kind == "pointerdown" || kind == "pointermove" && before.is_some());
    s.source = Source::Mouse;
    s.cursor = id as u32;
    if kind == "pointerup" || kind == "pointercancel" {
        s.flags.set(Flags::CONTACT, false);
    }
    let over = app.session.over_canvas(p.0, p.1);
    app.session.ed.pen(&s, over, Modifiers { space: true, ..Modifiers::default() });
}

/// Calls `window.recedoStore.<name>(...)` (the browser storage helper in index.html).
/// A function of the reference panel (`web/reference.js`), if the page has it.
fn reference_fn(name: &str) -> Option<js_sys::Function> {
    let panel = js_sys::Reflect::get(&window(), &JsValue::from_str("recedoReference")).ok()?;
    js_sys::Reflect::get(&panel, &JsValue::from_str(name)).ok()?.dyn_into().ok()
}

fn store_call(name: &str, args: &[JsValue]) -> Option<js_sys::Promise> {
    let store = js_sys::Reflect::get(&window(), &JsValue::from_str("recedoStore")).ok()?;
    let f: js_sys::Function = js_sys::Reflect::get(&store, &JsValue::from_str(name)).ok()?.dyn_into().ok()?;
    f.apply(&store, &args.iter().collect::<js_sys::Array>()).ok()?.dyn_into().ok()
}

/// After a crash or a closed tab: the painting's last autosaved copy opens again, unsaved.
fn restore_autosave(app: &Rc<RefCell<App>>) {
    let Some(p) = store_call("get", &[]) else { return };
    let app = app.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let Ok(rec) = wasm_bindgen_futures::JsFuture::from(p).await else { return };
        if rec.is_undefined() || rec.is_null() {
            return;
        }
        let name = js_sys::Reflect::get(&rec, &JsValue::from_str("name")).ok().and_then(|v| v.as_string()).unwrap_or_else(|| "Recovered.kra".into());
        let Ok(bytes) = js_sys::Reflect::get(&rec, &JsValue::from_str("bytes")) else { return };
        let bytes = js_sys::Uint8Array::new(&bytes).to_vec();
        let mut a = app.borrow_mut();
        let App { session, .. } = &mut *a;
        session.ui.open_bytes(&mut session.ed, &name, &bytes);
        session.ed.autosaver.changed(Instant::now());
        session.ed.notices.push(crate::editor::Notice {
            kind: crate::editor::NoticeKind::Info,
            text: format!("Your unsaved painting \"{name}\" was restored from the last autosave. Save it with Ctrl+S."),
        });
        a.frame_wanted = true;
    });
}
