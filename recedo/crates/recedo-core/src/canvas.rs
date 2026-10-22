//! New-canvas settings: presets (built-in and the user's own), checking a custom size, and
//! how much memory a canvas will need.

use serde::{Deserialize, Serialize};

use crate::doc::{Document, MAX_SIDE};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Background {
    White,
    Black,
    Transparent,
    /// Straight-alpha RGBA.
    Color([u8; 4]),
}

impl Background {
    pub fn pixel(self) -> Option<[u8; 4]> {
        match self {
            Background::White => Some([255, 255, 255, 255]),
            Background::Black => Some([0, 0, 0, 255]),
            Background::Transparent => None,
            Background::Color(c) => (c[3] != 0).then_some(c),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CanvasPreset {
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// Pixels per inch.
    pub dpi: f64,
    pub background: Background,
}

impl CanvasPreset {
    fn new(name: &str, width: u32, height: u32, dpi: f64) -> CanvasPreset {
        CanvasPreset { name: name.into(), width, height, dpi, background: Background::White }
    }

    /// Checks the size and resolution; the message says what to change.
    pub fn validate(&self) -> Result<(), String> {
        if self.width == 0 || self.height == 0 {
            return Err("Width and height must be at least 1 pixel.".into());
        }
        if self.width > MAX_SIDE || self.height > MAX_SIDE {
            return Err(format!("Width and height can be at most {MAX_SIDE} pixels."));
        }
        if !(self.dpi.is_finite() && (1.0..=10_000.0).contains(&self.dpi)) {
            return Err("Resolution must be between 1 and 10000 pixels per inch.".into());
        }
        Ok(())
    }

    /// Memory one fully painted layer of this canvas takes (8-bit RGBA, 64-pixel tiles).
    pub fn layer_bytes(&self) -> u64 {
        self.width.div_ceil(64) as u64 * self.height.div_ceil(64) as u64 * 64 * 64 * 4
    }

    /// The new painting: a background layer (unless transparent) and an empty layer.
    pub fn create(&self) -> Result<Document, String> {
        self.validate()?;
        let mut doc = Document::new(self.width, self.height, self.background.pixel());
        doc.dpi = self.dpi;
        Ok(doc)
    }
}

/// The built-in presets, the user's two canvas sizes first.
pub fn builtin_presets() -> Vec<CanvasPreset> {
    vec![
        CanvasPreset::new("4K UHD (3840 × 2160)", 3840, 2160, 300.0),
        CanvasPreset::new("HD 720p (1280 × 720)", 1280, 720, 300.0),
        CanvasPreset::new("Full HD (1920 × 1080)", 1920, 1080, 300.0),
        CanvasPreset::new("Square (2048 × 2048)", 2048, 2048, 300.0),
        CanvasPreset::new("A4 portrait, 300 ppi", 2480, 3508, 300.0),
        CanvasPreset::new("A4 landscape, 300 ppi", 3508, 2480, 300.0),
        CanvasPreset::new("Letter portrait, 300 ppi", 2550, 3300, 300.0),
        CanvasPreset::new("Comic page (6.625 × 10.25 in), 600 ppi", 3975, 6150, 600.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_valid_and_create_paintings() {
        let presets = builtin_presets();
        assert_eq!((presets[0].width, presets[0].height), (3840, 2160));
        assert_eq!((presets[1].width, presets[1].height), (1280, 720));
        for p in &presets {
            p.validate().unwrap();
        }
        let doc = presets[1].create().unwrap();
        assert_eq!((doc.width, doc.height, doc.dpi), (1280, 720, 300.0));
        assert_eq!(doc.layers.len(), 2);
        assert_eq!(doc.flatten().pixel(5, 5), [255, 255, 255, 255]);
        let clear = CanvasPreset { background: Background::Transparent, ..presets[1].clone() }.create().unwrap();
        assert_eq!(clear.layers.len(), 1);
        assert_eq!(presets[0].layer_bytes(), 60 * 34 * 16384, "4K: 60 x 34 tiles");
    }

    #[test]
    fn bad_sizes_are_explained() {
        let p = |w, h, dpi| CanvasPreset { name: "x".into(), width: w, height: h, dpi, background: Background::White };
        assert!(p(0, 10, 72.0).validate().unwrap_err().contains("at least 1"));
        assert!(p(40_000, 10, 72.0).validate().unwrap_err().contains("at most"));
        assert!(p(10, 10, 0.0).validate().is_err());
        assert!(p(10, 10, f64::NAN).validate().is_err());
        assert!(p(10, 10, 72.0).create().is_ok());
        // Custom presets save and load.
        let custom = CanvasPreset { background: Background::Color([10, 20, 30, 255]), ..p(500, 400, 144.0) };
        let back: CanvasPreset = serde_json::from_str(&serde_json::to_string(&custom).unwrap()).unwrap();
        assert_eq!(back, custom);
        assert_eq!(back.create().unwrap().flatten().pixel(0, 0), [10, 20, 30, 255]);
    }
}
