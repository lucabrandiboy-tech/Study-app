//! Painting one brush dab into a layer's tiles (the CPU side of painting, as in Krita: the
//! tiles are the picture, the GPU only shows them).
//!
//! The dab looks like the tablet test app's GPU dab (`shaders.wgsl`): an ellipse with about
//! one pixel of anti-aliasing at the edge and, below hardness 1, a smooth fall-off from
//! `hardness` x radius to the edge.

use std::collections::HashSet;

use crate::doc::{mul8, over};
use crate::tiles::{TILE, TILE_PIXELS, TileKey, TiledLayer};

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

/// Coverage (0..=1) of a pixel centre at offset (dx, dy) from the dab centre.
#[inline]
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
    let radius_ok = |r: f32| r > 0.0 && r <= MAX_RADIUS;
    if alpha.is_nan() || alpha <= 0.0 || !shape.cx.is_finite() || !shape.cy.is_finite() || !radius_ok(shape.rx) || !radius_ok(shape.ry) || !shape.angle.is_finite() {
        return;
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
    let a8_max = alpha * 255.0;
    let mut mask: Vec<u8> = Vec::with_capacity(TILE_PIXELS);
    for row in y0.div_euclid(TILE)..=(y1 - 1).div_euclid(TILE) {
        for col in x0.div_euclid(TILE)..=(x1 - 1).div_euclid(TILE) {
            let key = (col, row);
            if erase && layer.tile(key).is_none() && layer.default[3] == 0 {
                continue;
            }
            let (tx0, ty0) = (col * TILE, row * TILE);
            let xs = x0.max(tx0)..x1.min(tx0 + TILE);
            let ys = y0.max(ty0)..y1.min(ty0 + TILE);
            // Coverage first, so tiles the dab only grazes with zero coverage are not created.
            let w = (xs.end - xs.start).max(0) as usize;
            mask.clear();
            for y in ys.clone() {
                let dy = y as f32 + 0.5 - d.cy;
                mask.extend(xs.clone().map(|x| (coverage(x as f32 + 0.5 - d.cx, dy, &d, cos, sin, aa) * a8_max + 0.5) as u8));
            }
            if mask.iter().any(|&a| a != 0) {
                let t = layer.tile_mut(key);
                for (i, &a8) in mask.iter().enumerate().filter(|(_, a)| **a != 0) {
                    let (px, py) = (xs.start - tx0 + (i % w) as i32, ys.start - ty0 + (i / w) as i32);
                    let mut p = t.get(px, py);
                    if erase {
                        p[3] -= mul8(p[3], a8);
                    } else {
                        over(&mut p, [color[0], color[1], color[2], a8], 255);
                    }
                    t.set(px, py, p);
                }
                dirty.insert(key);
            }
        }
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
}
