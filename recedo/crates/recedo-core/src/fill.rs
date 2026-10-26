//! Fill tools: the paint bucket (flood fill by colour similarity), filled rectangles and
//! ellipses, and the linear gradient (colour fading to transparent). Each works out a
//! coverage mask (0..=255 per canvas pixel) and paints it onto a layer with "over".

use crate::doc::over;
use crate::image::Rgba8;
use crate::tiles::{TILE, TiledLayer};

/// Per-pixel coverage over the canvas, with the rectangle that has any.
pub struct Mask {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    /// x0, y0, x1, y1 (exclusive) of the covered area; empty if x0 >= x1.
    pub bounds: (u32, u32, u32, u32),
}

impl Mask {
    fn new(width: u32, height: u32) -> Mask {
        Mask { width, height, data: vec![0; (width * height) as usize], bounds: (width, height, 0, 0) }
    }

    fn update_bounds(&mut self) {
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
    let similar = |p: [u8; 4]| {
        // All fully transparent pixels count as the same colour.
        if p[3] == 0 && seed[3] == 0 {
            return true;
        }
        (0..4).all(|c| (p[c] as i32 - seed[c] as i32).abs() <= limit)
    };
    let mut mask = Mask::new(w, h);
    // Scanline fill: each popped point fills its whole run, then queues the rows next to it.
    let mut stack = vec![(x as u32, y as u32)];
    while let Some((sx, sy)) = stack.pop() {
        let row = (sy * w) as usize;
        if mask.data[row + sx as usize] != 0 {
            continue;
        }
        let (mut l, mut r) = (sx, sx);
        while l > 0 && mask.data[row + l as usize - 1] == 0 && similar(reference.pixel(l - 1, sy)) {
            l -= 1;
        }
        while r + 1 < w && mask.data[row + r as usize + 1] == 0 && similar(reference.pixel(r + 1, sy)) {
            r += 1;
        }
        for px in l..=r {
            mask.data[row + px as usize] = 255;
        }
        for ny in [sy.wrapping_sub(1), sy + 1] {
            if ny >= h {
                continue;
            }
            let nrow = (ny * w) as usize;
            let mut px = l;
            while px <= r {
                if mask.data[nrow + px as usize] == 0 && similar(reference.pixel(px, ny)) {
                    stack.push((px, ny));
                    // Skip the rest of this run; it is filled from the pushed point.
                    while px <= r && similar(reference.pixel(px, ny)) {
                        px += 1;
                    }
                } else {
                    px += 1;
                }
            }
        }
    }
    for _ in 0..grow {
        let src = mask.data.clone();
        for yy in 0..h {
            for xx in 0..w {
                let i = (yy * w + xx) as usize;
                if src[i] != 0 {
                    continue;
                }
                let near = (xx > 0 && src[i - 1] != 0) || (xx + 1 < w && src[i + 1] != 0) || (yy > 0 && src[i - w as usize] != 0) || (yy + 1 < h && src[i + w as usize] != 0);
                if near {
                    mask.data[i] = 255;
                }
            }
        }
    }
    mask.update_bounds();
    Some(mask)
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
            mask.data[(y * width + x) as usize] = (c * 255.0).round() as u8;
        }
    }
    mask.update_bounds();
    mask
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
            for y in py0..(py0 + t).min(mask.height) {
                for x in px0..(px0 + t).min(mask.width) {
                    let m = mask.data[(y * mask.width + x) as usize];
                    if m == 0 {
                        continue;
                    }
                    let (lx, ly) = ((x - px0) as i32, (y - py0) as i32);
                    let mut d = tile.get(lx, ly);
                    over(&mut d, [color[0], color[1], color[2], m], opacity);
                    tile.set(lx, ly, d);
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
    for y in 0..height {
        for x in 0..width {
            let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
            let t = if len2 < 1e-9 { 0.0 } else { (((px - a.0) * dx + (py - a.1) * dy) / len2).clamp(0.0, 1.0) };
            mask.data[(y * width + x) as usize] = ((1.0 - t) * 255.0).round() as u8;
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
    fn gradient_fades_from_the_colour_to_nothing() {
        let mut layer = TiledLayer::new();
        gradient(&mut layer, 100, 10, (20.0, 5.0), (80.0, 5.0), [0, 0, 255]);
        assert_eq!(layer.pixel(5, 5), [0, 0, 255, 255], "solid before the start");
        let mid = layer.pixel(50, 5)[3];
        assert!((120..=135).contains(&mid), "half way: {mid}");
        assert_eq!(layer.pixel(90, 5)[3], 0, "nothing after the end");
    }
}
