//! The projection: the flattened picture the screen shows, kept up to date tile by tile
//! (as in Krita). Painting changes a few tiles; only those are recomposited and uploaded.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::blend::BlendMode;
use crate::doc::{Document, Layer, composite_tile};
use crate::image::Rgba8;
use crate::tiles::{TILE, Tile, TileKey};

pub struct Projection {
    pub image: Rgba8,
    /// While a stroke is painted on one layer, the layers below it give the same result in
    /// every frame: per tile, that result is kept and only the layers from the stroke's up
    /// are combined again. See [`Projection::set_stroke_layer`].
    below: Option<BelowCache>,
}

struct BelowCache {
    /// Index of the layer being painted; the cache holds the layers under it.
    layer: usize,
    tiles: HashMap<TileKey, CachedBelow>,
}

struct CachedBelow {
    /// What each layer below looked like at this tile when this was made.
    sources: Vec<Source>,
    pixels: Vec<u8>,
}

/// One layer at one tile, as far as compositing is concerned. The tile is held (not just
/// its address), so a changed tile can never be mistaken for the cached one.
struct Source {
    tile: Option<Arc<Tile>>,
    default: [u8; 4],
    opacity: u8,
    blend: BlendMode,
    visible: bool,
}

impl Source {
    fn of(l: &Layer, key: TileKey) -> Source {
        Source { tile: l.pixels.tile(key).cloned(), default: l.pixels.default, opacity: l.opacity, blend: l.blend, visible: l.visible }
    }

    fn same(&self, l: &Layer, key: TileKey) -> bool {
        let tile_same = match (&self.tile, l.pixels.tile(key)) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        tile_same && self.default == l.pixels.default && self.opacity == l.opacity && self.blend == l.blend && self.visible == l.visible
    }
}

/// A layer that shows, with its fill colour as a tile for where it has no tiles.
struct Shown<'a> {
    index: usize,
    layer: &'a Layer,
    default: Option<Tile>,
}

/// Composites the shown layers onto `buf` (one tile's pixels, `tw` wide).
fn composite_layers(buf: &mut [u8], layers: &[Shown], key: TileKey, tw: u32, th: u32) {
    for s in layers {
        let tile = match (s.layer.pixels.tile(key), &s.default) {
            (Some(t), _) => &**t,
            (None, Some(d)) => d,
            (None, None) => continue,
        };
        composite_tile(buf, tw as usize, tile, tw as usize, th as usize, s.layer.opacity, s.layer.blend);
    }
}

impl Projection {
    pub fn new(doc: &Document) -> Projection {
        Projection { image: doc.flatten(), below: None }
    }

    /// Recomposites everything (after layer opacity, visibility, order or canvas changes).
    pub fn rebuild(&mut self, doc: &Document) {
        self.image = doc.flatten();
    }

    /// Tells the projection which layer a stroke is painting (`None` when no stroke is in
    /// progress, which frees the cache). Safe whatever else changes meanwhile: every cached
    /// tile is checked against the layers below before it is used.
    pub fn set_stroke_layer(&mut self, layer: Option<usize>) {
        match layer {
            Some(l) if self.below.as_ref().is_some_and(|b| b.layer == l) => {}
            Some(l) => self.below = Some(BelowCache { layer: l, tiles: HashMap::new() }),
            None => self.below = None,
        }
    }

    /// Every tile key of the canvas.
    pub fn all_tiles(&self) -> Vec<TileKey> {
        let (cols, rows) = (self.image.width.div_ceil(TILE as u32) as i32, self.image.height.div_ceil(TILE as u32) as i32);
        (0..rows).flat_map(|r| (0..cols).map(move |c| (c, r))).collect()
    }

    /// The canvas rectangle (x, y, w, h) of a tile, clipped to the canvas; `None` if the tile
    /// is outside it.
    pub fn tile_rect(&self, key: TileKey) -> Option<(u32, u32, u32, u32)> {
        let (x, y) = (key.0 * TILE, key.1 * TILE);
        if x < 0 || y < 0 || x >= self.image.width as i32 || y >= self.image.height as i32 {
            return None;
        }
        let (x, y) = (x as u32, y as u32);
        Some((x, y, (TILE as u32).min(self.image.width - x), (TILE as u32).min(self.image.height - y)))
    }

    /// Recomposites the given tiles from the document's visible layers. Returns the tiles
    /// inside the canvas (the ones to upload).
    ///
    /// Each tile is composited into its own buffer, the tiles split between the CPU cores
    /// (a big brush or an undo changes hundreds of tiles through every layer), then copied in.
    /// The result does not depend on the split: every tile is computed on its own.
    pub fn update(&mut self, doc: &Document, tiles: &HashSet<TileKey>) -> Vec<TileKey> {
        let jobs: Vec<(TileKey, (u32, u32, u32, u32))> = tiles.iter().filter_map(|&k| self.tile_rect(k).map(|r| (k, r))).collect();
        let shown: Vec<Shown> = doc
            .layers
            .iter()
            .enumerate()
            .filter(|(_, l)| l.shows())
            .map(|(index, layer)| Shown { index, layer, default: (layer.pixels.default[3] != 0).then(|| Tile::filled(layer.pixels.default)) })
            .collect();
        let split = self.below.as_ref().map(|b| b.layer.min(doc.layers.len()));
        let (under, over) = shown.split_at(split.map_or(0, |k| shown.partition_point(|s| s.index < k)));
        let cache = self.below.as_ref().map(|b| &b.tiles);
        // A tile's pixels, and (with a stroke layer) the layers below it if they had to be
        // combined anew.
        let composite = |&(key, (_, _, tw, th)): &(TileKey, (u32, u32, u32, u32))| -> (Vec<u8>, Option<CachedBelow>) {
            let Some(k) = split else {
                let mut buf = vec![0u8; (tw * th * 4) as usize];
                composite_layers(&mut buf, over, key, tw, th);
                return (buf, None);
            };
            let below = &doc.layers[..k];
            let cached = cache.and_then(|c| c.get(&key)).filter(|c| c.sources.len() == below.len() && c.sources.iter().zip(below).all(|(s, l)| s.same(l, key)));
            let (mut buf, fresh) = match cached {
                Some(c) => (c.pixels.clone(), None),
                None => {
                    let mut b = vec![0u8; (tw * th * 4) as usize];
                    composite_layers(&mut b, under, key, tw, th);
                    let entry = CachedBelow { sources: below.iter().map(|l| Source::of(l, key)).collect(), pixels: b.clone() };
                    (b, Some(entry))
                }
            };
            composite_layers(&mut buf, over, key, tw, th);
            (buf, fresh)
        };
        // Split between the CPU cores on the shared thread pool, except for a handful of
        // tiles (handing work to the pool costs about as much as a tile through 15 layers).
        let results: Vec<(Vec<u8>, Option<CachedBelow>)> = if jobs.len() >= 8 {
            use rayon::prelude::*;
            jobs.par_iter().map(composite).collect()
        } else {
            jobs.iter().map(composite).collect()
        };
        let w = self.image.width as usize;
        for ((key, (x, y, tw, th)), (buf, fresh)) in jobs.iter().zip(results) {
            let row_bytes = *tw as usize * 4;
            for row in 0..*th as usize {
                let start = ((*y as usize + row) * w + *x as usize) * 4;
                self.image.data[start..start + row_bytes].copy_from_slice(&buf[row * row_bytes..][..row_bytes]);
            }
            if let (Some(entry), Some(b)) = (fresh, self.below.as_mut()) {
                b.tiles.insert(*key, entry);
            }
        }
        jobs.into_iter().map(|(k, _)| k).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brush::{Brush, StrokePainter};
    use crate::doc::Layer;
    use crate::stabilizer::PaintInfo;
    use crate::tiles::TiledLayer;

    #[test]
    fn tile_updates_match_a_full_flatten() {
        let mut doc = Document::new(150, 100, Some([250, 250, 250, 255]));
        let mut top = Layer::new("top");
        top.opacity = 140;
        doc.layers.push(top);
        let mut proj = Projection::new(&doc);
        assert_eq!(proj.all_tiles().len(), 3 * 2);
        for (layer, color, y) in [(1usize, [200, 0, 0], 30.0), (2, [0, 0, 200], 50.0)] {
            let mut p = StrokePainter::new(Brush { size: 12.0, ..Default::default() }, color, false);
            let mut dirty = HashSet::new();
            for i in 0..=40 {
                let pt = PaintInfo { x: -10.0 + i as f64 * 5.0, y, pressure: 1.0, tilt_x: 0.0, tilt_y: 0.0, rotation: 0.0 };
                p.add(&mut doc.layers[layer].pixels, pt, &mut dirty);
            }
            let uploaded = proj.update(&doc, &dirty);
            assert!(uploaded.len() < dirty.len(), "tiles outside the canvas are skipped");
            assert_eq!(proj.image, doc.flatten());
        }
        doc.layers[2].visible = false;
        proj.rebuild(&doc);
        assert_eq!(proj.image, doc.flatten());
        assert_eq!(proj.tile_rect((2, 1)), Some((128, 64, 22, 36)));
        assert_eq!(proj.tile_rect((3, 0)), None);
        assert_eq!(proj.tile_rect((-1, 0)), None);
    }

    /// Many tiles (so the work is split between threads), every kind of layer: the result
    /// must be exactly a full flatten.
    #[test]
    fn parallel_update_matches_a_full_flatten() {
        use crate::blend::BlendMode;
        let (w, h) = (640u32, 400u32);
        let mut doc = Document::new(w, h, Some([240, 230, 220, 255]));
        let mut seed = 12345u32;
        let mut rnd = move || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed
        };
        for (i, (mode, opacity)) in [(BlendMode::Normal, 255u8), (BlendMode::Multiply, 200), (BlendMode::Screen, 128), (BlendMode::Overlay, 90), (BlendMode::Difference, 255)].into_iter().enumerate() {
            let mut l = Layer::new(&format!("l{i}"));
            l.blend = mode;
            l.opacity = opacity;
            // Painted in patches, so some tiles are missing on each layer.
            for y in 0..h as i32 {
                for x in 0..w as i32 {
                    if (x / 64 + y / 64 + i as i32) % 3 != 0 {
                        let r = rnd();
                        l.pixels.set_pixel(x, y, [r as u8, (r >> 8) as u8, (r >> 16) as u8, (r >> 24) as u8 | 0x10]);
                    }
                }
            }
            doc.layers.push(l);
        }
        // A hidden layer and a layer filled with a colour everywhere (no tiles at all).
        let mut hidden = Layer::new("hidden");
        hidden.visible = false;
        hidden.pixels = TiledLayer::filled([255, 0, 0, 255]);
        doc.layers.push(hidden);
        let mut fill = Layer::new("fill");
        fill.pixels = TiledLayer::filled([0, 0, 255, 60]);
        fill.blend = BlendMode::Multiply;
        doc.layers.push(fill);
        let mut proj = Projection { image: Rgba8::new(w, h), below: None };
        let all: HashSet<TileKey> = proj.all_tiles().into_iter().collect();
        assert_eq!(all.len(), 70);
        let uploaded = proj.update(&doc, &all);
        assert_eq!(uploaded.len(), 70);
        assert!(proj.image == doc.flatten(), "every tile at once");
        // Some tiles change; only those are updated, and the picture still matches.
        let mut changed = HashSet::new();
        for (x, y) in [(10, 10), (300, 200), (639, 399), (70, 330)] {
            doc.layers[2].pixels.set_pixel(x, y, [1, 2, 3, 200]);
            changed.insert(crate::tiles::tile_of(x, y).0);
        }
        let mut partial = proj.update(&doc, &changed);
        partial.sort();
        let mut want: Vec<TileKey> = changed.into_iter().collect();
        want.sort();
        assert_eq!(partial, want);
        assert!(proj.image == doc.flatten(), "after a partial update");
    }

    /// The cache of the layers under a stroke: every way the layers below can change while
    /// it is held must be noticed, so the picture always equals a full flatten.
    #[test]
    fn below_the_stroke_cache_never_goes_stale() {
        use crate::blend::BlendMode;
        let (w, h) = (300u32, 200u32);
        let mut doc = Document::new(w, h, Some([250, 250, 250, 255]));
        // Background, then l0..l4 (the empty layer a new document has is removed, so the
        // indexes below are exactly these).
        doc.layers.truncate(1);
        let mut seed = 777u32;
        let mut rnd = move || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed
        };
        for i in 0..5 {
            let mut l = Layer::new(&format!("l{i}"));
            l.opacity = 180 + i * 10;
            for y in (0..h as i32).step_by(3) {
                for x in (i as i32..w as i32).step_by(4) {
                    let r = rnd();
                    l.pixels.set_pixel(x, y, [r as u8, (r >> 8) as u8, (r >> 16) as u8, 90 + (r >> 26) as u8]);
                }
            }
            doc.layers.push(l);
        }
        let mut proj = Projection::new(&doc);
        let all: HashSet<TileKey> = proj.all_tiles().into_iter().collect();
        let stroke = 4;
        proj.set_stroke_layer(Some(stroke));
        // Paints a line on the stroke's layer and updates its tiles; those tiles must then
        // match a full flatten exactly.
        let paint = |doc: &mut Document, proj: &mut Projection, x: i32, what: &str| {
            for y in 20..180 {
                doc.layers[stroke].pixels.set_pixel(x, y, [x as u8, y as u8, 7, 255]);
            }
            let dirty: HashSet<TileKey> = (20..180).map(|y| crate::tiles::tile_of(x, y).0).collect();
            proj.update(doc, &dirty);
            let flat = doc.flatten();
            for &key in &dirty {
                let (x0, y0, tw, th) = proj.tile_rect(key).unwrap();
                for y in y0..y0 + th {
                    let at = ((y * w + x0) * 4) as usize;
                    assert_eq!(proj.image.data[at..at + tw as usize * 4], flat.data[at..at + tw as usize * 4], "{what}: tile {key:?} row {y}");
                }
            }
        };
        // Painting frame after frame (the cache is filled, then used).
        for (i, x) in [10, 11, 70, 71, 72, 150, 151, 290].into_iter().enumerate() {
            paint(&mut doc, &mut proj, x, &format!("stroke frame {i}"));
        }
        assert!(proj.below.as_ref().is_some_and(|b| !b.tiles.is_empty()), "the cache was used");
        // Each change below the stroke, then more painting over the cached tiles.
        // Each change below the stroke and its name.
        type Change = (&'static str, fn(&mut Document));
        let changes: Vec<Change> = vec![
            ("a pixel below", |d: &mut Document| d.layers[2].pixels.set_pixel(75, 50, [255, 0, 0, 255])),
            ("opacity below", |d: &mut Document| d.layers[1].opacity = 30),
            ("visibility below", |d: &mut Document| d.layers[3].visible = false),
            ("blend mode below", |d: &mut Document| d.layers[2].blend = BlendMode::Multiply),
            ("layers below swapped", |d: &mut Document| d.layers.swap(1, 2)),
            ("a layer below replaced by an empty one", |d: &mut Document| d.layers[3] = Layer::new("new")),
            ("the background colour", |d: &mut Document| d.layers[0].pixels = TiledLayer::filled([10, 20, 30, 255])),
        ];
        for (what, change) in changes {
            change(&mut doc);
            // The editor redraws the tiles a change touches; here the stroke's tiles only, to
            // prove the cache itself notices.
            paint(&mut doc, &mut proj, 71, what);
            paint(&mut doc, &mut proj, 151, what);
            proj.update(&doc, &all);
            assert!(proj.image == doc.flatten(), "{what}: whole picture");
        }
        // A layer removed below: the stroke's layer moved down by one.
        doc.layers.remove(1);
        proj.set_stroke_layer(Some(stroke - 1));
        proj.update(&doc, &all);
        assert!(proj.image == doc.flatten(), "layer removed");
        // A stroke layer past the top (every layer counts as below), then the top layer
        // removed: the cached tiles were made from one layer more than there now is.
        proj.set_stroke_layer(Some(99));
        proj.update(&doc, &all);
        assert!(proj.image == doc.flatten(), "stroke layer past the top");
        doc.layers.pop();
        proj.update(&doc, &all);
        assert!(proj.image == doc.flatten(), "top layer removed under a cache of all layers");
        proj.set_stroke_layer(None);
        assert!(proj.below.is_none(), "cache freed after the stroke");
    }
}
