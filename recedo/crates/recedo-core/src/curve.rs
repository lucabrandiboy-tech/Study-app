//! Sensor curves (pressure to size, pressure to opacity...), like Krita's curve editor: a
//! few control points in the unit square joined by a smooth curve that never overshoots.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Curve {
    /// Control points (input, output), both 0..=1, sorted by input. Always includes
    /// input 0 and input 1.
    points: Vec<(f32, f32)>,
}

impl Default for Curve {
    fn default() -> Curve {
        Curve::linear()
    }
}

impl Curve {
    pub fn linear() -> Curve {
        Curve { points: vec![(0.0, 0.0), (1.0, 1.0)] }
    }

    /// Builds a curve from any points: clamped to the unit square, sorted, duplicates
    /// removed, and end points added if missing (continuing the nearest point flat).
    pub fn from_points(points: &[(f32, f32)]) -> Curve {
        let mut p: Vec<(f32, f32)> = points.iter().filter(|(x, y)| x.is_finite() && y.is_finite()).map(|&(x, y)| (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0))).collect();
        p.sort_by(|a, b| a.0.total_cmp(&b.0));
        p.dedup_by(|b, a| (b.0 - a.0).abs() < 1e-4);
        if p.is_empty() {
            return Curve::linear();
        }
        if p[0].0 > 0.0 {
            p.insert(0, (0.0, p[0].1));
        }
        let last = *p.last().expect("not empty");
        if last.0 < 1.0 {
            p.push((1.0, last.1));
        }
        Curve { points: p }
    }

    pub fn points(&self) -> &[(f32, f32)] {
        &self.points
    }

    /// The curve's value at `x` (clamped to 0..=1). Monotone cubic (Fritsch-Carlson)
    /// interpolation: smooth, and between two points it stays between their values.
    pub fn eval(&self, x: f32) -> f32 {
        let p = &self.points;
        let x = if x.is_nan() { 0.0 } else { x.clamp(0.0, 1.0) };
        if p.len() == 2 {
            let ((x0, y0), (x1, y1)) = (p[0], p[1]);
            return y0 + (y1 - y0) * (x - x0) / (x1 - x0);
        }
        let i = p.partition_point(|q| q.0 <= x).clamp(1, p.len() - 1) - 1;
        let ((x0, y0), (x1, y1)) = (p[i], p[i + 1]);
        let h = x1 - x0;
        let t = (x - x0) / h;
        let (m0, m1) = (self.tangent(i), self.tangent(i + 1));
        let (t2, t3) = (t * t, t * t * t);
        let y = (2.0 * t3 - 3.0 * t2 + 1.0) * y0 + (t3 - 2.0 * t2 + t) * h * m0 + (-2.0 * t3 + 3.0 * t2) * y1 + (t3 - t2) * h * m1;
        y.clamp(y0.min(y1), y0.max(y1))
    }

    /// Fritsch-Carlson tangent at point `i`.
    fn tangent(&self, i: usize) -> f32 {
        let p = &self.points;
        let slope = |a: usize| (p[a + 1].1 - p[a].1) / (p[a + 1].0 - p[a].0);
        if i == 0 {
            return slope(0);
        }
        if i == p.len() - 1 {
            return slope(i - 1);
        }
        let (s0, s1) = (slope(i - 1), slope(i));
        if s0 * s1 <= 0.0 {
            return 0.0;
        }
        // Weighted harmonic mean keeps the curve monotone.
        let (h0, h1) = (p[i].0 - p[i - 1].0, p[i + 1].0 - p[i].0);
        let (w0, w1) = (2.0 * h1 + h0, h1 + 2.0 * h0);
        (w0 + w1) / (w0 / s0 + w1 / s1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_is_identity() {
        let c = Curve::linear();
        for i in 0..=10 {
            let x = i as f32 / 10.0;
            assert!((c.eval(x) - x).abs() < 1e-6);
        }
        assert_eq!(c.eval(-3.0), 0.0);
        assert_eq!(c.eval(7.0), 1.0);
        assert_eq!(c.eval(f32::NAN), 0.0);
    }

    #[test]
    fn passes_through_points_and_stays_monotone() {
        // A typical "soft start" pressure curve.
        let c = Curve::from_points(&[(0.0, 0.0), (0.3, 0.1), (0.7, 0.6), (1.0, 1.0)]);
        for &(x, y) in c.points() {
            assert!((c.eval(x) - y).abs() < 1e-5, "{x} -> {} not {y}", c.eval(x));
        }
        let mut prev = -1.0;
        for i in 0..=1000 {
            let y = c.eval(i as f32 / 1000.0);
            assert!(y >= prev - 1e-6, "not monotone at {i}");
            assert!((0.0..=1.0).contains(&y));
            prev = y;
        }
    }

    #[test]
    fn flat_parts_do_not_overshoot() {
        let c = Curve::from_points(&[(0.0, 0.0), (0.4, 0.8), (0.6, 0.8), (1.0, 1.0)]);
        for i in 400..=600 {
            let y = c.eval(i as f32 / 1000.0);
            assert!((y - 0.8).abs() < 1e-5, "{y} at {i}");
        }
    }

    #[test]
    fn messy_input_is_cleaned() {
        let c = Curve::from_points(&[(0.5, 2.0), (f32::NAN, 0.1), (0.5, 0.3), (-1.0, 0.2)]);
        assert_eq!(c.points(), &[(0.0, 0.2), (0.5, 1.0), (1.0, 1.0)]);
        assert_eq!(Curve::from_points(&[]), Curve::linear());
        // Single point: flat.
        let flat = Curve::from_points(&[(0.5, 0.25)]);
        assert_eq!(flat.eval(0.9), 0.25);
    }
}
