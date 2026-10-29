//! One brush stroke from pen-down to pen-up: pen points go through the stabilizer (if on)
//! into the brush, the layer's changed tiles are collected for the screen, and the stroke
//! is one undo step. This is what the canvas tool calls.

use std::collections::HashSet;

use crate::brush::{Brush, StrokePainter};
use crate::doc::Document;
use crate::history::History;
use crate::stabilizer::{PaintInfo, Stabilizer, StabilizerSettings};
use crate::tiles::TileKey;

/// Symmetric painting, in canvas pixels. Without spokes: `x` mirrors left and right about the
/// vertical line through `centre`, `y` top and bottom about the horizontal one (1, 2 or 4
/// copies). With `spokes` n > 1: n copies turned round `centre`, and with `x` (or `y`) each
/// is mirrored too, like a kaleidoscope (2n copies).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Symmetry {
    pub centre: (f64, f64),
    pub x: bool,
    pub y: bool,
    pub spokes: u32,
}

impl Default for Symmetry {
    fn default() -> Symmetry {
        Symmetry { centre: (0.0, 0.0), x: false, y: false, spokes: 1 }
    }
}

impl Symmetry {
    /// `p` and its copies (`p` first). Tilt and rotation are mirrored and turned with it, so a
    /// tilted pen leans the mirrored or turned way.
    pub fn copies(&self, p: PaintInfo) -> Vec<PaintInfo> {
        let (cx, cy) = self.centre;
        let mirror_x = |q: PaintInfo| PaintInfo { x: 2.0 * cx - q.x, tilt_x: -q.tilt_x, rotation: (360.0 - q.rotation).rem_euclid(360.0), ..q };
        let mirror_y = |q: PaintInfo| PaintInfo { y: 2.0 * cy - q.y, tilt_y: -q.tilt_y, rotation: (180.0 - q.rotation).rem_euclid(360.0), ..q };
        let mut out = vec![p];
        if self.spokes > 1 {
            if self.x {
                out.push(mirror_x(p));
            } else if self.y {
                out.push(mirror_y(p));
            }
            let base = out.clone();
            for k in 1..self.spokes {
                let turn = k as f64 * 360.0 / self.spokes as f64;
                let (s, c) = turn.to_radians().sin_cos();
                for q in &base {
                    // Clockwise on screen (y points down).
                    let (dx, dy) = (q.x - cx, q.y - cy);
                    let (tx, ty) = (q.tilt_x as f64, q.tilt_y as f64);
                    out.push(PaintInfo {
                        x: cx + dx * c - dy * s,
                        y: cy + dx * s + dy * c,
                        tilt_x: (tx * c - ty * s) as f32,
                        tilt_y: (tx * s + ty * c) as f32,
                        rotation: ((q.rotation as f64 + turn) % 360.0) as f32,
                        ..*q
                    });
                }
            }
            return out;
        }
        if self.x {
            out.push(mirror_x(p));
        }
        if self.y {
            let n = out.len();
            for i in 0..n {
                out.push(mirror_y(out[i]));
            }
        }
        out
    }

    pub fn count(&self) -> usize {
        if self.spokes > 1 {
            self.spokes as usize * (1 + (self.x || self.y) as usize)
        } else {
            (1 + self.x as usize) * (1 + self.y as usize)
        }
    }
}

pub struct PaintSession {
    layer: usize,
    /// The brush, painting every copy (symmetry) as one stroke.
    painter: StrokePainter,
    symmetry: Symmetry,
    stabilizer: Option<Stabilizer>,
    dirty: HashSet<TileKey>,
}

impl PaintSession {
    /// The layer this stroke paints on.
    pub fn layer(&self) -> usize {
        self.layer
    }

    /// Starts a stroke on `layer` at `first`. Records the undo step. Refuses (with a message
    /// for the user) on a locked or hidden layer, as Krita does.
    #[allow(clippy::too_many_arguments)]
    pub fn begin(
        doc: &mut Document,
        history: &mut History,
        layer: usize,
        brush: Brush,
        color: [u8; 3],
        erase: bool,
        stabilizer: StabilizerSettings,
        symmetry: Symmetry,
        zoom: f64,
        first: PaintInfo,
        now_us: u64,
    ) -> Result<PaintSession, String> {
        let l = doc.layers.get(layer).ok_or("No layer is selected")?;
        if l.locked {
            return Err(format!("Layer \"{}\" is locked", l.name));
        }
        if !l.visible {
            return Err(format!("Layer \"{}\" is hidden", l.name));
        }
        history.record(if erase { "Erase" } else { "Brush stroke" }, doc);
        let mut s = PaintSession {
            layer,
            painter: StrokePainter::with_copies(brush, color, erase, symmetry.count()),
            symmetry,
            stabilizer: stabilizer.enabled.then(|| Stabilizer::start(first, now_us, stabilizer, zoom)),
            dirty: HashSet::new(),
        };
        s.paint(doc, first);
        Ok(s)
    }

    fn paint(&mut self, doc: &mut Document, p: PaintInfo) {
        if let Some(l) = doc.layers.get_mut(self.layer) {
            self.painter.add_copies(&mut l.pixels, &self.symmetry.copies(p), &mut self.dirty);
        }
    }

    /// A pen point while the pen is down.
    pub fn add(&mut self, doc: &mut Document, p: PaintInfo, now_us: u64) {
        match &mut self.stabilizer {
            Some(st) => {
                st.add(p);
                self.tick(doc, now_us);
            }
            None => self.paint(doc, p),
        }
    }

    /// Call every frame while the stroke lasts: the stabilizer keeps moving the brush
    /// toward a pen that is held still.
    pub fn tick(&mut self, doc: &mut Document, now_us: u64) {
        let points = match &mut self.stabilizer {
            Some(st) => st.poll(now_us),
            None => return,
        };
        for p in points {
            self.paint(doc, p);
        }
    }

    /// Pen up: the stabilizer's finish line is drawn. Returns the tiles still to refresh.
    pub fn end(mut self, doc: &mut Document, now_us: u64) -> HashSet<TileKey> {
        if let Some(st) = &mut self.stabilizer {
            for p in st.finish(now_us) {
                self.paint(doc, p);
            }
        }
        self.dirty
    }

    /// Tiles changed since the last call (to recomposite and upload this frame).
    pub fn take_dirty(&mut self) -> HashSet<TileKey> {
        std::mem::take(&mut self.dirty)
    }

    /// Where the stabilizer's string is drawn, while one smooths the stroke: (brush, pen,
    /// dead-zone radius, 0 without one).
    pub fn leash(&self) -> Option<(PaintInfo, PaintInfo, f64)> {
        let st = self.stabilizer.as_ref()?;
        Some((st.brush_position(), st.pen_position(), st.leash_radius().unwrap_or(0.0)))
    }

    pub fn dabs_painted(&self) -> usize {
        self.painter.dabs_painted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::Projection;

    fn pt(x: f64, y: f64) -> PaintInfo {
        PaintInfo::new(x, y, 1.0)
    }

    #[test]
    fn a_stroke_paints_updates_the_screen_and_undoes() {
        let mut doc = Document::new(256, 128, Some([255; 4]));
        let mut history = History::default();
        let mut proj = Projection::new(&doc);
        let before = doc.clone();
        let brush = Brush { size: 8.0, size_curve: None, ..Default::default() };
        let mut s = PaintSession::begin(&mut doc, &mut history, 1, brush, [0, 0, 0], false, StabilizerSettings::default(), Symmetry::default(), 1.0, pt(10.0, 64.0), 0).unwrap();
        for i in 1..=40 {
            s.add(&mut doc, pt(10.0 + i as f64 * 5.0, 64.0), i * 5000);
            let dirty = s.take_dirty();
            proj.update(&doc, &dirty);
        }
        let rest = s.end(&mut doc, 205_000);
        proj.update(&doc, &rest);
        assert_eq!(proj.image, doc.flatten(), "the screen shows exactly the painting");
        assert_eq!(doc.flatten().pixel(100, 64), [0, 0, 0, 255]);
        assert_eq!(history.entries(), (vec!["Brush stroke"], 1));
        history.undo(&mut doc);
        assert_eq!(doc, before);
    }

    #[test]
    fn stabilized_stroke_needs_ticks_and_finishes_at_the_pen() {
        let mut doc = Document::new(300, 100, None);
        let mut history = History::default();
        let stab = StabilizerSettings { enabled: true, distance: 40.0, mode: crate::stabilizer::SmoothMode::Average, ..Default::default() };
        let brush = Brush { size: 4.0, size_curve: None, ..Default::default() };
        let mut s = PaintSession::begin(&mut doc, &mut history, 0, brush, [0, 0, 0], false, stab, Symmetry::default(), 1.0, pt(10.0, 50.0), 0).unwrap();
        // The pen jumps to x = 250 and is held there.
        s.add(&mut doc, pt(250.0, 50.0), 1000);
        let early = doc.layers[0].pixels.pixel(240, 50)[3];
        assert_eq!(early, 0, "the brush lags behind the pen");
        for t in 2..200u64 {
            s.tick(&mut doc, t * 1000);
        }
        assert!(doc.layers[0].pixels.pixel(200, 50)[3] > 0, "held still, the brush catches up");
        s.end(&mut doc, 200_000);
        assert!(doc.layers[0].pixels.pixel(249, 50)[3] > 0, "finish line reaches the pen");
    }

    #[test]
    fn locked_hidden_and_missing_layers_refuse() {
        let mut doc = Document::new(64, 64, None);
        let mut h = History::default();
        let go = |doc: &mut Document, h: &mut History, layer| {
            PaintSession::begin(doc, h, layer, Brush::default(), [0; 3], false, StabilizerSettings::default(), Symmetry::default(), 1.0, pt(5.0, 5.0), 0).map(|_| ())
        };
        doc.layers[0].locked = true;
        assert!(go(&mut doc, &mut h, 0).unwrap_err().contains("locked"));
        doc.layers[0].locked = false;
        doc.layers[0].visible = false;
        assert!(go(&mut doc, &mut h, 0).unwrap_err().contains("hidden"));
        assert!(go(&mut doc, &mut h, 7).is_err());
        assert!(!h.can_undo(), "refused strokes leave no undo step");
        assert_eq!(doc.layers[0].pixels.tile_count(), 0);
    }

    #[test]
    fn symmetry_mirrors_points_and_strokes() {
        let p = PaintInfo { x: 10.0, y: 20.0, pressure: 0.5, tilt_x: 30.0, tilt_y: 10.0, rotation: 40.0 };
        let both = Symmetry { centre: (50.0, 100.0), x: true, y: true, spokes: 1 }.copies(p);
        let xy: Vec<(f64, f64)> = both.iter().map(|q| (q.x, q.y)).collect();
        assert_eq!(xy, vec![(10.0, 20.0), (90.0, 20.0), (10.0, 180.0), (90.0, 180.0)]);
        assert_eq!((both[1].tilt_x, both[2].tilt_y), (-30.0, -10.0), "a tilted pen leans the mirrored way");
        assert_eq!(Symmetry::default().copies(p).len(), 1);
        // A stroke with a vertical mirror line at x = 64 paints its mirror image exactly.
        let mut doc = Document::new(128, 64, None);
        let mut history = History::new(10);
        let brush = Brush { size: 6.0, hardness: 1.0, size_curve: None, ..Brush::default() };
        let sym = Symmetry { centre: (64.0, 32.0), x: true, ..Symmetry::default() };
        let mut s = PaintSession::begin(&mut doc, &mut history, 0, brush, [0, 0, 0], false, StabilizerSettings::default(), sym, 1.0, pt(10.0, 20.0), 0).unwrap();
        for i in 1..=20 {
            s.add(&mut doc, pt(10.0 + i as f64, 20.0 + i as f64), i * 1000);
        }
        s.end(&mut doc, 30_000);
        let l = &doc.layers[0].pixels;
        let painted = (0..64).flat_map(|y| (0..128).map(move |x| (x, y))).filter(|&(x, y)| l.pixel(x, y)[3] > 0).count();
        assert!(painted > 100);
        for y in 0..64 {
            for x in 0..64 {
                assert_eq!(l.pixel(x, y), l.pixel(127 - x, y), "{x},{y} mirrors {},{y}", 127 - x);
            }
        }
    }

    #[test]
    fn spokes_turn_copies_round_the_centre() {
        let p = PaintInfo { x: 110.0, y: 100.0, pressure: 0.7, tilt_x: 20.0, tilt_y: 0.0, rotation: 10.0 };
        let four = Symmetry { centre: (100.0, 100.0), spokes: 4, ..Symmetry::default() };
        let c = four.copies(p);
        assert_eq!(c.len(), 4);
        assert_eq!(four.count(), 4);
        let round = |v: f64| (v * 1e6).round() / 1e6;
        let xy: Vec<(f64, f64)> = c.iter().map(|q| (round(q.x), round(q.y))).collect();
        // Clockwise on screen: right, below, left, above.
        assert_eq!(xy, vec![(110.0, 100.0), (100.0, 110.0), (90.0, 100.0), (100.0, 90.0)]);
        assert!((c[1].tilt_x.abs() < 1e-4) && (c[1].tilt_y - 20.0).abs() < 1e-4, "the pen's lean turns with it: {:?}", c[1]);
        assert_eq!((c[1].rotation, c[1].pressure), (100.0, 0.7));
        // A kaleidoscope: each spoke mirrored too.
        let snow = Symmetry { centre: (100.0, 100.0), x: true, spokes: 6, ..Symmetry::default() };
        assert_eq!((snow.copies(p).len(), snow.count()), (12, 12));
        let q = PaintInfo { x: 100.0 + 10.0 * 30f64.to_radians().cos(), y: 100.0 - 10.0 * 30f64.to_radians().sin(), ..p };
        let pts = snow.copies(q);
        assert!(pts.iter().all(|r| ((r.x - 100.0).hypot(r.y - 100.0) - 10.0).abs() < 1e-9), "all on the same circle");
        assert!(pts.iter().any(|r| (r.x - (100.0 - (q.x - 100.0))).abs() < 1e-9 && (r.y - q.y).abs() < 1e-9), "its left-right mirror is one of them");
        // Spokes with Y alone mirror top and bottom.
        let y = Symmetry { centre: (100.0, 100.0), y: true, spokes: 3, ..Symmetry::default() };
        assert_eq!(y.copies(q)[1].y, 200.0 - q.y);
    }

    #[test]
    fn a_stroke_with_four_spokes_is_the_same_turned_a_quarter() {
        let mut doc = Document::new(128, 128, None);
        let mut history = History::new(10);
        let brush = Brush { size: 7.0, hardness: 0.8, size_curve: None, ..Brush::default() };
        let sym = Symmetry { centre: (64.0, 64.0), spokes: 4, ..Symmetry::default() };
        let mut s = PaintSession::begin(&mut doc, &mut history, 0, brush, [0, 0, 0], false, StabilizerSettings::default(), sym, 1.0, pt(70.0, 20.0), 0).unwrap();
        for i in 1..=30 {
            s.add(&mut doc, pt(70.0 + i as f64 * 1.3, 20.0 + i as f64 * 0.7), i * 1000);
        }
        s.end(&mut doc, 40_000);
        let l = &doc.layers[0].pixels;
        let (mut painted, mut off) = (0, 0);
        for y in 0..128 {
            for x in 0..128 {
                // Turned a quarter clockwise round (64, 64): (x, y) -> (127 - y, x).
                let (a, b) = (l.pixel(x, y)[3] as i32, l.pixel(127 - y, x)[3] as i32);
                painted += (a > 0) as u32;
                off = off.max((a - b).abs());
            }
        }
        assert!(painted > 400, "four strokes painted: {painted} pixels");
        assert!(off <= 2, "each copy is the first turned (largest difference {off})");
        assert_eq!(l.pixel(64, 64)[3], 0, "nothing in the middle");
    }

    /// A stroke across its own mirror line: the copies overlap and are one stroke (no copy
    /// wipes out what another painted, and Wash never goes past the stroke's opacity).
    #[test]
    fn overlapping_copies_are_one_stroke() {
        for opacity in [1.0f32, 0.5] {
            let mut doc = Document::new(128, 64, None);
            let mut history = History::new(10);
            let brush = Brush { size: 9.0, hardness: 1.0, opacity, size_curve: None, ..Brush::default() };
            let sym = Symmetry { centre: (64.0, 32.0), x: true, ..Symmetry::default() };
            // Zigzagging over the line at x = 64, so each copy keeps coming back over the other.
            let mut s = PaintSession::begin(&mut doc, &mut history, 0, brush.clone(), [0, 0, 0], false, StabilizerSettings::default(), sym, 1.0, pt(40.0, 32.0), 0).unwrap();
            for i in 1..=60 {
                let x = 40.0 + 48.0 * ((i as f64 / 12.0).sin().abs());
                s.add(&mut doc, pt(x, 32.0 + (i % 3) as f64 * 0.5), i * 1000);
            }
            s.end(&mut doc, 70_000);
            let l = &doc.layers[0].pixels;
            let want = (opacity * 255.0).round() as i32;
            for x in 40..88 {
                let a = l.pixel(x, 32)[3] as i32;
                assert!((a - want).abs() <= 2, "opacity {opacity}: x {x} has {a}, the stroke is {want}");
            }
            // Everywhere, at least what each copy paints alone (nothing wiped out).
            for flip in [false, true] {
                let mut alone = Document::new(128, 64, None);
                let mut h = History::new(10);
                let at = |x: f64| if flip { 128.0 - x } else { x };
                let mut s = PaintSession::begin(&mut alone, &mut h, 0, brush.clone(), [0, 0, 0], false, StabilizerSettings::default(), Symmetry::default(), 1.0, pt(at(40.0), 32.0), 0).unwrap();
                for i in 1..=60 {
                    let x = 40.0 + 48.0 * ((i as f64 / 12.0).sin().abs());
                    s.add(&mut alone, pt(at(x), 32.0 + (i % 3) as f64 * 0.5), i * 1000);
                }
                s.end(&mut alone, 70_000);
                let one = &alone.layers[0].pixels;
                for y in 0..64 {
                    for x in 0..128 {
                        let (both, a) = (l.pixel(x, y)[3] as i32, one.pixel(x, y)[3] as i32);
                        assert!(both >= a - 2, "opacity {opacity}: {x},{y} has {both}, one copy alone paints {a}");
                    }
                }
            }
        }
    }
}
