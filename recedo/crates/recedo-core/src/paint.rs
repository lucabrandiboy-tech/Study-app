//! One brush stroke from pen-down to pen-up: pen points go through the stabilizer (if on)
//! into the brush, the layer's changed tiles are collected for the screen, and the stroke
//! is one undo step. This is what the canvas tool calls.

use std::collections::HashSet;

use crate::brush::{Brush, StrokePainter};
use crate::doc::Document;
use crate::history::History;
use crate::stabilizer::{PaintInfo, Stabilizer, StabilizerSettings};
use crate::tiles::TileKey;

pub struct PaintSession {
    layer: usize,
    painter: StrokePainter,
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
            painter: StrokePainter::new(brush, color, erase),
            stabilizer: stabilizer.enabled.then(|| Stabilizer::start(first, now_us, stabilizer, zoom)),
            dirty: HashSet::new(),
        };
        s.paint(doc, first);
        Ok(s)
    }

    fn paint(&mut self, doc: &mut Document, p: PaintInfo) {
        if let Some(l) = doc.layers.get_mut(self.layer) {
            self.painter.add(&mut l.pixels, p, &mut self.dirty);
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

    /// Where the stabilizer's leash is drawn: (brush, pen, radius), if the delay is on.
    pub fn leash(&self) -> Option<(PaintInfo, PaintInfo, f64)> {
        let st = self.stabilizer.as_ref()?;
        Some((st.brush_position(), st.pen_position(), st.leash_radius()?))
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
        let mut s = PaintSession::begin(&mut doc, &mut history, 1, brush, [0, 0, 0], false, StabilizerSettings::default(), 1.0, pt(10.0, 64.0), 0).unwrap();
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
        let stab = StabilizerSettings { enabled: true, distance: 40.0, ..Default::default() };
        let brush = Brush { size: 4.0, size_curve: None, ..Default::default() };
        let mut s = PaintSession::begin(&mut doc, &mut history, 0, brush, [0, 0, 0], false, stab, 1.0, pt(10.0, 50.0), 0).unwrap();
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
            PaintSession::begin(doc, h, layer, Brush::default(), [0; 3], false, StabilizerSettings::default(), 1.0, pt(5.0, 5.0), 0).map(|_| ())
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
}
