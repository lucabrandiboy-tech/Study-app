//! The round pixel brush: brush settings, turning a stroke's points into evenly spaced
//! dabs, and painting them into a layer.
//!
//! Two painting modes, as in Krita:
//! - **Wash**: the stroke's coverage is collected in a mask and the stroke is drawn from
//!   the layer as it was before the stroke, so a stroke never gets darker than its opacity
//!   where it crosses itself.
//! - **Build up**: every dab is painted straight onto the layer, so overlaps darken.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::curve::Curve;
use crate::dab::{DabShape, coverage_tiles, paint_dab};
use crate::doc::{mul8, over};
use crate::stabilizer::PaintInfo;
use crate::tiles::{TILE, TILE_PIXELS, Tile, TileKey, TiledLayer};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaintMode {
    #[default]
    Wash,
    BuildUp,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Brush {
    /// Diameter in pixels at full size.
    pub size: f32,
    /// Stroke opacity 0..=1 (the most a stroke can cover, in Wash mode).
    pub opacity: f32,
    /// How much each dab adds, 0..=1.
    pub flow: f32,
    /// Edge hardness 0..=1.
    pub hardness: f32,
    /// Distance between dabs as a fraction of the diameter.
    pub spacing: f32,
    pub mode: PaintMode,
    /// Pressure to size (multiplier 0..=1); `None` = always full size.
    pub size_curve: Option<Curve>,
    /// Pressure to opacity; `None` = no pressure control.
    pub opacity_curve: Option<Curve>,
    /// Pressure to flow; `None` = no pressure control.
    pub flow_curve: Option<Curve>,
    /// Smallest diameter as a fraction of `size`, however light the pressure.
    pub min_size: f32,
    /// Lean the dab with the pen's tilt.
    pub tilt_shape: bool,
    /// Tip shape: 1 = round, smaller = flatter (a highlighter or calligraphy nib).
    pub roundness: f32,
    /// The flat tip's angle, in degrees (0 = flat side horizontal).
    pub angle: f32,
}

impl Default for Brush {
    fn default() -> Brush {
        Brush {
            size: 20.0,
            opacity: 1.0,
            flow: 1.0,
            hardness: 0.8,
            spacing: 0.1,
            mode: PaintMode::Wash,
            size_curve: Some(Curve::linear()),
            opacity_curve: None,
            flow_curve: None,
            min_size: 0.02,
            tilt_shape: false,
            roundness: 1.0,
            angle: 0.0,
        }
    }
}

impl Brush {
    pub fn diameter(&self, pressure: f32) -> f32 {
        let p = pressure.clamp(0.0, 1.0);
        match &self.size_curve {
            Some(c) => self.size * c.eval(p).max(self.min_size.clamp(0.0, 1.0)),
            None => self.size,
        }
    }

    fn opacity_at(&self, pressure: f32) -> f32 {
        self.opacity.clamp(0.0, 1.0) * self.opacity_curve.as_ref().map_or(1.0, |c| c.eval(pressure))
    }

    fn flow_at(&self, pressure: f32) -> f32 {
        self.flow.clamp(0.0, 1.0) * self.flow_curve.as_ref().map_or(1.0, |c| c.eval(pressure))
    }

    /// Distance from one dab to the next: `spacing` times the tip's width through its centre
    /// in the direction the stroke moves (`dir`, a unit vector), as Krita's anisotropic
    /// spacing does, so a flat tip leaves no gaps or stripes whichever way it moves. Before
    /// the direction is known (the first dab), the narrowest width.
    fn step(&self, pressure: f32, dir: Option<(f64, f64)>) -> f64 {
        let round = self.tip_roundness() as f64;
        let width = match dir {
            Some((ux, uy)) if round < 1.0 => {
                let (sin, cos) = (self.angle as f64).to_radians().sin_cos();
                let (along, across) = (ux * cos + uy * sin, uy * cos - ux * sin);
                1.0 / along.hypot(across / round)
            }
            _ => round,
        };
        // In f32 like the size itself, so a round tip's step is exactly what it always was.
        (self.spacing.clamp(0.01, 10.0) * self.diameter(pressure) * width as f32).max(0.5) as f64
    }

    /// The tip's roundness, kept to 5..100 %.
    fn tip_roundness(&self) -> f32 {
        if self.roundness.is_finite() { self.roundness.clamp(0.05, 1.0) } else { 1.0 }
    }
}

/// One dab with its strength.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushDab {
    pub shape: DabShape,
    /// Wash: the most this stroke may cover here. Build up: unused (1).
    pub opacity: f32,
    /// How much this dab adds.
    pub flow: f32,
}

/// Turns a stroke's points into dabs spaced evenly along the path.
#[derive(Clone, Debug)]
pub struct DabSpacer {
    last: Option<PaintInfo>,
    to_next: f64,
}

impl Default for DabSpacer {
    fn default() -> DabSpacer {
        DabSpacer { last: None, to_next: 0.0 }
    }
}

fn lerp(a: &PaintInfo, b: &PaintInfo, t: f64) -> PaintInfo {
    let tf = t as f32;
    PaintInfo {
        x: a.x + (b.x - a.x) * t,
        y: a.y + (b.y - a.y) * t,
        pressure: a.pressure + (b.pressure - a.pressure) * tf,
        tilt_x: a.tilt_x + (b.tilt_x - a.tilt_x) * tf,
        tilt_y: a.tilt_y + (b.tilt_y - a.tilt_y) * tf,
        rotation: a.rotation + (b.rotation - a.rotation) * tf,
    }
}

impl DabSpacer {
    fn dab(brush: &Brush, p: &PaintInfo) -> BrushDab {
        let r = brush.diameter(p.pressure) * 0.5;
        let (mut rx, mut ry, mut angle) = (r, r * brush.tip_roundness(), brush.angle.to_radians());
        let lean = (p.tilt_x.hypot(p.tilt_y) / 90.0).clamp(0.0, 1.0);
        if brush.tilt_shape && lean > 0.0 {
            rx = r * (1.0 + 2.0 * lean);
            ry = r * (1.0 - 0.3 * lean);
            angle = p.tilt_y.atan2(p.tilt_x);
        }
        let shape = DabShape { cx: p.x as f32, cy: p.y as f32, rx, ry, angle, hardness: brush.hardness.clamp(0.0, 1.0) };
        BrushDab { shape, opacity: brush.opacity_at(p.pressure), flow: brush.flow_at(p.pressure) }
    }

    /// Adds a point of the stroke and appends the dabs it produces.
    pub fn add(&mut self, brush: &Brush, p: PaintInfo, out: &mut Vec<BrushDab>) {
        if !p.x.is_finite() || !p.y.is_finite() {
            return;
        }
        let Some(prev) = self.last else {
            out.push(Self::dab(brush, &p));
            self.to_next = brush.step(p.pressure, None);
            self.last = Some(p);
            return;
        };
        let len = (p.x - prev.x).hypot(p.y - prev.y);
        if len <= 0.0 {
            self.last = Some(PaintInfo { x: prev.x, y: prev.y, ..p });
            return;
        }
        let dir = ((p.x - prev.x) / len, (p.y - prev.y) / len);
        let mut d = self.to_next;
        while d <= len {
            let q = lerp(&prev, &p, d / len);
            out.push(Self::dab(brush, &q));
            d += brush.step(q.pressure, Some(dir));
        }
        self.to_next = d - len;
        self.last = Some(p);
    }
}

/// Paints one stroke into a layer.
pub struct StrokePainter {
    pub brush: Brush,
    pub color: [u8; 3],
    pub erase: bool,
    /// One per copy of the stroke (symmetry); they share the coverage and the tiles below.
    spacers: Vec<DabSpacer>,
    /// Wash: the stroke's coverage so far, per tile (alpha only).
    mask: HashMap<TileKey, Box<[u8; TILE_PIXELS]>>,
    /// Wash: the layer's tiles from before the stroke (`None` = the tile did not exist).
    before: HashMap<TileKey, Option<std::sync::Arc<Tile>>>,
    dabs: Vec<BrushDab>,
    pub dabs_painted: usize,
}

impl StrokePainter {
    pub fn new(brush: Brush, color: [u8; 3], erase: bool) -> StrokePainter {
        StrokePainter::with_copies(brush, color, erase, 1)
    }

    /// The same stroke painted at `copies` places at once (symmetry). They are one stroke:
    /// where copies overlap, Wash still covers no more than the stroke's opacity, and no copy
    /// redraws a tile from before another copy painted it.
    pub fn with_copies(brush: Brush, color: [u8; 3], erase: bool, copies: usize) -> StrokePainter {
        let spacers = (0..copies.max(1)).map(|_| DabSpacer::default()).collect();
        StrokePainter { brush, color, erase, spacers, mask: HashMap::new(), before: HashMap::new(), dabs: Vec::new(), dabs_painted: 0 }
    }

    /// Adds a stroke point and paints the dabs it makes; changed tiles go into `dirty`.
    pub fn add(&mut self, layer: &mut TiledLayer, p: PaintInfo, dirty: &mut HashSet<TileKey>) {
        self.add_copies(layer, &[p], dirty);
    }

    /// Adds a point for each copy (`ps[i]` continues copy `i`).
    pub fn add_copies(&mut self, layer: &mut TiledLayer, ps: &[PaintInfo], dirty: &mut HashSet<TileKey>) {
        let mut dabs = std::mem::take(&mut self.dabs);
        dabs.clear();
        for (spacer, &p) in self.spacers.iter_mut().zip(ps) {
            spacer.add(&self.brush, p, &mut dabs);
        }
        for d in &dabs {
            self.paint(layer, d, dirty);
        }
        self.dabs_painted += dabs.len();
        self.dabs = dabs;
    }

    fn paint(&mut self, layer: &mut TiledLayer, d: &BrushDab, dirty: &mut HashSet<TileKey>) {
        match self.brush.mode {
            PaintMode::BuildUp => paint_dab(layer, &d.shape, self.color, d.opacity * d.flow, self.erase, dirty),
            PaintMode::Wash => self.paint_wash(layer, d, dirty),
        }
    }

    fn paint_wash(&mut self, layer: &mut TiledLayer, d: &BrushDab, dirty: &mut HashSet<TileKey>) {
        // 1. Coverage of this dab.
        let target = (d.opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
        let flow = (d.flow.clamp(0.0, 1.0) * 255.0).round() as u8;
        for (key, cov) in coverage_tiles(&d.shape, 1.0) {
            // 2. Krita's "alpha darken": coverage rises toward the stroke opacity, never past it.
            let m = self.mask.entry(key).or_insert_with(|| Box::new([0; TILE_PIXELS]));
            let (mut first, mut last) = (usize::MAX, 0);
            for (i, &c) in cov.iter().enumerate() {
                let a = mul8(c, flow);
                if a == 0 || m[i] >= target {
                    continue;
                }
                let next = (m[i] as i32 + ((target as i32 - m[i] as i32) * a as i32 + 127) / 255) as u8;
                if next != m[i] {
                    m[i] = next;
                    first = first.min(i);
                    last = i;
                }
            }
            if first == usize::MAX {
                continue;
            }
            // 3. The changed rows of the layer tile = its pre-stroke pixels with the stroke on top.
            let before = self.before.entry(key).or_insert_with(|| layer.tile(key).cloned());
            if self.erase && before.is_none() && layer.default[3] == 0 {
                continue;
            }
            let default_tile;
            let base: &Tile = match before {
                Some(t) => t,
                None => {
                    default_tile = Tile::filled(layer.default);
                    &default_tile
                }
            };
            let rows = (first / TILE as usize * TILE as usize)..((last / TILE as usize + 1) * TILE as usize);
            let out = layer.tile_mut(key);
            let (o_px, b_px) = (out.0.as_chunks_mut::<4>().0, base.0.as_chunks::<4>().0);
            for i in rows {
                let a = m[i];
                let mut p = b_px[i];
                if a != 0 {
                    if self.erase {
                        p[3] -= mul8(p[3], a);
                    } else {
                        over(&mut p, [self.color[0], self.color[1], self.color[2], a], 255);
                    }
                }
                o_px[i] = p;
            }
            dirty.insert(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f64, y: f64, pressure: f32) -> PaintInfo {
        PaintInfo { x, y, pressure, tilt_x: 0.0, tilt_y: 0.0, rotation: 0.0 }
    }

    fn stroke(painter: &mut StrokePainter, layer: &mut TiledLayer, pts: &[(f64, f64)]) -> HashSet<TileKey> {
        let mut dirty = HashSet::new();
        for &(x, y) in pts {
            painter.add(layer, pt(x, y, 1.0), &mut dirty);
        }
        dirty
    }

    #[test]
    fn dabs_are_evenly_spaced() {
        let brush = Brush { size: 10.0, spacing: 0.1, size_curve: None, ..Default::default() };
        let mut s = DabSpacer::default();
        let mut out = Vec::new();
        for x in [0.0, 0.3, 2.5, 2.6, 7.0, 10.0] {
            s.add(&brush, pt(x, 5.0, 1.0), &mut out);
        }
        let xs: Vec<f32> = out.iter().map(|d| d.shape.cx).collect();
        assert_eq!(xs.len(), 11, "{xs:?}");
        for w in xs.windows(2) {
            assert!((w[1] - w[0] - 1.0).abs() < 1e-4, "{xs:?}");
        }
    }

    #[test]
    fn flat_tips_leave_no_gaps_in_any_direction() {
        // A very flat tip (5 % round), its long side horizontal: going down moves it along its
        // thin side, where dabs spaced by the full width would leave a comb of lines.
        let brush = Brush { size: 40.0, hardness: 1.0, roundness: 0.05, angle: 0.0, size_curve: None, ..Default::default() };
        for (dx, dy) in [(0.0, 1.0), (1.0, 0.0), (0.6, 0.8), (-0.8, 0.6)] {
            let mut layer = TiledLayer::new();
            let mut p = StrokePainter::new(brush.clone(), [0, 0, 0], false);
            let (x0, y0) = (300.5, 300.5);
            stroke(&mut p, &mut layer, &[(x0, y0), (x0 + 200.0 * dx, y0 + 200.0 * dy)]);
            for i in 0..=400 {
                let t = i as f64 / 2.0;
                let (x, y) = (x0 + t * dx, y0 + t * dy);
                assert_eq!(layer.pixel(x.floor() as i32, y.floor() as i32)[3], 255, "gap at {t} px going ({dx}, {dy})");
            }
        }
        // Spaced along the stroke by the tip's width in that direction: few dabs along the
        // long side, many along the thin side.
        let count = |dx: f64, dy: f64| {
            let mut s = DabSpacer::default();
            let mut out = Vec::new();
            s.add(&brush, pt(0.0, 0.0, 1.0), &mut out);
            s.add(&brush, pt(100.0 * dx, 100.0 * dy, 1.0), &mut out);
            out.len()
        };
        assert_eq!(count(1.0, 0.0), 26, "4 px apart along the long side");
        assert!((200..=201).contains(&count(0.0, 1.0)), "0.2 px along the thin side, at least 0.5 px: {}", count(0.0, 1.0));
        // A round tip is spaced as before, whatever the direction.
        let round = Brush { roundness: 1.0, ..brush.clone() };
        let mut s = DabSpacer::default();
        let mut out = Vec::new();
        s.add(&round, pt(0.0, 0.0, 1.0), &mut out);
        s.add(&round, pt(60.0, 80.0, 1.0), &mut out);
        assert_eq!(out.len(), 26);
    }

    #[test]
    fn pressure_curves() {
        let brush = Brush { size: 40.0, min_size: 0.1, opacity_curve: Some(Curve::linear()), ..Default::default() };
        assert_eq!(brush.diameter(1.0), 40.0);
        assert_eq!(brush.diameter(0.5), 20.0);
        assert_eq!(brush.diameter(0.0), 4.0, "min size");
        assert_eq!(brush.opacity_at(0.25), 0.25);
        assert_eq!(brush.flow_at(0.25), 1.0, "no flow curve");
        let soft = Brush { size_curve: Some(Curve::from_points(&[(0.0, 0.0), (0.5, 0.1), (1.0, 1.0)])), min_size: 0.0, ..brush };
        assert!((soft.diameter(0.5) - 4.0).abs() < 1e-4);
    }

    #[test]
    fn wash_stroke_never_darker_than_its_opacity() {
        let brush = Brush { size: 16.0, opacity: 0.5, hardness: 1.0, size_curve: None, ..Default::default() };
        let mut layer = TiledLayer::filled([255, 255, 255, 255]);
        let mut p = StrokePainter::new(brush, [0, 0, 0], false);
        // A stroke that crosses itself several times.
        stroke(&mut p, &mut layer, &[(10.0, 50.0), (90.0, 50.0), (50.0, 10.0), (50.0, 90.0), (10.0, 50.0), (90.0, 50.0)]);
        assert_eq!(layer.pixel(50, 50), [127, 127, 127, 255], "crossing point");
        assert_eq!(layer.pixel(20, 50), [127, 127, 127, 255]);
        let darkest = (0..100).flat_map(|y| (0..100).map(move |x| (x, y))).map(|(x, y)| layer.pixel(x, y)[0]).min().unwrap();
        assert_eq!(darkest, 127);
    }

    #[test]
    fn build_up_darkens_where_it_overlaps() {
        let brush = Brush { size: 16.0, opacity: 0.5, hardness: 1.0, size_curve: None, mode: PaintMode::BuildUp, ..Default::default() };
        let mut layer = TiledLayer::filled([255, 255, 255, 255]);
        let mut p = StrokePainter::new(brush, [0, 0, 0], false);
        stroke(&mut p, &mut layer, &[(10.0, 50.0), (90.0, 50.0)]);
        assert!(layer.pixel(50, 50)[0] < 20, "many overlapping half-strength dabs: {:?}", layer.pixel(50, 50));
    }

    #[test]
    fn wash_with_low_flow_builds_toward_opacity() {
        let brush = Brush { size: 16.0, opacity: 1.0, flow: 0.1, hardness: 1.0, spacing: 0.05, size_curve: None, ..Default::default() };
        let mut layer = TiledLayer::new();
        let mut p = StrokePainter::new(brush, [255, 0, 0], false);
        stroke(&mut p, &mut layer, &[(10.0, 40.0), (60.0, 40.0)]);
        let a = layer.pixel(35, 40)[3];
        assert!(a > 200, "many low-flow dabs approach full opacity: {a}");
        // One dab's worth at the very start of the stroke edge is faint.
        let mut layer2 = TiledLayer::new();
        let mut p2 = StrokePainter::new(p.brush.clone(), [255, 0, 0], false);
        stroke(&mut p2, &mut layer2, &[(10.0, 40.0)]);
        assert!((20..=30).contains(&layer2.pixel(10, 40)[3]), "{:?}", layer2.pixel(10, 40));
    }

    #[test]
    fn wash_erase_and_existing_pixels() {
        let mut layer = TiledLayer::new();
        for y in 0..64 {
            for x in 0..64 {
                layer.set_pixel(x, y, [0, 0, 255, 255]);
            }
        }
        let brush = Brush { size: 10.0, opacity: 0.5, hardness: 1.0, size_curve: None, ..Default::default() };
        let mut p = StrokePainter::new(brush.clone(), [0, 0, 0], true);
        let dirty = stroke(&mut p, &mut layer, &[(5.0, 20.0), (60.0, 20.0), (5.0, 20.0)]);
        assert_eq!(layer.pixel(30, 20)[3], 127, "half erased, not more, despite going over twice");
        assert!(dirty.contains(&(0, 0)));
        // Erasing where there is nothing creates nothing.
        let mut empty = TiledLayer::new();
        let mut p = StrokePainter::new(brush, [0, 0, 0], true);
        let dirty = stroke(&mut p, &mut empty, &[(5.0, 20.0), (60.0, 20.0)]);
        assert_eq!(empty.tile_count(), 0);
        assert!(dirty.is_empty());
    }

    #[test]
    fn tilt_leans_the_dab() {
        let brush = Brush { tilt_shape: true, size_curve: None, ..Default::default() };
        let d = DabSpacer::dab(&brush, &PaintInfo { tilt_x: 45.0, tilt_y: 0.0, ..pt(0.0, 0.0, 1.0) });
        assert!(d.shape.rx > d.shape.ry);
        assert!(d.shape.angle.abs() < 1e-6);
        let flat = DabSpacer::dab(&Brush { tilt_shape: false, ..brush }, &PaintInfo { tilt_x: 45.0, ..pt(0.0, 0.0, 1.0) });
        assert_eq!(flat.shape.rx, flat.shape.ry);
    }
}
