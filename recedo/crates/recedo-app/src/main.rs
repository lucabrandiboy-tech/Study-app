//! Recedo, the painting app.
//!
//! Everything but `win` is portable and tested on any OS (the interface runs off-screen in
//! the tests); the window, input and GPU surface code under `win` is Windows-only.

#![cfg_attr(windows, windows_subsystem = "windows")]
// Elsewhere the program is only a stub and the app's code is used by its tests alone.
#![cfg_attr(not(windows), allow(dead_code))]

mod bench;
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
        // The benchmark needs no window, so it runs anywhere: `recedo --benchmark [folder]`.
        let args: Vec<String> = std::env::args().skip(1).collect();
        if args.first().map(String::as_str) == Some("--benchmark") {
            let dir = args.get(1).map_or_else(std::env::temp_dir, std::path::PathBuf::from);
            match bench::run(&dir) {
                Ok(report) => println!("{report}"),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
            return;
        }
        eprintln!("Recedo runs on Windows. On this system only its tests and --benchmark run. Build {GIT_HASH}.");
        std::process::exit(1);
    }
}
