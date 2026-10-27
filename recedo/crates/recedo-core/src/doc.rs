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
    /// The selection, if any: tools only change pixels inside it. Part of the painting so
    /// undo takes it back too; shared, so undo steps that keep it cost nothing.
    pub selection: Option<std::sync::Arc<crate::fill::Mask>>,
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
        Document { width, height, dpi: 300.0, layers, selection: None }
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
            // Once the band is opaque (usually from the background up), Normal layers take
            // the faster path, as in the projection.
            let mut opaque = false;
            for l in &layers {
                let fast = opaque && l.blend == BlendMode::Normal;
                composite_band(rows, w, row as i32, &l.pixels, l.opacity, l.blend, fast);
                opaque = opaque || rows.as_chunks::<4>().0.iter().all(|p| p[3] == 255);
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

/// `a * 255 / b`, rounded, at most 255 (the division Krita's 8-bit compositing does).
#[inline]
fn div8(a: u8, b: u8) -> u8 {
    DIV8[a as usize][b as usize]
}

/// The arithmetic behind [`div8`], for building its table (and testing it).
const fn div8_exact(a: u8, b: u8) -> u8 {
    if b == 0 {
        return 255;
    }
    let q = (a as u32 * 255 + b as u32 / 2) / b as u32;
    if q > 255 { 255 } else { q as u8 }
}

/// Every answer of [`div8`] worked out when compiling (64 KB): a table read instead of a
/// division per pixel, which made painting on see-through layers slow.
static DIV8: [[u8; 256]; 256] = {
    let mut t = [[0u8; 256]; 256];
    let mut a = 0;
    while a < 256 {
        let mut b = 0;
        while b < 256 {
            t[a][b] = div8_exact(a as u8, b as u8);
            b += 1;
        }
        a += 1;
    }
    t
};

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
/// `onto_opaque`: every pixel of `rows` is opaque and the mode Normal (the faster path).
pub fn composite_band(rows: &mut [u8], width: usize, row: i32, layer: &TiledLayer, opacity: u8, mode: BlendMode, onto_opaque: bool) {
    let height = rows.len() / (width * 4);
    let default = (layer.default[3] != 0).then(|| Tile::filled(layer.default));
    for col in 0..width.div_ceil(TILE as usize) {
        let tile = match (layer.tile((col as i32, row)), &default) {
            (Some(t), _) => &**t,
            (None, Some(d)) => d,
            (None, None) => continue,
        };
        let x0 = col * TILE as usize;
        let cols = (TILE as usize).min(width - x0);
        if onto_opaque {
            composite_tile_onto_opaque(&mut rows[x0 * 4..], width, tile, cols, height, opacity);
        } else {
            composite_tile(&mut rows[x0 * 4..], width, tile, cols, height, opacity, mode);
        }
    }
}

/// Composites the top-left `cols` x `rows` pixels of `tile` over `dst`, whose rows are
/// `stride` pixels apart.
pub fn composite_tile(dst: &mut [u8], stride: usize, tile: &Tile, cols: usize, rows: usize, opacity: u8, mode: BlendMode) {
    let plain = mode == BlendMode::Normal && opacity == 255;
    for ty in 0..rows {
        let src = &tile.0[ty * TILE as usize * 4..][..cols * 4];
        let start = ty * stride * 4;
        let pixels = dst[start..start + cols * 4].as_chunks_mut::<4>().0.iter_mut().zip(src.as_chunks::<4>().0);
        if plain {
            // Normal at full opacity: an opaque pixel replaces what is below (exactly what
            // "over" gives), a clear one leaves it.
            for (d, s) in pixels {
                match s[3] {
                    0 => {}
                    255 => *d = *s,
                    _ => over(d, *s, 255),
                }
            }
        } else {
            for (d, s) in pixels {
                if s[3] != 0 || mode != BlendMode::Normal {
                    blend(d, *s, opacity, mode);
                }
            }
        }
    }
}

/// Like [`composite_tile`] for a Normal layer over pixels that are all opaque already (the
/// usual case above a background): the alpha stays 255 and each channel is
/// `blend8(src, dst, src alpha × opacity)`, which is exactly what [`over`] gives there.
/// Done with SIMD where there is some (SSE2 on x86-64, SIMD128 in browsers), four pixels at
/// a time, using an unsigned 16-bit form of the same arithmetic that was checked equal to
/// it for every source, destination and alpha (16.7 million cases): about twice as fast
/// natively and three times in the browser.
pub fn composite_tile_onto_opaque(dst: &mut [u8], stride: usize, tile: &Tile, cols: usize, rows: usize, opacity: u8) {
    for ty in 0..rows {
        let src = &tile.0[ty * TILE as usize * 4..][..cols * 4];
        let start = ty * stride * 4;
        blend_row_onto_opaque(&mut dst[start..start + cols * 4], src, opacity);
    }
}

fn blend_row_onto_opaque_scalar(dst: &mut [u8], src: &[u8], opacity: u8) {
    for (d, s) in dst.as_chunks_mut::<4>().0.iter_mut().zip(src.as_chunks::<4>().0) {
        let a = mul8(s[3], opacity);
        for c in 0..3 {
            d[c] = blend8(s[c], d[c], a);
        }
    }
}

#[cfg(target_arch = "x86_64")]
fn blend_row_onto_opaque(dst: &mut [u8], src: &[u8], opacity: u8) {
    use std::arch::x86_64::*;
    let (dc, dr) = dst.as_chunks_mut::<16>();
    let (sc, sr) = src.as_chunks::<16>();
    // SAFETY: SSE2 is part of every x86-64 CPU; the loads and stores are 16 bytes inside
    // the 16-byte chunks they come from.
    unsafe {
        let zero = _mm_setzero_si128();
        let op = _mm_set1_epi16(opacity as i16);
        let (c128, c255) = (_mm_set1_epi16(128), _mm_set1_epi16(255));
        let alpha = _mm_set1_epi32(0xFF00_0000u32 as i32);
        // Two pixels' channels in 16-bit lanes: blend8 as unsigned steps (see above).
        let blend = |s: __m128i, d: __m128i| -> __m128i {
            let sa = _mm_shufflehi_epi16::<0xFF>(_mm_shufflelo_epi16::<0xFF>(s));
            let t = _mm_add_epi16(_mm_mullo_epi16(sa, op), c128);
            let a = _mm_srli_epi16::<8>(_mm_add_epi16(_mm_srli_epi16::<8>(t), t));
            let tp = _mm_add_epi16(_mm_mullo_epi16(_mm_subs_epu16(s, d), a), c128);
            let up = _mm_srli_epi16::<8>(_mm_add_epi16(tp, _mm_srli_epi16::<8>(tp)));
            let zn = _mm_subs_epu16(_mm_mullo_epi16(_mm_subs_epu16(d, s), a), c128);
            let wn = _mm_add_epi16(zn, _mm_srli_epi16::<8>(_mm_add_epi16(zn, c255)));
            let down = _mm_srli_epi16::<8>(_mm_add_epi16(wn, c255));
            _mm_sub_epi16(_mm_add_epi16(d, up), down)
        };
        for (d, s) in dc.iter_mut().zip(sc) {
            let sv = _mm_loadu_si128(s.as_ptr() as *const __m128i);
            let dv = _mm_loadu_si128(d.as_ptr() as *const __m128i);
            let lo = blend(_mm_unpacklo_epi8(sv, zero), _mm_unpacklo_epi8(dv, zero));
            let hi = blend(_mm_unpackhi_epi8(sv, zero), _mm_unpackhi_epi8(dv, zero));
            _mm_storeu_si128(d.as_mut_ptr() as *mut __m128i, _mm_or_si128(_mm_packus_epi16(lo, hi), alpha));
        }
    }
    blend_row_onto_opaque_scalar(dr, sr, opacity);
}

#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
fn blend_row_onto_opaque(dst: &mut [u8], src: &[u8], opacity: u8) {
    use std::arch::wasm32::*;
    let (dc, dr) = dst.as_chunks_mut::<16>();
    let (sc, sr) = src.as_chunks::<16>();
    let op = u16x8_splat(opacity as u16);
    let (c128, c255) = (u16x8_splat(128), u16x8_splat(255));
    let alpha = u32x4_splat(0xFF00_0000);
    let blend = |s: v128, d: v128| -> v128 {
        let sa = i16x8_shuffle::<3, 3, 3, 3, 7, 7, 7, 7>(s, s);
        let t = i16x8_add(i16x8_mul(sa, op), c128);
        let a = u16x8_shr(i16x8_add(u16x8_shr(t, 8), t), 8);
        let tp = i16x8_add(i16x8_mul(u16x8_sub_sat(s, d), a), c128);
        let up = u16x8_shr(i16x8_add(tp, u16x8_shr(tp, 8)), 8);
        let zn = u16x8_sub_sat(i16x8_mul(u16x8_sub_sat(d, s), a), c128);
        let wn = i16x8_add(zn, u16x8_shr(i16x8_add(zn, c255), 8));
        let down = u16x8_shr(i16x8_add(wn, c255), 8);
        i16x8_sub(i16x8_add(d, up), down)
    };
    for (d, s) in dc.iter_mut().zip(sc) {
        // SAFETY: 16-byte loads and stores inside 16-byte chunks.
        let (sv, dv) = unsafe { (v128_load(s.as_ptr() as *const v128), v128_load(d.as_ptr() as *const v128)) };
        let lo = blend(u16x8_extend_low_u8x16(sv), u16x8_extend_low_u8x16(dv));
        let hi = blend(u16x8_extend_high_u8x16(sv), u16x8_extend_high_u8x16(dv));
        unsafe { v128_store(d.as_mut_ptr() as *mut v128, v128_or(u8x16_narrow_i16x8(lo, hi), alpha)) };
    }
    blend_row_onto_opaque_scalar(dr, sr, opacity);
}

#[cfg(not(any(target_arch = "x86_64", all(target_arch = "wasm32", target_feature = "simd128"))))]
fn blend_row_onto_opaque(dst: &mut [u8], src: &[u8], opacity: u8) {
    blend_row_onto_opaque_scalar(dst, src, opacity);
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

    #[test]
    fn the_opaque_fast_path_matches_over() {
        // Random layer pixels over random opaque pixels, at many opacities and every
        // source alpha: the fast path gives exactly what the general one does.
        let mut seed = 0x1234_5678u32;
        let mut rnd = move || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed
        };
        for opacity in [0u8, 1, 37, 128, 200, 254, 255] {
            let mut tile = Tile::filled([0; 4]);
            for (i, b) in tile.0.iter_mut().enumerate() {
                *b = if i % 4 == 3 { (i / 4 % 256) as u8 } else { rnd() as u8 };
            }
            let below: Vec<u8> = (0..crate::tiles::TILE_BYTES).map(|i| if i % 4 == 3 { 255 } else { rnd() as u8 }).collect();
            let (mut general, mut fast) = (below.clone(), below.clone());
            // Odd sizes too (the end of a canvas): 61 columns, 50 rows.
            for (cols, rows) in [(64, 64), (61, 50)] {
                composite_tile(&mut general, 64, &tile, cols, rows, opacity, BlendMode::Normal);
                composite_tile_onto_opaque(&mut fast, 64, &tile, cols, rows, opacity);
                assert_eq!(general, fast, "opacity {opacity}, {cols} x {rows}");
            }
        }
    }

    #[test]
    fn the_division_table_is_the_division() {
        for a in 0..=255u8 {
            for b in 1..=255u8 {
                assert_eq!(div8(a, b), ((a as u32 * 255 + b as u32 / 2) / b as u32).min(255) as u8, "{a} / {b}");
            }
        }
    }
}
