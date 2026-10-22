//! Recedo's painting engine.
//!
//! Portable (no Win32, no GPU), so all of it is unit tested on any OS.

pub mod stabilizer;

pub use stabilizer::{PaintInfo, Stabilizer, StabilizerSettings};
