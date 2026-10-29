//! The radial quick menu (R): a ring round the pointer with the tool bar's first tools in
//! the outer ring and colours in the inner one. Tap R to keep it open and click a slice, or
//! hold R, move to a slice and let go. Any press outside it, R again or Escape closes it.
//!
//! Whether letting go of R picks depends on the pointer having moved onto a slice, never on
//! time: a slow frame makes a quick tap arrive late, and it must still leave the menu open.

use std::f32::consts::TAU;

use egui::{Color32, Pos2, Rect, Stroke, Vec2, vec2};

use crate::editor::Tool;

/// The ring's sizes (interface points): the outer edge, where the tool and colour rings
/// meet, the colour ring's inner edge.
pub const OUTER: f32 = 124.0;
pub const SPLIT: f32 = 80.0;
pub const HOLE: f32 = 40.0;
/// At most this many tools and colours.
pub const MAX_SLICES: usize = 8;
/// How far (interface points) the pointer must move after R opened the menu before letting
/// go of R picks the slice under it.
pub const MOVE: f32 = 10.0;

/// The colours after the recent ones, when fewer than [`MAX_SLICES`] were used yet.
const BASIC: [[u8; 3]; 8] = [[0, 0, 0], [255, 255, 255], [220, 40, 40], [245, 200, 30], [40, 160, 70], [40, 170, 220], [40, 70, 200], [150, 60, 190]];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Slot {
    Tool(usize),
    Color(usize),
    Centre,
    Outside,
}

pub struct Radial {
    /// The ring's middle (interface points).
    pub at: Pos2,
    /// Where the pointer was when the menu opened, and whether it has moved off there since.
    start: Pos2,
    moved: bool,
    pub tools: Vec<Tool>,
    pub colors: Vec<[u8; 3]>,
    /// The press that started on the ring (a release elsewhere picks nothing).
    pressed: Option<Slot>,
}

/// What the user chose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pick {
    Tool(Tool),
    Color([u8; 3]),
}

impl Radial {
    /// A menu round `at`, moved in so the whole ring is inside `screen`.
    pub fn new(at: Pos2, screen: Rect, bar: &[Tool], recent: &[[u8; 3]]) -> Radial {
        let start = at;
        let room = screen.shrink(OUTER + 4.0);
        let at = if room.is_positive() { at.clamp(room.min, room.max) } else { screen.center() };
        let tools: Vec<Tool> = bar.iter().copied().take(MAX_SLICES).collect();
        let mut colors: Vec<[u8; 3]> = Vec::new();
        for &c in recent.iter().chain(BASIC.iter()) {
            if colors.len() < MAX_SLICES && !colors.contains(&c) {
                colors.push(c);
            }
        }
        Radial { at, start, moved: false, tools, colors, pressed: None }
    }

    pub fn slot_at(&self, p: Pos2) -> Slot {
        slot_at(p - self.at, self.tools.len(), self.colors.len())
    }

    /// The middle of a slice (for drawing its icon, and for the tests).
    pub fn centre_of(&self, s: Slot) -> Pos2 {
        let (i, n, r) = match s {
            Slot::Tool(i) => (i, self.tools.len(), (SPLIT + OUTER) / 2.0),
            Slot::Color(i) => (i, self.colors.len(), (HOLE + SPLIT) / 2.0),
            Slot::Centre | Slot::Outside => return self.at,
        };
        self.at + direction(i as f32 * TAU / n.max(1) as f32) * r
    }

    fn pick(&self, s: Slot) -> Option<Pick> {
        match s {
            Slot::Tool(i) => self.tools.get(i).map(|&t| Pick::Tool(t)),
            Slot::Color(i) => self.colors.get(i).map(|&c| Pick::Color(c)),
            Slot::Centre | Slot::Outside => None,
        }
    }

    /// The pointer this frame: `Some(pick)` and/or `true` to close.
    pub fn pointer(&mut self, hover: Option<Pos2>, pressed: bool, released: bool) -> (Option<Pick>, bool) {
        self.track(hover);
        let slot = hover.map_or(Slot::Outside, |p| self.slot_at(p));
        if pressed {
            if matches!(slot, Slot::Outside | Slot::Centre) {
                return (None, true);
            }
            self.pressed = Some(slot);
        }
        if released && self.pressed.take().is_some() {
            // Pressed on the ring: the slice where it is let go (a pen slid along the ring
            // picks where it stops).
            return match self.pick(slot) {
                Some(p) => (Some(p), true),
                None => (None, matches!(slot, Slot::Outside)),
            };
        }
        (None, false)
    }

    /// Notes whether the pointer has moved since the menu opened.
    pub fn track(&mut self, hover: Option<Pos2>) {
        if hover.is_some_and(|h| (h - self.start).length() > MOVE) {
            self.moved = true;
        }
    }

    /// R let go: if the pointer was moved onto a slice, that slice is picked and the menu
    /// closes; otherwise (a tap, however long it took to arrive) it stays open.
    pub fn key_released(&mut self, hover: Option<Pos2>) -> (Option<Pick>, bool) {
        self.track(hover);
        if !self.moved {
            return (None, false);
        }
        match hover.and_then(|p| self.pick(self.slot_at(p))) {
            Some(p) => (Some(p), true),
            None => (None, false),
        }
    }

    /// Draws the ring; `current` is the tool in use, `color` the colour.
    pub fn draw(&self, p: &egui::Painter, hover: Option<Pos2>, current: Tool, color: [u8; 3], accent: Color32) {
        let slot = hover.map_or(Slot::Outside, |h| self.slot_at(h));
        let c = self.at;
        p.circle_filled(c + vec2(0.0, 3.0), OUTER + 6.0, Color32::from_black_alpha(70));
        p.circle_filled(c, OUTER + 2.0, Color32::from_gray(38));
        let n = self.tools.len().max(1) as f32;
        for (i, &t) in self.tools.iter().enumerate() {
            let a = i as f32 * TAU / n;
            let on = slot == Slot::Tool(i);
            let fill = if t == current {
                accent
            } else if on {
                Color32::from_gray(78)
            } else {
                Color32::from_gray(52)
            };
            sector(p, c, SPLIT + 1.0, OUTER, a - TAU / n / 2.0 + 0.012, a + TAU / n / 2.0 - 0.012, fill);
            let mid = self.centre_of(Slot::Tool(i));
            let size = if on { 26.0 } else { 22.0 };
            crate::icons::tool(p, Rect::from_center_size(mid, vec2(size, size)), t, Color32::from_gray(if on || t == current { 250 } else { 205 }));
        }
        let m = self.colors.len().max(1) as f32;
        for (i, &col) in self.colors.iter().enumerate() {
            let a = i as f32 * TAU / m;
            let fill = Color32::from_rgb(col[0], col[1], col[2]);
            let on = slot == Slot::Color(i);
            let (r0, r1) = if on { (HOLE - 3.0, SPLIT + 3.0) } else { (HOLE + 1.0, SPLIT - 2.0) };
            sector(p, c, r0, r1, a - TAU / m / 2.0 + 0.02, a + TAU / m / 2.0 - 0.02, fill);
            if col == color {
                p.circle_stroke(self.centre_of(Slot::Color(i)), 6.0, Stroke::new(2.0, if luma(col) > 140 { Color32::BLACK } else { Color32::WHITE }));
            }
        }
        // The middle: what the slice under the pointer is, or what is in use.
        p.circle_filled(c, HOLE - 4.0, Color32::from_gray(30));
        let name = match slot {
            Slot::Tool(i) => self.tools[i].name().to_string(),
            Slot::Color(i) => {
                let [r, g, b] = self.colors[i];
                format!("#{r:02X}{g:02X}{b:02X}")
            }
            Slot::Centre | Slot::Outside => current.name().to_string(),
        };
        let font = egui::FontId::proportional(if name.len() > 9 { 10.0 } else { 11.5 });
        p.text(c, egui::Align2::CENTER_CENTER, name, font, Color32::from_gray(235));
    }
}

/// Which part of a ring with `tools` and `colors` slices `d` (from the middle) is on. Slice 0
/// is straight up; the rest follow clockwise.
pub fn slot_at(d: Vec2, tools: usize, colors: usize) -> Slot {
    let r = d.length();
    let (n, ring): (usize, fn(usize) -> Slot) = if r < HOLE {
        return Slot::Centre;
    } else if r < SPLIT {
        (colors, Slot::Color)
    } else if r <= OUTER {
        (tools, Slot::Tool)
    } else {
        return Slot::Outside;
    };
    if n == 0 {
        return Slot::Centre;
    }
    // Clockwise from straight up (screen y points down).
    let a = d.x.atan2(-d.y).rem_euclid(TAU);
    let step = TAU / n as f32;
    ring(((a + step / 2.0) / step) as usize % n)
}

fn direction(a: f32) -> Vec2 {
    vec2(a.sin(), -a.cos())
}

fn luma(c: [u8; 3]) -> u32 {
    (c[0] as u32 * 3 + c[1] as u32 * 6 + c[2] as u32) / 10
}

/// A slice of a ring, from radius `r0` to `r1` and angle `a0` to `a1` (clockwise from up).
fn sector(p: &egui::Painter, c: Pos2, r0: f32, r1: f32, a0: f32, a1: f32, fill: Color32) {
    let steps = (((a1 - a0) * r1 / 6.0).ceil() as usize).max(2);
    let mut mesh = egui::Mesh::default();
    for k in 0..=steps {
        let d = direction(a0 + (a1 - a0) * k as f32 / steps as f32);
        mesh.colored_vertex(c + d * r0, fill);
        mesh.colored_vertex(c + d * r1, fill);
        if k > 0 {
            let i = (2 * k) as u32;
            mesh.add_triangle(i - 2, i - 1, i);
            mesh.add_triangle(i - 1, i + 1, i);
        }
    }
    p.add(egui::Shape::mesh(mesh));
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    fn at(deg: f32, r: f32) -> Vec2 {
        direction(deg.to_radians()) * r
    }

    #[test]
    fn slices_go_clockwise_from_the_top() {
        let mid = (SPLIT + OUTER) / 2.0;
        assert_eq!(slot_at(at(0.0, mid), 8, 8), Slot::Tool(0));
        assert_eq!(slot_at(at(45.0, mid), 8, 8), Slot::Tool(1), "right of the top is next");
        assert_eq!(slot_at(at(90.0, mid), 8, 8), Slot::Tool(2));
        assert_eq!(slot_at(at(-44.0, mid), 8, 8), Slot::Tool(7));
        assert_eq!(slot_at(at(22.0, mid), 8, 8), Slot::Tool(0), "a slice spans ±22.5°");
        assert_eq!(slot_at(at(23.0, mid), 8, 8), Slot::Tool(1));
        assert_eq!(slot_at(at(180.0, (HOLE + SPLIT) / 2.0), 8, 4), Slot::Color(2));
        assert_eq!(slot_at(at(90.0, HOLE - 1.0), 8, 8), Slot::Centre);
        assert_eq!(slot_at(at(90.0, OUTER + 1.0), 8, 8), Slot::Outside);
        assert_eq!(slot_at(at(90.0, (HOLE + SPLIT) / 2.0), 8, 0), Slot::Centre, "no colours: nothing to pick there");
    }

    #[test]
    fn the_menu_stays_on_screen_and_fills_up_its_colours() {
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 700.0));
        let bar = Tool::DEFAULT_BAR;
        let m = Radial::new(pos2(5.0, 690.0), screen, &bar, &[[1, 2, 3], [0, 0, 0]]);
        assert!(m.at.x - OUTER >= 0.0 && m.at.y + OUTER <= 700.0, "{:?}", m.at);
        assert_eq!(m.tools, bar[..MAX_SLICES].to_vec());
        assert_eq!(m.colors[..3], [[1, 2, 3], [0, 0, 0], [255, 255, 255]], "recent first, no repeats");
        assert_eq!(m.colors.len(), MAX_SLICES);
        for i in 0..MAX_SLICES {
            assert_eq!(m.slot_at(m.centre_of(Slot::Tool(i))), Slot::Tool(i));
            assert_eq!(m.slot_at(m.centre_of(Slot::Color(i))), Slot::Color(i));
        }
    }

    #[test]
    fn presses_pick_on_release_and_close_outside() {
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 700.0));
        let mut m = Radial::new(pos2(500.0, 350.0), screen, &Tool::DEFAULT_BAR, &[]);
        let eraser = m.centre_of(Slot::Tool(1));
        assert_eq!(m.pointer(Some(eraser), true, false), (None, false), "a press alone picks nothing");
        assert_eq!(m.pointer(Some(eraser), false, true), (Some(Pick::Tool(Tool::DEFAULT_BAR[1])), true));
        // Pressed on one slice, slid to another: the one let go on.
        let white = m.centre_of(Slot::Color(1));
        m.pointer(Some(m.centre_of(Slot::Color(0))), true, false);
        assert_eq!(m.pointer(Some(white), false, true), (Some(Pick::Color([255, 255, 255])), true));
        assert_eq!(m.pointer(Some(pos2(10.0, 10.0)), true, false), (None, true), "a press outside closes");
        assert_eq!(m.pointer(Some(m.at), true, false), (None, true), "so does the middle");
        // A release with no press on the ring (the press that opened it, say) picks nothing.
        assert_eq!(m.pointer(Some(eraser), false, true), (None, false));
        // R let go after moving onto a slice: picked.
        assert_eq!(m.key_released(Some(eraser)), (Some(Pick::Tool(Tool::DEFAULT_BAR[1])), true));
        // Moved, then back to the middle and let go: stays open (nothing chosen).
        assert_eq!(m.key_released(Some(m.at)), (None, false));
    }

    #[test]
    fn a_tap_on_r_leaves_it_open_however_late_it_arrives() {
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 700.0));
        let mut m = Radial::new(pos2(500.0, 350.0), screen, &Tool::DEFAULT_BAR, &[]);
        // No time is involved: the pointer has not moved, so letting go never picks.
        assert_eq!(m.key_released(Some(pos2(500.0, 350.0))), (None, false));
        // Opened near the screen's edge the ring moves in, so the pointer may already be over
        // a slice: a tap still picks nothing.
        let mut edge = Radial::new(pos2(30.0, 350.0), screen, &Tool::DEFAULT_BAR, &[]);
        assert!(matches!(edge.slot_at(pos2(30.0, 350.0)), Slot::Tool(_) | Slot::Color(_)), "the pointer is on a slice");
        assert_eq!(edge.key_released(Some(pos2(31.0, 351.0))), (None, false));
        // A small wobble (under MOVE) is still a tap.
        assert_eq!(m.key_released(Some(pos2(505.0, 353.0))), (None, false));
    }
}
