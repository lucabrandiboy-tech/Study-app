//! Win32 helpers (shared with Recedo in `recedo-win`) and where this app keeps its files.

use std::path::PathBuf;

pub use recedo_win::sys::*;

/// `Documents\Recedo` (where recordings and reports go).
pub fn output_dir() -> PathBuf {
    documents_dir().join("Recedo")
}

/// `%APPDATA%\Recedo\tablet-test.json`.
pub fn settings_path() -> PathBuf {
    roaming_app_data_dir().join("Recedo").join("tablet-test.json")
}
