//! Splits a sample stream into strokes (pen down … pen up) and turns them into dabs.
//! Shared by the live app and by replay, so a recording renders exactly as it was drawn.
//!
//! With the stabilizer on, samples go through Krita's stabilizer first ([`Stabilizer`]),
//! and [`StrokeTracker::tick`] must be called every frame while a stroke lasts, so the
//! line keeps catching up with the pen even when no new samples arrive.

use recedo_core::{PaintInfo, SmoothMode, Stabilizer, StabilizerSettings};
use recedo_tablet::{Flags, PenSample, Source};

use crate::brush::{BrushSettings, DabBatch, StrokeBuilder, push_dabs};

struct Active {
    source: Source,
    cursor: u32,
    builder: StrokeBuilder,
    stabilizer: Option<Stabilizer>,
    has_tilt: bool,
}

#[derive(Default)]
pub struct StrokeTracker {
    active: Option<Active>,
    pub strokes_started: usize,
    pub dabs_made: usize,
    /// Applied to strokes that start from now on.
    pub stabilizer: StabilizerSettings,
    scratch: Vec<crate::brush::Dab>,
}

/// A sample as the stabilizer sees it (pressure 1 when the device reports none).
fn paint_info(s: &PenSample) -> PaintInfo {
    PaintInfo {
        x: s.x,
        y: s.y,
        pressure: if s.flags.has(Flags::HAS_PRESSURE) { s.pressure } else { 1.0 },
        tilt_x: s.tilt_x,
        tilt_y: s.tilt_y,
        rotation: s.twist,
    }
}

impl Active {
    fn paint(&mut self, points: &[PaintInfo], scratch: &mut Vec<crate::brush::Dab>, dabs_made: &mut usize, out: &mut Vec<DabBatch>) {
        scratch.clear();
        for p in points {
            self.builder.add_info(p, self.has_tilt, scratch);
        }
        *dabs_made += scratch.len();
        push_dabs(out, self.builder.erase, scratch);
    }
}

impl StrokeTracker {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }

    pub fn active_source(&self) -> Option<Source> {
        self.active.as_ref().map(|a| a.source)
    }

    /// True while a stabilized stroke is in progress (it needs [`StrokeTracker::tick`]).
    pub fn stabilizing(&self) -> bool {
        self.active.as_ref().is_some_and(|a| a.stabilizer.is_some())
    }

    /// For the delay "leash" overlay: brush position, pen position, dead-zone radius.
    pub fn leash(&self) -> Option<(PaintInfo, PaintInfo, f64)> {
        let st = self.active.as_ref()?.stabilizer.as_ref()?;
        Some((st.brush_position(), st.pen_position(), st.leash_radius()?))
    }

    /// Feeds one sample. A stroke starts on contact if `may_start(x, y)` agrees (e.g. the
    /// pen is not on a panel), continues while the same device keeps contact, and ends on
    /// lift-off. While one device draws, the others are ignored.
    pub fn feed(&mut self, s: &PenSample, brush: &BrushSettings, may_start: impl Fn(f64, f64) -> bool, out: &mut Vec<DabBatch>) {
        match &mut self.active {
            Some(a) if a.source == s.source && a.cursor == s.cursor => {
                if s.contact() {
                    match &mut a.stabilizer {
                        Some(st) => {
                            st.add(paint_info(s));
                            let pts = st.poll(s.recv_us);
                            a.paint(&pts, &mut self.scratch, &mut self.dabs_made, out);
                        }
                        None => {
                            self.scratch.clear();
                            a.builder.add(s, &mut self.scratch);
                            self.dabs_made += self.scratch.len();
                            push_dabs(out, a.builder.erase, &self.scratch);
                        }
                    }
                } else {
                    self.finish(s.recv_us, out);
                }
            }
            Some(_) => {}
            None => {
                if s.contact() && may_start(s.x, s.y) {
                    let erase = s.eraser() && brush.eraser_end_erases;
                    let mut builder = StrokeBuilder::new(*brush, erase);
                    self.scratch.clear();
                    builder.add(s, &mut self.scratch);
                    self.dabs_made += self.scratch.len();
                    push_dabs(out, erase, &self.scratch);
                    let stabilizer = self.stabilizer.enabled.then(|| Stabilizer::start(paint_info(s), s.recv_us, StabilizerSettings { mode: SmoothMode::Average, ..self.stabilizer }, 1.0));
                    let has_tilt = s.flags.has(Flags::HAS_TILT);
                    self.active = Some(Active { source: s.source, cursor: s.cursor, builder, stabilizer, has_tilt });
                    self.strokes_started += 1;
                }
            }
        }
    }

    /// Lets a stabilized stroke catch up with the pen. Call every frame.
    pub fn tick(&mut self, now_us: u64, out: &mut Vec<DabBatch>) {
        if let Some(a) = &mut self.active
            && let Some(st) = &mut a.stabilizer
        {
            let pts = st.poll(now_us);
            a.paint(&pts, &mut self.scratch, &mut self.dabs_made, out);
        }
    }

    /// Ends the current stroke normally (the stabilizer's finish line is drawn).
    fn finish(&mut self, now_us: u64, out: &mut Vec<DabBatch>) {
        if let Some(mut a) = self.active.take()
            && let Some(st) = &mut a.stabilizer
        {
            let pts = st.finish(now_us);
            a.paint(&pts, &mut self.scratch, &mut self.dabs_made, out);
        }
    }

    /// Drops the current stroke without finishing it (canvas cleared, API switched).
    pub fn end(&mut self) {
        self.active = None;
    }

    /// The pen of `source` left: its stroke ends as if lifted.
    pub fn end_source(&mut self, source: Source, now_us: u64, out: &mut Vec<DabBatch>) {
        if self.active_source() == Some(source) {
            self.finish(now_us, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use recedo_core::StabilizerSettings;

    fn s(x: f64, contact: bool, src: Source) -> PenSample {
        let mut p = PenSample::synthetic(0, x, 0.0, 1.0, contact);
        p.source = src;
        p
    }

    #[test]
    fn strokes_start_on_contact_and_end_on_lift() {
        let brush = BrushSettings::default();
        let mut t = StrokeTracker::default();
        let mut out = Vec::new();
        for (x, c) in [(0.0, false), (1.0, true), (5.0, true), (9.0, false), (10.0, true), (12.0, true)] {
            t.feed(&s(x, c, Source::Ink), &brush, |_, _| true, &mut out);
        }
        assert_eq!(t.strokes_started, 2);
        assert!(t.is_active());
    }

    #[test]
    fn other_devices_are_ignored_while_drawing() {
        let brush = BrushSettings::default();
        let mut t = StrokeTracker::default();
        let mut out = Vec::new();
        t.feed(&s(0.0, true, Source::Ink), &brush, |_, _| true, &mut out);
        let before = t.dabs_made;
        t.feed(&s(100.0, true, Source::Mouse), &brush, |_, _| true, &mut out);
        assert_eq!(t.dabs_made, before);
        assert_eq!(t.active_source(), Some(Source::Ink));
    }

    #[test]
    fn may_start_blocks_strokes_on_panels() {
        let brush = BrushSettings::default();
        let mut t = StrokeTracker::default();
        let mut out = Vec::new();
        t.feed(&s(10.0, true, Source::Ink), &brush, |x, _| x > 50.0, &mut out);
        assert!(!t.is_active());
        t.feed(&s(60.0, true, Source::Ink), &brush, |x, _| x > 50.0, &mut out);
        assert!(t.is_active());
    }

    fn timed(x: f64, y: f64, t_ms: u64, contact: bool) -> PenSample {
        let mut p = PenSample::synthetic(t_ms * 1000, x, y, 1.0, contact);
        p.source = Source::Ink;
        p
    }

    fn last_dab(out: &[DabBatch]) -> [f32; 2] {
        out.last().unwrap().dabs.last().unwrap().center
    }

    #[test]
    fn stabilized_stroke_ends_at_the_pen_with_finish_line() {
        let brush = BrushSettings { pressure_size: false, ..Default::default() };
        let mut t = StrokeTracker { stabilizer: StabilizerSettings { enabled: true, distance: 80.0, ..Default::default() }, ..Default::default() };
        let mut out = Vec::new();
        for i in 0..=60 {
            t.feed(&timed(i as f64 * 5.0, 100.0, i * 5, true), &brush, |_, _| true, &mut out);
        }
        assert!(t.stabilizing());
        // Lagging behind while drawing...
        assert!(last_dab(&out)[0] < 280.0, "{:?}", last_dab(&out));
        t.feed(&timed(300.0, 100.0, 301, false), &brush, |_, _| true, &mut out);
        assert!(!t.is_active());
        // ...and caught up at the end.
        let end = last_dab(&out);
        assert!((end[0] - 300.0).abs() < 1.0 && (end[1] - 100.0).abs() < 1e-3, "{end:?}");
    }

    #[test]
    fn ticks_catch_up_with_a_pen_held_still() {
        let brush = BrushSettings::default();
        let mut t = StrokeTracker { stabilizer: StabilizerSettings { enabled: true, distance: 50.0, ..Default::default() }, ..Default::default() };
        let mut out = Vec::new();
        t.feed(&timed(0.0, 0.0, 0, true), &brush, |_, _| true, &mut out);
        t.feed(&timed(200.0, 0.0, 1, true), &brush, |_, _| true, &mut out);
        let before = last_dab(&out)[0];
        assert!(before < 50.0, "{before}");
        // No new samples, but frames keep ticking for 200 ms.
        for f in 1..=12 {
            t.tick((1 + f * 16) * 1000, &mut out);
        }
        assert!((last_dab(&out)[0] - 200.0).abs() < 1.0, "{:?}", last_dab(&out));
    }

    #[test]
    fn pen_leaving_range_finishes_the_stroke() {
        let brush = BrushSettings::default();
        let mut t = StrokeTracker { stabilizer: StabilizerSettings { enabled: true, ..Default::default() }, ..Default::default() };
        let mut out = Vec::new();
        t.feed(&timed(0.0, 0.0, 0, true), &brush, |_, _| true, &mut out);
        t.feed(&timed(100.0, 0.0, 10, true), &brush, |_, _| true, &mut out);
        t.end_source(Source::Mouse, 20_000, &mut out);
        assert!(t.is_active(), "another source's leave does nothing");
        t.end_source(Source::Ink, 20_000, &mut out);
        assert!(!t.is_active());
        assert!((last_dab(&out)[0] - 100.0).abs() < 1.0);
    }

    #[test]
    fn without_the_stabilizer_nothing_needs_ticks() {
        let brush = BrushSettings::default();
        let mut t = StrokeTracker::default();
        let mut out = Vec::new();
        t.feed(&timed(0.0, 0.0, 0, true), &brush, |_, _| true, &mut out);
        assert!(t.is_active() && !t.stabilizing());
        assert!(t.leash().is_none());
    }

    #[test]
    fn eraser_end_makes_an_erase_batch() {
        let brush = BrushSettings::default();
        let mut t = StrokeTracker::default();
        let mut out = Vec::new();
        let mut p = s(0.0, true, Source::Wintab);
        p.flags.set(recedo_tablet::Flags::ERASER, true);
        t.feed(&p, &brush, |_, _| true, &mut out);
        assert!(out[0].erase);
        let off = BrushSettings { eraser_end_erases: false, ..brush };
        let mut t = StrokeTracker::default();
        let mut out = Vec::new();
        t.feed(&p, &off, |_, _| true, &mut out);
        assert!(!out[0].erase);
    }
}
