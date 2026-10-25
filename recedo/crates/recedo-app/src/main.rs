//! Recedo, the painting app.
//!
//! Everything but `win` is portable and tested on any OS (the interface runs off-screen in
//! the tests); the window, input and GPU surface code under `win` is Windows-only.

#![cfg_attr(windows, windows_subsystem = "windows")]
// Elsewhere the program is only a stub and the app's code is used by its tests alone.
#![cfg_attr(not(windows), allow(dead_code))]

mod dock;
mod editor;
mod session;
mod settings;
mod ui;
mod widgets;
#[cfg(windows)]
mod win;

/// The commit this build came from (with "-dirty" for uncommitted changes).
pub const GIT_HASH: &str = env!("RECEDO_GIT_HASH");

fn main() {
    #[cfg(windows)]
    std::process::exit(win::main());
    #[cfg(not(windows))]
    {
        eprintln!("Recedo runs on Windows. On this system only its tests run (cargo test). Build {GIT_HASH}.");
        std::process::exit(1);
    }
}
