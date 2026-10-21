//! `recedo-tablet-test.exe --selftest <dir>`: drives the real app with injected input and
//! checks what comes out. Used by CI on Windows and under Wine; needs no tablet.
//!
//! * Mouse: a drag made with `SetCursorPos` + `SendInput`.
//! * Windows Ink: a pen stroke injected with `InjectSyntheticPointerInput` (Windows 10
//!   1809+), with pressure and tilt, several frames per message so the history
//!   (coalesced samples) path is exercised. Skipped where the system cannot inject pens.
//! * Wintab: only if `RECEDO_WINTAB_DLL` points at the mock driver (`wintab_mock.dll`),
//!   which plays a scripted stroke with one deliberately missing packet.
//!
//! Each check prints PASS/FAIL/SKIP; the exit code is 1 if anything failed.

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use recedo_tablet::Source;
use windows_sys::Win32::Foundation::{GetLastError, POINT};
use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
use windows_sys::Win32::UI::Controls::{HSYNTHETICPOINTERDEVICE, POINTER_FEEDBACK_MODE, POINTER_FEEDBACK_NONE, POINTER_TYPE_INFO};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEINPUT, SendInput};
use windows_sys::Win32::UI::Input::Pointer::{POINTER_FLAG_DOWN, POINTER_FLAG_INCONTACT, POINTER_FLAG_INRANGE, POINTER_FLAG_UP, POINTER_FLAG_UPDATE};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::app::{App, client_size};
use super::{create_window, install, set_dpi_awareness, step, with_app};
use crate::gpu::{unpremultiply, write_png};
use crate::model::{Api, Settings};
use crate::replay::measure;

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

fn force_frame() {
    with_app(|a| a.request_frame());
    step(false);
}

fn screen_of(hwnd: windows_sys::Win32::Foundation::HWND, x: i32, y: i32) -> POINT {
    let mut p = POINT { x, y };
    // SAFETY: valid window.
    unsafe { ClientToScreen(hwnd, &mut p) };
    p
}

fn set_api(api: Api) {
    with_app(|a| {
        a.model.settings.api = api;
        a.tracker.end();
        if api == Api::Wintab { a.open_wintab() } else { a.close_wintab() }
    });
    run_for(Duration::from_millis(100));
}

fn canvas_pixels() -> Option<(u32, u32, Vec<u8>)> {
    with_app(|a| a.canvas.read_canvas(&a.gpu).ok().map(|px| (a.canvas.width, a.canvas.height, px))).flatten()
}

/// Inked pixels in a column, within ±r rows of y.
fn ink_near(px: &(u32, u32, Vec<u8>), x: i32, y: i32, r: i32) -> u32 {
    let (w, h, data) = px;
    if x < 0 || x >= *w as i32 {
        return 0;
    }
    let y0 = (y - r).max(0) as u32;
    let y1 = ((y + r) as u32).min(*h);
    measure::column_ink(data, *w, x as u32, y0, y1)
}

fn mouse_test(r: &mut Results, hwnd: windows_sys::Win32::Foundation::HWND, x0: i32, x1: i32, y: i32) {
    set_api(Api::MouseOnly);
    let before = with_app(|a| a.model.stats.mouse.contact_samples).unwrap_or(0);
    let start = screen_of(hwnd, x0, y);
    let send = |flags: u32| {
        let input = INPUT { r#type: INPUT_MOUSE, Anonymous: INPUT_0 { mi: MOUSEINPUT { dx: 0, dy: 0, mouseData: 0, dwFlags: flags, time: 0, dwExtraInfo: 0 } } };
        // SAFETY: one valid INPUT.
        unsafe { SendInput(1, &input, std::mem::size_of::<INPUT>() as i32) }
    };
    // SAFETY: plain calls.
    unsafe { SetCursorPos(start.x, start.y) };
    run_for(Duration::from_millis(50));
    let sent = send(MOUSEEVENTF_LEFTDOWN);
    run_for(Duration::from_millis(30));
    for i in 0..=60 {
        let p = screen_of(hwnd, x0 + (x1 - x0) * i / 60, y);
        // SAFETY: plain call.
        unsafe { SetCursorPos(p.x, p.y) };
        run_for(Duration::from_millis(8));
    }
    send(MOUSEEVENTF_LEFTUP);
    run_for(Duration::from_millis(150));
    force_frame();
    let after = with_app(|a| a.model.stats.mouse.contact_samples).unwrap_or(0);
    r.check("mouse: input reached the app", after > before + 20, format!("{} touching mouse samples (SendInput returned {sent})", after - before));
    if let Some(px) = canvas_pixels() {
        let cols = (x0 + 10..x1 - 10).filter(|&x| ink_near(&px, x, y, 4) > 0).count();
        let total = (x1 - x0 - 20) as usize;
        r.check("mouse: line drawn", cols * 100 >= total * 95, format!("{cols} of {total} columns inked along y={y}"));
    }
}

struct InjectedFrame {
    x: i32,
    y: i32,
    pressure: f32,
    tilt_x: i32,
    tilt_y: i32,
    flags: u32,
}

type FnCreateDevice = unsafe extern "system" fn(POINTER_INPUT_TYPE, u32, POINTER_FEEDBACK_MODE) -> HSYNTHETICPOINTERDEVICE;
type FnInject = unsafe extern "system" fn(HSYNTHETICPOINTERDEVICE, *const POINTER_TYPE_INFO, u32) -> i32;
type FnDestroyDevice = unsafe extern "system" fn(HSYNTHETICPOINTERDEVICE);

fn ink_test(r: &mut Results, hwnd: windows_sys::Win32::Foundation::HWND, x0: i32, x1: i32, y: i32) {
    set_api(Api::WindowsInk);
    use recedo_tablet::ink::user32_proc;
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
    let mut frames = Vec::new();
    let hover = POINTER_FLAG_INRANGE | POINTER_FLAG_UPDATE;
    for i in 0..10 {
        frames.push(InjectedFrame { x: x0 - 40 + i * 4, y: y - 20, pressure: 0.0, tilt_x: 0, tilt_y: 0, flags: hover });
    }
    let n = 120;
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let flags = if i == 0 { POINTER_FLAG_INRANGE | POINTER_FLAG_INCONTACT | POINTER_FLAG_DOWN } else { POINTER_FLAG_INRANGE | POINTER_FLAG_INCONTACT | POINTER_FLAG_UPDATE };
        frames.push(InjectedFrame {
            x: x0 + ((x1 - x0) as f32 * t) as i32,
            y,
            pressure: 0.1 + 0.9 * t,
            tilt_x: (-40.0 + 80.0 * t) as i32,
            tilt_y: 15,
            flags,
        });
    }
    let last = frames.last().map(|f| (f.x, f.y)).unwrap();
    frames.push(InjectedFrame { x: last.0, y: last.1, pressure: 0.0, tilt_x: 40, tilt_y: 15, flags: POINTER_FLAG_INRANGE | POINTER_FLAG_UP });
    frames.push(InjectedFrame { x: last.0 + 5, y: last.1 - 10, pressure: 0.0, tilt_x: 0, tilt_y: 0, flags: hover });
    frames.push(InjectedFrame { x: last.0 + 5, y: last.1 - 10, pressure: 0.0, tilt_x: 0, tilt_y: 0, flags: POINTER_FLAG_UPDATE });
    let contact_frames = frames.iter().filter(|f| f.flags & POINTER_FLAG_INCONTACT != 0).count() as u64;

    let before = with_app(|a| (a.model.stats.ink.contact_samples, a.model.stats.ink.samples)).unwrap_or((0, 0));
    let mut injected_ok = 0;
    let mut first_error = 0;
    let mut expected: Vec<Expect> = Vec::new();
    for (i, f) in frames.iter().enumerate() {
        let p = screen_of(hwnd, f.x, f.y);
        // SAFETY: zeroed POD, then filled in.
        let mut info: POINTER_TYPE_INFO = unsafe { std::mem::zeroed() };
        info.r#type = PT_PEN;
        // SAFETY: writing the pen variant of the union, which `type` selects.
        unsafe {
            let pen = &mut info.Anonymous.penInfo;
            pen.pointerInfo.pointerType = PT_PEN;
            pen.pointerInfo.pointerId = 0;
            pen.pointerInfo.ptPixelLocation = p;
            pen.pointerInfo.pointerFlags = f.flags;
            pen.penMask = PEN_MASK_PRESSURE | PEN_MASK_TILT_X | PEN_MASK_TILT_Y;
            pen.pressure = (f.pressure * 1024.0).round() as u32;
            pen.tiltX = f.tilt_x;
            pen.tiltY = f.tilt_y;
        }
        // SAFETY: valid device and one frame.
        if unsafe { inject(dev, &info, 1) } != 0 {
            injected_ok += 1;
            if f.flags & POINTER_FLAG_INCONTACT != 0 {
                let p = screen_of(hwnd, f.x, f.y);
                expected.push(Expect {
                    x: p.x as f64,
                    y: p.y as f64,
                    pressure: (f.pressure * 1024.0).round() / 1024.0,
                    tilt_x: f.tilt_x as f32,
                    tilt_y: f.tilt_y as f32,
                });
            }
        } else if first_error == 0 {
            // SAFETY: plain call.
            first_error = unsafe { GetLastError() };
        }
        // Inject four frames per message-loop turn so Windows coalesces them into one
        // message's history, which the app must unpack.
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
    if injected_ok == 0 {
        r.skip("windows ink", format!("InjectSyntheticPointerInput refused every frame (error {first_error})"));
        return;
    }
    let (contact, total, max_hist, coarse) = with_app(|a| {
        let st = &a.model.stats.ink;
        (st.contact_samples - before.0, st.samples - before.1, a.ink.max_history, a.ink.coarse_fallbacks)
    })
    .unwrap();
    r.note(format!(
        "injected {injected_ok}/{} frames ({contact_frames} touching); app got {total} pen samples, {contact} touching; most per message {max_hist}; whole-pixel fallbacks {coarse}",
        frames.len()
    ));
    if max_hist > 1 {
        r.check("ink: coalesced history unpacked", true, format!("up to {max_hist} samples in one message"));
    } else {
        r.note("Windows did not coalesce frames this run, so the history path was not exercised");
    }
    // Injected positions are whole pixels; the app's sub-pixel (HIMETRIC) position may differ
    // from them by up to a pixel.
    compare(r, "ink", Source::Ink, &expected, Tolerance { px: 1.5, pressure: 0.0005, tilt: 0.01 });
    if let Some(px) = canvas_pixels() {
        let thin = ink_near(&px, x0 + 20, y, 30);
        let thick = ink_near(&px, x1 - 20, y, 30);
        r.check("ink: pressure changes line width", thick >= thin * 2 && thin > 0, format!("{thin} px wide near the start, {thick} px near the end"));
    }
}

/// What a touching sample should be: position (screen px), pressure, tilt X/Y.
#[derive(Clone, Copy)]
struct Expect {
    x: f64,
    y: f64,
    pressure: f32,
    tilt_x: f32,
    tilt_y: f32,
}

/// Largest acceptable differences.
struct Tolerance {
    px: f64,
    pressure: f32,
    tilt: f32,
}

/// Compares the app's recorded touching samples of `source`, in order, with what was sent.
fn compare(r: &mut Results, label: &str, source: Source, expected: &[Expect], tol: Tolerance) {
    let got: Vec<Expect> = with_app(|a| {
        let o = recedo_tablet::wintab::client_origin(a.hwnd);
        a.model
            .recorded
            .iter()
            .filter(|s| s.source == source && s.contact())
            .map(|s| Expect { x: s.x + o.0, y: s.y + o.1, pressure: s.pressure, tilt_x: s.tilt_x, tilt_y: s.tilt_y })
            .collect()
    })
    .unwrap_or_default();
    r.check(&format!("{label}: every touching sample arrived"), got.len() == expected.len(), format!("{} of {}", got.len(), expected.len()));
    if got.len() != expected.len() || got.is_empty() {
        return;
    }
    let pairs = || got.iter().zip(expected);
    let pos = pairs().map(|(g, e)| (g.x - e.x).hypot(g.y - e.y)).fold(0.0, f64::max);
    let pr = pairs().map(|(g, e)| (g.pressure - e.pressure).abs()).fold(0.0, f32::max);
    let tilt = pairs().map(|(g, e)| (g.tilt_x - e.tilt_x).abs().max((g.tilt_y - e.tilt_y).abs())).fold(0.0, f32::max);
    r.check(&format!("{label}: positions"), pos <= tol.px, format!("worst error {pos:.3} px"));
    r.check(&format!("{label}: pressure"), pr <= tol.pressure, format!("worst error {pr:.5}"));
    r.check(&format!("{label}: tilt"), tilt <= tol.tilt, format!("worst error {tilt:.2}°"));
    let (pmin, pmax) = got.iter().fold((f32::MAX, f32::MIN), |(a, b), g| (a.min(g.pressure), b.max(g.pressure)));
    let (tmin, tmax) = got.iter().fold((f32::MAX, f32::MIN), |(a, b), g| (a.min(g.tilt_x), b.max(g.tilt_x)));
    r.note(format!("{label}: pressure {pmin:.3} .. {pmax:.3}, tilt X {tmin:.1} .. {tmax:.1}"));
}

type PlayFn = unsafe extern "system" fn(i32, i32, i32, i32) -> i32;

fn wintab_test(r: &mut Results, hwnd: windows_sys::Win32::Foundation::HWND, x0: i32, x1: i32, y0: i32, y1: i32) {
    if std::env::var(recedo_tablet::wintab::DLL_OVERRIDE_ENV).is_err() {
        r.skip("wintab", "RECEDO_WINTAB_DLL is not set (point it at wintab_mock.dll to test Wintab without a tablet)");
        return;
    }
    set_api(Api::Wintab);
    let play: Option<PlayFn> = with_app(|a| {
        a.wintab.as_ref().and_then(|w| w.ctx.proc_address(c"wtmock_play")).map(|f| {
            // SAFETY: the mock exports this signature.
            unsafe { std::mem::transmute::<unsafe extern "system" fn() -> isize, PlayFn>(f) }
        })
    })
    .flatten();
    let Some(play) = play else {
        let state = with_app(|a| format!("{:?}", a.model.wintab.state)).unwrap_or_default();
        r.check("wintab: mock driver opened", false, state);
        return;
    };
    let (layout, qsize) = with_app(|a| a.wintab.as_ref().map(|w| (w.ctx.layout, w.ctx.queue_size))).flatten().unwrap();
    r.check("wintab: context opened", true, format!("{} packets of {} bytes, fields {}", qsize, layout.size, recedo_tablet::wintab_abi::describe_mask(layout.mask)));
    let tl = screen_of(hwnd, x0, y0);
    let br = screen_of(hwnd, x1, y1);
    let before = with_app(|a| (a.model.stats.wintab.samples, a.model.stats.wintab.contact_samples)).unwrap();
    // SAFETY: mock export.
    let sent = unsafe { play(tl.x, tl.y, br.x - tl.x, br.y - tl.y) };
    // The script is ~220 packets at 5 ms.
    run_for(Duration::from_millis(1800));
    force_frame();
    let (samples, contact, gaps, active, mismatch, batch_max) = with_app(|a| {
        let st = &a.model.stats.wintab;
        let w = a.wintab.as_ref().unwrap();
        (st.samples - before.0, st.contact_samples - before.1, st.serial_gaps, w.calibrator.active().label(), w.calibrator.mismatch(), st.batch_max)
    })
    .unwrap();
    r.note(format!("mock sent {sent} packets; app got {samples}, {contact} touching, up to {batch_max} per message; mapping: {active}"));
    r.check("wintab: every packet arrived", samples as i32 == sent, format!("{samples} of {sent}"));
    r.check("wintab: the deliberately lost packet was detected", gaps == 1, format!("{gaps} lost by serial number"));
    r.check("wintab: mapping lines up with the cursor", !mismatch, active);
    let script = recedo_tablet::mock_script::stroke(tl.x as f64, tl.y as f64, (br.x - tl.x) as f64, (br.y - tl.y) as f64);
    let expected: Vec<Expect> = script
        .iter()
        .filter(|s| s.tip && !s.drop)
        .map(|s| Expect { x: s.sx, y: s.sy, pressure: s.pressure as f32, tilt_x: s.tilt_x as f32, tilt_y: s.tilt_y as f32 })
        .collect();
    // Tablet units are 20-50 per pixel here, pressure has 8192 levels and angles tenths of
    // a degree, so all three should come back almost exactly.
    compare(r, "wintab", Source::Wintab, &expected, Tolerance { px: 0.1, pressure: 0.0002, tilt: 0.3 });
    if let Some(px) = canvas_pixels() {
        let mid_y = (y0 + y1) / 2;
        let inked = (x0 + 100..x1 - 100).step_by(10).filter(|&x| ink_near(&px, x, mid_y, (y1 - y0) / 2) > 0).count();
        let total = (x0 + 100..x1 - 100).step_by(10).count();
        r.check("wintab: stroke drawn", inked == total, format!("{inked} of {total} sampled columns inked"));
    }
}

pub fn run(out_dir: Option<&str>) -> i32 {
    super::install_crash_report(false);
    set_dpi_awareness();
    let out = std::path::PathBuf::from(out_dir.unwrap_or("selftest-output"));
    let _ = std::fs::create_dir_all(&out);
    let mut r = Results::default();
    // The same defaults the user starts with (red cross on).
    let settings = Settings::default();
    let hwnd = match create_window("Recedo self-test", Some((40, 40, 1340, 860, false)), true) {
        Ok(h) => h,
        Err(e) => {
            println!("FAIL  window: {e}");
            return 1;
        }
    };
    let app = match App::new(hwnd, settings, false) {
        Ok(a) => a,
        Err(e) => {
            println!("FAIL  gpu: {e}");
            return 1;
        }
    };
    let gpu = app.model.gpu.clone();
    install(app);
    // SAFETY: valid window.
    unsafe {
        ShowWindow(hwnd, SW_SHOWNORMAL);
        SetForegroundWindow(hwnd);
    }
    with_app(|a| a.model.recording = true);
    run_for(Duration::from_millis(500));
    let (cw, ch) = client_size(hwnd);
    // SAFETY: valid window.
    let dpi = unsafe { windows_sys::Win32::UI::HiDpi::GetDpiForWindow(hwnd) };
    r.note(format!("os: {}", super::sys::os_version()));
    r.note(format!("gpu: {gpu}"));
    r.note(format!("client area {cw}x{ch} px at {dpi} dpi"));
    let frames = with_app(|a| a.model.frame.frames).unwrap_or(0);
    r.check("window renders", frames > 0, format!("{frames} frames while idle for 0.5 s (it redraws only when something changes)"));
    // Redraw continuously for a second, as when drawing.
    let start = Instant::now();
    let f0 = frames;
    while start.elapsed() < Duration::from_secs(1) {
        with_app(|a| a.request_frame());
        step(false);
    }
    let (fps, work, wait) = with_app(|a| (a.model.frame.frames - f0, a.model.frame.work_ms, a.model.frame.wait_ms)).unwrap_or((0, 0.0, 0.0));
    // The overall frame rate depends on this machine's GPU and display (CI machines render
    // in software), so it is reported, not judged. What the app itself spends per frame is
    // its own responsibility: it must stay well under a 60 Hz frame (16.7 ms).
    r.note(format!("continuous redraw: {fps} frames in 1 s; per frame {work:.1} ms app work, {wait:.1} ms waiting for GPU/display"));
    r.check("app work per frame", fps > 0 && work <= 8.0, format!("{work:.1} ms (limit 8 ms, half a 60 Hz frame)"));
    let panel = (crate::ui::PANEL_WIDTH * dpi as f32 / 96.0) as i32;
    let x0 = panel + 60;
    let x1 = cw as i32 - 60;
    if x1 - x0 < 300 {
        r.check("layout", false, format!("canvas area too small ({cw}x{ch})"));
    } else {
        mouse_test(&mut r, hwnd, x0, x1, 80);
        ink_test(&mut r, hwnd, x0, x1, 220);
        wintab_test(&mut r, hwnd, x0, x1, 330, 560);
    }
    force_frame();
    if let Some((w, h, mut px)) = canvas_pixels() {
        unpremultiply(&mut px);
        let path = out.join("selftest-canvas.png");
        match write_png(&path, w, h, &px) {
            Ok(()) => r.note(format!("canvas saved to {}", path.display())),
            Err(e) => r.note(format!("could not save canvas: {e}")),
        }
    }
    let report = with_app(|a| crate::report::build(&mut a.model, recedo_tablet::clock::now_us())).unwrap_or_default();
    let _ = std::fs::write(out.join("selftest-app-report.txt"), &report);
    let csv = with_app(|a| recedo_tablet::record::to_csv(&a.model.recorded)).unwrap_or_default();
    let _ = std::fs::write(out.join("selftest-recording.csv"), csv);
    let summary = format!("{} passed, {} failed, {} skipped", r.passed, r.failed, r.skipped);
    let _ = writeln!(r.text, "\n{summary}");
    println!("\n{summary}");
    let _ = std::fs::write(out.join("selftest-results.txt"), &r.text);
    super::shutdown(hwnd);
    if r.failed > 0 { 1 } else { 0 }
}
