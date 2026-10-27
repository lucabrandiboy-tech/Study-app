//! Painting one brush dab into a layer's tiles (the CPU side of painting, as in Krita: the
//! tiles are the picture, the GPU only shows them).
//!
//! The dab looks like the tablet test app's GPU dab (`shaders.wgsl`): an ellipse with about
//! one pixel of anti-aliasing at the edge and, below hardness 1, a smooth fall-off from
//! `hardness` x radius to the edge.

use std::collections::HashSet;
use std::sync::Arc;

use crate::doc::{mul8, over};
use crate::tiles::{TILE, TILE_PIXELS, Tile, TileKey, TiledLayer};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DabShape {
    /// Centre in layer pixels (pixel (x, y) covers x..x+1, y..y+1).
    pub cx: f32,
    pub cy: f32,
    /// Half-axes in pixels.
    pub rx: f32,
    pub ry: f32,
    /// Rotation of the first axis, radians.
    pub angle: f32,
    /// 0 = soft all the way from the centre, 1 = hard edge.
    pub hardness: f32,
}

/// Smallest half-axis drawn as is; smaller dabs are drawn at this size, fainter, so very
/// thin strokes keep the right amount of ink instead of breaking into dots.
const MIN_RADIUS: f32 = 0.5;
/// Largest half-axis accepted (a 20000-pixel brush).
pub const MAX_RADIUS: f32 = 10_000.0;

/// Coverage (0..=1) of a pixel centre at offset (dx, dy) from the dab centre: the plain
/// form of what `Job::coverage_row` computes (a test holds them equal).
#[cfg(test)]
fn coverage(dx: f32, dy: f32, d: &DabShape, cos: f32, sin: f32, aa: f32) -> f32 {
    let u = (dx * cos + dy * sin) / d.rx;
    let v = (-dx * sin + dy * cos) / d.ry;
    let r = (u * u + v * v).sqrt();
    let edge = ((1.0 - r) / aa + 0.5).clamp(0.0, 1.0);
    if edge <= 0.0 {
        return 0.0;
    }
    if d.hardness < 0.999 {
        let h = d.hardness.max(0.0);
        let t = ((r - h) / (1.0 - h)).clamp(0.0, 1.0);
        edge * (1.0 - t * t * (3.0 - 2.0 * t))
    } else {
        edge
    }
}

/// Paints a dab of `color` at `alpha` (0..=1), or erases with it, and adds every tile it
/// changed to `dirty`. Erasing never creates tiles.
pub fn paint_dab(layer: &mut TiledLayer, shape: &DabShape, color: [u8; 3], alpha: f32, erase: bool, dirty: &mut HashSet<TileKey>) {
    let Some(job) = Job::new(shape, alpha, color, erase, layer.default) else { return };
    let (x0, x1, y0, y1) = (job.x0, job.x1, job.y0, job.y1);
    // The tiles under the dab, taken out of the layer so each can be painted on its own
    // thread; put back afterwards (tiles that stayed untouched are not created).
    let mut jobs: Vec<(TileKey, Option<Arc<Tile>>, bool)> = Vec::new();
    for row in y0.div_euclid(TILE)..=(y1 - 1).div_euclid(TILE) {
        for col in x0.div_euclid(TILE)..=(x1 - 1).div_euclid(TILE) {
            let key = (col, row);
            if erase && layer.tile(key).is_none() && layer.default[3] == 0 {
                continue;
            }
            jobs.push((key, layer.take_tile(key), false));
        }
    }
    if jobs.len() >= PARALLEL_FROM_TILES {
        // On the shared thread pool: starting threads for every dab would cost more than
        // the painting on Windows.
        use rayon::prelude::*;
        jobs.par_iter_mut().for_each_init(|| Vec::with_capacity(TILE_PIXELS), |mask, (key, tile, touched)| *touched = job.paint_tile(*key, tile, mask));
    } else {
        let mut mask = Vec::with_capacity(TILE_PIXELS);
        for (key, tile, touched) in &mut jobs {
            *touched = job.paint_tile(*key, tile, &mut mask);
        }
    }
    for (key, tile, touched) in jobs {
        if tile.is_some() {
            layer.put_tile(key, tile);
        }
        if touched {
            dirty.insert(key);
        }
    }
}

/// The dab's coverage (what a white dab of `alpha` leaves as alpha on an empty layer), per
/// tile it touches: one byte per pixel of the tile, tiles it covers nowhere left out. The
/// brush's wash mode needs only this; painting it into a scratch layer cost a layer, its
/// tiles and four bytes per pixel for every dab.
pub fn coverage_tiles(shape: &DabShape, alpha: f32) -> Vec<(TileKey, Box<[u8; TILE_PIXELS]>)> {
    let Some(job) = Job::new(shape, alpha, [255, 255, 255], false, [0; 4]) else { return Vec::new() };
    let mut keys = Vec::new();
    for row in job.y0.div_euclid(TILE)..=(job.y1 - 1).div_euclid(TILE) {
        for col in job.x0.div_euclid(TILE)..=(job.x1 - 1).div_euclid(TILE) {
            keys.push((col, row));
        }
    }
    let one = |key: TileKey| {
        let mut out = Box::new([0u8; TILE_PIXELS]);
        job.coverage_tile(key, &mut out).then_some((key, out))
    };
    if keys.len() >= PARALLEL_FROM_TILES {
        use rayon::prelude::*;
        keys.into_par_iter().filter_map(one).collect()
    } else {
        keys.into_iter().filter_map(one).collect()
    }
}

/// Dabs covering at least this many tiles are painted on several threads.
const PARALLEL_FROM_TILES: usize = 9;

/// One dab, ready to paint tile by tile.
struct Job {
    d: DabShape,
    cos: f32,
    sin: f32,
    aa: f32,
    a8_max: f32,
    x0: i32,
    x1: i32,
    y0: i32,
    y1: i32,
    color: [u8; 3],
    erase: bool,
    default: [u8; 4],
}

impl Job {
    /// The dab ready to paint, or `None` if it paints nothing (no alpha, bad numbers).
    fn new(shape: &DabShape, alpha: f32, color: [u8; 3], erase: bool, default: [u8; 4]) -> Option<Job> {
        let radius_ok = |r: f32| r > 0.0 && r <= MAX_RADIUS;
        if alpha.is_nan() || alpha <= 0.0 || !shape.cx.is_finite() || !shape.cy.is_finite() || !radius_ok(shape.rx) || !radius_ok(shape.ry) || !shape.angle.is_finite() {
            return None;
        }
        let mut d = *shape;
        let mut alpha = alpha.min(1.0);
        if d.rx < MIN_RADIUS || d.ry < MIN_RADIUS {
            alpha *= (d.rx.min(MIN_RADIUS) * d.ry.min(MIN_RADIUS)) / (MIN_RADIUS * MIN_RADIUS);
            d.rx = d.rx.max(MIN_RADIUS);
            d.ry = d.ry.max(MIN_RADIUS);
        }
        let (sin, cos) = d.angle.sin_cos();
        // The distance field changes by 1/radius per pixel: about one pixel of smoothing.
        let aa = (1.0 / d.rx.min(d.ry)).max(1e-4);
        let hw = ((d.rx * cos).powi(2) + (d.ry * sin).powi(2)).sqrt() + 1.0;
        let hh = ((d.rx * sin).powi(2) + (d.ry * cos).powi(2)).sqrt() + 1.0;
        let x0 = (d.cx - hw).floor() as i32;
        let x1 = (d.cx + hw).ceil() as i32;
        let y0 = (d.cy - hh).floor() as i32;
        let y1 = (d.cy + hh).ceil() as i32;
        Some(Job { d, cos, sin, aa, a8_max: alpha * 255.0, x0, x1, y0, y1, color, erase, default })
    }

    /// The dab's coverage in one tile, into `out` (one byte per pixel of the tile, the same
    /// values `paint_tile` paints with). Returns whether it covers any of it.
    fn coverage_tile(&self, key: TileKey, out: &mut [u8; TILE_PIXELS]) -> bool {
        let (tx0, ty0) = (key.0 * TILE, key.1 * TILE);
        let xs = self.x0.max(tx0)..self.x1.min(tx0 + TILE);
        let ys = self.y0.max(ty0)..self.y1.min(ty0 + TILE);
        let mut any = false;
        if xs.start >= xs.end {
            return false;
        }
        for y in ys {
            let row = &mut out[((y - ty0) * TILE) as usize..][..TILE as usize];
            let part = &mut row[(xs.start - tx0) as usize..(xs.end - tx0) as usize];
            self.coverage_row(y, xs.start, part);
            any |= part.iter().any(|&a| a != 0);
        }
        any
    }

    /// Coverage bytes of pixels `x0..` of row `y` into `out`: [`coverage`] times the dab's
    /// alpha, rounded, with the same arithmetic in the same order, but without branches so
    /// the compiler can work on several pixels at once (SIMD).
    fn coverage_row(&self, y: i32, x0: i32, out: &mut [u8]) {
        let d = &self.d;
        let dy = y as f32 + 0.5 - d.cy;
        let (cos, sin, aa, max) = (self.cos, self.sin, self.aa, self.a8_max);
        let (dys, dyc) = (dy * sin, dy * cos);
        let soft = d.hardness < 0.999;
        let h = d.hardness.max(0.0);
        for (i, o) in out.iter_mut().enumerate() {
            let dx = (x0 + i as i32) as f32 + 0.5 - d.cx;
            let u = (dx * cos + dys) / d.rx;
            let v = (-dx * sin + dyc) / d.ry;
            let r = (u * u + v * v).sqrt();
            let edge = ((1.0 - r) / aa + 0.5).clamp(0.0, 1.0);
            let c = if soft {
                let t = ((r - h) / (1.0 - h)).clamp(0.0, 1.0);
                edge * (1.0 - t * t * (3.0 - 2.0 * t))
            } else {
                edge
            };
            *o = (c * max + 0.5) as u8;
        }
    }

    /// Paints the dab's part in one tile; creates the tile (from the default pixel) only if
    /// the dab covers some of it. Returns whether anything changed.
    fn paint_tile(&self, key: TileKey, tile: &mut Option<Arc<Tile>>, mask: &mut Vec<u8>) -> bool {
        let (tx0, ty0) = (key.0 * TILE, key.1 * TILE);
        let xs = self.x0.max(tx0)..self.x1.min(tx0 + TILE);
        let ys = self.y0.max(ty0)..self.y1.min(ty0 + TILE);
        // Coverage first, so tiles the dab only grazes with zero coverage are not created.
        let w = (xs.end - xs.start).max(0) as usize;
        mask.clear();
        mask.resize(w * ys.clone().count(), 0);
        if w > 0 {
            for (y, row) in ys.clone().zip(mask.chunks_exact_mut(w)) {
                self.coverage_row(y, xs.start, row);
            }
        }
        if mask.iter().all(|&a| a == 0) {
            return false;
        }
        let t = Arc::make_mut(tile.get_or_insert_with(|| Arc::new(Tile::filled(self.default))));
        for (i, &a8) in mask.iter().enumerate().filter(|(_, a)| **a != 0) {
            let (px, py) = (xs.start - tx0 + (i % w) as i32, ys.start - ty0 + (i / w) as i32);
            let mut p = t.get(px, py);
            if self.erase {
                p[3] -= mul8(p[3], a8);
            } else {
                over(&mut p, [self.color[0], self.color[1], self.color[2], a8], 255);
            }
            t.set(px, py, p);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round(cx: f32, cy: f32, r: f32, hardness: f32) -> DabShape {
        DabShape { cx, cy, rx: r, ry: r, angle: 0.0, hardness }
    }

    fn total_alpha(l: &TiledLayer, x0: i32, y0: i32, x1: i32, y1: i32) -> f64 {
        let mut s = 0.0;
        for y in y0..y1 {
            for x in x0..x1 {
                s += l.pixel(x, y)[3] as f64 / 255.0;
            }
        }
        s
    }

    #[test]
    fn hard_round_dab() {
        let mut l = TiledLayer::new();
        let mut dirty = HashSet::new();
        paint_dab(&mut l, &round(32.0, 32.0, 10.0, 1.0), [200, 10, 20], 1.0, false, &mut dirty);
        assert_eq!(l.pixel(32, 32), [200, 10, 20, 255]);
        assert_eq!(l.pixel(32, 23), [200, 10, 20, 255], "just inside the edge");
        assert_eq!(l.pixel(32, 20)[3], 0, "outside");
        let area = total_alpha(&l, 0, 0, 64, 64);
        let expected = std::f64::consts::PI * 100.0;
        assert!((area - expected).abs() / expected < 0.02, "area {area} vs {expected}");
        assert_eq!(dirty.len(), 1);
    }

    #[test]
    fn soft_dab_fades_out() {
        let mut l = TiledLayer::new();
        paint_dab(&mut l, &round(32.0, 32.0, 20.0, 0.0), [0, 0, 0], 1.0, false, &mut HashSet::new());
        let a = |x| l.pixel(x, 32)[3];
        assert!(a(32) > 250);
        assert!(a(32) > a(40) && a(40) > a(46) && a(46) > a(50), "{} {} {} {}", a(32), a(40), a(46), a(50));
        assert_eq!(a(53), 0);
    }

    #[test]
    fn dabs_across_tiles_and_negative_coordinates() {
        let mut l = TiledLayer::new();
        let mut dirty = HashSet::new();
        paint_dab(&mut l, &round(0.0, 0.0, 5.0, 1.0), [1, 2, 3], 1.0, false, &mut dirty);
        let mut keys: Vec<_> = dirty.into_iter().collect();
        keys.sort();
        assert_eq!(keys, vec![(-1, -1), (-1, 0), (0, -1), (0, 0)]);
        assert_eq!(l.pixel(-1, -1)[3], 255);
        assert_eq!(l.pixel(0, 0)[3], 255);
        assert_eq!(l.tile_count(), 4);
    }

    #[test]
    fn half_alpha_and_erasing() {
        let mut l = TiledLayer::filled([255, 255, 255, 255]);
        let mut dirty = HashSet::new();
        paint_dab(&mut l, &round(10.0, 10.0, 4.0, 1.0), [0, 0, 0], 0.5, false, &mut dirty);
        assert_eq!(l.pixel(10, 10), [127, 127, 127, 255]);
        paint_dab(&mut l, &round(30.0, 10.0, 4.0, 1.0), [0, 0, 0], 1.0, true, &mut dirty);
        assert_eq!(l.pixel(30, 10)[3], 0, "erased through the white default");
        assert_eq!(l.pixel(40, 10), [255, 255, 255, 255]);
        let mut empty = TiledLayer::new();
        paint_dab(&mut empty, &round(10.0, 10.0, 4.0, 1.0), [0, 0, 0], 1.0, true, &mut dirty);
        assert_eq!(empty.tile_count(), 0, "erasing nothing creates nothing");
    }

    #[test]
    fn big_dabs_on_many_threads_match_small_tile_by_tile_painting() {
        // The same big dab painted whole (threads) and as the union of its tiles' results
        // painted one tile-sized window at a time must agree exactly.
        let shape = DabShape { cx: 300.3, cy: 200.7, rx: 180.0, ry: 120.0, angle: 0.6, hardness: 0.3 };
        let mut base = TiledLayer::new();
        for y in (0..400).step_by(7) {
            for x in (0..600).step_by(5) {
                base.set_pixel(x, y, [x as u8, y as u8, 99, 200]);
            }
        }
        let mut threaded = base.clone();
        let mut dirty = HashSet::new();
        paint_dab(&mut threaded, &shape, [10, 200, 30], 0.7, false, &mut dirty);
        let job_tiles = dirty.len();
        assert!(job_tiles >= PARALLEL_FROM_TILES, "{job_tiles}");
        // Reference: one thread.
        let mut single = base.clone();
        let (sin, cos) = shape.angle.sin_cos();
        let job = Job { d: shape, cos, sin, aa: 1.0 / 120.0, a8_max: 0.7 * 255.0, x0: -1000, x1: 1000, y0: -1000, y1: 1000, color: [10, 200, 30], erase: false, default: [0; 4] };
        let mut mask = Vec::new();
        for &key in &dirty {
            let mut t = single.take_tile(key);
            job.paint_tile(key, &mut t, &mut mask);
            single.put_tile(key, t);
        }
        assert_eq!(threaded, single);
        assert_eq!(threaded.pixel(300, 200)[1], over_px([0, 0, 0, 0], 0.7)[1]);
    }

    #[test]
    fn big_hard_dab_area() {
        let mut l = TiledLayer::new();
        let mut dirty = HashSet::new();
        paint_dab(&mut l, &round(0.0, 0.0, 150.0, 1.0), [0, 0, 0], 1.0, false, &mut dirty);
        let area = total_alpha(&l, -160, -160, 160, 160);
        let expected = std::f64::consts::PI * 150.0 * 150.0;
        assert!((area - expected).abs() / expected < 0.005, "area {area} vs {expected}");
        assert_eq!(dirty.len(), l.tile_count(), "every created tile reported");
        assert!(l.tile_count() <= 36);
    }

    fn over_px(mut p: [u8; 4], a: f32) -> [u8; 4] {
        over(&mut p, [10, 200, 30, (a * 255.0 + 0.5) as u8], 255);
        p
    }

    #[test]
    fn rotated_ellipse() {
        let mut l = TiledLayer::new();
        let shape = DabShape { cx: 32.0, cy: 32.0, rx: 20.0, ry: 4.0, angle: std::f32::consts::FRAC_PI_2, hardness: 1.0 };
        paint_dab(&mut l, &shape, [9, 9, 9], 1.0, false, &mut HashSet::new());
        assert_eq!(l.pixel(32, 48)[3], 255, "long axis now vertical");
        assert_eq!(l.pixel(42, 32)[3], 0, "short axis horizontal");
    }

    #[test]
    fn tiny_dabs_keep_their_ink_and_bad_input_is_ignored() {
        let mut l = TiledLayer::new();
        paint_dab(&mut l, &round(10.0, 10.0, 0.25, 1.0), [0, 0, 0], 1.0, false, &mut HashSet::new());
        let ink = total_alpha(&l, 0, 0, 64, 64);
        let expected = std::f64::consts::PI * 0.25 * 0.25;
        assert!(ink > expected * 0.5 && ink < expected * 2.0, "{ink} vs {expected}");
        let before = l.clone();
        let mut none = HashSet::new();
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            paint_dab(&mut l, &round(bad, 10.0, 5.0, 1.0), [0, 0, 0], 1.0, false, &mut none);
            paint_dab(&mut l, &round(10.0, bad, 5.0, 1.0), [0, 0, 0], 1.0, false, &mut none);
            paint_dab(&mut l, &DabShape { angle: bad, ..round(10.0, 10.0, 5.0, 1.0) }, [0, 0, 0], 1.0, false, &mut none);
        }
        for bad in [f32::NAN, f32::INFINITY, -1.0, 0.0, 1e9] {
            paint_dab(&mut l, &round(10.0, 10.0, bad, 1.0), [0, 0, 0], 1.0, false, &mut none);
        }
        for bad in [f32::NAN, -1.0, 0.0] {
            paint_dab(&mut l, &round(10.0, 10.0, 5.0, 1.0), [0, 0, 0], bad, false, &mut none);
        }
        assert!(none.is_empty());
        assert_eq!(l, before);
    }

    #[test]
    fn coverage_tiles_are_what_a_white_dab_leaves() {
        for shape in [round(32.0, 32.0, 10.0, 1.0), round(100.3, 70.6, 90.0, 0.3), DabShape { cx: 5.0, cy: -3.0, rx: 40.0, ry: 8.0, angle: 0.7, hardness: 0.8 }, round(10.0, 10.0, 0.2, 1.0)] {
            for alpha in [1.0, 0.37] {
                let mut l = TiledLayer::new();
                let mut dirty = HashSet::new();
                paint_dab(&mut l, &shape, [255, 255, 255], alpha, false, &mut dirty);
                let tiles = coverage_tiles(&shape, alpha);
                let keys: HashSet<TileKey> = tiles.iter().map(|(k, _)| *k).collect();
                assert_eq!(keys, dirty, "{shape:?}");
                for (key, cov) in &tiles {
                    let t = l.tile(*key).unwrap();
                    for (i, &c) in cov.iter().enumerate() {
                        assert_eq!(t.0[i * 4 + 3], c, "{shape:?} {key:?} pixel {i}");
                    }
                }
            }
        }
    }

    #[test]
    fn coverage_rows_equal_the_plain_formula() {
        for shape in [round(32.3, 20.7, 10.0, 1.0), round(-5.5, 70.25, 90.0, 0.3), DabShape { cx: 5.0, cy: -3.0, rx: 40.0, ry: 8.0, angle: 0.7, hardness: 0.8 }, round(10.0, 10.0, 0.7, 0.0)] {
            let job = Job::new(&shape, 0.83, [0; 3], false, [0; 4]).unwrap();
            for y in job.y0..job.y1 {
                let mut row = vec![0u8; (job.x1 - job.x0) as usize];
                job.coverage_row(y, job.x0, &mut row);
                for (i, &got) in row.iter().enumerate() {
                    let x = job.x0 + i as i32;
                    let want = (coverage(x as f32 + 0.5 - job.d.cx, y as f32 + 0.5 - job.d.cy, &job.d, job.cos, job.sin, job.aa) * job.a8_max + 0.5) as u8;
                    assert_eq!(got, want, "{shape:?} at ({x}, {y})");
                }
            }
        }
    }
}
