//! Recedo in the browser: the same editor, interface and canvas display as `recedo.exe`
//! (its modules are compiled in from `recedo-app`), with a web shell instead of the Win32
//! one. Pen input comes from the browser's pointer events (pressure, tilt and every
//! coalesced sample); files are opened from a file picker and saved as downloads.
//! Installed from the browser it runs as an app in its own window (see `web/`).

#![cfg(target_arch = "wasm32")]
// Desktop-only parts of the shared modules (file paths, crash recovery) are unused here.
#![allow(dead_code)]

#[path = "../../recedo-app/src/dock.rs"]
mod dock;
#[path = "../../recedo-app/src/editor.rs"]
mod editor;
#[path = "../../recedo-app/src/icons.rs"]
mod icons;
#[path = "../../recedo-app/src/previews.rs"]
mod previews;
#[path = "../../recedo-app/src/radial.rs"]
mod radial;
#[path = "../../recedo-app/src/session.rs"]
mod session;
#[path = "../../recedo-app/src/settings.rs"]
mod settings;
#[path = "../../recedo-app/src/ui.rs"]
mod ui;
#[path = "../../recedo-app/src/widgets.rs"]
mod widgets;

mod shell;
