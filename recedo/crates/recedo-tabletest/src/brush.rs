//! The test brush: turns pen samples into evenly spaced round dabs.
//!
//! Deliberately raw: samples are joined with straight segments and nothing is smoothed,
//! so what you see is exactly what the tablet sent. Smoothing (the stabilizer) is Phase 1.

use bytemuck::{Pod, Zeroable};
use recedo_core::PaintInfo;
use recedo_tablet::{Flags, PenSample};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BrushSettings {
    /// Diameter in pixels at full pressure.
    pub size: f32,
    /// Diameter at zero pressure, as a fraction of `size` (when pressure controls size).
    pub min_size: f32,
    /// Stroke opacity 0..=1.
    pub opacity: f32,
    /// Edge hardness 0..=1.
    pub hardness: f32,
    /// Distance between dabs as a fraction of the diameter.
    pub spacing: f32,
    pub pressure_size: bool,
    pub pressure_opacity: bool,
    pub tilt_shape: bool,
    /// sRGB colour 0..=1.
    pub color: [f32; 3],
    pub eraser_end_erases: bool,
}

impl Default for BrushSettings {
    fn default() -> Self {
        BrushSettings {
            size: 24.0,
            min_size: 0.04,
            opacity: 1.0,
            hardness: 0.85,
            spacing: 0.08,
            pressure_size: true,
            pressure_opacity: false,
            tilt_shape: false,
            color: [0.08, 0.08, 0.1],
            eraser_end_erases: true,
        }
    }
}

/// One dab as uploaded to the GPU (48 bytes, see `shaders.wgsl`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct Dab {
    pub center: [f32; 2],
    /// Ellipse half-axes in pixels.
    pub radii: [f32; 2],
    /// Rotation of the first axis, radians.
    pub angle: f32,
    pub hardness: f32,
    pub _pad: [f32; 2],
    /// Premultiplied RGBA.
    pub color: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<Dab>() == 48);

/// Dabs that share a blend mode, in drawing order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DabBatch {
    pub erase: bool,
    pub dabs: Vec<Dab>,
}

/// Appends dabs to a batch list, starting a new batch when the blend mode changes.
pub fn push_dabs(batches: &mut Vec<DabBatch>, erase: bool, dabs: &[Dab]) {
    if dabs.is_empty() {
        return;
    }
    match batches.last_mut() {
        Some(b) if b.erase == erase => b.dabs.extend_from_slice(dabs),
        _ => batches.push(DabBatch { erase, dabs: dabs.to_vec() }),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Point {
    x: f64,
    y: f64,
    pressure: f32,
    tilt_x: f32,
    tilt_y: f32,
    has_tilt: bool,
}

impl Point {
    fn from(s: &PenSample) -> Point {
        Point {
            x: s.x,
            y: s.y,
            pressure: if s.flags.has(Flags::HAS_PRESSURE) { s.pressure } else { 1.0 },
            tilt_x: s.tilt_x,
            tilt_y: s.tilt_y,
            has_tilt: s.flags.has(Flags::HAS_TILT),
        }
    }

    fn from_info(p: &PaintInfo, has_tilt: bool) -> Point {
        Point { x: p.x, y: p.y, pressure: p.pressure, tilt_x: p.tilt_x, tilt_y: p.tilt_y, has_tilt }
    }

    fn lerp(a: &Point, b: &Point, t: f64) -> Point {
        let tf = t as f32;
        Point {
            x: a.x + (b.x - a.x) * t,
            y: a.y + (b.y - a.y) * t,
            pressure: a.pressure + (b.pressure - a.pressure) * tf,
            tilt_x: a.tilt_x + (b.tilt_x - a.tilt_x) * tf,
            tilt_y: a.tilt_y + (b.tilt_y - a.tilt_y) * tf,
            has_tilt: a.has_tilt && b.has_tilt,
        }
    }
}

/// Builds the dabs of one stroke from its samples.
#[derive(Clone, Debug)]
pub struct StrokeBuilder {
    pub settings: BrushSettings,
    pub erase: bool,
    last: Option<Point>,
    /// Distance along the path to the next dab.
    to_next: f64,
    pub dabs_made: usize,
    pub samples_used: usize,
}

impl StrokeBuilder {
    pub fn new(settings: BrushSettings, erase: bool) -> StrokeBuilder {
        StrokeBuilder { settings, erase, last: None, to_next: 0.0, dabs_made: 0, samples_used: 0 }
    }

    /// Diameter at a pressure.
    pub fn diameter(&self, pressure: f32) -> f32 {
        let s = &self.settings;
        if s.pressure_size { s.size * (s.min_size + (1.0 - s.min_size) * pressure.clamp(0.0, 1.0)) } else { s.size }
    }

    fn step(&self, pressure: f32) -> f64 {
        (self.settings.spacing.max(0.01) * self.diameter(pressure)).max(0.5) as f64
    }

    fn dab(&self, p: &Point) -> Dab {
        let s = &self.settings;
        let mut r = self.diameter(p.pressure) * 0.5;
        let mut stroke_alpha = s.opacity.clamp(0.0, 1.0);
        if s.pressure_opacity {
            stroke_alpha *= p.pressure.clamp(0.0, 1.0);
        }
        // Each point of the stroke's centre line is covered by about 1/spacing dabs; pick
        // the per-dab alpha so that they add up to the stroke opacity.
        let overlap = (1.0 / s.spacing.clamp(0.01, 1.0)).max(1.0);
        let mut a = 1.0 - (1.0 - stroke_alpha).powf(1.0 / overlap);
        if r < 0.5 {
            // Sub-pixel dabs: keep the size, fade by area so thin lines stay thin.
            a *= (r / 0.5).powi(2);
            r = 0.5;
        }
        let (mut rx, mut ry, mut angle) = (r, r, 0.0);
        if s.tilt_shape && p.has_tilt {
            let lean = (p.tilt_x.hypot(p.tilt_y) / 90.0).clamp(0.0, 1.0);
            rx = r * (1.0 + 2.0 * lean);
            ry = r * (1.0 - 0.3 * lean);
            // Screen Y points down and tilt Y is positive toward the user (down), so the
            // lean direction on screen is simply (tilt_x, tilt_y).
            angle = p.tilt_y.atan2(p.tilt_x);
        }
        let c = s.color;
        Dab {
            center: [p.x as f32, p.y as f32],
            radii: [rx, ry],
            angle,
            hardness: s.hardness.clamp(0.0, 1.0),
            _pad: [0.0; 2],
            color: [c[0] * a, c[1] * a, c[2] * a, a],
        }
    }

    /// Adds one sample; appends the dabs it produces to `out`.
    pub fn add(&mut self, sample: &PenSample, out: &mut Vec<Dab>) {
        self.add_point(Point::from(sample), out);
    }

    /// Adds a point that already went through the stabilizer.
    pub fn add_info(&mut self, info: &PaintInfo, has_tilt: bool, out: &mut Vec<Dab>) {
        self.add_point(Point::from_info(info, has_tilt), out);
    }

    fn add_point(&mut self, p: Point, out: &mut Vec<Dab>) {
        self.samples_used += 1;
        let Some(prev) = self.last else {
            out.push(self.dab(&p));
            self.dabs_made += 1;
            self.to_next = self.step(p.pressure);
            self.last = Some(p);
            return;
        };
        let len = (p.x - prev.x).hypot(p.y - prev.y);
        if len <= 0.0 {
            self.last = Some(Point { pressure: p.pressure, tilt_x: p.tilt_x, tilt_y: p.tilt_y, ..prev });
            return;
        }
        let mut d = self.to_next;
        while d <= len {
            let q = Point::lerp(&prev, &p, d / len);
            out.push(self.dab(&q));
            self.dabs_made += 1;
            d += self.step(q.pressure);
        }
        self.to_next = d - len;
        self.last = Some(p);
    }
}

/// A small dot marking one raw sample (the "show samples" overlay).
pub fn sample_dot(s: &PenSample) -> Dab {
    let (rgb, a) = if s.contact() { ([0.85, 0.1, 0.1], 0.95) } else { ([0.0, 0.55, 0.65], 0.6) };
    Dab {
        center: [s.x as f32, s.y as f32],
        radii: [1.4, 1.4],
        angle: 0.0,
        hardness: 0.6,
        _pad: [0.0; 2],
        color: [rgb[0] * a, rgb[1] * a, rgb[2] * a, a],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(x: f64, y: f64, p: f32) -> PenSample {
        PenSample::synthetic(0, x, y, p, true)
    }

    #[test]
    fn dabs_are_evenly_spaced_across_samples() {
        let settings = BrushSettings { size: 10.0, spacing: 0.1, pressure_size: false, ..Default::default() };
        let mut b = StrokeBuilder::new(settings, false);
        let mut out = Vec::new();
        // Samples at irregular distances along a line.
        for x in [0.0, 0.3, 2.5, 2.6, 7.0, 10.0] {
            b.add(&s(x, 5.0, 1.0), &mut out);
        }
        let xs: Vec<f32> = out.iter().map(|d| d.center[0]).collect();
        assert_eq!(xs.len(), 11, "{xs:?}");
        for w in xs.windows(2) {
            assert!((w[1] - w[0] - 1.0).abs() < 1e-4, "{xs:?}");
        }
    }

    #[test]
    fn no_gaps_on_a_fast_stroke() {
        // A fast stroke: samples 40 px apart, 10 px brush. Dabs must still be 1 px apart.
        let settings = BrushSettings { size: 10.0, spacing: 0.1, pressure_size: false, ..Default::default() };
        let mut b = StrokeBuilder::new(settings, false);
        let mut out = Vec::new();
        for i in 0..10 {
            b.add(&s(i as f64 * 40.0, 0.0, 1.0), &mut out);
        }
        for w in out.windows(2) {
            let gap = (w[1].center[0] - w[0].center[0]).abs();
            assert!(gap <= 1.0 + 1e-3, "gap {gap}");
        }
        assert_eq!(out.len(), 361);
    }

    #[test]
    fn pressure_controls_size() {
        let settings = BrushSettings { size: 40.0, min_size: 0.0, spacing: 0.5, ..Default::default() };
        let mut b = StrokeBuilder::new(settings, false);
        let mut out = Vec::new();
        b.add(&s(0.0, 0.0, 0.25), &mut out);
        assert_eq!(out[0].radii, [5.0, 5.0]);
        out.clear();
        let mut b = StrokeBuilder::new(settings, false);
        b.add(&s(0.0, 0.0, 1.0), &mut out);
        assert_eq!(out[0].radii, [20.0, 20.0]);
    }

    #[test]
    fn pressure_interpolates_between_samples() {
        let settings = BrushSettings { size: 20.0, min_size: 0.0, spacing: 0.05, ..Default::default() };
        let mut b = StrokeBuilder::new(settings, false);
        let mut out = Vec::new();
        b.add(&s(0.0, 0.0, 0.2), &mut out);
        b.add(&s(100.0, 0.0, 1.0), &mut out);
        // Radii grow monotonically along the segment.
        for w in out.windows(2) {
            assert!(w[1].radii[0] >= w[0].radii[0]);
        }
        assert!(out.last().unwrap().radii[0] > 9.0);
    }

    #[test]
    fn opacity_accumulates_to_the_stroke_opacity() {
        let settings = BrushSettings { opacity: 0.5, spacing: 0.1, ..Default::default() };
        let b = StrokeBuilder::new(settings, false);
        let a = b.dab(&Point { x: 0.0, y: 0.0, pressure: 1.0, tilt_x: 0.0, tilt_y: 0.0, has_tilt: false }).color[3];
        // Ten overlapping dabs of alpha a composite to 1 - (1 - a)^10.
        let total = 1.0 - (1.0 - a).powi(10);
        assert!((total - 0.5).abs() < 1e-4, "{total}");
    }

    #[test]
    fn tilt_stretches_the_dab_along_the_lean() {
        let settings = BrushSettings { tilt_shape: true, pressure_size: false, size: 10.0, ..Default::default() };
        let b = StrokeBuilder::new(settings, false);
        let mut p = s(0.0, 0.0, 1.0);
        p.flags.set(Flags::HAS_TILT, true);
        p.tilt_x = 0.0;
        p.tilt_y = 45.0;
        let d = b.dab(&Point::from(&p));
        assert!(d.radii[0] > d.radii[1]);
        assert!((d.angle - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
    }

    #[test]
    fn batches_split_on_blend_mode() {
        let mut v = Vec::new();
        let d = [Dab::default()];
        push_dabs(&mut v, false, &d);
        push_dabs(&mut v, false, &d);
        push_dabs(&mut v, true, &d);
        push_dabs(&mut v, false, &[]);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].dabs.len(), 2);
        assert!(v[1].erase);
    }
}
