//! Plain RGBA8 images: PNG reading and writing, thumbnails.

use std::io::Cursor;

/// Row-major RGBA8, straight (not premultiplied) alpha.
#[derive(Clone, PartialEq, Eq)]
pub struct Rgba8 {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl std::fmt::Debug for Rgba8 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rgba8({}x{})", self.width, self.height)
    }
}

impl Rgba8 {
    pub fn new(width: u32, height: u32) -> Rgba8 {
        Rgba8 { width, height, data: vec![0; width as usize * height as usize * 4] }
    }

    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = (y as usize * self.width as usize + x as usize) * 4;
        [self.data[i], self.data[i + 1], self.data[i + 2], self.data[i + 3]]
    }
}

/// Encodes as an 8-bit RGBA PNG. `fast` trades file size for speed (autosave, previews).
pub fn encode_png(img: &Rgba8, fast: bool) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, img.width, img.height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_compression(if fast { png::Compression::Fast } else { png::Compression::Balanced });
    let mut wr = enc.write_header().map_err(|e| format!("PNG: {e}"))?;
    wr.write_image_data(&img.data).map_err(|e| format!("PNG: {e}"))?;
    wr.finish().map_err(|e| format!("PNG: {e}"))?;
    Ok(out)
}

/// Encodes as a JPEG at `quality` (1..=100), with see-through areas filled with
/// `background` (JPEG has no transparency; Krita fills with white by default). Colour is
/// not subsampled (4:4:4), which keeps sharp coloured lines clean.
pub fn encode_jpeg(img: &Rgba8, quality: u8, background: [u8; 3], dpi: f64) -> Result<Vec<u8>, String> {
    let (w, h) = (u16::try_from(img.width).map_err(|_| "too wide for JPEG")?, u16::try_from(img.height).map_err(|_| "too tall for JPEG")?);
    let mut rgb = Vec::with_capacity(img.width as usize * img.height as usize * 3);
    for p in img.data.as_chunks::<4>().0 {
        let mut px = [background[0], background[1], background[2], 255];
        crate::doc::over(&mut px, *p, 255);
        rgb.extend_from_slice(&px[..3]);
    }
    let mut out = Vec::new();
    let mut enc = jpeg_encoder::Encoder::new(&mut out, quality.clamp(1, 100));
    enc.set_sampling_factor(jpeg_encoder::SamplingFactor::F_1_1);
    enc.set_optimized_huffman_tables(true);
    if dpi.is_finite() && dpi >= 1.0 {
        enc.set_density(jpeg_encoder::PixelDensity::dpi(dpi.round().min(65535.0) as u16));
    }
    enc.encode(&rgb, w, h, jpeg_encoder::ColorType::Rgb).map_err(|e| format!("JPEG: {e}"))?;
    Ok(out)
}

/// Largest picture accepted from a file (pixels), so a broken or hostile file cannot make
/// Recedo allocate gigabytes.
pub const MAX_PIXELS: u64 = 400_000_000;

/// Decodes any PNG (grey, palette, 16-bit, with or without alpha) to RGBA8.
pub fn decode_png(bytes: &[u8]) -> Result<Rgba8, String> {
    let mut dec = png::Decoder::new(Cursor::new(bytes));
    dec.set_transformations(png::Transformations::normalize_to_color8() | png::Transformations::ALPHA);
    let mut reader = dec.read_info().map_err(|e| format!("PNG: {e}"))?;
    let (w, h) = (reader.info().width, reader.info().height);
    if w as u64 * h as u64 > MAX_PIXELS {
        return Err(format!("PNG is too large ({w} x {h})"));
    }
    let size = reader.output_buffer_size().ok_or("PNG is too large")?;
    let mut buf = vec![0; size];
    let info = reader.next_frame(&mut buf).map_err(|e| format!("PNG: {e}"))?;
    buf.truncate(info.buffer_size());
    let data = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::GrayscaleAlpha => buf.as_chunks::<2>().0.iter().flat_map(|&[g, a]| [g, g, g, a]).collect(),
        png::ColorType::Rgb => buf.as_chunks::<3>().0.iter().flat_map(|&[r, g, b]| [r, g, b, 255]).collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        other => return Err(format!("PNG colour type {other:?} not supported")),
    };
    if data.len() != w as usize * h as usize * 4 {
        return Err("PNG decoded to an unexpected size".into());
    }
    Ok(Rgba8 { width: w, height: h, data })
}

/// Scales down to fit in `max` x `max` (never up), averaging the covered pixels with
/// their alpha so transparent pixels do not darken edges.
pub fn thumbnail(img: &Rgba8, max: u32) -> Rgba8 {
    let scale = (max as f64 / img.width.max(1) as f64).min(max as f64 / img.height.max(1) as f64).min(1.0);
    let tw = ((img.width as f64 * scale).round() as u32).max(1);
    let th = ((img.height as f64 * scale).round() as u32).max(1);
    let mut out = Rgba8::new(tw, th);
    for ty in 0..th {
        let y0 = (ty as u64 * img.height as u64 / th as u64) as u32;
        let y1 = (((ty + 1) as u64 * img.height as u64 / th as u64) as u32).max(y0 + 1);
        for tx in 0..tw {
            let x0 = (tx as u64 * img.width as u64 / tw as u64) as u32;
            let x1 = (((tx + 1) as u64 * img.width as u64 / tw as u64) as u32).max(x0 + 1);
            let mut sum = [0u64; 4];
            for y in y0..y1.min(img.height) {
                for x in x0..x1.min(img.width) {
                    let p = img.pixel(x, y);
                    let a = p[3] as u64;
                    sum[0] += p[0] as u64 * a;
                    sum[1] += p[1] as u64 * a;
                    sum[2] += p[2] as u64 * a;
                    sum[3] += a;
                }
            }
            let n = ((y1 - y0) * (x1 - x0)) as u64;
            let i = ((ty * tw + tx) * 4) as usize;
            if let Some(alpha) = std::num::NonZeroU64::new(sum[3]) {
                for (o, s) in out.data[i..i + 3].iter_mut().zip(sum) {
                    *o = ((s + alpha.get() / 2) / alpha) as u8;
                }
                out.data[i + 3] = ((alpha.get() + n / 2) / n) as u8;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_roundtrip() {
        let mut img = Rgba8::new(5, 3);
        for (i, b) in img.data.iter_mut().enumerate() {
            *b = (i * 13 % 256) as u8;
        }
        for fast in [false, true] {
            let back = decode_png(&encode_png(&img, fast).unwrap()).unwrap();
            assert_eq!(back, img);
        }
        assert!(decode_png(b"not a png").is_err());
    }

    #[test]
    fn jpeg_export() {
        let mut img = Rgba8::new(96, 64);
        for y in 0..64u32 {
            for x in 0..96u32 {
                let i = ((y * 96 + x) * 4) as usize;
                // Left: colour gradient, right third: transparent.
                let a = if x < 64 { 255 } else { 0 };
                img.data[i..i + 4].copy_from_slice(&[(x * 4) as u8, (y * 4) as u8, 128, a]);
            }
        }
        let jpg = encode_jpeg(&img, 95, [255, 255, 255], 300.0).unwrap();
        let mut dec = jpeg_decoder::Decoder::new(std::io::Cursor::new(&jpg));
        let px = dec.decode().unwrap();
        let info = dec.info().unwrap();
        assert_eq!((info.width, info.height), (96, 64));
        let mut worst = 0u8;
        for y in 0..64usize {
            for x in 0..96usize {
                let i = (y * 96 + x) * 3;
                let want = if x < 64 { [(x * 4) as u8, (y * 4) as u8, 128] } else { [255, 255, 255] };
                // Skip the pixels next to the sharp edge, where JPEG rings.
                if (60..68).contains(&x) {
                    continue;
                }
                for c in 0..3 {
                    worst = worst.max(px[i + c].abs_diff(want[c]));
                }
            }
        }
        assert!(worst <= 8, "JPEG at quality 95 off by {worst}");
        assert!(encode_jpeg(&Rgba8::new(70_000, 1), 90, [0; 3], 72.0).is_err());
    }

    #[test]
    fn thumbnail_keeps_aspect_and_ignores_transparent_colour() {
        let mut img = Rgba8::new(400, 100);
        // Left half opaque red, right half transparent *black*.
        for y in 0..100 {
            for x in 0..200 {
                let i = ((y * 400 + x) * 4) as usize;
                img.data[i..i + 4].copy_from_slice(&[255, 0, 0, 255]);
            }
        }
        let t = thumbnail(&img, 256);
        assert_eq!((t.width, t.height), (256, 64));
        assert_eq!(t.pixel(10, 10), [255, 0, 0, 255]);
        assert_eq!(t.pixel(250, 10)[3], 0);
        // A pixel covering red and transparent black stays pure red, half transparent.
        let mut small = Rgba8::new(4, 2);
        for y in 0..2 {
            let i = (y * 4 * 4) as usize;
            small.data[i..i + 4].copy_from_slice(&[255, 0, 0, 255]);
        }
        assert_eq!(thumbnail(&small, 2).pixel(0, 0), [255, 0, 0, 128]);
        // Small images are not scaled up.
        assert_eq!(thumbnail(&Rgba8::new(10, 20), 256).width, 10);
    }
}
