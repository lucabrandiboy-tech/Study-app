//! Tiled layer storage, like Krita's: 64×64-pixel tiles created on first write, shared
//! copy-on-write so an undo step only keeps the tiles a stroke changed.
//!
//! Pixels are 8-bit RGBA, straight (not premultiplied) alpha, sRGB-encoded, which is
//! what Krita's 8-bit RGBA layers hold (Krita orders them B, G, R, A in memory; the file
//! code converts).

use std::collections::HashMap;
use std::sync::Arc;

pub const TILE: i32 = 64;
pub const TILE_PIXELS: usize = (TILE * TILE) as usize;
pub const TILE_BYTES: usize = TILE_PIXELS * 4;

/// One tile's pixels, row-major RGBA8.
#[derive(Clone, PartialEq, Eq)]
pub struct Tile(pub Box<[u8; TILE_BYTES]>);

impl std::fmt::Debug for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Tile(..)")
    }
}

impl Tile {
    pub fn filled(px: [u8; 4]) -> Tile {
        let mut b = Box::new([0u8; TILE_BYTES]);
        for c in b.as_chunks_mut::<4>().0 {
            *c = px;
        }
        Tile(b)
    }

    pub fn get(&self, x: i32, y: i32) -> [u8; 4] {
        let i = ((y * TILE + x) * 4) as usize;
        [self.0[i], self.0[i + 1], self.0[i + 2], self.0[i + 3]]
    }

    pub fn set(&mut self, x: i32, y: i32, px: [u8; 4]) {
        let i = ((y * TILE + x) * 4) as usize;
        self.0[i..i + 4].copy_from_slice(&px);
    }

    /// True if every pixel equals `px`.
    pub fn is_uniform(&self, px: [u8; 4]) -> bool {
        self.0.as_chunks::<4>().0.iter().all(|c| *c == px)
    }
}

/// Tile coordinates (column, row); tile (c, r) covers pixels c*64.. and r*64...
pub type TileKey = (i32, i32);

pub fn tile_of(x: i32, y: i32) -> (TileKey, i32, i32) {
    (
        (x.div_euclid(TILE), y.div_euclid(TILE)),
        x.rem_euclid(TILE),
        y.rem_euclid(TILE),
    )
}

/// A layer's pixels: sparse tiles over an infinite plane, `default` everywhere else.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TiledLayer {
    tiles: HashMap<TileKey, Arc<Tile>>,
    pub default: [u8; 4],
}

impl TiledLayer {
    pub fn new() -> TiledLayer {
        TiledLayer::default()
    }

    pub fn filled(px: [u8; 4]) -> TiledLayer {
        TiledLayer {
            tiles: HashMap::new(),
            default: px,
        }
    }

    pub fn pixel(&self, x: i32, y: i32) -> [u8; 4] {
        let (k, tx, ty) = tile_of(x, y);
        self.tiles.get(&k).map_or(self.default, |t| t.get(tx, ty))
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, px: [u8; 4]) {
        let (k, tx, ty) = tile_of(x, y);
        self.tile_mut(k).set(tx, ty, px);
    }

    /// The tile for writing: created from the default if missing, and copied first if it
    /// is shared with an undo step (copy-on-write).
    pub fn tile_mut(&mut self, k: TileKey) -> &mut Tile {
        let default = self.default;
        Arc::make_mut(
            self.tiles
                .entry(k)
                .or_insert_with(|| Arc::new(Tile::filled(default))),
        )
    }

    pub fn tile(&self, k: TileKey) -> Option<&Arc<Tile>> {
        self.tiles.get(&k)
    }

    /// Replaces (or with `None` removes) a tile; used by undo and file loading.
    pub fn put_tile(&mut self, k: TileKey, t: Option<Arc<Tile>>) {
        match t {
            Some(t) => {
                self.tiles.insert(k, t);
            }
            None => {
                self.tiles.remove(&k);
            }
        }
    }

    /// Removes a tile and hands it over (to be put back with `put_tile`), so several tiles
    /// can be changed at once on different threads.
    pub fn take_tile(&mut self, k: TileKey) -> Option<Arc<Tile>> {
        self.tiles.remove(&k)
    }

    pub fn tile_keys(&self) -> impl Iterator<Item = TileKey> + '_ {
        self.tiles.keys().copied()
    }

    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// Bytes of pixel memory used (shared tiles counted once per layer).
    pub fn memory_bytes(&self) -> usize {
        self.tiles.len() * TILE_BYTES
    }

    /// Drops tiles that hold only the default pixel.
    pub fn compact(&mut self) {
        let d = self.default;
        self.tiles.retain(|_, t| !t.is_uniform(d));
    }

    /// Copies a rectangle out as row-major RGBA8.
    pub fn read_rect(&self, x0: i32, y0: i32, w: u32, h: u32) -> Vec<u8> {
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for y in y0..y0 + h as i32 {
            for x in x0..x0 + w as i32 {
                out.extend_from_slice(&self.pixel(x, y));
            }
        }
        out
    }

    /// Writes row-major RGBA8 into the layer at (x0, y0).
    pub fn write_rect(&mut self, x0: i32, y0: i32, w: u32, h: u32, rgba: &[u8]) {
        assert_eq!(rgba.len(), (w * h * 4) as usize, "rectangle size mismatch");
        for (i, px) in rgba.as_chunks::<4>().0.iter().enumerate() {
            let x = x0 + (i as u32 % w) as i32;
            let y = y0 + (i as u32 / w) as i32;
            self.set_pixel(x, y, *px);
        }
        self.compact();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_coordinates_floor_for_negatives() {
        assert_eq!(tile_of(0, 0), ((0, 0), 0, 0));
        assert_eq!(tile_of(63, 64), ((0, 1), 63, 0));
        assert_eq!(tile_of(-1, -64), ((-1, -1), 63, 0));
        assert_eq!(tile_of(-65, 5), ((-2, 0), 63, 5));
    }

    #[test]
    fn tiles_are_created_on_write_only() {
        let mut l = TiledLayer::new();
        assert_eq!(l.pixel(1000, -5), [0, 0, 0, 0]);
        assert_eq!(l.tile_count(), 0);
        l.set_pixel(1000, -5, [1, 2, 3, 4]);
        assert_eq!(l.tile_count(), 1);
        assert_eq!(l.pixel(1000, -5), [1, 2, 3, 4]);
        assert_eq!(l.pixel(1001, -5), [0, 0, 0, 0]);
    }

    #[test]
    fn copy_on_write_keeps_snapshots_intact() {
        let mut l = TiledLayer::new();
        l.set_pixel(5, 5, [9, 9, 9, 255]);
        let snapshot = l.tile((0, 0)).cloned().unwrap();
        l.set_pixel(5, 5, [1, 1, 1, 255]);
        assert_eq!(
            snapshot.get(5, 5),
            [9, 9, 9, 255],
            "the old tile is untouched"
        );
        assert_eq!(l.pixel(5, 5), [1, 1, 1, 255]);
        l.put_tile((0, 0), Some(snapshot));
        assert_eq!(l.pixel(5, 5), [9, 9, 9, 255]);
    }

    #[test]
    fn rect_roundtrip_and_compact() {
        let mut l = TiledLayer::filled([255, 255, 255, 255]);
        let data: Vec<u8> = (0..(70 * 3 * 4)).map(|i| (i % 251) as u8).collect();
        l.write_rect(60, 62, 70, 3, &data);
        assert_eq!(l.read_rect(60, 62, 70, 3), data);
        // Writing the default back leaves no tiles.
        let white = vec![255u8; 70 * 3 * 4];
        l.write_rect(60, 62, 70, 3, &white);
        assert_eq!(l.tile_count(), 0);
    }
}
