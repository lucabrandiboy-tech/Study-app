//! Undo and redo.
//!
//! Each step keeps the whole document as it was before the change. That sounds expensive
//! but is not: layers share their 64 x 64 tiles through `Arc`, so a snapshot only costs a
//! list of tile pointers, and a stroke copies just the tiles it paints on (copy-on-write).
//! The same mechanism undoes everything (strokes, layer changes, canvas changes), so there
//! is no per-action undo code to get wrong.

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use crate::doc::Document;
use crate::tiles::TILE_BYTES;

/// Krita's default undo limit.
pub const DEFAULT_LIMIT: usize = 50;

#[derive(Clone, Debug)]
struct Step {
    name: String,
    /// The document as it was on the other side of this step.
    doc: Document,
}

#[derive(Debug)]
pub struct History {
    undo: VecDeque<Step>,
    redo: Vec<Step>,
    limit: usize,
}

impl Default for History {
    fn default() -> History {
        History::new(DEFAULT_LIMIT)
    }
}

impl History {
    pub fn new(limit: usize) -> History {
        History { undo: VecDeque::new(), redo: Vec::new(), limit: limit.max(1) }
    }

    /// Call with the document as it is *before* a change named `name` (e.g. "Brush stroke").
    /// Clears the redo list.
    pub fn record(&mut self, name: &str, before: &Document) {
        self.redo.clear();
        self.undo.push_back(Step { name: name.to_string(), doc: before.clone() });
        while self.undo.len() > self.limit {
            self.undo.pop_front();
        }
    }

    /// Undoes the last change; returns its name, or `None` if there is nothing to undo.
    pub fn undo(&mut self, doc: &mut Document) -> Option<String> {
        let step = self.undo.pop_back()?;
        let after = std::mem::replace(doc, step.doc);
        self.redo.push(Step { name: step.name.clone(), doc: after });
        Some(step.name)
    }

    /// Redoes the last undone change; returns its name.
    pub fn redo(&mut self, doc: &mut Document) -> Option<String> {
        let step = self.redo.pop()?;
        let before = std::mem::replace(doc, step.doc);
        self.undo.push_back(Step { name: step.name.clone(), doc: before });
        Some(step.name)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// The history list for a History panel: every change's name, oldest first, and how
    /// many of them are currently applied (the rest can be redone).
    pub fn entries(&self) -> (Vec<&str>, usize) {
        let mut names: Vec<&str> = self.undo.iter().map(|s| s.name.as_str()).collect();
        let applied = names.len();
        names.extend(self.redo.iter().rev().map(|s| s.name.as_str()));
        (names, applied)
    }

    /// Undoes or redoes until exactly `applied` changes are applied (clicking a History
    /// panel row).
    pub fn jump_to(&mut self, doc: &mut Document, applied: usize) {
        while self.undo.len() > applied && self.undo(doc).is_some() {}
        while self.undo.len() < applied && self.redo(doc).is_some() {}
    }

    /// Pixel memory held only by the history (tiles the current document no longer uses).
    pub fn memory_bytes(&self, current: &Document) -> usize {
        let live: HashSet<*const _> = tile_ptrs(current).collect();
        let mut held = HashSet::new();
        for s in self.undo.iter().chain(&self.redo) {
            held.extend(tile_ptrs(&s.doc).filter(|p| !live.contains(p)));
        }
        held.len() * TILE_BYTES
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

fn tile_ptrs(doc: &Document) -> impl Iterator<Item = *const crate::tiles::Tile> + '_ {
    doc.layers.iter().flat_map(|l| l.pixels.tile_keys().filter_map(move |k| l.pixels.tile(k).map(Arc::as_ptr)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::Layer;

    fn paint(doc: &mut Document, layer: usize, x0: i32, colour: [u8; 4]) {
        for y in 0..10 {
            for x in x0..x0 + 10 {
                doc.layers[layer].pixels.set_pixel(x, y, colour);
            }
        }
    }

    #[test]
    fn undo_redo_restore_exactly() {
        let mut doc = Document::new(256, 256, Some([255; 4]));
        let mut h = History::default();
        let original = doc.clone();
        h.record("Brush stroke", &doc);
        paint(&mut doc, 1, 0, [255, 0, 0, 255]);
        let one = doc.clone();
        h.record("Brush stroke", &doc);
        paint(&mut doc, 1, 5, [0, 0, 255, 255]);
        let two = doc.clone();
        h.record("Add layer", &doc);
        doc.layers.push(Layer::new("Layer 2"));
        let three = doc.clone();

        assert_eq!(h.undo(&mut doc).as_deref(), Some("Add layer"));
        assert_eq!(doc, two);
        assert_eq!(h.undo(&mut doc).as_deref(), Some("Brush stroke"));
        assert_eq!(doc, one);
        assert_eq!(h.undo(&mut doc).as_deref(), Some("Brush stroke"));
        assert_eq!(doc, original);
        assert_eq!(h.undo(&mut doc), None);
        assert_eq!(doc, original);
        assert_eq!(h.redo(&mut doc).as_deref(), Some("Brush stroke"));
        assert_eq!(h.redo(&mut doc).as_deref(), Some("Brush stroke"));
        assert_eq!(h.redo(&mut doc).as_deref(), Some("Add layer"));
        assert_eq!(doc, three);
        assert_eq!(h.redo(&mut doc), None);
    }

    #[test]
    fn a_new_change_clears_redo() {
        let mut doc = Document::new(64, 64, None);
        let mut h = History::default();
        h.record("a", &doc);
        paint(&mut doc, 0, 0, [1, 1, 1, 255]);
        h.undo(&mut doc);
        assert!(h.can_redo());
        h.record("b", &doc);
        paint(&mut doc, 0, 20, [2, 2, 2, 255]);
        assert!(!h.can_redo());
        assert_eq!(h.entries(), (vec!["b"], 1));
    }

    #[test]
    fn limit_drops_the_oldest() {
        let mut doc = Document::new(64, 64, None);
        let mut h = History::default();
        for i in 0..60 {
            h.record(&format!("step {i}"), &doc);
            paint(&mut doc, 0, 0, [i as u8, 0, 0, 255]);
        }
        let (names, applied) = h.entries();
        assert_eq!((names.len(), applied), (50, 50));
        assert_eq!(names[0], "step 10");
        while h.undo(&mut doc).is_some() {}
        assert_eq!(doc.layers[0].pixels.pixel(0, 0), [9, 0, 0, 255], "oldest kept state is after step 9");
    }

    #[test]
    fn snapshots_share_untouched_tiles() {
        let mut doc = Document::new(1024, 1024, None);
        for x in (0..1024).step_by(64) {
            for y in (0..1024).step_by(64) {
                doc.layers[0].pixels.set_pixel(x, y, [9, 9, 9, 255]);
            }
        }
        assert_eq!(doc.layers[0].pixels.tile_count(), 256);
        let mut h = History::default();
        h.record("stroke", &doc);
        assert_eq!(h.memory_bytes(&doc), 0, "a snapshot alone copies no pixels");
        paint(&mut doc, 0, 0, [1, 2, 3, 255]); // touches 1 tile
        assert_eq!(h.memory_bytes(&doc), TILE_BYTES, "only the painted tile is kept twice");
    }

    #[test]
    fn jump_to_moves_both_ways() {
        let mut doc = Document::new(64, 64, None);
        let mut h = History::default();
        let mut states = vec![doc.clone()];
        for i in 0..5 {
            h.record(&format!("s{i}"), &doc);
            paint(&mut doc, 0, i * 3, [i as u8 + 1, 0, 0, 255]);
            states.push(doc.clone());
        }
        h.jump_to(&mut doc, 2);
        assert_eq!(doc, states[2]);
        assert_eq!(h.entries().1, 2);
        h.jump_to(&mut doc, 4);
        assert_eq!(doc, states[4]);
        h.jump_to(&mut doc, 0);
        assert_eq!(doc, states[0]);
        h.jump_to(&mut doc, 99);
        assert_eq!(doc, states[5]);
    }
}
