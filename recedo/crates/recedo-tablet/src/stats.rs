//! Event-rate and quality statistics for each input source.

use std::collections::{HashSet, VecDeque};

use crate::sample::{Flags, PenSample};

/// Counts events in a sliding time window.
#[derive(Clone, Debug)]
pub struct RateMeter {
    window_us: u64,
    times: VecDeque<u64>,
}

impl RateMeter {
    pub fn new(window_us: u64) -> RateMeter {
        RateMeter { window_us: window_us.max(1), times: VecDeque::new() }
    }

    pub fn push(&mut self, t_us: u64) {
        self.times.push_back(t_us);
        // Bound memory even if rate() is never called.
        if self.times.len() > 100_000 {
            self.times.pop_front();
        }
    }

    pub fn push_n(&mut self, t_us: u64, n: usize) {
        for _ in 0..n {
            self.push(t_us);
        }
    }

    /// Events per second over the window ending at `now_us`.
    pub fn rate(&mut self, now_us: u64) -> f64 {
        // The window is (now - window, now]; written without subtraction so it is also
        // right during the first second of the clock.
        while self.times.front().is_some_and(|&t| t + self.window_us <= now_us) {
            self.times.pop_front();
        }
        self.times.len() as f64 * 1_000_000.0 / self.window_us as f64
    }
}

/// Keeps the most recent intervals and reports percentiles.
#[derive(Clone, Debug, Default)]
pub struct Intervals {
    ring: VecDeque<u32>,
}

impl Intervals {
    const CAP: usize = 1024;

    pub fn push(&mut self, dt_us: u32) {
        if self.ring.len() == Self::CAP {
            self.ring.pop_front();
        }
        self.ring.push_back(dt_us);
    }

    pub fn len(&self) -> usize {
        self.ring.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ring.is_empty()
    }

    /// Percentile 0..=100 (nearest rank), in µs.
    pub fn percentile(&self, p: f64) -> Option<u32> {
        if self.ring.is_empty() {
            return None;
        }
        let mut v: Vec<u32> = self.ring.iter().copied().collect();
        v.sort_unstable();
        let rank = ((p / 100.0) * v.len() as f64).ceil() as usize;
        Some(v[rank.clamp(1, v.len()) - 1])
    }

    pub fn max(&self) -> Option<u32> {
        self.ring.iter().copied().max()
    }
}

/// Min/max tracker.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Range {
    pub min: f32,
    pub max: f32,
    pub seen: bool,
}

impl Range {
    pub fn add(&mut self, v: f32) {
        if !self.seen {
            *self = Range { min: v, max: v, seen: true };
        } else {
            self.min = self.min.min(v);
            self.max = self.max.max(v);
        }
    }
}

/// Everything measured for one input source.
#[derive(Clone, Debug)]
pub struct SourceStats {
    pub samples: u64,
    pub contact_samples: u64,
    pub messages: u64,
    pub rate: RateMeter,
    pub msg_rate: RateMeter,
    pub peak_rate: f64,
    pub batch_max: u32,
    /// Wintab: packets missing according to their serial numbers.
    pub serial_gaps: u64,
    /// Wintab: packets flagged with a queue overflow.
    pub queue_errors: u64,
    /// Samples whose position fell back to whole pixels.
    pub coarse_positions: u64,
    pub intervals: Intervals,
    pub pressure: Range,
    pub tilt_x: Range,
    pub tilt_y: Range,
    pub twist: Range,
    pressure_levels: HashSet<u32>,
    last_serial: Option<u32>,
    last_t: Option<u64>,
    pub last: Option<PenSample>,
    pub eraser_seen: bool,
    pub barrel_seen: u32,
}

impl Default for SourceStats {
    fn default() -> Self {
        SourceStats {
            samples: 0,
            contact_samples: 0,
            messages: 0,
            rate: RateMeter::new(1_000_000),
            msg_rate: RateMeter::new(1_000_000),
            peak_rate: 0.0,
            batch_max: 0,
            serial_gaps: 0,
            queue_errors: 0,
            coarse_positions: 0,
            intervals: Intervals::default(),
            pressure: Range::default(),
            tilt_x: Range::default(),
            tilt_y: Range::default(),
            twist: Range::default(),
            pressure_levels: HashSet::new(),
            last_serial: None,
            last_t: None,
            last: None,
            eraser_seen: false,
            barrel_seen: 0,
        }
    }
}

/// A pause longer than this starts a new run for interval statistics (pen left range).
const RUN_BREAK_US: u64 = 100_000;

impl SourceStats {
    /// Records the samples delivered by one message (a `WT_PACKET` drain, one `WM_POINTER*`
    /// with its history, one mouse message). `use_serials` turns on gap detection.
    pub fn record_batch(&mut self, batch: &[PenSample], now_us: u64, use_serials: bool) {
        if batch.is_empty() {
            return;
        }
        self.messages += 1;
        self.msg_rate.push(now_us);
        self.rate.push_n(now_us, batch.len());
        self.batch_max = self.batch_max.max(batch.len() as u32);
        for s in batch {
            self.samples += 1;
            if use_serials {
                if let Some(prev) = self.last_serial {
                    let step = s.serial.wrapping_sub(prev);
                    // Small forward jumps are lost packets; anything else (repeats, resets,
                    // a new context) is not counted.
                    if (2..10_000).contains(&step) {
                        self.serial_gaps += u64::from(step - 1);
                    }
                }
                self.last_serial = Some(s.serial);
            }
            if s.flags.has(Flags::QUEUE_ERROR) {
                self.queue_errors += 1;
            }
            if s.flags.has(Flags::COARSE_POSITION) {
                self.coarse_positions += 1;
            }
            if let Some(prev) = self.last_t {
                let dt = s.t_us.saturating_sub(prev);
                if dt > 0 && dt < RUN_BREAK_US {
                    self.intervals.push(dt as u32);
                }
            }
            self.last_t = Some(s.t_us);
            if s.contact() {
                self.contact_samples += 1;
                if s.flags.has(Flags::HAS_PRESSURE) {
                    self.pressure.add(s.pressure);
                    if self.pressure_levels.len() < 70_000 {
                        self.pressure_levels.insert(s.raw_pressure);
                    }
                }
            }
            if s.flags.has(Flags::HAS_TILT) {
                self.tilt_x.add(s.tilt_x);
                self.tilt_y.add(s.tilt_y);
            }
            if s.flags.has(Flags::HAS_TWIST) {
                self.twist.add(s.twist);
            }
            self.eraser_seen |= s.eraser();
            self.barrel_seen |= s.flags.0 & (Flags::BARREL1 | Flags::BARREL2);
            self.last = Some(*s);
        }
    }

    /// Samples per second over the last second; also updates the peak.
    pub fn rate_now(&mut self, now_us: u64) -> f64 {
        let r = self.rate.rate(now_us);
        self.peak_rate = self.peak_rate.max(r);
        r
    }

    pub fn messages_now(&mut self, now_us: u64) -> f64 {
        self.msg_rate.rate(now_us)
    }

    /// Distinct raw pressure values seen while touching.
    pub fn pressure_levels(&self) -> usize {
        self.pressure_levels.len()
    }

    pub fn samples_per_message(&self) -> f64 {
        if self.messages == 0 { 0.0 } else { self.samples as f64 / self.messages as f64 }
    }

    /// Device report rate estimated from the median interval between samples.
    pub fn median_rate(&self) -> Option<f64> {
        self.intervals.percentile(50.0).filter(|&d| d > 0).map(|d| 1_000_000.0 / d as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(t_us: u64, serial: u32, contact: bool, pressure: f32) -> PenSample {
        let mut s = PenSample::synthetic(t_us, 0.0, 0.0, pressure, contact);
        s.serial = serial;
        s
    }

    #[test]
    fn rate_meter_counts_last_second() {
        let mut r = RateMeter::new(1_000_000);
        for i in 0..200 {
            r.push(i * 5_000); // 200 Hz for 1 s
        }
        assert_eq!(r.rate(999_999), 200.0);
        // (0.5 s, 1.5 s] holds the events at 505 ms ..= 995 ms.
        assert_eq!(r.rate(1_500_000), 99.0);
        assert_eq!(r.rate(3_000_000), 0.0);
    }

    #[test]
    fn serial_gaps_are_counted() {
        let mut st = SourceStats::default();
        let batch: Vec<_> = [1u32, 2, 3, 5, 6, 9].iter().map(|&n| sample(n as u64 * 5000, n, true, 0.5)).collect();
        st.record_batch(&batch, 50_000, true);
        assert_eq!(st.serial_gaps, 1 + 2);
        // A reset (serial goes back) is not a gap.
        st.record_batch(&[sample(60_000, 1, true, 0.5)], 60_000, true);
        assert_eq!(st.serial_gaps, 3);
        // Wrap-around continues normally.
        let mut st = SourceStats::default();
        st.record_batch(&[sample(0, u32::MAX, true, 0.5), sample(5000, 0, true, 0.5)], 5000, true);
        assert_eq!(st.serial_gaps, 0);
    }

    #[test]
    fn intervals_and_median_rate() {
        let mut st = SourceStats::default();
        let batch: Vec<_> = (0..100).map(|i| sample(i * 4_000, i as u32, false, 0.0)).collect();
        st.record_batch(&batch, 400_000, false);
        assert_eq!(st.intervals.percentile(50.0), Some(4_000));
        assert_eq!(st.median_rate(), Some(250.0));
        // A long pause is not an interval.
        st.record_batch(&[sample(2_000_000, 0, false, 0.0)], 2_000_000, false);
        assert_eq!(st.intervals.max(), Some(4_000));
    }

    #[test]
    fn pressure_levels_and_ranges() {
        let mut st = SourceStats::default();
        let batch: Vec<_> = (0..50).map(|i| sample(i * 5000, 0, true, i as f32 / 49.0)).collect();
        st.record_batch(&batch, 250_000, false);
        assert_eq!(st.pressure_levels(), 50);
        assert_eq!(st.pressure.min, 0.0);
        assert_eq!(st.pressure.max, 1.0);
        assert_eq!(st.contact_samples, 50);
        assert_eq!(st.batch_max, 50);
        assert_eq!(st.samples_per_message(), 50.0);
    }

    #[test]
    fn percentile_nearest_rank() {
        let mut iv = Intervals::default();
        for v in [10, 20, 30, 40, 50] {
            iv.push(v);
        }
        assert_eq!(iv.percentile(50.0), Some(30));
        assert_eq!(iv.percentile(95.0), Some(50));
        assert_eq!(iv.percentile(0.0), Some(10));
    }
}
