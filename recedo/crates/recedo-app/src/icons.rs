//! The interface's icons, drawn with shapes on a 24 × 24 grid (egui's fonts have no tool
//! symbols): one for every tool and for each toolbar action.

use std::f32::consts::{PI, TAU};

use egui::{Color32, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, pos2, vec2};

use crate::editor::Tool;

/// The toolbar's actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    New,
    Open,
    Save,
    Undo,
    Redo,
    Smaller,
    Bigger,
    ZoomIn,
    ZoomOut,
    Fit,
    Reference,
    AllTools,
    LayerNew,
    LayerDuplicate,
    LayerDelete,
    LayerUp,
    LayerDown,
    LayerMerge,
}

impl Action {
    #[cfg(test)]
    pub const ALL: [Action; 18] = [
        Action::New,
        Action::Open,
        Action::Save,
        Action::Undo,
        Action::Redo,
        Action::Smaller,
        Action::Bigger,
        Action::ZoomIn,
        Action::ZoomOut,
        Action::Fit,
        Action::Reference,
        Action::AllTools,
        Action::LayerNew,
        Action::LayerDuplicate,
        Action::LayerDelete,
        Action::LayerUp,
        Action::LayerDown,
        Action::LayerMerge,
    ];
}

/// Draws on the 24 × 24 grid scaled into a rectangle.
struct Pen<'a> {
    p: &'a Painter,
    origin: Pos2,
    scale: f32,
    col: Color32,
}

type P = (f32, f32);

impl Pen<'_> {
    fn at(&self, (x, y): P) -> Pos2 {
        self.origin + vec2(x, y) * self.scale
    }

    fn pts(&self, pts: &[P]) -> Vec<Pos2> {
        pts.iter().map(|&q| self.at(q)).collect()
    }

    fn stroke(&self, w: f32) -> Stroke {
        Stroke::new(w * self.scale, self.col)
    }

    fn line(&self, pts: &[P], w: f32) {
        self.p.add(Shape::line(self.pts(pts), self.stroke(w)));
    }

    /// A line with round ends.
    fn round(&self, a: P, b: P, w: f32) {
        self.line(&[a, b], w);
        self.disc(a, w / 2.0);
        self.disc(b, w / 2.0);
    }

    fn closed(&self, pts: &[P], w: f32) {
        self.p.add(Shape::closed_line(self.pts(pts), self.stroke(w)));
    }

    fn dashed(&self, pts: &[P], w: f32) {
        let mut v = self.pts(pts);
        v.push(v[0]);
        self.p.extend(Shape::dashed_line(&v, self.stroke(w), 2.4 * self.scale, 1.8 * self.scale));
    }

    /// A filled convex shape.
    fn fill(&self, pts: &[P]) {
        self.p.add(Shape::convex_polygon(self.pts(pts), self.col, Stroke::NONE));
    }

    /// Fills the band between two lines with the same number of points (a crescent).
    fn band(&self, a: &[P], b: &[P]) {
        let mut mesh = egui::Mesh::default();
        for (&p, &q) in a.iter().zip(b) {
            mesh.colored_vertex(self.at(p), self.col);
            mesh.colored_vertex(self.at(q), self.col);
        }
        for i in 0..a.len().min(b.len()).saturating_sub(1) as u32 {
            mesh.add_triangle(2 * i, 2 * i + 1, 2 * i + 2);
            mesh.add_triangle(2 * i + 1, 2 * i + 3, 2 * i + 2);
        }
        self.p.add(Shape::mesh(mesh));
    }

    fn disc(&self, c: P, r: f32) {
        self.p.circle_filled(self.at(c), r * self.scale, self.col);
    }

    fn ring(&self, c: P, r: f32, w: f32) {
        self.p.circle_stroke(self.at(c), r * self.scale, self.stroke(w));
    }

    fn rect(&self, (x0, y0): P, (x1, y1): P, w: f32) {
        self.closed(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)], w);
    }

    fn fill_rect(&self, (x0, y0): P, (x1, y1): P) {
        self.fill(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)]);
    }
}

/// Points on an ellipse (angles in radians, screen y down).
fn arc(c: P, rx: f32, ry: f32, from: f32, to: f32, n: usize) -> Vec<P> {
    (0..=n)
        .map(|i| {
            let a = from + (to - from) * i as f32 / n as f32;
            (c.0 + rx * a.cos(), c.1 + ry * a.sin())
        })
        .collect()
}

fn ellipse(c: P, rx: f32, ry: f32) -> Vec<P> {
    let mut v = arc(c, rx, ry, 0.0, TAU, 40);
    v.pop();
    v
}

/// A block (eraser, ruler) along the direction `deg` (0 = right, counter-clockwise).
fn block(c: P, deg: f32, half_len: f32, half_w: f32) -> [P; 4] {
    let (s, co) = deg.to_radians().sin_cos();
    let (u, v) = ((co, -s), (s, co));
    let at = |a: f32, b: f32| (c.0 + u.0 * a + v.0 * b, c.1 + u.1 * a + v.1 * b);
    [at(-half_len, -half_w), at(half_len, -half_w), at(half_len, half_w), at(-half_len, half_w)]
}

fn sparkle(g: &Pen, c: P, r: f32) {
    g.line(&[(c.0, c.1 - r), (c.0, c.1 + r)], 1.4);
    g.line(&[(c.0 - r, c.1), (c.0 + r, c.1)], 1.4);
    let d = r * 0.45;
    g.line(&[(c.0 - d, c.1 - d), (c.0 + d, c.1 + d)], 1.0);
    g.line(&[(c.0 + d, c.1 - d), (c.0 - d, c.1 + d)], 1.0);
}

fn magnifier(g: &Pen) {
    g.ring((10.0, 10.0), 6.0, 1.8);
    g.round((14.6, 14.6), (20.0, 20.0), 3.0);
}

fn eraser(g: &Pen, c: P, half_len: f32, half_w: f32) {
    let [a, b, cc, d] = block(c, 45.0, half_len, half_w);
    g.closed(&[a, b, cc, d], 1.6);
    // The rubber end, filled.
    let mid = |p: P, q: P| ((p.0 + q.0) / 2.0, (p.1 + q.1) / 2.0);
    g.fill(&[a, mid(a, b), mid(d, cc), d]);
}

/// The outline a lasso draws.
const LOOP: [P; 7] = [(5.0, 9.0), (10.0, 4.5), (17.0, 5.5), (20.5, 10.5), (17.5, 15.5), (10.5, 16.5), (5.5, 13.5)];

fn draw_tool(g: &Pen, t: Tool) {
    match t {
        Tool::Brush => {
            g.line(&[(20.5, 3.5), (13.8, 10.2)], 2.6);
            g.line(&[(13.8, 10.2), (11.6, 12.4)], 3.8);
            g.fill(&[(12.6, 13.6), (10.4, 11.4), (6.6, 14.2), (3.5, 20.5), (9.8, 17.4)]);
        }
        Tool::Eraser => {
            eraser(g, (12.5, 10.5), 7.5, 4.0);
            g.line(&[(3.0, 21.0), (21.0, 21.0)], 1.4);
        }
        Tool::Spray => {
            g.rect((11.0, 9.5), (18.5, 21.0), 1.6);
            g.fill_rect((12.5, 6.5), (17.0, 9.5));
            g.fill_rect((13.6, 4.0), (15.9, 6.5));
            for c in [(8.5, 5.0), (5.5, 7.0), (8.5, 8.5), (4.0, 10.0), (7.0, 11.0), (4.5, 4.5), (9.0, 2.5)] {
                g.disc(c, 0.95);
            }
        }
        Tool::Eyedropper => {
            g.disc((18.5, 5.5), 3.0);
            g.line(&[(14.2, 6.2), (17.8, 9.8)], 1.8);
            g.line(&[(16.0, 8.0), (7.0, 17.0)], 2.4);
            g.line(&[(7.0, 17.0), (4.0, 20.0)], 1.2);
        }
        Tool::Smudge => {
            // A smear: wide where the finger started, thinning out.
            for i in 0..=16 {
                let t = i as f32 / 16.0;
                let (x, y) = (5.0 + 14.5 * t, 18.0 - 12.0 * t + 2.5 * (TAU * t).sin());
                g.disc((x, y), 3.3 * (1.0 - t) + 0.7);
            }
        }
        Tool::Blur => {
            // A drop: its tip, then round the bottom.
            let mut v = vec![(12.0, 3.0)];
            v.extend(arc((12.0, 14.5), 6.0, 6.0, -PI / 6.0, PI + PI / 6.0, 24));
            g.closed(&v, 1.6);
        }
        Tool::Sharpen => g.fill(&[(12.0, 3.0), (19.5, 20.0), (4.5, 20.0)]),
        Tool::Dodge => {
            // A sun: lightens.
            g.ring((12.0, 12.0), 4.2, 1.6);
            for i in 0..8 {
                let a = i as f32 * TAU / 8.0;
                g.line(&[(12.0 + 6.6 * a.cos(), 12.0 + 6.6 * a.sin()), (12.0 + 9.4 * a.cos(), 12.0 + 9.4 * a.sin())], 1.6);
            }
        }
        Tool::Burn => {
            // A crescent moon: darkens. The circle (12, 12) r 8.5 less the circle (15.5, 8.5)
            // r 7, which cross at about (20.4, 13.5) and (10.5, 3.6): the outer arc between
            // them the long way round, the inner one the short way.
            let outer = arc((12.0, 12.0), 8.5, 8.5, 10.4f32.to_radians(), 259.6f32.to_radians(), 40);
            let inner = arc((15.5, 8.5), 7.0, 7.0, 46.0f32.to_radians(), 224.0f32.to_radians(), 40);
            g.band(&outer, &inner);
            // A fine outline smooths the band's edge.
            let mut edge = outer.clone();
            edge.extend(inner.iter().rev());
            g.closed(&edge, 0.5);
        }
        Tool::Sponge => {
            g.closed(&[(4.0, 8.0), (20.0, 6.0), (20.5, 17.0), (4.5, 18.5)], 1.6);
            for c in [(8.0, 11.0), (12.5, 13.5), (16.0, 10.0), (9.0, 15.5), (16.5, 14.5)] {
                g.disc(c, 1.2);
            }
        }
        Tool::Recolour => {
            g.closed(&ellipse((12.0, 12.0), 9.0, 7.5), 1.6);
            g.ring((14.5, 16.0), 1.6, 1.2);
            for (c, col) in [((7.5, 10.0), Color32::from_rgb(230, 70, 60)), ((11.5, 7.5), Color32::from_rgb(90, 200, 90)), ((16.0, 8.5), Color32::from_rgb(80, 140, 240))] {
                g.p.circle_filled(g.at(c), 1.9 * g.scale, col);
            }
        }
        Tool::Clone => {
            g.disc((12.0, 5.0), 2.8);
            g.line(&[(12.0, 7.0), (12.0, 12.5)], 2.6);
            g.fill_rect((5.0, 12.5), (19.0, 16.5));
            g.line(&[(5.5, 19.0), (18.5, 19.0)], 1.6);
        }
        Tool::Push => {
            // A swirl, as pushed paint does.
            let v: Vec<P> = (0..=60)
                .map(|i| {
                    let t = i as f32 / 60.0;
                    let a = t * 1.8 * TAU;
                    let r = 1.0 + 7.5 * t;
                    (12.0 + r * a.cos(), 12.0 + r * a.sin())
                })
                .collect();
            g.line(&v, 1.6);
        }
        Tool::Fill => {
            g.closed(&[(3.5, 11.0), (11.0, 3.5), (18.5, 11.0), (11.0, 18.5)], 1.6);
            g.fill(&[(3.5, 11.0), (18.5, 11.0), (11.0, 18.5)]);
            g.disc((20.5, 18.5), 1.8);
            g.fill(&[(20.5, 14.0), (22.2, 17.9), (18.8, 17.9)]);
        }
        Tool::Gradient => {
            let (a, b) = (g.at((4.0, 6.0)), g.at((20.0, 18.0)));
            let mut mesh = egui::Mesh::default();
            let clear = Color32::from_rgba_unmultiplied(g.col.r(), g.col.g(), g.col.b(), 0);
            mesh.colored_vertex(a, g.col);
            mesh.colored_vertex(pos2(b.x, a.y), clear);
            mesh.colored_vertex(b, clear);
            mesh.colored_vertex(pos2(a.x, b.y), g.col);
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            g.p.add(Shape::mesh(mesh));
            g.rect((4.0, 6.0), (20.0, 18.0), 1.4);
        }
        Tool::LassoFill => {
            g.fill(&LOOP);
            g.line(&[(8.0, 16.2), (7.2, 19.0), (5.0, 21.0)], 1.4);
        }
        Tool::MagicEraser => {
            eraser(g, (10.0, 14.0), 6.5, 3.4);
            sparkle(g, (18.0, 6.0), 3.8);
        }
        Tool::Line => {
            g.line(&[(5.0, 19.0), (19.0, 5.0)], 1.8);
            g.fill_rect((3.4, 17.4), (6.6, 20.6));
            g.fill_rect((17.4, 3.4), (20.6, 6.6));
        }
        Tool::Rectangle => g.rect((4.0, 6.0), (20.0, 18.0), 1.8),
        Tool::Ellipse => g.closed(&ellipse((12.0, 12.0), 8.5, 6.5), 1.8),
        Tool::SelectRect => g.dashed(&[(4.0, 6.0), (20.0, 6.0), (20.0, 18.0), (4.0, 18.0)], 1.5),
        Tool::SelectEllipse => g.dashed(&ellipse((12.0, 12.0), 8.5, 6.5), 1.5),
        Tool::SelectLasso => {
            g.dashed(&LOOP, 1.5);
            g.line(&[(8.0, 16.2), (7.2, 19.0), (5.0, 21.0)], 1.4);
        }
        Tool::MagicWand => {
            g.line(&[(4.0, 20.0), (14.0, 10.0)], 2.4);
            sparkle(g, (17.0, 7.0), 4.5);
        }
        Tool::Move => {
            g.line(&[(12.0, 5.0), (12.0, 19.0)], 1.6);
            g.line(&[(5.0, 12.0), (19.0, 12.0)], 1.6);
            g.fill(&[(12.0, 2.0), (15.0, 5.6), (9.0, 5.6)]);
            g.fill(&[(12.0, 22.0), (9.0, 18.4), (15.0, 18.4)]);
            g.fill(&[(2.0, 12.0), (5.6, 9.0), (5.6, 15.0)]);
            g.fill(&[(22.0, 12.0), (18.4, 15.0), (18.4, 9.0)]);
        }
        Tool::LassoTransform => {
            // A lasso loop in a box with corner handles: the part drawn round, scaled.
            g.rect((4.0, 4.0), (20.0, 20.0), 1.0);
            for (x, y) in [(4.0, 4.0), (20.0, 4.0), (20.0, 20.0), (4.0, 20.0)] {
                g.fill_rect((x - 2.2, y - 2.2), (x + 2.2, y + 2.2));
            }
            let small: Vec<P> = LOOP.iter().map(|&(x, y)| (6.0 + (x - 5.0) * 0.75, 7.5 + (y - 4.5) * 0.75)).collect();
            g.dashed(&small, 1.3);
        }
        Tool::Crop => {
            g.line(&[(7.0, 3.0), (7.0, 17.0), (21.0, 17.0)], 2.2);
            g.line(&[(3.0, 7.0), (17.0, 7.0), (17.0, 21.0)], 2.2);
        }
        Tool::Text => {
            g.line(&[(5.0, 7.5), (5.0, 4.5), (19.0, 4.5), (19.0, 7.5)], 2.2);
            g.line(&[(12.0, 4.5), (12.0, 20.0)], 2.6);
            g.line(&[(8.5, 20.0), (15.5, 20.0)], 1.8);
        }
        Tool::Pan => {
            for (x, top, bottom) in [(8.6, 6.5, 13.0), (11.6, 4.2, 12.0), (14.6, 4.8, 12.0), (17.6, 7.6, 13.0)] {
                g.round((x, top), (x, bottom), 2.7);
            }
            g.fill(&[(7.3, 12.0), (18.9, 12.0), (18.6, 16.0), (15.8, 20.5), (10.2, 20.5), (7.3, 17.0)]);
            g.round((8.2, 16.0), (4.6, 12.0), 2.7);
        }
        Tool::Zoom => magnifier(g),
        Tool::Rotate => {
            let v = arc((12.0, 12.0), 7.5, 7.5, -0.35 * PI, 1.35 * PI, 36);
            g.line(&v, 1.8);
            // The arrowhead at the end, pointing along the arc.
            let end = v[v.len() - 1];
            g.fill(&[(end.0 - 3.0, end.1 - 1.0), (end.0 + 3.0, end.1 - 1.0), (end.0, end.1 + 3.4)]);
        }
        Tool::Measure => {
            let [a, b, c, d] = block((12.0, 12.0), 45.0, 9.5, 3.6);
            g.closed(&[a, b, c, d], 1.6);
            // Ticks from the edge a-b inward.
            for i in 1..8 {
                let t = i as f32 / 8.0;
                let e = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
                let depth = if i % 2 == 0 { 3.0 } else { 1.8 };
                let n = ((d.0 - a.0) / 7.2, (d.1 - a.1) / 7.2);
                g.line(&[e, (e.0 + n.0 * depth, e.1 + n.1 * depth)], 1.1);
            }
        }
    }
}

fn draw_action(g: &Pen, a: Action) {
    match a {
        Action::New => {
            g.closed(&[(6.0, 3.0), (14.0, 3.0), (19.0, 8.0), (19.0, 21.0), (6.0, 21.0)], 1.6);
            g.line(&[(14.0, 3.0), (14.0, 8.0), (19.0, 8.0)], 1.4);
        }
        Action::Open => {
            g.closed(&[(3.0, 5.5), (9.0, 5.5), (11.0, 7.5), (19.0, 7.5), (19.0, 19.0), (3.0, 19.0)], 1.6);
            g.closed(&[(6.0, 11.0), (22.0, 11.0), (19.0, 19.0), (3.0, 19.0)], 1.6);
        }
        Action::Save => {
            g.closed(&[(4.0, 4.0), (17.0, 4.0), (20.0, 7.0), (20.0, 20.0), (4.0, 20.0)], 1.6);
            g.fill_rect((8.0, 4.0), (15.0, 9.0));
            g.rect((7.5, 13.0), (16.5, 20.0), 1.4);
        }
        Action::Undo | Action::Redo => {
            // A hook: up from the bottom, over the top, the arrowhead on the far side.
            let flip = |p: P| if a == Action::Redo { (24.0 - p.0, p.1) } else { p };
            let mut v = vec![flip((18.5, 19.0))];
            v.extend(arc((12.0, 12.5), 6.5, 6.5, 0.0, -PI, 24).into_iter().map(flip));
            g.line(&v, 1.8);
            g.fill(&[flip((5.5, 18.0)), flip((2.0, 12.0)), flip((9.0, 12.0))]);
        }
        Action::Smaller => g.disc((12.0, 12.0), 3.2),
        Action::Bigger => g.disc((12.0, 12.0), 7.5),
        Action::ZoomIn | Action::ZoomOut => {
            magnifier(g);
            g.line(&[(7.0, 10.0), (13.0, 10.0)], 1.6);
            if a == Action::ZoomIn {
                g.line(&[(10.0, 7.0), (10.0, 13.0)], 1.6);
            }
        }
        Action::Fit => {
            for (x, y, dx, dy) in [(3.0, 3.0, 1.0, 1.0), (21.0, 3.0, -1.0, 1.0), (3.0, 21.0, 1.0, -1.0), (21.0, 21.0, -1.0, -1.0)] {
                g.line(&[(x, y + 5.5 * dy), (x, y), (x + 5.5 * dx, y)], 1.8);
            }
            g.rect((8.0, 9.0), (16.0, 15.0), 1.4);
        }
        Action::Reference => {
            g.rect((3.0, 5.0), (21.0, 19.0), 1.6);
            g.fill(&[(5.0, 17.0), (10.0, 10.5), (15.0, 17.0)]);
            g.fill(&[(11.5, 17.0), (15.5, 12.0), (19.0, 17.0)]);
            g.disc((16.5, 8.8), 1.7);
        }
        Action::LayerNew => {
            g.rect((4.0, 4.0), (15.0, 18.0), 1.6);
            g.line(&[(18.0, 13.5), (18.0, 22.0)], 1.8);
            g.line(&[(13.75, 17.75), (22.25, 17.75)], 1.8);
        }
        Action::LayerDuplicate => {
            g.rect((8.0, 8.0), (20.0, 20.0), 1.6);
            g.line(&[(4.0, 15.5), (4.0, 4.0), (15.5, 4.0)], 1.6);
        }
        Action::LayerDelete => {
            g.line(&[(4.0, 6.5), (20.0, 6.5)], 1.6);
            g.line(&[(9.5, 6.5), (9.5, 3.5), (14.5, 3.5), (14.5, 6.5)], 1.4);
            g.closed(&[(6.0, 9.0), (18.0, 9.0), (17.0, 21.0), (7.0, 21.0)], 1.6);
            g.line(&[(10.0, 11.5), (10.3, 18.5)], 1.2);
            g.line(&[(14.0, 11.5), (13.7, 18.5)], 1.2);
        }
        Action::LayerUp | Action::LayerDown => {
            let f = |p: P| if a == Action::LayerDown { (p.0, 24.0 - p.1) } else { p };
            g.line(&[f((12.0, 7.0)), f((12.0, 21.0))], 1.8);
            g.fill(&[f((12.0, 2.5)), f((18.0, 9.0)), f((6.0, 9.0))]);
        }
        Action::LayerMerge => {
            g.line(&[(5.0, 3.5), (12.0, 11.0), (19.0, 3.5)], 1.8);
            g.line(&[(12.0, 11.0), (12.0, 15.5)], 1.8);
            g.fill(&[(12.0, 19.0), (8.0, 14.5), (16.0, 14.5)]);
            g.line(&[(4.0, 21.0), (20.0, 21.0)], 1.8);
        }
        Action::AllTools => {
            for y in 0..3 {
                for x in 0..3 {
                    let (x0, y0) = (4.5 + 5.5 * x as f32, 4.5 + 5.5 * y as f32);
                    g.fill_rect((x0, y0), (x0 + 4.0, y0 + 4.0));
                }
            }
        }
    }
}

fn pen(p: &Painter, rect: Rect, col: Color32) -> Pen<'_> {
    let side = rect.width().min(rect.height());
    Pen { p, origin: rect.center() - vec2(side, side) / 2.0, scale: side / 24.0, col }
}

pub fn tool(p: &Painter, rect: Rect, t: Tool, col: Color32) {
    draw_tool(&pen(p, rect, col), t);
}

pub fn action(p: &Painter, rect: Rect, a: Action, col: Color32) {
    draw_action(&pen(p, rect, col), a);
}

/// A square button with an icon, highlighted when `selected`.
pub fn button(ui: &mut Ui, size: f32, selected: bool, enabled: bool, draw: impl FnOnce(&Painter, Rect, Color32)) -> Response {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), sense);
    if ui.is_rect_visible(rect) {
        let v = ui.visuals();
        let (fill, col) = if selected {
            (v.selection.bg_fill, v.selection.stroke.color)
        } else if !enabled {
            (Color32::TRANSPARENT, v.weak_text_color().gamma_multiply(0.6))
        } else if resp.hovered() {
            (v.widgets.hovered.weak_bg_fill, v.widgets.hovered.fg_stroke.color)
        } else {
            (Color32::TRANSPARENT, v.widgets.inactive.fg_stroke.color)
        };
        ui.painter().rect_filled(rect, 3.0, fill);
        draw(ui.painter(), rect.shrink(size * 0.16), col);
    }
    resp
}

/// A tool's button in a list: its icon, then its name, in a light outline.
pub fn list_button(ui: &mut Ui, width: f32, height: f32, selected: bool, label: &str, draw: impl FnOnce(&Painter, Rect, Color32)) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    if ui.is_rect_visible(rect) {
        let v = ui.visuals();
        let (fill, col) = if selected {
            (v.selection.bg_fill, v.selection.stroke.color)
        } else if resp.hovered() {
            (v.widgets.hovered.weak_bg_fill, v.widgets.hovered.fg_stroke.color)
        } else {
            (v.widgets.inactive.weak_bg_fill, v.widgets.inactive.fg_stroke.color)
        };
        ui.painter().rect(rect, 3.0, fill, egui::Stroke::new(1.0, Color32::from_gray(if selected { 150 } else { 110 })), egui::StrokeKind::Inside);
        let icon = height - 6.0;
        draw(ui.painter(), Rect::from_center_size(egui::pos2(rect.left() + 4.0 + icon / 2.0, rect.center().y), vec2(icon, icon)), col);
        ui.painter().text(egui::pos2(rect.left() + icon + 10.0, rect.center().y), egui::Align2::LEFT_CENTER, label, egui::TextStyle::Button.resolve(ui.style()), col);
    }
    resp
}

/// A tool's button on the strip: its icon with its name under it, `width` wide.
pub fn tool_button(ui: &mut Ui, width: f32, height: f32, selected: bool, label: &str, draw: impl FnOnce(&Painter, Rect, Color32)) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    if ui.is_rect_visible(rect) {
        let v = ui.visuals();
        let (fill, col) = if selected {
            (v.selection.bg_fill, v.selection.stroke.color)
        } else if resp.hovered() {
            (v.widgets.hovered.weak_bg_fill, v.widgets.hovered.fg_stroke.color)
        } else {
            (Color32::TRANSPARENT, v.widgets.inactive.fg_stroke.color)
        };
        ui.painter().rect_filled(rect, 3.0, fill);
        let icon = (height - 13.0).clamp(12.0, 20.0);
        draw(ui.painter(), Rect::from_center_size(egui::pos2(rect.center().x, rect.top() + 2.0 + icon / 2.0), vec2(icon, icon)), col);
        // The name, shrunk to fit the strip if it has to.
        let font = egui::FontId::proportional(9.0);
        let galley = ui.painter().layout_no_wrap(label.to_string(), font, col);
        let scale = ((width - 4.0) / galley.size().x).min(1.0);
        let at = egui::pos2(rect.center().x, rect.bottom() - 2.0);
        if scale < 1.0 {
            ui.painter().text(at, egui::Align2::CENTER_BOTTOM, label, egui::FontId::proportional(9.0 * scale), col);
        } else {
            ui.painter().galley(at - vec2(galley.size().x / 2.0, galley.size().y), galley, col);
        }
    }
    resp
}
