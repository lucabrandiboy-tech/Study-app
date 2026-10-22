//! Opening pictures: Krita files, PNG and JPEG, recognised by their content (a renamed
//! file still opens).

use std::path::Path;

use crate::doc::{Document, Layer, MAX_SIDE};
use crate::image::{self, MAX_PIXELS, Rgba8};
use crate::kra::{self, Loaded};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Kra,
    Png,
    Jpeg,
}

pub fn detect(bytes: &[u8]) -> Option<Format> {
    if bytes.starts_with(b"PK\x03\x04") {
        Some(Format::Kra)
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(Format::Png)
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(Format::Jpeg)
    } else {
        None
    }
}

/// Decodes a JPEG (grey, RGB or CMYK) to RGBA8.
pub fn decode_jpeg(bytes: &[u8]) -> Result<Rgba8, String> {
    let mut dec = jpeg_decoder::Decoder::new(std::io::Cursor::new(bytes));
    dec.read_info().map_err(|e| format!("JPEG: {e}"))?;
    let info = dec.info().ok_or("JPEG: no picture information")?;
    let (w, h) = (info.width as u32, info.height as u32);
    if w as u64 * h as u64 > MAX_PIXELS {
        return Err(format!("JPEG is too large ({w} x {h})"));
    }
    let px = dec.decode().map_err(|e| format!("JPEG: {e}"))?;
    let data: Vec<u8> = match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => px.as_chunks::<3>().0.iter().flat_map(|&[r, g, b]| [r, g, b, 255]).collect(),
        jpeg_decoder::PixelFormat::L8 => px.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        jpeg_decoder::PixelFormat::CMYK32 => px
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|&[c, m, y, k]| {
                let f = |v: u8| ((255 - v as u32) * (255 - k as u32) / 255) as u8;
                [f(c), f(m), f(y), 255]
            })
            .collect(),
        jpeg_decoder::PixelFormat::L16 => return Err("16-bit grey JPEGs are not supported".into()),
    };
    if data.len() != w as usize * h as usize * 4 {
        return Err("JPEG decoded to an unexpected size".into());
    }
    Ok(Rgba8 { width: w, height: h, data })
}

/// A one-layer painting from a picture.
pub fn document_from_image(img: &Rgba8) -> Result<Document, String> {
    if img.width == 0 || img.height == 0 || img.width > MAX_SIDE || img.height > MAX_SIDE {
        return Err(format!("Pictures up to {MAX_SIDE} pixels a side can be opened ({} x {})", img.width, img.height));
    }
    let mut layer = Layer::new("Background");
    layer.pixels.write_rect(0, 0, img.width, img.height, &img.data);
    Ok(Document { width: img.width, height: img.height, dpi: 72.0, layers: vec![layer] })
}

/// Opens a `.kra`, PNG or JPEG from its bytes.
pub fn open(bytes: &[u8]) -> Result<Loaded, String> {
    match detect(bytes) {
        Some(Format::Kra) => kra::load(bytes),
        Some(Format::Png) => Ok(Loaded { doc: document_from_image(&image::decode_png(bytes)?)?, warnings: Vec::new() }),
        Some(Format::Jpeg) => Ok(Loaded { doc: document_from_image(&decode_jpeg(bytes)?)?, warnings: Vec::new() }),
        None => Err("This is not a picture Recedo can open (Krita .kra, PNG or JPEG).".into()),
    }
}

pub fn open_file(path: &Path) -> Result<Loaded, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Could not open {}: {e}", path.display()))?;
    open(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture() -> Rgba8 {
        let mut img = Rgba8::new(40, 30);
        for (i, p) in img.data.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            *p = [(i % 40 * 6) as u8, (i / 40 * 8) as u8, 100, 255];
        }
        img
    }

    #[test]
    fn opens_png_and_kra_by_content() {
        let img = picture();
        let png = image::encode_png(&img, true).unwrap();
        assert_eq!(detect(&png), Some(Format::Png));
        let l = open(&png).unwrap();
        assert_eq!(l.doc.flatten(), img);
        let kra_bytes = kra::save(&l.doc).unwrap();
        assert_eq!(detect(&kra_bytes), Some(Format::Kra));
        assert_eq!(open(&kra_bytes).unwrap().doc.flatten(), img);
        assert!(open(b"GIF89a....").is_err());
        assert!(open(b"").is_err());
    }

    #[test]
    fn opens_jpeg_colour_and_grey() {
        let img = picture();
        let jpg = image::encode_jpeg(&img, 98, [255; 3], 72.0).unwrap();
        assert_eq!(detect(&jpg), Some(Format::Jpeg));
        let back = open(&jpg).unwrap().doc.flatten();
        assert_eq!((back.width, back.height), (40, 30));
        let worst = back.data.iter().zip(&img.data).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
        assert!(worst <= 10, "{worst}");
        // Greyscale JPEG.
        let mut grey = Vec::new();
        let enc = jpeg_encoder::Encoder::new(&mut grey, 95);
        enc.encode(&[200u8; 16 * 8], 16, 8, jpeg_encoder::ColorType::Luma).unwrap();
        let g = decode_jpeg(&grey).unwrap();
        assert!(g.pixel(3, 3)[0].abs_diff(200) <= 2 && g.pixel(3, 3)[0] == g.pixel(3, 3)[2]);
        // Truncated or garbage JPEG data is an error, not a crash.
        for cut in (0..jpg.len()).step_by(97) {
            let _ = decode_jpeg(&jpg[..cut]);
        }
        assert!(decode_jpeg(&[0xFF, 0xD8, 0xFF, 0x00, 0x01]).is_err());
    }

    #[test]
    fn huge_or_empty_pictures_are_refused() {
        assert!(document_from_image(&Rgba8 { width: 0, height: 5, data: Vec::new() }).is_err());
        assert!(document_from_image(&Rgba8 { width: MAX_SIDE + 1, height: 1, data: vec![0; (MAX_SIDE as usize + 1) * 4] }).is_err());
    }
}
