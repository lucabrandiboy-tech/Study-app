//! Recedo, the painting app.
//!
//! `editor` is portable and unit-tested on any OS; the window, input and GPU code under
//! `win` is Windows-only.

#![cfg_attr(windows, windows_subsystem = "windows")]
// Off Windows only the tests use the editor.
#![cfg_attr(not(windows), allow(dead_code))]

mod editor;

fn main() {
    #[cfg(not(windows))]
    {
        eprintln!("Recedo runs on Windows. On this system only its tests run (cargo test).");
        std::process::exit(1);
    }
}
