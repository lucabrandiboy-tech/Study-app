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

/// Which part of the colour wheel a drag started on: it stays with that part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WheelPart {
    Ring,
    Square,
    Neither,
}

/// Clip Studio's colour wheel: a hue ring around a saturation/value square. Red is at the
/// top and the hues go round clockwise. `hsv` keeps the hue while the colour is grey.
/// Returns true when the user changed the colour.
pub fn color_wheel(ui: &mut Ui, rgb: &mut [u8; 3], hsv: &mut Hsv) -> bool {
    if hsv_to_rgb(*hsv) != *rgb {
        // Changed elsewhere (eyedropper, hex field, swap).
        *hsv = rgb_to_hsv(*rgb, hsv.0);
    }
    // As wide as the panel, but short enough that the swatches under it show too (the
    // panel's visible height, which a scroll area does not limit).
    let side = ui.available_width().min(ui.clip_rect().height() - 56.0).clamp(110.0, 220.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(side, side), Sense::click_and_drag());
    let c = rect.center();
    let outer = side / 2.0 - 1.0;
    let ring = (side * 0.09).clamp(10.0, 18.0);
    let inner = outer - ring;
    let half = inner / std::f32::consts::SQRT_2 - 4.0;
    let square = egui::Rect::from_center_size(c, vec2(half * 2.0, half * 2.0));
    let hue_angle = |h: f32| h * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
    let mut changed = false;
    let part_id = resp.id.with("part");
    if let Some(p) = resp.interact_pointer_pos() {
        let start = ui.input(|i| i.pointer.press_origin()).unwrap_or(p);
        let part = ui.data_mut(|d| {
            *d.get_temp_mut_or_insert_with(part_id, || {
                let r = (start - c).length();
                if square.expand(2.0).contains(start) {
                    WheelPart::Square
                } else if r >= inner - 3.0 && r <= outer + 3.0 {
                    WheelPart::Ring
                } else {
                    WheelPart::Neither
                }
            })
        });
        match part {
            WheelPart::Ring => {
                let d = p - c;
                hsv.0 = ((d.y.atan2(d.x) + std::f32::consts::FRAC_PI_2) / std::f32::consts::TAU).rem_euclid(1.0).min(0.9999);
                changed = true;
            }
            WheelPart::Square => {
                hsv.1 = ((p.x - square.left()) / square.width()).clamp(0.0, 1.0);
                hsv.2 = 1.0 - ((p.y - square.top()) / square.height()).clamp(0.0, 1.0);
                changed = true;
            }
            WheelPart::Neither => {}
        }
    } else {
        ui.data_mut(|d| d.remove::<WheelPart>(part_id));
    }
    let painter = ui.painter();
    // The ring: a band of quads, each edge in its hue.
    let steps = 90;
    let mut mesh = egui::Mesh::default();
    for i in 0..=steps {
        let h = i as f32 / steps as f32;
        let (s, co) = hue_angle(h).sin_cos();
        let col = c32(hsv_to_rgb(Hsv(h, 1.0, 1.0)));
        mesh.colored_vertex(c + vec2(co, s) * outer, col);
        mesh.colored_vertex(c + vec2(co, s) * inner, col);
    }
    for i in 0..steps as u32 {
        let a = i * 2;
        mesh.add_triangle(a, a + 1, a + 2);
        mesh.add_triangle(a + 1, a + 3, a + 2);
    }
    painter.add(Shape::mesh(mesh));
    // Smooth the band's edges with the panel colour.
    let edge = Stroke::new(1.2, ui.visuals().panel_fill);
    painter.circle_stroke(c, outer + 0.4, edge);
    painter.circle_stroke(c, inner - 0.4, edge);
    // The square: saturation left to right, value bottom to top.
    let n = 24;
    let mut mesh = egui::Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            let (s, v) = (i as f32 / n as f32, 1.0 - j as f32 / n as f32);
            mesh.colored_vertex(pos2(square.left() + s * square.width(), square.top() + (1.0 - v) * square.height()), c32(hsv_to_rgb(Hsv(hsv.0, s, v))));
        }
    }
    for j in 0..n {
        for i in 0..n {
            let a = (j * (n + 1) + i) as u32;
            let b = a + 1;
            let cc = a + (n + 1) as u32;
            let d = cc + 1;
            mesh.add_triangle(a, b, cc);
            mesh.add_triangle(b, d, cc);
        }
    }
    painter.add(Shape::mesh(mesh));
    // The markers.
    let marker = pos2(square.left() + hsv.1 * square.width(), square.top() + (1.0 - hsv.2) * square.height());
    let dark = hsv.2 > 0.55 && hsv.1 < 0.6;
    painter.circle_stroke(marker, 5.0, Stroke::new(1.6, if dark { Color32::BLACK } else { Color32::WHITE }));
    let (s, co) = hue_angle(hsv.0).sin_cos();
    let on_ring = c + vec2(co, s) * (inner + outer) / 2.0;
    painter.circle_stroke(on_ring, ring / 2.0 + 1.0, Stroke::new(2.0, Color32::WHITE));
    painter.circle_stroke(on_ring, ring / 2.0 + 2.5, Stroke::new(1.0, Color32::BLACK));
    if changed {
        *rgb = hsv_to_rgb(*hsv);
    }
    changed
}

/// The main colour (front, top left) and the sub colour (behind, bottom right), as in Clip
/// Studio. Clicking anywhere on them swaps them (as does X).
pub fn swatches(ui: &mut Ui, size: f32, main: [u8; 3], sub: [u8; 3]) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    let s = size * 0.64;
    let back = egui::Rect::from_min_size(rect.max - vec2(s, s), vec2(s, s));
    let front = egui::Rect::from_min_size(rect.min, vec2(s, s));
    let outline = |r: egui::Rect| {
        ui.painter().rect_stroke(r, 2.0, Stroke::new(1.0, Color32::from_gray(20)), egui::StrokeKind::Outside);
        ui.painter().rect_stroke(r, 2.0, Stroke::new(1.0, Color32::from_gray(200)), egui::StrokeKind::Inside);
    };
    ui.painter().rect_filled(back, 2.0, c32(sub));
    outline(back);
    ui.painter().rect_filled(front, 2.0, c32(main));
    outline(front);
    resp
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
