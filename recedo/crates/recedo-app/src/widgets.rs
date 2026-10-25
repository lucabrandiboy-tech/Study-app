//! Small custom widgets: the colour selector and painted icons (egui's fonts have no symbol
//! glyphs, so icons are drawn).

use egui::{Color32, Pos2, Response, Sense, Shape, Stroke, Ui, pos2, vec2};

/// Hue, saturation, value, each 0..=1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsv(pub f32, pub f32, pub f32);

pub fn hsv_to_rgb(Hsv(h, s, v): Hsv) -> [u8; 3] {
    let h = (h.rem_euclid(1.0)) * 6.0;
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    let q = |f: f32| ((f + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    [q(r), q(g), q(b)]
}

/// RGB to HSV; `keep_hue` is used when the colour is grey (its hue is undefined).
pub fn rgb_to_hsv(rgb: [u8; 3], keep_hue: f32) -> Hsv {
    let [r, g, b] = rgb.map(|c| c as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0.0 {
        keep_hue
    } else if max == r {
        ((g - b) / d).rem_euclid(6.0) / 6.0
    } else if max == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    Hsv(h, if max == 0.0 { 0.0 } else { d / max }, max)
}

fn c32(rgb: [u8; 3]) -> Color32 {
    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

/// A saturation/value square over a hue bar. `hsv` keeps the hue while the colour is grey.
/// Returns true when the user changed the colour.
pub fn color_selector(ui: &mut Ui, rgb: &mut [u8; 3], hsv: &mut Hsv) -> bool {
    if hsv_to_rgb(*hsv) != *rgb {
        // Changed elsewhere (eyedropper, hex field).
        *hsv = rgb_to_hsv(*rgb, hsv.0);
    }
    let side = ui.available_width().clamp(120.0, 220.0);
    let mut changed = false;
    // Saturation (left to right) and value (bottom to top).
    let (rect, resp) = ui.allocate_exact_size(vec2(side, (side * 0.6).max(90.0)), Sense::click_and_drag());
    if let Some(p) = resp.interact_pointer_pos() {
        hsv.1 = ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        hsv.2 = 1.0 - ((p.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
        changed = true;
    }
    let n = 24;
    let mut mesh = egui::Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            let (s, v) = (i as f32 / n as f32, 1.0 - j as f32 / n as f32);
            mesh.colored_vertex(pos2(rect.left() + s * rect.width(), rect.top() + (1.0 - v) * rect.height()), c32(hsv_to_rgb(Hsv(hsv.0, s, v))));
        }
    }
    for j in 0..n {
        for i in 0..n {
            let a = (j * (n + 1) + i) as u32;
            let b = a + 1;
            let c = a + (n + 1) as u32;
            let d = c + 1;
            mesh.add_triangle(a, b, c);
            mesh.add_triangle(b, d, c);
        }
    }
    ui.painter().add(Shape::mesh(mesh));
    let marker = pos2(rect.left() + hsv.1 * rect.width(), rect.top() + (1.0 - hsv.2) * rect.height());
    let ring = if hsv.2 > 0.5 { Color32::BLACK } else { Color32::WHITE };
    ui.painter().circle_stroke(marker, 5.0, Stroke::new(1.5, ring));
    // Hue bar.
    let (bar, bresp) = ui.allocate_exact_size(vec2(side, 16.0), Sense::click_and_drag());
    if let Some(p) = bresp.interact_pointer_pos() {
        hsv.0 = ((p.x - bar.left()) / bar.width()).clamp(0.0, 0.9999);
        changed = true;
    }
    let mut mesh = egui::Mesh::default();
    let steps = 36;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let col = c32(hsv_to_rgb(Hsv(t, 1.0, 1.0)));
        mesh.colored_vertex(pos2(bar.left() + t * bar.width(), bar.top()), col);
        mesh.colored_vertex(pos2(bar.left() + t * bar.width(), bar.bottom()), col);
    }
    for i in 0..steps as u32 {
        let a = i * 2;
        mesh.add_triangle(a, a + 1, a + 2);
        mesh.add_triangle(a + 1, a + 3, a + 2);
    }
    ui.painter().add(Shape::mesh(mesh));
    let x = bar.left() + hsv.0 * bar.width();
    ui.painter().line_segment([pos2(x, bar.top() - 2.0), pos2(x, bar.bottom() + 2.0)], Stroke::new(2.0, Color32::WHITE));
    ui.painter().line_segment([pos2(x + 1.5, bar.top()), pos2(x + 1.5, bar.bottom())], Stroke::new(1.0, Color32::BLACK));
    if changed {
        *rgb = hsv_to_rgb(*hsv);
    }
    changed
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Eye,
    Lock,
}

/// A clickable painted icon, drawn bright when `on`; crossed out (eye) or open (lock) when off.
pub fn icon_toggle(ui: &mut Ui, on: bool, icon: Icon) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(20.0, 18.0), Sense::click());
    let visuals = ui.style().interact(&resp);
    let col = if on { visuals.fg_stroke.color } else { ui.visuals().weak_text_color() };
    let s = Stroke::new(1.4, col);
    let c = rect.center();
    let p = ui.painter();
    match icon {
        Icon::Eye => {
            // Almond outline from two arcs, and the pupil.
            let arc = |up: bool| -> Vec<Pos2> {
                (0..=12)
                    .map(|i| {
                        let t = i as f32 / 12.0;
                        let x = c.x - 7.0 + 14.0 * t;
                        let y = 4.5 * (std::f32::consts::PI * t).sin();
                        pos2(x, if up { c.y - y } else { c.y + y })
                    })
                    .collect()
            };
            p.add(Shape::line(arc(true), s));
            p.add(Shape::line(arc(false), s));
            if on {
                p.circle_filled(c, 2.2, col);
            } else {
                p.line_segment([c + vec2(-6.0, 6.0), c + vec2(6.0, -6.0)], s);
            }
        }
        Icon::Lock => {
            let body = egui::Rect::from_center_size(c + vec2(0.0, 2.5), vec2(10.0, 7.0));
            p.rect_stroke(body, 1.0, s, egui::StrokeKind::Inside);
            if on {
                p.rect_filled(body.shrink(2.0), 0.0, col);
            }
            // Shackle: a half ring, lifted to one side when unlocked.
            let lift = if on { 0.0 } else { -2.5 };
            let pts: Vec<Pos2> = (0..=10)
                .map(|i| {
                    let a = std::f32::consts::PI * i as f32 / 10.0;
                    pos2(c.x - 3.0 * a.cos(), body.top() + lift - 4.0 * a.sin())
                })
                .collect();
            p.add(Shape::line(pts, s));
        }
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsv_roundtrip() {
        for rgb in [[0, 0, 0], [255, 255, 255], [30, 60, 160], [255, 0, 0], [12, 250, 99], [128, 128, 128]] {
            let hsv = rgb_to_hsv(rgb, 0.3);
            assert_eq!(hsv_to_rgb(hsv), rgb, "{rgb:?} -> {hsv:?}");
        }
        // Grey keeps the hue it is given.
        assert_eq!(rgb_to_hsv([90, 90, 90], 0.7).0, 0.7);
        assert_eq!(hsv_to_rgb(Hsv(0.0, 1.0, 1.0)), [255, 0, 0]);
        assert_eq!(hsv_to_rgb(Hsv(1.0 / 3.0, 1.0, 1.0)), [0, 255, 0]);
    }
}
