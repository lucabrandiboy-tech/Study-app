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
}
