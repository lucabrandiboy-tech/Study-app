//! Recedo Phase 0: the tablet test app.
//!
//! * no arguments: the test window (Windows).
//! * `--selftest [dir]`: automated end-to-end check with injected input (Windows).
//! * `--replay <recording.csv> <out.png> [width height]`: draws a pen recording off-screen
//!   exactly as the app would (any OS; used to reproduce what a user drew).

#![cfg_attr(all(windows, not(test)), windows_subsystem = "windows")]

// Win32 handles are raw pointers in windows-sys but are only passed back to the system.
#![allow(clippy::not_unsafe_ptr_arg_deref)]
// The window, panel and report only run on Windows; elsewhere only --replay is built in.
#![cfg_attr(not(windows), allow(dead_code))]

mod brush;
mod gpu;
mod model;
mod replay;
mod report;
mod strokes;
mod ui;
#[cfg(windows)]
mod win;

const USAGE: &str = "\
Recedo tablet test

  recedo-tablet-test                          open the test window (Windows)
  recedo-tablet-test --selftest [dir]         automated self-test with injected input (Windows)
  recedo-tablet-test --replay <in.csv> <out.png> [width height]
                                              draw a saved pen recording to a PNG
";

fn replay_cli(args: &[String]) -> i32 {
    let (Some(input), Some(output)) = (args.first(), args.get(1)) else {
        eprintln!("{USAGE}");
        return 2;
    };
    let text = match std::fs::read_to_string(input) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{input}: {e}");
            return 1;
        }
    };
    let samples = match recedo_tablet::record::from_csv(&text) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{input}: {e}");
            return 1;
        }
    };
    let max_x = samples.iter().map(|s| s.x).fold(0.0, f64::max);
    let max_y = samples.iter().map(|s| s.y).fold(0.0, f64::max);
    let w: u32 = args.get(2).and_then(|v| v.parse().ok()).unwrap_or((max_x + 64.0).clamp(64.0, 8192.0) as u32);
    let h: u32 = args.get(3).and_then(|v| v.parse().ok()).unwrap_or((max_y + 64.0).clamp(64.0, 8192.0) as u32);
    let gpu = match gpu::Gpu::new_headless() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("no GPU: {e}");
            return 1;
        }
    };
    // Draw the API that has the most touching samples (a recording usually holds one).
    let source = recedo_tablet::Source::ALL
        .into_iter()
        .max_by_key(|src| samples.iter().filter(|s| s.source == *src && s.contact()).count());
    let brush = brush::BrushSettings::default();
    match replay::render(&gpu, &samples, &brush, source, w, h) {
        Ok(mut r) => {
            gpu::unpremultiply(&mut r.canvas);
            if let Err(e) = gpu::write_png(std::path::Path::new(output), r.width, r.height, &r.canvas) {
                eprintln!("{e}");
                return 1;
            }
            println!("{} samples, {} strokes, {} dabs -> {output} ({}x{}) on {}", samples.len(), r.strokes, r.dabs, r.width, r.height, gpu.describe());
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

#[cfg(windows)]
fn attach_console() {
    use windows_sys::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
    // So command-line runs (self-test, replay) can print to the console they came from.
    // SAFETY: plain call; harmless if there is no parent console.
    unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    #[cfg(windows)]
    if !args.is_empty() {
        attach_console();
    }
    let code = match args.first().map(String::as_str) {
        Some("--replay") => replay_cli(&args[1..]),
        Some("--help" | "-h" | "/?") => {
            println!("{USAGE}");
            0
        }
        #[cfg(windows)]
        Some("--selftest") => win::selftest::run(args.get(1).map(String::as_str)),
        #[cfg(windows)]
        None => win::run(),
        _ => {
            eprintln!("{USAGE}");
            if cfg!(windows) { 2 } else { 0 }
        }
    };
    std::process::exit(code);
}
