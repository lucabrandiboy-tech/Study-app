//! The interface, run off-screen: frames render without panics, the canvas area is where it
//! should be, menus and dialogs work, and (with `RECEDO_SCREENSHOTS=<dir>`) screenshots are
//! saved for a person to look at.

use egui::{Event, PointerButton, Pos2, RawInput, Rect, pos2, vec2};
use recedo_tablet::PenSample;

use crate::editor::Modifiers;
use crate::session::{Renderer, Session};
use crate::settings::Settings;

use super::*;

const W: u32 = 1600;
const H: u32 = 900;

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

fn gpu() -> Option<Gpu> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor { backends: wgpu::Backends::all(), ..wgpu::InstanceDescriptor::new_without_display_handle() });
    let found = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).map_err(|e| e.to_string()).and_then(|adapter| {
        let desc = wgpu::DeviceDescriptor { required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()), ..Default::default() };
        pollster::block_on(adapter.request_device(&desc)).map_err(|e| e.to_string())
    });
    match found {
        Ok((device, queue)) => Some(Gpu { device, queue }),
        Err(e) if std::env::var("RECEDO_REQUIRE_GPU").as_deref() == Ok("1") => {
            panic!("no GPU for rendering tests: {e}")
        }
        Err(e) => {
            eprintln!("SKIPPED GPU test (set RECEDO_REQUIRE_GPU=1 to make this a failure): {e}");
            None
        }
    }
}

fn raw(events: Vec<Event>) -> RawInput {
    // A second per frame, so menus and dialogs have finished fading in when photographed.
    thread_local!(static T: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) });
    let time = T.with(|t| {
        t.set(t.get() + 1.0);
        t.get()
    });
    RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(W as f32, H as f32))), events, time: Some(time), predicted_dt: 1.0, ..Default::default() }
}

fn click(at: Pos2) -> Vec<Event> {
    vec![
        Event::PointerMoved(at),
        Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::NONE },
        Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::NONE },
    ]
}

/// Runs frames with `events` (then two quiet frames so popups settle) and returns the last
/// frame's output, to be drawn by `shot` (the others hand their textures to `r`).
fn frames(g: &Gpu, r: &mut Renderer, s: &mut Session, events: Vec<Event>) -> (egui::FullOutput, FrameOutput) {
    for ev in [events, vec![]] {
        let (mut out, _) = s.ui_frame(raw(ev));
        r.skip_frame(&g.device, &g.queue, &mut out);
    }
    s.ui_frame(raw(vec![]))
}

/// Like `frames`, for when the last output is not drawn either.
fn quiet(g: &Gpu, r: &mut Renderer, s: &mut Session, events: Vec<Event>) -> FrameOutput {
    let (mut out, f) = frames(g, r, s, events);
    r.skip_frame(&g.device, &g.queue, &mut out);
    f
}

/// Renders a frame into an image (and saves it when RECEDO_SCREENSHOTS is set).
fn shot(g: &Gpu, r: &mut Renderer, s: &mut Session, out: egui::FullOutput, name: &str) -> recedo_core::image::Rgba8 {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let target = g.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    r.sync(&g.device, &g.queue, &mut s.ed);
    r.render(&g.device, &g.queue, crate::session::Target { view: &view, size: [W, H] }, &s.ed, &s.egui, out);
    let img = crate::session::read_texture(&g.device, &g.queue, &target).unwrap();
    if let Ok(dir) = std::env::var("RECEDO_SCREENSHOTS") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/{name}.png"), recedo_core::image::encode_png(&img, true).unwrap()).unwrap();
    }
    img
}

fn session() -> Session {
    let mut s = Session::new(Settings { last_preset: 1, ..Settings::default() }, None);
    s.ed.color = [30, 60, 160];
    s
}

#[test]
fn layout_canvas_and_painting_on_screen() {
    let Some(g) = gpu() else { return };
    let mut s = session();
    let mut r = Renderer::new(&g.device, wgpu::TextureFormat::Rgba8Unorm, &s.ed);
    let f = quiet(&g, &mut r, &mut s, vec![]);
    let c = f.canvas_rect;
    assert!(c.width() > 900.0 && c.height() > 700.0, "canvas area {c:?}");
    assert!(c.min.x > 50.0, "left dock");
    assert!(c.max.x < W as f32 - 200.0, "right dock");
    assert!(s.over_canvas(c.center().x as f64, c.center().y as f64));
    assert!(!s.over_canvas(5.0, 5.0), "the menu bar is not canvas");
    assert!(!s.over_canvas(W as f64 - 20.0, 300.0), "the right dock is not canvas");
    // The 1280 x 720 painting is fitted inside the canvas area.
    let (x0, y0) = s.ed.view.to_screen(0.0, 0.0);
    let (x1, y1) = s.ed.view.to_screen(1280.0, 720.0);
    assert!(x0 >= c.min.x as f64 && y0 >= c.min.y as f64 && x1 <= c.max.x as f64 && y1 <= c.max.y as f64, "{x0},{y0} {x1},{y1} in {c:?}");
    // Paint a stroke through the editor, as the pen would.
    let (a, b) = (s.ed.view.to_screen(200.0, 360.0), s.ed.view.to_screen(1000.0, 360.0));
    for i in 0..=40 {
        let t = i as f64 / 40.0;
        let p = PenSample::synthetic(i * 4000, a.0 + (b.0 - a.0) * t, a.1, 0.3 + 0.7 * t as f32, true);
        s.ed.pen(&p, true, Modifiers::default());
    }
    s.ed.pen(&PenSample::synthetic(200_000, b.0, b.1, 0.0, false), true, Modifiers::default());
    let (out, _) = frames(&g, &mut r, &mut s, vec![]);
    let img = shot(&g, &mut r, &mut s, out, "01-main-window");
    // The stroke shows on screen in the painting colour, the page around it is white.
    let mid = s.ed.view.to_screen(600.0, 360.0);
    let px = img.pixel(mid.0 as u32, mid.1 as u32);
    assert!(px[2] > 120 && px[0] < 80, "stroke colour on screen {px:?}");
    let page = s.ed.view.to_screen(600.0, 600.0);
    assert_eq!(img.pixel(page.0 as u32, page.1 as u32), [255, 255, 255, 255]);
    // Grey outside the painting, inside the canvas area.
    let out_px = img.pixel(c.min.x as u32 + 3, (c.center().y) as u32);
    assert!(out_px[0] < 80 && out_px[0] == out_px[1], "background {out_px:?}");
}

#[test]
fn menus_dialogs_and_shortcuts() {
    let Some(g) = gpu() else { return };
    let mut s = session();
    let mut r = Renderer::new(&g.device, wgpu::TextureFormat::Rgba8Unorm, &s.ed);
    quiet(&g, &mut r, &mut s, vec![]);
    // Open the File menu.
    let (out, _) = frames(&g, &mut r, &mut s, click(pos2(18.0, 10.0)));
    shot(&g, &mut r, &mut s, out, "02-file-menu");
    // Ctrl+N opens the New dialog (nothing unsaved yet).
    quiet(&g, &mut r, &mut s, vec![Event::Key { key: Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }]);
    let ctrl_n = Event::Key { key: Key::N, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::COMMAND };
    let (out, _) = frames(&g, &mut r, &mut s, vec![ctrl_n.clone()]);
    assert!(s.ui.dialog_open(), "new canvas dialog");
    assert!(!s.over_canvas(800.0, 450.0), "a dialog covers the canvas");
    shot(&g, &mut r, &mut s, out, "03-new-canvas");
    s.ui.dialog = None;
    // With unsaved changes, Ctrl+N asks first.
    s.ed.add_layer();
    quiet(&g, &mut r, &mut s, vec![ctrl_n]);
    assert!(matches!(s.ui.dialog, Some(Dialog::Unsaved(Then::NewCanvas))));
    let (out, _) = frames(&g, &mut r, &mut s, vec![]);
    shot(&g, &mut r, &mut s, out, "04-unsaved");
    s.ui.dialog = None;
    // Shortcuts reach the editor.
    let key = |k: Key, m: egui::Modifiers| Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: m };
    quiet(&g, &mut r, &mut s, vec![key(Key::E, egui::Modifiers::NONE)]);
    assert_eq!(s.ed.tool, Tool::Eraser);
    let layers = s.ed.doc.layers.len();
    quiet(&g, &mut r, &mut s, vec![key(Key::Z, egui::Modifiers::COMMAND)]);
    assert_eq!(s.ed.doc.layers.len(), layers - 1, "Ctrl+Z undid the new layer");
    quiet(&g, &mut r, &mut s, vec![key(Key::Z, egui::Modifiers::COMMAND | egui::Modifiers::SHIFT)]);
    assert_eq!(s.ed.doc.layers.len(), layers, "Ctrl+Shift+Z redid it");
    let zoom = s.ed.view.zoom;
    quiet(&g, &mut r, &mut s, vec![key(Key::Num1, egui::Modifiers::NONE)]);
    assert_eq!(s.ed.view.zoom, 1.0);
    assert_ne!(zoom, 1.0);
    // Help > Keyboard shortcuts.
    s.ui.dialog = Some(Dialog::Shortcuts);
    let (out, _) = frames(&g, &mut r, &mut s, vec![]);
    shot(&g, &mut r, &mut s, out, "05-shortcuts");
}

#[test]
fn panels_float_and_close_from_the_window_menu() {
    let Some(g) = gpu() else { return };
    let mut s = session();
    let mut r = Renderer::new(&g.device, wgpu::TextureFormat::Rgba8Unorm, &s.ed);
    s.ui.settings.dock.float(Panel::Color, [500.0, 200.0]);
    s.ui.settings.dock.remove(Panel::History);
    let (out, f) = frames(&g, &mut r, &mut s, vec![]);
    assert!(f.canvas_rect.width() > 900.0);
    assert!(!s.over_canvas(560.0, 260.0), "a floating panel is not canvas");
    shot(&g, &mut r, &mut s, out, "06-floating-panel");
    assert!(!s.ui.settings.dock.is_open(Panel::History));
}

#[test]
fn hex_colour_parsing() {
    assert_eq!(parse_hex("#ff8000"), Some([255, 128, 0]));
    assert_eq!(parse_hex("0F0"), Some([0, 255, 0]));
    assert_eq!(parse_hex("12345"), None);
    assert_eq!(parse_hex("zzzzzz"), None);
}

#[test]
fn every_character_the_interface_uses_has_a_glyph() {
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(raw(vec![]), |_| {});
    out.textures_delta.clear();
    let source = include_str!("../ui.rs").to_string() + include_str!("../dock.rs") + include_str!("../editor.rs");
    let mut special: Vec<char> = source.chars().filter(|c| !c.is_ascii()).collect();
    special.sort();
    special.dedup();
    ctx.fonts_mut(|f| {
        for c in special {
            assert!(f.has_glyph(&egui::FontId::proportional(14.0), c), "the interface uses {c:?}, which the fonts do not have");
        }
    });
}
