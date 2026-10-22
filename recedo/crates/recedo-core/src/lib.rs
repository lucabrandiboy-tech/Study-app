//! Recedo's painting engine.
//!
//! Portable (no Win32, no GPU), so all of it is unit tested on any OS.

pub mod doc;
pub mod image;
pub mod kra;
pub mod lzf;
pub mod stabilizer;
pub mod tiles;

pub use stabilizer::{PaintInfo, Stabilizer, StabilizerSettings};
