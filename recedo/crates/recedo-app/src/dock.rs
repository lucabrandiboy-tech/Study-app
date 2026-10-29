//! Panel docking: panels sit as tabs in stacks in the left or right dock. A tab can be
//! dragged onto another stack (or the empty strip below the stacks for a new one), floated as
//! a window, docked back, closed, and reopened from the Window menu. The layout is saved.
//!
//! (The docking libraries for egui do not support the egui version Recedo uses, so this is
//! Recedo's own, kept deliberately small.)

use egui::{Id, Ui};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Panel {
    Tools,
    ToolOptions,
    Color,
    Layers,
    History,
    /// The minimap of the whole painting, the clock, flip and symmetry.
    Navigator,
}

impl Panel {
    pub const ALL: [Panel; 6] = [Panel::Tools, Panel::ToolOptions, Panel::Color, Panel::Layers, Panel::History, Panel::Navigator];

    pub fn title(self) -> &'static str {
        match self {
            Panel::Tools => "Sub tool",
            Panel::ToolOptions => "Tool property",
            Panel::Color => "Colour",
            Panel::Layers => "Layers",
            Panel::History => "History",
            Panel::Navigator => "Navigator",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Side {
    Left,
    Right,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stack {
    pub tabs: Vec<Panel>,
    pub active: usize,
    /// Share of the dock's height (relative to the other stacks).
    #[serde(default = "one")]
    pub weight: f32,
}

fn one() -> f32 {
    1.0
}

impl Stack {
    fn of(tabs: &[Panel]) -> Stack {
        Stack { tabs: tabs.to_vec(), active: 0, weight: 1.0 }
    }

    fn weighted(tabs: &[Panel], weight: f32) -> Stack {
        Stack { weight, ..Stack::of(tabs) }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Floating {
    pub panel: Panel,
    pub pos: [f32; 2],
    pub size: [f32; 2],
}

/// The layout's version: an older saved layout is replaced by the current default once.
/// 2: the Clip Studio-like layout (colour, sub tool and tool property on the left).
/// 3: the same, narrower (the compact interface).
/// 4: the Navigator panel at the top of the right dock (added to version 3 layouts as they are).
pub const LAYOUT_VERSION: u32 = 4;

/// The Navigator's share of the right dock (its minimap, clock and buttons).
const NAVIGATOR_WEIGHT: f32 = 0.5;

/// The Colour panel's share of the left dock: what the wheel needs, no empty space below.
const COLOR_WEIGHT: f32 = 1.15;

/// The left dock's standard width: Tool property's longest rows fit (its contents no longer
/// stretch it; at 196 "Roundness" was cut off).
const LEFT_WIDTH: f32 = 232.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DockLayout {
    /// Missing in layouts saved before versions were kept (0).
    #[serde(default)]
    pub version: u32,
    pub left: Vec<Stack>,
    pub right: Vec<Stack>,
    pub floating: Vec<Floating>,
    /// Dock widths in points.
    pub left_width: f32,
    pub right_width: f32,
}

impl Default for DockLayout {
    fn default() -> DockLayout {
        DockLayout {
            version: LAYOUT_VERSION,
            left: vec![Stack::weighted(&[Panel::Color], COLOR_WEIGHT), Stack::weighted(&[Panel::Tools], 1.1), Stack::weighted(&[Panel::ToolOptions], 1.5)],
            right: vec![Stack::weighted(&[Panel::Navigator], NAVIGATOR_WEIGHT), Stack::of(&[Panel::Layers, Panel::History])],
            floating: Vec::new(),
            left_width: LEFT_WIDTH,
            right_width: 200.0,
        }
    }
}

/// Where a tab is dropped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// Into an existing stack.
    Stack(Side, usize),
    /// As a new stack at the bottom of a dock.
    NewStack(Side),
}

impl DockLayout {
    fn side(&mut self, side: Side) -> &mut Vec<Stack> {
        match side {
            Side::Left => &mut self.left,
            Side::Right => &mut self.right,
        }
    }

    pub fn is_open(&self, p: Panel) -> bool {
        self.left.iter().chain(&self.right).any(|s| s.tabs.contains(&p)) || self.floating.iter().any(|f| f.panel == p)
    }

    /// Takes a panel out of wherever it is (it is then closed).
    pub fn remove(&mut self, p: Panel) {
        for stacks in [&mut self.left, &mut self.right] {
            for s in stacks.iter_mut() {
                if let Some(i) = s.tabs.iter().position(|&t| t == p) {
                    s.tabs.remove(i);
                    if s.active > i || s.active >= s.tabs.len() {
                        s.active = s.active.saturating_sub(1);
                    }
                }
            }
            stacks.retain(|s| !s.tabs.is_empty());
        }
        self.floating.retain(|f| f.panel != p);
    }

    /// Opens a closed panel (as a new tab in the right dock's last stack) or brings an open
    /// one to the front of its stack.
    pub fn show(&mut self, p: Panel) {
        for s in self.left.iter_mut().chain(self.right.iter_mut()) {
            if let Some(i) = s.tabs.iter().position(|&t| t == p) {
                s.active = i;
                return;
            }
        }
        if self.floating.iter().any(|f| f.panel == p) {
            return;
        }
        match self.right.last_mut() {
            Some(s) => {
                s.tabs.push(p);
                s.active = s.tabs.len() - 1;
            }
            None => self.right.push(Stack::of(&[p])),
        }
    }

    pub fn toggle(&mut self, p: Panel) {
        if self.is_open(p) { self.remove(p) } else { self.show(p) }
    }

    /// Moves a panel to a stack (as its active tab) or to a new stack.
    pub fn move_to(&mut self, p: Panel, target: Target) {
        // Indexes refer to the layout before the move; resolve the target stack first.
        let marker = match target {
            Target::Stack(side, i) => self.side(side).get(i).map(|s| (side, s.tabs.clone())),
            Target::NewStack(_) => None,
        };
        self.remove(p);
        match (target, marker) {
            (Target::Stack(side, _), Some((_, tabs))) => {
                let stacks = self.side(side);
                // Find the stack again (removing `p` may have emptied and dropped another).
                let left: Vec<Panel> = tabs.into_iter().filter(|&t| t != p).collect();
                match stacks.iter_mut().find(|s| !left.is_empty() && s.tabs == left) {
                    Some(s) => {
                        s.tabs.push(p);
                        s.active = s.tabs.len() - 1;
                    }
                    None => stacks.push(Stack::of(&[p])),
                }
            }
            (Target::Stack(side, _), None) | (Target::NewStack(side), _) => self.side(side).push(Stack::of(&[p])),
        }
    }

    pub fn float(&mut self, p: Panel, pos: [f32; 2]) {
        self.remove(p);
        self.floating.push(Floating { panel: p, pos, size: [280.0, 320.0] });
    }

    /// Keeps a loaded layout valid: each panel at most once, no empty stacks, sane widths.
    /// A layout from before the current version becomes the current default.
    pub fn sanitize(&mut self) {
        if self.version < 3 {
            *self = DockLayout::default();
        }
        if self.version == 3 {
            // Laid out before the Navigator: it goes on top of the right dock, the rest stays.
            if !self.is_open(Panel::Navigator) {
                self.right.insert(0, Stack::weighted(&[Panel::Navigator], NAVIGATOR_WEIGHT));
            }
            self.version = LAYOUT_VERSION;
        }
        // The standard left dock as first saved (with empty space under the colour wheel).
        let weights: Vec<f32> = self.left.iter().map(|s| s.weight).collect();
        let tabs: Vec<&[Panel]> = self.left.iter().map(|s| s.tabs.as_slice()).collect();
        if weights == [1.5, 1.1, 1.5] && tabs == [&[Panel::Color][..], &[Panel::Tools], &[Panel::ToolOptions]] {
            self.left[0].weight = COLOR_WEIGHT;
        }
        // Saved at the old standard width, which is now too narrow for Tool property.
        if self.left_width == 196.0 {
            self.left_width = LEFT_WIDTH;
        }
        let mut seen = Vec::new();
        for stacks in [&mut self.left, &mut self.right] {
            for s in stacks.iter_mut() {
                s.tabs.retain(|t| {
                    let new = !seen.contains(t);
                    seen.push(*t);
                    new
                });
                s.active = s.active.min(s.tabs.len().saturating_sub(1));
                s.weight = if s.weight.is_finite() { s.weight.clamp(0.1, 10.0) } else { 1.0 };
            }
            stacks.retain(|s| !s.tabs.is_empty());
        }
        self.floating.retain(|f| {
            let new = !seen.contains(&f.panel);
            seen.push(f.panel);
            new && f.pos.iter().chain(&f.size).all(|v| v.is_finite())
        });
        for f in &mut self.floating {
            f.size = [f.size[0].clamp(120.0, 2000.0), f.size[1].clamp(80.0, 2000.0)];
        }
        let w = |v: f32, d: f32| {
            if v.is_finite() { v.clamp(60.0, 800.0) } else { d }
        };
        let d = DockLayout::default();
        self.left_width = w(self.left_width, d.left_width);
        self.right_width = w(self.right_width, d.right_width);
    }

    /// Shows the docks and floating panels around the canvas. `title` names a panel's tab
    /// (it may say what the panel shows now), `content` draws its contents. Returns the
    /// rectangle left for the canvas (in points).
    pub fn ui(&mut self, root: &mut Ui, title: &dyn Fn(Panel) -> String, content: &mut dyn FnMut(&mut Ui, Panel)) -> egui::Rect {
        let mut moves: Vec<(Panel, Target)> = Vec::new();
        let mut floats: Vec<(Panel, [f32; 2])> = Vec::new();
        let mut closes: Vec<Panel> = Vec::new();
        for side in [Side::Left, Side::Right] {
            if self.side(side).is_empty() {
                continue;
            }
            let (panel, width) = match side {
                Side::Left => (egui::Panel::left("dock_left"), self.left_width),
                Side::Right => (egui::Panel::right("dock_right"), self.right_width),
            };
            // Each dock takes at most a quarter of the window, so a small window keeps room
            // for the painting.
            let most = (root.available_width() * 0.25).clamp(120.0, 800.0);
            let resp = panel.resizable(true).default_size(width.min(most)).min_size(60.0).max_size(most).show(root, |ui| {
                let mut ops = DockOps { moves: &mut moves, floats: &mut floats, closes: &mut closes };
                let stacks = match side {
                    Side::Left => &mut self.left,
                    Side::Right => &mut self.right,
                };
                // The panels inside cannot make the dock wider (something too wide for it is
                // cut off instead): only dragging its edge changes its width. A dock that grew
                // with what it showed moved the canvas, and a lasso being drawn with it.
                let rect = ui.available_rect_before_wrap();
                let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(*ui.layout()));
                inner.set_clip_rect(rect.intersect(ui.clip_rect()));
                stacks_ui(&mut inner, side, stacks, title, content, &mut ops);
                ui.advance_cursor_after_rect(rect);
            });
            let w = resp.response.rect.width();
            match side {
                Side::Left => self.left_width = w,
                Side::Right => self.right_width = w,
            }
        }
        // Floating panels.
        let ctx = root.ctx().clone();
        let mut docks = Vec::new();
        for f in &mut self.floating {
            let mut open = true;
            let r = egui::Window::new(title(f.panel))
                .id(Id::unique(("recedo floating", f.panel)))
                .default_pos(f.pos)
                .default_size(f.size)
                .collapsible(false)
                .resizable(true)
                .open(&mut open)
                .show(&ctx, |ui| {
                    if ui.small_button("Dock to the right").on_hover_text("Put this panel back in the right dock").clicked() {
                        docks.push(f.panel);
                    }
                    ui.separator();
                    content(ui, f.panel);
                });
            if let Some(r) = r {
                f.pos = [r.response.rect.min.x, r.response.rect.min.y];
                f.size = [r.response.rect.width(), r.response.rect.height()];
            }
            if !open {
                closes.push(f.panel);
            }
        }
        for p in docks {
            moves.push((p, Target::NewStack(Side::Right)));
        }
        for (p, t) in moves {
            self.move_to(p, t);
        }
        for (p, pos) in floats {
            self.float(p, pos);
        }
        for p in closes {
            self.remove(p);
        }
        root.available_rect_before_wrap()
    }
}

/// The panels' title strips.
pub const HEADER_FILL: egui::Color32 = egui::Color32::from_gray(42);

/// A panel's tab on the header strip: plain text, the open one bright with a blue line
/// under it (as in Clip Studio), the others dimmer.
fn tab_label(ui: &mut egui::Ui, active: bool, text: &str) -> egui::Response {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, egui::Color32::WHITE);
    let size = galley.size() + egui::vec2(10.0, 5.0);
    let (rect, r) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        let color = if active {
            egui::Color32::from_gray(240)
        } else if r.hovered() {
            egui::Color32::from_gray(210)
        } else {
            egui::Color32::from_gray(150)
        };
        if active {
            p.rect_filled(rect, 0.0, egui::Color32::from_gray(54));
            p.rect_filled(egui::Rect::from_min_max(egui::pos2(rect.min.x, rect.max.y - 2.0), rect.max), 0.0, crate::ui::ACCENT);
        }
        p.galley(rect.min + egui::vec2(5.0, 2.0), galley, color);
    }
    r
}

struct DockOps<'a> {
    moves: &'a mut Vec<(Panel, Target)>,
    floats: &'a mut Vec<(Panel, [f32; 2])>,
    closes: &'a mut Vec<Panel>,
}

fn stacks_ui(ui: &mut Ui, side: Side, stacks: &mut [Stack], title: &dyn Fn(Panel) -> String, content: &mut dyn FnMut(&mut Ui, Panel), ops: &mut DockOps<'_>) {
    let n = stacks.len();
    let dragging_tab = ui.ctx().dragged_id().is_some_and(|id| Panel::ALL.iter().any(|&p| id == Id::unique(("recedo tab", p))));
    // While a tab is dragged, a strip at the bottom takes drops that make a new stack.
    let strip = if dragging_tab { 24.0 } else { 0.0 };
    let handle = 6.0;
    let avail = ui.available_height() - strip - handle * (n.saturating_sub(1)) as f32;
    let total: f32 = stacks.iter().map(|s| s.weight).sum::<f32>().max(0.1);
    let heights: Vec<f32> = stacks.iter().map(|s| (avail * s.weight / total).max(40.0)).collect();
    for i in 0..n {
        let stack = &mut stacks[i];
        ui.push_id(("stack", side, i), |ui| {
            ui.set_height(heights[i]);
            ui.vertical(|ui| {
                // The tabs on a darker header strip, as in Clip Studio.
                let header = egui::Frame::NONE.fill(HEADER_FILL).inner_margin(egui::Margin::symmetric(3, 2));
                let (_, dropped) = ui.dnd_drop_zone::<Panel, ()>(header, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal_wrapped(|ui| {
                        for (t, &tab) in stack.tabs.iter().enumerate() {
                            let id = Id::unique(("recedo tab", tab));
                            let r = ui.dnd_drag_source(id, tab, |ui| tab_label(ui, t == stack.active, &title(tab)));
                            if r.inner.clicked() {
                                stack.active = t;
                            }
                            r.inner.on_hover_text("Drag to move this panel; right-click for more").context_menu(|ui| {
                                if ui.button("Float as a window").clicked() {
                                    let p = ui.ctx().pointer_latest_pos().map_or([200.0, 150.0], |p| [p.x - 100.0, p.y]);
                                    ops.floats.push((tab, p));
                                    ui.close();
                                }
                                let (other, label) = match side {
                                    Side::Left => (Side::Right, "Move to the right dock"),
                                    Side::Right => (Side::Left, "Move to the left dock"),
                                };
                                if ui.button(label).clicked() {
                                    ops.moves.push((tab, Target::NewStack(other)));
                                    ui.close();
                                }
                                if stack.tabs.len() > 1 && ui.button("Separate (own group)").clicked() {
                                    ops.moves.push((tab, Target::NewStack(side)));
                                    ui.close();
                                }
                                if ui.button("Close").clicked() {
                                    ops.closes.push(tab);
                                    ui.close();
                                }
                            });
                        }
                    });
                });
                if let Some(p) = dropped {
                    ops.moves.push((*p, Target::Stack(side, i)));
                }
                ui.add_space(3.0);
                if let Some(&active) = stack.tabs.get(stack.active) {
                    egui::ScrollArea::vertical().id_salt(("panel", active)).auto_shrink([false, false]).show(ui, |ui| content(ui, active));
                }
            });
        });
        // Drag the line between two stacks to share the height differently.
        if i + 1 < n {
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), handle), egui::Sense::drag());
            let resp = resp.on_hover_cursor(egui::CursorIcon::ResizeVertical);
            let col = if resp.hovered() || resp.dragged() {
                ui.visuals().widgets.hovered.bg_stroke.color
            } else {
                ui.visuals().widgets.noninteractive.bg_stroke.color
            };
            ui.painter().hline(rect.x_range(), rect.center().y, egui::Stroke::new(1.0, col));
            let dy = resp.drag_delta().y;
            if dy != 0.0 {
                let per_point = total / avail.max(1.0);
                let (a, b) = (stacks[i].weight, stacks[i + 1].weight);
                let shift = (dy * per_point).clamp(-(a - 0.1), b - 0.1);
                stacks[i].weight = a + shift;
                stacks[i + 1].weight = b - shift;
            }
        }
    }
    if dragging_tab {
        let (_, dropped) = ui.dnd_drop_zone::<Panel, ()>(egui::Frame::NONE, |ui| {
            ui.set_min_size(egui::vec2(ui.available_width(), strip));
            ui.weak("Drop here for a new group");
        });
        if let Some(p) = dropped {
            ops.moves.push((*p, Target::NewStack(side)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_once(d: &DockLayout) -> bool {
        Panel::ALL.iter().all(|&p| {
            let n = d.left.iter().chain(&d.right).filter(|s| s.tabs.contains(&p)).count() + d.floating.iter().filter(|f| f.panel == p).count();
            n <= 1
        })
    }

    #[test]
    fn close_reopen_toggle() {
        let mut d = DockLayout::default();
        assert!(Panel::ALL.iter().all(|&p| d.is_open(p)));
        d.remove(Panel::History);
        assert!(!d.is_open(Panel::History));
        // (The right dock: the Navigator, then Layers and History.)
        assert_eq!(d.right[1].tabs, vec![Panel::Layers]);
        d.toggle(Panel::History);
        assert!(d.is_open(Panel::History));
        assert_eq!(d.right.last().unwrap().tabs.last(), Some(&Panel::History));
        d.remove(Panel::Tools);
        assert_eq!(d.left.len(), 2, "empty stacks disappear");
        d.show(Panel::Tools);
        assert!(all_once(&d));
    }

    #[test]
    fn moving_and_floating() {
        let mut d = DockLayout::default();
        // History onto the Colour stack.
        d.move_to(Panel::History, Target::Stack(Side::Left, 0));
        assert_eq!(d.left[0].tabs, vec![Panel::Color, Panel::History]);
        assert_eq!(d.left[0].active, 1);
        // Layers was alone in its stack now; moving it into a left stack leaves the Navigator.
        d.move_to(Panel::Layers, Target::Stack(Side::Left, 2));
        assert_eq!(d.right.len(), 1);
        assert_eq!(d.left[2].tabs, vec![Panel::ToolOptions, Panel::Layers]);
        // Moving the only tab of a stack onto a later stack whose index shifts.
        let mut e = DockLayout::default();
        e.move_to(Panel::Color, Target::Stack(Side::Left, 2));
        assert_eq!(e.left.len(), 2);
        assert_eq!(e.left[1].tabs, vec![Panel::ToolOptions, Panel::Color]);
        // Float and dock back.
        d.float(Panel::Color, [10.0, 20.0]);
        assert!(d.floating.iter().any(|f| f.panel == Panel::Color));
        assert!(!d.right.iter().any(|s| s.tabs.contains(&Panel::Color)));
        d.move_to(Panel::Color, Target::NewStack(Side::Right));
        assert!(d.floating.is_empty());
        assert_eq!(d.right.last().unwrap().tabs, vec![Panel::Color]);
        assert!(all_once(&d) && all_once(&e));
    }

    /// A layout saved before the Navigator gets it on top of the right dock, and nothing else
    /// moves; the default has it there.
    #[test]
    fn the_navigator_joins_an_older_layout_once() {
        let mut old = DockLayout::default();
        old.remove(Panel::Navigator);
        old.version = 3;
        old.left_width = 250.0;
        old.move_to(Panel::History, Target::Stack(Side::Left, 0));
        let before = old.clone();
        old.sanitize();
        assert_eq!(old.right[0].tabs, vec![Panel::Navigator]);
        assert_eq!((old.left.clone(), old.left_width, old.right[1..].to_vec()), (before.left, 250.0, before.right));
        // Closed by the user afterwards: it stays closed.
        old.remove(Panel::Navigator);
        old.sanitize();
        assert!(!old.is_open(Panel::Navigator));
        assert_eq!(DockLayout::default().right[0].tabs, vec![Panel::Navigator]);
    }

    #[test]
    fn active_tab_stays_valid() {
        let mut d = DockLayout::default();
        d.right[1].active = 1; // History
        d.remove(Panel::Layers);
        assert_eq!(d.right[1].tabs[d.right[1].active], Panel::History);
        d.remove(Panel::History);
        assert_eq!(d.right.len(), 1, "only the Navigator's stack is left");
    }

    #[test]
    fn sanitize_repairs_saved_layouts() {
        let mut d = DockLayout {
            version: LAYOUT_VERSION,
            left: vec![Stack { tabs: vec![Panel::Tools, Panel::Tools], active: 7, weight: f32::NAN }, Stack { tabs: vec![], active: 0, weight: 1.0 }],
            right: vec![Stack { tabs: vec![Panel::Tools, Panel::Layers], active: 0, weight: 99.0 }],
            floating: vec![
                Floating { panel: Panel::Layers, pos: [0.0, 0.0], size: [1.0, 1e9] },
                Floating { panel: Panel::Color, pos: [f32::NAN, 0.0], size: [100.0, 100.0] },
            ],
            left_width: f32::NAN,
            right_width: 5000.0,
        };
        d.sanitize();
        assert_eq!(d.left, vec![Stack { tabs: vec![Panel::Tools], active: 0, weight: 1.0 }]);
        assert_eq!(d.right, vec![Stack { tabs: vec![Panel::Layers], active: 0, weight: 10.0 }]);
        assert!(d.floating.is_empty());
        assert_eq!((d.left_width, d.right_width), (LEFT_WIDTH, 800.0));
        assert!(all_once(&d));
    }

    #[test]
    fn an_older_layout_becomes_the_new_default_once() {
        // Saved before versions were kept: no "version" field.
        let old: DockLayout = serde_json::from_str(r#"{"left":[{"tabs":["Tools"],"active":0}],"right":[{"tabs":["Layers"],"active":0}],"left_width":110,"right_width":290}"#).unwrap();
        assert_eq!(old.version, 0);
        let mut d = old;
        d.sanitize();
        assert_eq!(d, DockLayout::default());
        // The standard layout as first saved gets the smaller Colour panel.
        let mut first = DockLayout::default();
        first.left[0].weight = 1.5;
        first.sanitize();
        assert_eq!(first, DockLayout::default());
        // A current layout the user changed is kept.
        d.remove(Panel::History);
        let kept = d.clone();
        d.sanitize();
        assert_eq!(d, kept);
    }
}
