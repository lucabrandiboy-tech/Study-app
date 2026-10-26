use std::time::Duration;

use web_time::Instant;

use recedo_core::canvas::builtin_presets;
use recedo_core::doc::Layer;
use recedo_tablet::{Flags, PenSample, Source};

use super::*;

/// A 400 x 300 white painting with an empty "Layer 1", shown 1:1 at the top-left of an
/// 800 x 600 viewport.
fn editor() -> Editor {
    let mut e = Editor::new(Document::new(400, 300, Some([255, 255, 255, 255])), None);
    e.set_viewport(0.0, 0.0, 800.0, 600.0);
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
    // With the fill, line or rectangle tool too: the eraser end erases freehand.
    for tool in [Tool::Fill, Tool::Line, Tool::Rectangle] {
        e.tool = tool;
        assert!(inked(&e, 40, 50));
        for i in 0..=4 {
            let mut s = pt(300 + i, 36.0 + 2.0 * i as f64, 50.0, true);
            s.flags.set(Flags::ERASER, true);
            e.pen(&s, true, Modifiers::default());
        }
        let mut up = pt(310, 44.0, 50.0, false);
        up.flags.set(Flags::ERASER, true);
        e.pen(&up, true, Modifiers::default());
        assert!(!inked(&e, 40, 50), "{tool:?} erased");
        assert_eq!(e.history.entries().0, vec!["Brush stroke", "Erase", "Erase"], "{tool:?}");
        e.undo();
    }
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
fn emergency_autosave_writes_at_once_and_only_when_needed() {
    let dir = temp_dir("emergency");
    let mut e = editor();
    e.recovery_dir = Some(dir.clone());
    // Nothing changed: nothing written.
    e.emergency_autosave();
    assert!(autosave::find_recoverable(&dir).is_empty());
    // A change, with the next autosave still an hour away: written now.
    e.autosaver = Autosaver::new(Duration::from_secs(3600));
    stroke(&mut e, 50.0, 20.0, 120.0);
    e.emergency_autosave();
    let found = autosave::find_recoverable(&dir);
    assert_eq!(found.len(), 1);
    let mut g = editor();
    g.open_recovery(&found[0]).unwrap();
    assert_eq!(g.doc.flatten(), e.doc.flatten());
    // Still unsaved (only the recovery copy was written), and not written twice.
    assert!(e.has_unsaved_changes() && !e.autosaver.copy_is_behind());
    e.closing();
    assert!(autosave::find_recoverable(&dir).is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The screen's picture during a stroke on a middle layer (where the projection keeps the
/// layers below combined) is exactly the flattened painting, every frame, also around a
/// layer edit in the middle (which ends that stroke; the pen, still down, starts another).
#[test]
fn picture_stays_exact_while_painting_between_layers() {
    let mut e = editor();
    // Background, four half-transparent painted layers; paint on the third.
    for i in 0..4u8 {
        let mut l = Layer::new(&format!("p{i}"));
        for y in 0..300 {
            for x in (i as i32..400).step_by(3) {
                l.pixels.set_pixel(x, y, [i * 60, 255 - i * 50, (x % 256) as u8, 100 + i * 30]);
            }
        }
        e.doc.layers.push(l);
    }
    // 0 background, 1 the new document's empty layer, 2..=5 the painted ones.
    assert_eq!(e.doc.layers.len(), 6);
    e.active = 3;
    e.tools.brush.size = 40.0;
    e.take_refresh();
    e.projection.rebuild(&e.doc);
    for i in 0..=30 {
        let x = 20.0 + i as f64 * 12.0;
        e.pen(&pt(i, x, 150.0 + (i as f64 * 0.4).sin() * 60.0, true), true, Modifiers::default());
        if i == 15 {
            // A layer below changes while the pen is down (e.g. its opacity slider).
            e.set_layer_opacity(2, 40);
        }
        e.take_refresh();
        assert!(e.projection.image == e.doc.flatten(), "frame {i}");
    }
    e.pen(&pt(31, 380.0, 150.0, false), true, Modifiers::default());
    e.take_refresh();
    assert!(e.projection.image == e.doc.flatten(), "after the stroke");
    e.undo();
    e.take_refresh();
    assert!(e.projection.image == e.doc.flatten(), "after undo");
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
    f.set_viewport(100.0, 40.0, 1048.0, 600.0);
    assert!((f.view.zoom - 1000.0 / 4000.0).abs() < 1e-9);
    // Centred in the canvas area, which starts at (100, 40).
    let c = f.view.to_screen(2000.0, 500.0);
    assert!((c.0 - 624.0).abs() < 1e-6 && (c.1 - 340.0).abs() < 1e-6, "{c:?}");
}


/// Drags from a to b (screen pixels) with the current tool, Shift as given.
fn drag_tool(e: &mut Editor, a: (f64, f64), b: (f64, f64), shift: bool) {
    let mods = Modifiers { shift, ..Modifiers::default() };
    e.pen(&pt(1, a.0, a.1, true), true, mods);
    for i in 1..=10 {
        let t = i as f64 / 10.0;
        e.pen(&pt(1 + i, a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, true), true, mods);
    }
    e.pen(&pt(20, b.0, b.1, false), true, mods);
}

fn applied(e: &Editor) -> usize {
    e.history.entries().1
}

#[test]
fn fill_tool_fills_inside_a_drawn_outline_in_one_step() {
    let mut e = editor();
    // A closed box drawn with the rectangle tool, then filled inside.
    e.tool = Tool::Rectangle;
    drag_tool(&mut e, (100.0, 100.0), (200.0, 180.0), false);
    assert_eq!(applied(&e), 1, "the rectangle is one step");
    assert_eq!(e.doc.layers[e.active].pixels.pixel(100, 140)[3], 255, "outline painted");
    assert_eq!(e.doc.layers[e.active].pixels.pixel(150, 140)[3], 0, "inside still empty");
    e.take_refresh();
    e.color = [200, 0, 0];
    e.tool = Tool::Fill;
    e.pen(&pt(30, 150.0, 140.0, true), true, Modifiers::default());
    // Dragging the pen while it is down does not fill again.
    e.pen(&pt(31, 300.0, 250.0, true), true, Modifiers::default());
    e.pen(&pt(32, 300.0, 250.0, false), true, Modifiers::default());
    assert_eq!(applied(&e), 2, "the fill is one step");
    let l = &e.doc.layers[e.active].pixels;
    assert_eq!(l.pixel(150, 140), [200, 0, 0, 255], "inside filled");
    assert_eq!(l.pixel(300, 250)[3], 0, "outside the box not filled");
    assert_eq!(e.recent_colors[0], [200, 0, 0]);
    e.undo();
    assert_eq!(e.doc.layers[e.active].pixels.pixel(150, 140)[3], 0, "undo removes the fill");
}

#[test]
fn line_rectangle_ellipse_and_gradient_paint_where_dragged() {
    let mut e = editor();
    e.tool = Tool::Line;
    drag_tool(&mut e, (20.0, 20.0), (120.0, 20.0), false);
    let l = &e.doc.layers[e.active].pixels;
    assert!((20..=120).step_by(10).all(|x| l.pixel(x, 20)[3] == 255), "the whole line");
    assert_eq!(l.pixel(20, 40)[3], 0);
    // Shift: a nearly horizontal drag becomes exactly horizontal.
    drag_tool(&mut e, (20.0, 60.0), (120.0, 64.0), true);
    let l = &e.doc.layers[e.active].pixels;
    assert_eq!(l.pixel(118, 60)[3], 255, "straightened to 0°");
    // A filled ellipse: inside filled, the corner of its box not.
    e.tool = Tool::Ellipse;
    e.tools.shape_filled = true;
    drag_tool(&mut e, (200.0, 100.0), (300.0, 200.0), false);
    let l = &e.doc.layers[e.active].pixels;
    assert_eq!(l.pixel(250, 150)[3], 255);
    assert_eq!(l.pixel(203, 103)[3], 0);
    // Shift makes the ellipse a circle: the box's lower edge moves to make it square.
    drag_tool(&mut e, (20.0, 150.0), (80.0, 170.0), true);
    let l = &e.doc.layers[e.active].pixels;
    assert_eq!(l.pixel(50, 200)[3], 255, "circle reaches y = 210 - its outline at 210");
    let steps = applied(&e);
    assert_eq!(steps, 4);
    // Gradient across the canvas on a new layer: solid before the start, half way in the
    // middle, nothing past the end.
    e.add_layer();
    e.tool = Tool::Gradient;
    e.color = [0, 0, 255];
    drag_tool(&mut e, (50.0, 0.0), (250.0, 0.0), false);
    let l = &e.doc.layers[e.active].pixels;
    assert_eq!(l.pixel(20, 100), [0, 0, 255, 255]);
    let mid = l.pixel(150, 100)[3];
    assert!((120..=135).contains(&mid), "half way: {mid}");
    assert_eq!(l.pixel(260, 100)[3], 0);
    assert_eq!(applied(&e), steps + 2, "the new layer and the gradient");
}

#[test]
fn shapes_and_fill_refuse_a_locked_layer() {
    let mut e = editor();
    let a = e.active;
    e.set_layer_locked(a, true);
    let before = applied(&e);
    e.tool = Tool::Fill;
    e.pen(&pt(1, 50.0, 50.0, true), true, Modifiers::default());
    e.pen(&pt(2, 50.0, 50.0, false), true, Modifiers::default());
    e.tool = Tool::Rectangle;
    drag_tool(&mut e, (10.0, 10.0), (60.0, 60.0), false);
    assert_eq!(applied(&e), before, "nothing changed");
    assert!(e.notices.iter().filter(|n| n.text.contains("locked")).count() >= 2);
}

// ----- the tool library's tools ----------------------------------------------------------

/// Drags from a to b (screen pixels) with the current tool, these keys held.
fn drag_with(e: &mut Editor, a: (f64, f64), b: (f64, f64), mods: Modifiers) {
    e.pen(&pt(1, a.0, a.1, true), true, mods);
    for i in 1..=20 {
        let t = i as f64 / 20.0;
        e.pen(&pt(1 + i, a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, true), true, mods);
    }
    e.pen(&pt(30, b.0, b.1, false), true, mods);
}

fn click(e: &mut Editor, x: f64, y: f64, mods: Modifiers) {
    e.pen(&pt(1, x, y, true), true, mods);
    e.pen(&pt(2, x, y, false), true, mods);
}

fn px(e: &Editor, x: i32, y: i32) -> [u8; 4] {
    e.doc.layers[e.active].pixels.pixel(x, y)
}

fn steps(e: &Editor) -> Vec<String> {
    let (names, applied) = e.history.entries();
    names[..applied].iter().map(|s| s.to_string()).collect()
}

const SHIFT: Modifiers = Modifiers { ctrl: false, shift: true, alt: false, space: false };
const ALT: Modifiers = Modifiers { ctrl: false, shift: false, alt: true, space: false };
const CTRL: Modifiers = Modifiers { ctrl: true, shift: false, alt: false, space: false };

/// Fills a rectangle of the selected layer directly (not as a tool) and shows it.
fn paint_rect(e: &mut Editor, x0: i32, y0: i32, x1: i32, y1: i32, f: impl Fn(i32, i32) -> [u8; 4]) {
    for y in y0..y1 {
        for x in x0..x1 {
            e.doc.layers[e.active].pixels.set_pixel(x, y, f(x, y));
        }
    }
    e.refresh_all = true;
    e.take_refresh();
}

#[test]
fn the_tool_list_is_complete_and_consistent() {
    assert_eq!(Tool::ALL.len(), 31);
    let names: HashSet<&str> = Tool::ALL.iter().map(|t| t.name()).collect();
    assert_eq!(names.len(), Tool::ALL.len(), "names are distinct");
    let keys: Vec<&str> = Tool::ALL.iter().filter_map(|t| t.key()).collect();
    assert_eq!(keys.iter().collect::<HashSet<_>>().len(), keys.len(), "no key used twice: {keys:?}");
    for g in ToolGroup::ALL {
        assert!(Tool::ALL.iter().any(|t| t.group() == g), "{g:?} has tools");
    }
    // Listed group by group, as the library shows them.
    let order: Vec<ToolGroup> = Tool::ALL.iter().map(|t| t.group()).collect();
    let mut seen = Vec::new();
    for g in order {
        if seen.last() != Some(&g) {
            assert!(!seen.contains(&g), "{g:?} is in one piece");
            seen.push(g);
        }
    }
    assert!(Tool::DEFAULT_BAR.iter().all(|t| Tool::ALL.contains(t)));
    // 22 tools more than the 9 there were.
    let old = [Tool::Brush, Tool::Eraser, Tool::Eyedropper, Tool::Pan, Tool::Fill, Tool::Line, Tool::Rectangle, Tool::Ellipse, Tool::Gradient];
    assert_eq!(Tool::ALL.iter().filter(|t| !old.contains(t)).count(), 22);
}

#[test]
fn retouch_tools_change_what_is_under_the_brush_in_one_step() {
    for tool in [Tool::Smudge, Tool::Blur, Tool::Sharpen, Tool::Dodge, Tool::Burn, Tool::Sponge, Tool::Recolour, Tool::Push] {
        let mut e = editor();
        // Red and blue stripes: something for every tool to work on.
        paint_rect(&mut e, 50, 50, 350, 250, |x, _| if x % 8 < 4 { [200, 40, 40, 255] } else { [40, 40, 200, 255] });
        e.tool = tool;
        e.color = [0, 200, 0];
        e.tools.retouch = RetouchBrush { size: 40.0, strength: 1.0, hardness: 0.5, ..RetouchBrush::default() };
        let before = e.doc.layers[e.active].pixels.clone();
        drag_with(&mut e, (100.0, 150.0), (300.0, 150.0), Modifiers::default());
        assert_eq!(steps(&e), vec![tool.name()], "{tool:?}: one undo step");
        let changed = (100..300).filter(|&x| px(&e, x, 150) != before.pixel(x, 150)).count();
        assert!(changed > 100, "{tool:?} changed {changed} of 200 pixels along the stroke");
        assert_eq!(px(&e, 200, 70), before.pixel(200, 70), "{tool:?}: away from the brush nothing changes");
        e.take_refresh();
        assert_eq!(e.projection.image, e.doc.flatten(), "{tool:?}: the screen shows it");
        e.undo();
        assert_eq!(px(&e, 200, 150), before.pixel(200, 150), "{tool:?}: undone");
    }
}

#[test]
fn a_retouch_stroke_stays_on_its_layer_when_another_is_selected_meanwhile() {
    let mut e = editor();
    paint_rect(&mut e, 50, 50, 350, 250, |x, _| if x % 8 < 4 { [200, 40, 40, 255] } else { [40, 40, 200, 255] });
    let first = e.active;
    let painted = e.doc.layers[first].pixels.clone();
    e.add_layer();
    e.select_layer(first);
    e.tool = Tool::Smudge;
    e.tools.retouch = RetouchBrush { size: 40.0, strength: 1.0, ..RetouchBrush::default() };
    e.pen(&pt(1, 100.0, 150.0, true), true, Modifiers::default());
    // Page Up in the middle of the stroke.
    e.select_neighbour(true);
    assert_ne!(e.active, first);
    for i in 1..=20 {
        e.pen(&pt(1 + i, 100.0 + 10.0 * i as f64, 150.0, true), true, Modifiers::default());
    }
    e.pen(&pt(30, 300.0, 150.0, false), true, Modifiers::default());
    assert_eq!(e.doc.layers[first + 1].pixels.tile_count(), 0, "the layer above is untouched");
    let smudged = (150..300).filter(|&x| e.doc.layers[first].pixels.pixel(x, 150) != painted.pixel(x, 150)).count();
    assert!(smudged > 100, "{smudged} pixels smudged on the stroke's own layer");
    e.undo();
    assert_eq!(e.doc.layers[first].pixels.pixel(200, 150), painted.pixel(200, 150));
}

#[test]
fn retouching_nothing_is_no_undo_step_and_locks_and_the_eraser_end_are_kept() {
    let mut e = editor();
    e.tool = Tool::Blur;
    // Blurring an empty layer changes nothing: no step.
    drag_with(&mut e, (100.0, 150.0), (300.0, 150.0), Modifiers::default());
    assert!(steps(&e).is_empty());
    paint_rect(&mut e, 0, 0, 400, 300, |x, _| if x % 6 < 3 { [0, 0, 0, 255] } else { [255, 255, 255, 255] });
    let a = e.active;
    e.set_layer_locked(a, true);
    let n = steps(&e).len();
    drag_with(&mut e, (100.0, 150.0), (300.0, 150.0), Modifiers::default());
    assert_eq!(steps(&e).len(), n, "a locked layer is not retouched");
    assert!(e.notices.iter().any(|m| m.text.contains("locked")));
    e.set_layer_locked(a, false);
    // The pen's eraser end erases, also with a retouch tool.
    let mut s = pt(1, 200.0, 150.0, true);
    s.flags.set(Flags::ERASER, true);
    e.pen(&s, true, Modifiers::default());
    let mut up = pt(2, 200.0, 150.0, false);
    up.flags.set(Flags::ERASER, true);
    e.pen(&up, true, Modifiers::default());
    assert_eq!(px(&e, 200, 150)[3], 0);
    assert_eq!(steps(&e).last().map(String::as_str), Some("Erase"));
}

#[test]
fn clone_copies_from_the_source_set_with_ctrl_click() {
    let mut e = editor();
    paint_rect(&mut e, 20, 20, 80, 80, |x, y| [(x * 3) as u8, (y * 3) as u8, 90, 255]);
    e.tool = Tool::Clone;
    e.tools.retouch = RetouchBrush { size: 20.0, strength: 1.0, hardness: 1.0, ..RetouchBrush::default() };
    // No source yet: nothing happens, and it says what to do.
    drag_with(&mut e, (250.0, 50.0), (270.0, 50.0), Modifiers::default());
    assert!(steps(&e).is_empty() && e.notices.iter().any(|m| m.text.contains("Ctrl+click")));
    click(&mut e, 50.0, 50.0, CTRL);
    assert_eq!(e.clone_source, Some((50.0, 50.0)));
    assert!(steps(&e).is_empty(), "choosing the source changes nothing");
    drag_with(&mut e, (250.0, 50.0), (270.0, 50.0), Modifiers::default());
    for x in 252..268 {
        assert_eq!(px(&e, x, 50), px(&e, x - 200, 50), "x {x}");
    }
    assert_eq!(steps(&e), vec!["Clone"]);
    // The distance stays for the next stroke (aligned clone).
    drag_with(&mut e, (250.0, 60.0), (265.0, 60.0), Modifiers::default());
    assert_eq!(px(&e, 258, 60), px(&e, 58, 60));
}

#[test]
fn spray_scatters_the_colour() {
    let mut e = editor();
    e.tool = Tool::Spray;
    e.color = [10, 120, 250];
    e.tools.retouch = RetouchBrush { size: 40.0, strength: 1.0, spray_density: 15.0, spray_dot: 3.0, ..RetouchBrush::default() };
    drag_with(&mut e, (100.0, 150.0), (300.0, 150.0), Modifiers::default());
    assert_eq!(steps(&e), vec!["Spray"]);
    let painted = (130..170).flat_map(|y| (100..300).map(move |x| (x, y))).filter(|&(x, y)| px(&e, x, y)[3] > 0).count();
    assert!(painted > 300 && painted < 200 * 40 * 9 / 10, "dots, not a solid band: {painted}");
    assert_eq!(e.recent_colors.first(), Some(&[10, 120, 250]));
}

#[test]
fn magic_eraser_and_lasso_fill() {
    let mut e = editor();
    paint_rect(&mut e, 100, 100, 200, 200, |_, _| [220, 0, 0, 255]);
    e.tool = Tool::MagicEraser;
    click(&mut e, 150.0, 150.0, Modifiers::default());
    assert_eq!(steps(&e), vec!["Magic eraser"]);
    assert_eq!(px(&e, 150, 150)[3], 0, "the red square is erased");
    // The background around it was white on another layer: the layer itself is untouched there.
    assert_eq!(e.doc.layers[0].pixels.pixel(150, 150), [255, 255, 255, 255]);
    e.undo();
    assert_eq!(px(&e, 150, 150), [220, 0, 0, 255]);
    // Lasso fill: a triangle drawn freehand is filled with the colour.
    e.tool = Tool::LassoFill;
    e.color = [0, 0, 200];
    let m = Modifiers::default();
    e.pen(&pt(1, 250.0, 50.0, true), true, m);
    for (i, (x, y)) in [(350.0, 50.0), (300.0, 150.0), (252.0, 52.0)].into_iter().enumerate() {
        for k in 1..=10 {
            let (lx, ly) = if i == 0 { (250.0, 50.0) } else { [(350.0, 50.0), (300.0, 150.0)][i - 1] };
            let t = k as f64 / 10.0;
            e.pen(&pt(2 + i as u64 * 10 + k, lx + (x - lx) * t, ly + (y - ly) * t, true), true, m);
        }
    }
    e.pen(&pt(60, 252.0, 52.0, false), true, m);
    assert_eq!(steps(&e), vec!["Lasso fill"]);
    assert_eq!(px(&e, 300, 80), [0, 0, 200, 255], "inside the triangle");
    assert_eq!(px(&e, 260, 140)[3], 0, "outside it");
}

#[test]
fn selections_are_made_combined_and_undone() {
    let mut e = editor();
    e.tool = Tool::SelectRect;
    drag_with(&mut e, (50.0, 50.0), (150.0, 150.0), Modifiers::default());
    assert!(e.has_selection());
    let sel = |e: &Editor, x: i32, y: i32| e.doc.selection.as_ref().map_or(0, |m| m.get(x, y));
    assert_eq!((sel(&e, 100, 100), sel(&e, 200, 100)), (255, 0));
    // Shift adds, Alt takes away (held when the drag starts).
    drag_with(&mut e, (200.0, 50.0), (300.0, 150.0), SHIFT);
    assert_eq!((sel(&e, 100, 100), sel(&e, 250, 100)), (255, 255));
    drag_with(&mut e, (60.0, 60.0), (90.0, 90.0), ALT);
    assert_eq!((sel(&e, 70, 70), sel(&e, 120, 120)), (0, 255));
    // Ellipse select replaces it.
    e.tool = Tool::SelectEllipse;
    drag_with(&mut e, (100.0, 100.0), (300.0, 200.0), Modifiers::default());
    assert_eq!((sel(&e, 200, 150), sel(&e, 105, 105), sel(&e, 250, 100)), (255, 0, 0));
    e.invert_selection();
    assert_eq!((sel(&e, 200, 150), sel(&e, 5, 5)), (0, 255));
    e.select_all();
    assert_eq!(sel(&e, 399, 299), 255);
    e.deselect();
    assert!(!e.has_selection());
    assert_eq!(steps(&e), vec!["Select", "Select", "Select", "Select", "Invert selection", "Select all", "Deselect"]);
    // Undo walks back through them.
    e.undo();
    assert_eq!(sel(&e, 399, 299), 255);
    e.undo();
    e.undo();
    assert_eq!(sel(&e, 200, 150), 255, "the ellipse again");
    // A click without dragging deselects.
    e.tool = Tool::SelectRect;
    click(&mut e, 10.0, 10.0, Modifiers::default());
    assert!(!e.has_selection());
    // The outline the screen draws follows the selection.
    e.tool = Tool::SelectRect;
    drag_with(&mut e, (50.0, 50.0), (150.0, 150.0), Modifiers::default());
    e.update_outline();
    let segs = e.selection_outline().and_then(|o| o.clone()).unwrap();
    assert_eq!(segs.len(), 4);
}

#[test]
fn lasso_select_and_magic_wand() {
    let mut e = editor();
    paint_rect(&mut e, 100, 100, 200, 200, |_, _| [220, 0, 0, 255]);
    e.tool = Tool::MagicWand;
    click(&mut e, 150.0, 150.0, Modifiers::default());
    let m = e.doc.selection.clone().unwrap();
    assert_eq!(m.bounds, (100, 100, 200, 200), "exactly the red square");
    click(&mut e, 10.0, 10.0, SHIFT);
    assert_eq!(e.doc.selection.as_ref().unwrap().bounds, (0, 0, 400, 300), "and the white around it");
    e.tool = Tool::SelectLasso;
    let m = Modifiers::default();
    let path = [(20.0, 20.0), (120.0, 20.0), (120.0, 120.0), (20.0, 120.0), (21.0, 21.0)];
    e.pen(&pt(1, path[0].0, path[0].1, true), true, m);
    for (i, w) in path.windows(2).enumerate() {
        for k in 1..=10 {
            let t = k as f64 / 10.0;
            e.pen(&pt(2 + i as u64 * 10 + k, w[0].0 + (w[1].0 - w[0].0) * t, w[0].1 + (w[1].1 - w[0].1) * t, true), true, m);
        }
    }
    e.pen(&pt(60, 21.0, 21.0, false), true, m);
    let sel = e.doc.selection.clone().unwrap();
    assert_eq!((sel.get(70, 70), sel.get(150, 150)), (255, 0));
}

#[test]
fn painting_filling_and_clearing_stay_inside_the_selection() {
    let mut e = editor();
    e.tool = Tool::SelectRect;
    drag_with(&mut e, (100.0, 0.0), (200.0, 300.0), Modifiers::default());
    // A brush stroke across the whole canvas paints only inside.
    e.tool = Tool::Brush;
    stroke(&mut e, 150.0, 20.0, 380.0);
    assert!(inked(&e, 150, 150) && !inked(&e, 50, 150) && !inked(&e, 300, 150));
    e.take_refresh();
    assert_eq!(e.projection.image, e.doc.flatten());
    // The fill finds the whole white area but fills only the selected part of it.
    e.tool = Tool::Fill;
    e.color = [0, 150, 0];
    click(&mut e, 120.0, 40.0, Modifiers::default());
    assert_eq!(px(&e, 120, 40), [0, 150, 0, 255]);
    assert_eq!(px(&e, 50, 40)[3], 0);
    // Delete clears the selected pixels; Backspace fills them.
    e.clear_layer();
    assert_eq!(px(&e, 120, 40)[3], 0);
    e.color = [10, 10, 200];
    e.fill_selection();
    assert_eq!((px(&e, 150, 10), px(&e, 50, 10)[3]), ([10, 10, 200, 255], 0));
    // Retouch tools too.
    e.tool = Tool::Burn;
    e.tools.retouch = RetouchBrush { size: 40.0, strength: 1.0, ..RetouchBrush::default() };
    let outside = e.doc.layers[0].pixels.pixel(50, 100);
    drag_with(&mut e, (20.0, 100.0), (380.0, 100.0), Modifiers::default());
    assert!(px(&e, 150, 100)[2] < 200, "burnt inside");
    assert_eq!(e.doc.layers[0].pixels.pixel(50, 100), outside);
    assert_eq!(px(&e, 50, 100)[3], 0, "nothing outside");
    assert_eq!(steps(&e), vec!["Select", "Brush stroke", "Fill", "Clear selection", "Fill with colour", "Burn"]);
}

#[test]
fn move_tool_moves_the_layer_or_the_selected_pixels() {
    let mut e = editor();
    paint_rect(&mut e, 100, 100, 120, 120, |_, _| [200, 0, 0, 255]);
    paint_rect(&mut e, 300, 100, 320, 120, |_, _| [0, 0, 200, 255]);
    e.tool = Tool::Move;
    drag_with(&mut e, (150.0, 150.0), (180.0, 170.0), Modifiers::default());
    assert_eq!(steps(&e), vec!["Move"]);
    assert_eq!((px(&e, 135, 125), px(&e, 105, 105)[3]), ([200, 0, 0, 255], 0), "the whole layer moved by (30, 20)");
    assert_eq!(px(&e, 335, 125), [0, 0, 200, 255]);
    e.undo();
    // With a selection only the selected pixels move, and the selection with them.
    e.tool = Tool::SelectRect;
    drag_with(&mut e, (90.0, 90.0), (130.0, 130.0), Modifiers::default());
    e.tool = Tool::Move;
    drag_with(&mut e, (110.0, 110.0), (110.0, 160.0), SHIFT);
    assert_eq!(px(&e, 110, 160), [200, 0, 0, 255], "moved down 50");
    assert_eq!(px(&e, 110, 110)[3], 0);
    assert_eq!(px(&e, 310, 110), [0, 0, 200, 255], "the rest stays");
    assert_eq!(e.doc.selection.as_ref().unwrap().bounds, (90, 140, 130, 180));
    e.undo();
    assert_eq!((px(&e, 110, 110), e.doc.selection.as_ref().unwrap().bounds), ([200, 0, 0, 255], (90, 90, 130, 130)), "undo puts both back");
    // Arrow keys nudge.
    e.nudge(1, 0);
    assert_eq!(px(&e, 120, 110), [200, 0, 0, 255]);
}

#[test]
fn crop_tool_and_crop_to_selection() {
    let mut e = editor();
    paint_rect(&mut e, 60, 50, 70, 60, |_, _| [0, 0, 0, 255]);
    e.tool = Tool::Crop;
    drag_with(&mut e, (50.0, 40.0), (250.0, 140.0), Modifiers::default());
    assert_eq!(e.crop_rect, Some((50, 40, 200, 100)));
    assert!(steps(&e).is_empty(), "nothing is cropped until it is confirmed");
    e.apply_crop();
    assert_eq!((e.doc.width, e.doc.height), (200, 100));
    assert_eq!(px(&e, 10, 10), [0, 0, 0, 255], "(60, 50) is now (10, 10)");
    assert_eq!(e.take_refresh(), Refresh::All, "a new canvas size: everything is uploaded again");
    assert_eq!(e.projection.image.width, 200);
    e.undo();
    assert_eq!((e.doc.width, e.doc.height), (400, 300));
    // (A new canvas size is fitted to the window again; back to 1:1 for the screen points.)
    e.view = View::default();
    // Escape-style cancel, and cropping to a selection.
    drag_with(&mut e, (10.0, 10.0), (20.0, 20.0), Modifiers::default());
    e.cancel_crop();
    assert_eq!(e.crop_rect, None);
    e.tool = Tool::SelectRect;
    drag_with(&mut e, (30.0, 30.0), (130.0, 80.0), Modifiers::default());
    e.crop_to_selection();
    assert_eq!((e.doc.width, e.doc.height, e.has_selection()), (100, 50, false));
}

#[test]
fn text_tool_places_text_in_one_step() {
    let mut e = editor();
    e.tool = Tool::Text;
    e.color = [200, 0, 100];
    e.tools.text.size = 40.0;
    click(&mut e, 20.0, 20.0, Modifiers::default());
    assert_eq!(e.text_edit.as_ref().map(|t| t.at), Some((20.0, 20.0)));
    // Clicking again moves it; placing empty text does nothing.
    click(&mut e, 30.0, 30.0, Modifiers::default());
    assert_eq!(e.text_edit.as_ref().map(|t| t.at), Some((30.0, 30.0)));
    e.text_edit.as_mut().unwrap().text = "Hello\n".into();
    e.place_text();
    assert_eq!(steps(&e), vec!["Text"]);
    assert!(e.text_edit.is_none());
    let inked: Vec<(i32, i32)> = (0..300).flat_map(|y| (0..400).map(move |x| (x, y))).filter(|&(x, y)| px(&e, x, y)[3] > 200).collect();
    assert!(inked.len() > 200, "{} pixels of text", inked.len());
    assert!(inked.iter().all(|&(x, y)| (30..160).contains(&x) && (30..90).contains(&y)), "where it was placed");
    assert_eq!(px(&e, inked[0].0, inked[0].1)[0..3], [200, 0, 100]);
    click(&mut e, 50.0, 200.0, Modifiers::default());
    e.place_text();
    assert_eq!(steps(&e).len(), 1, "no text, no step");
}

#[test]
fn zoom_rotate_and_measure_tools_change_no_pixels() {
    let mut e = editor();
    e.tool = Tool::Zoom;
    click(&mut e, 200.0, 150.0, Modifiers::default());
    let zoomed = e.view.zoom;
    assert!(zoomed > 1.0);
    let (cx, cy) = e.view.to_canvas(200.0, 150.0);
    assert!((cx - 200.0).abs() < 1e-6 && (cy - 150.0).abs() < 1e-6, "about the point clicked");
    click(&mut e, 200.0, 150.0, ALT);
    assert!(e.view.zoom < zoomed);
    let z = e.view.zoom;
    drag_with(&mut e, (200.0, 150.0), (200.0, 100.0), Modifiers::default());
    assert!(e.view.zoom > z * 1.5, "dragging up zooms in");
    e.tool = Tool::Rotate;
    drag_with(&mut e, (500.0, 300.0), (400.0, 400.0), Modifiers::default());
    assert!(e.view.angle.abs() > 0.1);
    e.tool = Tool::Measure;
    e.view = View::default();
    drag_with(&mut e, (10.0, 10.0), (40.0, 50.0), Modifiers::default());
    assert_eq!(e.measure, Some(((10.0, 10.0), (40.0, 50.0))));
    assert!(steps(&e).is_empty());
}

#[test]
fn selection_to_a_new_layer() {
    let mut e = editor();
    paint_rect(&mut e, 0, 0, 400, 300, |_, _| [0, 120, 0, 255]);
    e.tool = Tool::SelectRect;
    drag_with(&mut e, (10.0, 10.0), (60.0, 60.0), Modifiers::default());
    let layers = e.doc.layers.len();
    e.selection_to_layer(false);
    assert_eq!(e.doc.layers.len(), layers + 1);
    assert_eq!((px(&e, 30, 30), px(&e, 100, 100)[3]), ([0, 120, 0, 255], 0), "only the selected part");
    assert_eq!(e.doc.layers[e.active - 1].pixels.pixel(30, 30), [0, 120, 0, 255], "copied: still below");
    e.select_layer(e.active - 1);
    e.selection_to_layer(true);
    assert_eq!(e.doc.layers[e.active - 1].pixels.pixel(30, 30)[3], 0, "cut: gone below");
    assert_eq!(steps(&e), vec!["Select", "Copy to new layer", "Cut to new layer"]);
}
