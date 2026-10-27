//! Pictures the panels show: stroke samples painted by the real brush engine (the sub tool
//! list and the tool property panel), and small copies of the layers (the layers panel).

use std::collections::{HashMap, HashSet};

use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use recedo_core::brush::{Brush, StrokePainter};
use recedo_core::doc::Document;
use recedo_core::stabilizer::PaintInfo;
use recedo_core::tiles::{TILE, TiledLayer};
use std::time::Duration;

use web_time::Instant;

/// A stroke across a `w` × `h` pixel box with `brush` as it is: an S-curve, the pressure
/// rising and falling again, so a brush that follows pressure tapers at both ends.
pub fn stroke_image(brush: &Brush, color: [u8; 3], w: u32, h: u32) -> ColorImage {
    let mut layer = TiledLayer::new();
    let mut painter = StrokePainter::new(brush.clone(), color, false);
    let mut dirty = HashSet::new();
    let (wf, hf) = (w as f64, h as f64);
    let half = (brush.size as f64 / 2.0).min(hf / 2.0);
    let margin = (half + 3.0).min(wf / 4.0);
    let amp = ((hf / 2.0 - half - 2.0) * 0.8).max(0.0);
    let n = 96;
    for i in 0..=n {
        let t = i as f64 / n as f64;
        let x = margin + (wf - 2.0 * margin) * t;
        let y = hf / 2.0 - amp * (std::f64::consts::TAU * t).sin();
        let pressure = ((std::f64::consts::PI * t).sin().powf(0.6) as f32).max(0.05);
        painter.add(&mut layer, PaintInfo::new(x, y, pressure), &mut dirty);
    }
    ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &layer.read_rect(0, 0, w, h))
}

/// The brush made `size` pixels wide for a sample (the rest as it is).
pub fn sized(brush: &Brush, size: f32) -> Brush {
    Brush { size, ..brush.clone() }
}

/// A layer's pixels shrunk to `w` × `h`: each pixel the coverage-weighted average of all the
/// layer's pixels under it (every second one in each direction on paintings over two million
/// pixels), so even a thin line shows. Only the tiles that exist are read; the rest of the
/// layer is its default pixel.
pub fn thumbnail(doc: &Document, index: usize, w: u32, h: u32) -> ColorImage {
    let layer = &doc.layers[index].pixels;
    let (dw, dh) = (doc.width as i64, doc.height as i64);
    let stride: i64 = if dw * dh > 2_000_000 { 2 } else { 1 };
    let bin_x = |x: i64| (x * w as i64 / dw) as usize;
    let bin_y = |y: i64| (y * h as i64 / dh) as usize;
    // Per thumbnail pixel: red, green and blue times alpha, alpha, and how many were read.
    let mut bins = vec![[0u64; 5]; (w * h) as usize];
    for key in layer.tile_keys() {
        let Some(tile) = layer.tile(key) else { continue };
        let (x0, y0) = (key.0 as i64 * TILE as i64, key.1 as i64 * TILE as i64);
        let px = tile.0.as_chunks::<4>().0;
        for ty in (0..TILE as i64).step_by(stride as usize) {
            let y = y0 + ty;
            if !(0..dh).contains(&y) {
                continue;
            }
            let row = bin_y(y) * w as usize;
            for tx in (0..TILE as i64).step_by(stride as usize) {
                let x = x0 + tx;
                if !(0..dw).contains(&x) {
                    continue;
                }
                let [r, g, b, a] = px[(ty * TILE as i64 + tx) as usize];
                let bin = &mut bins[row + bin_x(x)];
                let a64 = a as u64;
                bin[0] += r as u64 * a64;
                bin[1] += g as u64 * a64;
                bin[2] += b as u64 * a64;
                bin[3] += a64;
                bin[4] += 1;
            }
        }
    }
    // How many pixels each column and row of the thumbnail covers (at the stride).
    let mut cols = vec![0u64; w as usize];
    for x in (0..dw).step_by(stride as usize) {
        cols[bin_x(x)] += 1;
    }
    let mut rows = vec![0u64; h as usize];
    for y in (0..dh).step_by(stride as usize) {
        rows[bin_y(y)] += 1;
    }
    let [dr, dg, db, da] = layer.default.map(|c| c as u64);
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for by in 0..h as usize {
        for bx in 0..w as usize {
            let mut bin = bins[by * w as usize + bx];
            let total = (cols[bx] * rows[by]).max(1);
            let missing = total.saturating_sub(bin[4]);
            bin[0] += dr * da * missing;
            bin[1] += dg * da * missing;
            bin[2] += db * da * missing;
            bin[3] += da * missing;
            let rgb = [0, 1, 2].map(|c| bin[c].checked_div(bin[3]).unwrap_or(0) as u8);
            out.extend_from_slice(&[rgb[0], rgb[1], rgb[2], (bin[3] / total) as u8]);
        }
    }
    ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &out)
}

/// A thumbnail's size for a painting: `side` pixels along its longer edge.
pub fn thumbnail_size(doc: &Document, side: u32) -> (u32, u32) {
    let (w, h) = (doc.width.max(1) as f64, doc.height.max(1) as f64);
    let s = side as f64 / w.max(h);
    (((w * s).round() as u32).max(4), ((h * s).round() as u32).max(4))
}

/// While pixels keep changing (a stroke), thumbnails are redone at most this often.
const THUMBNAIL_EVERY: Duration = Duration::from_millis(400);

struct Thumb {
    uuid: String,
    rev: u64,
    /// Which tiles the layer had when the picture was made (see [`tiles_signature`]).
    signature: (usize, usize),
    at: Instant,
    texture: TextureHandle,
}

/// A cheap fingerprint of which tiles a layer holds: their number and their addresses.
/// Undo, redo, fills and moves give a layer other tiles, which changes it; painting with a
/// brush changes the selected layer's tiles in place, which it does not (that layer is
/// redone whenever pixels changed).
fn tiles_signature(layer: &TiledLayer) -> (usize, usize) {
    let addresses = layer.tile_keys().filter_map(|k| layer.tile(k)).fold(0usize, |acc, t| acc.wrapping_add(std::sync::Arc::as_ptr(t) as usize).rotate_left(5));
    (layer.tile_count(), addresses ^ layer.default.iter().fold(0usize, |a, &c| a << 8 | c as usize))
}

/// The textures, made when first needed and kept until what they show changes.
pub struct Previews {
    strokes: HashMap<String, TextureHandle>,
    thumbs: Vec<Option<Thumb>>,
    /// How long thumbnails wait between updates ([`THUMBNAIL_EVERY`]; the tests change it).
    pub every: Duration,
}

impl Default for Previews {
    fn default() -> Previews {
        Previews { strokes: HashMap::new(), thumbs: Vec::new(), every: THUMBNAIL_EVERY }
    }
}

impl Previews {
    /// A stroke sample, `w` × `h` pixels.
    pub fn stroke(&mut self, ctx: &Context, brush: &Brush, color: [u8; 3], w: u32, h: u32) -> TextureHandle {
        let key = format!("{brush:?} {color:?} {w}x{h}");
        if let Some(t) = self.strokes.get(&key) {
            return t.clone();
        }
        if self.strokes.len() > 64 {
            self.strokes.clear();
        }
        let t = ctx.load_texture(format!("stroke {}", self.strokes.len()), stroke_image(brush, color, w, h), TextureOptions::LINEAR);
        self.strokes.insert(key, t.clone());
        t
    }

    /// The thumbnail of layer `index`, redone when its pixels changed (`rev` is the editor's
    /// pixel revision; `selected`: the layer being painted on): at most every
    /// [`Previews::every`] while they keep changing, and only for layers that did change.
    pub fn thumbnail(&mut self, ctx: &Context, doc: &Document, index: usize, rev: u64, selected: bool, side: u32) -> Option<TextureHandle> {
        let layer = doc.layers.get(index)?;
        if self.thumbs.len() != doc.layers.len() {
            self.thumbs.resize_with(doc.layers.len(), || None);
        }
        let redo = match &mut self.thumbs[index] {
            None => true,
            Some(t) if t.uuid != layer.uuid => true,
            Some(t) if t.rev == rev => false,
            Some(t) => {
                let since = t.at.elapsed();
                if since < self.every {
                    // Too soon: come back when it is time, even if nothing else happens.
                    ctx.request_repaint_after(self.every - since);
                    false
                } else if selected || tiles_signature(&layer.pixels) != t.signature {
                    true
                } else {
                    // Pixels changed elsewhere: this layer's picture still holds.
                    t.rev = rev;
                    false
                }
            }
        };
        if redo {
            let (w, h) = thumbnail_size(doc, side);
            let texture = ctx.load_texture(format!("layer {index}"), thumbnail(doc, index, w, h), TextureOptions::LINEAR);
            self.thumbs[index] = Some(Thumb { uuid: layer.uuid.clone(), rev, signature: tiles_signature(&layer.pixels), at: Instant::now(), texture });
        }
        self.thumbs[index].as_ref().map(|t| t.texture.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ink(img: &ColorImage) -> usize {
        img.pixels.iter().filter(|p| p.a() > 0).count()
    }

    #[test]
    fn a_stroke_sample_follows_the_brush() {
        let base = Brush::default();
        let thin = stroke_image(&sized(&base, 4.0), [255, 255, 255], 200, 40);
        let thick = stroke_image(&sized(&base, 16.0), [255, 255, 255], 200, 40);
        assert!(ink(&thin) > 300, "a 4 px stroke paints: {}", ink(&thin));
        assert!(ink(&thick) > 2 * ink(&thin), "bigger brush, more paint: {} / {}", ink(&thick), ink(&thin));
        // Pressure tapers the ends: the middle column is wider than one near the start.
        let column = |img: &ColorImage, x: usize| (0..40).filter(|&y| img.pixels[y * 200 + x].a() > 128).count();
        assert!(column(&thick, 100) > column(&thick, 14), "{} / {}", column(&thick, 100), column(&thick, 14));
        // Half the opacity: no pixel more than about half covered.
        let faint = stroke_image(&Brush { opacity: 0.5, ..sized(&base, 16.0) }, [255, 255, 255], 200, 40);
        assert!(faint.pixels.iter().all(|p| p.a() <= 130));
        // Stays inside its box.
        let big = stroke_image(&sized(&base, 120.0), [255, 255, 255], 200, 40);
        assert!(ink(&big) > 0);
    }

    #[test]
    fn a_thumbnail_shows_a_thin_line() {
        let mut doc = Document::new(1000, 500, None);
        let l = &mut doc.layers[0].pixels;
        for x in 100..900 {
            for y in 249..252 {
                l.set_pixel(x, y, [200, 0, 0, 255]);
            }
        }
        let (w, h) = thumbnail_size(&doc, 48);
        assert_eq!((w, h), (48, 24));
        let img = thumbnail(&doc, 0, w, h);
        let row = (0..48).filter(|&x| img.pixels[12 * 48 + x].a() > 0).count();
        assert!(row > 30, "the 3 px line shows across the thumbnail: {row}");
        assert!(img.pixels[12 * 48 + 24].to_srgba_unmultiplied()[0] > 150, "in its colour (egui keeps pixels premultiplied)");
        assert_eq!(img.pixels[2 * 48 + 24].a(), 0, "empty elsewhere");
        // A layer that is all its default pixel (a white background) is white.
        let white = Document::new(1000, 500, Some([255, 255, 255, 255]));
        let img = thumbnail(&white, 0, w, h);
        assert!(img.pixels.iter().all(|p| *p == egui::Color32::WHITE));
        // A big painting is read at every second pixel: a 2 px line still shows.
        let mut big = Document::new(4000, 2000, None);
        for x in 0..4000 {
            for y in 1000..1002 {
                big.layers[0].pixels.set_pixel(x, y, [0, 0, 255, 255]);
            }
        }
        let (w, h) = thumbnail_size(&big, 48);
        let img = thumbnail(&big, 0, w, h);
        assert!((0..48).all(|x| img.pixels[12 * 48 + x].a() > 0), "across the whole width");
    }

    #[test]
    fn only_the_layers_that_changed_get_new_thumbnails() {
        let ctx = Context::default();
        let mut doc = Document::new(256, 256, None);
        for n in 0..3 {
            let mut l = recedo_core::doc::Layer::new(&format!("L{n}"));
            l.pixels.set_pixel(10 + n, 10, [255, 0, 0, 255]);
            doc.layers.push(l);
        }
        let mut p = Previews::default();
        let ids = |p: &mut Previews, doc: &Document, rev: u64, selected: usize| (0..doc.layers.len()).map(|i| p.thumbnail(&ctx, doc, i, rev, i == selected, 32).unwrap().id()).collect::<Vec<_>>();
        let first = ids(&mut p, &doc, 0, 1);
        // Painting on layer 1 (in place) changes the pixel revision.
        doc.layers[1].pixels.set_pixel(100, 100, [0, 0, 255, 255]);
        std::thread::sleep(THUMBNAIL_EVERY + Duration::from_millis(30));
        let second = ids(&mut p, &doc, 1, 1);
        assert_eq!(first[0], second[0]);
        assert_ne!(first[1], second[1], "the painted layer is redone");
        assert_eq!(first[2], second[2]);
        assert_eq!(first[3], second[3], "the others are not");
        // Undo-like: layer 3 gets other tiles while layer 1 stays selected.
        doc.layers[3].pixels = TiledLayer::new();
        std::thread::sleep(THUMBNAIL_EVERY + Duration::from_millis(30));
        let third = ids(&mut p, &doc, 2, 1);
        assert_ne!(second[3], third[3], "a layer with other tiles is redone");
        assert_eq!(second[2], third[2]);
    }
}
