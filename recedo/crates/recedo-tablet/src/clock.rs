//! The app clock, in microseconds.
//!
//! On Windows this is `QueryPerformanceCounter`, the same clock Windows Ink stamps pointer
//! frames with (`POINTER_INFO::PerformanceCount`), so Ink timestamps convert exactly.
//! Elsewhere it is a monotonic clock started at first use.

#[cfg(windows)]
mod imp {
    use std::sync::OnceLock;
    use windows_sys::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

    fn freq() -> u64 {
        static FREQ: OnceLock<u64> = OnceLock::new();
        *FREQ.get_or_init(|| {
            let mut f = 0i64;
            // SAFETY: plain out-parameter call.
            unsafe { QueryPerformanceFrequency(&mut f) };
            f.max(1) as u64
        })
    }

    pub fn now_us() -> u64 {
        let mut c = 0i64;
        // SAFETY: plain out-parameter call.
        unsafe { QueryPerformanceCounter(&mut c) };
        qpc_to_us(c as u64)
    }

    pub fn qpc_to_us(ticks: u64) -> u64 {
        let f = freq();
        // Split to avoid overflow for large tick counts.
        (ticks / f) * 1_000_000 + (ticks % f) * 1_000_000 / f
    }
}

#[cfg(not(windows))]
mod imp {
    use std::sync::OnceLock;
    use web_time::Instant;

    fn start() -> Instant {
        static START: OnceLock<Instant> = OnceLock::new();
        *START.get_or_init(Instant::now)
    }

    pub fn now_us() -> u64 {
        start().elapsed().as_micros() as u64
    }

    /// Without a performance counter, ticks are taken to already be microseconds.
    pub fn qpc_to_us(ticks: u64) -> u64 {
        ticks
    }
}

pub use imp::{now_us, qpc_to_us};

/// Converts a 32-bit millisecond timestamp (Wintab `pkTime`, `GetMessageTime`) to the app clock.
///
/// The first timestamp seen is anchored to the time it was received; later ones keep the
/// device's own spacing. Wrap-around of the 32-bit counter (every 49.7 days) is handled.
#[derive(Clone, Debug, Default)]
pub struct MsClockMapper {
    anchor: Option<(u32, u64)>,
}

impl MsClockMapper {
    pub fn map(&mut self, device_ms: u32, recv_us: u64) -> u64 {
        let (anchor_ms, anchor_us) = *self.anchor.get_or_insert((device_ms, recv_us));
        let delta_ms = device_ms.wrapping_sub(anchor_ms) as i32 as i64;
        let t = anchor_us as i64 + delta_ms * 1000;
        // A device clock that drifts ahead of real time is re-anchored so samples never
        // appear to come from the future.
        if t > recv_us as i64 + 50_000 {
            self.anchor = Some((device_ms, recv_us));
            return recv_us;
        }
        t.max(0) as u64
    }

    pub fn reset(&mut self) {
        self.anchor = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ms_mapper_keeps_device_spacing() {
        let mut m = MsClockMapper::default();
        assert_eq!(m.map(1000, 5_000_000), 5_000_000);
        assert_eq!(m.map(1005, 5_009_000), 5_005_000);
        assert_eq!(m.map(1010, 5_009_500), 5_010_000);
    }

    #[test]
    fn ms_mapper_handles_wraparound() {
        let mut m = MsClockMapper::default();
        assert_eq!(m.map(u32::MAX - 2, 10_000_000), 10_000_000);
        assert_eq!(m.map(2, 10_005_000), 10_005_000);
    }

    #[test]
    fn ms_mapper_reanchors_when_device_runs_ahead() {
        let mut m = MsClockMapper::default();
        m.map(0, 1_000_000);
        // Device claims 1 s passed but only 10 ms did.
        assert_eq!(m.map(1000, 1_010_000), 1_010_000);
        assert_eq!(m.map(1005, 1_016_000), 1_015_000);
    }

    #[test]
    fn now_is_monotonic() {
        let a = now_us();
        let b = now_us();
        assert!(b >= a);
    }
}
