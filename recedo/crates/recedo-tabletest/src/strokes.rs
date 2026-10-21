//! Splits a sample stream into strokes (pen down … pen up) and turns them into dabs.
//! Shared by the live app and by replay, so a recording renders exactly as it was drawn.

use recedo_tablet::{PenSample, Source};

use crate::brush::{BrushSettings, DabBatch, StrokeBuilder, push_dabs};

struct Active {
    source: Source,
    cursor: u32,
    builder: StrokeBuilder,
}

#[derive(Default)]
pub struct StrokeTracker {
    active: Option<Active>,
    pub strokes_started: usize,
    pub dabs_made: usize,
    scratch: Vec<crate::brush::Dab>,
}

impl StrokeTracker {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }

    pub fn active_source(&self) -> Option<Source> {
        self.active.as_ref().map(|a| a.source)
    }

    /// Feeds one sample. A stroke starts on contact if `may_start(x, y)` agrees (e.g. the
    /// pen is not on a panel), continues while the same device keeps contact, and ends on
    /// lift-off. While one device draws, the others are ignored.
    pub fn feed(&mut self, s: &PenSample, brush: &BrushSettings, may_start: impl Fn(f64, f64) -> bool, out: &mut Vec<DabBatch>) {
        match &mut self.active {
            Some(a) if a.source == s.source && a.cursor == s.cursor => {
                if s.contact() {
                    self.scratch.clear();
                    a.builder.add(s, &mut self.scratch);
                    self.dabs_made += self.scratch.len();
                    push_dabs(out, a.builder.erase, &self.scratch);
                } else {
                    self.active = None;
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
                    self.active = Some(Active { source: s.source, cursor: s.cursor, builder });
                    self.strokes_started += 1;
                }
            }
        }
    }

    /// Ends the current stroke (pen left range, input API switched...).
    pub fn end(&mut self) {
        self.active = None;
    }

    /// Ends the stroke only if it belongs to `source`.
    pub fn end_source(&mut self, source: Source) {
        if self.active_source() == Some(source) {
            self.active = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
