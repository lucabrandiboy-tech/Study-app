//! Recedo, the painting app.
//!
//! `editor` is portable and unit-tested on any OS; the window, input and GPU code under
//! `win` is Windows-only.

#![cfg_attr(windows, windows_subsystem = "windows")]
// Until the Windows shell is added (next commits), nothing outside the tests uses the editor.
#![allow(dead_code)]

mod dock;
mod editor;
mod session;
mod settings;
mod ui;
mod widgets;

fn main() {
    #[cfg(not(windows))]
    {
        eprintln!("Recedo runs on Windows. On this system only its tests run (cargo test).");
        std::process::exit(1);
    }
}
