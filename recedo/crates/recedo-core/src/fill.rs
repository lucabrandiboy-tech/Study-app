//! Fill tools: the paint bucket (flood fill by colour similarity), filled rectangles and
//! ellipses, and the linear gradient (colour fading to transparent). Each works out a
//! coverage mask (0..=255 per canvas pixel) and paints it onto a layer with "over".

use crate::doc::over;
use crate::image::Rgba8;
use crate::tiles::{TILE, TiledLayer};

/// Per-pixel coverage over the canvas, with the rectangle that has any.
#[derive(Clone, PartialEq)]
pub struct Mask {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    /// x0, y0, x1, y1 (exclusive) of the covered area; empty if x0 >= x1.
    pub bounds: (u32, u32, u32, u32),
}

impl std::fmt::Debug for Mask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mask({}x{}, bounds {:?})", self.width, self.height, self.bounds)
    }
}

impl Mask {
    /// An empty mask over a canvas of this size.
    pub fn new(width: u32, height: u32) -> Mask {
        Mask { width, height, data: vec![0; (width * height) as usize], bounds: (width, height, 0, 0) }
    }

    /// Coverage at a pixel; 0 outside the canvas.
    pub fn get(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
            return 0;
        }
        self.data[y as usize * self.width as usize + x as usize]
    }

    /// Works out `bounds` again after `data` was changed.
    pub fn update_bounds(&mut self) {
        let (mut x0, mut y0, mut x1, mut y1) = (self.width, self.height, 0, 0);
        for y in 0..self.height {
            let row = &self.data[(y * self.width) as usize..][..self.width as usize];
            if let (Some(a), Some(b)) = (row.iter().position(|&v| v != 0), row.iter().rposition(|&v| v != 0)) {
                x0 = x0.min(a as u32);
                x1 = x1.max(b as u32 + 1);
                y0 = y0.min(y);
                y1 = y + 1;
            }
        }
        self.bounds = (x0, y0, x1, y1);
    }

    pub fn is_empty(&self) -> bool {
        self.bounds.0 >= self.bounds.2
    }
}

/// The paint bucket's area: every pixel connected to (x, y) whose colour is within
/// `tolerance` (0..=1, of the largest channel difference) of the clicked one, grown by
/// `grow` pixels so the fill tucks under the soft edges of the lines around it.
pub fn flood(reference: &Rgba8, x: i32, y: i32, tolerance: f32, grow: u32) -> Option<Mask> {
    let (w, h) = (reference.width, reference.height);
    if x < 0 || y < 0 || x as u32 >= w || y as u32 >= h {
        return None;
    }
    let seed = reference.pixel(x as u32, y as u32);
    let limit = (tolerance.clamp(0.0, 1.0) * 255.0).round() as i32;
    // First, in one pass: 1 where the colour counts as the clicked one, else 0. All fully
    // transparent pixels count as the same colour.
    let mut mask = Mask::new(w, h);
    for (m, p) in mask.data.iter_mut().zip(reference.data.as_chunks::<4>().0) {
        let same = (p[3] == 0 && seed[3] == 0) || (0..4).all(|c| (p[c] as i32 - seed[c] as i32).abs() <= limit);
        *m = same as u8;
    }
    // Scanline fill over those: each popped point fills its whole run (255), then queues the
    // runs next to it in the rows above and below.
    let (wu, d) = (w as usize, &mut mask.data);
    let mut stack = vec![(x as usize, y as usize)];
    let (mut y0, mut y1) = (y as usize, y as usize);
    while let Some((sx, sy)) = stack.pop() {
        let row = sy * wu;
        if d[row + sx] != 1 {
            continue;
        }
        let (mut l, mut r) = (sx, sx);
        while l > 0 && d[row + l - 1] == 1 {
            l -= 1;
        }
        while r + 1 < wu && d[row + r + 1] == 1 {
            r += 1;
        }
        d[row + l..=row + r].fill(255);
        (y0, y1) = (y0.min(sy), y1.max(sy));
        for ny in [sy.wrapping_sub(1), sy + 1] {
            if ny >= h as usize {
                continue;
            }
            let nrow = ny * wu;
            let mut px = l;
            while px <= r {
                if d[nrow + px] == 1 {
                    stack.push((px, ny));
                    // The rest of this run is filled from the pushed point.
                    while px <= r && d[nrow + px] == 1 {
                        px += 1;
                    }
                } else {
                    px += 1;
                }
            }
        }
    }
    // Similar pixels the fill did not reach are not filled.
    for m in d[y0 * wu..(y1 + 1) * wu].iter_mut() {
        *m = if *m == 255 { 255 } else { 0 };
    }
    d[..y0 * wu].fill(0);
    d[(y1 + 1) * wu..].fill(0);
    for _ in 0..grow {
        (y0, y1) = (y0.saturating_sub(1), (y1 + 1).min(h as usize - 1));
        grow_rows(&mut mask.data, wu, y0, y1);
    }
    mask.update_bounds();
    Some(mask)
}

/// Grows the covered area (255) by one pixel up, down, left and right, in rows `y0..=y1`
/// (the rows outside are empty and stay so).
fn grow_rows(data: &mut [u8], w: usize, y0: usize, y1: usize) {
    let src = data[y0.saturating_sub(1) * w..((y1 + 2) * w).min(data.len())].to_vec();
    let base = y0.saturating_sub(1);
    let h = data.len() / w;
    for y in y0..=y1 {
        let at = |yy: usize| &src[(yy - base) * w..][..w];
        let (row, out) = (at(y), &mut data[y * w..][..w]);
        // Left and right neighbours.
        out[0] = row[0] | row.get(1).copied().unwrap_or(0);
        if w > 1 {
            out[w - 1] = row[w - 1] | row[w - 2];
            for (o, win) in out[1..w - 1].iter_mut().zip(row.windows(3)) {
                *o = win[0] | win[1] | win[2];
            }
        }
        // Up and down.
        if y > 0 {
            for (o, &a) in out.iter_mut().zip(at(y - 1)) {
                *o |= a;
            }
        }
        if y + 1 < h {
            for (o, &b) in out.iter_mut().zip(at(y + 1)) {
                *o |= b;
            }
        }
    }
}

/// A filled rectangle between two corners (canvas pixels), with soft edges at fractions.
pub fn rectangle(width: u32, height: u32, a: (f64, f64), b: (f64, f64)) -> Mask {
    let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
    let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
    shape(width, height, (x0, y0, x1, y1), |px, py| {
        let cover = |p: f64, lo: f64, hi: f64| ((p + 0.5).min(hi) - (p - 0.5).max(lo)).clamp(0.0, 1.0);
        cover(px, x0, x1) * cover(py, y0, y1)
    })
}

/// A filled ellipse in the box between two corners, with a one-pixel soft edge.
pub fn ellipse(width: u32, height: u32, a: (f64, f64), b: (f64, f64)) -> Mask {
    let (cx, cy) = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    let (rx, ry) = (((a.0 - b.0) / 2.0).abs().max(0.5), ((a.1 - b.1) / 2.0).abs().max(0.5));
    shape(width, height, (cx - rx, cy - ry, cx + rx, cy + ry), |px, py| {
        let (dx, dy) = ((px - cx) / rx, (py - cy) / ry);
        let n = (dx * dx + dy * dy).sqrt();
        // Distance to the edge in pixels, roughly: soft over one pixel.
        let d = (n - 1.0) * rx.min(ry);
        (0.5 - d).clamp(0.0, 1.0)
    })
}

/// Coverage from a function of the pixel centre, inside a box.
fn shape(width: u32, height: u32, (x0, y0, x1, y1): (f64, f64, f64, f64), f: impl Fn(f64, f64) -> f64) -> Mask {
    let mut mask = Mask::new(width, height);
    let clamp = |v: f64, max: u32| v.floor().clamp(0.0, max as f64) as u32;
    let (bx0, by0, bx1, by1) = (clamp(x0 - 1.0, width), clamp(y0 - 1.0, height), clamp(x1 + 2.0, width), clamp(y1 + 2.0, height));
    for y in by0..by1 {
        for x in bx0..bx1 {
            let c = f(x as f64 + 0.5, y as f64 + 0.5);
            mask.data[(y * width + x) as usize] = (c * 255.0 + 0.5) as u8;
        }
    }
    mask.update_bounds();
    mask
}

/// Closed polygons (canvas pixels) filled by the nonzero rule, so a loop drawn over itself
/// stays filled, with anti-aliased edges: each pixel row is sampled on 4 sub-rows, with
/// exact coverage along each sub-row.
pub fn polygons(width: u32, height: u32, contours: &[Vec<(f64, f64)>]) -> Mask {
    const SUB: usize = 4;
    let mut mask = Mask::new(width, height);
    // Edges top to bottom, with +1 or -1 for the direction they were drawn in.
    type Edge = ((f64, f64), (f64, f64), i32);
    let mut edges: Vec<Edge> = Vec::new();
    for c in contours.iter().filter(|c| c.len() >= 3) {
        for i in 0..c.len() {
            let (a, b) = (c[i], c[(i + 1) % c.len()]);
            if a.1 == b.1 || ![a.0, a.1, b.0, b.1].iter().all(|v| v.is_finite()) {
                continue;
            }
            edges.push(if a.1 < b.1 { (a, b, 1) } else { (b, a, -1) });
        }
    }
    if edges.is_empty() || width == 0 || height == 0 {
        return mask;
    }
    edges.sort_by(|a, b| a.0.1.total_cmp(&b.0.1));
    let top = edges[0].0.1.max(0.0).floor() as usize;
    let bottom = edges.iter().map(|e| e.1.1).fold(f64::MIN, f64::max).min(height as f64).ceil().max(0.0) as usize;
    let w = width as usize;
    let mut cover = vec![0f32; w];
    let (mut active, mut next) = (Vec::new(), 0);
    let mut crossings: Vec<(f64, i32)> = Vec::new();
    for row in top..bottom {
        cover.fill(0.0);
        for k in 0..SUB {
            let sy = row as f64 + (k as f64 + 0.5) / SUB as f64;
            while next < edges.len() && edges[next].0.1 <= sy {
                active.push(next);
                next += 1;
            }
            active.retain(|&i| edges[i].1.1 > sy);
            crossings.clear();
            for &i in &active {
                let ((x0, y0), (x1, y1), dir) = edges[i];
                crossings.push((x0 + (x1 - x0) * (sy - y0) / (y1 - y0), dir));
            }
            crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut winding = 0;
            let mut start = 0.0;
            for &(x, dir) in &crossings {
                let was = winding;
                winding += dir;
                if was == 0 && winding != 0 {
                    start = x;
                } else if was != 0 && winding == 0 {
                    add_span(&mut cover, start, x, 1.0 / SUB as f32);
                }
            }
        }
        for (m, c) in mask.data[row * w..][..w].iter_mut().zip(&cover) {
            *m = (c.min(1.0) * 255.0 + 0.5) as u8;
        }
    }
    mask.update_bounds();
    mask
}

/// Adds `amount` times the part of each pixel that the span a..b covers.
fn add_span(cover: &mut [f32], a: f64, b: f64, amount: f32) {
    let (a, b) = (a.max(0.0), b.min(cover.len() as f64));
    if a >= b {
        return;
    }
    let (ia, ib) = (a.floor() as usize, b.floor() as usize);
    if ia == ib {
        cover[ia] += (b - a) as f32 * amount;
        return;
    }
    cover[ia] += ((ia + 1) as f64 - a) as f32 * amount;
    for c in &mut cover[ia + 1..ib] {
        *c += amount;
    }
    if ib < cover.len() {
        cover[ib] += (b - ib as f64) as f32 * amount;
    }
}

/// Erases a layer where the mask covers it (all of a pixel where coverage is 255). Returns
/// the tiles it changed.
pub fn erase_mask(layer: &mut TiledLayer, mask: &Mask) -> Vec<(i32, i32)> {
    let mut changed = Vec::new();
    if mask.is_empty() {
        return changed;
    }
    let (x0, y0, x1, y1) = mask.bounds;
    let t = TILE as u32;
    for ty in y0 / t..y1.div_ceil(t) {
        for tx in x0 / t..x1.div_ceil(t) {
            let key = (tx as i32, ty as i32);
            // A transparent layer has nothing to erase where it has no tile.
            if layer.tile(key).is_none() && layer.default[3] == 0 {
                continue;
            }
            let (px0, py0) = (tx * t, ty * t);
            let n = ((px0 + t).min(mask.width) - px0) as usize;
            let rows = py0..(py0 + t).min(mask.height);
            if !rows.clone().any(|y| mask.data[(y * mask.width + px0) as usize..][..n].iter().any(|&v| v != 0)) {
                continue;
            }
            let tile = layer.tile_mut(key);
            for y in rows {
                let m_row = &mask.data[(y * mask.width + px0) as usize..][..n];
                let start = ((y - py0) * t * 4) as usize;
                for (&m, d) in m_row.iter().zip(tile.0[start..start + n * 4].as_chunks_mut::<4>().0) {
                    if m != 0 {
                        let a = crate::doc::mul8(d[3], 255 - m);
                        *d = if a == 0 { [0; 4] } else { [d[0], d[1], d[2], a] };
                    }
                }
            }
            changed.push(key);
        }
    }
    changed
}

/// Paints `color` over a layer where the mask covers it (fully opaque where coverage is 255).
/// Returns the tiles it changed.
pub fn paint_mask(layer: &mut TiledLayer, mask: &Mask, color: [u8; 3], opacity: u8) -> Vec<(i32, i32)> {
    let mut changed = Vec::new();
    if mask.is_empty() {
        return changed;
    }
    let (x0, y0, x1, y1) = mask.bounds;
    let t = TILE as u32;
    for ty in y0 / t..y1.div_ceil(t) {
        for tx in x0 / t..x1.div_ceil(t) {
            let key = (tx as i32, ty as i32);
            let (px0, py0) = (tx * t, ty * t);
            let any = (py0..(py0 + t).min(mask.height)).any(|y| mask.data[(y * mask.width + px0) as usize..][..((px0 + t).min(mask.width) - px0) as usize].iter().any(|&v| v != 0));
            if !any {
                continue;
            }
            let tile = layer.tile_mut(key);
            let n = ((px0 + t).min(mask.width) - px0) as usize;
            for y in py0..(py0 + t).min(mask.height) {
                let m_row = &mask.data[(y * mask.width + px0) as usize..][..n];
                let start = ((y - py0) * t * 4) as usize;
                let t_row = tile.0[start..start + n * 4].as_chunks_mut::<4>().0;
                for (&m, d) in m_row.iter().zip(t_row) {
                    match m {
                        0 => {}
                        // What "over" gives for a fully covered pixel at full opacity.
                        255 if opacity == 255 => *d = [color[0], color[1], color[2], 255],
                        _ => over(d, [color[0], color[1], color[2], m], opacity),
                    }
                }
            }
            changed.push(key);
        }
    }
    changed
}

/// A linear gradient from `a` (the colour at full strength) to `b` (transparent), painted
/// over the whole layer: before `a` the colour is solid, after `b` the layer is unchanged.
pub fn gradient(layer: &mut TiledLayer, width: u32, height: u32, a: (f64, f64), b: (f64, f64), color: [u8; 3]) -> Vec<(i32, i32)> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let mut mask = Mask::new(width, height);
    // t = ((pixel centre - a) . (b - a)) / |b - a|^2, which grows by dx / len2 per pixel along
    // a row.
    let (step, len2) = if len2 < 1e-9 { (0.0, 1.0) } else { (dx / len2, len2) };
    for (y, row) in mask.data.chunks_exact_mut(width as usize).enumerate() {
        let start = if dx == 0.0 && dy == 0.0 { 0.0 } else { ((0.5 - a.0) * dx + (y as f64 + 0.5 - a.1) * dy) / len2 };
        for (x, m) in row.iter_mut().enumerate() {
            let t = (start + x as f64 * step).clamp(0.0, 1.0);
            // + 0.5 and truncate: rounding (the value is never negative).
            *m = ((1.0 - t) * 255.0 + 0.5) as u8;
        }
    }
    mask.update_bounds();
    paint_mask(layer, &mask, color, 255)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas_with_ring() -> Rgba8 {
        // White, with a black square outline from (10,10) to (29,29), 2 px thick.
        let mut img = Rgba8::new(40, 40);
        for y in 0..40u32 {
            for x in 0..40u32 {
                let on = (10..30).contains(&x) && (10..30).contains(&y) && !((12..28).contains(&x) && (12..28).contains(&y));
                let i = ((y * 40 + x) * 4) as usize;
                img.data[i..i + 4].copy_from_slice(if on { &[0, 0, 0, 255] } else { &[255, 255, 255, 255] });
            }
        }
        img
    }

    #[test]
    fn flood_fills_inside_lines_only() {
        let img = canvas_with_ring();
        let m = flood(&img, 20, 20, 0.1, 0).unwrap();
        assert_eq!(m.bounds, (12, 12, 28, 28), "exactly the inside of the outline");
        assert_eq!(m.data.iter().filter(|&&v| v == 255).count(), 16 * 16);
        // Outside: everything but the outline and the inside.
        let out = flood(&img, 0, 0, 0.1, 0).unwrap();
        assert_eq!(out.data.iter().filter(|&&v| v == 255).count(), 40 * 40 - 20 * 20);
        // Grown by one pixel it reaches under the line's inner edge, no further.
        let g = flood(&img, 20, 20, 0.1, 1).unwrap();
        assert_eq!(g.bounds, (11, 11, 29, 29));
        assert!(flood(&img, -1, 3, 0.1, 0).is_none());
    }

    #[test]
    fn tolerance_decides_what_counts_as_the_same_colour() {
        let mut img = Rgba8::new(10, 1);
        for x in 0..10u32 {
            let v = 200 - x as u8 * 10;
            img.data[(x * 4) as usize..][..4].copy_from_slice(&[v, v, v, 255]);
        }
        // 10 per step: a tolerance of 25 (about 0.1) reaches two steps either way.
        assert_eq!(flood(&img, 5, 0, 25.0 / 255.0, 0).unwrap().bounds, (3, 0, 8, 1));
        assert_eq!(flood(&img, 5, 0, 1.0, 0).unwrap().bounds, (0, 0, 10, 1));
    }

    #[test]
    fn painted_fill_and_shapes() {
        let mut layer = TiledLayer::new();
        let img = canvas_with_ring();
        let m = flood(&img, 20, 20, 0.1, 0).unwrap();
        let changed = paint_mask(&mut layer, &m, [200, 30, 30], 255);
        assert_eq!(changed, vec![(0, 0)]);
        assert_eq!(layer.pixel(20, 20), [200, 30, 30, 255]);
        assert_eq!(layer.pixel(5, 5), [0, 0, 0, 0]);
        // Rectangle: full inside, soft at a half-pixel edge, nothing outside.
        let r = rectangle(100, 100, (10.0, 10.0), (20.5, 20.0));
        assert_eq!(r.data[15 * 100 + 15], 255);
        assert_eq!(r.data[15 * 100 + 20], 128);
        assert_eq!(r.data[15 * 100 + 25], 0);
        // Ellipse: centre in, corner of its box out, symmetric.
        let e = ellipse(100, 100, (10.0, 20.0), (50.0, 40.0));
        assert_eq!(e.data[30 * 100 + 30], 255);
        assert_eq!(e.data[21 * 100 + 11], 0);
        assert_eq!(e.data[30 * 100 + 12], e.data[30 * 100 + 47]);
    }

    #[test]
    fn erasing_by_a_mask() {
        let mut layer = TiledLayer::new();
        for y in 0..40 {
            for x in 0..40 {
                layer.set_pixel(x, y, [10, 200, 10, 255]);
            }
        }
        let m = rectangle(100, 100, (10.0, 10.0), (20.5, 30.0));
        let changed = erase_mask(&mut layer, &m);
        assert_eq!(changed, vec![(0, 0)]);
        assert_eq!(layer.pixel(15, 15), [0, 0, 0, 0], "inside: gone, and clean");
        assert_eq!(layer.pixel(20, 15), [10, 200, 10, 127], "half-covered edge: half left");
        assert_eq!(layer.pixel(5, 15), [10, 200, 10, 255], "outside: kept");
        // Nothing to erase on an empty layer: no tiles are made.
        let mut empty = TiledLayer::new();
        assert!(erase_mask(&mut empty, &m).is_empty());
        assert_eq!(empty.tile_count(), 0);
        // A background (filled everywhere) gets a hole.
        let mut bg = TiledLayer::filled([255, 255, 255, 255]);
        assert_eq!(erase_mask(&mut bg, &m), vec![(0, 0)]);
        assert_eq!((bg.pixel(15, 15)[3], bg.pixel(5, 5)[3]), (0, 255));
    }

    #[test]
    fn gradient_fades_from_the_colour_to_nothing() {
        let mut layer = TiledLayer::new();
        gradient(&mut layer, 100, 10, (20.0, 5.0), (80.0, 5.0), [0, 0, 255]);
        assert_eq!(layer.pixel(5, 5), [0, 0, 255, 255], "solid before the start");
        let mid = layer.pixel(50, 5)[3];
        assert!((120..=135).contains(&mid), "half way: {mid}");
        assert_eq!(layer.pixel(90, 5)[3], 0, "nothing after the end");
    }
}
