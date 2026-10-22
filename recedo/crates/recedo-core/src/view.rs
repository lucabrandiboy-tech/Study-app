//! Where the canvas sits on screen: pan, zoom, rotation and horizontal mirror, and the
//! conversions between screen pixels and canvas pixels.
//!
//! screen = offset + rotate(angle) * zoom * mirror * canvas

use serde::{Deserialize, Serialize};

pub const MIN_ZOOM: f64 = 0.01;
pub const MAX_ZOOM: f64 = 64.0;

/// Zoom steps for the zoom keys and the zoom menu (Krita's list, 1% to 6400%).
pub const ZOOM_STEPS: [f64; 26] = [
    0.01, 0.02, 0.03, 0.04, 0.0625, 0.0833, 0.125, 0.1667, 0.25, 0.3333, 0.5, 0.6667, 1.0, 1.3333, 2.0, 3.0, 4.0, 6.0, 8.0, 12.0, 16.0, 24.0, 32.0, 40.0, 48.0, 64.0,
];

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct View {
    /// Screen position of canvas pixel (0, 0)'s corner.
    pub offset: (f64, f64),
    /// Screen pixels per canvas pixel.
    pub zoom: f64,
    /// Rotation in radians, clockwise on screen (screen y points down).
    pub angle: f64,
    /// Mirrored left-right.
    pub mirror: bool,
}

impl Default for View {
    fn default() -> View {
        View { offset: (0.0, 0.0), zoom: 1.0, angle: 0.0, mirror: false }
    }
}

impl View {
    fn axes(&self) -> ((f64, f64), (f64, f64)) {
        let (s, c) = self.angle.sin_cos();
        let m = if self.mirror { -1.0 } else { 1.0 };
        // Images of the canvas x and y unit vectors on screen.
        ((c * self.zoom * m, s * self.zoom * m), (-s * self.zoom, c * self.zoom))
    }

    pub fn to_screen(&self, cx: f64, cy: f64) -> (f64, f64) {
        let (ax, ay) = self.axes();
        (self.offset.0 + ax.0 * cx + ay.0 * cy, self.offset.1 + ax.1 * cx + ay.1 * cy)
    }

    pub fn to_canvas(&self, sx: f64, sy: f64) -> (f64, f64) {
        let (dx, dy) = (sx - self.offset.0, sy - self.offset.1);
        let (s, c) = self.angle.sin_cos();
        // Undo the rotation, then the zoom and the mirror.
        let (ux, uy) = (c * dx + s * dy, -s * dx + c * dy);
        let m = if self.mirror { -1.0 } else { 1.0 };
        (ux / self.zoom * m, uy / self.zoom)
    }

    /// The 2x3 matrix (row-major: [a, b, tx, c, d, ty]) taking canvas to screen pixels.
    pub fn matrix(&self) -> [f64; 6] {
        let (ax, ay) = self.axes();
        [ax.0, ay.0, self.offset.0, ax.1, ay.1, self.offset.1]
    }

    /// Moves the view so canvas point `c` lands on screen point `s`.
    fn pin(&mut self, c: (f64, f64), s: (f64, f64)) {
        let now = self.to_screen(c.0, c.1);
        self.offset.0 += s.0 - now.0;
        self.offset.1 += s.1 - now.1;
    }

    pub fn pan(&mut self, dx: f64, dy: f64) {
        self.offset.0 += dx;
        self.offset.1 += dy;
    }

    /// Sets the zoom, keeping the canvas point under screen point `at` in place.
    pub fn set_zoom_at(&mut self, zoom: f64, at: (f64, f64)) {
        if !zoom.is_finite() {
            return;
        }
        let c = self.to_canvas(at.0, at.1);
        self.zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        self.pin(c, at);
    }

    /// Multiplies the zoom (pinch, scroll, drag-zoom) around a screen point.
    pub fn zoom_by(&mut self, factor: f64, at: (f64, f64)) {
        if factor.is_finite() && factor > 0.0 {
            self.set_zoom_at(self.zoom * factor, at);
        }
    }

    /// Next zoom step in or out from the current zoom.
    pub fn step_zoom(&mut self, zoom_in: bool, at: (f64, f64)) {
        let z = self.zoom;
        let next = if zoom_in { ZOOM_STEPS.iter().copied().find(|&s| s > z * 1.001) } else { ZOOM_STEPS.iter().rev().copied().find(|&s| s < z / 1.001) };
        if let Some(n) = next {
            self.set_zoom_at(n, at);
        }
    }

    /// Rotates by `delta` radians around a screen point.
    pub fn rotate_by(&mut self, delta: f64, at: (f64, f64)) {
        if !delta.is_finite() {
            return;
        }
        let c = self.to_canvas(at.0, at.1);
        self.angle = (self.angle + delta).rem_euclid(std::f64::consts::TAU);
        self.pin(c, at);
    }

    /// Sets the rotation (e.g. reset to 0) around a screen point.
    pub fn set_angle_at(&mut self, angle: f64, at: (f64, f64)) {
        self.rotate_by(angle - self.angle, at);
    }

    /// Mirrors left-right around a screen point.
    pub fn toggle_mirror(&mut self, at: (f64, f64)) {
        let c = self.to_canvas(at.0, at.1);
        self.mirror = !self.mirror;
        self.pin(c, at);
    }

    /// Zooms so the whole canvas fits the viewport with `margin` pixels around it, centred,
    /// keeping the rotation and mirror.
    pub fn fit(&mut self, canvas: (u32, u32), viewport: (f64, f64), margin: f64) {
        let (cw, ch) = (canvas.0.max(1) as f64, canvas.1.max(1) as f64);
        let (s, c) = self.angle.sin_cos();
        // Size of the rotated canvas at zoom 1.
        let (bw, bh) = (cw * c.abs() + ch * s.abs(), cw * s.abs() + ch * c.abs());
        let zw = (viewport.0 - 2.0 * margin).max(1.0) / bw;
        let zh = (viewport.1 - 2.0 * margin).max(1.0) / bh;
        self.zoom = zw.min(zh).clamp(MIN_ZOOM, MAX_ZOOM);
        self.pin((cw / 2.0, ch / 2.0), (viewport.0 / 2.0, viewport.1 / 2.0));
    }

    /// The canvas area visible in a viewport, as a canvas-space bounding box
    /// (x0, y0, x1, y1), for drawing only the tiles on screen.
    pub fn visible_canvas_rect(&self, viewport: (f64, f64)) -> (f64, f64, f64, f64) {
        let corners = [(0.0, 0.0), (viewport.0, 0.0), (0.0, viewport.1), viewport];
        let pts = corners.map(|(x, y)| self.to_canvas(x, y));
        let xs = pts.map(|p| p.0);
        let ys = pts.map(|p| p.1);
        let min = |v: [f64; 4]| v.into_iter().fold(f64::INFINITY, f64::min);
        let max = |v: [f64; 4]| v.into_iter().fold(f64::NEG_INFINITY, f64::max);
        (min(xs), min(ys), max(xs), max(ys))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: (f64, f64), b: (f64, f64)) -> bool {
        (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6
    }

    fn views() -> Vec<View> {
        let mut v = Vec::new();
        for &zoom in &[0.25, 1.0, 3.7] {
            for &angle in &[0.0, 0.5, 2.0, 4.5] {
                for mirror in [false, true] {
                    v.push(View { offset: (123.0, -45.0), zoom, angle, mirror });
                }
            }
        }
        v
    }

    #[test]
    fn screen_and_canvas_roundtrip() {
        for v in views() {
            for &p in &[(0.0, 0.0), (10.0, 20.0), (-300.5, 1234.25)] {
                let s = v.to_screen(p.0, p.1);
                assert!(close(v.to_canvas(s.0, s.1), p), "{v:?} {p:?}");
            }
            // The matrix agrees with to_screen.
            let m = v.matrix();
            let s = v.to_screen(7.0, -3.0);
            assert!(close((m[0] * 7.0 + m[1] * -3.0 + m[2], m[3] * 7.0 + m[4] * -3.0 + m[5]), s));
        }
    }

    #[test]
    fn basics() {
        let v = View { offset: (100.0, 50.0), zoom: 2.0, angle: 0.0, mirror: false };
        assert!(close(v.to_screen(10.0, 10.0), (120.0, 70.0)));
        // 90 degrees clockwise: canvas +x points down the screen.
        let r = View { angle: std::f64::consts::FRAC_PI_2, ..v };
        assert!(close(r.to_screen(10.0, 0.0), (100.0, 70.0)));
        // Mirror: canvas +x points left.
        let m = View { mirror: true, ..v };
        assert!(close(m.to_screen(10.0, 0.0), (80.0, 50.0)));
    }

    #[test]
    fn zoom_rotate_and_mirror_keep_the_point_under_the_cursor() {
        for mut v in views() {
            let at = (400.0, 300.0);
            let c = v.to_canvas(at.0, at.1);
            v.zoom_by(1.7, at);
            assert!(close(v.to_canvas(at.0, at.1), c));
            v.rotate_by(0.3, at);
            assert!(close(v.to_canvas(at.0, at.1), c));
            v.toggle_mirror(at);
            assert!(close(v.to_canvas(at.0, at.1), c));
            v.step_zoom(true, at);
            assert!(close(v.to_canvas(at.0, at.1), c));
            v.set_angle_at(0.0, at);
            assert!(v.angle.abs() < 1e-9);
            assert!(close(v.to_canvas(at.0, at.1), c));
        }
    }

    #[test]
    fn zoom_steps_and_limits() {
        let mut v = View::default();
        v.step_zoom(true, (0.0, 0.0));
        assert_eq!(v.zoom, 1.3333);
        v.step_zoom(false, (0.0, 0.0));
        assert_eq!(v.zoom, 1.0);
        v.step_zoom(false, (0.0, 0.0));
        assert_eq!(v.zoom, 0.6667);
        v.zoom_by(1e9, (0.0, 0.0));
        assert_eq!(v.zoom, MAX_ZOOM);
        v.step_zoom(true, (0.0, 0.0));
        assert_eq!(v.zoom, MAX_ZOOM);
        v.zoom_by(0.0, (0.0, 0.0));
        v.zoom_by(f64::NAN, (0.0, 0.0));
        assert_eq!(v.zoom, MAX_ZOOM);
        v.zoom_by(1e-12, (0.0, 0.0));
        assert_eq!(v.zoom, MIN_ZOOM);
    }

    #[test]
    fn fit_centres_the_canvas() {
        let mut v = View::default();
        v.fit((3840, 2160), (1600.0, 1000.0), 20.0);
        assert!((v.zoom - 1560.0 / 3840.0).abs() < 1e-9);
        assert!(close(v.to_screen(1920.0, 1080.0), (800.0, 500.0)));
        // Rotated 90 degrees the tall side limits the zoom.
        let mut r = View { angle: std::f64::consts::FRAC_PI_2, ..View::default() };
        r.fit((3840, 2160), (1600.0, 1000.0), 20.0);
        assert!((r.zoom - 960.0 / 3840.0).abs() < 1e-9);
        let (x0, y0, x1, y1) = r.visible_canvas_rect((1600.0, 1000.0));
        assert!(x0 < 0.0 && y0 < 0.0 && x1 > 3840.0 && y1 > 2160.0, "whole canvas visible");
    }
}
