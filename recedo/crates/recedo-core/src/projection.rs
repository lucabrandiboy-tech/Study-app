//! The projection: the flattened picture the screen shows, kept up to date tile by tile
//! (as in Krita). Painting changes a few tiles; only those are recomposited and uploaded.

use std::collections::HashSet;

use crate::blend::blend;
use crate::doc::Document;
use crate::image::Rgba8;
use crate::tiles::{TILE, TileKey};

pub struct Projection {
    pub image: Rgba8,
}

impl Projection {
    pub fn new(doc: &Document) -> Projection {
        Projection { image: doc.flatten() }
    }

    /// Recomposites everything (after layer opacity, visibility, order or canvas changes).
    pub fn rebuild(&mut self, doc: &Document) {
        self.image = doc.flatten();
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
    pub fn update(&mut self, doc: &Document, tiles: &HashSet<TileKey>) -> Vec<TileKey> {
        let w = self.image.width as usize;
        let mut done = Vec::new();
        for &key in tiles {
            let Some((x, y, tw, th)) = self.tile_rect(key) else { continue };
            for row in 0..th as usize {
                let start = ((y as usize + row) * w + x as usize) * 4;
                self.image.data[start..start + tw as usize * 4].fill(0);
            }
            for l in doc.layers.iter().filter(|l| l.shows()) {
                let tile = l.pixels.tile(key);
                if tile.is_none() && l.pixels.default[3] == 0 {
                    continue;
                }
                for row in 0..th as usize {
                    let start = ((y as usize + row) * w + x as usize) * 4;
                    let dst = &mut self.image.data[start..start + tw as usize * 4];
                    for (i, d) in dst.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                        let s = tile.map_or(l.pixels.default, |t| t.get(i as i32, row as i32));
                        if s[3] != 0 || l.blend != crate::blend::BlendMode::Normal {
                            blend(d, s, l.opacity, l.blend);
                        }
                    }
                }
            }
            done.push(key);
        }
        done
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brush::{Brush, StrokePainter};
    use crate::doc::Layer;
    use crate::stabilizer::PaintInfo;

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
}
