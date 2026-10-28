//! Recedo's painting engine.
//!
//! Portable (no Win32, no GPU), so all of it is unit tested on any OS.

pub mod autosave;
pub mod blend;
pub mod brush;
pub mod canvas;
pub mod curve;
pub mod dab;
pub mod doc;
pub mod fill;
pub mod history;
pub mod image;
pub mod import;
pub mod kra;
pub mod lzf;
pub mod ops;
pub mod paint;
pub mod projection;
pub mod retouch;
pub mod select;
pub mod stabilizer;
pub mod text;
pub mod tiles;
pub mod view;

pub use stabilizer::{PaintInfo, SmoothMode, Stabilizer, StabilizerSettings};
