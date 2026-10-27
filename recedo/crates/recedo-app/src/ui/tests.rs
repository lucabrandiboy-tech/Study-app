//! The interface, run off-screen: frames render without panics, the canvas area is where it
//! should be, menus and dialogs work, and (with `RECEDO_SCREENSHOTS=<dir>`) screenshots are
//! saved for a person to look at.

use egui::{Event, PointerButton, Pos2, RawInput, Rect, pos2, vec2};
use recedo_tablet::PenSample;

use crate::editor::Modifiers;
use crate::session::{Renderer, Session};
use crate::settings::Settings;

use super::*;

/// The screen the tests draw: width and height in pixels, and Windows' display scale.
#[derive(Clone, Copy)]
struct Screen {
    w: u32,
    h: u32,
    scale: f32,
}

thread_local!(static SCREEN: std::cell::Cell<Screen> = const { std::cell::Cell::new(Screen { w: 1600, h: 900, scale: 1.0 }) });

fn screen() -> Screen {
    SCREEN.with(|s| s.get())
}

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
    let sc = screen();
    let mut raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(sc.w as f32 / sc.scale, sc.h as f32 / sc.scale))),
        events,
        time: Some(time),
        predicted_dt: 1.0,
        ..Default::default()
    };
    raw.viewports.entry(raw.viewport_id).or_default().native_pixels_per_point = Some(sc.scale);
    raw
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
    let Screen { w, h, .. } = screen();
    let target = g.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    r.sync(&g.device, &g.queue, &mut s.ed);
    r.render(&g.device, &g.queue, crate::session::Target { view: &view, size: [w, h] }, &s.ed, &s.egui, out);
    let img = crate::session::read_texture(&g.device, &g.queue, &target).unwrap();
    if let Ok(dir) = std::env::var("RECEDO_SCREENSHOTS") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/{name}.png"), recedo_core::image::encode_png(&img, true).unwrap()).unwrap();
    }
    img
}

/// An image dropped in the same frame that still draws it (a layer thumbnail or brush
/// sample being replaced) must not be destroyed before that frame is drawn: submitting a
/// frame that uses a destroyed texture is a wgpu error, which crashed the app.
#[test]
fn an_image_dropped_while_drawn_does_not_crash() {
    let Some(g) = gpu() else { return };
    let mut s = session();
    let mut r = Renderer::new(&g.device, wgpu::TextureFormat::Rgba8Unorm, &s.ed);
    let image = egui::ColorImage::filled([8, 8], egui::Color32::RED);
    let mut handle = None;
    for drop_it in [false, true] {
        let out = s.egui.run_ui(raw(vec![]), |ui| {
            let t = handle.get_or_insert_with(|| ui.ctx().load_texture("test image", image.clone(), egui::TextureOptions::LINEAR));
            ui.image((t.id(), vec2(40.0, 40.0)));
            if drop_it {
                handle = None;
            }
        });
        if drop_it {
            assert!(!out.textures_delta.free.is_empty(), "the image is freed in the frame that draws it");
        }
        shot(&g, &mut r, &mut s, out, "dropped-image");
    }
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
    assert!(c.max.x < screen().w as f32 - 150.0, "right dock");
    assert!(s.over_canvas(c.center().x as f64, c.center().y as f64));
    assert!(!s.over_canvas(5.0, 5.0), "the menu bar is not canvas");
    assert!(!s.over_canvas(screen().w as f64 - 20.0, 300.0), "the right dock is not canvas");
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

/// The XP-Pen Artist 12 is 1920 x 1080 on 11.6 inches; Windows often scales such screens
/// to 150 %, which leaves the interface 1280 x 720 points.
#[test]
fn small_screen_at_150_percent() {
    let Some(g) = gpu() else { return };
    SCREEN.with(|sc| sc.set(Screen { w: 1920, h: 1080, scale: 1.5 }));
    let mut s = Session::new(Settings::default(), None);
    let mut r = Renderer::new(&g.device, wgpu::TextureFormat::Rgba8Unorm, &s.ed);
    let (out, f) = frames(&g, &mut r, &mut s, vec![]);
    shot(&g, &mut r, &mut s, out, "07-1080p-at-150-percent");
    let c = f.canvas_rect;
    assert!(c.width() >= 700.0 && c.height() >= 560.0, "canvas area {c:?} of 1280 x 720 points");
    // Every tool on the strip shows whole, above the colours.
    let bottom = s.ui.strip.iter().map(|(_, r)| r.max.y).fold(0.0, f32::max);
    assert!(bottom <= c.max.y - 30.0, "the strip's last icon ends at {bottom}, the canvas at {}", c.max.y);
    // The canvas area in pixels is what the editor fits the painting into.
    let (_, _, vw, vh) = s.ed.viewport;
    assert!((vw - c.width() as f64 * 1.5).abs() < 1.0 && (vh - c.height() as f64 * 1.5).abs() < 1.0);
}

#[test]
fn shortcuts_fit_a_small_screen_and_the_select_menu_works() {
    let Some(g) = gpu() else { return };
    SCREEN.with(|sc| sc.set(Screen { w: 1366, h: 768, scale: 1.0 }));
    let mut s = session();
    let mut r = Renderer::new(&g.device, wgpu::TextureFormat::Rgba8Unorm, &s.ed);
    s.ui.dialog = Some(Dialog::Shortcuts);
    let (out, _) = frames(&g, &mut r, &mut s, vec![]);
    // The list scrolls inside the dialog: its first line and the Close button are on screen.
    let close = find_text(&out, "Close");
    let first = find_text(&out, "Brush");
    assert!(close.iter().any(|c| c.max.y <= 768.0) && first.iter().any(|f| f.min.y >= 0.0), "{close:?} {first:?}");
    shot(&g, &mut r, &mut s, out, "13-shortcuts-1366x768");
    s.ui.dialog = Some(Dialog::GettingStarted);
    let (out, _) = frames(&g, &mut r, &mut s, vec![]);
    assert!(find_text(&out, "Close").iter().any(|c| c.max.y <= 768.0) && !find_text(&out, "Getting started").is_empty());
    shot(&g, &mut r, &mut s, out, "14-getting-started");
    s.ui.dialog = None;
    // Select > Select all, then Select > Clear the selection.
    let open_select = |s: &mut Session, g: &Gpu, r: &mut Renderer| {
        let (out, _) = frames(g, r, s, vec![]);
        let menu = find_text(&out, "Select").into_iter().find(|m| m.min.y < 20.0).expect("the Select menu");
        r.skip_frame(&g.device, &g.queue, &mut { out });
        frames(g, r, s, click(menu.center())).0
    };
    let out = open_select(&mut s, &g, &mut r);
    let all = find_text(&out, "Select all")[0];
    r.skip_frame(&g.device, &g.queue, &mut { out });
    quiet(&g, &mut r, &mut s, click(all.center()));
    assert!(s.ed.has_selection());
    let out = open_select(&mut s, &g, &mut r);
    let deselect = find_text(&out, "Deselect")[0];
    r.skip_frame(&g.device, &g.queue, &mut { out });
    quiet(&g, &mut r, &mut s, click(deselect.center()));
    assert!(!s.ed.has_selection());
    SCREEN.with(|sc| sc.set(Screen { w: 1600, h: 900, scale: 1.0 }));
}

#[test]
fn g_shows_only_the_canvas_and_the_new_keys() {
    let Some(g) = gpu() else { return };
    let mut s = Session::new(Settings::default(), None);
    assert_eq!(s.ed.color, [0, 0, 0], "black to start with");
    let mut r = Renderer::new(&g.device, wgpu::TextureFormat::Rgba8Unorm, &s.ed);
    let key = |k: Key, m: egui::Modifiers| vec![Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m }];
    let normal = quiet(&g, &mut r, &mut s, vec![]).canvas_rect;
    let f = quiet(&g, &mut r, &mut s, key(Key::G, egui::Modifiers::NONE));
    assert!(s.ui.canvas_only);
    let (w, h) = (screen().w as f32, screen().h as f32);
    assert!(f.canvas_rect.min == pos2(0.0, 0.0) && f.canvas_rect.max == pos2(w, h), "the whole window: {:?}", f.canvas_rect);
    let (out, _) = frames(&g, &mut r, &mut s, vec![]);
    assert!(find_text(&out, "File").is_empty() && find_text(&out, "Layers").is_empty(), "no menus or panels");
    shot(&g, &mut r, &mut s, out, "15-canvas-only");
    let f = quiet(&g, &mut r, &mut s, key(Key::G, egui::Modifiers::NONE));
    assert!(!s.ui.canvas_only && f.canvas_rect == normal, "G again: everything back");
    assert!(s.ui.toasts.iter().all(|(_, _, t)| t != CANVAS_ONLY_HINT), "its hint goes with it");
    quiet(&g, &mut r, &mut s, key(Key::G, egui::Modifiers::NONE));
    let f = quiet(&g, &mut r, &mut s, key(Key::Escape, egui::Modifiers::NONE));
    assert!(!s.ui.canvas_only && f.canvas_rect == normal, "Escape: everything back");
    // Typed into a text field (here the layer name), G is a letter, not the shortcut.
    s.ui.rename = Some((s.ed.active, String::new()));
    quiet(&g, &mut r, &mut s, vec![]);
    let mut typing = key(Key::G, egui::Modifiers::NONE);
    typing.push(Event::Text("g".into()));
    quiet(&g, &mut r, &mut s, typing);
    assert!(!s.ui.canvas_only, "not while typing");
    // Enter finishes the renaming (the layer gets the name typed); then the keys are
    // shortcuts again.
    quiet(&g, &mut r, &mut s, key(Key::Enter, egui::Modifiers::NONE));
    assert!(s.ui.rename.is_none(), "Enter ends the renaming");
    assert_eq!(s.ed.doc.layers[s.ed.active].name, "g");
    assert!(!s.egui.egui_wants_keyboard_input(), "the keyboard is free again");
    quiet(&g, &mut r, &mut s, key(Key::G, egui::Modifiers::SHIFT));
    assert_eq!((s.ed.tool, s.ui.canvas_only), (Tool::Gradient, false), "Shift+G: gradient");
    s.ed.select_all();
    quiet(&g, &mut r, &mut s, key(Key::I, egui::Modifiers::COMMAND));
    assert!(!s.ed.has_selection(), "Ctrl+I inverts (everything selected: nothing left)");
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
    apply_theme(&ctx);
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

/// Where `text` is drawn in a frame (every place, as screen rectangles in points).
fn find_text(out: &egui::FullOutput, text: &str) -> Vec<Rect> {
    fn walk(shape: &egui::Shape, text: &str, found: &mut Vec<Rect>) {
        match shape {
            egui::Shape::Text(t) if t.galley.text() == text => found.push(t.galley.rect.translate(t.pos.to_vec2())),
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, text, found)),
            _ => {}
        }
    }
    let mut found = Vec::new();
    for c in &out.shapes {
        walk(&c.shape, text, &mut found);
    }
    found
}

/// Where the tool library window is, in points.
fn library_rect(s: &Session) -> Rect {
    s.egui.memory(|m| m.area_rect(egui::Id::unique("recedo tool library"))).expect("the tool library is open")
}

#[test]
fn the_side_bar_and_the_tool_library() {
    let Some(g) = gpu() else { return };
    let mut s = session();
    let mut r = Renderer::new(&g.device, wgpu::TextureFormat::Rgba8Unorm, &s.ed);
    let (out, f) = frames(&g, &mut r, &mut s, vec![]);
    shot(&g, &mut r, &mut s, out, "08a-main-window-tool-strip");
    // The tool strip shows the standard tools as icons, left of the canvas, all on screen.
    let on_strip = |s: &Session, t: Tool| s.ui.strip.iter().find(|(x, _)| *x == Some(t)).map(|(_, r)| *r);
    for t in Tool::DEFAULT_BAR {
        let at = on_strip(&s, t).unwrap_or_else(|| panic!("{} on the strip", t.name()));
        assert!(at.max.x <= f.canvas_rect.min.x && at.max.y <= screen().h as f32, "{} at {at:?}", t.name());
    }
    assert!(on_strip(&s, Tool::Clone).is_none(), "not on the bar yet");
    // A click on an icon picks its tool.
    let fill = on_strip(&s, Tool::Fill).unwrap().center();
    quiet(&g, &mut r, &mut s, click(fill));
    assert_eq!(s.ed.tool, Tool::Fill);
    let open = s.ui.strip.iter().find(|(x, _)| x.is_none()).map(|(_, r)| *r).expect("the All tools button");
    // Open the full list from the strip's button.
    let (out, _) = frames(&g, &mut r, &mut s, click(open.center()));
    assert!(s.ui.tool_library, "the button opens it");
    let opened = library_rect(&s);
    let missing: Vec<&str> = Tool::ALL
        .iter()
        .filter(|t| {
            let label = match t.key() {
                Some(k) => format!("{}  ({k})", t.name()),
                None => t.name().to_string(),
            };
            find_text(&out, &label).is_empty()
        })
        .map(|t| t.name())
        .collect();
    let clone = find_text(&out, "Clone").first().copied();
    // Its own checkbox: on its row, the first one to its right.
    let on_bar = clone.and_then(|c| {
        find_text(&out, "On bar").into_iter().filter(|r| (r.center().y - c.center().y).abs() < 6.0 && r.min.x > c.max.x).min_by(|a, b| a.min.x.total_cmp(&b.min.x))
    });
    shot(&g, &mut r, &mut s, out, "08-tool-library");
    assert!(missing.is_empty(), "every tool is listed without scrolling at 1600 x 900; missing: {missing:?}");
    let (clone, on_bar) = (clone.unwrap(), on_bar.unwrap());
    // Click the tool to use it, then tick "On bar" in its row.
    quiet(&g, &mut r, &mut s, click(clone.center()));
    assert_eq!(s.ed.tool, Tool::Clone);
    quiet(&g, &mut r, &mut s, click(on_bar.center()));
    assert!(s.ui.settings.hotbar_tools().contains(&Tool::Clone), "{:?}", s.ui.settings.hotbar);
    assert_eq!(s.ui.settings.hotbar_tools().len(), Tool::DEFAULT_BAR.len() + 1);
    // The window keeps its width when the bar changes (one more row makes it taller), and
    // its size from frame to frame.
    let ticked = library_rect(&s);
    assert!(ticked.width() == opened.width() && ticked.height() >= opened.height(), "{opened:?} -> {ticked:?}");
    for _ in 0..20 {
        quiet(&g, &mut r, &mut s, vec![]);
    }
    assert_eq!(library_rect(&s), ticked, "the window does not grow by itself");
    // The canvas overlay (here a selection's outline) stays under the window.
    let (out, _) = frames(&g, &mut r, &mut s, vec![]);
    let plain = shot(&g, &mut r, &mut s, out, "08b-tool-library-again");
    s.ed.select_all();
    let (out, _) = frames(&g, &mut r, &mut s, vec![]);
    let selected = shot(&g, &mut r, &mut s, out, "08c-tool-library-with-a-selection");
    let w = ticked.shrink(4.0);
    let changed = (w.min.y as u32..w.max.y as u32).flat_map(|y| (w.min.x as u32..w.max.x as u32).map(move |x| (x, y))).filter(|&(x, y)| plain.pixel(x, y) != selected.pixel(x, y)).count();
    assert_eq!(changed, 0, "pixels of the window changed by the selection's outline");
    s.ed.deselect();
    // Closed again, the strip has it.
    s.ui.tool_library = false;
    s.ed.tool = Tool::Brush;
    let (out, f) = frames(&g, &mut r, &mut s, vec![]);
    assert!(on_strip(&s, Tool::Clone).is_some_and(|r| r.max.x <= f.canvas_rect.min.x), "Clone is on the strip now");
    shot(&g, &mut r, &mut s, out, "09-side-bar-with-clone");
}

#[test]
fn the_text_tool_keeps_the_keyboard_and_is_placed_when_the_tool_changes() {
    let Some(g) = gpu() else { return };
    let mut s = session();
    let mut r = Renderer::new(&g.device, wgpu::TextureFormat::Rgba8Unorm, &s.ed);
    // The text is typed in Tool options, so writing brings that panel back if it was closed.
    s.ui.settings.dock.remove(Panel::ToolOptions);
    s.ed.tool = Tool::Text;
    quiet(&g, &mut r, &mut s, vec![]);
    // A click on the canvas as the shells pass it on: to the editor and to the interface.
    let canvas_click = |s: &mut Session, at: (f64, f64), t: u64| {
        let p = s.ed.view.to_screen(at.0, at.1);
        s.ed.pen(&PenSample::synthetic(t, p.0, p.1, 0.5, true), true, Modifiers::default());
        s.ed.pen(&PenSample::synthetic(t + 8000, p.0, p.1, 0.0, false), true, Modifiers::default());
        click(pos2(p.0 as f32, p.1 as f32))
    };
    let typed = |text: &str| text.chars().map(|c| Event::Text(c.to_string())).collect::<Vec<_>>();
    let ev = canvas_click(&mut s, (100.0, 100.0), 1_000_000);
    quiet(&g, &mut r, &mut s, ev);
    assert!(s.ui.settings.dock.is_open(Panel::ToolOptions));
    quiet(&g, &mut r, &mut s, typed("Hi"));
    assert_eq!(s.ed.text_edit.as_ref().map(|t| t.text.as_str()), Some("Hi"));
    // Another click moves the text, and what is typed next still goes into it (B would
    // otherwise pick the brush).
    let ev = canvas_click(&mut s, (300.0, 200.0), 2_000_000);
    quiet(&g, &mut r, &mut s, ev);
    let mut ev = vec![Event::Key { key: Key::B, physical_key: Some(Key::B), pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }];
    ev.extend(typed("b"));
    quiet(&g, &mut r, &mut s, ev);
    assert_eq!(s.ed.tool, Tool::Text);
    let t = s.ed.text_edit.clone().expect("still writing");
    assert_eq!(t.text, "Hib");
    assert!((t.at.0 - 300.0).abs() < 0.01 && (t.at.1 - 200.0).abs() < 0.01, "moved to {:?}", t.at);
    // Picking another tool places the text where it was shown.
    let steps = s.ed.history.entries().1;
    s.ed.tool = Tool::Brush;
    quiet(&g, &mut r, &mut s, vec![]);
    assert!(s.ed.text_edit.is_none());
    assert_eq!(s.ed.history.entries().1, steps + 1, "one undo step");
    let inked = |s: &Session, x: i32, y: i32| {
        let layer = &s.ed.doc.layers[s.ed.active].pixels;
        (x..x + 100).flat_map(|x| (y..y + 80).map(move |y| (x, y))).filter(|&(x, y)| layer.pixel(x, y)[2] > layer.pixel(x, y)[0] + 60).count()
    };
    let n = inked(&s, 300, 200);
    assert!(n > 200, "{n} pixels of text where it was written");
    // Ctrl+Enter places it too (with no other sign of Ctrl than the key press).
    s.ed.tool = Tool::Text;
    quiet(&g, &mut r, &mut s, vec![]);
    let ev = canvas_click(&mut s, (600.0, 400.0), 3_000_000);
    quiet(&g, &mut r, &mut s, ev);
    quiet(&g, &mut r, &mut s, typed("Ok"));
    let enter = |m: egui::Modifiers| Event::Key { key: Key::Enter, physical_key: Some(Key::Enter), pressed: true, repeat: false, modifiers: m };
    quiet(&g, &mut r, &mut s, vec![enter(egui::Modifiers::NONE)]);
    assert_eq!(s.ed.text_edit.as_ref().map(|t| t.text.as_str()), Some("Ok\n"), "Enter alone starts a new line");
    quiet(&g, &mut r, &mut s, vec![enter(egui::Modifiers::COMMAND)]);
    assert!(s.ed.text_edit.is_none(), "Ctrl+Enter placed it");
    let n = inked(&s, 600, 400);
    assert!(n > 200, "{n} pixels of text");
    // Letters that come with the click starting a text (before its box has the keys) are not
    // shortcuts either (B, I, G: brush, opacity, gradient), and Escape cancels the text.
    let opacity = s.ed.tools.brush.opacity;
    let mut ev = canvas_click(&mut s, (800.0, 500.0), 4_000_000);
    for k in [Key::B, Key::I, Key::G] {
        ev.push(Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: egui::Modifiers::NONE });
    }
    quiet(&g, &mut r, &mut s, ev);
    assert_eq!((s.ed.tool, s.ed.tools.brush.opacity), (Tool::Text, opacity));
    assert!(s.ed.text_edit.is_some());
    let escape = Event::Key { key: Key::Escape, physical_key: Some(Key::Escape), pressed: true, repeat: false, modifiers: egui::Modifiers::NONE };
    quiet(&g, &mut r, &mut s, vec![escape]);
    assert!(s.ed.text_edit.is_none(), "Escape cancelled it");
}

#[test]
fn messages_take_the_width_of_their_text_and_show_once() {
    let Some(g) = gpu() else { return };
    let mut s = session();
    let mut r = Renderer::new(&g.device, wgpu::TextureFormat::Rgba8Unorm, &s.ed);
    let say = |s: &mut Session, text: &str| s.ed.notices.push(crate::editor::Notice { kind: NoticeKind::Info, text: text.into() });
    // A short message first: the next, longer ones are not squeezed to its width.
    say(&mut s, "Undo: Line");
    quiet(&g, &mut r, &mut s, vec![]);
    let long = "Clone source set: now paint where the copy goes.";
    for _ in 0..3 {
        say(&mut s, long);
        say(&mut s, "Nothing to undo");
    }
    let (out, _) = frames(&g, &mut r, &mut s, vec![]);
    let (one_line, again) = (find_text(&out, "Undo: Line"), find_text(&out, long));
    assert_eq!((one_line.len(), again.len()), (1, 1), "each message once");
    assert!(again[0].height() < one_line[0].height() * 1.5, "on one line: {:?} vs {:?}", again[0], one_line[0]);
    assert_eq!(find_text(&out, "Nothing to undo").len(), 1);
    shot(&g, &mut r, &mut s, out, "12-messages");
}

#[test]
fn the_crop_shade_is_even() {
    // The canvas outside the crop rectangle is dimmed evenly: no lighter line where the
    // rectangle's top and bottom edges run out to the canvas edge.
    let Some(g) = gpu() else { return };
    let mut s = session();
    let mut r = Renderer::new(&g.device, wgpu::TextureFormat::Rgba8Unorm, &s.ed);
    s.ed.tool = Tool::Crop;
    s.ed.crop_rect = Some((400, 200, 300, 200));
    let (out, _) = frames(&g, &mut r, &mut s, vec![]);
    let img = shot(&g, &mut r, &mut s, out, "11-crop");
    for edge in [200.0, 400.0] {
        let (x, y) = s.ed.view.to_screen(200.0, edge);
        let shades: Vec<u8> = (-3..=3).map(|d| img.pixel(x as u32, (y + d as f64) as u32)[0]).collect();
        assert!(shades[0] < 160 && shades.iter().all(|&v| v.abs_diff(shades[0]) <= 1), "edge at {edge}: {shades:?}");
    }
}

#[test]
fn every_tool_has_options_and_draws_without_trouble() {
    // Each tool's options render, with and without a selection, and with work in progress
    // (a crop rectangle, text being written, a measurement).
    let Some(g) = gpu() else { return };
    let mut s = session();
    let mut r = Renderer::new(&g.device, wgpu::TextureFormat::Rgba8Unorm, &s.ed);
    for t in Tool::ALL {
        s.ed.tool = t;
        quiet(&g, &mut r, &mut s, vec![]);
    }
    s.ed.select_all();
    s.ed.crop_rect = Some((10, 10, 200, 100));
    s.ed.text_edit = Some(crate::editor::TextEdit { at: (50.0, 50.0), text: "Hello".into() });
    s.ed.measure = Some(((10.0, 10.0), (200.0, 120.0)));
    s.ed.clone_source = Some((300.0, 300.0));
    for t in Tool::ALL {
        s.ed.tool = t;
        let (out, _) = frames(&g, &mut r, &mut s, vec![]);
        if t == Tool::Text {
            assert!(!find_text(&out, "Place text").is_empty(), "the text tool offers to place the text");
        }
        if t == Tool::Crop {
            assert!(!find_text(&out, "Keep 200 × 100 px from 10, 10").is_empty());
        }
        if t == Tool::SelectRect {
            assert!(!find_text(&out, "Copy to new layer").is_empty(), "selection commands with a selection");
            shot(&g, &mut r, &mut s, out, "10-selection-options");
        } else {
            r.skip_frame(&g.device, &g.queue, &mut { out });
        }
    }
}

#[test]
fn the_text_preview_turns_and_mirrors_with_the_view() {
    // The text being written shows where it will be painted, also on a turned or mirrored
    // view: its dark pixels before placing match those of the painted text after.
    let Some(g) = gpu() else { return };
    for mirror in [false, true] {
        let mut s = session();
        let mut r = Renderer::new(&g.device, wgpu::TextureFormat::Rgba8Unorm, &s.ed);
        quiet(&g, &mut r, &mut s, vec![]);
        s.ed.color = [0, 0, 0];
        s.ed.tool = Tool::Text;
        s.ed.tools.text.size = 220.0;
        s.ed.rotate_by_degrees(40.0);
        if mirror {
            s.ed.toggle_mirror();
        }
        let area = quiet(&g, &mut r, &mut s, vec![]).canvas_rect;
        // Pixels of the white painting (not the grey around it) that the text darkens.
        let (out, _) = frames(&g, &mut r, &mut s, vec![]);
        let blank = shot(&g, &mut r, &mut s, out, "16-text-blank");
        let (w, h) = (s.ed.doc.width as f64, s.ed.doc.height as f64);
        s.ed.text_edit = Some(crate::editor::TextEdit { at: (w * 0.3, h * 0.35), text: "RFRFRF".into() });
        let dark = |img: &recedo_core::image::Rgba8| {
            let mut px = Vec::new();
            for y in area.min.y as u32..area.max.y as u32 {
                for x in area.min.x as u32..area.max.x as u32 {
                    if blank.pixel(x, y)[..3].iter().all(|&v| v > 200) && img.pixel(x, y)[..3].iter().all(|&v| v < 80) {
                        px.push((x, y));
                    }
                }
            }
            px
        };
        let (out, _) = frames(&g, &mut r, &mut s, vec![]);
        let before = dark(&shot(&g, &mut r, &mut s, out, if mirror { "16-text-preview-mirrored" } else { "16-text-preview" }));
        s.ed.place_text();
        assert!(s.ed.text_edit.is_none());
        let (out, _) = frames(&g, &mut r, &mut s, vec![]);
        let after = dark(&shot(&g, &mut r, &mut s, out, if mirror { "17-text-placed-mirrored" } else { "17-text-placed" }));
        assert!(before.len() > 2000 && after.len() > 2000, "{} / {} dark pixels", before.len(), after.len());
        let mid = |p: &[(u32, u32)]| {
            let n = p.len() as f64;
            (p.iter().map(|q| q.0 as f64).sum::<f64>() / n, p.iter().map(|q| q.1 as f64).sum::<f64>() / n)
        };
        let (a, b) = (mid(&before), mid(&after));
        let set: std::collections::HashSet<_> = after.iter().copied().collect();
        let both = before.iter().filter(|p| set.contains(p)).count() as f64;
        let overlap = both / (before.len() + after.len()) as f64 * 2.0;
        eprintln!("mirror {mirror}: centres {a:?} {b:?}, overlap {overlap:.2}");
        assert!((a.0 - b.0).hypot(a.1 - b.1) < 6.0, "mirror {mirror}: the preview's middle {a:?}, the painted text's {b:?}");
        assert!(overlap > 0.6, "mirror {mirror}: only {overlap:.2} of the pixels agree");
    }
}

#[test]
fn every_icon_draws_something_of_its_own() {
    // Each tool and action icon paints, and no two look the same (a copy-paste slip would).
    // With RECEDO_SCREENSHOTS set, the sheet is saved for a person to look at.
    let Some(g) = gpu() else { return };
    let mut s = session();
    let mut r = Renderer::new(&g.device, wgpu::TextureFormat::Rgba8Unorm, &s.ed);
    // Twenty icons a row, each at three sizes one above the other.
    let n = Tool::ALL.len() + crate::icons::Action::ALL.len();
    let place = |i: usize, size: usize| {
        let (row, col) = (i / 20, i % 20);
        let side = [20.0, 28.0, 64.0][size];
        Rect::from_min_size(pos2(10.0 + col as f32 * 72.0, 10.0 + row as f32 * 190.0 + [0.0, 40.0, 90.0][size]), vec2(side, side))
    };
    let out = s.egui.run_ui(raw(vec![]), |root| {
        let p = root.painter();
        p.rect_filled(root.max_rect(), 0.0, Color32::from_gray(58));
        for size in 0..3 {
            for (i, t) in Tool::ALL.into_iter().enumerate() {
                crate::icons::tool(p, place(i, size), t, Color32::from_gray(225));
            }
            for (j, a) in crate::icons::Action::ALL.into_iter().enumerate() {
                crate::icons::action(p, place(Tool::ALL.len() + j, size), a, Color32::from_gray(225));
            }
        }
    });
    let img = shot(&g, &mut r, &mut s, out, "18-icons");
    let mut seen: Vec<Vec<bool>> = Vec::new();
    for i in 0..n {
        let rect = place(i, 2);
        let mut ink = Vec::new();
        for y in rect.min.y as u32..rect.max.y as u32 {
            for x in rect.min.x as u32..rect.max.x as u32 {
                ink.push(img.pixel(x, y)[0] > 120);
            }
        }
        let count = ink.iter().filter(|&&b| b).count();
        assert!(count > 80, "icon {i} paints only {count} pixels");
        assert!(!seen.contains(&ink), "icon {i} looks like another one");
        seen.push(ink);
    }
}

#[test]
fn layer_thumbnails_follow_the_painting() {
    // The layers panel's small picture of a layer shows what is painted on it, also when
    // painting stops within the moment thumbnails wait between updates.
    let Some(g) = gpu() else { return };
    let mut s = session();
    let mut r = Renderer::new(&g.device, wgpu::TextureFormat::Rgba8Unorm, &s.ed);
    // A long wait between thumbnail updates, so the frames below are "too soon" whatever
    // the speed of the computer running the test.
    s.ui.previews.every = std::time::Duration::from_secs(30);
    quiet(&g, &mut r, &mut s, vec![]);
    let before = s.ed.pixels_rev;
    // A wide stroke across the middle of Layer 1.
    s.ed.tools.brush.size = 120.0;
    let (a, b) = (s.ed.view.to_screen(100.0, 360.0), s.ed.view.to_screen(1180.0, 360.0));
    for i in 0..=30 {
        let t = i as f64 / 30.0;
        s.ed.pen(&PenSample::synthetic(i * 4000, a.0 + (b.0 - a.0) * t, a.1, 1.0, true), true, Modifiers::default());
    }
    s.ed.pen(&PenSample::synthetic(200_000, b.0, b.1, 0.0, false), true, Modifiers::default());
    assert_ne!(s.ed.pixels_rev, before, "painting moves the pixel revision on");
    // The thumbnails were made a moment ago: this frame is too soon, and asks for one later.
    let (out, _) = frames(&g, &mut r, &mut s, vec![]);
    let later = out.viewport_output.values().next().map(|v| v.repaint_delay);
    r.skip_frame(&g.device, &g.queue, &mut { out });
    assert!(later.is_some_and(|d| d <= std::time::Duration::from_secs(30)), "asks to be drawn again: {later:?}");
    // When that time comes (here: now), the next frame redoes it.
    s.ui.previews.every = std::time::Duration::ZERO;
    let (out, _) = frames(&g, &mut r, &mut s, vec![]);
    let row = find_text(&out, "Layer 1")[0];
    let img = shot(&g, &mut r, &mut s, out, "19-layer-thumbnail");
    // Left of the name, in the picture: the stroke's colour.
    let blue = (row.min.y as u32 - 14..row.max.y as u32 + 6)
        .flat_map(|y| (row.min.x as u32 - 46..row.min.x as u32 - 4).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            let p = img.pixel(x, y);
            p[2] > 120 && p[0] < 110
        })
        .count();
    assert!(blue > 20, "the thumbnail shows the stroke: {blue} blue pixels");
}

#[test]
fn the_lasso_transform_on_screen() {
    // The box with its handles shows over the canvas; another tool puts the part down.
    let Some(g) = gpu() else { return };
    let mut s = session();
    let mut r = Renderer::new(&g.device, wgpu::TextureFormat::Rgba8Unorm, &s.ed);
    quiet(&g, &mut r, &mut s, vec![]);
    let a = s.ed.active;
    for y in 300..420 {
        for x in 300..420 {
            s.ed.doc.layers[a].pixels.set_pixel(x, y, [220, 40, 40, 255]);
        }
    }
    s.ed.tool = Tool::LassoTransform;
    let sc = |s: &Session, x: f64, y: f64| s.ed.view.to_screen(x, y);
    let path: Vec<(f64, f64)> = [(280.0, 280.0), (440.0, 280.0), (440.0, 440.0), (280.0, 440.0), (280.0, 285.0)].iter().map(|&(x, y)| sc(&s, x, y)).collect();
    for (i, &(x, y)) in path.iter().enumerate() {
        s.ed.pen(&PenSample::synthetic(i as u64 * 4000, x, y, 1.0, true), true, Modifiers::default());
    }
    s.ed.pen(&PenSample::synthetic(50_000, path[4].0, path[4].1, 0.0, false), true, Modifiers::default());
    let src = s.ed.transform.as_ref().expect("lifted").src;
    // Drag the bottom-right handle out by half the size again.
    let (from, to) = (sc(&s, src.2, src.3), sc(&s, src.2 + (src.2 - src.0) / 2.0, src.3 + (src.3 - src.1) / 2.0));
    for i in 0..=10 {
        let t = i as f64 / 10.0;
        s.ed.pen(&PenSample::synthetic(100_000 + i * 4000, from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t, 1.0, true), true, Modifiers::default());
    }
    s.ed.pen(&PenSample::synthetic(200_000, to.0, to.1, 0.0, false), true, Modifiers::default());
    let (out, _) = frames(&g, &mut r, &mut s, vec![]);
    assert!(!find_text(&out, "Put down (Enter)").is_empty(), "Tool property offers to put it down");
    let img = shot(&g, &mut r, &mut s, out, "20-lasso-transform");
    // A white handle at the new bottom-right corner, over the canvas.
    let h = s.ed.view.to_screen(s.ed.transform.as_ref().unwrap().rect.2, s.ed.transform.as_ref().unwrap().rect.3);
    assert!(img.pixel(h.0 as u32, h.1 as u32)[..3].iter().all(|&v| v > 230), "handle at {h:?}: {:?}", img.pixel(h.0 as u32, h.1 as u32));
    // The square is bigger on screen: red well past where it ended.
    let past = s.ed.view.to_screen(430.0, 430.0);
    assert!(img.pixel(past.0 as u32, past.1 as u32)[0] > 180 && img.pixel(past.0 as u32, past.1 as u32)[1] < 90);
    // Picking the brush on the strip puts it down: one undo step.
    let brush = s.ui.strip.iter().find(|(t, _)| *t == Some(Tool::Brush)).map(|(_, r)| r.center()).unwrap();
    quiet(&g, &mut r, &mut s, click(brush));
    assert_eq!(s.ed.tool, Tool::Brush);
    assert!(s.ed.transform.is_none());
    assert_eq!(s.ed.history.entries().0, vec!["Transform"]);
}

#[test]
fn a_big_painting_shows_in_a_small_window() {
    // Like the Windows self-test's screen check: a 3840 × 2160 painting fitted into the
    // 1008 × 681 window (zoomed far out, so the smaller mip levels are drawn).
    let Some(g) = gpu() else { return };
    SCREEN.with(|sc| sc.set(Screen { w: 1008, h: 681, scale: 1.0 }));
    let mut s = Session::new(Settings::default(), None);
    let mut r = Renderer::new(&g.device, wgpu::TextureFormat::Rgba8Unorm, &s.ed);
    quiet(&g, &mut r, &mut s, vec![]);
    s.ed.tools.brush.size = 120.0;
    let y = s.ed.doc.height as f64 / 2.0;
    let (a, b) = (s.ed.view.to_screen(800.0, y), s.ed.view.to_screen(3000.0, y));
    for i in 0..=60 {
        let t = i as f64 / 60.0;
        s.ed.pen(&PenSample::synthetic(i * 4000, a.0 + (b.0 - a.0) * t, a.1, 1.0, true), true, Modifiers::default());
    }
    s.ed.pen(&PenSample::synthetic(300_000, b.0, b.1, 0.0, false), true, Modifiers::default());
    let (out, _) = frames(&g, &mut r, &mut s, vec![]);
    let img = shot(&g, &mut r, &mut s, out, "22-big-painting-small-window");
    SCREEN.with(|sc| sc.set(Screen { w: 1600, h: 900, scale: 1.0 }));
    let lum = |(x, y): (f64, f64)| {
        let p = img.pixel(x as u32, y as u32);
        (p[0] as u32 + p[1] as u32 + p[2] as u32) / 3
    };
    let (dark, light) = (lum(s.ed.view.to_screen(1900.0, y)), lum(s.ed.view.to_screen(1900.0, y + 600.0)));
    assert!(dark < 90 && light > 200, "painted spot {dark}, empty spot {light}");
}

#[test]
fn a_small_window_keeps_room_for_the_painting() {
    // The Windows self-test's window: 1008 × 681. The panels give way so the canvas area
    // stays at least 500 × 400 (it was 430 wide once).
    let Some(g) = gpu() else { return };
    SCREEN.with(|sc| sc.set(Screen { w: 1008, h: 681, scale: 1.0 }));
    let mut s = Session::new(Settings::default(), None);
    let mut r = Renderer::new(&g.device, wgpu::TextureFormat::Rgba8Unorm, &s.ed);
    let (out, f) = frames(&g, &mut r, &mut s, vec![]);
    shot(&g, &mut r, &mut s, out, "21-small-window");
    let c = f.canvas_rect;
    SCREEN.with(|sc| sc.set(Screen { w: 1600, h: 900, scale: 1.0 }));
    assert!(c.width() >= 500.0 && c.height() >= 400.0, "canvas area {c:?}");
    // Every tool on the strip, whole.
    let bottom = s.ui.strip.iter().map(|(_, r)| r.max.y).fold(0.0, f32::max);
    assert!(bottom <= c.max.y, "{bottom}");
}
