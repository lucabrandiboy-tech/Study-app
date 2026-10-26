//! Retouching tools: they change the pixels already on a layer, under a soft round brush
//! dragged along a stroke. Smudge and push move paint around, blur and sharpen soften or
//! crisp it, dodge and burn lighten or darken it, saturate and desaturate (the sponge)
//! change how colourful it is, recolour gives it the brush colour's hue and saturation while
//! keeping its lightness, clone copies another part of the layer, and spray scatters dots of
//! the brush colour.
//!
//! Colours are mixed with premultiplied alpha, so paint smudged or blurred into a transparent
//! area keeps its colour instead of picking up dark fringes. Only the tiles a dab really
//! changes are created or copied.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::brush::{Brush, BrushDab, DabSpacer};
use crate::curve::Curve;
use crate::dab::{DabShape, paint_dab};
use crate::stabilizer::PaintInfo;
use crate::tiles::{TILE, TileKey, TiledLayer};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Retouch {
    Smudge,
    Blur,
    Sharpen,
    /// Lighten.
    Dodge,
    /// Darken.
    Burn,
    Saturate,
    Desaturate,
    /// The brush colour's hue and saturation, the pixel's own lightness.
    Recolour,
    /// Copies from the same layer at an offset (set by the clone source).
    Clone,
    /// Pushes pixels along with the brush, like Krita's liquify "move".
    Push,
    /// Dots of the brush colour scattered over the brush area.
    Spray,
}

/// The brush the retouch tools share.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RetouchBrush {
    /// Diameter in canvas pixels.
    pub size: f32,
    /// How strong one pass is, 0..=1.
    pub strength: f32,
    /// 0 = soft all the way from the centre, 1 = hard edge.
    pub hardness: f32,
    /// Smudge: how far paint is dragged, 0..=1 (Krita's smudge length).
    pub smudge_length: f32,
    /// Spray: dots per dab.
    pub spray_density: f32,
    /// Spray: dot diameter in canvas pixels.
    pub spray_dot: f32,
}

impl Default for RetouchBrush {
    fn default() -> RetouchBrush {
        RetouchBrush { size: 60.0, strength: 0.5, hardness: 0.3, smudge_length: 0.85, spray_density: 12.0, spray_dot: 4.0 }
    }
}

/// Dabs are this fraction of the brush size apart (spray: further).
const SPACING: f32 = 0.1;
const SPRAY_SPACING: f32 = 0.25;

/// How far blur and sharpen look around each pixel, in canvas pixels: in proportion to the
/// brush, so they show at any painting size (a fixed few pixels is invisible on a 4K canvas).
fn blur_radius(size: f32) -> i32 {
    ((size / 8.0).round() as i32).clamp(1, 64)
}

fn sharpen_radius(size: f32) -> i32 {
    ((size / 32.0).round() as i32).clamp(1, 16)
}

/// One retouch stroke, pen-down to pen-up.
pub struct RetouchStroke {
    kind: Retouch,
    brush: RetouchBrush,
    color: [u8; 3],
    spacer: DabSpacer,
    spacer_brush: Brush,
    /// The canvas: dabs change nothing outside it.
    bounds: (i32, i32),
    /// The layer as the stroke found it (clone copies from it, so it never copies its own
    /// strokes).
    source: TiledLayer,
    /// Clone: the source is this far from where the brush paints.
    clone_offset: (f64, f64),
    /// Smudge: the paint the brush carries, premultiplied, one entry per dab pixel.
    carried: Option<Vec<[f32; 4]>>,
    /// Push: the previous dab's centre.
    last_centre: Option<(f64, f64)>,
    rng: u64,
    dirty: HashSet<TileKey>,
    dabs: usize,
}

impl RetouchStroke {
    /// `layer` is the layer the stroke will change, as it is now; `canvas` its picture's size.
    pub fn new(kind: Retouch, brush: RetouchBrush, color: [u8; 3], layer: &TiledLayer, canvas: (u32, u32), clone_offset: (f64, f64), seed: u64) -> RetouchStroke {
        let size = brush.size.clamp(1.0, 2000.0);
        let spacing = if kind == Retouch::Spray { SPRAY_SPACING } else { SPACING };
        // Pressure scales the strength: a linear opacity curve makes the dab's opacity the
        // pen pressure.
        let spacer_brush = Brush { size, spacing, size_curve: None, opacity: 1.0, opacity_curve: Some(Curve::linear()), flow: 1.0, flow_curve: None, ..Brush::default() };
        RetouchStroke {
            kind,
            brush: RetouchBrush { size, ..brush },
            color,
            spacer: DabSpacer::default(),
            spacer_brush,
            bounds: (canvas.0 as i32, canvas.1 as i32),
            source: layer.clone(),
            clone_offset,
            carried: None,
            last_centre: None,
            rng: seed | 1,
            dirty: HashSet::new(),
            dabs: 0,
        }
    }

    /// A pen point while the pen is down.
    pub fn add(&mut self, layer: &mut TiledLayer, p: PaintInfo) {
        let mut dabs: Vec<BrushDab> = Vec::new();
        self.spacer.add(&self.spacer_brush, p, &mut dabs);
        for d in dabs {
            self.dab(layer, d.shape.cx as f64, d.shape.cy as f64, d.opacity.clamp(0.0, 1.0));
        }
    }

    /// Tiles changed since the last call.
    pub fn take_dirty(&mut self) -> HashSet<TileKey> {
        std::mem::take(&mut self.dirty)
    }

    pub fn dabs(&self) -> usize {
        self.dabs
    }

    fn random(&mut self) -> f64 {
        // xorshift64*
        self.rng ^= self.rng >> 12;
        self.rng ^= self.rng << 25;
        self.rng ^= self.rng >> 27;
        (self.rng.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    }

    fn dab(&mut self, layer: &mut TiledLayer, cx: f64, cy: f64, pressure: f32) {
        self.dabs += 1;
        let strength = (self.brush.strength.clamp(0.0, 1.0) * pressure) as f64;
        let r = self.brush.size as f64 / 2.0;
        if self.kind == Retouch::Spray {
            self.spray(layer, cx, cy, r, strength);
            return;
        }
        let centre = (cx, cy);
        let previous = self.last_centre.replace(centre);
        if strength <= 0.0 {
            return;
        }
        // The dab's pixels: the square around it, clipped to the canvas.
        let (x0, y0) = ((cx - r).floor() as i32, (cy - r).floor() as i32);
        let side = (2.0 * r).ceil() as i32 + 1;
        let (rx0, ry0) = (x0.max(0), y0.max(0));
        let (rx1, ry1) = ((x0 + side).min(self.bounds.0), (y0 + side).min(self.bounds.1));
        if rx0 >= rx1 || ry0 >= ry1 {
            return;
        }
        // How much each pixel is affected: 1 in the middle, fading out to the edge.
        let hardness = self.brush.hardness;
        let weight = |x: i32, y: i32| falloff(((x as f64 + 0.5 - cx).hypot(y as f64 + 0.5 - cy) / r) as f32, hardness) as f64;
        // The incremental tools reach about 75 % of their full effect in one pass at full
        // strength, whatever the brush size (about 1 / SPACING dabs overlap every point).
        let per_dab = 1.0 - (1.0 - 0.75 * strength).powf(SPACING as f64);
        let margin = match self.kind {
            // Two box blurs reach twice their radius.
            Retouch::Blur => 2 * blur_radius(self.brush.size),
            Retouch::Sharpen => sharpen_radius(self.brush.size),
            Retouch::Push => match previous {
                Some(p) => ((cx - p.0).hypot(cy - p.1) * strength).ceil() as i32 + 2,
                None => return,
            },
            _ => 0,
        };
        let (w, h) = (rx1 - rx0, ry1 - ry0);
        // Read with a margin (blur and push look at neighbours), clamped at the canvas edge.
        let (mx0, my0) = ((rx0 - margin).max(0), (ry0 - margin).max(0));
        let (mx1, my1) = ((rx1 + margin).min(self.bounds.0), (ry1 + margin).min(self.bounds.1));
        let (mw, mh) = (mx1 - mx0, my1 - my0);
        let src = read_region(layer, mx0, my0, mw, mh);
        let at = |x: i32, y: i32| src[((y.clamp(my0, my1 - 1) - my0) * mw + (x.clamp(mx0, mx1 - 1) - mx0)) as usize];
        let old: Vec<[u8; 4]> = (ry0..ry1).flat_map(|y| (rx0..rx1).map(move |x| (x, y))).map(|(x, y)| at(x, y)).collect();
        let mut new = old.clone();
        match self.kind {
            Retouch::Blur => {
                let blurred = box_blur(&src, mw, mh, blur_radius(self.brush.size), 2);
                for (i, (x, y)) in (ry0..ry1).flat_map(|y| (rx0..rx1).map(move |x| (x, y))).enumerate() {
                    let t = weight(x, y) * per_dab;
                    if t > 0.0 {
                        new[i] = from_pm(mix(to_pm(old[i]), blurred[((y - my0) * mw + (x - mx0)) as usize], t as f32));
                    }
                }
            }
            Retouch::Sharpen => {
                let around = box_blur(&src, mw, mh, sharpen_radius(self.brush.size), 1);
                for (i, (x, y)) in (ry0..ry1).flat_map(|y| (rx0..rx1).map(move |x| (x, y))).enumerate() {
                    let t = weight(x, y) * per_dab;
                    let (p, a) = (old[i], around[((y - my0) * mw + (x - mx0)) as usize]);
                    if t <= 0.0 || p[3] == 0 || a[3] <= 0.0 {
                        continue;
                    }
                    // Unsharp mask: colour pushed away from the colour around it (transparent
                    // pixels around count for nothing, so the edge of paint gets no halo).
                    for c in 0..3 {
                        let (v, avg) = (p[c] as f32, a[c] * 255.0 / a[3]);
                        new[i][c] = to_u8(v + (v - avg) * 1.5 * t as f32);
                    }
                }
            }
            Retouch::Dodge | Retouch::Burn | Retouch::Saturate | Retouch::Desaturate | Retouch::Recolour => {
                let target = self.color;
                for (i, (x, y)) in (ry0..ry1).flat_map(|y| (rx0..rx1).map(move |x| (x, y))).enumerate() {
                    let t = (weight(x, y) * per_dab) as f32;
                    let p = old[i];
                    if t <= 0.0 || p[3] == 0 {
                        continue;
                    }
                    let rgb = [p[0] as f32, p[1] as f32, p[2] as f32];
                    let goal = match self.kind {
                        Retouch::Dodge => rgb.map(|v| v + (255.0 - v)),
                        Retouch::Burn => [0.0; 3],
                        Retouch::Saturate => {
                            let y = luma(rgb);
                            rgb.map(|v| y + (v - y) * 2.0)
                        }
                        Retouch::Desaturate => [luma(rgb); 3],
                        _ => set_luma([target[0] as f32, target[1] as f32, target[2] as f32], luma(rgb)),
                    };
                    for c in 0..3 {
                        new[i][c] = to_u8(rgb[c] + (goal[c] - rgb[c]) * t);
                    }
                }
            }
            Retouch::Smudge => {
                let length = self.brush.smudge_length.clamp(0.0, 0.99);
                // The brush picks up the paint under it at its first dab.
                let carried = self.carried.get_or_insert_with(|| {
                    (0..side * side).map(|i| to_pm(layer.pixel(x0 + i % side, y0 + i / side))).collect()
                });
                for (i, (x, y)) in (ry0..ry1).flat_map(|y| (rx0..rx1).map(move |x| (x, y))).enumerate() {
                    let wgt = weight(x, y) as f32;
                    if wgt <= 0.0 {
                        continue;
                    }
                    let k = ((y - y0) * side + (x - x0)) as usize;
                    let here = to_pm(old[i]);
                    new[i] = from_pm(mix(here, carried[k], wgt * strength as f32));
                    // It leaves some of what it carries and takes up some of the canvas.
                    carried[k] = mix(carried[k], here, 1.0 - length);
                }
            }
            Retouch::Clone => {
                let (ox, oy) = (self.clone_offset.0.round() as i32, self.clone_offset.1.round() as i32);
                for (i, (x, y)) in (ry0..ry1).flat_map(|y| (rx0..rx1).map(move |x| (x, y))).enumerate() {
                    let t = (weight(x, y) * strength) as f32;
                    if t <= 0.0 {
                        continue;
                    }
                    let from = to_pm(self.source.pixel(x + ox, y + oy));
                    new[i] = from_pm(mix(to_pm(old[i]), from, t));
                }
            }
            Retouch::Push => {
                let Some(prev) = previous else { return };
                let (dx, dy) = ((cx - prev.0) * strength, (cy - prev.1) * strength);
                for (i, (x, y)) in (ry0..ry1).flat_map(|y| (rx0..rx1).map(move |x| (x, y))).enumerate() {
                    let wgt = weight(x, y);
                    if wgt <= 0.0 {
                        continue;
                    }
                    // Where this pixel's new colour comes from: behind it along the push.
                    let (sx, sy) = (x as f64 + 0.5 - dx * wgt, y as f64 + 0.5 - dy * wgt);
                    new[i] = from_pm(bilinear(&at, sx, sy));
                }
            }
            Retouch::Spray => unreachable!("handled above"),
        }
        write_region(layer, rx0, ry0, w, h, &old, &new, &mut self.dirty);
    }

    fn spray(&mut self, layer: &mut TiledLayer, cx: f64, cy: f64, r: f64, strength: f64) {
        if strength <= 0.0 {
            return;
        }
        let n = self.brush.spray_density.clamp(1.0, 200.0).round() as usize;
        let dot = (self.brush.spray_dot.clamp(0.5, 200.0) / 2.0) as f64;
        for _ in 0..n {
            // Evenly over the brush's disc.
            let (u, v, s) = (self.random(), self.random(), self.random());
            let (dist, angle) = (r * u.sqrt(), v * std::f64::consts::TAU);
            let (x, y) = (cx + dist * angle.cos(), cy + dist * angle.sin());
            if x < 0.0 || y < 0.0 || x >= self.bounds.0 as f64 || y >= self.bounds.1 as f64 {
                continue;
            }
            let rad = (dot * (0.6 + 0.4 * s)) as f32;
            let shape = DabShape { cx: x as f32, cy: y as f32, rx: rad, ry: rad, angle: 0.0, hardness: 0.8 };
            paint_dab(layer, &shape, self.color, strength as f32, false, &mut self.dirty);
        }
    }
}

/// A region (`w` x `h`, row-major) blurred by `passes` box blurs of radius `r` across and
/// down (two make a soft, nearly Gaussian blur), premultiplied; its edge pixels are repeated
/// beyond it.
fn box_blur(src: &[[u8; 4]], w: i32, h: i32, r: i32, passes: usize) -> Vec<[f32; 4]> {
    let (w, h) = (w as usize, h as usize);
    let mut data: Vec<[f32; 4]> = src.iter().map(|&p| to_pm(p)).collect();
    let mut line = Vec::new();
    for _ in 0..passes {
        box_pass(&mut data, h, w, w, 1, r as isize, &mut line);
        box_pass(&mut data, w, h, 1, w, r as isize, &mut line);
    }
    data
}

/// One box blur along `lines` lines of `len` pixels: pixel `i` of line `l` is
/// `data[l * line_step + i * step]`. A running sum, so any radius costs the same.
fn box_pass(data: &mut [[f32; 4]], lines: usize, len: usize, line_step: usize, step: usize, r: isize, line: &mut Vec<[f32; 4]>) {
    let n = (2 * r + 1) as f32;
    let last = len as isize - 1;
    for l in 0..lines {
        let base = l * line_step;
        line.clear();
        line.extend((0..len).map(|i| data[base + i * step]));
        let at = |i: isize| line[i.clamp(0, last) as usize];
        let mut sum = [0f32; 4];
        for i in -r..=r {
            let p = at(i);
            for c in 0..4 {
                sum[c] += p[c];
            }
        }
        for i in 0..len as isize {
            data[base + i as usize * step] = sum.map(|v| v / n);
            let (add, sub) = (at(i + r + 1), at(i - r));
            for c in 0..4 {
                sum[c] += add[c] - sub[c];
            }
        }
    }
}

/// 1 in the middle of the brush, falling smoothly to 0 at its edge (`d` = distance / radius).
fn falloff(d: f32, hardness: f32) -> f32 {
    if d >= 1.0 {
        return 0.0;
    }
    let h = hardness.clamp(0.0, 0.99);
    if d <= h {
        1.0
    } else {
        let t = (d - h) / (1.0 - h);
        1.0 - t * t * (3.0 - 2.0 * t)
    }
}

fn to_pm(p: [u8; 4]) -> [f32; 4] {
    let a = p[3] as f32 / 255.0;
    [p[0] as f32 * a, p[1] as f32 * a, p[2] as f32 * a, p[3] as f32]
}

fn from_pm(q: [f32; 4]) -> [u8; 4] {
    let a = q[3];
    if a < 0.5 {
        return [0; 4];
    }
    let k = 255.0 / a;
    [to_u8(q[0] * k), to_u8(q[1] * k), to_u8(q[2] * k), to_u8(a)]
}

/// Rounded and clamped to 0..=255.
fn to_u8(v: f32) -> u8 {
    (v + 0.5).clamp(0.0, 255.0) as u8
}

fn mix(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t, a[3] + (b[3] - a[3]) * t]
}

/// The colour at a point between pixel centres, premultiplied.
fn bilinear(at: &impl Fn(i32, i32) -> [u8; 4], x: f64, y: f64) -> [f32; 4] {
    let (fx, fy) = (x - 0.5, y - 0.5);
    let (x0, y0) = (fx.floor() as i32, fy.floor() as i32);
    let (tx, ty) = ((fx - x0 as f64) as f32, (fy - y0 as f64) as f32);
    let top = mix(to_pm(at(x0, y0)), to_pm(at(x0 + 1, y0)), tx);
    let bottom = mix(to_pm(at(x0, y0 + 1)), to_pm(at(x0 + 1, y0 + 1)), tx);
    mix(top, bottom, ty)
}

/// Perceived lightness (Rec. 601 weights, as the W3C blend modes use).
fn luma(c: [f32; 3]) -> f32 {
    0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
}

/// `c` moved to lightness `l`, kept inside 0..=255 without changing its hue (the W3C
/// compositing spec's SetLum and ClipColor, as Krita's "Color" blend mode uses).
fn set_luma(c: [f32; 3], l: f32) -> [f32; 3] {
    let d = l - luma(c);
    let c = c.map(|v| v + d);
    let l = luma(c);
    let (n, x) = (c[0].min(c[1]).min(c[2]), c[0].max(c[1]).max(c[2]));
    let mut out = c;
    if n < 0.0 {
        out = out.map(|v| l + (v - l) * l / (l - n).max(1e-6));
    }
    if x > 255.0 {
        out = out.map(|v| l + (v - l) * (255.0 - l) / (x - l).max(1e-6));
    }
    out
}

/// The pixels of a rectangle, row-major; the layer's default where it has no tile.
fn read_region(layer: &TiledLayer, x0: i32, y0: i32, w: i32, h: i32) -> Vec<[u8; 4]> {
    let mut out = vec![layer.default; (w * h) as usize];
    for ty in y0.div_euclid(TILE)..=(y0 + h - 1).div_euclid(TILE) {
        for tx in x0.div_euclid(TILE)..=(x0 + w - 1).div_euclid(TILE) {
            let Some(tile) = layer.tile((tx, ty)) else { continue };
            let (ax, ay) = ((tx * TILE).max(x0), (ty * TILE).max(y0));
            let (bx, by) = (((tx + 1) * TILE).min(x0 + w), ((ty + 1) * TILE).min(y0 + h));
            for y in ay..by {
                let start = (((y - ty * TILE) * TILE + (ax - tx * TILE)) * 4) as usize;
                let row = tile.0[start..start + ((bx - ax) * 4) as usize].as_chunks::<4>().0;
                let at = ((y - y0) * w + (ax - x0)) as usize;
                out[at..at + row.len()].copy_from_slice(row);
            }
        }
    }
    out
}

/// Writes `new` where it differs from `old` (both a rectangle's pixels, row-major). Tiles are
/// only created or copied where something changes; those are added to `dirty`.
#[allow(clippy::too_many_arguments)]
fn write_region(layer: &mut TiledLayer, x0: i32, y0: i32, w: i32, h: i32, old: &[[u8; 4]], new: &[[u8; 4]], dirty: &mut HashSet<TileKey>) {
    for ty in y0.div_euclid(TILE)..=(y0 + h - 1).div_euclid(TILE) {
        for tx in x0.div_euclid(TILE)..=(x0 + w - 1).div_euclid(TILE) {
            let (ax, ay) = ((tx * TILE).max(x0), (ty * TILE).max(y0));
            let (bx, by) = (((tx + 1) * TILE).min(x0 + w), ((ty + 1) * TILE).min(y0 + h));
            let idx = |x: i32, y: i32| ((y - y0) * w + (x - x0)) as usize;
            if !(ay..by).any(|y| (ax..bx).any(|x| old[idx(x, y)] != new[idx(x, y)])) {
                continue;
            }
            let tile = layer.tile_mut((tx, ty));
            for y in ay..by {
                for x in ax..bx {
                    let i = idx(x, y);
                    if old[i] != new[i] {
                        tile.set(x - tx * TILE, y - ty * TILE, new[i]);
                    }
                }
            }
            dirty.insert((tx, ty));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f64, y: f64) -> PaintInfo {
        PaintInfo::new(x, y, 1.0)
    }

    /// Drags a retouch stroke in a straight line, in `n` steps.
    fn stroke(kind: Retouch, brush: RetouchBrush, color: [u8; 3], layer: &mut TiledLayer, a: (f64, f64), b: (f64, f64), offset: (f64, f64)) -> HashSet<TileKey> {
        let mut s = RetouchStroke::new(kind, brush, color, layer, (400, 300), offset, 7);
        let n = 40;
        for i in 0..=n {
            let t = i as f64 / n as f64;
            s.add(layer, pt(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
        }
        s.take_dirty()
    }

    fn filled(px: [u8; 4]) -> TiledLayer {
        let mut l = TiledLayer::new();
        for y in 0..300 {
            for x in 0..400 {
                l.set_pixel(x, y, px);
            }
        }
        l
    }

    fn brush(strength: f32) -> RetouchBrush {
        RetouchBrush { size: 30.0, strength, hardness: 0.5, ..RetouchBrush::default() }
    }

    #[test]
    fn blur_softens_an_edge_and_only_under_the_brush() {
        // Black on the left, white on the right; blur along the edge.
        let mut l = filled([255, 255, 255, 255]);
        for y in 0..300 {
            for x in 0..200 {
                l.set_pixel(x, y, [0, 0, 0, 255]);
            }
        }
        let dirty = stroke(Retouch::Blur, brush(1.0), [0; 3], &mut l, (200.0, 50.0), (200.0, 250.0), (0.0, 0.0));
        let (left, right) = (l.pixel(199, 150)[0], l.pixel(200, 150)[0]);
        assert!(left > 30 && right < 225, "edge softened: {left} {right}");
        assert!(left < right, "still darker on the left");
        assert_eq!(l.pixel(150, 150), [0, 0, 0, 255], "outside the brush: unchanged");
        assert_eq!(l.pixel(200, 10), [255, 255, 255, 255]);
        assert!(!dirty.is_empty() && dirty.iter().all(|&(tx, _)| (2..=3).contains(&tx)), "{dirty:?}");
    }

    #[test]
    fn blur_reaches_further_with_a_bigger_brush() {
        // The same edge blurred with a 30 px and a 120 px brush: the big one softens it 10 px
        // out on both sides (a blur of a fixed few pixels would not show on a 4K canvas).
        let blurred = |size: f32| {
            let mut l = filled([255, 255, 255, 255]);
            for y in 0..300 {
                for x in 0..200 {
                    l.set_pixel(x, y, [0, 0, 0, 255]);
                }
            }
            let b = RetouchBrush { size, strength: 0.5, hardness: 0.3, ..RetouchBrush::default() };
            stroke(Retouch::Blur, b, [0; 3], &mut l, (200.0, 20.0), (200.0, 280.0), (0.0, 0.0));
            (l.pixel(190, 150)[0], l.pixel(209, 150)[0])
        };
        assert_eq!(blurred(30.0), (0, 255));
        let (dark, light) = blurred(120.0);
        assert!(dark > 8 && light < 247, "{dark} {light}");
    }

    #[test]
    fn sharpen_leaves_no_halo_at_the_edge_of_paint() {
        // A flat grey square on a transparent layer: nothing around it to sharpen against.
        let mut l = TiledLayer::new();
        for y in 50..250 {
            for x in 100..300 {
                l.set_pixel(x, y, [128, 128, 128, 255]);
            }
        }
        stroke(Retouch::Sharpen, brush(1.0), [0; 3], &mut l, (100.0, 60.0), (100.0, 240.0), (0.0, 0.0));
        for x in 100..120 {
            assert_eq!(l.pixel(x, 150), [128, 128, 128, 255], "x {x}");
        }
        assert_eq!(l.pixel(99, 150), [0; 4]);
    }

    #[test]
    fn sharpen_adds_contrast_at_an_edge() {
        let mut l = filled([100, 100, 100, 255]);
        for y in 0..300 {
            for x in 200..400 {
                l.set_pixel(x, y, [150, 150, 150, 255]);
            }
        }
        stroke(Retouch::Sharpen, brush(1.0), [0; 3], &mut l, (200.0, 50.0), (200.0, 250.0), (0.0, 0.0));
        let (dark, light) = (l.pixel(199, 150)[0], l.pixel(200, 150)[0]);
        assert!(dark < 100 && light > 150, "both sides pushed apart: {dark} {light}");
        assert_eq!(l.pixel(150, 150)[0], 100, "flat areas stay as they are");
    }

    #[test]
    fn dodge_lightens_burn_darkens_and_neither_touches_alpha_or_empty_areas() {
        let mut l = filled([128, 64, 32, 200]);
        stroke(Retouch::Dodge, brush(1.0), [0; 3], &mut l, (50.0, 100.0), (350.0, 100.0), (0.0, 0.0));
        let p = l.pixel(200, 100);
        assert!(p[0] > 180 && p[1] > 120 && p[2] > 100 && p[3] == 200, "{p:?}");
        stroke(Retouch::Burn, brush(1.0), [0; 3], &mut l, (50.0, 200.0), (350.0, 200.0), (0.0, 0.0));
        let p = l.pixel(200, 200);
        assert!(p[0] < 80 && p[1] < 40 && p[2] < 20 && p[3] == 200, "{p:?}");
        assert_eq!(l.pixel(200, 150), [128, 64, 32, 200], "between the strokes: unchanged");
        // Nothing to lighten on an empty layer: no tiles are made.
        let mut empty = TiledLayer::new();
        let dirty = stroke(Retouch::Dodge, brush(1.0), [0; 3], &mut empty, (50.0, 100.0), (350.0, 100.0), (0.0, 0.0));
        assert_eq!((empty.tile_count(), dirty.len()), (0, 0));
    }

    #[test]
    fn strength_scales_the_effect() {
        let mut weak = filled([128, 128, 128, 255]);
        let mut strong = weak.clone();
        stroke(Retouch::Dodge, brush(0.2), [0; 3], &mut weak, (50.0, 100.0), (350.0, 100.0), (0.0, 0.0));
        stroke(Retouch::Dodge, brush(1.0), [0; 3], &mut strong, (50.0, 100.0), (350.0, 100.0), (0.0, 0.0));
        let (w, s) = (weak.pixel(200, 100)[0], strong.pixel(200, 100)[0]);
        assert!(128 < w && w < s, "{w} {s}");
        let mut none = filled([128, 128, 128, 255]);
        let dirty = stroke(Retouch::Dodge, brush(0.0), [0; 3], &mut none, (50.0, 100.0), (350.0, 100.0), (0.0, 0.0));
        assert!(dirty.is_empty() && none.pixel(200, 100) == [128, 128, 128, 255]);
    }

    #[test]
    fn sponge_and_recolour() {
        let sat = |p: [u8; 4]| p[0].max(p[1]).max(p[2]) - p[0].min(p[1]).min(p[2]);
        let mut l = filled([160, 110, 100, 255]);
        let before = sat(l.pixel(200, 100));
        stroke(Retouch::Saturate, brush(1.0), [0; 3], &mut l, (50.0, 100.0), (350.0, 100.0), (0.0, 0.0));
        stroke(Retouch::Desaturate, brush(1.0), [0; 3], &mut l, (50.0, 200.0), (350.0, 200.0), (0.0, 0.0));
        assert!(sat(l.pixel(200, 100)) > before + 20, "{:?}", l.pixel(200, 100));
        // One pass at full strength takes it about two thirds of the way to grey.
        assert!(sat(l.pixel(200, 200)) < before / 2, "{:?}", l.pixel(200, 200));
        // Recolour a red area blue: blue wins, the lightness stays about the same.
        let mut l = filled([200, 40, 40, 255]);
        stroke(Retouch::Recolour, RetouchBrush { strength: 1.0, hardness: 1.0, ..brush(1.0) }, [0, 0, 255], &mut l, (50.0, 100.0), (350.0, 100.0), (0.0, 0.0));
        let p = l.pixel(200, 100);
        assert!(p[2] > p[0] && p[2] > p[1], "{p:?}");
        let lum = |p: [u8; 4]| luma([p[0] as f32, p[1] as f32, p[2] as f32]);
        assert!((lum(p) - lum([200, 40, 40, 255])).abs() < 12.0, "{p:?}");
    }

    #[test]
    fn smudge_drags_paint_along_the_stroke() {
        // A red square at the left; smudge from inside it to the right.
        let mut l = filled([255, 255, 255, 255]);
        for y in 80..220 {
            for x in 40..140 {
                l.set_pixel(x, y, [220, 0, 0, 255]);
            }
        }
        stroke(Retouch::Smudge, RetouchBrush { strength: 0.8, ..brush(0.8) }, [0; 3], &mut l, (110.0, 150.0), (260.0, 150.0), (0.0, 0.0));
        let near = l.pixel(160, 150);
        let far = l.pixel(240, 150);
        assert!(near[1] < 200 && near[0] > near[1], "red dragged into the white: {near:?}");
        assert!(far[1] > near[1], "less of it further on: {near:?} {far:?}");
        assert_eq!(l.pixel(160, 100), [255, 255, 255, 255], "beside the stroke: untouched");
    }

    #[test]
    fn clone_copies_the_source_with_a_hard_brush() {
        let mut l = TiledLayer::new();
        for y in 0..60 {
            for x in 0..60 {
                l.set_pixel(x + 20, y + 20, [(x * 4) as u8, (y * 4) as u8, 77, 255]);
            }
        }
        let b = RetouchBrush { size: 30.0, strength: 1.0, hardness: 1.0, ..RetouchBrush::default() };
        // Paint at x 220.. copying from x 20.. (offset -200).
        stroke(Retouch::Clone, b, [0; 3], &mut l, (230.0, 50.0), (270.0, 50.0), (-200.0, 0.0));
        for x in 230..270 {
            assert_eq!(l.pixel(x, 50), l.pixel(x - 200, 50), "x {x}");
        }
        assert_eq!(l.pixel(250, 90), [0, 0, 0, 0], "outside the brush nothing is copied");
    }

    #[test]
    fn push_moves_pixels_the_way_the_brush_goes() {
        // A black vertical line at x 100; push across it to the right.
        let mut l = filled([255, 255, 255, 255]);
        for y in 0..300 {
            for x in 98..102 {
                l.set_pixel(x, y, [0, 0, 0, 255]);
            }
        }
        stroke(Retouch::Push, RetouchBrush { size: 40.0, strength: 1.0, hardness: 0.6, ..RetouchBrush::default() }, [0; 3], &mut l, (90.0, 150.0), (130.0, 150.0), (0.0, 0.0));
        let dark = |x: i32| l.pixel(x, 150)[0] < 128;
        assert!((104..140).any(dark), "the line was pushed right where the brush went");
        assert!(dark(100) || (102..140).any(dark));
        assert!(l.pixel(100, 20)[0] < 128, "away from the brush the line stays");
    }

    #[test]
    fn spray_scatters_dots_inside_the_brush() {
        let mut l = TiledLayer::new();
        let b = RetouchBrush { size: 40.0, strength: 1.0, spray_density: 10.0, spray_dot: 3.0, ..RetouchBrush::default() };
        stroke(Retouch::Spray, b, [0, 120, 255], &mut l, (100.0, 150.0), (300.0, 150.0), (0.0, 0.0));
        let mut inside = 0;
        for y in 0..300 {
            for x in 0..400 {
                let p = l.pixel(x, y);
                if p[3] > 0 {
                    assert!((y as f64 - 150.0).abs() <= 24.0 && (76..=324).contains(&x), "dot outside the brush at ({x}, {y})");
                    assert_eq!(p[0..3], [0, 120, 255], "the brush colour");
                    inside += 1;
                }
            }
        }
        // Dots, not a solid band: some of the band is painted, not all of it.
        let band = 200 * 40;
        assert!(inside > band / 20 && inside < band * 9 / 10, "{inside} of {band}");
    }

    #[test]
    fn region_reads_and_writes_across_tiles() {
        let mut l = TiledLayer::filled([1, 2, 3, 4]);
        l.set_pixel(63, 63, [9, 9, 9, 9]);
        let r = read_region(&l, 60, 60, 10, 10);
        assert_eq!(r[3 * 10 + 3], [9, 9, 9, 9]);
        assert_eq!(r[0], [1, 2, 3, 4], "default where there is no tile");
        let mut new = r.clone();
        new[5 * 10 + 5] = [7, 7, 7, 7];
        let mut dirty = HashSet::new();
        write_region(&mut l, 60, 60, 10, 10, &r, &new, &mut dirty);
        assert_eq!(l.pixel(65, 65), [7, 7, 7, 7]);
        assert_eq!(dirty, [(1, 1)].into_iter().collect(), "only the tile that changed");
        assert_eq!(l.tile_count(), 2);
    }
}
