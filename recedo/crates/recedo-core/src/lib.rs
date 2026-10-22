//! Recedo's painting engine.
//!
//! Portable (no Win32, no GPU), so all of it is unit tested on any OS.

pub mod autosave;
pub mod brush;
pub mod curve;
pub mod dab;
pub mod doc;
pub mod history;
pub mod image;
pub mod kra;
pub mod lzf;
pub mod ops;
pub mod projection;
pub mod stabilizer;
pub mod tiles;
pub mod view;

pub use stabilizer::{PaintInfo, Stabilizer, StabilizerSettings};
