use std::time::{Duration, Instant};

use recedo_core::canvas::builtin_presets;
use recedo_core::doc::Layer;
use recedo_tablet::{Flags, PenSample, Source};

use super::*;

/// A 400 x 300 white painting with an empty "Layer 1", shown 1:1 at the top-left of an
/// 800 x 600 viewport.
fn editor() -> Editor {
    let mut e = Editor::new(Document::new(400, 300, Some([255, 255, 255, 255])), None);
    e.set_viewport(800.0, 600.0);
    e.view = View::default();
    e.tools.brush = Brush { size: 10.0, hardness: 1.0, size_curve: None, ..Brush::default() };
    e.color = [0, 0, 0];
    e
}

fn pt(t: u64, x: f64, y: f64, contact: bool) -> PenSample {
    PenSample::synthetic(t * 1000, x, y, 1.0, contact)
}

/// A straight stroke from (x0, y) to (x1, y) in screen pixels.
fn stroke(e: &mut Editor, y: f64, x0: f64, x1: f64) {
    let n = 20;
    for i in 0..=n {
        let x = x0 + (x1 - x0) * i as f64 / n as f64;
        e.pen(&pt(i, x, y, true), true, Modifiers::default());
    }
    e.pen(&pt(n + 1, x1, y, false), true, Modifiers::default());
}

fn inked(e: &Editor, x: u32, y: u32) -> bool {
    e.doc.flatten().pixel(x, y)[0] < 128
}

#[test]
fn a_stroke_paints_the_active_layer_and_refreshes_only_its_tiles() {
    let mut e = editor();
    assert_eq!(e.take_refresh(), Refresh::All, "first frame uploads everything");
    stroke(&mut e, 50.0, 20.0, 120.0);
    assert!(inked(&e, 70, 50));
    assert!(!inked(&e, 70, 80));
    assert!(e.doc.layers[0].pixels.tile_count() == 0, "background untouched (default pixel only)");
    assert!(e.doc.layers[1].pixels.tile_count() > 0);
    match e.take_refresh() {
        Refresh::Tiles(t) => assert!(!t.is_empty() && t.len() <= 4, "{t:?}"),
        other => panic!("{other:?}"),
    }
    assert_eq!(e.projection.image, e.doc.flatten());
    assert_eq!(e.history.entries(), (vec!["Brush stroke"], 1));
    assert!(e.has_unsaved_changes());
    // Undo puts the pixels back and refreshes just those tiles.
    e.undo();
    assert!(!inked(&e, 70, 50));
    assert!(matches!(e.take_refresh(), Refresh::Tiles(t) if !t.is_empty()));
    assert_eq!(e.projection.image, e.doc.flatten());
    e.redo();
    assert!(inked(&e, 70, 50));
    e.take_refresh();
    assert_eq!(e.projection.image, e.doc.flatten());
}

#[test]
fn strokes_follow_a_rotated_zoomed_mirrored_view() {
    let mut e = editor();
    e.view = View { offset: (500.0, 100.0), zoom: 2.0, angle: 0.7, mirror: true };
    let target = e.view.to_screen(200.0, 150.0);
    let (a, b) = (e.view.to_screen(180.0, 150.0), e.view.to_screen(220.0, 150.0));
    let n = 20;
    for i in 0..=n {
        let t = i as f64 / n as f64;
        e.pen(&pt(i, a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, true), true, Modifiers::default());
    }
    e.pen(&pt(30, b.0, b.1, false), true, Modifiers::default());
    assert!(inked(&e, 200, 150), "stroke centre at canvas (200, 150) = screen {target:?}");
    assert!(inked(&e, 185, 150) && inked(&e, 215, 150));
    assert!(!inked(&e, 200, 170));
}

#[test]
fn space_pans_shift_space_rotates_ctrl_space_zooms() {
    let mut e = editor();
    let space = Modifiers { space: true, ..Default::default() };
    e.pen(&pt(0, 100.0, 100.0, true), true, space);
    e.pen(&pt(1, 130.0, 90.0, true), true, space);
    e.pen(&pt(2, 130.0, 90.0, false), true, space);
    assert_eq!(e.view.offset, (30.0, -10.0));
    assert!(e.history.entries().0.is_empty(), "panning paints nothing");

    let rot = Modifiers { space: true, shift: true, ..Default::default() };
    // A quarter turn around the viewport centre (400, 300).
    e.pen(&pt(3, 500.0, 300.0, true), true, rot);
    e.pen(&pt(4, 400.0, 400.0, true), true, rot);
    e.pen(&pt(5, 400.0, 400.0, false), true, rot);
    assert!((e.view.angle - std::f64::consts::FRAC_PI_2).abs() < 1e-9, "{}", e.view.angle);

    let zoom = Modifiers { space: true, ctrl: true, ..Default::default() };
    let z0 = e.view.zoom;
    let anchor_canvas = e.view.to_canvas(300.0, 200.0);
    e.pen(&pt(6, 300.0, 200.0, true), true, zoom);
    e.pen(&pt(7, 300.0, 150.0, true), true, zoom);
    e.pen(&pt(8, 300.0, 150.0, false), true, zoom);
    assert!((e.view.zoom / z0 - 1.01f64.powi(50)).abs() < 1e-9, "dragging up 50 px zooms in");
    let after = e.view.to_canvas(300.0, 200.0);
    assert!((after.0 - anchor_canvas.0).abs() < 1e-6 && (after.1 - anchor_canvas.1).abs() < 1e-6, "zoom keeps the anchor");

    e.tool = Tool::Pan;
    let before = e.view.offset;
    e.pen(&pt(9, 10.0, 10.0, true), true, Modifiers::default());
    e.pen(&pt(10, 15.0, 10.0, true), true, Modifiers::default());
    e.pen(&pt(11, 15.0, 10.0, false), true, Modifiers::default());
    assert_eq!(e.view.offset, (before.0 + 5.0, before.1));
}

#[test]
fn rotation_drag_across_the_left_side_does_not_spin() {
    let mut e = editor();
    let rot = Modifiers { space: true, shift: true, ..Default::default() };
    // Just above and just below the left horizontal, where atan2 jumps from +pi to -pi.
    e.pen(&pt(0, 300.0, 301.0, true), true, rot);
    e.pen(&pt(1, 300.0, 299.0, true), true, rot);
    e.pen(&pt(2, 300.0, 299.0, false), true, rot);
    let a = e.view.angle;
    let small = a.min(std::f64::consts::TAU - a);
    assert!(small < 0.05, "turned by {a} rad");
}

#[test]
fn eyedropper_and_ctrl_click_pick_colours() {
    let mut e = editor();
    e.doc.layers[1].pixels.set_pixel(50, 50, [200, 30, 40, 255]);
    e.refresh_all = true;
    e.tool = Tool::Eyedropper;
    e.pen(&pt(0, 50.5, 50.5, true), true, Modifiers::default());
    e.pen(&pt(1, 50.5, 50.5, false), true, Modifiers::default());
    assert_eq!(e.color, [200, 30, 40]);
    // Ctrl-click with the brush picks too, and paints nothing.
    e.tool = Tool::Brush;
    let ctrl = Modifiers { ctrl: true, ..Default::default() };
    e.pen(&pt(2, 10.5, 10.5, true), true, ctrl);
    e.pen(&pt(3, 10.5, 10.5, false), true, ctrl);
    assert_eq!(e.color, [255, 255, 255]);
    assert!(e.history.entries().0.is_empty());
}

#[test]
fn the_eraser_end_erases_with_any_tool() {
    let mut e = editor();
    stroke(&mut e, 50.0, 20.0, 200.0);
    assert!(inked(&e, 100, 50));
    let n = 10;
    for i in 0..=n {
        let mut s = pt(100 + i, 80.0 + 4.0 * i as f64, 50.0, true);
        s.flags.set(Flags::ERASER, true);
        e.pen(&s, true, Modifiers::default());
    }
    let mut up = pt(200, 120.0, 50.0, false);
    up.flags.set(Flags::ERASER, true);
    e.pen(&up, true, Modifiers::default());
    assert!(!inked(&e, 100, 50), "erased back to the white background");
    assert!(inked(&e, 40, 50));
    assert_eq!(e.history.entries().0, vec!["Brush stroke", "Erase"]);
}

#[test]
fn locked_layers_and_panels_refuse_strokes() {
    let mut e = editor();
    e.doc.layers[1].locked = true;
    stroke(&mut e, 50.0, 20.0, 120.0);
    assert!(!inked(&e, 70, 50));
    assert!(e.history.entries().0.is_empty());
    assert_eq!(e.notices.iter().filter(|n| n.text.contains("locked")).count(), 1, "one message per stroke: {:?}", e.notices);
    e.doc.layers[1].locked = false;
    // Pressing over a panel starts nothing; a stroke that leaves the canvas carries on.
    e.pen(&pt(0, 70.0, 50.0, true), false, Modifiers::default());
    e.pen(&pt(1, 90.0, 50.0, true), true, Modifiers::default());
    assert!(!e.stroke_active(), "a press that began on a panel does not paint later");
    e.pen(&pt(2, 90.0, 50.0, false), true, Modifiers::default());
    e.pen(&pt(3, 20.0, 80.0, true), true, Modifiers::default());
    e.pen(&pt(4, 60.0, 80.0, true), false, Modifiers::default());
    e.pen(&pt(5, 60.0, 80.0, false), false, Modifiers::default());
    assert!(inked(&e, 40, 80));
    assert!(!e.stroke_active());
}

#[test]
fn a_second_device_cannot_join_a_stroke() {
    let mut e = editor();
    e.pen(&pt(0, 20.0, 50.0, true), true, Modifiers::default());
    let mut mouse = pt(1, 200.0, 200.0, true);
    mouse.source = Source::Mouse;
    e.pen(&mouse, true, Modifiers::default());
    e.pen(&pt(2, 60.0, 50.0, true), true, Modifiers::default());
    e.pen(&pt(3, 60.0, 50.0, false), true, Modifiers::default());
    assert!(inked(&e, 40, 50));
    assert!(!inked(&e, 200, 200));
    // The pen leaving range ends a stroke it started.
    e.pen(&pt(4, 20.0, 90.0, true), true, Modifiers::default());
    e.pen_gone(Source::Synthetic, 5000);
    assert!(!e.stroke_active());
    assert_eq!(e.history.entries().0.len(), 2);
}

#[test]
fn the_stabilizer_needs_ticks_and_finishes_on_lift() {
    let mut e = editor();
    e.tools.stabilizer = StabilizerSettings { enabled: true, distance: 50.0, ..Default::default() };
    e.pen(&pt(0, 20.0, 50.0, true), true, Modifiers::default());
    e.pen(&pt(1, 300.0, 50.0, true), true, Modifiers::default());
    assert!(e.needs_ticks());
    assert!(!inked(&e, 290, 50), "the brush trails the pen");
    for t in 2..300u64 {
        e.tick(t * 1000, Instant::now());
    }
    assert!(inked(&e, 200, 50), "held still, the brush catches up");
    e.pen(&pt(300, 300.0, 50.0, false), true, Modifiers::default());
    assert!(inked(&e, 299, 50), "finish line");
    assert!(!e.needs_ticks());
}

#[test]
fn tilt_turns_with_the_view() {
    let mut e = editor();
    let mut s = pt(0, 10.0, 10.0, true);
    s.tilt_x = 40.0;
    s.flags.set(Flags::HAS_TILT, true);
    // View turned a quarter clockwise: leaning right on screen is leaning "up" on the canvas.
    e.view.angle = std::f64::consts::FRAC_PI_2;
    let info = e.paint_info(&s);
    assert!(info.tilt_x.abs() < 1e-4 && (info.tilt_y + 40.0).abs() < 1e-4, "{info:?}");
    e.view.angle = 0.0;
    e.view.mirror = true;
    let info = e.paint_info(&s);
    assert!((info.tilt_x + 40.0).abs() < 1e-4);
    // Mouse samples (no pressure) paint at full pressure.
    let mut m = pt(0, 10.0, 10.0, true);
    m.flags.set(Flags::HAS_PRESSURE, false);
    m.pressure = 0.0;
    assert_eq!(e.paint_info(&m).pressure, 1.0);
}

#[test]
fn layer_edits_are_undoable_and_opacity_drags_make_one_step() {
    let mut e = editor();
    e.add_layer();
    assert_eq!(e.doc.layers.len(), 3);
    assert_eq!(e.active, 2);
    assert_eq!(e.doc.layers[2].name, "Layer 2");
    stroke(&mut e, 50.0, 20.0, 120.0);
    for o in [250u8, 200, 150, 100] {
        e.set_layer_opacity(2, o);
    }
    e.end_edit();
    assert_eq!(e.history.entries().0, vec!["New layer", "Brush stroke", "Layer opacity"]);
    e.take_refresh();
    assert_eq!(e.projection.image, e.doc.flatten(), "opacity change redrawn");
    e.set_layer_blend(2, BlendMode::Multiply);
    e.set_layer_visible(2, false);
    e.rename_layer(2, "  Ink  ");
    assert_eq!(e.doc.layers[2].name, "Ink");
    e.take_refresh();
    assert_eq!(e.projection.image, e.doc.flatten());
    e.duplicate_layer();
    e.move_layer(false);
    e.merge_down();
    e.delete_layer();
    e.take_refresh();
    assert_eq!(e.projection.image, e.doc.flatten());
    let steps = e.history.entries().0.len();
    for _ in 0..steps {
        e.undo();
    }
    assert_eq!(e.doc.layers.len(), 2);
    e.take_refresh();
    assert_eq!(e.projection.image, e.doc.flatten());
    // Refused operations leave no undo step and say why.
    e.select_layer(0);
    e.merge_down();
    assert!(e.notices.iter().any(|n| n.kind == NoticeKind::Warning && n.text.contains("below")));
    assert_eq!(e.history.entries().1, 0);
}

#[test]
fn page_keys_select_layers_and_brush_keys_resize() {
    let mut e = editor();
    e.select_neighbour(false);
    assert_eq!(e.active, 0);
    e.select_neighbour(false);
    assert_eq!(e.active, 0, "no layer below the bottom");
    e.select_neighbour(true);
    e.select_neighbour(true);
    assert_eq!(e.active, 1);
    e.tools.brush.size = 10.0;
    e.resize_brush(true);
    assert_eq!(e.tools.brush.size, 11.0);
    for _ in 0..200 {
        e.resize_brush(true);
    }
    assert_eq!(e.tools.brush.size, MAX_BRUSH);
    for _ in 0..200 {
        e.resize_brush(false);
    }
    assert_eq!(e.tools.brush.size, MIN_BRUSH);
    e.toggle_eraser();
    assert_eq!(e.tool, Tool::Eraser);
    e.resize_brush(true);
    assert_eq!(e.tools.eraser.size, 44.0, "the eraser has its own size");
    e.toggle_eraser();
    assert_eq!(e.tool, Tool::Brush);
}

fn temp_dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("recedo-editor-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn save_open_and_export() {
    let dir = temp_dir("files");
    let mut e = editor();
    stroke(&mut e, 50.0, 20.0, 120.0);
    assert!(!e.save(), "no file yet: the shell asks for one");
    e.save_as(&dir.join("cat"));
    e.wait_for_saves();
    assert_eq!(e.file.as_deref(), Some(dir.join("cat.kra").as_path()));
    assert!(!e.has_unsaved_changes());
    assert_eq!(e.title(), "cat.kra");
    e.export(&dir.join("cat.png"), ExportFormat::Png);
    e.export(&dir.join("cat"), ExportFormat::Jpeg(90));
    e.wait_for_saves();
    assert!(dir.join("cat.png").exists() && dir.join("cat.jpg").exists());
    assert!(e.notices.iter().any(|n| n.text.starts_with("Exported")));
    let painted = e.doc.flatten();

    let mut f = editor();
    f.open(&dir.join("cat.kra")).unwrap();
    assert_eq!(f.doc.flatten(), painted);
    assert_eq!(f.file.as_deref(), Some(dir.join("cat.kra").as_path()));
    assert!(!f.has_unsaved_changes());
    assert!(f.history.entries().0.is_empty());
    assert_eq!(f.take_refresh(), Refresh::All);
    // A PNG opens as a new, unsaved painting named after the picture.
    f.open(&dir.join("cat.png")).unwrap();
    assert_eq!(f.doc.flatten(), painted);
    assert_eq!(f.file, None);
    assert_eq!(f.title(), "cat");
    assert!(!f.save());
    assert!(f.open(&dir.join("missing.kra")).is_err());
    // New canvas from a preset.
    f.new_document(&builtin_presets()[1]).unwrap();
    assert_eq!((f.doc.width, f.doc.height), (1280, 720));
    assert_eq!(f.title(), "Untitled 2");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn saving_while_painting_keeps_later_strokes_unsaved() {
    let dir = temp_dir("racing");
    let mut e = editor();
    stroke(&mut e, 50.0, 20.0, 120.0);
    e.save_as(&dir.join("a.kra"));
    std::thread::sleep(Duration::from_millis(5));
    stroke(&mut e, 80.0, 20.0, 120.0);
    e.wait_for_saves();
    assert!(e.has_unsaved_changes(), "the second stroke is not in the file");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn autosave_writes_and_cleans_up_recovery_copies() {
    let dir = temp_dir("recovery");
    let mut e = editor();
    e.recovery_dir = Some(dir.clone());
    e.autosaver = Autosaver::new(Duration::ZERO);
    stroke(&mut e, 50.0, 20.0, 120.0);
    e.tick(0, Instant::now());
    while e.needs_ticks() {
        std::thread::sleep(Duration::from_millis(5));
        e.tick(0, Instant::now());
    }
    let found = autosave::find_recoverable(&dir);
    assert_eq!(found.len(), 1);
    // A new Editor (after a crash) can open it.
    let mut g = editor();
    g.recovery_dir = Some(dir.clone());
    g.open_recovery(&found[0]).unwrap();
    assert_eq!(g.doc.flatten(), e.doc.flatten());
    assert!(g.has_unsaved_changes());
    // Saving the recovered painting removes the copy.
    g.save_as(&dir.join("saved.kra"));
    g.wait_for_saves();
    assert!(autosave::find_recoverable(&dir).is_empty());
    // A normal close removes the original's copy too.
    e.tick(0, Instant::now());
    e.closing();
    assert!(autosave::find_recoverable(&dir).is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn changed_tiles_spots_pixels_and_structure() {
    let mut a = Document::new(200, 200, None);
    a.layers.push(Layer::new("x"));
    let mut b = a.clone();
    assert_eq!(changed_tiles(&a, &b), Some(HashSet::new()));
    b.layers[1].pixels.set_pixel(70, 5, [1, 2, 3, 255]);
    assert_eq!(changed_tiles(&a, &b), Some([(1, 0)].into_iter().collect()));
    b.layers[1].opacity = 3;
    assert_eq!(changed_tiles(&a, &b), None);
    let mut c = a.clone();
    c.layers.swap(0, 1);
    assert_eq!(changed_tiles(&a, &c), None);
}

#[test]
fn view_commands() {
    let mut e = editor();
    e.fit_to_window();
    let fitted = e.view.zoom;
    e.zoom_in();
    assert!(e.view.zoom > fitted);
    e.zoom_actual_pixels();
    assert_eq!(e.view.zoom, 1.0);
    e.rotate_by_degrees(15.0);
    assert!((e.view.angle - 15f64.to_radians()).abs() < 1e-9);
    e.reset_rotation();
    assert!(e.view.angle.abs() < 1e-9);
    e.toggle_mirror();
    assert!(e.view.mirror);
    let c = e.view.to_canvas(400.0, 300.0);
    e.wheel(2.0, (400.0, 300.0));
    assert!((e.view.zoom - 1.44).abs() < 1e-9);
    let c2 = e.view.to_canvas(400.0, 300.0);
    assert!((c.0 - c2.0).abs() < 1e-9 && (c.1 - c2.1).abs() < 1e-9);
    // A new painting is fitted into the viewport once its size is known.
    let mut f = Editor::new(Document::new(4000, 1000, None), None);
    f.set_viewport(1048.0, 600.0);
    assert!((f.view.zoom - 1000.0 / 4000.0).abs() < 1e-9);
}
