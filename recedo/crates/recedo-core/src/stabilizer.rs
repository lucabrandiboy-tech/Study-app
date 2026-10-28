//! Krita's stabilizer ("Stabilizer" smoothing mode of the freehand tools).
//!
//! The behaviour follows Krita's `KisToolFreehandHelper` stabilizer and
//! `KisStabilizedEventsSampler` (GPL; studied, re-implemented here):
//!
//! 1. **Time resampling.** Pen events are resampled to one sample per millisecond. Over each
//!    polling interval of `n` ms, the events that arrived are spread evenly over the `n`
//!    samples; if none arrived, the last event is repeated. So the smoothing is over *time*,
//!    and a pen held still keeps feeding its position: the line catches up with the pen.
//! 2. **Moving average.** A queue of the last `distance` samples is kept. Each new sample
//!    produces a painted point that is the plain average of the new sample and the queue
//!    (minus its oldest entry), then the oldest entry is dropped and the new sample queued.
//!    With "stabilize sensors" on, pressure, tilt and rotation are averaged too; otherwise
//!    they come from the newest sample.
//! 3. **Delay.** A dead zone of `delay_distance` screen pixels around the last painted
//!    point: while the pen stays inside it nothing is painted and the queue is reset to the
//!    painted point. The line follows the pen like on a leash.
//! 4. **Finish line.** When the pen lifts, the last event is fed `distance` more times, so
//!    the line catches up with where the pen left the surface.
//!
//! Distances are divided by the view zoom when "scalable distance" is on (Krita's default),
//! so the stabilizer feels the same at any zoom.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

/// One point of a stroke: position in image pixels plus the pen's sensors.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PaintInfo {
    pub x: f64,
    pub y: f64,
    pub pressure: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub rotation: f32,
}

impl PaintInfo {
    pub fn new(x: f64, y: f64, pressure: f32) -> PaintInfo {
        PaintInfo { x, y, pressure, ..Default::default() }
    }

    fn same_pos(&self, other: &PaintInfo) -> bool {
        self.x == other.x && self.y == other.y
    }

    /// `self = t * self + (1 - t) * other`, position only (Krita's `mixOtherOnlyPosition`).
    fn mix_position(&mut self, t: f64, other: &PaintInfo) {
        self.x = t * self.x + (1.0 - t) * other.x;
        self.y = t * self.y + (1.0 - t) * other.y;
    }

    /// Position and sensors (Krita's `mixOtherWithoutTime`).
    fn mix_all(&mut self, t: f64, other: &PaintInfo) {
        self.mix_position(t, other);
        let tf = t as f32;
        self.pressure = tf * self.pressure + (1.0 - tf) * other.pressure;
        self.tilt_x = tf * self.tilt_x + (1.0 - tf) * other.tilt_x;
        self.tilt_y = tf * self.tilt_y + (1.0 - tf) * other.tilt_y;
        self.rotation = mix_angle_deg(self.rotation, other.rotation, tf);
    }
}

/// Mixes two angles in degrees along the shorter way round.
fn mix_angle_deg(a: f32, b: f32, t: f32) -> f32 {
    let mut d = (a - b) % 360.0;
    if d > 180.0 {
        d -= 360.0;
    } else if d < -180.0 {
        d += 360.0;
    }
    (b + t * d).rem_euclid(360.0)
}

/// The stabilizer's options, with Krita's defaults.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StabilizerSettings {
    pub enabled: bool,
    /// Number of 1 ms samples averaged (Krita's "Distance"), 3..=1000.
    pub distance: f32,
    pub delay: bool,
    /// Dead-zone radius in screen pixels (Krita's "Delay" distance).
    pub delay_distance: f32,
    pub finish_line: bool,
    /// Average pressure/tilt/rotation too (Krita's "Stabilize sensors").
    pub stabilize_sensors: bool,
    /// Divide distances by the zoom (Krita's "Scalable distance").
    pub scalable_distance: bool,
}

impl Default for StabilizerSettings {
    fn default() -> Self {
        StabilizerSettings {
            enabled: false,
            distance: 50.0,
            delay: false,
            delay_distance: 50.0,
            finish_line: true,
            stabilize_sensors: true,
            scalable_distance: true,
        }
    }
}

/// The window, in ms, of the strongest stability (100): about as heavy as Clip Studio's 100.
const STRONGEST: f32 = 400.0;
/// The window of the weakest stability above 0 (1).
const WEAKEST: f32 = 3.0;

impl StabilizerSettings {
    /// One number for how smooth lines are, 0 (off) to 100 (strongest), like the
    /// "Stabilization" slider of other painting apps. Steps are even in feel: each one
    /// lengthens the averaging window by the same factor, so low settings stay fine-grained.
    pub fn stability(&self) -> u8 {
        if !self.enabled {
            return 0;
        }
        let t = (self.distance.max(WEAKEST) / WEAKEST).ln() / (STRONGEST / WEAKEST).ln();
        (t * 100.0).round().clamp(1.0, 100.0) as u8
    }

    /// Sets [`StabilizerSettings::stability`]. Above 0 the line always finishes where the pen
    /// lifts, and the smoothing is the same at every zoom (a window of time, as the pen
    /// moves on the screen).
    pub fn set_stability(&mut self, v: u8) {
        let v = v.min(100);
        self.enabled = v > 0;
        if v > 0 {
            self.distance = WEAKEST * (STRONGEST / WEAKEST).powf(v as f32 / 100.0);
            self.finish_line = true;
            self.scalable_distance = false;
        }
    }
}

/// One stroke's stabilizer.
#[derive(Clone, Debug)]
pub struct Stabilizer {
    settings: StabilizerSettings,
    /// Screen pixels per image pixel.
    zoom: f64,
    queue: VecDeque<PaintInfo>,
    /// Events that arrived since the last poll.
    events: Vec<PaintInfo>,
    /// The newest event ever received (repeated while the pen does not move).
    last_event: PaintInfo,
    /// Time already turned into samples, in µs on the caller's clock.
    sampled_until_us: u64,
    /// The last point painted (Krita's `previousPaintInformation`).
    last_painted: PaintInfo,
}

impl Stabilizer {
    /// Starts a stroke at `first`. The caller paints `first` itself (the stroke's first dab).
    pub fn start(first: PaintInfo, now_us: u64, settings: StabilizerSettings, zoom: f64) -> Stabilizer {
        let zoom = if zoom.is_finite() && zoom > 0.0 { zoom } else { 1.0 };
        let mut s = Stabilizer {
            settings,
            zoom,
            queue: VecDeque::new(),
            events: Vec::new(),
            last_event: first,
            sampled_until_us: now_us,
            last_painted: first,
        };
        let n = s.sample_count();
        s.queue.extend(std::iter::repeat_n(first, n));
        s
    }

    /// The number of samples averaged (Krita: `max(3, round(effectiveSmoothnessDistance))`).
    pub fn sample_count(&self) -> usize {
        let d = self.settings.distance.max(0.0) as f64;
        let d = if self.settings.scalable_distance { d / self.zoom } else { d };
        (d.round() as usize).clamp(3, 10_000)
    }

    fn delay_radius(&self) -> Option<f64> {
        self.settings.delay.then(|| self.settings.delay_distance.max(0.0) as f64 / self.zoom)
    }

    /// The last painted point (where the brush is).
    pub fn brush_position(&self) -> PaintInfo {
        self.last_painted
    }

    /// The newest pen position received.
    pub fn pen_position(&self) -> PaintInfo {
        self.last_event
    }

    /// Delay dead-zone radius in image pixels, if the delay is on.
    pub fn leash_radius(&self) -> Option<f64> {
        self.delay_radius()
    }

    /// A pen event (in order of arrival).
    pub fn add(&mut self, info: PaintInfo) {
        self.events.push(info);
        self.last_event = info;
    }

    /// Turns the time since the last poll into samples and returns the points to paint, in
    /// order (each to be joined to the previous painted point). Call regularly while the
    /// stroke lasts (every frame), even without new events.
    pub fn poll(&mut self, now_us: u64) -> Vec<PaintInfo> {
        let elapsed_ms = now_us.saturating_sub(self.sampled_until_us) / 1000;
        if elapsed_ms == 0 {
            // Keep the events for the next poll rather than dropping them.
            return Vec::new();
        }
        self.sampled_until_us += elapsed_ms * 1000;
        // A stalled app can come back after seconds; beyond the queue length more samples
        // of the same held position change nothing, so cap the work.
        let samples = elapsed_ms.min((self.queue.len() as u64 + 1).max(self.events.len() as u64) * 4) as usize;
        self.run(samples)
    }

    /// Ends the stroke: with "finish line" on, the line catches up with the last pen
    /// position. Returns the remaining points to paint.
    pub fn finish(&mut self, now_us: u64) -> Vec<PaintInfo> {
        let mut out = self.poll(now_us);
        // Events not yet sampled (poll was within the same millisecond) still count.
        if !self.events.is_empty() {
            let n = self.events.len();
            out.extend(self.run(n));
        }
        if self.settings.finish_line {
            let n = self.queue.len();
            self.events.clear();
            self.events.push(self.last_event);
            out.extend(self.run(n));
        }
        out
    }

    /// Produces `samples` samples from the pending events and paints them.
    fn run(&mut self, samples: usize) -> Vec<PaintInfo> {
        let mut out = Vec::new();
        let n = self.events.len();
        for i in 0..samples {
            // Krita's KisStabilizedEventsSampler: event floor(i * n / samples), or the last
            // event when none arrived. (Inside the loop, samples >= 1.)
            let k = i * n / samples;
            let sample = if k < n { self.events[k] } else { self.last_event };
            let mut paint = true;
            if let Some(r) = self.delay_radius() {
                let d = (sample.x - self.last_painted.x).hypot(sample.y - self.last_painted.y);
                if d <= r {
                    paint = false;
                }
            }
            if paint {
                let p = self.stabilized(&sample);
                out.push(p);
                self.last_painted = p;
                self.queue.pop_front();
                self.queue.push_back(sample);
            } else if self.queue.front().is_some_and(|h| !h.same_pos(&self.last_painted)) {
                for q in self.queue.iter_mut() {
                    *q = self.last_painted;
                }
            }
        }
        self.events.clear();
        out
    }

    /// Average of `newest` and the queue without its oldest entry (Krita's
    /// `getStabilizedPaintInfo`).
    fn stabilized(&self, newest: &PaintInfo) -> PaintInfo {
        let mut result = *newest;
        for (j, q) in self.queue.iter().skip(1).enumerate() {
            let i = (j + 2) as f64;
            let k = (i - 1.0) / i;
            if self.settings.stabilize_sensors {
                result.mix_all(k, q);
            } else {
                result.mix_position(k, q);
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(distance: f32) -> StabilizerSettings {
        StabilizerSettings { enabled: true, distance, ..Default::default() }
    }

    /// Feeds events at `rate_hz` along `path(t)` for `duration_ms`, polling every
    /// `poll_ms`, then finishes. Returns all painted points.
    fn run_stroke(
        s: &mut Stabilizer,
        start_us: u64,
        duration_ms: u64,
        rate_hz: u64,
        poll_ms: u64,
        path: impl Fn(f64) -> PaintInfo,
    ) -> (Vec<PaintInfo>, u64) {
        let mut out = Vec::new();
        let step_us = 1_000_000 / rate_hz;
        let mut t = start_us;
        let mut next_poll = start_us + poll_ms * 1000;
        let end = start_us + duration_ms * 1000;
        while t < end {
            t += step_us;
            s.add(path((t - start_us) as f64 / 1e6));
            if t >= next_poll {
                out.extend(s.poll(t));
                next_poll += poll_ms * 1000;
            }
        }
        out.extend(s.finish(t));
        (out, t)
    }

    #[test]
    fn queue_starts_full_of_the_first_point() {
        let s = Stabilizer::start(PaintInfo::new(10.0, 20.0, 0.5), 0, settings(50.0), 1.0);
        assert_eq!(s.sample_count(), 50);
        assert_eq!(s.queue.len(), 50);
        assert!(s.queue.iter().all(|q| q.same_pos(&PaintInfo::new(10.0, 20.0, 0.5))));
    }

    #[test]
    fn distance_has_a_minimum_and_scales_with_zoom() {
        let s = Stabilizer::start(PaintInfo::default(), 0, settings(1.0), 1.0);
        assert_eq!(s.sample_count(), 3);
        let s = Stabilizer::start(PaintInfo::default(), 0, settings(50.0), 2.0);
        assert_eq!(s.sample_count(), 25);
        let fixed = StabilizerSettings { scalable_distance: false, ..settings(50.0) };
        let s = Stabilizer::start(PaintInfo::default(), 0, fixed, 2.0);
        assert_eq!(s.sample_count(), 50);
    }

    #[test]
    fn one_sample_per_millisecond_and_events_spread_evenly() {
        let mut s = Stabilizer::start(PaintInfo::new(0.0, 0.0, 1.0), 0, settings(3.0), 1.0);
        s.add(PaintInfo::new(10.0, 0.0, 1.0));
        s.add(PaintInfo::new(20.0, 0.0, 1.0));
        // 4 ms elapsed, 2 events: samples are e0, e0, e1, e1.
        let out = s.poll(4_000);
        assert_eq!(out.len(), 4);
        let xs: Vec<f64> = s.queue.iter().map(|q| q.x).collect();
        assert_eq!(xs, vec![10.0, 20.0, 20.0]);
        // Within the same millisecond nothing happens and no event is lost.
        s.add(PaintInfo::new(30.0, 0.0, 1.0));
        assert!(s.poll(4_500).is_empty());
        let out = s.poll(5_000);
        assert_eq!(out.len(), 1);
        assert_eq!(s.queue.back().unwrap().x, 30.0);
    }

    #[test]
    fn painted_point_is_the_average_of_the_window() {
        // distance 3: average of the new sample and the 2 newest queued ones.
        let mut s = Stabilizer::start(PaintInfo::new(0.0, 0.0, 1.0), 0, settings(3.0), 1.0);
        s.add(PaintInfo::new(9.0, 0.0, 1.0));
        let out = s.poll(1_000);
        // queue was [0,0,0]; average of 9, 0, 0.
        assert!((out[0].x - 3.0).abs() < 1e-12, "{}", out[0].x);
        // Pen held at 9: next samples average (9, 0, 9) then (9, 9, 9).
        let out = s.poll(3_000);
        assert!((out[0].x - 6.0).abs() < 1e-12);
        assert!((out[1].x - 9.0).abs() < 1e-12);
    }

    #[test]
    fn a_held_pen_is_caught_up_with() {
        let mut s = Stabilizer::start(PaintInfo::new(0.0, 0.0, 1.0), 0, settings(50.0), 1.0);
        s.add(PaintInfo::new(100.0, 50.0, 1.0));
        let out = s.poll(200_000); // 200 ms later
        let last = out.last().unwrap();
        assert!((last.x - 100.0).abs() < 1e-9 && (last.y - 50.0).abs() < 1e-9);
    }

    #[test]
    fn straight_lines_stay_straight() {
        let mut s = Stabilizer::start(PaintInfo::new(0.0, 100.0, 1.0), 0, settings(50.0), 1.0);
        let (out, _) = run_stroke(&mut s, 0, 500, 200, 15, |t| PaintInfo::new(t * 1000.0, 100.0, 1.0));
        assert!(out.iter().all(|p| (p.y - 100.0).abs() < 1e-9));
        // x never goes backwards.
        for w in out.windows(2) {
            assert!(w[1].x >= w[0].x - 1e-9);
        }
    }

    #[test]
    fn jitter_is_smoothed() {
        // A horizontal line with ±3 px of alternating jitter, like a shaky hand.
        let jitter = |t: f64| {
            let i = (t * 200.0).round() as i64;
            PaintInfo::new(t * 600.0, 100.0 + if i % 2 == 0 { 3.0 } else { -3.0 }, 1.0)
        };
        let mut s = Stabilizer::start(jitter(0.0), 0, settings(50.0), 1.0);
        let (out, _) = run_stroke(&mut s, 0, 600, 200, 15, jitter);
        // Away from the ends, the painted line stays within 0.5 px of the true line.
        let mid: Vec<&PaintInfo> = out.iter().filter(|p| p.x > 60.0 && p.x < 300.0).collect();
        assert!(mid.len() > 50);
        let worst = mid.iter().map(|p| (p.y - 100.0).abs()).fold(0.0, f64::max);
        assert!(worst < 0.5, "worst {worst}");
    }

    #[test]
    fn stability_numbers_read_back_and_only_strengthen() {
        let mut s = StabilizerSettings::default();
        s.set_stability(0);
        assert!(!s.enabled);
        assert_eq!(s.stability(), 0);
        let mut last = 0.0;
        for v in 1..=100u8 {
            s.set_stability(v);
            assert!(s.enabled && s.finish_line && !s.scalable_distance);
            assert_eq!(s.stability(), v, "reads back");
            assert!(s.distance > last, "{v}: each step is stronger");
            last = s.distance;
        }
        assert!((s.distance - STRONGEST).abs() < 0.01);
    }

    /// How the slider feels, on a shaky hand: a horizontal line at 600 px/s with random
    /// ±3 px wobble, the pen at 200 readings/s. Each step up gives a smoother line; the line
    /// always ends where the pen lifted; low settings barely trail the pen, high ones trail it
    /// more (as in other apps).
    #[test]
    fn the_stability_slider_smooths_more_at_each_step_and_ends_at_the_pen() {
        let shaky = |t: f64| {
            // Deterministic noise in -1..1, a new value every reading.
            let i = (t * 200.0).round() as u64;
            let h = (i.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407) >> 33) as f64 / (1u64 << 31) as f64;
            PaintInfo::new(t * 600.0, 100.0 + 3.0 * (2.0 * h - 1.0), 1.0)
        };
        let mut previous_wobble = 3.0f64;
        let mut lags = Vec::new();
        for v in [10u8, 30, 50, 70, 100] {
            let mut set = StabilizerSettings::default();
            set.set_stability(v);
            let mut s = Stabilizer::start(shaky(0.0), 0, set, 1.0);
            // The drawing part: how far the line trails the pen in the middle of the stroke.
            let mut out = Vec::new();
            let mut t = 0u64;
            while t < 500_000 {
                t += 5_000;
                s.add(shaky(t as f64 / 1e6));
                if t.is_multiple_of(15_000) {
                    out.extend(s.poll(t));
                }
            }
            lags.push(s.pen_position().x - s.brush_position().x);
            out.extend(s.finish(t));
            let mid: Vec<f64> = out.iter().filter(|p| p.x > 100.0 && p.x < 250.0).map(|p| (p.y - 100.0).abs()).collect();
            assert!(mid.len() > 20, "{v}: points in the middle");
            let wobble = mid.iter().sum::<f64>() / mid.len() as f64;
            assert!(wobble < previous_wobble, "stability {v}: wobble {wobble:.3} px, not below {previous_wobble:.3}");
            println!("stability {v:3}: wobble {wobble:.2} px, trails the pen by {:.1} px", lags.last().unwrap());
            previous_wobble = wobble;
            let (last, pen) = (out.last().unwrap(), s.pen_position());
            assert!((last.x - pen.x).abs() < 1e-6 && (last.y - pen.y).abs() < 1e-6, "stability {v}: ends where the pen lifted");
        }
        assert!(lags[0] < 5.0, "stability 10 trails by {:.1} px at 600 px/s", lags[0]);
        assert!(lags.windows(2).all(|w| w[1] > w[0]), "stronger trails more: {lags:?}");
        assert!(lags[4] > 50.0, "stability 100 is heavy: {:.1} px", lags[4]);
    }

    #[test]
    fn finish_line_reaches_the_last_pen_position() {
        let mut s = Stabilizer::start(PaintInfo::new(0.0, 0.0, 0.2), 0, settings(80.0), 1.0);
        let (out, _) = run_stroke(&mut s, 0, 300, 200, 15, |t| PaintInfo::new(t * 1000.0, t * 500.0, 0.2 + t as f32));
        let last = out.last().unwrap();
        let pen = s.pen_position();
        assert!((last.x - pen.x).abs() < 1e-6 && (last.y - pen.y).abs() < 1e-6, "{last:?} vs {pen:?}");
    }

    #[test]
    fn without_finish_line_the_end_lags_behind() {
        let st = StabilizerSettings { finish_line: false, ..settings(80.0) };
        let mut s = Stabilizer::start(PaintInfo::new(0.0, 0.0, 1.0), 0, st, 1.0);
        let (out, _) = run_stroke(&mut s, 0, 300, 200, 15, |t| PaintInfo::new(t * 1000.0, 0.0, 1.0));
        let last = out.last().unwrap();
        assert!(s.pen_position().x - last.x > 20.0, "{} vs {}", last.x, s.pen_position().x);
    }

    #[test]
    fn delay_holds_the_brush_inside_the_dead_zone() {
        let st = StabilizerSettings { delay: true, delay_distance: 20.0, ..settings(10.0) };
        let mut s = Stabilizer::start(PaintInfo::new(0.0, 0.0, 1.0), 0, st, 1.0);
        // Small wiggles inside 20 px paint nothing.
        for (i, x) in [5.0, -8.0, 12.0, 0.0, 15.0].into_iter().enumerate() {
            s.add(PaintInfo::new(x, 3.0, 1.0));
            assert!(s.poll((i as u64 + 1) * 10_000).is_empty());
        }
        // Moving out pulls the brush along, staying about 20 px behind the pen.
        s.add(PaintInfo::new(100.0, 0.0, 1.0));
        let out = s.poll(500_000);
        assert!(!out.is_empty());
        let gap = 100.0 - s.brush_position().x;
        assert!(gap > 0.0 && gap <= 20.0 + 1e-9, "gap {gap}");
    }

    #[test]
    fn delay_scales_with_zoom() {
        let st = StabilizerSettings { delay: true, delay_distance: 20.0, ..settings(10.0) };
        let s = Stabilizer::start(PaintInfo::default(), 0, st, 4.0);
        assert_eq!(s.leash_radius(), Some(5.0));
    }

    #[test]
    fn sensors_are_averaged_only_when_asked() {
        let mut on = Stabilizer::start(PaintInfo::new(0.0, 0.0, 0.0), 0, settings(4.0), 1.0);
        on.add(PaintInfo::new(4.0, 0.0, 1.0));
        let p = on.poll(1_000)[0];
        assert!((p.pressure - 0.25).abs() < 1e-6, "{}", p.pressure);
        let off_settings = StabilizerSettings { stabilize_sensors: false, ..settings(4.0) };
        let mut off = Stabilizer::start(PaintInfo::new(0.0, 0.0, 0.0), 0, off_settings, 1.0);
        off.add(PaintInfo::new(4.0, 0.0, 1.0));
        let p = off.poll(1_000)[0];
        assert_eq!(p.pressure, 1.0);
        assert!((p.x - 1.0).abs() < 1e-12);
    }

    #[test]
    fn rotation_averages_the_short_way_round() {
        assert!((mix_angle_deg(350.0, 10.0, 0.5) - 0.0).abs() < 1e-4 || (mix_angle_deg(350.0, 10.0, 0.5) - 360.0).abs() < 1e-4);
        assert!((mix_angle_deg(90.0, 0.0, 0.5) - 45.0).abs() < 1e-4);
    }

    #[test]
    fn polling_rate_does_not_change_the_result() {
        // Samples are per millisecond, so polling every 1 ms or every 16 ms paints the
        // same line (only grouped differently).
        let path = |t: f64| PaintInfo::new(t * 800.0, (t * 9.0).sin() * 80.0, 0.5);
        let mut a = Stabilizer::start(path(0.0), 0, settings(40.0), 1.0);
        let mut b = Stabilizer::start(path(0.0), 0, settings(40.0), 1.0);
        let (pa, _) = run_stroke(&mut a, 0, 400, 1000, 1, path);
        let (pb, _) = run_stroke(&mut b, 0, 400, 1000, 16, path);
        assert_eq!(pa.len(), pb.len());
        for (x, y) in pa.iter().zip(&pb) {
            assert!((x.x - y.x).abs() < 1e-9 && (x.y - y.y).abs() < 1e-9);
        }
    }

    #[test]
    fn a_long_stall_is_bounded() {
        let mut s = Stabilizer::start(PaintInfo::new(0.0, 0.0, 1.0), 0, settings(50.0), 1.0);
        s.add(PaintInfo::new(10.0, 0.0, 1.0));
        let out = s.poll(60_000_000); // the app froze for a minute
        assert!(out.len() <= 51 * 4, "{}", out.len());
        assert!((out.last().unwrap().x - 10.0).abs() < 1e-9);
    }
}
