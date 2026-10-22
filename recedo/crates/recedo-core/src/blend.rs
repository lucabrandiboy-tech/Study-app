//! Layer blend modes, with Krita's formulas and ids (as stored in `.kra` files).
//!
//! Separable modes follow Krita's `KoCompositeOpGenericSC`: the blend function `f(src, dst)`
//! is mixed in where both layers are opaque, and each layer shows through where the other
//! is transparent:
//!
//! ```text
//! a   = sa + da - sa*da
//! out = (src*sa*(1-da) + dst*da*(1-sa) + f(src, dst)*sa*da) / a
//! ```

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    HardLight,
    SoftLight,
    ColorDodge,
    ColorBurn,
    LinearDodge,
    LinearBurn,
    Darken,
    Lighten,
    Difference,
    Exclusion,
    Subtract,
    Divide,
}

impl BlendMode {
    pub const ALL: [BlendMode; 16] = [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
        BlendMode::HardLight,
        BlendMode::SoftLight,
        BlendMode::ColorDodge,
        BlendMode::ColorBurn,
        BlendMode::LinearDodge,
        BlendMode::LinearBurn,
        BlendMode::Darken,
        BlendMode::Lighten,
        BlendMode::Difference,
        BlendMode::Exclusion,
        BlendMode::Subtract,
        BlendMode::Divide,
    ];

    /// Krita's id (the `compositeop` attribute in `.kra` files).
    pub fn krita_id(self) -> &'static str {
        match self {
            BlendMode::Normal => "normal",
            BlendMode::Multiply => "multiply",
            BlendMode::Screen => "screen",
            BlendMode::Overlay => "overlay",
            BlendMode::HardLight => "hard_light",
            BlendMode::SoftLight => "soft_light",
            BlendMode::ColorDodge => "dodge",
            BlendMode::ColorBurn => "burn",
            BlendMode::LinearDodge => "linear_dodge",
            BlendMode::LinearBurn => "linear_burn",
            BlendMode::Darken => "darken",
            BlendMode::Lighten => "lighten",
            BlendMode::Difference => "diff",
            BlendMode::Exclusion => "exclusion",
            BlendMode::Subtract => "subtract",
            BlendMode::Divide => "divide",
        }
    }

    pub fn from_krita_id(id: &str) -> Option<BlendMode> {
        BlendMode::ALL.into_iter().find(|m| m.krita_id() == id)
    }

    /// The name Krita shows.
    pub fn name(self) -> &'static str {
        match self {
            BlendMode::Normal => "Normal",
            BlendMode::Multiply => "Multiply",
            BlendMode::Screen => "Screen",
            BlendMode::Overlay => "Overlay",
            BlendMode::HardLight => "Hard Light",
            BlendMode::SoftLight => "Soft Light (Photoshop)",
            BlendMode::ColorDodge => "Color Dodge",
            BlendMode::ColorBurn => "Color Burn",
            BlendMode::LinearDodge => "Linear Dodge",
            BlendMode::LinearBurn => "Linear Burn",
            BlendMode::Darken => "Darken",
            BlendMode::Lighten => "Lighten",
            BlendMode::Difference => "Difference",
            BlendMode::Exclusion => "Exclusion",
            BlendMode::Subtract => "Subtract",
            BlendMode::Divide => "Divide",
        }
    }

    /// The blend function on 8-bit values. Where Krita's integer version rounds differently
    /// from the exact formula, Krita's is used.
    fn f8(self, s: u8, d: u8) -> u8 {
        let div = |a: u8, b: u8| ((a as u32 * 255 + b as u32 / 2) / b as u32).min(255) as u8;
        match self {
            BlendMode::Exclusion => (s as i32 + d as i32 - 2 * crate::doc::mul8(s, d) as i32).clamp(0, 255) as u8,
            BlendMode::ColorDodge => {
                if d == 0 {
                    0
                } else if 255 - s < d {
                    255
                } else {
                    div(d, 255 - s)
                }
            }
            BlendMode::Divide => {
                if s == 0 {
                    if d == 0 { 0 } else { 255 }
                } else {
                    div(d, s)
                }
            }
            BlendMode::ColorBurn => {
                if d == 255 {
                    255
                } else if s < 255 - d {
                    0
                } else {
                    255 - div(255 - d, s)
                }
            }
            _ => (self.f(s as f32 / 255.0, d as f32 / 255.0) * 255.0).round() as u8,
        }
    }

    /// The blend function on one channel, 0..=1 (`s` = this layer, `d` = below).
    fn f(self, s: f32, d: f32) -> f32 {
        let hard_light = |s: f32, d: f32| if s > 0.5 { let s2 = 2.0 * s - 1.0; s2 + d - s2 * d } else { 2.0 * s * d };
        match self {
            BlendMode::Normal => s,
            BlendMode::Multiply => s * d,
            BlendMode::Screen => s + d - s * d,
            BlendMode::Overlay => hard_light(d, s),
            BlendMode::HardLight => hard_light(s, d),
            BlendMode::SoftLight => {
                if s > 0.5 {
                    d + (2.0 * s - 1.0) * (d.sqrt() - d)
                } else {
                    d - (1.0 - 2.0 * s) * d * (1.0 - d)
                }
            }
            BlendMode::ColorDodge => {
                if d == 0.0 {
                    0.0
                } else if 1.0 - s < d {
                    1.0
                } else {
                    d / (1.0 - s)
                }
            }
            BlendMode::ColorBurn => {
                if d == 1.0 {
                    1.0
                } else if s < 1.0 - d {
                    0.0
                } else {
                    1.0 - (1.0 - d) / s
                }
            }
            BlendMode::LinearDodge => (s + d).min(1.0),
            BlendMode::LinearBurn => (s + d - 1.0).max(0.0),
            BlendMode::Darken => s.min(d),
            BlendMode::Lighten => s.max(d),
            BlendMode::Difference => (s - d).abs(),
            BlendMode::Exclusion => s + d - 2.0 * s * d,
            BlendMode::Subtract => (d - s).max(0.0),
            BlendMode::Divide => {
                if s == 0.0 {
                    if d == 0.0 { 0.0 } else { 1.0 }
                } else {
                    (d / s).min(1.0)
                }
            }
        }
        .clamp(0.0, 1.0)
    }
}

/// `a * b * c / 255²`, rounded, as Krita computes it for 8-bit channels (`UINT8_MULT3`).
#[inline]
fn mul3(a: u8, b: u8, c: u8) -> u32 {
    let t = a as u32 * b as u32 * c as u32 + 0x7F5B;
    ((t >> 7) + t) >> 16
}

/// Composites `src` (this layer's pixel, straight alpha) over `dst` with `opacity` using
/// `mode`, with Krita's 8-bit integer arithmetic (`KoCompositeOpGenericSC`), so results
/// match Krita's, including its rounding on nearly transparent pixels.
#[inline]
pub fn blend(dst: &mut [u8; 4], src: [u8; 4], opacity: u8, mode: BlendMode) {
    if mode == BlendMode::Normal {
        crate::doc::over(dst, src, opacity);
        return;
    }
    // Unlike Normal, Krita's other modes also visit fully transparent source pixels (inside
    // the layer's tiles), which re-rounds a semi-transparent destination; do the same.
    let sa = crate::doc::mul8(src[3], opacity);
    let da = dst[3];
    let na = sa as u32 + da as u32 - crate::doc::mul8(sa, da) as u32;
    if na == 0 {
        return;
    }
    for c in 0..3 {
        let f = mode.f8(src[c], dst[c]);
        let mixed = mul3(255 - sa, da, dst[c]) + mul3(255 - da, sa, src[c]) + mul3(sa, da, f);
        dst[c] = ((mixed * 255 + na / 2) / na).min(255) as u8;
    }
    dst[3] = na.min(255) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_roundtrip() {
        for m in BlendMode::ALL {
            assert_eq!(BlendMode::from_krita_id(m.krita_id()), Some(m));
        }
        assert_eq!(BlendMode::from_krita_id("luminize"), None);
    }

    #[test]
    fn opaque_results_match_the_formulas() {
        let s = [200, 100, 50, 255];
        let d = [100, 150, 250, 255];
        let check = |m: BlendMode, want: [u8; 3]| {
            let mut px = d;
            blend(&mut px, s, 255, m);
            for c in 0..3 {
                assert!(px[c].abs_diff(want[c]) <= 1, "{m:?}: got {px:?}, want {want:?}");
            }
            assert_eq!(px[3], 255);
        };
        check(BlendMode::Multiply, [78, 59, 49]);
        check(BlendMode::Screen, [222, 191, 251]);
        check(BlendMode::Darken, [100, 100, 50]);
        check(BlendMode::Lighten, [200, 150, 250]);
        check(BlendMode::Difference, [100, 50, 200]);
        check(BlendMode::LinearDodge, [255, 250, 255]);
        check(BlendMode::Subtract, [0, 50, 200]);
        // Overlay = hard light with the layers swapped: d < 0.5 gives 2*s*d, else screen.
        check(BlendMode::Overlay, [157, 127, 247]);
    }

    #[test]
    fn krita_integer_rounding_cases() {
        // Values where the exact formula and Krita's integer one round differently.
        assert_eq!(BlendMode::ColorDodge.f8(45, 7), 9);
        // A fully transparent pixel of a Multiply layer still re-rounds what is below it.
        let mut px = [0, 176, 200, 128];
        blend(&mut px, [9, 9, 9, 0], 255, BlendMode::Multiply);
        assert_eq!(px, [0, 175, 199, 128]);
        let mut px = [0, 176, 200, 128];
        blend(&mut px, [9, 9, 9, 0], 255, BlendMode::Normal);
        assert_eq!(px, [0, 176, 200, 128], "Normal leaves it alone");
    }

    #[test]
    fn transparent_parts_show_the_other_layer() {
        for m in BlendMode::ALL {
            // Source over nothing: the source itself.
            let mut px = [0, 0, 0, 0];
            blend(&mut px, [10, 200, 30, 255], 255, m);
            assert_eq!(px, [10, 200, 30, 255], "{m:?}");
            // Nothing over the destination: unchanged.
            let mut px = [10, 200, 30, 255];
            blend(&mut px, [255, 255, 255, 0], 255, m);
            assert_eq!(px, [10, 200, 30, 255], "{m:?}");
        }
    }

    #[test]
    fn opacity_mixes_towards_the_result() {
        let mut half = [100, 100, 100, 255];
        blend(&mut half, [0, 0, 0, 255], 128, BlendMode::Multiply);
        // Multiply with black gives black; at half opacity half way.
        assert!(half[0].abs_diff(50) <= 1, "{half:?}");
    }
}
