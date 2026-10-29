//! `recedo.exe --selftest <folder>`: drives the real app with injected input and checks the
//! painting, the screen and the files that come out. Used by CI on Windows and under Wine;
//! needs no tablet.
//!
//! * Mouse and keyboard: `SendInput`, through the same message handling as a real mouse.
//! * Windows Ink: a pen stroke injected with `InjectSyntheticPointerInput` (Windows 10
//!   1809+), with pressure. Skipped where the system cannot inject pens (Wine).
//! * Wintab: only if `RECEDO_WINTAB_DLL` points at the mock driver (`wintab_mock.dll`).
//!
//! Each check prints PASS/FAIL/SKIP; the exit code is 1 if anything failed. The settings
//! file is never written (the user's own settings stay as they are), and autosaves go to a
//! folder inside the output folder.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{GetLastError, HWND, POINT};
use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
use windows_sys::Win32::UI::Controls::{HSYNTHETICPOINTERDEVICE, POINTER_FEEDBACK_MODE, POINTER_FEEDBACK_NONE, POINTER_TYPE_INFO};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
    MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_WHEEL, MOUSEINPUT, SendInput, VIRTUAL_KEY, VK_CONTROL, VK_DELETE, VK_ESCAPE, VK_INSERT, VK_OEM_4, VK_OEM_6,
    VK_RETURN, VK_RIGHT, VK_SHIFT, VK_SPACE,
};
use windows_sys::Win32::UI::Input::Pointer::{POINTER_FLAG_DOWN, POINTER_FLAG_INCONTACT, POINTER_FLAG_INRANGE, POINTER_FLAG_UP, POINTER_FLAG_UPDATE};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::app::{App, client_size};
use super::{create_window, install, install_crash_report, set_dpi_awareness, shutdown, step, with_app};
use crate::editor::{ExportFormat, Tool};
use crate::settings::{Settings, TabletApi};

#[derive(Default)]
struct Results {
    text: String,
    failed: u32,
    passed: u32,
    skipped: u32,
}

impl Results {
    fn check(&mut self, name: &str, ok: bool, detail: impl AsRef<str>) {
        let tag = if ok { "PASS" } else { "FAIL" };
        if ok { self.passed += 1 } else { self.failed += 1 }
        let _ = writeln!(self.text, "{tag}  {name}: {}", detail.as_ref());
        println!("{tag}  {name}: {}", detail.as_ref());
    }

    fn skip(&mut self, name: &str, why: impl AsRef<str>) {
        self.skipped += 1;
        let _ = writeln!(self.text, "SKIP  {name}: {}", why.as_ref());
        println!("SKIP  {name}: {}", why.as_ref());
    }

    fn note(&mut self, line: impl AsRef<str>) {
        let _ = writeln!(self.text, "      {}", line.as_ref());
        println!("      {}", line.as_ref());
    }
}

// ----- driving the app --------------------------------------------------------------------

/// Runs the loop for a while.
fn run_for(d: Duration) {
    let end = Instant::now() + d;
    while Instant::now() < end {
        if !step(false) {
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Runs the loop until `done` (or the time is up); returns whether it happened.
fn run_until(limit: Duration, mut done: impl FnMut(&mut App) -> bool) -> bool {
    let end = Instant::now() + limit;
    while Instant::now() < end {
        if with_app(&mut done).unwrap_or(false) {
            return true;
        }
        if !step(false) {
            return false;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    with_app(done).unwrap_or(false)
}

fn force_frame() {
    with_app(|a| a.request_frame());
    step(false);
}

fn screen_of(hwnd: HWND, x: f64, y: f64) -> POINT {
    let mut p = POINT { x: x.round() as i32, y: y.round() as i32 };
    // SAFETY: valid window.
    unsafe { ClientToScreen(hwnd, &mut p) };
    p
}

/// Is the whole client area on the screen?
fn on_screen_whole(hwnd: HWND, w: u32, h: u32) -> bool {
    let (tl, br) = (screen_of(hwnd, 0.0, 0.0), screen_of(hwnd, w as f64 - 1.0, h as f64 - 1.0));
    // SAFETY: plain calls.
    let (sx, sy, sw, sh) =
        unsafe { (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN), GetSystemMetrics(SM_CXVIRTUALSCREEN), GetSystemMetrics(SM_CYVIRTUALSCREEN)) };
    tl.x >= sx && tl.y >= sy && br.x < sx + sw && br.y < sy + sh
}

fn send(input: INPUT) -> u32 {
    // SAFETY: one valid INPUT.
    unsafe { SendInput(1, &input, std::mem::size_of::<INPUT>() as i32) }
}

fn mouse(flags: u32, data: i32) -> u32 {
    send(INPUT { r#type: INPUT_MOUSE, Anonymous: INPUT_0 { mi: MOUSEINPUT { dx: 0, dy: 0, mouseData: data as u32, dwFlags: flags, time: 0, dwExtraInfo: 0 } } })
}

fn key(vk: VIRTUAL_KEY, up: bool) -> u32 {
    let flags = if up { KEYEVENTF_KEYUP } else { 0 };
    send(INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: flags, time: 0, dwExtraInfo: 0 } } })
}

/// Presses keys in order, then releases them in reverse, and lets the app handle them.
fn keys(vks: &[VIRTUAL_KEY]) {
    for &vk in vks {
        key(vk, false);
        run_for(Duration::from_millis(15));
    }
    for &vk in vks.iter().rev() {
        key(vk, true);
        run_for(Duration::from_millis(15));
    }
    run_for(Duration::from_millis(100));
}

fn move_to(hwnd: HWND, p: (f64, f64)) {
    let s = screen_of(hwnd, p.0, p.1);
    // SAFETY: plain call.
    unsafe { SetCursorPos(s.x, s.y) };
}

#[derive(Clone, Copy, PartialEq)]
enum Button {
    Left,
    Middle,
}

/// A mouse drag in client pixels, in `steps` moves.
fn drag(hwnd: HWND, from: (f64, f64), to: (f64, f64), button: Button, steps: u32) {
    let (down, up) = match button {
        Button::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
        Button::Middle => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
    };
    move_to(hwnd, from);
    run_for(Duration::from_millis(40));
    mouse(down, 0);
    run_for(Duration::from_millis(30));
    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        move_to(hwnd, (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t));
        run_for(Duration::from_millis(8));
    }
    mouse(up, 0);
    run_for(Duration::from_millis(150));
    force_frame();
}

// ----- looking at the result --------------------------------------------------------------

/// The canvas area in client pixels: left, top, right, bottom.
#[derive(Clone, Copy, Debug)]
struct Area {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

impl Area {
    fn width(&self) -> f64 {
        self.right - self.left
    }

    fn height(&self) -> f64 {
        self.bottom - self.top
    }
}

fn canvas_area() -> Area {
    with_app(|a| {
        let r = a.session.canvas_rect;
        let ppp = a.session.egui.pixels_per_point() as f64;
        Area { left: r.min.x as f64 * ppp, top: r.min.y as f64 * ppp, right: r.max.x as f64 * ppp, bottom: r.max.y as f64 * ppp }
    })
    .unwrap_or(Area { left: 0.0, top: 0.0, right: 0.0, bottom: 0.0 })
}

/// Undo steps applied.
fn history_applied() -> usize {
    with_app(|a| a.session.ed.history.entries().1).unwrap_or(0)
}

/// Is there paint on layer `layer` within `r` canvas pixels of the canvas point under the
/// screen point?
fn painted_at(a: &App, layer: usize, screen: (f64, f64), r: i32) -> bool {
    let ed = &a.session.ed;
    let Some(l) = ed.doc.layers.get(layer) else { return false };
    let (cx, cy) = ed.view.to_canvas(screen.0, screen.1);
    let (cx, cy) = (cx.floor() as i32, cy.floor() as i32);
    (-r..=r).any(|dy| (-r..=r).any(|dx| l.pixels.pixel(cx + dx, cy + dy)[3] > 0))
}

/// Painted canvas pixels on `layer` in the canvas column under screen x, between screen rows
/// y0 and y1 (the view is not rotated in these tests).
fn column_ink(a: &App, layer: usize, x: f64, y0: f64, y1: f64) -> u32 {
    let ed = &a.session.ed;
    let Some(l) = ed.doc.layers.get(layer) else { return 0 };
    let (cx, c0) = ed.view.to_canvas(x, y0);
    let (_, c1) = ed.view.to_canvas(x, y1);
    (c0.floor() as i32..=c1.ceil() as i32).filter(|&y| l.pixels.pixel(cx.floor() as i32, y)[3] > 0).count() as u32
}

/// Share (0..=1) of evenly spaced points along a screen line that have paint near them.
fn coverage(layer: usize, from: (f64, f64), to: (f64, f64), r: i32) -> f64 {
    with_app(|a| {
        let n = 40;
        let hits = (0..=n)
            .filter(|&i| {
                let t = i as f64 / n as f64;
                painted_at(a, layer, (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t), r)
            })
            .count();
        hits as f64 / (n + 1) as f64
    })
    .unwrap_or(0.0)
}

fn active_layer() -> usize {
    with_app(|a| a.session.ed.active).unwrap_or(0)
}

// ----- the tests --------------------------------------------------------------------------

fn mouse_tests(r: &mut Results, hwnd: HWND, c: Area) {
    let layer = active_layer();
    let y = c.top + 70.0;
    let (from, to) = ((c.left + 60.0, y), (c.right - 60.0, y));
    let before = history_applied();
    drag(hwnd, from, to, Button::Left, 60);
    let after = history_applied();
    r.check("mouse: a stroke is one undo step", after == before + 1, format!("undo steps {before} -> {after}"));
    let cov = coverage(layer, (from.0 + 10.0, y), (to.0 - 10.0, y), 2);
    r.check("mouse: the line is painted where the mouse went", cov >= 0.97, format!("{:.0} % of points along it painted", cov * 100.0));

    // A press on the status bar that slides onto the canvas never paints.
    let status = (c.left + 300.0, c.bottom + 10.0);
    let over = with_app(|a| a.session.over_canvas(status.0, status.1)).unwrap_or(true);
    let target = (c.left + 300.0, c.top + 160.0);
    drag(hwnd, status, target, Button::Left, 20);
    let after2 = history_applied();
    let dirty = with_app(|a| painted_at(a, layer, target, 3)).unwrap_or(true);
    r.check("mouse: a press outside the canvas does not paint", !over && after2 == after && !dirty, format!("undo steps {after} -> {after2}, paint at the release point: {dirty}"));

    // Ctrl+Z undoes the stroke, Ctrl+Shift+Z brings it back.
    let mid = (c.left + c.width() / 2.0, y);
    keys(&[VK_CONTROL, b'Z' as u16]);
    let (undone, gone) = (history_applied(), !with_app(|a| painted_at(a, layer, mid, 2)).unwrap_or(true));
    r.check("keyboard: Ctrl+Z undoes the stroke", undone + 1 == after && gone, format!("undo steps {after} -> {undone}, stroke gone: {gone}"));
    keys(&[VK_CONTROL, VK_SHIFT, b'Z' as u16]);
    let (redone, back) = (history_applied(), with_app(|a| painted_at(a, layer, mid, 2)).unwrap_or(false));
    r.check("keyboard: Ctrl+Shift+Z redoes it", redone == after && back, format!("undo steps {undone} -> {redone}, stroke back: {back}"));
}

/// Where a canvas point is on screen.
fn on_screen(p: (f64, f64)) -> (f64, f64) {
    with_app(|a| a.session.ed.view.to_screen(p.0, p.1)).unwrap_or((0.0, 0.0))
}

fn view_tests(r: &mut Results, hwnd: HWND, c: Area) {
    let start = (c.left + c.width() * 0.4, c.top + c.height() * 0.5);
    let anchor = with_app(|a| a.session.ed.view.to_canvas(start.0, start.1)).unwrap_or((0.0, 0.0));
    let before = history_applied();

    // Space + drag pans. Not under Wine: it releases keys held with SendInput when the mouse
    // is pressed (it re-reads the keyboard from the X server, which never saw them).
    if let Some(v) = recedo_win::sys::wine_version() {
        r.skip("view: Space + drag pans", format!("Wine {v} releases keys held with SendInput at the next mouse press; checked on Windows"));
    } else {
        space_pan(r, hwnd, start, anchor, before);
    }
    let before = history_applied();
    let moved = on_screen(anchor);

    // The middle mouse button pans too.
    let from = moved;
    drag(hwnd, from, (from.0 - 100.0, from.1 - 40.0), Button::Middle, 15);
    let moved2 = on_screen(anchor);
    let ok = (moved2.0 - from.0 + 100.0).abs() < 2.0 && (moved2.1 - from.1 + 40.0).abs() < 2.0 && history_applied() == before;
    r.check("view: middle-button drag pans", ok, format!("moved by ({:.1}, {:.1}), expected (-100, -40)", moved2.0 - from.0, moved2.1 - from.1));

    // The wheel zooms about the pointer.
    let z0 = with_app(|a| a.session.ed.view.zoom).unwrap_or(1.0);
    let at = moved2;
    move_to(hwnd, at);
    run_for(Duration::from_millis(40));
    mouse(MOUSEEVENTF_WHEEL, 120);
    run_for(Duration::from_millis(100));
    let z1 = with_app(|a| a.session.ed.view.zoom).unwrap_or(1.0);
    let stays = on_screen(anchor);
    let ok = (z1 / z0 - 1.2).abs() < 0.01 && (stays.0 - at.0).abs() < 1.5 && (stays.1 - at.1).abs() < 1.5;
    r.check("view: wheel zooms in about the pointer", ok, format!("zoom x{:.3}; the point under the pointer moved {:.2} px", z1 / z0, (stays.0 - at.0).hypot(stays.1 - at.1)));

    // "2" fits the painting to the window again.
    keys(&[b'2' as u16]);
    let (z2, fit) = with_app(|a| {
        let z = a.session.ed.view.zoom;
        a.session.ed.fit_to_window();
        (z, a.session.ed.view.zoom)
    })
    .unwrap_or((0.0, 1.0));
    r.check("keyboard: 2 fits the painting to the window", (z2 - fit).abs() < 1e-9, format!("zoom {:.4}, fit {:.4}", z2, fit));
}

fn space_pan(r: &mut Results, hwnd: HWND, start: (f64, f64), anchor: (f64, f64), before: usize) {
    key(VK_SPACE, false);
    run_for(Duration::from_millis(40));
    let held = with_app(|a| a.space_held).unwrap_or(false);
    drag(hwnd, start, (start.0 + 150.0, start.1 + 60.0), Button::Left, 20);
    key(VK_SPACE, true);
    run_for(Duration::from_millis(50));
    let moved = on_screen(anchor);
    let after = history_applied();
    let ok = (moved.0 - start.0 - 150.0).abs() < 2.0 && (moved.1 - start.1 - 60.0).abs() < 2.0 && after == before;
    r.check(
        "view: Space + drag pans (and does not paint)",
        ok,
        format!("the point under the cursor moved by ({:.1}, {:.1}), expected (150, 60); undo steps {before} -> {after}; Space seen held: {held}", moved.0 - start.0, moved.1 - start.1),
    );
}

fn layer_tests(r: &mut Results, hwnd: HWND, c: Area) {
    let (n0, a0) = with_app(|a| (a.session.ed.doc.layers.len(), a.session.ed.active)).unwrap_or((0, 0));
    keys(&[VK_INSERT]);
    let (n1, a1) = with_app(|a| (a.session.ed.doc.layers.len(), a.session.ed.active)).unwrap_or((0, 0));
    r.check("layers: Insert adds a layer above and selects it", n1 == n0 + 1 && a1 == a0 + 1, format!("{n0} -> {n1} layers, selected {a0} -> {a1}"));
    let y = c.top + c.height() * 0.8;
    let (from, to) = ((c.left + 80.0, y), (c.left + c.width() * 0.6, y));
    drag(hwnd, from, to, Button::Left, 30);
    let mid = (c.left + c.width() * 0.35, y);
    let (on_new, on_old) = with_app(|a| (painted_at(a, a1, mid, 2), painted_at(a, a0, mid, 2))).unwrap_or((false, true));
    r.check("layers: painting goes to the selected layer only", on_new && !on_old, format!("new layer painted: {on_new}, layer below painted: {on_old}"));
}

/// Painted pixels on `layer` in a canvas rectangle.
fn ink_in(a: &App, layer: usize, (x0, y0): (i32, i32), (x1, y1): (i32, i32)) -> usize {
    let Some(l) = a.session.ed.doc.layers.get(layer) else { return 0 };
    (y0..y1).map(|y| (x0..x1).filter(|&x| l.pixels.pixel(x, y)[3] > 0).count()).sum()
}

/// The first painted canvas column on row `y` from `x0` on, if any within `len`.
fn first_ink(a: &App, layer: usize, x0: i32, y: i32, len: i32) -> Option<i32> {
    let l = a.session.ed.doc.layers.get(layer)?;
    (x0..x0 + len).find(|&x| l.pixels.pixel(x, y)[3] > 0)
}

/// The new tools with real keys and mouse drags: picking tools by key, a selection and
/// Delete, smudging, cropping with Enter, moving with the arrow keys, typing text, and the
/// brush size keys.
fn tool_tests(r: &mut Results, hwnd: HWND, c: Area) {
    let layer = active_layer();
    let tool = || with_app(|a| a.session.ed.tool).unwrap_or(Tool::Brush);
    let canvas = |p: (f64, f64)| with_app(|a| a.session.ed.view.to_canvas(p.0, p.1)).unwrap_or((0.0, 0.0));
    let doc_size = || with_app(|a| (a.session.ed.doc.width, a.session.ed.doc.height)).unwrap_or((0, 0));
    // A line to work on, painted with the brush.
    keys(&[b'B' as u16]);
    let y = c.top + c.height() * 0.55;
    let (x0, x1) = (c.left + 100.0, c.right - 100.0);
    drag(hwnd, (x0, y), (x1, y), Button::Left, 40);
    let third = (x1 - x0) / 3.0;
    let (left, mid, right) = ((x0 + third * 0.5, y), (x0 + third * 1.5, y), (x0 + third * 2.5, y));

    // R opens the radial menu round the pointer; Escape closes it.
    move_to(hwnd, (x0 + third, y - 40.0));
    keys(&[b'R' as u16]);
    let opened = with_app(|a| a.session.ui.radial.is_some()).unwrap_or(false);
    keys(&[VK_ESCAPE]);
    let closed = with_app(|a| a.session.ui.radial.is_none()).unwrap_or(false);
    r.check("keyboard: R opens the radial menu, Escape closes it", opened && closed, format!("opened: {opened}, closed: {closed}"));

    // Shift+R picks rectangle select; Delete clears inside the selection only.
    keys(&[VK_SHIFT, b'R' as u16]);
    let picked = tool();
    drag(hwnd, (x0 + third, y - 40.0), (x0 + third * 2.0, y + 40.0), Button::Left, 10);
    let selected = with_app(|a| a.session.ed.has_selection()).unwrap_or(false);
    keys(&[VK_DELETE]);
    let (l, m, rt) = with_app(|a| (painted_at(a, layer, left, 2), painted_at(a, layer, mid, 2), painted_at(a, layer, right, 2))).unwrap_or_default();
    r.check(
        "tools: Shift+R selects a rectangle, Delete clears inside it only",
        picked == Tool::SelectRect && selected && l && !m && rt,
        format!("tool {picked:?}, selected: {selected}; paint left / middle / right of the line: {l} / {m} / {rt}"),
    );
    keys(&[VK_CONTROL, b'Z' as u16]);
    let back = with_app(|a| painted_at(a, layer, mid, 2)).unwrap_or(false);
    keys(&[VK_CONTROL, b'D' as u16]);
    let kept = with_app(|a| a.session.ed.has_selection()).unwrap_or(true);
    r.check("tools: Ctrl+Z brings it back, Ctrl+D deselects", back && !kept, format!("middle painted again: {back}, still selected: {kept}"));

    // S picks smudge: a stroke across the line drags its paint along, as one undo step.
    keys(&[b'S' as u16]);
    let picked = tool();
    let (cx, cy) = canvas(left);
    // Just below the line (the cursor only goes to whole screen pixels, so the line can be
    // a screen pixel off).
    let (zoom, size) = with_app(|a| (a.session.ed.view.zoom, a.session.ed.tools.brush.size)).unwrap_or((1.0, 20.0));
    let edge = (size as f64 / 2.0 + 3.0 + 1.0 / zoom).ceil() as i32;
    let below = || with_app(|a| ink_in(a, layer, (cx as i32 - 15, cy as i32 + edge), (cx as i32 + 15, cy as i32 + edge + 40)));
    let (before, steps) = (below().unwrap_or(1), history_applied());
    drag(hwnd, (left.0, y - 25.0), (left.0, y + 25.0), Button::Left, 25);
    let (after, steps2) = (below().unwrap_or(0), history_applied());
    r.check(
        "tools: S smudges paint along the stroke, one undo step",
        picked == Tool::Smudge && before == 0 && after > 50 && steps2 == steps + 1,
        format!("tool {picked:?}; painted pixels just below the line {before} -> {after}; undo steps {steps} -> {steps2}"),
    );

    // C picks crop: Enter crops to the rectangle dragged, Ctrl+Z gives the whole painting back.
    keys(&[b'C' as u16]);
    let picked = tool();
    let size0 = doc_size();
    drag(hwnd, (x0 + 50.0, y - 100.0), (x0 + 350.0, y + 100.0), Button::Left, 10);
    let rect = with_app(|a| a.session.ed.crop_rect).flatten();
    keys(&[VK_RETURN]);
    let size1 = doc_size();
    keys(&[VK_CONTROL, b'Z' as u16]);
    let size2 = doc_size();
    r.check(
        "tools: C and Enter crop the painting; Ctrl+Z undoes it",
        picked == Tool::Crop && rect.is_some_and(|(_, _, w, h)| (w, h) == size1 && w > 500) && size2 == size0,
        format!("tool {picked:?}, crop rectangle {rect:?}; size {size0:?} -> {size1:?} -> {size2:?}"),
    );
    keys(&[b'2' as u16]);

    // The Move tool and the arrow keys: Right moves the layer 1 px, Shift+Right 10 px.
    with_app(|a| a.session.ed.tool = Tool::Move);
    let (lx, ly) = canvas((x0 - 20.0, y));
    let start = || with_app(|a| first_ink(a, layer, lx as i32, ly as i32, 400)).flatten();
    let s0 = start();
    keys(&[VK_RIGHT]);
    let s1 = start();
    keys(&[VK_SHIFT, VK_RIGHT]);
    let s2 = start();
    keys(&[VK_CONTROL, b'Z' as u16]);
    keys(&[VK_CONTROL, b'Z' as u16]);
    let s3 = start();
    r.check(
        "tools: arrow keys move the layer with the Move tool (Shift: 10 px)",
        s0.is_some() && s1 == s0.map(|x| x + 1) && s2 == s0.map(|x| x + 11) && s3 == s0,
        format!("the line starts at x {s0:?}, then {s1:?}, {s2:?}; after two undos {s3:?}"),
    );

    // T picks text: a click, then typing; Ctrl+Enter places it.
    keys(&[b'T' as u16]);
    let picked = tool();
    let at = (right.0, y + 60.0);
    drag(hwnd, at, at, Button::Left, 1);
    keys(&[b'H' as u16]);
    keys(&[b'I' as u16]);
    let typed = with_app(|a| a.session.ed.text_edit.as_ref().map(|t| t.text.clone())).flatten();
    let steps = history_applied();
    keys(&[VK_CONTROL, VK_RETURN]);
    let (placed, steps2) = (with_app(|a| a.session.ed.text_edit.is_none()).unwrap_or(false), history_applied());
    let (tx, ty) = canvas(at);
    let ink = with_app(|a| ink_in(a, layer, (tx as i32, ty as i32), (tx as i32 + 200, ty as i32 + 120))).unwrap_or(0);
    r.check(
        "tools: T, a click and typing write text; Ctrl+Enter places it",
        picked == Tool::Text && typed.as_deref() == Some("hi") && placed && steps2 == steps + 1 && ink > 300,
        format!("tool {picked:?}, typed {typed:?}, placed: {placed}, undo steps {steps} -> {steps2}, {ink} pixels of text"),
    );

    // ] and [ make the brush bigger and smaller again.
    keys(&[b'B' as u16]);
    let size = || with_app(|a| a.session.ed.tools.brush.size).unwrap_or(0.0);
    let z0 = size();
    keys(&[VK_OEM_6]);
    let z1 = size();
    keys(&[VK_OEM_4]);
    let z2 = size();
    r.check("keyboard: ] and [ change the brush size", z1 > z0 && z2 < z1, format!("{z0} -> {z1} -> {z2} px"));
}

struct InjectedFrame {
    x: f64,
    y: f64,
    pressure: f32,
    flags: u32,
}

type FnCreateDevice = unsafe extern "system" fn(POINTER_INPUT_TYPE, u32, POINTER_FEEDBACK_MODE) -> HSYNTHETICPOINTERDEVICE;
type FnInject = unsafe extern "system" fn(HSYNTHETICPOINTERDEVICE, *const POINTER_TYPE_INFO, u32) -> i32;
type FnDestroyDevice = unsafe extern "system" fn(HSYNTHETICPOINTERDEVICE);

/// Thinnest and thickest painted run along a stroke from x0 to x1 (screen), within `band`
/// screen pixels of row y.
fn widths(layer: usize, x0: f64, x1: f64, y: f64, band: f64) -> (u32, u32) {
    with_app(|a| {
        let near_start = column_ink(a, layer, x0 + 25.0, y - band, y + band);
        let near_end = column_ink(a, layer, x1 - 25.0, y - band, y + band);
        (near_start, near_end)
    })
    .unwrap_or((0, 0))
}

fn ink_test(r: &mut Results, hwnd: HWND, c: Area) {
    use recedo_tablet::ink::user32_proc;
    with_app(|a| a.set_tablet_api(TabletApi::Ink));
    type Raw = unsafe extern "system" fn() -> isize;
    let (Some(create), Some(inject), Some(destroy)) =
        (user32_proc(c"CreateSyntheticPointerDevice"), user32_proc(c"InjectSyntheticPointerInput"), user32_proc(c"DestroySyntheticPointerDevice"))
    else {
        r.skip("windows ink", "this system has no pen injection API (needs Windows 10 1809 or later)");
        return;
    };
    // SAFETY: documented signatures of the three functions.
    let (create, inject, destroy) = unsafe {
        (
            std::mem::transmute::<Raw, FnCreateDevice>(create),
            std::mem::transmute::<Raw, FnInject>(inject),
            std::mem::transmute::<Raw, FnDestroyDevice>(destroy),
        )
    };
    // SAFETY: plain call; returns null if pen injection is unsupported.
    let dev = unsafe { create(PT_PEN, 1, POINTER_FEEDBACK_NONE) };
    if dev.is_null() {
        // SAFETY: plain call.
        let err = unsafe { GetLastError() };
        r.skip("windows ink", format!("this system cannot inject pen input (CreateSyntheticPointerDevice failed, error {err})"));
        return;
    }
    let layer = active_layer();
    let before = history_applied();
    with_app(|a| a.pen_log = Some(crate::win::app::PenLog::new()));
    // Whole pixels: the pen goes to whole screen pixels, so the line is measured where it
    // really went (zoomed far out, half a pixel is several canvas pixels).
    let y = (c.top + c.height() * 0.3).round();
    let (x0, x1) = ((c.left + 60.0).round(), (c.right - 60.0).round());
    let hover = POINTER_FLAG_INRANGE | POINTER_FLAG_UPDATE;
    let mut frames = Vec::new();
    for i in 0..10 {
        frames.push(InjectedFrame { x: x0 - 30.0 + i as f64 * 3.0, y: y - 20.0, pressure: 0.0, flags: hover });
    }
    let n = 120;
    for i in 0..=n {
        let t = i as f64 / n as f64;
        let flags = POINTER_FLAG_INRANGE | POINTER_FLAG_INCONTACT | if i == 0 { POINTER_FLAG_DOWN } else { POINTER_FLAG_UPDATE };
        frames.push(InjectedFrame { x: x0 + (x1 - x0) * t, y, pressure: 0.1 + 0.9 * t as f32, flags });
    }
    frames.push(InjectedFrame { x: x1, y, pressure: 0.0, flags: POINTER_FLAG_INRANGE | POINTER_FLAG_UP });
    frames.push(InjectedFrame { x: x1 + 5.0, y: y - 10.0, pressure: 0.0, flags: hover });
    frames.push(InjectedFrame { x: x1 + 5.0, y: y - 10.0, pressure: 0.0, flags: POINTER_FLAG_UPDATE });
    let mut ok = 0;
    let mut first_error = 0;
    for (i, f) in frames.iter().enumerate() {
        // SAFETY: zeroed POD.
        let mut info: POINTER_TYPE_INFO = unsafe { std::mem::zeroed() };
        info.r#type = PT_PEN;
        // SAFETY: writing the pen variant of the union, which `type` selects.
        unsafe {
            let pen = &mut info.Anonymous.penInfo;
            pen.pointerInfo.pointerType = PT_PEN;
            pen.pointerInfo.ptPixelLocation = screen_of(hwnd, f.x, f.y);
            pen.pointerInfo.pointerFlags = f.flags;
            pen.penMask = PEN_MASK_PRESSURE;
            pen.pressure = (f.pressure * 1024.0).round() as u32;
        }
        // SAFETY: valid device and one frame.
        if unsafe { inject(dev, &info, 1) } != 0 {
            ok += 1;
        } else if first_error == 0 {
            // SAFETY: plain call.
            first_error = unsafe { GetLastError() };
        }
        if i % 4 == 3 {
            run_for(Duration::from_millis(16));
        } else {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    run_for(Duration::from_millis(300));
    force_frame();
    // SAFETY: device from CreateSyntheticPointerDevice.
    unsafe { destroy(dev) };
    if ok == 0 {
        r.skip("windows ink", format!("InjectSyntheticPointerInput refused every frame (error {first_error})"));
        return;
    }
    let after = history_applied();
    let log = with_app(|a| a.pen_log.take()).flatten();
    r.check("ink: a pen stroke is one undo step", after == before + 1, format!("undo steps {before} -> {after} ({ok} of {} frames injected)", frames.len()));
    let cov = coverage(layer, (x0 + 10.0, y), (x1 - 10.0, y), 2);
    r.check("ink: the line is painted where the pen went", cov >= 0.97, format!("{:.0} % of points along it painted", cov * 100.0));
    if cov < 0.97 {
        // Which points along the line have no paint near them, and where the paint is.
        let missed = with_app(|a| {
            let ed = &a.session.ed;
            (0..=40)
                .map(|i| (x0 + 10.0 + (x1 - x0 - 20.0) * i as f64 / 40.0, y))
                .filter(|&p| !painted_at(a, layer, p, 2))
                .map(|p| {
                    let (cx, cy) = ed.view.to_canvas(p.0, p.1);
                    let rows: Vec<i32> = (-60..=60).filter(|d| ed.doc.layers[layer].pixels.pixel(cx as i32, cy as i32 + d)[3] > 0).collect();
                    format!("screen ({:.1}, {:.1}) = canvas ({cx:.1}, {cy:.1}): paint in that column at rows {:?}..{:?} from it", p.0, p.1, rows.first(), rows.last())
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
        for m in missed {
            r.note(format!("  no paint near {m}"));
        }
    }
    if after != before + 1 || cov < 0.97 {
        // What the app received and where the stroke started and ended, to see why.
        r.note("pen trace (frame ids; c = touching, h = hovering):");
        for line in log.map(|l| l.lines).unwrap_or_default() {
            r.note(format!("  {line}"));
        }
    }
    let (thin, thick) = widths(layer, x0, x1, y, 40.0);
    r.check("ink: pressure changes the line width", thin > 0 && thick >= thin * 3, format!("{thin} canvas px wide near the start, {thick} near the end"));
}

type PlayFn = unsafe extern "system" fn(i32, i32, i32, i32) -> i32;

fn wintab_test(r: &mut Results, hwnd: HWND, c: Area) {
    if std::env::var(recedo_tablet::wintab::DLL_OVERRIDE_ENV).is_err() {
        r.skip("wintab", "RECEDO_WINTAB_DLL is not set (point it at wintab_mock.dll to test Wintab without a tablet)");
        return;
    }
    with_app(|a| a.set_tablet_api(TabletApi::Wintab));
    run_for(Duration::from_millis(100));
    let play: Option<PlayFn> = with_app(|a| {
        a.wintab.as_ref().and_then(|w| w.ctx.proc_address(c"wtmock_play")).map(|f| {
            // SAFETY: the mock exports this signature.
            unsafe { std::mem::transmute::<unsafe extern "system" fn() -> isize, PlayFn>(f) }
        })
    })
    .flatten();
    let Some(play) = play else {
        let status = with_app(|a| a.session.ui.tablet_status.clone()).unwrap_or_default();
        r.check("wintab: mock driver opened", false, status);
        return;
    };
    let status = with_app(|a| a.session.ui.tablet_status.clone()).unwrap_or_default();
    r.check("wintab: mock driver opened", true, status);
    let layer = active_layer();
    let before = history_applied();
    // The script draws within this rectangle (client pixels).
    let (x0, y0, x1, y1) = (c.left + 40.0, c.top + c.height() * 0.4, c.right - 40.0, c.top + c.height() * 0.7);
    let tl = screen_of(hwnd, x0, y0);
    let br = screen_of(hwnd, x1, y1);
    // SAFETY: mock export.
    let sent = unsafe { play(tl.x, tl.y, br.x - tl.x, br.y - tl.y) };
    run_for(Duration::from_millis(1800));
    force_frame();
    let after = history_applied();
    r.check("wintab: a pen stroke is one undo step", after == before + 1, format!("undo steps {before} -> {after} (mock sent {sent} packets)"));
    // Every point the pen touched (in client pixels) should have paint at it.
    let script = recedo_tablet::mock_script::stroke(tl.x as f64, tl.y as f64, (br.x - tl.x) as f64, (br.y - tl.y) as f64);
    let touched: Vec<(f64, f64)> = script.iter().filter(|s| s.tip).map(|s| (s.sx - tl.x as f64 + x0, s.sy - tl.y as f64 + y0)).collect();
    let hits = with_app(|a| touched.iter().filter(|&&p| painted_at(a, layer, p, 2)).count()).unwrap_or(0);
    r.check("wintab: paint lies where the pen touched", hits * 100 >= touched.len() * 97, format!("{hits} of {} touched points painted", touched.len()));
    // Pressure rises along the stroke, so the line gets wider.
    let (first, last) = (touched[5], touched[touched.len() - 6]);
    let (thin, thick) = with_app(|a| (column_ink(a, layer, first.0, first.1 - 40.0, first.1 + 40.0), column_ink(a, layer, last.0, last.1 - 40.0, last.1 + 40.0))).unwrap_or((0, 0));
    r.check("wintab: pressure changes the line width", thin > 0 && thick >= thin * 3, format!("{thin} canvas px wide near the start, {thick} near the end"));
    // While the pen is near, the mouse messages the driver makes must not paint a second line.
    let extra = history_applied() - after;
    r.check("wintab: no duplicate stroke from the driver's mouse messages", extra == 0, format!("{extra} extra undo steps"));
    with_app(|a| a.set_tablet_api(TabletApi::Ink));
}

/// The screen shows the painting: a painted spot is dark, an empty spot is white.
fn screen_test(r: &mut Results, out: &Path, capture: bool) {
    if !capture {
        r.skip("screen", "this GPU does not let the window's image be read back");
        return;
    }
    // A painted pixel and an empty one, from the document.
    let spots = with_app(|a| {
        let ed = &a.session.ed;
        let flat = ed.doc.flatten();
        let mut dark = None;
        let mut light = None;
        'search: for y in (0..flat.height).step_by(7) {
            for x in (0..flat.width).step_by(7) {
                let p = flat.pixel(x, y);
                let lum = (p[0] as u32 + p[1] as u32 + p[2] as u32) / 3;
                let s = ed.view.to_screen(x as f64 + 0.5, y as f64 + 0.5);
                let inside = {
                    let (vx, vy, vw, vh) = ed.viewport;
                    s.0 > vx + 4.0 && s.1 > vy + 4.0 && s.0 < vx + vw - 4.0 && s.1 < vy + vh - 4.0
                };
                // Well inside a stroke, or well away from all paint (by 3 screen pixels) and
                // inside the painting, so neither filtering nor the painting's edge can blur it.
                let around = |want_dark: bool| {
                    let reach = if want_dark { 6 } else { (3.0 / ed.view.zoom).ceil().max(6.0) as i32 };
                    let step = (reach / 3).max(1) as usize;
                    (-reach..=reach).step_by(step).all(|d| {
                        (-reach..=reach).step_by(step).all(|e| {
                            let (qx, qy) = (x as i32 + d, y as i32 + e);
                            if qx < 0 || qy < 0 || qx >= flat.width as i32 || qy >= flat.height as i32 {
                                return false;
                            }
                            let q = flat.pixel(qx as u32, qy as u32);
                            let l = (q[0] as u32 + q[1] as u32 + q[2] as u32) / 3;
                            if want_dark { l < 60 } else { l > 250 }
                        })
                    })
                };
                if inside && dark.is_none() && lum < 60 && around(true) {
                    dark = Some(s);
                }
                if inside && light.is_none() && lum > 250 && around(false) {
                    light = Some(s);
                }
                if dark.is_some() && light.is_some() {
                    break 'search;
                }
            }
        }
        (dark, light)
    });
    let Some((Some(dark), Some(light))) = spots else {
        r.check("screen", false, "no clearly painted and clearly empty spots to compare");
        return;
    };
    with_app(|a| a.capture_next = true);
    force_frame();
    match with_app(|a| a.captured.take()).flatten() {
        Some(Ok(img)) => {
            let lum = |(x, y): (f64, f64)| {
                let p = img.pixel((x as u32).min(img.width - 1), (y as u32).min(img.height - 1));
                (p[0] as u32 + p[1] as u32 + p[2] as u32) / 3
            };
            let (d, l) = (lum(dark), lum(light));
            r.check("screen: the window shows the painting", d < 90 && l > 200, format!("painted spot brightness {d} (want < 90), empty spot {l} (want > 200)"));
            if d >= 90 || l <= 200 {
                let (view, viewport) = with_app(|a| (format!("{:?}", a.session.ed.view), a.session.ed.viewport)).unwrap_or_default();
                let at = |p: (f64, f64)| img.pixel((p.0 as u32).min(img.width - 1), (p.1 as u32).min(img.height - 1));
                let (vx, vy, vw, vh) = viewport;
                let centre = (vx + vw / 2.0, vy + vh / 2.0);
                r.note(format!(
                    "  image {}x{}; viewport {viewport:?}; painted spot at {dark:?} is {:?}, empty spot at {light:?} is {:?}, viewport centre {:?}; view {view}",
                    img.width,
                    img.height,
                    at(dark),
                    at(light),
                    at(centre)
                ));
            }
            let path = out.join("selftest-screen.png");
            if let Ok(bytes) = recedo_core::image::encode_png(&img, true) {
                let _ = std::fs::write(&path, bytes);
                r.note(format!("screen saved to {}", path.display()));
            }
        }
        Some(Err(e)) => r.check("screen: the window shows the painting", false, e),
        None => r.check("screen: the window shows the painting", false, "no frame was captured"),
    }
}

fn file_tests(r: &mut Results, out: &Path) {
    // A name with non-English letters, as people's folders and files often have.
    let kra = out.join("selftest Übung.kra");
    let _ = std::fs::remove_file(&kra);
    // Waits right here, so the messages the save leaves are read before the interface takes them.
    let notes = with_app(|a| {
        a.save_as(kra.clone());
        a.session.ed.wait_for_saves();
        a.session.ed.notices.iter().map(|n| n.text.clone()).collect::<Vec<_>>()
    })
    .unwrap_or_default();
    let (title, unsaved) = with_app(|a| (a.session.ed.title(), a.session.ed.has_unsaved_changes())).unwrap_or_default();
    r.check("file: Save as finishes", title == "selftest Übung.kra" && !unsaved, format!("title \"{title}\", unsaved changes: {unsaved}, messages {notes:?}"));
    let flat = with_app(|a| a.session.ed.doc.flatten());
    match (recedo_core::kra::load_file(&kra), flat) {
        (Ok(l), Some(flat)) => {
            let same = l.doc.flatten() == flat;
            let size = std::fs::metadata(&kra).map_or(0, |m| m.len());
            r.check("file: the .kra reads back identical", same && l.warnings.is_empty(), format!("{size} bytes, {} layers, identical: {same}, warnings {:?}", l.doc.layers.len(), l.warnings));
        }
        (Err(e), _) => r.check("file: the .kra reads back identical", false, e),
        (_, None) => r.check("file: the .kra reads back identical", false, "app gone"),
    }

    let png = out.join("selftest-export.png");
    let _ = std::fs::remove_file(&png);
    with_app(|a| a.export(png.clone(), ExportFormat::Png));
    run_until(Duration::from_secs(60), |a| !a.session.ed.is_busy_saving());
    let flat = with_app(|a| a.session.ed.doc.flatten());
    match (std::fs::read(&png).map_err(|e| e.to_string()).and_then(|b| recedo_core::image::decode_png(&b)), flat) {
        (Ok(img), Some(flat)) => r.check("file: PNG export matches the painting", img == flat, format!("{}x{}", img.width, img.height)),
        (Err(e), _) => r.check("file: PNG export matches the painting", false, e),
        (_, None) => r.check("file: PNG export matches the painting", false, "app gone"),
    }

    // Opening the saved file again (through the same path a dropped file takes).
    with_app(|a| {
        a.session.ed.add_layer();
    });
    // Unsaved changes now: opening asks first, and Escape (Cancel) keeps the painting.
    with_app(|a| a.open_path(kra.clone()));
    run_for(Duration::from_millis(200));
    let asked = with_app(|a| a.session.ui.dialog_open()).unwrap_or(false);
    keys(&[VK_ESCAPE]);
    let (closed, layers) = with_app(|a| (!a.session.ui.dialog_open(), a.session.ed.doc.layers.len())).unwrap_or((false, 0));
    r.check("file: opening with unsaved changes asks first; Escape cancels", asked && closed, format!("asked: {asked}, dialog closed by Escape: {closed}, {layers} layers kept"));
    with_app(|a| a.session.ed.undo());
}

fn recovery_tests(r: &mut Results, hwnd: HWND, c: Area, dir: &Path) {
    // Paint something new so there are unsaved changes, then let an autosave run now.
    let y = c.top + c.height() * 0.9;
    drag(hwnd, (c.left + 100.0, y), (c.left + 300.0, y), Button::Left, 15);
    with_app(|a| {
        a.session.ed.autosaver.interval = Duration::ZERO;
        a.session.ed.tick(recedo_tablet::clock::now_us(), Instant::now());
    });
    run_until(Duration::from_secs(60), |a| !a.session.ed.needs_ticks());
    let found = recedo_core::autosave::find_recoverable(dir);
    r.check("autosave: a recovery copy is written", found.len() == 1, format!("{} copies in {}", found.len(), dir.display()));

    // Closing with unsaved changes asks first; Escape cancels and the app stays open.
    // SAFETY: valid window.
    unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
    run_for(Duration::from_millis(200));
    let (asked, closing) = with_app(|a| (a.session.ui.dialog_open(), a.close_requested)).unwrap_or((false, true));
    keys(&[VK_ESCAPE]);
    let (open, closing2) = with_app(|a| (!a.session.ui.dialog_open(), a.close_requested)).unwrap_or((false, true));
    r.check("close: unsaved changes are asked about; Escape keeps the app open", asked && !closing && open && !closing2, format!("asked: {asked}, closed: {}", closing || closing2));

    // A normal close (changes discarded or saved) removes the recovery copy.
    with_app(|a| a.closing());
    let left = recedo_core::autosave::find_recoverable(dir).len();
    r.check("close: a normal close removes the recovery copy", left == 0, format!("{left} copies left"));
}

pub fn run(out: Option<PathBuf>) -> i32 {
    install_crash_report(false);
    set_dpi_awareness();
    let out = out.unwrap_or_else(|| PathBuf::from("selftest-output"));
    let _ = std::fs::create_dir_all(&out);
    let recovery = out.join("recovery");
    let _ = std::fs::remove_dir_all(&recovery);
    let mut r = Results::default();
    // The settings a new user starts with (4K canvas, Windows Ink).
    let settings = Settings::default();
    // As big as the screen allows, up to 1400 x 900: every point the test clicks must be on
    // the screen (Windows keeps the cursor on it, so a click below the screen's edge would
    // land somewhere else).
    let place = super::work_area().map_or((40, 40, 1440, 940, false), |w| (w.left, w.top, w.left + (w.right - w.left).min(1400), w.top + (w.bottom - w.top).min(900), false));
    let hwnd = match create_window("Recedo self-test", Some(place), true) {
        Ok(h) => h,
        Err(e) => {
            println!("FAIL  window: {e}");
            return 1;
        }
    };
    let mut app = match App::new(hwnd, settings, Some(recovery.clone()), false) {
        Ok(a) => a,
        Err(e) => {
            println!("FAIL  gpu: {e}");
            return 1;
        }
    };
    let capture = app.enable_capture();
    let gpu = app.describe_gpu();
    install(app);
    // SAFETY: valid window.
    unsafe {
        ShowWindow(hwnd, SW_SHOWNORMAL);
        SetForegroundWindow(hwnd);
    }
    run_for(Duration::from_millis(600));
    let (cw, ch) = client_size(hwnd);
    // SAFETY: valid window.
    let dpi = unsafe { windows_sys::Win32::UI::HiDpi::GetDpiForWindow(hwnd) };
    let (dw, dh) = with_app(|a| (a.session.ed.doc.width, a.session.ed.doc.height)).unwrap_or((0, 0));
    r.note(format!("recedo {} ({})", crate::ui::VERSION, crate::GIT_HASH));
    let wine = recedo_win::sys::wine_version().map_or_else(String::new, |v| format!(" (under Wine {v})"));
    r.note(format!("os: {}{wine}", recedo_win::sys::os_version()));
    r.note(format!("gpu: {gpu}"));
    r.note(format!("client area {cw}x{ch} px at {dpi} dpi; painting {dw}x{dh}"));
    let frames = with_app(|a| a.stats.frames).unwrap_or(0);
    r.check("window renders", frames > 0, format!("{frames} frames in the first 0.6 s"));
    let idle0 = with_app(|a| a.stats.frames).unwrap_or(0);
    run_for(Duration::from_millis(1000));
    let idle = with_app(|a| a.stats.frames).unwrap_or(0) - idle0;
    r.check("idle: no redrawing while nothing changes", idle <= 5, format!("{idle} frames in 1 s of idling"));

    // Redraw continuously for a second, as when painting, and collect each frame's time.
    let start = Instant::now();
    let mut last = with_app(|a| a.stats.frames).unwrap_or(0);
    let f0 = last;
    let mut work = Vec::new();
    while start.elapsed() < Duration::from_secs(1) {
        with_app(|a| a.request_frame());
        step(false);
        if let Some((n, wk)) = with_app(|a| (a.stats.frames, a.stats.last_work_ms))
            && n != last
        {
            last = n;
            work.push(wk);
        }
    }
    work.sort_by(f64::total_cmp);
    let med = work.get(work.len() / 2).copied().unwrap_or(0.0);
    let worst = work.last().copied().unwrap_or(0.0);
    r.note(format!("continuous redraw: {} frames in 1 s; app work per frame median {med:.1} ms, worst {worst:.1} ms", last - f0));
    r.check("app work per frame", last > f0 && med <= 8.0, format!("median {med:.1} ms (limit 8 ms, half a 60 Hz frame)"));

    let c = canvas_area();
    r.note(format!("canvas area {:.0}x{:.0} at ({:.0}, {:.0})", c.width(), c.height(), c.left, c.top));
    let on_screen = on_screen_whole(hwnd, cw, ch);
    if !on_screen {
        r.check("layout: the window fits on the screen", false, format!("client area {cw}x{ch} is partly off the screen; the clicks would land elsewhere"));
    } else if c.width() < 500.0 || c.height() < 400.0 {
        r.check("layout", false, format!("canvas area too small: {c:?}"));
    } else {
        r.check("layout: canvas area between the panels", true, format!("{:.0}x{:.0}", c.width(), c.height()));
        mouse_tests(&mut r, hwnd, c);
        view_tests(&mut r, hwnd, c);
        ink_test(&mut r, hwnd, c);
        wintab_test(&mut r, hwnd, c);
        layer_tests(&mut r, hwnd, c);
        screen_test(&mut r, &out, capture);
        tool_tests(&mut r, hwnd, c);
        file_tests(&mut r, &out);
        recovery_tests(&mut r, hwnd, c, &recovery);
    }
    let summary = format!("{} passed, {} failed, {} skipped", r.passed, r.failed, r.skipped);
    let _ = writeln!(r.text, "\n{summary}");
    println!("\n{summary}");
    let _ = std::fs::write(out.join("selftest-results.txt"), &r.text);
    shutdown(hwnd);
    if r.failed > 0 { 1 } else { 0 }
}

