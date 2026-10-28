//! Selections, and the operations that move or cut pixels.
//!
//! A selection is a [`Mask`] over the canvas: 255 = selected, 0 = not, in between for soft
//! edges. While there is one, tools only change selected pixels: after a tool changed some
//! tiles, [`clip_to`] puts back what was there outside the selection. The move tool carries
//! the selected pixels (or the whole layer), crop cuts the canvas down.

use serde::{Deserialize, Serialize};

use crate::doc::{Document, mul8, over};
use crate::fill::Mask;
use crate::tiles::{TILE, TileKey, TiledLayer};

/// How a new selection goes with the one there already is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Combine {
    #[default]
    Replace,
    Add,
    Subtract,
    Intersect,
}

/// The selection after selecting `new` with `mode`; `None` when nothing is left selected.
pub fn combine(current: Option<&Mask>, new: Mask, mode: Combine) -> Option<Mask> {
    let mut out = match (mode, current) {
        (Combine::Replace, _) | (Combine::Add, None) => new,
        (Combine::Subtract | Combine::Intersect, None) => return None,
        (_, Some(cur)) => {
            let mut m = cur.clone();
            for (o, &n) in m.data.iter_mut().zip(&new.data) {
                *o = match mode {
                    Combine::Add => (*o).max(n),
                    Combine::Subtract => mul8(*o, 255 - n),
                    _ => (*o).min(n),
                };
            }
            m
        }
    };
    out.update_bounds();
    (!out.is_empty()).then_some(out)
}

/// Everything not selected (and the other way round); `None` if that is nothing.
pub fn invert(mask: &Mask) -> Option<Mask> {
    let mut m = mask.clone();
    m.data.iter_mut().for_each(|v| *v = 255 - *v);
    m.update_bounds();
    (!m.is_empty()).then_some(m)
}

/// The whole canvas.
pub fn all(width: u32, height: u32) -> Mask {
    let mut m = Mask::new(width, height);
    m.data.fill(255);
    m.update_bounds();
    m
}

/// The selection's outline: the borders between selected and unselected pixels (taking a
/// pixel as selected from half coverage), merged into straight runs, in canvas pixels.
/// `None` if it takes more than `max_segments` (too intricate to draw).
pub fn outline(mask: &Mask, max_segments: usize) -> Option<Vec<[(f64, f64); 2]>> {
    let mut out = Vec::new();
    if mask.is_empty() {
        return Some(out);
    }
    let (x0, y0, x1, y1) = mask.bounds;
    let (x0, y0, x1, y1) = (x0 as i32, y0 as i32, x1 as i32, y1 as i32);
    let on = |x: i32, y: i32| mask.get(x, y) >= 128;
    // Borders along rows (between y - 1 and y), then along columns.
    for y in y0..=y1 {
        let mut run: Option<i32> = None;
        for x in x0..=x1 {
            let edge = x < x1 && on(x, y - 1) != on(x, y);
            match (edge, run) {
                (true, None) => run = Some(x),
                (false, Some(s)) => {
                    out.push([(s as f64, y as f64), (x as f64, y as f64)]);
                    run = None;
                }
                _ => {}
            }
        }
        if out.len() > max_segments {
            return None;
        }
    }
    for x in x0..=x1 {
        let mut run: Option<i32> = None;
        for y in y0..=y1 {
            let edge = y < y1 && on(x - 1, y) != on(x, y);
            match (edge, run) {
                (true, None) => run = Some(y),
                (false, Some(s)) => {
                    out.push([(x as f64, s as f64), (x as f64, y as f64)]);
                    run = None;
                }
                _ => {}
            }
        }
        if out.len() > max_segments {
            return None;
        }
    }
    Some(out)
}

/// After a tool changed `tiles` of `layer`: outside the selection the pixels get back what
/// they were in `before` (the same layer before the tool); in its soft edge, a mix.
pub fn clip_to(layer: &mut TiledLayer, before: &TiledLayer, mask: &Mask, tiles: impl IntoIterator<Item = TileKey>) {
    for key in tiles {
        let (px, py) = (key.0 * TILE, key.1 * TILE);
        let mut all = true;
        let mut none = true;
        for y in py..py + TILE {
            for x in px..px + TILE {
                let m = mask.get(x, y);
                all &= m == 255;
                none &= m == 0;
            }
        }
        if all {
            continue;
        }
        if none {
            // Nothing of this tile is selected: it is what it was (shared again with `before`).
            layer.put_tile(key, before.tile(key).cloned());
            continue;
        }
        let old = before.tile(key).cloned();
        let default = before.default;
        let tile = layer.tile_mut(key);
        for ty in 0..TILE {
            for tx in 0..TILE {
                let m = mask.get(px + tx, py + ty);
                if m == 255 {
                    continue;
                }
                let was = old.as_ref().map_or(default, |t| t.get(tx, ty));
                let now = tile.get(tx, ty);
                tile.set(tx, ty, if m == 0 { was } else { mix_premultiplied(was, now, m) });
            }
        }
    }
}

/// `a` towards `b` by `t` / 255, mixing premultiplied colours (straight alpha in and out).
fn mix_premultiplied(a: [u8; 4], b: [u8; 4], t: u8) -> [u8; 4] {
    let t = t as f32 / 255.0;
    let (aa, ba) = (a[3] as f32, b[3] as f32);
    let alpha = aa + (ba - aa) * t;
    if alpha < 0.5 {
        return [0; 4];
    }
    let mut out = [0u8; 4];
    for c in 0..3 {
        let (pa, pb) = (a[c] as f32 * aa, b[c] as f32 * ba);
        out[c] = ((pa + (pb - pa) * t) / alpha + 0.5).clamp(0.0, 255.0) as u8;
    }
    out[3] = (alpha + 0.5) as u8;
    out
}

/// The layer's pixels moved by (dx, dy). Pixels moved off the canvas are kept (as in Krita,
/// moving them back brings them back) until the canvas is cropped.
pub fn offset_layer(layer: &TiledLayer, dx: i32, dy: i32) -> TiledLayer {
    let mut out = TiledLayer::filled(layer.default);
    if dx.rem_euclid(TILE) == 0 && dy.rem_euclid(TILE) == 0 {
        // Whole tiles: only their places change.
        for k in layer.tile_keys() {
            out.put_tile((k.0 + dx / TILE, k.1 + dy / TILE), layer.tile(k).cloned());
        }
        return out;
    }
    for k in layer.tile_keys() {
        let src = layer.tile(k).expect("listed").clone();
        let (x0, y0) = (k.0 * TILE + dx, k.1 * TILE + dy);
        for ty in y0.div_euclid(TILE)..=(y0 + TILE - 1).div_euclid(TILE) {
            for tx in x0.div_euclid(TILE)..=(x0 + TILE - 1).div_euclid(TILE) {
                let dst = out.tile_mut((tx, ty));
                let (ax, ay) = ((tx * TILE).max(x0), (ty * TILE).max(y0));
                let (bx, by) = (((tx + 1) * TILE).min(x0 + TILE), ((ty + 1) * TILE).min(y0 + TILE));
                let n = ((bx - ax) * 4) as usize;
                for y in ay..by {
                    let s = (((y - y0) * TILE + (ax - x0)) * 4) as usize;
                    let d = (((y - ty * TILE) * TILE + (ax - tx * TILE)) * 4) as usize;
                    dst.0[d..d + n].copy_from_slice(&src.0[s..s + n]);
                }
            }
        }
    }
    out
}

/// The mask moved by (dx, dy), on the same canvas (what moves off it is dropped).
pub fn offset_mask(mask: &Mask, dx: i32, dy: i32) -> Mask {
    let mut out = Mask::new(mask.width, mask.height);
    let (w, h) = (mask.width as i32, mask.height as i32);
    for y in 0..h {
        let sy = y - dy;
        if sy < 0 || sy >= h {
            continue;
        }
        let (xa, xb) = (dx.max(0), (w + dx).min(w));
        if xa >= xb {
            continue;
        }
        let src = &mask.data[(sy * w + xa - dx) as usize..][..(xb - xa) as usize];
        out.data[(y * w + xa) as usize..][..(xb - xa) as usize].copy_from_slice(src);
    }
    out.update_bounds();
    out
}

/// Splits `layer` into what stays (the selected pixels taken out) and what is lifted (only
/// the selected pixels), as for moving or cutting them.
pub fn lift(layer: &TiledLayer, mask: &Mask) -> (TiledLayer, TiledLayer) {
    let mut rest = layer.clone();
    let mut lifted = TiledLayer::new();
    if mask.is_empty() {
        return (rest, lifted);
    }
    let (x0, y0, x1, y1) = mask.bounds;
    let t = TILE as u32;
    for ty in y0 / t..y1.div_ceil(t) {
        for tx in x0 / t..x1.div_ceil(t) {
            let key = (tx as i32, ty as i32);
            if layer.tile(key).is_none() && layer.default[3] == 0 {
                continue;
            }
            let (px, py) = (key.0 * TILE, key.1 * TILE);
            if !(0..TILE).any(|y| (0..TILE).any(|x| mask.get(px + x, py + y) != 0)) {
                continue;
            }
            let (r, l) = (rest.tile_mut(key), lifted.tile_mut(key));
            for y in 0..TILE {
                for x in 0..TILE {
                    let m = mask.get(px + x, py + y);
                    if m == 0 {
                        continue;
                    }
                    let p = r.get(x, y);
                    let (keep, take) = (mul8(p[3], 255 - m), mul8(p[3], m));
                    r.set(x, y, if keep == 0 { [0; 4] } else { [p[0], p[1], p[2], keep] });
                    l.set(x, y, if take == 0 { [0; 4] } else { [p[0], p[1], p[2], take] });
                }
            }
        }
    }
    lifted.compact();
    (rest, lifted)
}

/// Puts `floating` (a lifted part) down over `base`.
pub fn put_down(base: &mut TiledLayer, floating: &TiledLayer) {
    for key in floating.tile_keys().collect::<Vec<_>>() {
        let src = floating.tile(key).expect("listed").clone();
        let dst = base.tile_mut(key);
        for (d, s) in dst.0.as_chunks_mut::<4>().0.iter_mut().zip(src.0.as_chunks::<4>().0) {
            if s[3] != 0 {
                over(d, *s, 255);
            }
        }
    }
}

/// The layer with its selected pixels moved by (dx, dy): taken out where they were and put
/// down over what is at their new place.
pub fn move_selected(layer: &TiledLayer, mask: &Mask, dx: i32, dy: i32) -> TiledLayer {
    let (mut rest, lifted) = lift(layer, mask);
    put_down(&mut rest, &offset_layer(&lifted, dx, dy));
    rest
}

/// The painting cut down to the rectangle (x, y, w, h) of its canvas; every layer loses what
/// lies outside it.
pub fn crop(doc: &Document, x: i32, y: i32, w: u32, h: u32) -> Document {
    let mut out = doc.clone();
    out.width = w;
    out.height = h;
    // A selection was made for the old canvas.
    out.selection = None;
    for l in &mut out.layers {
        l.pixels = offset_layer(&l.pixels, -x, -y);
        keep_inside(&mut l.pixels, w, h);
    }
    out
}

/// Removes everything a layer has outside a w x h canvas (its tiles there, and the part of
/// edge tiles beyond the canvas), leaving the layer's default.
fn keep_inside(layer: &mut TiledLayer, w: u32, h: u32) {
    let (w, h) = (w as i32, h as i32);
    let default = layer.default;
    for key in layer.tile_keys().collect::<Vec<_>>() {
        let (px, py) = (key.0 * TILE, key.1 * TILE);
        if px >= w || py >= h || px + TILE <= 0 || py + TILE <= 0 {
            layer.put_tile(key, None);
        } else if px + TILE > w || py + TILE > h || px < 0 || py < 0 {
            let tile = layer.tile_mut(key);
            for y in 0..TILE {
                for x in 0..TILE {
                    if px + x < 0 || py + y < 0 || px + x >= w || py + y >= h {
                        tile.set(x, y, default);
                    }
                }
            }
        }
    }
    layer.compact();
}

/// Pixels lifted out of a layer to be scaled: a straight-alpha copy of the area
/// (`x0`, `y0`, `w` × `h`) they came from.
#[derive(Clone, Debug)]
pub struct Lifted {
    pub x0: i32,
    pub y0: i32,
    pub w: u32,
    pub h: u32,
    px: Vec<[u8; 4]>,
    /// The same, premultiplied (0..=1), for [`Lifted::mapped`].
    pm: Vec<[f32; 4]>,
    /// For each 16 x 16 block of the copy: does it have any paint? (Turning mostly-empty
    /// line art skips the empty parts.)
    blocks: Vec<bool>,
}

/// The side of [`Lifted`]'s paint blocks.
const BLOCK: usize = 16;

impl Lifted {
    /// Copies the area `(x0, y0, x1, y1)` (exclusive) of `layer`.
    pub fn new(layer: &TiledLayer, (x0, y0, x1, y1): (i32, i32, i32, i32)) -> Lifted {
        let (w, h) = ((x1 - x0).max(1) as u32, (y1 - y0).max(1) as u32);
        let mut px = Vec::with_capacity((w * h) as usize);
        for y in y0..y0 + h as i32 {
            for x in x0..x0 + w as i32 {
                px.push(layer.pixel(x, y));
            }
        }
        let pm: Vec<[f32; 4]> = px
            .iter()
            .map(|&[r, g, b, a]| {
                let a = a as f32 / 255.0;
                [r as f32 / 255.0 * a, g as f32 / 255.0 * a, b as f32 / 255.0 * a, a]
            })
            .collect();
        let (bw, bh) = ((w as usize).div_ceil(BLOCK), (h as usize).div_ceil(BLOCK));
        let mut blocks = vec![false; bw * bh];
        for y in 0..h as usize {
            for x in 0..w as usize {
                if px[y * w as usize + x][3] != 0 {
                    blocks[(y / BLOCK) * bw + x / BLOCK] = true;
                }
            }
        }
        Lifted { x0, y0, w, h, px, pm, blocks }
    }

    /// Is there paint anywhere in the copy's rectangle (x0..=x1, y0..=y1, its own pixels)?
    fn paint_in(&self, x0: i64, y0: i64, x1: i64, y1: i64) -> bool {
        let bw = (self.w as usize).div_ceil(BLOCK) as i64;
        let bh = (self.h as usize).div_ceil(BLOCK) as i64;
        let (bx0, by0) = ((x0.max(0) / BLOCK as i64), (y0.max(0) / BLOCK as i64));
        let (bx1, by1) = ((x1.min(self.w as i64 - 1) / BLOCK as i64).min(bw - 1), (y1.min(self.h as i64 - 1) / BLOCK as i64).min(bh - 1));
        if x1 < 0 || y1 < 0 || bx0 > bx1 || by0 > by1 {
            return false;
        }
        (by0..=by1).any(|by| (bx0..=bx1).any(|bx| self.blocks[(by * bw + bx) as usize]))
    }

    /// The pixel at (x, y) of the copy, premultiplied, 0..=1.
    fn premul(&self, x: usize, y: usize) -> [f32; 4] {
        let [r, g, b, a] = self.px[y * self.w as usize + x];
        let a = a as f32 / 255.0;
        [r as f32 / 255.0 * a, g as f32 / 255.0 * a, b as f32 / 255.0 * a, a]
    }

    /// The pixels scaled into `dst` (x0, y0, x1, y1 in canvas pixels), as a layer holding
    /// only them, cut to a `width` × `height` canvas. Enlarging blends neighbours (bilinear);
    /// shrinking averages everything each new pixel covers, so thin lines stay; past the
    /// copy's edge its edge pixels continue (the box ends the picture), and a pixel the box
    /// only partly covers gets that share. At the original size and place the copy comes back
    /// unchanged.
    ///
    /// Done in two passes (across, then down) with each new column's and row's weights
    /// worked out once, a band of tile rows at a time.
    pub fn scaled(&self, dst: (f64, f64, f64, f64), (width, height): (u32, u32)) -> TiledLayer {
        let mut out = TiledLayer::new();
        let (dx0, dy0, dx1, dy1) = (dst.0.min(dst.2), dst.1.min(dst.3), dst.0.max(dst.2), dst.1.max(dst.3));
        if dx1 - dx0 < 0.5 || dy1 - dy0 < 0.5 {
            return out;
        }
        let px0 = (dx0.floor() as i64).max(0);
        let py0 = (dy0.floor() as i64).max(0);
        let px1 = (dx1.ceil() as i64).min(width as i64);
        let py1 = (dy1.ceil() as i64).min(height as i64);
        if px0 >= px1 || py0 >= py1 {
            return out;
        }
        let cols = Taps::new(px0, px1, dx0, dx1, self.w);
        let rows = Taps::new(py0, py1, dy0, dy1, self.h);
        let n = (px1 - px0) as usize;
        let t = TILE as i64;
        // One band of tile rows at a time: the rows of the copy it needs, scaled across.
        let mut band_y = py0.div_euclid(t) * t;
        let mut across: Vec<[f32; 4]> = Vec::new();
        let mut line: Vec<[f32; 4]> = Vec::with_capacity(self.w as usize);
        while band_y < py1 {
            let (y0, y1) = (band_y.max(py0), (band_y + t).min(py1));
            let (lo, hi) = rows.span(y0 - py0, y1 - py0);
            across.clear();
            across.resize((hi - lo + 1) * n, [0.0; 4]);
            for sy in lo..=hi {
                // The copy's row, premultiplied once.
                line.clear();
                line.extend((0..self.w as usize).map(|sx| self.premul(sx, sy)));
                let row = &mut across[(sy - lo) * n..][..n];
                for (i, out_px) in row.iter_mut().enumerate() {
                    let mut acc = [0.0f32; 4];
                    for &(sx, w) in cols.of(i) {
                        let p = line[sx];
                        for c in 0..4 {
                            acc[c] += p[c] * w;
                        }
                    }
                    *out_px = acc;
                }
            }
            // Down, a tile at a time.
            let mut tx = px0.div_euclid(t);
            while tx * t < px1 {
                let (x0, x1) = ((tx * t).max(px0), ((tx + 1) * t).min(px1));
                let mut tile = Box::new([0u8; crate::tiles::TILE_BYTES]);
                let mut any = false;
                for y in y0..y1 {
                    let taps = rows.of((y - py0) as usize);
                    for x in x0..x1 {
                        let i = (x - px0) as usize;
                        let mut acc = [0.0f32; 4];
                        for &(sy, w) in taps {
                            let p = across[(sy - lo) * n + i];
                            for c in 0..4 {
                                acc[c] += p[c] * w;
                            }
                        }
                        if acc[3] * 255.0 < 0.5 {
                            continue;
                        }
                        // Rounded by adding a half (the float-to-int cast saturates); f32
                        // round() is a library call on plain x86-64, four per pixel.
                        let q = |v: f32| (v * 255.0 + 0.5) as u8;
                        let inv = 1.0 / acc[3];
                        let at = (((y - band_y) * t + (x - tx * t)) * 4) as usize;
                        tile[at..at + 4].copy_from_slice(&[q(acc[0] * inv), q(acc[1] * inv), q(acc[2] * inv), q(acc[3])]);
                        any = true;
                    }
                }
                if any {
                    out.put_tile((tx as i32, (band_y / t) as i32), Some(std::sync::Arc::new(crate::tiles::Tile(tile))));
                }
                tx += 1;
            }
            band_y += t;
        }
        out
    }
}

impl Lifted {
    /// The pixels put through any affine map (rotated, skewed, flipped, scaled), as a layer
    /// holding only them, cut to a `width` × `height` canvas. `to_src` takes a canvas point
    /// of the result to the canvas point of the original it shows (x' = a·x + b·y + c,
    /// y' = d·x + e·y + f); `bounds` (x0, y0, x1, y1) is where the result can be. Each new
    /// pixel blends the four original pixels round its point (past the original's edge is
    /// see-through, so turned edges come out smooth); where the result is smaller than the
    /// original it averages up to 4 × 4 such samples, so thin lines stay. A flip maps pixel
    /// centres onto pixel centres and is exact.
    pub fn mapped(&self, to_src: [f64; 6], bounds: (f64, f64, f64, f64), (width, height): (u32, u32)) -> TiledLayer {
        use rayon::prelude::*;
        let [a, b, c, d, e, f] = to_src;
        let px0 = (bounds.0.floor() as i64).max(0);
        let py0 = (bounds.1.floor() as i64).max(0);
        let px1 = (bounds.2.ceil() as i64).min(width as i64);
        let py1 = (bounds.3.ceil() as i64).min(height as i64);
        let mut out = TiledLayer::new();
        if px0 >= px1 || py0 >= py1 {
            return out;
        }
        // How much of the original one new pixel covers: slants and turns keep the area (one
        // sample, so no edge gets blurred), shrinking to a third covers 9 (3 x 3 samples).
        let area = (a * e - b * d).abs();
        let k = (area.sqrt() - 1e-9).ceil().clamp(1.0, 4.0) as usize;
        let t = TILE as i64;
        let keys: Vec<(i64, i64)> = (py0.div_euclid(t)..=(py1 - 1).div_euclid(t)).flat_map(|ty| (px0.div_euclid(t)..=(px1 - 1).div_euclid(t)).map(move |tx| (tx, ty))).collect();
        let sample = |sx: f64, sy: f64| -> [f32; 4] {
            // Bilinear between pixel centres, in the copy's own coordinates.
            let (u, v) = (sx - self.x0 as f64 - 0.5, sy - self.y0 as f64 - 0.5);
            let (x, y) = (u.floor(), v.floor());
            let (fx, fy) = ((u - x) as f32, (v - y) as f32);
            let mut acc = [0.0f32; 4];
            for (ox, oy, w) in [(0, 0, (1.0 - fx) * (1.0 - fy)), (1, 0, fx * (1.0 - fy)), (0, 1, (1.0 - fx) * fy), (1, 1, fx * fy)] {
                let (cx, cy) = (x as i64 + ox, y as i64 + oy);
                if w <= 0.0 || cx < 0 || cy < 0 || cx >= self.w as i64 || cy >= self.h as i64 {
                    continue;
                }
                let p = self.pm[cy as usize * self.w as usize + cx as usize];
                for i in 0..4 {
                    acc[i] += p[i] * w;
                }
            }
            acc
        };
        let tiles: Vec<_> = keys
            .par_iter()
            .filter_map(|&(tx, ty)| {
                // Where this tile's pixels come from in the copy (its corners mapped, a pixel
                // more all round for the blending): nothing there, nothing to do.
                let corners = [(tx * t, ty * t), ((tx + 1) * t, ty * t), (tx * t, (ty + 1) * t), ((tx + 1) * t, (ty + 1) * t)]
                    .map(|(x, y)| (a * x as f64 + b * y as f64 + c - self.x0 as f64, d * x as f64 + e * y as f64 + f - self.y0 as f64));
                let lo = corners.iter().fold((f64::INFINITY, f64::INFINITY), |m, p| (m.0.min(p.0), m.1.min(p.1)));
                let hi = corners.iter().fold((f64::NEG_INFINITY, f64::NEG_INFINITY), |m, p| (m.0.max(p.0), m.1.max(p.1)));
                if !self.paint_in(lo.0.floor() as i64 - 2, lo.1.floor() as i64 - 2, hi.0.ceil() as i64 + 2, hi.1.ceil() as i64 + 2) {
                    return None;
                }
                let mut tile = Box::new([0u8; crate::tiles::TILE_BYTES]);
                let mut any = false;
                for y in (ty * t).max(py0)..((ty + 1) * t).min(py1) {
                    for x in (tx * t).max(px0)..((tx + 1) * t).min(px1) {
                        let mut acc = [0.0f32; 4];
                        for j in 0..k {
                            for i in 0..k {
                                let (qx, qy) = (x as f64 + (i as f64 + 0.5) / k as f64, y as f64 + (j as f64 + 0.5) / k as f64);
                                let s = sample(a * qx + b * qy + c, d * qx + e * qy + f);
                                for n in 0..4 {
                                    acc[n] += s[n];
                                }
                            }
                        }
                        let kk = (k * k) as f32;
                        let alpha = acc[3] / kk;
                        if alpha * 255.0 < 0.5 {
                            continue;
                        }
                        let q = |v: f32| (v * 255.0 + 0.5) as u8;
                        let inv = 1.0 / acc[3];
                        let at = (((y - ty * t) * t + (x - tx * t)) * 4) as usize;
                        tile[at..at + 4].copy_from_slice(&[q(acc[0] * inv), q(acc[1] * inv), q(acc[2] * inv), q(alpha)]);
                        any = true;
                    }
                }
                any.then(|| ((tx as i32, ty as i32), std::sync::Arc::new(crate::tiles::Tile(tile))))
            })
            .collect();
        for (key, tile) in tiles {
            out.put_tile(key, Some(tile));
        }
        out
    }
}

/// For each new column (or row) of a scaled copy, the copy's columns it takes from and how
/// much of each: bilinear samples, several per new pixel when shrinking (up to 16), weighted
/// by how much of the new pixel the box covers.
struct Taps {
    start: Vec<usize>,
    taps: Vec<(usize, f32)>,
}

impl Taps {
    fn new(p0: i64, p1: i64, d0: f64, d1: f64, src: u32) -> Taps {
        let scale = (d1 - d0) / src as f64;
        let k = (1.0 / scale).ceil().clamp(1.0, 16.0) as usize;
        let last = src as i64 - 1;
        let (mut start, mut taps) = (Vec::new(), Vec::new());
        for p in p0..p1 {
            start.push(taps.len());
            let cover = ((p + 1) as f64).min(d1) - (p as f64).max(d0);
            let first = taps.len();
            for i in 0..k {
                let u = ((p as f64 + (i as f64 + 0.5) / k as f64) - d0) / scale - 0.5;
                let x = u.floor();
                let f = u - x;
                for (s, w) in [(x as i64, 1.0 - f), (x as i64 + 1, f)] {
                    if w <= 0.0 {
                        continue;
                    }
                    let s = s.clamp(0, last) as usize;
                    let w = (w * cover / k as f64) as f32;
                    match taps[first..].iter_mut().find(|t: &&mut (usize, f32)| t.0 == s) {
                        Some(t) => t.1 += w,
                        None => taps.push((s, w)),
                    }
                }
            }
        }
        start.push(taps.len());
        Taps { start, taps }
    }

    fn of(&self, i: usize) -> &[(usize, f32)] {
        &self.taps[self.start[i]..self.start[i + 1]]
    }

    /// The lowest and highest source index the new ones `a..b` use.
    fn span(&self, a: i64, b: i64) -> (usize, usize) {
        let all = &self.taps[self.start[a as usize]..self.start[b as usize]];
        (all.iter().map(|t| t.0).min().unwrap_or(0), all.iter().map(|t| t.0).max().unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fill::{polygons, rectangle};

    fn rect(w: u32, h: u32, a: (f64, f64), b: (f64, f64)) -> Mask {
        rectangle(w, h, a, b)
    }

    #[test]
    fn combining_selections() {
        let a = rect(100, 100, (10.0, 10.0), (50.0, 50.0));
        let b = rect(100, 100, (30.0, 30.0), (70.0, 70.0));
        let add = combine(Some(&a), b.clone(), Combine::Add).unwrap();
        assert_eq!((add.get(15, 15), add.get(65, 65), add.get(80, 20)), (255, 255, 0));
        let sub = combine(Some(&a), b.clone(), Combine::Subtract).unwrap();
        assert_eq!((sub.get(15, 15), sub.get(40, 40)), (255, 0));
        let both = combine(Some(&a), b.clone(), Combine::Intersect).unwrap();
        assert_eq!((both.get(15, 15), both.get(40, 40)), (0, 255));
        assert_eq!(both.bounds, (30, 30, 50, 50));
        assert!(combine(Some(&a), a.clone(), Combine::Subtract).is_none(), "nothing left");
        assert!(combine(None, b.clone(), Combine::Subtract).is_none());
        assert_eq!(combine(None, b.clone(), Combine::Add).unwrap().get(40, 40), 255);
        let inv = invert(&a).unwrap();
        assert_eq!((inv.get(15, 15), inv.get(80, 80)), (0, 255));
        assert!(invert(&all(10, 10)).is_none());
    }

    #[test]
    fn polygon_masks_fill_inside_with_soft_edges() {
        // A triangle and a square drawn twice over (nonzero: still filled once).
        let tri = polygons(100, 100, &[vec![(10.0, 10.0), (90.0, 10.0), (10.0, 90.0)]]);
        assert_eq!(tri.get(20, 20), 255);
        assert_eq!(tri.get(80, 80), 0);
        let diag = tri.get(49, 50);
        assert!(diag > 0 && diag < 255, "the slanted edge is soft: {diag}");
        let sq = [(20.0, 20.0), (60.0, 20.0), (60.0, 60.0), (20.0, 60.0)];
        let twice: Vec<(f64, f64)> = sq.iter().chain(sq.iter()).copied().collect();
        let m = polygons(100, 100, &[twice]);
        assert_eq!((m.get(40, 40), m.get(70, 40)), (255, 0));
        assert_eq!(m.bounds, (20, 20, 60, 60));
        // Exact coverage at a half-pixel edge, and nothing for degenerate input.
        let half = polygons(10, 10, &[vec![(2.0, 2.0), (6.5, 2.0), (6.5, 8.0), (2.0, 8.0)]]);
        assert_eq!((half.get(5, 5), half.get(6, 5), half.get(7, 5)), (255, 128, 0));
        assert!(polygons(10, 10, &[vec![(1.0, 1.0), (5.0, 5.0)]]).is_empty());
        // Partly off the canvas: clipped, no panic.
        let off = polygons(50, 50, &[vec![(-20.0, -20.0), (30.0, -20.0), (30.0, 30.0), (-20.0, 30.0)]]);
        assert_eq!((off.get(0, 0), off.get(29, 29), off.get(31, 31)), (255, 255, 0));
    }

    #[test]
    fn outline_follows_the_border() {
        let m = rect(100, 100, (10.0, 20.0), (30.0, 50.0));
        let segs = outline(&m, 100).unwrap();
        assert_eq!(segs.len(), 4, "a rectangle is four runs: {segs:?}");
        assert!(segs.contains(&[(10.0, 20.0), (30.0, 20.0)]));
        assert!(segs.contains(&[(30.0, 20.0), (30.0, 50.0)]));
        assert!(outline(&all(64, 64), 10).unwrap().len() == 4);
        // A checkerboard: each inner border between rows (and columns) is one long run;
        // the outer edge alternates, 20 short pieces on each side.
        let mut checker = Mask::new(40, 40);
        for (i, v) in checker.data.iter_mut().enumerate() {
            *v = if (i % 40 + i / 40) % 2 == 0 { 255 } else { 0 };
        }
        checker.update_bounds();
        assert_eq!(outline(&checker, 500).unwrap().len(), 2 * (39 + 20 + 20));
        // Separate dots: four short borders each, too many to draw.
        let mut dots = Mask::new(40, 40);
        for (i, v) in dots.data.iter_mut().enumerate() {
            *v = if i % 2 == 0 && (i / 40) % 2 == 0 { 255 } else { 0 };
        }
        dots.update_bounds();
        assert!(outline(&dots, 500).is_none());
        assert_eq!(outline(&dots, 10_000).unwrap().len(), 20 * 20 * 4);
    }

    #[test]
    fn clipping_keeps_changes_inside_the_selection_only() {
        let mut layer = TiledLayer::new();
        for y in 0..128 {
            for x in 0..128 {
                layer.set_pixel(x, y, [10, 20, 30, 255]);
            }
        }
        let before = layer.clone();
        // Some tool painted everything red; the selection is x 10..40.
        for y in 0..128 {
            for x in 0..128 {
                layer.set_pixel(x, y, [255, 0, 0, 255]);
            }
        }
        let sel = rect(128, 128, (10.0, 0.0), (40.0, 128.0));
        clip_to(&mut layer, &before, &sel, [(0, 0), (1, 0), (0, 1), (1, 1)]);
        assert_eq!(layer.pixel(20, 5), [255, 0, 0, 255], "inside: the change stays");
        assert_eq!(layer.pixel(5, 5), [10, 20, 30, 255], "outside: as before");
        assert_eq!(layer.pixel(100, 100), [10, 20, 30, 255]);
        // A tile with nothing selected is the very tile from before again.
        assert!(std::sync::Arc::ptr_eq(layer.tile((1, 1)).unwrap(), before.tile((1, 1)).unwrap()));
        // Soft edge: half selected is half way (premultiplied mix).
        let soft = rect(128, 128, (10.5, 0.0), (40.0, 128.0));
        let mut l2 = before.clone();
        l2.set_pixel(10, 3, [255, 0, 0, 255]);
        clip_to(&mut l2, &before, &soft, [(0, 0)]);
        assert_eq!(l2.pixel(10, 3), [133, 10, 15, 255]);
    }

    #[test]
    fn moving_a_layer_by_any_amount() {
        let mut l = TiledLayer::new();
        l.set_pixel(5, 5, [1, 2, 3, 255]);
        l.set_pixel(70, 130, [4, 5, 6, 255]);
        for (dx, dy) in [(64, -128), (3, 0), (-7, 61), (100, 100)] {
            let m = offset_layer(&l, dx, dy);
            assert_eq!(m.pixel(5 + dx, 5 + dy), [1, 2, 3, 255], "({dx}, {dy})");
            assert_eq!(m.pixel(70 + dx, 130 + dy), [4, 5, 6, 255]);
            assert_eq!(m.pixel(5, 5)[3], if (dx, dy) == (0, 0) { 255 } else { 0 });
            // And back again: exactly the same pixels.
            let back = offset_layer(&m, -dx, -dy);
            for (x, y) in [(5, 5), (70, 130), (6, 5), (0, 0)] {
                assert_eq!(back.pixel(x, y), l.pixel(x, y));
            }
        }
    }

    #[test]
    fn moving_selected_pixels_carries_only_them() {
        let mut l = TiledLayer::new();
        for x in 0..100 {
            l.set_pixel(x, 10, [200, 0, 0, 255]);
        }
        // Select x 0..20 and move it down 30.
        let sel = rect(200, 200, (0.0, 0.0), (20.0, 50.0));
        let m = move_selected(&l, &sel, 0, 30);
        assert_eq!(m.pixel(5, 40), [200, 0, 0, 255], "moved");
        assert_eq!(m.pixel(5, 10)[3], 0, "gone from where it was");
        assert_eq!(m.pixel(50, 10), [200, 0, 0, 255], "the rest stays");
        let mo = offset_mask(&sel, 0, 30);
        assert_eq!((mo.get(5, 40), mo.get(5, 10), mo.bounds), (255, 0, (0, 30, 20, 80)));
        // Moving on a layer with a background colour leaves transparency behind.
        let bg = TiledLayer::filled([255, 255, 255, 255]);
        let moved = move_selected(&bg, &rect(200, 200, (0.0, 0.0), (10.0, 10.0)), 50, 0);
        assert_eq!(moved.pixel(5, 5)[3], 0);
        assert_eq!(moved.pixel(55, 5), [255, 255, 255, 255]);
    }

    #[test]
    fn crop_cuts_every_layer() {
        let mut doc = Document::new(300, 200, Some([255, 255, 255, 255]));
        for x in 0..300 {
            doc.layers[1].pixels.set_pixel(x, 100, [0, 0, 255, 255]);
        }
        let c = crop(&doc, 50, 80, 100, 40);
        assert_eq!((c.width, c.height), (100, 40));
        assert_eq!(c.layers[1].pixels.pixel(0, 20), [0, 0, 255, 255], "old (50, 100) is now (0, 20)");
        assert_eq!(c.layers[1].pixels.pixel(99, 20), [0, 0, 255, 255]);
        assert_eq!(c.layers[1].pixels.pixel(100, 20)[3], 0, "nothing kept outside the new canvas");
        assert_eq!(c.layers[0].pixels.pixel(10, 10), [255, 255, 255, 255], "the background stays filled");
        assert_eq!(c.flatten().pixel(30, 20), [0, 0, 255, 255]);
        assert_eq!(c.flatten().pixel(30, 21), [255, 255, 255, 255]);
    }

    #[test]
    fn lifted_pixels_scale_up_down_and_back() {
        let mut l = TiledLayer::new();
        // A 4 × 2 block: red left half, half-transparent blue right half, at (10, 20).
        for y in 20..22 {
            for x in 10..14 {
                l.set_pixel(x, y, if x < 12 { [255, 0, 0, 255] } else { [0, 0, 255, 128] });
            }
        }
        let lifted = Lifted::new(&l, (10, 20, 14, 22));
        // Same size, same place: exactly the same pixels.
        let same = lifted.scaled((10.0, 20.0, 14.0, 22.0), (100, 100));
        for y in 18..24 {
            for x in 8..16 {
                assert_eq!(same.pixel(x, y), l.pixel(x, y), "({x}, {y})");
            }
        }
        // Twice the size from (40, 40): 8 × 4, the colours where they were.
        let big = lifted.scaled((40.0, 40.0, 48.0, 44.0), (100, 100));
        assert_eq!(big.pixel(40, 40), [255, 0, 0, 255]);
        let right = big.pixel(47, 43);
        assert_eq!((right[2], right[3]), (255, 128), "{right:?}");
        assert_eq!(big.pixel(48, 40)[3], 0, "nothing past the new edge");
        assert_eq!(big.pixel(39, 40)[3], 0);
        // An edge half way through a pixel: that pixel half covered (blue at 128 → 64).
        let part = lifted.scaled((40.0, 40.0, 47.5, 44.0), (100, 100));
        assert!((62..=66).contains(&part.pixel(47, 42)[3]), "{:?}", part.pixel(47, 42));
        // Half the size: 2 × 1, each pixel the average of what it covers.
        let small = lifted.scaled((0.0, 0.0, 2.0, 1.0), (100, 100));
        assert_eq!(small.pixel(0, 0), [255, 0, 0, 255]);
        assert_eq!(small.pixel(1, 0)[2..], [255, 128]);
        assert_eq!(small.pixel(2, 0)[3], 0);
        // Cut to the canvas.
        let off = lifted.scaled((98.0, 98.0, 106.0, 102.0), (100, 100));
        assert!(off.pixel(99, 99)[3] > 0 && off.pixel(100, 99)[3] == 0);
    }

    #[test]
    fn a_thin_line_survives_shrinking() {
        let mut l = TiledLayer::new();
        for x in 0..100 {
            l.set_pixel(x, 50, [0, 0, 0, 255]);
        }
        let lifted = Lifted::new(&l, (0, 0, 100, 100));
        let small = lifted.scaled((0.0, 0.0, 25.0, 25.0), (100, 100));
        // A quarter the size: the 1 px line covers a quarter of a pixel row.
        let row: Vec<u8> = (0..25).map(|x| small.pixel(x, 12)[3]).collect();
        assert!(row.iter().all(|&a| (55..=72).contains(&a)), "{row:?}");
    }

    /// A small painting to transform: a 30 x 20 block of different colours with see-through
    /// parts, at (100, 60).
    fn pattern() -> (TiledLayer, Lifted) {
        let mut l = TiledLayer::new();
        for y in 0..20 {
            for x in 0..30 {
                let a = if (x + y) % 7 == 0 { 0 } else { 120 + ((x * 5 + y * 3) % 136) as u8 };
                l.set_pixel(100 + x, 60 + y, [(x * 8) as u8, (y * 12) as u8, ((x * y) % 255) as u8, a]);
            }
        }
        let lifted = Lifted::new(&l, (100, 60, 130, 80));
        (l, lifted)
    }

    #[test]
    fn mapped_copies_flips_and_quarter_turns_exactly() {
        let (l, lifted) = pattern();
        let size = (400, 300);
        // The same place: the same pixels.
        let same = lifted.mapped([1.0, 0.0, 0.0, 0.0, 1.0, 0.0], (100.0, 60.0, 130.0, 80.0), size);
        for y in 60..80 {
            for x in 100..130 {
                let (a, b) = (l.pixel(x, y), same.pixel(x, y));
                assert!(a == b || (a[3] == 0 && b[3] == 0), "{x},{y}: {a:?} vs {b:?}");
            }
        }
        // Flipped left to right about x = 115: x' = 230 - x.
        let flip = lifted.mapped([-1.0, 0.0, 230.0, 0.0, 1.0, 0.0], (100.0, 60.0, 130.0, 80.0), size);
        for y in 60..80 {
            for x in 100..130 {
                let (a, b) = (l.pixel(229 - x, y), flip.pixel(x, y));
                assert!(a == b || (a[3] == 0 && b[3] == 0), "flip {x},{y}");
            }
        }
        // A quarter turn about (115, 70): the result's (x, y) shows the original's
        // (115 + (y - 70), 70 - (x - 115)).
        let turn = lifted.mapped([0.0, 1.0, 45.0, -1.0, 0.0, 185.0], (105.0, 55.0, 125.0, 85.0), size);
        for y in 55..85 {
            for x in 105..125 {
                let (a, b) = (l.pixel(115 + (y - 70), 69 - (x - 115)), turn.pixel(x, y));
                assert!(a == b || (a[3] == 0 && b[3] == 0), "turn {x},{y}: {a:?} vs {b:?}");
            }
        }
    }

    #[test]
    fn mapped_turns_keep_the_paint_and_shrinking_keeps_thin_lines() {
        let (l, lifted) = pattern();
        let mass = |l: &TiledLayer| (0..300).flat_map(|y| (0..400).map(move |x| (x, y))).map(|(x, y)| l.pixel(x, y)[3] as f64).sum::<f64>();
        // 45 degrees about (115, 70).
        let (s, c) = (std::f64::consts::FRAC_PI_4.sin(), std::f64::consts::FRAC_PI_4.cos());
        let to_src = [c, s, 115.0 - c * 115.0 - s * 70.0, -s, c, 70.0 + s * 115.0 - c * 70.0];
        let turned = lifted.mapped(to_src, (90.0, 45.0, 140.0, 95.0), (400, 300));
        let (m0, m1) = (mass(&l), mass(&turned));
        assert!((m1 / m0 - 1.0).abs() < 0.03, "paint kept: {m0} -> {m1}");
        // A one-pixel line shrunk to a third still shows along its length.
        let mut line = TiledLayer::new();
        for x in 0..90 {
            line.set_pixel(10 + x, 20, [0, 0, 0, 255]);
        }
        let lifted = Lifted::new(&line, (10, 10, 100, 30));
        let small = lifted.mapped([3.0, 0.0, -20.0, 0.0, 3.0, -40.0], (10.0, 16.0, 40.0, 24.0), (400, 300));
        let shown = (10..40).filter(|&x| (16..24).any(|y| small.pixel(x, y)[3] > 40)).count();
        assert!(shown >= 29, "{shown} of 30 columns show the line");
    }
}
