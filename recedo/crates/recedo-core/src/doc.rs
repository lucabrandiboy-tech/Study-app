//! A painting: canvas size, resolution and a stack of layers, and flattening it to one
//! picture the way Krita composites 8-bit RGBA layers.

use std::hash::{BuildHasher, Hasher};

use crate::blend::{BlendMode, blend};
use crate::image::Rgba8;
use crate::tiles::{TILE, Tile, TiledLayer};

#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
    /// 0 = invisible, 255 = fully opaque (Krita stores it the same way).
    pub opacity: u8,
    pub locked: bool,
    /// How the layer mixes with the layers below.
    pub blend: BlendMode,
    /// Krita's layer id, kept so a file saved back keeps its identity.
    pub uuid: String,
    pub pixels: TiledLayer,
}

impl Layer {
    /// Whether the layer takes part in the picture. A hidden layer never does; a layer at
    /// opacity 0 does only with a blend mode other than Normal, which (as in Krita) still
    /// re-rounds the semi-transparent pixels under it.
    pub fn shows(&self) -> bool {
        self.visible && (self.opacity > 0 || self.blend != BlendMode::Normal)
    }

    pub fn new(name: &str) -> Layer {
        Layer { name: name.to_string(), visible: true, opacity: 255, locked: false, blend: BlendMode::Normal, uuid: new_uuid(), pixels: TiledLayer::new() }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub width: u32,
    pub height: u32,
    /// Pixels per inch.
    pub dpi: f64,
    /// Bottom layer first.
    pub layers: Vec<Layer>,
}

/// Largest canvas side Recedo creates or opens.
pub const MAX_SIDE: u32 = 30_000;

impl Document {
    /// A canvas with a "Background" layer filled with `background` (if any) and an empty
    /// "Layer 1" above it, like Krita's new document.
    pub fn new(width: u32, height: u32, background: Option<[u8; 4]>) -> Document {
        let mut layers = Vec::new();
        if let Some(bg) = background {
            let mut l = Layer::new("Background");
            l.pixels = TiledLayer::filled(bg);
            layers.push(l);
        }
        layers.push(Layer::new("Layer 1"));
        Document { width, height, dpi: 300.0, layers }
    }

    /// The visible layers composited inside the canvas.
    pub fn flatten(&self) -> Rgba8 {
        let mut out = Rgba8::new(self.width, self.height);
        let layers: Vec<&Layer> = self.layers.iter().filter(|l| l.shows()).collect();
        let w = self.width as usize;
        let band = w * TILE as usize * 4;
        if out.data.is_empty() || layers.is_empty() {
            return out;
        }
        // One band per row of tiles, spread over the CPU cores (one at a time in a browser).
        use rayon::prelude::*;
        out.data.par_chunks_mut(band).enumerate().for_each(|(row, rows)| {
            for l in &layers {
                composite_band(rows, w, row as i32, &l.pixels, l.opacity, l.blend);
            }
        });
        out
    }
}

/// `a * b / 255`, rounded, as Krita computes it for 8-bit channels.
#[inline]
pub fn mul8(a: u8, b: u8) -> u8 {
    let t = a as u32 * b as u32 + 0x80;
    (((t >> 8) + t) >> 8) as u8
}

#[inline]
fn div8(a: u8, b: u8) -> u8 {
    ((a as u32 * 255 + b as u32 / 2) / b as u32).min(255) as u8
}

/// `dst + (src - dst) * alpha / 255`, rounded like Krita.
#[inline]
fn blend8(src: u8, dst: u8, alpha: u8) -> u8 {
    let c = (src as i32 - dst as i32) * alpha as i32 + 0x80;
    let c = ((c >> 8) + c) >> 8;
    (c + dst as i32) as u8
}

/// Krita's "Normal" blend mode on straight-alpha 8-bit pixels.
#[inline]
pub fn over(dst: &mut [u8; 4], src: [u8; 4], opacity: u8) {
    let sa = mul8(src[3], opacity);
    if sa == 0 {
        return;
    }
    let da = dst[3];
    let blend = if da == 255 {
        sa
    } else {
        let na = da + mul8(255 - da, sa);
        dst[3] = na;
        if na != 0 { div8(sa, na) } else { sa }
    };
    if blend == 255 {
        dst[..3].copy_from_slice(&src[..3]);
    } else {
        for c in 0..3 {
            dst[c] = blend8(src[c], dst[c], blend);
        }
    }
}

/// Composites tile row `row` of a layer over `rows`: the canvas pixels of that tile row
/// (`width` pixels wide, up to 64 rows), one tile at a time so empty areas cost nothing.
pub fn composite_band(rows: &mut [u8], width: usize, row: i32, layer: &TiledLayer, opacity: u8, mode: BlendMode) {
    let height = rows.len() / (width * 4);
    let default = (layer.default[3] != 0).then(|| Tile::filled(layer.default));
    for col in 0..width.div_ceil(TILE as usize) {
        let tile = match (layer.tile((col as i32, row)), &default) {
            (Some(t), _) => &**t,
            (None, Some(d)) => d,
            (None, None) => continue,
        };
        let x0 = col * TILE as usize;
        composite_tile(&mut rows[x0 * 4..], width, tile, (TILE as usize).min(width - x0), height, opacity, mode);
    }
}

/// Composites the top-left `cols` x `rows` pixels of `tile` over `dst`, whose rows are
/// `stride` pixels apart.
pub fn composite_tile(dst: &mut [u8], stride: usize, tile: &Tile, cols: usize, rows: usize, opacity: u8, mode: BlendMode) {
    for ty in 0..rows {
        let src = &tile.0[ty * TILE as usize * 4..][..cols * 4];
        let start = ty * stride * 4;
        for (d, s) in dst[start..start + cols * 4].as_chunks_mut::<4>().0.iter_mut().zip(src.as_chunks::<4>().0) {
            if s[3] != 0 || mode != BlendMode::Normal {
                blend(d, *s, opacity, mode);
            }
        }
    }
}

/// A random (version 4) UUID in Krita's `{…}` form.
pub fn new_uuid() -> String {
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
    let a = h.finish();
    h.write_u64(a);
    let b = h.finish();
    let b = (b & 0x3FFF_FFFF_FFFF_FFFF) | 0x8000_0000_0000_0000; // variant bits 10
    let a = (a & !0xF000) | 0x4000; // version 4
    format!("{{{:08x}-{:04x}-{:04x}-{:04x}-{:012x}}}", a >> 32, (a >> 16) & 0xFFFF, a & 0xFFFF, b >> 48, b & 0xFFFF_FFFF_FFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn krita_rounding() {
        assert_eq!(mul8(255, 255), 255);
        assert_eq!(mul8(255, 128), 128);
        assert_eq!(mul8(128, 128), 64);
        assert_eq!(mul8(0, 200), 0);
        assert_eq!(div8(64, 128), 128);
        assert_eq!(blend8(255, 0, 128), 128);
        assert_eq!(blend8(0, 255, 128), 127);
        assert_eq!(blend8(10, 200, 255), 10);
    }

    #[test]
    fn over_cases() {
        // Opaque over anything replaces it.
        let mut d = [1, 2, 3, 255];
        over(&mut d, [200, 100, 50, 255], 255);
        assert_eq!(d, [200, 100, 50, 255]);
        // Anything over transparent copies its colour, alpha scaled by opacity.
        let mut d = [9, 9, 9, 0];
        over(&mut d, [200, 100, 50, 200], 128);
        assert_eq!(d, [200, 100, 50, mul8(200, 128)]);
        // Half red over opaque blue: purple, still opaque.
        let mut d = [0, 0, 255, 255];
        over(&mut d, [255, 0, 0, 128], 255);
        assert_eq!(d, [128, 0, 127, 255]);
        // Half red over half blue: alpha 1 - 0.5 * 0.5, red weighted 0.5 / 0.75.
        let mut d = [0, 0, 255, 128];
        over(&mut d, [255, 0, 0, 128], 255);
        assert_eq!(d[3], 192);
        assert_eq!(d[0], 170);
        assert_eq!(d[2], 85);
        // Fully transparent source changes nothing.
        let mut d = [5, 6, 7, 8];
        over(&mut d, [255, 255, 255, 0], 255);
        assert_eq!(d, [5, 6, 7, 8]);
    }

    #[test]
    fn flatten_respects_order_visibility_opacity_and_canvas() {
        let mut doc = Document::new(100, 70, Some([255, 255, 255, 255]));
        assert_eq!(doc.layers.len(), 2);
        doc.layers[1].pixels.set_pixel(10, 10, [255, 0, 0, 255]);
        doc.layers[1].pixels.set_pixel(-5, 3, [255, 0, 0, 255]); // outside the canvas
        let mut top = Layer::new("top");
        top.pixels.set_pixel(10, 10, [0, 0, 255, 255]);
        top.pixels.set_pixel(99, 69, [0, 0, 0, 255]);
        top.opacity = 0;
        doc.layers.push(top);
        let f = doc.flatten();
        assert_eq!((f.width, f.height), (100, 70));
        assert_eq!(f.pixel(10, 10), [255, 0, 0, 255], "invisible top layer (opacity 0)");
        assert_eq!(f.pixel(50, 50), [255, 255, 255, 255], "background fills the canvas");
        doc.layers[2].opacity = 255;
        let f = doc.flatten();
        assert_eq!(f.pixel(10, 10), [0, 0, 255, 255], "top layer wins");
        assert_eq!(f.pixel(99, 69), [0, 0, 0, 255], "last pixel of a partial tile");
        doc.layers[2].visible = false;
        doc.layers[0].visible = false;
        let f = doc.flatten();
        assert_eq!(f.pixel(10, 10), [255, 0, 0, 255]);
        assert_eq!(f.pixel(50, 50), [0, 0, 0, 0], "hidden background");
    }

    #[test]
    fn uuids_look_right_and_differ() {
        let a = new_uuid();
        let b = new_uuid();
        assert_ne!(a, b);
        assert_eq!(a.len(), 38);
        assert!(a.starts_with('{') && a.ends_with('}'));
        assert_eq!(&a[15..16], "4", "{a}");
        assert!(matches!(&a[20..21], "8" | "9" | "a" | "b"), "{a}");
    }
}
