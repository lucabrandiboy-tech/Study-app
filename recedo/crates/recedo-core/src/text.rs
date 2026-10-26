//! Text for the text tool: lines of text turned into a coverage mask with the fonts the
//! interface itself uses (Ubuntu Light as "Sans", Hack as "Mono"), so no font files are
//! needed. Glyph outlines come from `skrifa`, are flattened to polygons and filled with
//! [`crate::fill::polygons`] (nonzero winding keeps the holes in letters like "o").

use serde::{Deserialize, Serialize};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::OutlinePen;
use skrifa::{FontRef, GlyphId, MetadataProvider};

use crate::fill::{Mask, polygons};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Font {
    #[default]
    Sans,
    Mono,
}

impl Font {
    pub const ALL: [Font; 2] = [Font::Sans, Font::Mono];

    pub fn name(self) -> &'static str {
        match self {
            Font::Sans => "Sans",
            Font::Mono => "Mono",
        }
    }

    fn data(self) -> &'static [u8] {
        match self {
            Font::Sans => epaint_default_fonts::UBUNTU_LIGHT,
            Font::Mono => epaint_default_fonts::HACK_REGULAR,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextStyle {
    pub font: Font,
    /// Letter size (the font's em) in canvas pixels.
    pub size: f32,
    /// Thicker strokes.
    pub bold: bool,
    /// Distance between lines, as a multiple of the font's own.
    pub line_spacing: f32,
}

impl Default for TextStyle {
    fn default() -> TextStyle {
        TextStyle { font: Font::Sans, size: 64.0, bold: false, line_spacing: 1.0 }
    }
}

/// Glyph outlines, placed: closed polygons in canvas pixels.
struct Layout {
    contours: Vec<Vec<(f64, f64)>>,
    width: f64,
    height: f64,
}

fn layout(text: &str, style: &TextStyle, at: (f64, f64)) -> Layout {
    let size = style.size.clamp(1.0, 2000.0);
    let font = FontRef::new(style.font.data()).expect("the built-in fonts are valid");
    let (sz, loc) = (Size::new(size), LocationRef::default());
    let metrics = font.metrics(sz, loc);
    let glyph_metrics = font.glyph_metrics(sz, loc);
    let (charmap, outlines) = (font.charmap(), font.outline_glyphs());
    let line_height = ((metrics.ascent - metrics.descent + metrics.leading) * style.line_spacing.clamp(0.5, 4.0)) as f64;
    let mut pen = Flatten { contours: Vec::new(), current: Vec::new(), origin: (0.0, 0.0), tolerance: 0.15 };
    let mut width: f64 = 0.0;
    let lines: Vec<&str> = text.split('\n').collect();
    for (i, line) in lines.iter().enumerate() {
        let baseline = at.1 + metrics.ascent as f64 + i as f64 * line_height;
        let mut x = at.0;
        for ch in line.chars().filter(|c| !c.is_control()) {
            // Letters the font lacks show as its "missing" box, as in Krita.
            let gid = charmap.map(ch).unwrap_or(GlyphId::NOTDEF);
            if let Some(glyph) = outlines.get(gid) {
                pen.origin = (x, baseline);
                let _ = glyph.draw((sz, loc), &mut pen);
                pen.finish();
            }
            x += glyph_metrics.advance_width(gid).unwrap_or(size * 0.5) as f64;
        }
        width = width.max(x - at.0);
    }
    let height = line_height * (lines.len() as f64 - 1.0) + (metrics.ascent - metrics.descent) as f64;
    Layout { contours: pen.contours, width, height }
}

/// The size the text takes, (width, height) in canvas pixels.
pub fn measure(text: &str, style: &TextStyle) -> (f64, f64) {
    let l = layout(text, style, (0.0, 0.0));
    (l.width, l.height)
}

/// The text as a mask on a width x height canvas, with the top-left corner of its first
/// line at `at`.
pub fn render(width: u32, height: u32, text: &str, style: &TextStyle, at: (f64, f64)) -> Mask {
    let mut mask = polygons(width, height, &layout(text, style, at).contours);
    if style.bold {
        let r = (style.size / 24.0).round().max(1.0) as usize;
        thicken(&mut mask, r);
    }
    mask
}

/// Grows the covered area by `r` pixels (the largest coverage nearby, along rows then
/// columns), for bold text.
fn thicken(mask: &mut Mask, r: usize) {
    if mask.is_empty() {
        return;
    }
    let (w, h) = (mask.width as usize, mask.height as usize);
    let (x0, y0, x1, y1) = mask.bounds;
    let (x0, y0) = ((x0 as usize).saturating_sub(r), (y0 as usize).saturating_sub(r));
    let (x1, y1) = ((x1 as usize + r).min(w), (y1 as usize + r).min(h));
    let src = mask.data.clone();
    let mut rows = src.clone();
    for y in y0..y1 {
        for x in x0..x1 {
            let (a, b) = (x.saturating_sub(r), (x + r + 1).min(w));
            rows[y * w + x] = src[y * w + a..y * w + b].iter().copied().max().unwrap_or(0);
        }
    }
    for y in y0..y1 {
        for x in x0..x1 {
            let (a, b) = (y.saturating_sub(r), (y + r + 1).min(h));
            mask.data[y * w + x] = (a..b).map(|yy| rows[yy * w + x]).max().unwrap_or(0);
        }
    }
    mask.update_bounds();
}

/// Turns glyph outlines into polygons: curves become short lines (within `tolerance`
/// pixels), font units become canvas pixels with y pointing down.
struct Flatten {
    contours: Vec<Vec<(f64, f64)>>,
    current: Vec<(f64, f64)>,
    /// Where the glyph's origin (on the baseline) is on the canvas.
    origin: (f64, f64),
    tolerance: f64,
}

impl Flatten {
    fn point(&self, x: f32, y: f32) -> (f64, f64) {
        (self.origin.0 + x as f64, self.origin.1 - y as f64)
    }

    fn last(&self) -> (f64, f64) {
        *self.current.last().unwrap_or(&self.origin)
    }

    fn finish(&mut self) {
        if self.current.len() >= 3 {
            self.contours.push(std::mem::take(&mut self.current));
        } else {
            self.current.clear();
        }
    }

    /// Enough segments that the curve stays within the tolerance.
    fn steps(&self, bend: f64) -> usize {
        ((bend / (4.0 * self.tolerance)).sqrt().ceil() as usize).clamp(1, 64)
    }
}

impl OutlinePen for Flatten {
    fn move_to(&mut self, x: f32, y: f32) {
        self.finish();
        let p = self.point(x, y);
        self.current.push(p);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.point(x, y);
        self.current.push(p);
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let (p0, p1, p2) = (self.last(), self.point(cx, cy), self.point(x, y));
        let bend = (p0.0 - 2.0 * p1.0 + p2.0).hypot(p0.1 - 2.0 * p1.1 + p2.1);
        let n = self.steps(bend);
        for i in 1..=n {
            let t = i as f64 / n as f64;
            let u = 1.0 - t;
            self.current.push((u * u * p0.0 + 2.0 * u * t * p1.0 + t * t * p2.0, u * u * p0.1 + 2.0 * u * t * p1.1 + t * t * p2.1));
        }
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        let (p0, p1, p2, p3) = (self.last(), self.point(cx0, cy0), self.point(cx1, cy1), self.point(x, y));
        let b1 = (p0.0 - 2.0 * p1.0 + p2.0).hypot(p0.1 - 2.0 * p1.1 + p2.1);
        let b2 = (p1.0 - 2.0 * p2.0 + p3.0).hypot(p1.1 - 2.0 * p2.1 + p3.1);
        let n = self.steps(1.5 * b1.max(b2));
        for i in 1..=n {
            let t = i as f64 / n as f64;
            let u = 1.0 - t;
            let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            self.current.push((a * p0.0 + b * p1.0 + c * p2.0 + d * p3.0, a * p0.1 + b * p1.1 + c * p2.1 + d * p3.1));
        }
    }

    fn close(&mut self) {
        self.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn covered(m: &Mask) -> usize {
        m.data.iter().filter(|&&v| v >= 128).count()
    }

    #[test]
    fn letters_are_where_the_text_goes() {
        let style = TextStyle { size: 60.0, ..TextStyle::default() };
        let m = render(400, 200, "Hi", &style, (20.0, 30.0));
        assert!(!m.is_empty());
        let (x0, y0, x1, y1) = m.bounds;
        assert!(x0 >= 20 && y0 >= 30, "starts at the corner given: {:?}", m.bounds);
        assert!(x1 < 20 + 80 && y1 < 30 + 70, "about the size of two letters at 60 px: {:?}", m.bounds);
        // The H's left stem: solid ink part way down the letter.
        let stem = (x0..x0 + 10).any(|x| m.get(x as i32, ((y0 + y1) / 2) as i32) == 255);
        assert!(stem, "{:?}", m.bounds);
        let (w, h) = measure("Hi", &style);
        assert!((w - (x1 - 20) as f64).abs() < 15.0 && h > 50.0 && h < 80.0, "{w} x {h}");
    }

    #[test]
    fn holes_stay_open_and_lines_stack() {
        let style = TextStyle { size: 80.0, ..TextStyle::default() };
        let o = render(200, 200, "o", &style, (10.0, 10.0));
        let (x0, y0, x1, y1) = o.bounds;
        let (cx, cy) = (((x0 + x1) / 2) as i32, ((y0 + y1) / 2) as i32);
        assert_eq!(o.get(cx, cy), 0, "the inside of an o is empty");
        assert!((x0 as i32..cx).any(|x| o.get(x, cy) == 255), "its ring is inked");
        let two = render(300, 400, "A\nB", &style, (10.0, 10.0));
        let one = render(300, 400, "A", &style, (10.0, 10.0));
        assert!(two.bounds.3 > one.bounds.3 + 50, "the second line is below the first");
        assert!(measure("A\nB", &style).1 > measure("A", &style).1 + 50.0);
    }

    #[test]
    fn fonts_bold_size_and_odd_input() {
        let sans = TextStyle { size: 40.0, ..TextStyle::default() };
        let mono = TextStyle { font: Font::Mono, ..sans };
        // In the monospace font an "i" is as wide as an "m".
        let (wi, wm) = (measure("iiii", &mono).0, measure("mmmm", &mono).0);
        assert!((wi - wm).abs() < 0.01, "{wi} {wm}");
        assert!(measure("iiii", &sans).0 < measure("mmmm", &sans).0 * 0.6);
        let normal = covered(&render(300, 100, "Bold", &sans, (5.0, 5.0)));
        let bold = covered(&render(300, 100, "Bold", &TextStyle { bold: true, ..sans }, (5.0, 5.0)));
        assert!(bold > normal * 5 / 4, "{normal} -> {bold}");
        let big = covered(&render(600, 300, "Bold", &TextStyle { size: 80.0, ..sans }, (5.0, 5.0)));
        assert!(big > normal * 3, "twice the size, about four times the ink: {normal} -> {big}");
        // Empty text, spaces, a letter the fonts lack, text off the canvas: no panic.
        assert!(render(100, 100, "", &sans, (0.0, 0.0)).is_empty());
        assert!(render(100, 100, "   ", &sans, (0.0, 0.0)).is_empty());
        assert!(!render(100, 100, "\u{1F600}", &sans, (5.0, 5.0)).is_empty(), "a box for a missing letter");
        assert!(render(100, 100, "far away", &sans, (5000.0, 5000.0)).is_empty());
    }
}
