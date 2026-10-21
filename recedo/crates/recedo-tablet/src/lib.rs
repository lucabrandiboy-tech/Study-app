//! Pen tablet input for Recedo.
//!
//! Two Windows tablet APIs are supported, like Krita's "Tablet input API" setting:
//!
//! * **Wintab** ([`wintab`]): the driver's `Wintab32.dll`, loaded at run time. Every queued
//!   packet is drained with `WTPacketsGet`, so no sample is lost between frames.
//! * **Windows Ink** ([`ink`]): `WM_POINTER*` messages. Every coalesced sample is read with
//!   `GetPointerPenInfoHistory`, and sub-pixel positions come from the HIMETRIC location.
//!
//! Everything that does not touch Win32 (the sample model, the Wintab packet layout, tilt
//! math, screen mapping and its auto-calibration, statistics, recording) is portable so it
//! can be unit tested on any OS.

// Win32 handles (HWND, HCTX) are raw pointers in windows-sys, but they are opaque values
// that are never dereferenced here, only passed back to the system.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub mod clock;
pub mod mapping;
pub mod mock_script;
pub mod orientation;
pub mod record;
pub mod sample;
pub mod stats;
pub mod wintab_abi;

#[cfg(windows)]
pub mod ink;
#[cfg(windows)]
pub mod wintab;

pub use sample::{Flags, PenSample, Source};
