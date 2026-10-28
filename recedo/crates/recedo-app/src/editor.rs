//! The editor: everything Recedo does to a painting, independent of Windows and the GPU, so
//! all of it is unit-tested on any OS. The Windows shell feeds it pen samples, keys and menu
//! actions, and shows its projection (the flattened picture) on screen.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use recedo_core::autosave::{self, Autosaver};
use recedo_core::blend::BlendMode;
use recedo_core::brush::Brush;
use recedo_core::canvas::CanvasPreset;
use recedo_core::doc::{Document, new_uuid};
use recedo_core::fill::Mask;
use recedo_core::history::History;
use recedo_core::image;
use recedo_core::paint::PaintSession;
use recedo_core::projection::Projection;
use recedo_core::retouch::{Retouch, RetouchBrush, RetouchStroke};
use recedo_core::select::{self, Combine};
use recedo_core::stabilizer::{PaintInfo, StabilizerSettings};
use recedo_core::text::{self, TextStyle};
use recedo_core::tiles::{TileKey, TiledLayer};
use recedo_core::view::View;
use recedo_core::{fill, import, kra, ops};
use recedo_tablet::{Flags, PenSample, Source};
use serde::{Deserialize, Serialize};
// Like std's, and also works in a browser.
use web_time::Instant;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tool {
    #[default]
    Brush,
    Eraser,
    Eyedropper,
    Pan,
    /// Paint bucket: fills the area of similar colour that was clicked.
    Fill,
    Line,
    Rectangle,
    Ellipse,
    /// The colour fading to transparent, from where the drag starts to where it ends.
    Gradient,
    Smudge,
    Blur,
    Sharpen,
    /// Lighten under the brush.
    Dodge,
    /// Darken under the brush.
    Burn,
    /// Saturate or desaturate under the brush (Tool options).
    Sponge,
    /// The colour's hue and saturation, keeping the light and shade under the brush.
    Recolour,
    /// Copies from where Ctrl+click set the source.
    Clone,
    /// Pushes pixels along with the brush (liquify).
    Push,
    Spray,
    /// Erases the connected area of similar colour that was clicked.
    MagicEraser,
    /// Fills a freehand outline.
    LassoFill,
    SelectRect,
    SelectEllipse,
    SelectLasso,
    MagicWand,
    /// Moves the selected pixels, or the whole layer.
    Move,
    Crop,
    Text,
    Zoom,
    Rotate,
    Measure,
    /// Draw around part of the selected layer, then scale or move that part.
    LassoTransform,
}

/// The groups the full tool list is shown in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolGroup {
    Paint,
    Retouch,
    FillAndShapes,
    Select,
    Arrange,
    View,
}

impl ToolGroup {
    pub const ALL: [ToolGroup; 6] = [ToolGroup::Paint, ToolGroup::Retouch, ToolGroup::FillAndShapes, ToolGroup::Select, ToolGroup::Arrange, ToolGroup::View];

    pub fn name(self) -> &'static str {
        match self {
            ToolGroup::Paint => "Paint",
            ToolGroup::Retouch => "Retouch",
            ToolGroup::FillAndShapes => "Fill and shapes",
            ToolGroup::Select => "Select",
            ToolGroup::Arrange => "Move, crop and text",
            ToolGroup::View => "View and measure",
        }
    }
}

impl Tool {
    /// Every tool, group by group.
    pub const ALL: [Tool; 32] = [
        Tool::Brush,
        Tool::Eraser,
        Tool::Spray,
        Tool::Eyedropper,
        Tool::Smudge,
        Tool::Blur,
        Tool::Sharpen,
        Tool::Dodge,
        Tool::Burn,
        Tool::Sponge,
        Tool::Recolour,
        Tool::Clone,
        Tool::Push,
        Tool::Fill,
        Tool::Gradient,
        Tool::LassoFill,
        Tool::MagicEraser,
        Tool::Line,
        Tool::Rectangle,
        Tool::Ellipse,
        Tool::SelectRect,
        Tool::SelectEllipse,
        Tool::SelectLasso,
        Tool::MagicWand,
        Tool::Move,
        Tool::LassoTransform,
        Tool::Crop,
        Tool::Text,
        Tool::Pan,
        Tool::Zoom,
        Tool::Rotate,
        Tool::Measure,
    ];

    /// The tools on the side bar until the user picks their own.
    pub const DEFAULT_BAR: [Tool; 16] = [
        Tool::Brush,
        Tool::Eraser,
        Tool::Eyedropper,
        Tool::Fill,
        Tool::Gradient,
        Tool::Smudge,
        Tool::Blur,
        Tool::Line,
        Tool::Rectangle,
        Tool::Ellipse,
        Tool::SelectRect,
        Tool::MagicWand,
        Tool::Move,
        Tool::LassoTransform,
        Tool::Text,
        Tool::Pan,
    ];

    /// The side bar's standard tools before the lasso transform came (settings still holding
    /// exactly these get the new standard set).
    pub const OLD_DEFAULT_BAR: [&'static str; 15] =
        ["brush", "eraser", "eyedropper", "fill", "gradient", "smudge", "blur", "line", "rectangle", "ellipse", "select_rect", "magic_wand", "move", "text", "pan"];

    pub fn name(self) -> &'static str {
        match self {
            Tool::Brush => "Brush",
            Tool::Eraser => "Eraser",
            Tool::Eyedropper => "Eyedropper",
            Tool::Pan => "Pan",
            Tool::Fill => "Fill",
            Tool::Line => "Line",
            Tool::Rectangle => "Rectangle",
            Tool::Ellipse => "Ellipse",
            Tool::Gradient => "Gradient",
            Tool::Smudge => "Smudge",
            Tool::Blur => "Blur",
            Tool::Sharpen => "Sharpen",
            Tool::Dodge => "Dodge",
            Tool::Burn => "Burn",
            Tool::Sponge => "Sponge",
            Tool::Recolour => "Recolour",
            Tool::Clone => "Clone",
            Tool::Push => "Push",
            Tool::Spray => "Spray",
            Tool::MagicEraser => "Magic eraser",
            Tool::LassoFill => "Lasso fill",
            Tool::SelectRect => "Rectangle select",
            Tool::SelectEllipse => "Ellipse select",
            Tool::SelectLasso => "Lasso select",
            Tool::MagicWand => "Magic wand",
            Tool::Move => "Move",
            Tool::LassoTransform => "Lasso transform",
            Tool::Crop => "Crop",
            Tool::Text => "Text",
            Tool::Zoom => "Zoom",
            Tool::Rotate => "Rotate view",
            Tool::Measure => "Measure",
        }
    }

    /// The name under its icon on the narrow tool strip.
    pub fn label(self) -> &'static str {
        match self {
            Tool::SelectRect => "Box select",
            Tool::SelectEllipse => "Oval select",
            Tool::SelectLasso => "Lasso select",
            Tool::LassoTransform => "Transform",
            Tool::Eyedropper => "Picker",
            Tool::Rotate => "Rotate",
            t => t.name(),
        }
    }

    /// The name kept in the settings (never changes, unlike the shown name).
    pub fn id(self) -> &'static str {
        match self {
            Tool::Brush => "brush",
            Tool::Eraser => "eraser",
            Tool::Eyedropper => "eyedropper",
            Tool::Pan => "pan",
            Tool::Fill => "fill",
            Tool::Line => "line",
            Tool::Rectangle => "rectangle",
            Tool::Ellipse => "ellipse",
            Tool::Gradient => "gradient",
            Tool::Smudge => "smudge",
            Tool::Blur => "blur",
            Tool::Sharpen => "sharpen",
            Tool::Dodge => "dodge",
            Tool::Burn => "burn",
            Tool::Sponge => "sponge",
            Tool::Recolour => "recolour",
            Tool::Clone => "clone",
            Tool::Push => "push",
            Tool::Spray => "spray",
            Tool::MagicEraser => "magic_eraser",
            Tool::LassoFill => "lasso_fill",
            Tool::SelectRect => "select_rect",
            Tool::SelectEllipse => "select_ellipse",
            Tool::SelectLasso => "select_lasso",
            Tool::MagicWand => "magic_wand",
            Tool::Move => "move",
            Tool::LassoTransform => "lasso_transform",
            Tool::Crop => "crop",
            Tool::Text => "text",
            Tool::Zoom => "zoom",
            Tool::Rotate => "rotate",
            Tool::Measure => "measure",
        }
    }

    pub fn from_id(id: &str) -> Option<Tool> {
        Tool::ALL.into_iter().find(|t| t.id() == id)
    }

    pub fn group(self) -> ToolGroup {
        match self {
            Tool::Brush | Tool::Eraser | Tool::Spray | Tool::Eyedropper => ToolGroup::Paint,
            Tool::Smudge | Tool::Blur | Tool::Sharpen | Tool::Dodge | Tool::Burn | Tool::Sponge | Tool::Recolour | Tool::Clone | Tool::Push => ToolGroup::Retouch,
            Tool::Fill | Tool::Gradient | Tool::LassoFill | Tool::MagicEraser | Tool::Line | Tool::Rectangle | Tool::Ellipse => ToolGroup::FillAndShapes,
            Tool::SelectRect | Tool::SelectEllipse | Tool::SelectLasso | Tool::MagicWand => ToolGroup::Select,
            Tool::Move | Tool::LassoTransform | Tool::Crop | Tool::Text => ToolGroup::Arrange,
            Tool::Pan | Tool::Zoom | Tool::Rotate | Tool::Measure => ToolGroup::View,
        }
    }

    /// One line on what the tool does and how.
    pub fn hint(self) -> &'static str {
        match self {
            Tool::Brush => "Paint with the brush (presets in Tool options)",
            Tool::Eraser => "Erase with a brush",
            Tool::Spray => "Spray paint: dots of the colour over the brush area",
            Tool::Eyedropper => "Pick a colour from the picture (or Ctrl+click with a painting tool)",
            Tool::Smudge => "Drag the paint along, like a wet finger",
            Tool::Blur => "Soften what is under the brush",
            Tool::Sharpen => "Make what is under the brush crisper",
            Tool::Dodge => "Lighten what is under the brush",
            Tool::Burn => "Darken what is under the brush",
            Tool::Sponge => "Make colours more vivid or more grey (Tool options)",
            Tool::Recolour => "Give what is under the brush the colour's hue, keeping its light and shade",
            Tool::Clone => "Ctrl+click where to copy from, then paint the copy somewhere else",
            Tool::Push => "Push pixels along with the brush, like liquify",
            Tool::Fill => "Fill the area of the colour you click",
            Tool::Gradient => "Drag from where the colour is full to where it fades out",
            Tool::LassoFill => "Draw around an area to fill it with the colour",
            Tool::MagicEraser => "Erase the area of the colour you click",
            Tool::Line => "Drag a straight line; Shift for 15° steps",
            Tool::Rectangle => "Drag a rectangle; Shift for a square",
            Tool::Ellipse => "Drag an ellipse; Shift for a circle",
            Tool::SelectRect => "Drag to select a rectangle (Shift adds, Alt takes away)",
            Tool::SelectEllipse => "Drag to select an ellipse (Shift adds, Alt takes away)",
            Tool::SelectLasso => "Draw around what to select (Shift adds, Alt takes away)",
            Tool::MagicWand => "Click to select an area of one colour (Shift adds, Alt takes away)",
            Tool::Move => "Drag the selected pixels, or the whole layer; arrow keys nudge",
            Tool::LassoTransform => "Draw around part of this layer, then drag a corner to scale it (Shift keeps its shape) or inside to move it; Enter puts it down",
            Tool::Crop => "Drag over the part to keep, then Crop (Enter)",
            Tool::Text => "Click where the text goes, type it, then Place (Ctrl+Enter)",
            Tool::Pan => "Drag to move the view (or hold Space)",
            Tool::Zoom => "Click to zoom in, Alt+click to zoom out, or drag up and down",
            Tool::Rotate => "Drag to turn the view (or Shift+Space)",
            Tool::Measure => "Drag to measure a distance and an angle",
        }
    }

    /// Its keyboard shortcut, if it has one.
    pub fn key(self) -> Option<&'static str> {
        Some(match self {
            Tool::Brush => "B",
            Tool::Eraser => "E",
            Tool::Eyedropper => "P",
            Tool::Fill => "Shift+F",
            Tool::Gradient => "Shift+G",
            Tool::Line => "V",
            Tool::Smudge => "S",
            Tool::SelectRect => "R",
            Tool::SelectLasso => "L",
            Tool::MagicWand => "W",
            Tool::Crop => "C",
            Tool::Text => "T",
            Tool::Zoom => "Z",
            _ => return None,
        })
    }

    /// Dragged from one point to another (with a preview while dragging).
    pub fn is_shape(self) -> bool {
        matches!(self, Tool::Line | Tool::Rectangle | Tool::Ellipse | Tool::Gradient | Tool::SelectRect | Tool::SelectEllipse | Tool::Crop | Tool::Measure | Tool::Move)
    }

    /// The retouch brush engine behind the tool, if it is a retouch tool.
    pub fn retouch(self, sponge_saturates: bool) -> Option<Retouch> {
        Some(match self {
            Tool::Smudge => Retouch::Smudge,
            Tool::Blur => Retouch::Blur,
            Tool::Sharpen => Retouch::Sharpen,
            Tool::Dodge => Retouch::Dodge,
            Tool::Burn => Retouch::Burn,
            Tool::Sponge if sponge_saturates => Retouch::Saturate,
            Tool::Sponge => Retouch::Desaturate,
            Tool::Recolour => Retouch::Recolour,
            Tool::Clone => Retouch::Clone,
            Tool::Push => Retouch::Push,
            Tool::Spray => Retouch::Spray,
            _ => return None,
        })
    }

    /// Changes pixels by painting them (the pen's eraser end erases freehand instead).
    pub fn paints(self) -> bool {
        matches!(self.group(), ToolGroup::Paint | ToolGroup::Retouch | ToolGroup::FillAndShapes) && self != Tool::Eyedropper || self == Tool::Text
    }

    /// Paints with the colour: Ctrl+click picks one, as with the brush.
    pub fn uses_colour(self) -> bool {
        matches!(self, Tool::Brush | Tool::Eraser | Tool::Spray | Tool::Recolour | Tool::Fill | Tool::LassoFill | Tool::Text)
    }

    fn is_select(self) -> bool {
        self.group() == ToolGroup::Select
    }
}

/// Settings of the tools, kept between sessions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ToolSettings {
    pub brush: Brush,
    pub eraser: Brush,
    pub stabilizer: StabilizerSettings,
    /// Holding the pen still at the end of a nearly straight stroke makes it straight.
    pub hold_to_straighten: bool,
    /// Eyedropper: sample the visible picture (true) or only the current layer.
    pub pick_all_layers: bool,
    /// Eyedropper: average over this radius, in canvas pixels (0 = one pixel).
    pub pick_radius: f32,
    /// Fill: how different a colour may be and still be filled (0..=1).
    pub fill_tolerance: f32,
    /// Fill: find the area in the visible picture (true) or in the selected layer only.
    pub fill_all_layers: bool,
    /// Rectangle and ellipse: filled with the colour (and outlined with the brush).
    pub shape_filled: bool,
    /// The brush of the retouch tools (smudge, blur, ... spray).
    pub retouch: RetouchBrush,
    /// Sponge: more vivid (true) or more grey.
    pub sponge_saturates: bool,
    /// Selection tools: how a new selection goes with the one there is.
    pub select_mode: Combine,
    /// Magic wand and magic eraser: how different a colour may be (0..=1).
    pub wand_tolerance: f32,
    /// Magic wand and magic eraser: find the area in the visible picture (true) or the layer.
    pub wand_all_layers: bool,
    pub text: TextStyle,
}

impl Default for ToolSettings {
    fn default() -> ToolSettings {
        ToolSettings {
            brush: Brush::default(),
            eraser: Brush { size: 40.0, hardness: 0.7, ..Brush::default() },
            stabilizer: StabilizerSettings::default(),
            hold_to_straighten: true,
            pick_all_layers: true,
            pick_radius: 0.0,
            fill_tolerance: 0.12,
            fill_all_layers: true,
            shape_filled: false,
            retouch: RetouchBrush::default(),
            sponge_saturates: false,
            select_mode: Combine::Replace,
            wand_tolerance: 0.12,
            wand_all_layers: true,
            text: TextStyle::default(),
        }
    }
}

/// Smallest and largest brush size the size keys and sliders allow, in canvas pixels.
pub const MIN_BRUSH: f32 = 1.0;
pub const MAX_BRUSH: f32 = 1000.0;
pub const MAX_RECENT_COLORS: usize = 16;

/// Keyboard modifier state at the time of an input event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    /// The space bar is held (pan, with Shift rotate, with Ctrl zoom: Krita's keys).
    pub space: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Notice {
    pub kind: NoticeKind,
    pub text: String,
}

/// Krita's stabilizer leash, in screen pixels: where the brush is, where the pen is, and the
/// delay circle's radius around the pen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Leash {
    pub brush: (f64, f64),
    pub pen: (f64, f64),
    pub radius: f64,
}

/// What the screen needs after a change.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Refresh {
    #[default]
    None,
    Tiles(Vec<TileKey>),
    All,
}

/// A selection's edge as short segments (canvas pixels), or `None` when it is too
/// intricate to outline (then its bounding box is shown).
pub type Outline = Option<Vec<[CanvasPoint; 2]>>;

/// Text the text tool is writing, before it is placed.
#[derive(Clone, Debug, PartialEq)]
pub struct TextEdit {
    /// Top-left corner of the first line, canvas pixels.
    pub at: CanvasPoint,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Png,
    /// Quality 1..=100.
    Jpeg(u8),
}

enum Interaction {
    Idle,
    Stroke(Box<PaintSession>),
    /// Drag the view (space or pan tool).
    Pan { last: (f64, f64) },
    /// Rotate the view about the viewport centre.
    Rotate { last_angle: f64 },
    /// Drag up/down to zoom about where the drag started.
    Zoom { anchor: (f64, f64), last_y: f64 },
    /// Eyedropper held down.
    Pick { all_layers: bool },
    /// Dragged from a point to another (canvas pixels): a line, rectangle, ellipse, gradient,
    /// rectangle or ellipse selection, crop, measurement or move.
    Shape { tool: Tool, start: (f64, f64), end: (f64, f64) },
    /// A retouch stroke on `layer`, with the painting as it was before it (for undo and the
    /// selection).
    Retouch { stroke: Box<RetouchStroke>, before: Box<Document>, layer: usize, name: &'static str },
    /// A freehand outline being drawn (lasso fill or lasso select), canvas pixels.
    Lasso { tool: Tool, points: Vec<(f64, f64)> },
    /// The zoom tool held down: dragging zooms; a click without moving zooms a step.
    ZoomTool { anchor: (f64, f64), last_y: f64, moved: f64, out: bool },
    /// A handle of the lasso transform's box dragged; `from` is the box when the pen went
    /// down at `start` (canvas pixels).
    TransformDrag { handle: Handle, start: (f64, f64), from: Rect4, from_m: Affine },
    /// The pen is down but nothing happens until it lifts (e.g. a locked layer).
    Ignore,
}

/// How long the pen must stay still to straighten a stroke, its least length, how far it may
/// stray from straight (the larger of these screen pixels and this share of its length), and
/// how far the pen may move while "still" (screen pixels).
const STRAIGHTEN_HOLD_US: u64 = 500_000;
const STRAIGHTEN_MIN_LENGTH: f64 = 30.0;
const STRAIGHTEN_TOLERANCE: (f64, f64) = (6.0, 0.08);
const STILL_RADIUS: f64 = 3.0;

/// A brush stroke under way, as needed to straighten it.
struct StrokeTrack {
    /// The layer before the stroke.
    before: TiledLayer,
    brush: Brush,
    color: [u8; 3],
    erase: bool,
    /// Where the pen went (canvas pixels, with pressure), from the first point.
    path: Vec<PaintInfo>,
    /// Where the pen came to rest (screen pixels) and since when (µs).
    still: ((f64, f64), u64),
    /// Straightened: the stroke is a straight line from the first point, and the pen moves its
    /// end until it lifts.
    straight: bool,
}

/// A box in canvas pixels: left, top, right, bottom.
pub type Rect4 = (f64, f64, f64, f64);

/// What of the lasso transform's box is dragged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle {
    /// A corner: 0 top left, 1 top right, 2 bottom right, 3 bottom left.
    Corner(u8),
    /// The middle of a side: 0 top, 1 right, 2 bottom, 3 left.
    Side(u8),
    /// Inside: moves it.
    Inside,
    /// Just outside a corner, or the round handle above the box: turns it about its middle.
    Rotate,
    /// The middle of a side with Ctrl: slants it.
    Skew(u8),
}

/// Part of a layer lifted by the lasso transform, shown scaled into `rect` until it is put
/// down (Enter, another tool or command) or cancelled (Escape).
pub struct Transform {
    /// The layer it came from (only that one changes).
    pub layer: usize,
    /// The painting before, for undo and Escape.
    before: Box<Document>,
    /// The layer with the part taken out.
    rest: TiledLayer,
    lifted: select::Lifted,
    /// Where the part was, and where it is now (canvas pixels).
    pub src: Rect4,
    pub rect: Rect4,
    /// The outline drawn round it, where it was drawn.
    pub outline: Vec<CanvasPoint>,
    /// Tiles the part covered where it came from, and those the scaled part covers now.
    origin: HashSet<TileKey>,
    shown: HashSet<TileKey>,
    /// The box moved since the pixels were last redone (they are redone once a frame, not
    /// for every pen sample).
    pending: bool,
    /// Rotation, skew and flips on top of the box: a point of the box `p` shows at
    /// (m0·x + m1·y + m2, m3·x + m4·y + m5) on the canvas.
    pub m: Affine,
}

/// An affine map of canvas points: x' = a·x + b·y + c, y' = d·x + e·y + f.
pub type Affine = [f64; 6];

pub const IDENTITY: Affine = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0];

/// Where `m` takes `p`.
pub fn apply(m: &Affine, p: (f64, f64)) -> (f64, f64) {
    (m[0] * p.0 + m[1] * p.1 + m[2], m[3] * p.0 + m[4] * p.1 + m[5])
}

/// `a` after `b`.
fn compose(a: &Affine, b: &Affine) -> Affine {
    [
        a[0] * b[0] + a[1] * b[3],
        a[0] * b[1] + a[1] * b[4],
        a[0] * b[2] + a[1] * b[5] + a[2],
        a[3] * b[0] + a[4] * b[3],
        a[3] * b[1] + a[4] * b[4],
        a[3] * b[2] + a[4] * b[5] + a[5],
    ]
}

fn invert(m: &Affine) -> Option<Affine> {
    let det = m[0] * m[4] - m[1] * m[3];
    if det.abs() < 1e-12 || !det.is_finite() {
        return None;
    }
    let (a, b, d, e) = (m[4] / det, -m[1] / det, -m[3] / det, m[0] / det);
    Some([a, b, -(a * m[2] + b * m[5]), d, e, -(d * m[2] + e * m[5])])
}

/// The linear map `[a, b, d, e]` about `pivot`.
fn about(l: [f64; 4], pivot: (f64, f64)) -> Affine {
    [l[0], l[1], pivot.0 - l[0] * pivot.0 - l[1] * pivot.1, l[2], l[3], pivot.1 - l[2] * pivot.0 - l[3] * pivot.1]
}

fn rotation(radians: f64) -> [f64; 4] {
    let (s, c) = radians.sin_cos();
    [c, -s, s, c]
}

fn is_identity(m: &Affine) -> bool {
    m.iter().zip(IDENTITY).all(|(a, b)| (a - b).abs() < 1e-9)
}

impl Transform {
    /// The box's corners on the canvas: top left, top right, bottom right, bottom left.
    pub fn corners(&self) -> [(f64, f64); 4] {
        let (x0, y0, x1, y1) = self.rect;
        [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].map(|p| apply(&self.m, p))
    }

    /// Where a point of the original shows now (through the box and the rotation).
    pub fn to_canvas(&self, p: (f64, f64)) -> (f64, f64) {
        let (s, r) = (self.src, self.rect);
        let u = if s.2 > s.0 { (p.0 - s.0) / (s.2 - s.0) } else { 0.0 };
        let v = if s.3 > s.1 { (p.1 - s.1) / (s.3 - s.1) } else { 0.0 };
        apply(&self.m, (r.0 + u * (r.2 - r.0), r.1 + v * (r.3 - r.1)))
    }

    /// The middle of the box, on the canvas.
    pub fn centre(&self) -> (f64, f64) {
        let (x0, y0, x1, y1) = self.rect;
        apply(&self.m, ((x0 + x1) / 2.0, (y0 + y1) / 2.0))
    }

    /// How far it is turned, in degrees (-180..=180).
    pub fn angle(&self) -> f64 {
        self.m[3].atan2(self.m[0]).to_degrees()
    }

    /// Changed from how it was lifted?
    pub fn changed(&self) -> bool {
        self.rect != self.src || !is_identity(&self.m)
    }
}

/// How near (screen pixels) the pen must be to a handle to take it.
pub const HANDLE_REACH: f64 = 12.0;

#[derive(Clone, Debug)]
enum JobKind {
    Save(PathBuf),
    Export(PathBuf),
    Recovery,
}

struct Job {
    kind: JobKind,
    snapshot: Instant,
    rx: mpsc::Receiver<Result<(), String>>,
}

pub struct Editor {
    pub doc: Document,
    pub history: History,
    pub projection: Projection,
    pub view: View,
    /// The canvas area on screen (between the panels): left, top, width, height in pixels.
    pub viewport: (f64, f64, f64, f64),
    /// Index of the selected layer (bottom = 0).
    pub active: usize,
    pub tool: Tool,
    pub tools: ToolSettings,
    /// Painting colour, sRGB.
    pub color: [u8; 3],
    /// The sub colour (Clip Studio's second swatch); X swaps it with the colour.
    pub color2: [u8; 3],
    /// Colours painted with, newest first (shown as swatches in the Colour panel).
    pub recent_colors: Vec<[u8; 3]>,
    /// Keys held at the last pen sample (Shift straightens shapes).
    mods: Modifiers,
    /// Keys held when the pen went down (Shift / Alt choose how a selection combines).
    press_mods: Modifiers,
    /// The current selection and its outline, worked out once per selection.
    outline: Option<(std::sync::Arc<Mask>, Outline)>,
    /// The selected layer as it was when the current brush stroke began (to keep the stroke
    /// inside the selection).
    stroke_before: Option<TiledLayer>,
    /// The brush stroke under way, for straightening it when the pen is held still.
    track: Option<Box<StrokeTrack>>,
    /// Clone tool: where it copies from (canvas pixels), set with Ctrl+click.
    pub clone_source: Option<CanvasPoint>,
    /// Clone tool: source minus brush, fixed by the first stroke after the source was set.
    clone_offset: Option<CanvasPoint>,
    /// Text tool: the text being written and where, until it is placed.
    pub text_edit: Option<TextEdit>,
    /// Crop tool: the part to keep (x, y, width, height in canvas pixels), until confirmed.
    pub crop_rect: Option<(i32, i32, u32, u32)>,
    /// Measure tool: the last line measured (canvas pixels).
    pub measure: Option<(CanvasPoint, CanvasPoint)>,
    /// The `.kra` this painting is saved in, if any.
    pub file: Option<PathBuf>,
    /// Name shown when there is no file yet ("Untitled 1"), or the opened picture's name.
    pub untitled_name: String,
    pub autosaver: Autosaver,
    /// Identifies this painting's crash-recovery copy.
    pub doc_id: String,
    /// Where crash-recovery copies go (`None`: no autosave, e.g. in tests).
    pub recovery_dir: Option<PathBuf>,
    /// Where the pen or mouse hovers over the canvas, in screen pixels.
    pub hover: Option<(f64, f64)>,
    /// Messages for the user, oldest first; the shell shows and clears them.
    pub notices: Vec<Notice>,
    interaction: Interaction,
    /// Device that owns the current interaction.
    owner: Option<(Source, u32)>,
    dirty: HashSet<TileKey>,
    refresh_all: bool,
    fit_pending: bool,
    /// Goes up whenever pixels change (the layer thumbnails follow it).
    pub pixels_rev: u64,
    /// The lasso transform's part, while it is being scaled or moved.
    pub transform: Option<Transform>,
    /// The view just after the painting was fitted to the window, while it is still that
    /// (no pan, zoom or turn since): a new canvas area size fits it again.
    fitted: Option<View>,
    jobs: Vec<Job>,
    /// The crash-recovery copy this painting was opened from; deleted once it is saved or
    /// closed normally.
    recovered_from: Option<PathBuf>,
    /// Coalesces repeated property edits (a slider drag) into one undo step.
    last_edit: Option<(&'static str, usize)>,
    untitled_counter: u32,
}

impl Editor {
    pub fn new(doc: Document, recovery_dir: Option<PathBuf>) -> Editor {
        let projection = Projection::new(&doc);
        let active = doc.layers.len().saturating_sub(1);
        Editor {
            doc,
            history: History::default(),
            projection,
            view: View::default(),
            viewport: (0.0, 0.0, 800.0, 600.0),
            active,
            tool: Tool::Brush,
            tools: ToolSettings::default(),
            color: [0, 0, 0],
            color2: [255, 255, 255],
            recent_colors: Vec::new(),
            mods: Modifiers::default(),
            press_mods: Modifiers::default(),
            outline: None,
            stroke_before: None,
            track: None,
            clone_source: None,
            clone_offset: None,
            text_edit: None,
            crop_rect: None,
            measure: None,
            file: None,
            untitled_name: "Untitled 1".into(),
            autosaver: Autosaver::new(autosave::DEFAULT_INTERVAL),
            doc_id: new_uuid(),
            recovery_dir,
            hover: None,
            notices: Vec::new(),
            interaction: Interaction::Idle,
            owner: None,
            dirty: HashSet::new(),
            refresh_all: true,
            fit_pending: true,
            pixels_rev: 0,
            transform: None,
            fitted: None,
            jobs: Vec::new(),
            recovered_from: None,
            last_edit: None,
            untitled_counter: 1,
        }
    }

    // ----- status -----------------------------------------------------------------------

    /// "cat.kra" or "Untitled 1".
    pub fn title(&self) -> String {
        match &self.file {
            Some(p) => p.file_name().map_or_else(|| p.display().to_string(), |n| n.to_string_lossy().into_owned()),
            None => self.untitled_name.clone(),
        }
    }

    pub fn has_unsaved_changes(&self) -> bool {
        // A lasso transform not yet put down counts once it was scaled or moved.
        self.autosaver.has_unsaved_changes() || self.transform.as_ref().is_some_and(|t| t.changed())
    }

    pub fn is_busy_saving(&self) -> bool {
        self.jobs.iter().any(|j| matches!(j.kind, JobKind::Save(_) | JobKind::Export(_)))
    }

    /// Is a pen or mouse button held down (a press under way, on the canvas or not)?
    pub fn pen_held(&self) -> bool {
        self.owner.is_some()
    }

    pub fn stroke_active(&self) -> bool {
        matches!(self.interaction, Interaction::Stroke(_))
    }

    /// The stabilizer's delay leash on screen, while a stroke uses it.
    pub fn leash(&self) -> Option<Leash> {
        let Interaction::Stroke(s) = &self.interaction else { return None };
        // A straightened stroke is a line to the pen: nothing trails.
        if self.track.as_ref().is_some_and(|t| t.straight) {
            return None;
        }
        let (b, p, r) = s.leash()?;
        Some(Leash { brush: self.view.to_screen(b.x, b.y), pen: self.view.to_screen(p.x, p.y), radius: r * self.view.zoom })
    }

    /// The brush outline at the hover point: (centre, radius in screen pixels), for the tools
    /// that paint.
    pub fn brush_outline(&self) -> Option<((f64, f64), f64)> {
        let size = match self.tool {
            Tool::Brush => self.tools.brush.size,
            Tool::Eraser => self.tools.eraser.size,
            t if t.retouch(false).is_some() => self.tools.retouch.size,
            _ => return None,
        };
        let at = self.hover?;
        Some((at, (size as f64 * 0.5 * self.view.zoom).max(1.0)))
    }

    pub fn current_brush_mut(&mut self) -> &mut Brush {
        if self.tool == Tool::Eraser { &mut self.tools.eraser } else { &mut self.tools.brush }
    }

    /// X: the sub colour becomes the colour and the other way round.
    pub fn swap_colors(&mut self) {
        std::mem::swap(&mut self.color, &mut self.color2);
    }

    fn notice(&mut self, kind: NoticeKind, text: impl Into<String>) {
        self.notices.push(Notice { kind, text: text.into() });
    }

    // ----- screen -----------------------------------------------------------------------

    /// Where the canvas area is on screen; fits a newly opened painting into it. When the
    /// area changes (the window resized, panels or the reference panel opened or closed, G),
    /// a painting still fitted to the window is fitted again, and otherwise what was in the
    /// middle stays in the middle.
    pub fn set_viewport(&mut self, x: f64, y: f64, w: f64, h: f64) {
        if w < 1.0 || h < 1.0 || ![x, y, w, h].iter().all(|v| v.is_finite()) {
            return;
        }
        let old = std::mem::replace(&mut self.viewport, (x, y, w, h));
        if self.fit_pending {
            self.fit_pending = false;
            self.fit_to_window();
        } else if old != self.viewport {
            if self.fitted == Some(self.view) {
                self.fit_to_window();
            } else {
                self.view.pan(x + w / 2.0 - (old.0 + old.2 / 2.0), y + h / 2.0 - (old.1 + old.3 / 2.0));
            }
        }
    }

    /// Brings the projection up to date and says what the screen must re-upload.
    pub fn take_refresh(&mut self) -> Refresh {
        if self.transform.as_ref().is_some_and(|t| t.pending) {
            self.update_transform();
        }
        // While a stroke paints one layer, the projection keeps the layers below it combined.
        let stroke_layer = match &self.interaction {
            Interaction::Stroke(s) => Some(s.layer()),
            _ => None,
        };
        self.projection.set_stroke_layer(stroke_layer);
        if self.refresh_all {
            self.refresh_all = false;
            self.dirty.clear();
            self.projection.rebuild(&self.doc);
            return Refresh::All;
        }
        if self.dirty.is_empty() {
            return Refresh::None;
        }
        let dirty = std::mem::take(&mut self.dirty);
        Refresh::Tiles(self.projection.update(&self.doc, &dirty))
    }

    fn mark_tiles(&mut self, tiles: impl IntoIterator<Item = TileKey>) {
        self.dirty.extend(tiles);
        self.pixels_rev += 1;
    }

    fn refresh_everything(&mut self) {
        self.refresh_all = true;
        self.pixels_rev += 1;
    }

    /// After the document changed from `before`: refresh only what differs.
    fn refresh_from(&mut self, before: &Document) {
        match changed_tiles(before, &self.doc) {
            Some(tiles) => self.mark_tiles(tiles),
            None => self.refresh_everything(),
        }
    }

    /// Marks every tile a layer covers (after a change to its opacity, visibility, mode).
    fn refresh_layer(&mut self, index: usize) {
        let Some(l) = self.doc.layers.get(index) else { return };
        if l.pixels.default[3] != 0 {
            self.refresh_everything();
        } else {
            let keys: Vec<TileKey> = l.pixels.tile_keys().collect();
            self.mark_tiles(keys);
        }
    }

    // ----- pen and mouse ------------------------------------------------------------------

    fn paint_info(&self, s: &PenSample) -> PaintInfo {
        let (x, y) = self.view.to_canvas(s.x, s.y);
        let pressure = if s.flags.has(Flags::HAS_PRESSURE) { s.pressure.clamp(0.0, 1.0) } else { 1.0 };
        // Tilt is a direction on screen; turn it into one on the canvas.
        let (mut tx, mut ty) = (s.tilt_x as f64, s.tilt_y as f64);
        if self.view.angle != 0.0 || self.view.mirror {
            let (sin, cos) = self.view.angle.sin_cos();
            let (rx, ry) = (cos * tx + sin * ty, -sin * tx + cos * ty);
            tx = if self.view.mirror { -rx } else { rx };
            ty = ry;
        }
        PaintInfo { x, y, pressure, tilt_x: tx as f32, tilt_y: ty as f32, rotation: s.twist }
    }

    /// One pen or mouse sample. `over_canvas`: the point is on the canvas area, not on a
    /// panel (a stroke can only start there; once started it continues anywhere).
    pub fn pen(&mut self, s: &PenSample, over_canvas: bool, mods: Modifiers) {
        self.mods = mods;
        self.hover = over_canvas.then_some((s.x, s.y));
        let me = (s.source, s.cursor);
        if self.owner.is_some_and(|o| o != me) {
            return;
        }
        if !s.contact() {
            if self.owner.is_some() {
                self.release(s.recv_us);
            }
            return;
        }
        if self.owner.is_none() {
            self.owner = Some(me);
            if over_canvas {
                self.press(s, mods);
            } else {
                // Pressed on a panel: this press never paints, even if it slides onto the
                // canvas (e.g. dragging a slider).
                self.interaction = Interaction::Ignore;
            }
        } else {
            self.drag(s);
        }
    }

    fn press(&mut self, s: &PenSample, mods: Modifiers) {
        self.last_edit = None;
        self.press_mods = mods;
        let at = (s.x, s.y);
        let c = self.view.to_canvas(s.x, s.y);
        let tool = self.tool;
        // The eraser end of the pen erases freehand with the tools that paint.
        let eraser_end = s.flags.has(Flags::ERASER);
        self.interaction = if mods.space && mods.shift {
            Interaction::Rotate { last_angle: self.angle_from_centre(at) }
        } else if mods.space && mods.ctrl {
            Interaction::Zoom { anchor: at, last_y: s.y }
        } else if mods.space || tool == Tool::Pan {
            Interaction::Pan { last: at }
        } else if tool == Tool::Rotate {
            Interaction::Rotate { last_angle: self.angle_from_centre(at) }
        } else if tool == Tool::Zoom {
            Interaction::ZoomTool { anchor: at, last_y: s.y, moved: 0.0, out: mods.alt }
        } else if tool == Tool::Clone && mods.ctrl && !eraser_end {
            self.clone_source = Some(c);
            self.clone_offset = None;
            self.notice(NoticeKind::Info, "Clone source set: now paint where the copy goes.");
            Interaction::Ignore
        } else if tool == Tool::Eyedropper || (mods.ctrl && tool.uses_colour()) {
            let all_layers = self.tools.pick_all_layers;
            self.pick(s, all_layers);
            Interaction::Pick { all_layers }
        } else if matches!(tool, Tool::Brush | Tool::Eraser) || (eraser_end && tool.paints()) {
            self.begin_stroke(s, tool == Tool::Eraser || eraser_end)
        } else if tool == Tool::Fill {
            self.fill_at(s);
            Interaction::Ignore
        } else if tool == Tool::MagicEraser {
            self.magic_erase_at(c);
            Interaction::Ignore
        } else if tool == Tool::MagicWand {
            self.wand_at(c);
            Interaction::Ignore
        } else if tool == Tool::Text {
            self.text_at(c);
            Interaction::Ignore
        } else if tool.is_shape() {
            Interaction::Shape { tool, start: c, end: c }
        } else if tool == Tool::LassoTransform {
            match self.transform_handle_at(at, mods.ctrl) {
                Some(handle) => {
                    let t = self.transform.as_ref().expect("a handle");
                    Interaction::TransformDrag { handle, start: c, from: t.rect, from_m: t.m }
                }
                None => {
                    // Away from the box: the part goes down, and a new outline starts.
                    self.commit_transform();
                    Interaction::Lasso { tool, points: vec![c] }
                }
            }
        } else if matches!(tool, Tool::LassoFill | Tool::SelectLasso) {
            Interaction::Lasso { tool, points: vec![c] }
        } else if let Some(kind) = tool.retouch(self.tools.sponge_saturates) {
            self.begin_retouch(kind, s)
        } else {
            Interaction::Ignore
        };
    }

    /// A brush or eraser stroke starts.
    fn begin_stroke(&mut self, s: &PenSample, erase: bool) -> Interaction {
        let brush = if erase { self.tools.eraser.clone() } else { self.tools.brush.clone() };
        let info = self.paint_info(s);
        let layer = self.active;
        self.stroke_before = self.doc.selection.is_some().then(|| self.doc.layers.get(layer).map(|l| l.pixels.clone())).flatten();
        match PaintSession::begin(&mut self.doc, &mut self.history, layer, brush, self.color, erase, self.tools.stabilizer, self.view.zoom, info, s.recv_us) {
            Ok(mut session) => {
                if !erase {
                    self.remember_color();
                }
                if self.tools.hold_to_straighten
                    && let Some(l) = self.doc.layers.get(layer)
                {
                    let brush = if erase { self.tools.eraser.clone() } else { self.tools.brush.clone() };
                    self.track = Some(Box::new(StrokeTrack { before: l.pixels.clone(), brush, color: self.color, erase, path: vec![info], still: ((s.x, s.y), s.recv_us), straight: false }));
                }
                let d = session.take_dirty();
                self.clip_stroke(layer, &d);
                self.mark_tiles(d);
                Interaction::Stroke(Box::new(session))
            }
            Err(e) => {
                self.stroke_before = None;
                self.notice(NoticeKind::Warning, format!("{e}: nothing was painted."));
                Interaction::Ignore
            }
        }
    }

    /// Keeps what a brush stroke just painted inside the selection.
    fn clip_stroke(&mut self, layer: usize, tiles: &HashSet<TileKey>) {
        if let (Some(sel), Some(before), Some(l)) = (&self.doc.selection, &self.stroke_before, self.doc.layers.get_mut(layer)) {
            select::clip_to(&mut l.pixels, before, sel, tiles.iter().copied());
        }
    }

    /// Straightens the stroke if the pen has been still long enough at the end of a stroke
    /// that is nearly straight already (a curve is never straightened).
    fn straighten_if_held(&mut self, layer: usize, now_us: u64) {
        let Some(t) = &self.track else { return };
        if t.straight || now_us.saturating_sub(t.still.1) < STRAIGHTEN_HOLD_US || t.path.len() < 2 {
            return;
        }
        let zoom = self.view.zoom;
        let (a, b) = (t.path[0], *t.path.last().expect("points"));
        let chord = (b.x - a.x).hypot(b.y - a.y) * zoom;
        if chord < STRAIGHTEN_MIN_LENGTH {
            return;
        }
        let (dx, dy) = ((b.x - a.x) * zoom / chord, (b.y - a.y) * zoom / chord);
        let worst = t.path.iter().map(|p| ((p.x - a.x) * zoom * dy - (p.y - a.y) * zoom * dx).abs()).fold(0.0, f64::max);
        if worst > STRAIGHTEN_TOLERANCE.0.max(STRAIGHTEN_TOLERANCE.1 * chord) {
            return;
        }
        if let Some(t) = &mut self.track {
            t.straight = true;
        }
        self.draw_straight(layer, b);
    }

    /// Repaints the straightened stroke: the layer as it was, then a straight line from the
    /// stroke's first point to `end`, with the pressure the hand used along the way.
    fn draw_straight(&mut self, layer: usize, end: PaintInfo) {
        let Some(t) = &self.track else { return };
        let Some(l) = self.doc.layers.get_mut(layer) else { return };
        let old = std::mem::replace(&mut l.pixels, t.before.clone());
        // The pressure at each share of the way along the hand's path.
        let mut along = vec![0.0];
        for w in t.path.windows(2) {
            along.push(along.last().expect("one") + (w[1].x - w[0].x).hypot(w[1].y - w[0].y));
        }
        let total = along.last().copied().unwrap_or(0.0).max(1e-9);
        let pressure_at = |f: f64| {
            let i = along.partition_point(|&d| d < f * total).min(t.path.len() - 1);
            t.path[i].pressure
        };
        let a = t.path[0];
        let mut painter = recedo_core::brush::StrokePainter::new(t.brush.clone(), t.color, t.erase);
        let mut dirty = HashSet::new();
        let steps = 32;
        for i in 0..=steps {
            let f = i as f64 / steps as f64;
            let mut p = a;
            p.x = a.x + (end.x - a.x) * f;
            p.y = a.y + (end.y - a.y) * f;
            p.pressure = pressure_at(f);
            painter.add(&mut l.pixels, p, &mut dirty);
        }
        // Every tile the freehand stroke or the line touched.
        let mut tiles: HashSet<TileKey> = HashSet::new();
        for k in old.tile_keys().chain(l.pixels.tile_keys()) {
            let same = match (old.tile(k), l.pixels.tile(k)) {
                (Some(x), Some(y)) => std::sync::Arc::ptr_eq(x, y),
                (None, None) => true,
                _ => false,
            };
            if !same {
                tiles.insert(k);
            }
        }
        self.clip_stroke(layer, &tiles);
        self.mark_tiles(tiles);
    }

    /// A retouch stroke starts (smudge, blur, ... spray).
    fn begin_retouch(&mut self, kind: Retouch, s: &PenSample) -> Interaction {
        if let Some(why) = self.layer_refusal() {
            self.notice(NoticeKind::Warning, format!("{why}: nothing was changed."));
            return Interaction::Ignore;
        }
        let c = self.view.to_canvas(s.x, s.y);
        let offset = if kind == Retouch::Clone {
            let Some(src) = self.clone_source else {
                self.notice(NoticeKind::Info, "Clone: first Ctrl+click where to copy from.");
                return Interaction::Ignore;
            };
            // The first stroke after choosing the source fixes the distance; later strokes
            // keep it (Krita's "aligned" clone).
            *self.clone_offset.get_or_insert((src.0 - c.0, src.1 - c.1))
        } else {
            (0.0, 0.0)
        };
        let before = Box::new(self.doc.clone());
        let (w, h) = (self.doc.width, self.doc.height);
        let a = self.active;
        let info = self.paint_info(s);
        let mut stroke = Box::new(RetouchStroke::new(kind, self.tools.retouch, self.color, &self.doc.layers[a].pixels, (w, h), offset, s.recv_us));
        stroke.add(&mut self.doc.layers[a].pixels, info);
        let d = stroke.take_dirty();
        if let Some(sel) = &self.doc.selection {
            select::clip_to(&mut self.doc.layers[a].pixels, &before.layers[a].pixels, sel, d.iter().copied());
        }
        self.mark_tiles(d);
        if kind == Retouch::Spray {
            self.remember_color();
        }
        Interaction::Retouch { stroke, before, layer: a, name: self.tool.name() }
    }

    fn drag(&mut self, s: &PenSample) {
        let at = (s.x, s.y);
        let canvas = self.view.to_canvas(s.x, s.y);
        let shift = self.mods.shift;
        let info = self.paint_info(s);
        let angle = self.angle_from_centre(at);
        let centre = self.centre();
        let zoom = self.view.zoom;
        let mut pick = None;
        match &mut self.interaction {
            Interaction::Stroke(session) => {
                let layer = session.layer();
                if let Some(t) = &mut self.track {
                    t.path.push(info);
                    if (s.x - t.still.0.0).hypot(s.y - t.still.0.1) > STILL_RADIUS {
                        t.still = ((s.x, s.y), s.recv_us);
                    }
                }
                if self.track.as_ref().is_some_and(|t| t.straight) {
                    // Straightened: the pen moves the line's end.
                    self.draw_straight(layer, info);
                } else {
                    session.add(&mut self.doc, info, s.recv_us);
                    let d = session.take_dirty();
                    if let (Some(sel), Some(before), Some(l)) = (&self.doc.selection, &self.stroke_before, self.doc.layers.get_mut(layer)) {
                        select::clip_to(&mut l.pixels, before, sel, d.iter().copied());
                    }
                    self.mark_tiles(d);
                    self.straighten_if_held(layer, s.recv_us);
                }
            }
            Interaction::Retouch { stroke, before, layer, .. } => {
                // Its own layer, even if another one is selected meanwhile (Page Up / Down).
                let a = *layer;
                stroke.add(&mut self.doc.layers[a].pixels, info);
                let d = stroke.take_dirty();
                if let Some(sel) = &self.doc.selection {
                    select::clip_to(&mut self.doc.layers[a].pixels, &before.layers[a].pixels, sel, d.iter().copied());
                }
                self.mark_tiles(d);
            }
            Interaction::Lasso { points, .. } => {
                // A point at least a screen pixel from the last one.
                if points.last().is_none_or(|p| (p.0 - canvas.0).hypot(p.1 - canvas.1) * zoom >= 1.0) {
                    points.push(canvas);
                }
            }
            Interaction::Pan { last } => {
                let (dx, dy) = (at.0 - last.0, at.1 - last.1);
                *last = at;
                self.view.pan(dx, dy);
            }
            Interaction::Rotate { last_angle } => {
                // Shortest way round, so crossing the left side does not spin the view.
                let delta = (angle - *last_angle + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI;
                *last_angle = angle;
                self.view.rotate_by(delta, centre);
            }
            Interaction::Zoom { anchor, last_y } => {
                // 1 % per pixel dragged; up zooms in.
                let factor = 1.01f64.powf(*last_y - s.y);
                *last_y = s.y;
                let anchor = *anchor;
                self.view.zoom_by(factor, anchor);
            }
            Interaction::ZoomTool { anchor, last_y, moved, .. } => {
                *moved += (s.y - *last_y).abs();
                let factor = 1.01f64.powf(*last_y - s.y);
                *last_y = s.y;
                let anchor = *anchor;
                self.view.zoom_by(factor, anchor);
            }
            Interaction::Pick { all_layers } => pick = Some(*all_layers),
            Interaction::TransformDrag { handle, start, from, from_m } => {
                let (rect, m) = drag_transform(*handle, *from, from_m, *start, canvas, shift);
                if let Some(t) = &mut self.transform {
                    t.rect = rect;
                    t.m = m;
                    t.pending = true;
                }
            }
            Interaction::Shape { tool, start, end } => {
                // Shift straightens; for selections it was the "add" key when the pen went down.
                *end = if shift && !tool.is_select() && *tool != Tool::Crop { constrain(*tool, *start, canvas) } else { canvas };
            }
            Interaction::Ignore | Interaction::Idle => {}
        }
        if let Some(all) = pick {
            self.pick(s, all);
        }
    }

    fn release(&mut self, now_us: u64) {
        let interaction = std::mem::replace(&mut self.interaction, Interaction::Idle);
        self.owner = None;
        match interaction {
            Interaction::Stroke(session) => {
                // A straightened stroke is already the line (the stabilizer's catching up
                // would paint the freehand ending over it).
                if !self.track.take().is_some_and(|t| t.straight) {
                    let layer = session.layer();
                    let d = session.end(&mut self.doc, now_us);
                    self.clip_stroke(layer, &d);
                    self.mark_tiles(d);
                }
                self.stroke_before = None;
                self.autosaver.changed(Instant::now());
            }
            Interaction::Retouch { before, name, .. } => {
                // One undo step, if the stroke changed anything.
                if !changed_tiles(&before, &self.doc).is_some_and(|t| t.is_empty()) {
                    self.history.record(name, &before);
                    self.autosaver.changed(Instant::now());
                }
            }
            Interaction::Shape { tool, start, end } => self.commit_shape(tool, start, end, now_us),
            Interaction::Lasso { tool, points } => self.commit_lasso(tool, points),
            Interaction::ZoomTool { anchor, moved, out, .. } if moved < 3.0 => self.view.step_zoom(!out, anchor),
            Interaction::TransformDrag { .. } => self.update_transform(),
            _ => {}
        }
    }

    /// The shape being dragged out: the tool and its two points in canvas pixels (for the
    /// preview on screen).
    pub fn shape_preview(&self) -> Option<(Tool, CanvasPoint, CanvasPoint)> {
        match &self.interaction {
            Interaction::Shape { tool, start, end } => Some((*tool, *start, *end)),
            _ => None,
        }
    }

    /// The freehand outline being drawn (lasso fill or lasso select), canvas pixels.
    pub fn lasso_preview(&self) -> Option<&[CanvasPoint]> {
        match &self.interaction {
            Interaction::Lasso { points, .. } => Some(points),
            _ => None,
        }
    }

    /// Why painting on the selected layer is not possible, if it is not.
    fn layer_refusal(&self) -> Option<String> {
        match self.doc.layers.get(self.active) {
            None => Some("No layer is selected".into()),
            Some(l) if l.locked => Some(format!("Layer \"{}\" is locked", l.name)),
            Some(l) if !l.visible => Some(format!("Layer \"{}\" is hidden", l.name)),
            Some(_) => None,
        }
    }

    /// Ends a change to the selected layer (`before` is the painting as it was) as one undo
    /// step: kept inside the selection, if there is one, and not recorded if nothing changed.
    fn finish_paint(&mut self, name: &str, before: Document) -> bool {
        let a = self.active;
        if let (Some(sel), Some(tiles)) = (&self.doc.selection, changed_tiles(&before, &self.doc)) {
            select::clip_to(&mut self.doc.layers[a].pixels, &before.layers[a].pixels, sel, tiles);
        }
        if changed_tiles(&before, &self.doc).is_some_and(|t| t.is_empty()) {
            return false;
        }
        self.history.record(name, &before);
        self.refresh_from(&before);
        self.autosaver.changed(Instant::now());
        true
    }

    /// The picture the area tools look at: the visible picture (with changes not yet on
    /// screen included) or the selected layer.
    fn reference(&mut self, all_layers: bool) -> image::Rgba8 {
        let (w, h) = (self.doc.width, self.doc.height);
        if !all_layers {
            let mut img = image::Rgba8::new(w, h);
            img.data = self.doc.layers[self.active].pixels.read_rect(0, 0, w, h);
            return img;
        }
        if self.refresh_all {
            return self.doc.flatten();
        }
        if !self.dirty.is_empty() {
            let pending = self.dirty.clone();
            self.projection.update(&self.doc, &pending);
        }
        self.projection.image.clone()
    }

    /// The paint bucket at a pen sample: fills the connected area of similar colour.
    fn fill_at(&mut self, s: &PenSample) {
        if let Some(why) = self.layer_refusal() {
            self.notice(NoticeKind::Warning, format!("{why}: nothing was filled."));
            return;
        }
        let (cx, cy) = self.view.to_canvas(s.x, s.y);
        let reference = self.reference(self.tools.fill_all_layers);
        let Some(mask) = fill::flood(&reference, cx.floor() as i32, cy.floor() as i32, self.tools.fill_tolerance, 1) else { return };
        let before = self.doc.clone();
        fill::paint_mask(&mut self.doc.layers[self.active].pixels, &mask, self.color, 255);
        self.finish_paint("Fill", before);
        self.remember_color();
    }

    /// The magic eraser: erases the connected area of similar colour.
    fn magic_erase_at(&mut self, c: CanvasPoint) {
        if let Some(why) = self.layer_refusal() {
            self.notice(NoticeKind::Warning, format!("{why}: nothing was erased."));
            return;
        }
        let reference = self.reference(self.tools.wand_all_layers);
        let Some(mask) = fill::flood(&reference, c.0.floor() as i32, c.1.floor() as i32, self.tools.wand_tolerance, 1) else { return };
        let before = self.doc.clone();
        fill::erase_mask(&mut self.doc.layers[self.active].pixels, &mask);
        self.finish_paint("Magic eraser", before);
    }

    /// The magic wand: selects the connected area of similar colour.
    fn wand_at(&mut self, c: CanvasPoint) {
        let reference = self.reference(self.tools.wand_all_layers);
        if let Some(mask) = fill::flood(&reference, c.0.floor() as i32, c.1.floor() as i32, self.tools.wand_tolerance, 0) {
            self.select_mask(mask);
        }
    }

    fn remember_color(&mut self) {
        let c = self.color;
        self.recent_colors.retain(|&r| r != c);
        self.recent_colors.insert(0, c);
        self.recent_colors.truncate(MAX_RECENT_COLORS);
    }

    /// A finished drag: paints a line, rectangle, ellipse or gradient (one undo step), or
    /// selects, sets the crop, measures or moves.
    fn commit_shape(&mut self, tool: Tool, a: (f64, f64), b: (f64, f64), now_us: u64) {
        match tool {
            Tool::SelectRect | Tool::SelectEllipse => return self.select_shape(tool, a, b),
            Tool::Crop => return self.set_crop(a, b),
            Tool::Measure => {
                self.measure = Some((a, b));
                return;
            }
            Tool::Move => return self.move_by((b.0 - a.0).round() as i32, (b.1 - a.1).round() as i32),
            _ => {}
        }
        if let Some(why) = self.layer_refusal() {
            self.notice(NoticeKind::Warning, format!("{why}: nothing was painted."));
            return;
        }
        let (w, h) = (self.doc.width, self.doc.height);
        let color = self.color;
        let before = self.doc.clone();
        if tool == Tool::Gradient {
            fill::gradient(&mut self.doc.layers[self.active].pixels, w, h, a, b, color);
            self.finish_paint("Gradient", before);
            self.remember_color();
            return;
        }
        let tiny = (a.0 - b.0).abs() < 0.5 && (a.1 - b.1).abs() < 0.5;
        if tiny && tool != Tool::Line {
            return;
        }
        if self.tools.shape_filled && tool != Tool::Line {
            let mask = if tool == Tool::Rectangle { fill::rectangle(w, h, a, b) } else { fill::ellipse(w, h, a, b) };
            fill::paint_mask(&mut self.doc.layers[self.active].pixels, &mask, color, 255);
        }
        // The outline, with the brush at full pressure.
        let pt = |(x, y): (f64, f64)| PaintInfo { x, y, pressure: 1.0, tilt_x: 0.0, tilt_y: 0.0, rotation: 0.0 };
        let points: Vec<PaintInfo> = match tool {
            Tool::Line => vec![pt(a), pt(b)],
            Tool::Rectangle => vec![pt(a), pt((b.0, a.1)), pt(b), pt((a.0, b.1)), pt(a)],
            _ => {
                let (cx, cy, rx, ry) = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0, (a.0 - b.0).abs() / 2.0, (a.1 - b.1).abs() / 2.0);
                let n = ((rx + ry) * std::f64::consts::PI / 3.0).clamp(24.0, 2000.0) as usize;
                (0..=n).map(|i| i as f64 / n as f64 * std::f64::consts::TAU).map(|t| pt((cx + rx * t.cos(), cy + ry * t.sin()))).collect()
            }
        };
        let mut scratch = History::new(1);
        let brush = self.tools.brush.clone();
        if let Ok(mut session) = PaintSession::begin(&mut self.doc, &mut scratch, self.active, brush, color, false, StabilizerSettings::default(), self.view.zoom, points[0], now_us) {
            for p in &points[1..] {
                session.add(&mut self.doc, *p, now_us);
            }
            session.end(&mut self.doc, now_us);
        }
        if self.finish_paint(tool.name(), before) {
            self.remember_color();
        }
    }

    /// A finished freehand outline: fills it (lasso fill) or selects it (lasso select).
    fn commit_lasso(&mut self, tool: Tool, points: Vec<CanvasPoint>) {
        if points.len() < 3 {
            return;
        }
        if tool == Tool::LassoTransform {
            return self.start_transform(points);
        }
        let mask = fill::polygons(self.doc.width, self.doc.height, &[points]);
        if tool == Tool::SelectLasso {
            return self.select_mask(mask);
        }
        if let Some(why) = self.layer_refusal() {
            self.notice(NoticeKind::Warning, format!("{why}: nothing was filled."));
            return;
        }
        let before = self.doc.clone();
        fill::paint_mask(&mut self.doc.layers[self.active].pixels, &mask, self.color, 255);
        if self.finish_paint("Lasso fill", before) {
            self.remember_color();
        }
    }

    // ----- lasso transform --------------------------------------------------------------

    /// A finished outline with the lasso transform: lifts what it surrounds on the selected
    /// layer, to be scaled or moved.
    fn start_transform(&mut self, points: Vec<CanvasPoint>) {
        if let Some(why) = self.layer_refusal() {
            self.notice(NoticeKind::Warning, format!("{why}: nothing to transform."));
            return;
        }
        let (w, h) = (self.doc.width, self.doc.height);
        let mask = fill::polygons(w, h, std::slice::from_ref(&points));
        let a = self.active;
        let (rest, lifted) = select::lift(&self.doc.layers[a].pixels, &mask);
        if lifted.tile_count() == 0 || mask.is_empty() {
            let name = self.doc.layers[a].name.clone();
            self.notice(NoticeKind::Info, format!("Nothing there to transform: draw around some paint on layer \"{name}\"."));
            return;
        }
        let (x0, y0, x1, y1) = mask.bounds;
        let src = (x0 as f64, y0 as f64, x1 as f64, y1 as f64);
        let origin: HashSet<TileKey> = lifted.tile_keys().collect();
        let lifted = select::Lifted::new(&lifted, (x0 as i32, y0 as i32, x1 as i32, y1 as i32));
        self.transform = Some(Transform { layer: a, before: Box::new(self.doc.clone()), rest, lifted, src, rect: src, outline: points, shown: origin.clone(), origin, pending: false, m: IDENTITY });
        self.update_transform();
        self.notice(NoticeKind::Info, "Drag a corner or side to scale (Shift keeps the shape), inside to move, just outside a corner to turn it, Ctrl + a side to slant it. Enter puts it down, Escape cancels.");
    }

    /// Shows the part scaled into its box on its layer.
    fn update_transform(&mut self) {
        let size = (self.doc.width, self.doc.height);
        let Some(t) = &mut self.transform else { return };
        t.pending = false;
        let floating = if is_identity(&t.m) {
            t.lifted.scaled(t.rect, size)
        } else {
            // Canvas point -> point of the box -> point of the original.
            let (s, r) = (t.src, t.rect);
            let (kx, ky) = ((s.2 - s.0) / (r.2 - r.0).max(1e-9), (s.3 - s.1) / (r.3 - r.1).max(1e-9));
            let to_box = invert(&t.m).unwrap_or(IDENTITY);
            let to_src = compose(&[kx, 0.0, s.0 - r.0 * kx, 0.0, ky, s.1 - r.1 * ky], &to_box);
            let c = t.corners();
            let xs = c.map(|p| p.0);
            let ys = c.map(|p| p.1);
            let min = |v: [f64; 4]| v.into_iter().fold(f64::INFINITY, f64::min);
            let max = |v: [f64; 4]| v.into_iter().fold(f64::NEG_INFINITY, f64::max);
            t.lifted.mapped(to_src, (min(xs) - 1.0, min(ys) - 1.0, max(xs) + 1.0, max(ys) + 1.0), size)
        };
        let mut pixels = t.rest.clone();
        select::put_down(&mut pixels, &floating);
        let now: HashSet<TileKey> = floating.tile_keys().collect();
        // Where it was shown, and where it is now (the first time: where it came from).
        let dirty: Vec<TileKey> = t.shown.union(&now).copied().collect();
        t.shown = now;
        let layer = t.layer;
        if let Some(l) = self.doc.layers.get_mut(layer) {
            l.pixels = pixels;
        }
        self.mark_tiles(dirty);
    }

    /// Puts the lasso transform's part down where it is shown: one undo step. Unchanged, the
    /// layer is left exactly as it was.
    pub fn commit_transform(&mut self) {
        let Some(t) = self.transform.take() else { return };
        if !t.changed() {
            self.restore_layer(&t);
            return;
        }
        self.history.record("Transform", &t.before);
        self.last_edit = None;
        self.autosaver.changed(Instant::now());
    }

    /// Gives the lasso transform's part `percent` of its original width and height, about the
    /// middle of its box.
    pub fn scale_transform(&mut self, percent: f64) {
        let Some(t) = &mut self.transform else { return };
        let k = (percent / 100.0).max(0.01);
        let (cx, cy) = ((t.rect.0 + t.rect.2) / 2.0, (t.rect.1 + t.rect.3) / 2.0);
        let (w, h) = (((t.src.2 - t.src.0) * k).max(1.0), ((t.src.3 - t.src.1) * k).max(1.0));
        t.rect = (cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0);
        self.update_transform();
    }

    /// Flips the lasso transform's part left to right (`horizontal`) or upside down, in its own
    /// frame (a turned part flips along its own sides).
    pub fn flip_transform(&mut self, horizontal: bool) {
        let Some(t) = &mut self.transform else { return };
        let (x0, y0, x1, y1) = t.rect;
        let l = if horizontal { [-1.0, 0.0, 0.0, 1.0] } else { [1.0, 0.0, 0.0, -1.0] };
        t.m = compose(&t.m, &about(l, ((x0 + x1) / 2.0, (y0 + y1) / 2.0)));
        self.update_transform();
    }

    /// Turns the lasso transform's part by `degrees` (clockwise on screen) about its middle.
    pub fn rotate_transform(&mut self, degrees: f64) {
        let Some(t) = &mut self.transform else { return };
        t.m = compose(&about(rotation(degrees.to_radians()), t.centre()), &t.m);
        self.update_transform();
    }

    /// Turns the lasso transform's part to `degrees`.
    pub fn set_transform_angle(&mut self, degrees: f64) {
        let Some(t) = &self.transform else { return };
        let by = degrees - t.angle();
        self.rotate_transform(by);
    }

    /// Escape: the part goes back as it was.
    pub fn cancel_transform(&mut self) {
        if let Some(t) = self.transform.take() {
            self.restore_layer(&t);
        }
    }

    fn restore_layer(&mut self, t: &Transform) {
        if let (Some(l), Some(b)) = (self.doc.layers.get_mut(t.layer), t.before.layers.get(t.layer)) {
            l.pixels = b.pixels.clone();
        }
        let tiles: Vec<TileKey> = t.shown.union(&t.origin).copied().collect();
        self.mark_tiles(tiles);
    }

    /// The handle of the lasso transform's box under a screen point, if any.
    fn transform_handle_at(&self, at: (f64, f64), ctrl: bool) -> Option<Handle> {
        let t = self.transform.as_ref()?;
        let (x0, y0, x1, y1) = t.rect;
        let dist = |c: (f64, f64)| {
            let s = self.view.to_screen(c.0, c.1);
            (s.0 - at.0).hypot(s.1 - at.1)
        };
        let near = |c: (f64, f64)| dist(c) <= HANDLE_REACH;
        if self.transform_rotate_knob().is_some_and(|k| (k.0 - at.0).hypot(k.1 - at.1) <= HANDLE_REACH) {
            return Some(Handle::Rotate);
        }
        let corners = t.corners();
        if let Some(i) = corners.into_iter().position(near) {
            return Some(Handle::Corner(i as u8));
        }
        let (mx, my) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        let sides = [(mx, y0), (x1, my), (mx, y1), (x0, my)].map(|p| apply(&t.m, p));
        if let Some(i) = sides.into_iter().position(near) {
            return Some(if ctrl { Handle::Skew(i as u8) } else { Handle::Side(i as u8) });
        }
        let c = self.view.to_canvas(at.0, at.1);
        let inside = invert(&t.m).map(|inv| apply(&inv, c)).is_some_and(|p| p.0 >= x0 && p.0 <= x1 && p.1 >= y0 && p.1 <= y1);
        if inside {
            return Some(Handle::Inside);
        }
        // Just outside a corner: turn it (as in Clip Studio and Krita).
        corners.into_iter().any(|c| dist(c) <= HANDLE_REACH * 2.5).then_some(Handle::Rotate)
    }

    /// The round handle above the lasso transform's box (screen pixels), for turning it.
    pub fn transform_rotate_knob(&self) -> Option<(f64, f64)> {
        let t = self.transform.as_ref()?;
        let (x0, y0, x1, _) = t.rect;
        let top = apply(&t.m, ((x0 + x1) / 2.0, y0));
        let (ts, cs) = (self.view.to_screen(top.0, top.1), self.view.to_screen(t.centre().0, t.centre().1));
        let (dx, dy) = (ts.0 - cs.0, ts.1 - cs.1);
        let len = dx.hypot(dy);
        // Straight up from the box's top edge, 26 pixels out.
        let (ux, uy) = if len > 1e-6 { (dx / len, dy / len) } else { (0.0, -1.0) };
        Some((ts.0 + ux * 26.0, ts.1 + uy * 26.0))
    }

    // ----- selection ------------------------------------------------------------------------

    pub fn has_selection(&self) -> bool {
        self.doc.selection.is_some()
    }

    /// The selection's outline (canvas pixels) for the screen: `Some(None)` when the selection
    /// is too intricate to outline. Call [`Editor::update_outline`] once per frame first.
    pub fn selection_outline(&self) -> Option<&Outline> {
        self.outline.as_ref().map(|(_, o)| o)
    }

    /// Works the outline out again if the selection changed since.
    pub fn update_outline(&mut self) {
        match &self.doc.selection {
            None => self.outline = None,
            Some(sel) => {
                if !self.outline.as_ref().is_some_and(|(m, _)| std::sync::Arc::ptr_eq(m, sel)) {
                    self.outline = Some((sel.clone(), select::outline(sel, 40_000)));
                }
            }
        }
    }

    /// Replaces the selection (one undo step, as in Krita).
    fn set_selection(&mut self, name: &str, mask: Option<Mask>) {
        let mask = mask.map(std::sync::Arc::new);
        if mask == self.doc.selection {
            return;
        }
        self.edit_now(name, |doc, _| {
            doc.selection = mask;
            Ok(())
        });
    }

    /// A new selected area, combined with the selection there is: Shift (when the pen went
    /// down) adds, Alt takes away, both keep only the overlap; otherwise the tool's mode.
    fn select_mask(&mut self, mask: Mask) {
        let mode = match (self.press_mods.shift, self.press_mods.alt) {
            (true, true) => Combine::Intersect,
            (true, false) => Combine::Add,
            (false, true) => Combine::Subtract,
            _ => self.tools.select_mode,
        };
        let new = select::combine(self.doc.selection.as_deref(), mask, mode);
        self.set_selection(if new.is_some() { "Select" } else { "Deselect" }, new);
    }

    fn select_shape(&mut self, tool: Tool, a: CanvasPoint, b: CanvasPoint) {
        let (w, h) = (self.doc.width, self.doc.height);
        // A click without dragging clears the selection, as in Krita.
        if (a.0 - b.0).abs() < 1.0 && (a.1 - b.1).abs() < 1.0 {
            if self.tools.select_mode == Combine::Replace && !self.press_mods.shift && !self.press_mods.alt {
                self.deselect();
            }
            return;
        }
        let mask = if tool == Tool::SelectRect { fill::rectangle(w, h, a, b) } else { fill::ellipse(w, h, a, b) };
        self.select_mask(mask);
    }

    /// Ctrl+A.
    pub fn select_all(&mut self) {
        self.finish_interaction();
        let all = select::all(self.doc.width, self.doc.height);
        self.set_selection("Select all", Some(all));
    }

    /// Ctrl+Shift+A.
    pub fn deselect(&mut self) {
        self.finish_interaction();
        self.set_selection("Deselect", None);
    }

    /// Ctrl+Shift+I.
    pub fn invert_selection(&mut self) {
        self.finish_interaction();
        let inverted = match &self.doc.selection {
            Some(m) => select::invert(m),
            None => {
                // Said, not silent: otherwise the key looks broken.
                self.notice(NoticeKind::Info, "Nothing is selected, so there is nothing to invert. Select something first.");
                return;
            }
        };
        self.set_selection("Invert selection", inverted);
    }

    /// Fills the selection (or the whole layer) with the colour: Backspace.
    pub fn fill_selection(&mut self) {
        self.finish_interaction();
        if let Some(why) = self.layer_refusal() {
            self.notice(NoticeKind::Warning, format!("{why}: nothing was filled."));
            return;
        }
        let (w, h) = (self.doc.width, self.doc.height);
        let area = self.doc.selection.as_deref().cloned().unwrap_or_else(|| select::all(w, h));
        let before = self.doc.clone();
        fill::paint_mask(&mut self.doc.layers[self.active].pixels, &area, self.color, 255);
        if self.finish_paint("Fill with colour", before) {
            self.remember_color();
        }
    }

    /// Crops the canvas to the selection's bounding box.
    pub fn crop_to_selection(&mut self) {
        let Some((x0, y0, x1, y1)) = self.doc.selection.as_ref().map(|m| m.bounds) else { return };
        self.crop_rect = Some((x0 as i32, y0 as i32, x1 - x0, y1 - y0));
        self.apply_crop();
    }

    /// Copies (or with `cut`, moves) the selected pixels to a new layer above.
    pub fn selection_to_layer(&mut self, cut: bool) {
        self.finish_interaction();
        let Some(sel) = self.doc.selection.clone() else { return };
        if cut && let Some(why) = self.layer_refusal() {
            self.notice(NoticeKind::Warning, format!("{why}: nothing was cut."));
            return;
        }
        let name = if cut { "Cut to new layer" } else { "Copy to new layer" };
        self.edit(name, |doc, active| {
            let src = doc.layers.get(*active).ok_or("No layer is selected")?;
            let (rest, lifted) = select::lift(&src.pixels, &sel);
            let base = format!("{} (part)", src.name);
            if cut {
                doc.layers[*active].pixels = rest;
            }
            *active = ops::add_layer(doc, Some(*active));
            doc.layers[*active].pixels = lifted;
            doc.layers[*active].name = base;
            Ok(())
        });
    }

    // ----- move, crop, text -----------------------------------------------------------------

    /// Moves the selected pixels (and the selection) or, without a selection, the whole
    /// layer, by whole pixels.
    pub fn move_by(&mut self, dx: i32, dy: i32) {
        if dx == 0 && dy == 0 {
            return;
        }
        if let Some(why) = self.layer_refusal() {
            self.notice(NoticeKind::Warning, format!("{why}: nothing was moved."));
            return;
        }
        let sel = self.doc.selection.clone();
        self.edit_now("Move", |doc, active| {
            let l = &mut doc.layers[*active].pixels;
            match &sel {
                Some(m) => {
                    *l = select::move_selected(l, m, dx, dy);
                    let moved = select::offset_mask(m, dx, dy);
                    doc.selection = (!moved.is_empty()).then(|| std::sync::Arc::new(moved));
                }
                None => *l = select::offset_layer(l, dx, dy),
            }
            Ok(())
        });
    }

    /// The arrow keys with the move tool.
    pub fn nudge(&mut self, dx: i32, dy: i32) {
        self.finish_interaction();
        self.move_by(dx, dy);
    }

    fn set_crop(&mut self, a: CanvasPoint, b: CanvasPoint) {
        let (w, h) = (self.doc.width as f64, self.doc.height as f64);
        let (x0, x1) = (a.0.min(b.0).clamp(0.0, w).round(), a.0.max(b.0).clamp(0.0, w).round());
        let (y0, y1) = (a.1.min(b.1).clamp(0.0, h).round(), a.1.max(b.1).clamp(0.0, h).round());
        self.crop_rect = (x1 - x0 >= 1.0 && y1 - y0 >= 1.0).then_some((x0 as i32, y0 as i32, (x1 - x0) as u32, (y1 - y0) as u32));
    }

    /// Crops the canvas to the crop rectangle (Enter or the Crop button).
    pub fn apply_crop(&mut self) {
        self.finish_interaction();
        let Some((x, y, w, h)) = self.crop_rect.take() else { return };
        if (x, y, w, h) == (0, 0, self.doc.width, self.doc.height) {
            return;
        }
        // The picture stays where it is on screen.
        let keep = self.view.to_screen(x as f64, y as f64);
        self.edit("Crop", |doc, _| {
            *doc = select::crop(doc, x, y, w, h);
            Ok(())
        });
        self.view.offset = keep;
    }

    pub fn cancel_crop(&mut self) {
        self.crop_rect = None;
    }

    /// The text tool clicked at `c`: starts writing there, or moves the text being written.
    fn text_at(&mut self, c: CanvasPoint) {
        match &mut self.text_edit {
            Some(t) => t.at = c,
            None => self.text_edit = Some(TextEdit { at: c, text: String::new() }),
        }
    }

    /// Paints the text being written onto the selected layer (Place, or Ctrl+Enter).
    pub fn place_text(&mut self) {
        self.finish_interaction();
        let Some(mut t) = self.text_edit.clone() else { return };
        // An Enter typed with the Ctrl+Enter that places the text is not a line.
        t.text.truncate(t.text.trim_end_matches(['\n', '\r']).len());
        if t.text.trim().is_empty() {
            self.text_edit = None;
            return;
        }
        if let Some(why) = self.layer_refusal() {
            self.notice(NoticeKind::Warning, format!("{why}: the text was not placed."));
            return;
        }
        let mask = text::render(self.doc.width, self.doc.height, &t.text, &self.tools.text, t.at);
        let before = self.doc.clone();
        fill::paint_mask(&mut self.doc.layers[self.active].pixels, &mask, self.color, 255);
        if self.finish_paint("Text", before) {
            self.remember_color();
        }
        self.text_edit = None;
    }

    pub fn cancel_text(&mut self) {
        self.text_edit = None;
    }

    /// The clone tool forgets where it copies from (the next Ctrl+click sets it again).
    pub fn forget_clone_source(&mut self) {
        self.clone_source = None;
        self.clone_offset = None;
    }

    /// The pen left the tablet's range or the window lost the pen: ends what it was doing.
    pub fn pen_gone(&mut self, source: Source, now_us: u64) {
        if self.owner.is_some_and(|(s, _)| s == source) {
            self.release(now_us);
        }
        self.hover = None;
    }

    /// Call every frame: the stabilizer keeps moving the brush towards a pen held still,
    /// background saves finish, autosave runs when due.
    pub fn tick(&mut self, now_us: u64, now: Instant) {
        if let Interaction::Stroke(session) = &mut self.interaction
            && !self.track.as_ref().is_some_and(|t| t.straight)
        {
            session.tick(&mut self.doc, now_us);
            let d = session.take_dirty();
            let layer = session.layer();
            if let (Some(sel), Some(before), Some(l)) = (&self.doc.selection, &self.stroke_before, self.doc.layers.get_mut(layer)) {
                select::clip_to(&mut l.pixels, before, sel, d.iter().copied());
            }
            self.mark_tiles(d);
            // A pen held still sends nothing: the frames check how long it has been still.
            self.straighten_if_held(layer, now_us);
        }
        self.poll_jobs();
        let busy = self.stroke_active() || !self.jobs.is_empty();
        if let Some(dir) = self.recovery_dir.clone()
            && self.autosaver.due(now, busy)
        {
            let (tx, rx) = mpsc::channel();
            let (doc, id, original) = (self.doc.clone(), self.doc_id.clone(), self.file.clone());
            std::thread::spawn(move || {
                let _ = tx.send(autosave::write_recovery(&dir, &id, &doc, original.as_deref()).map(|_| ()));
            });
            self.jobs.push(Job { kind: JobKind::Recovery, snapshot: now, rx });
        }
    }

    /// Whether the shell should keep drawing frames without new input (stabilizer catching
    /// up, a save in progress).
    pub fn needs_ticks(&self) -> bool {
        self.stroke_active() || !self.jobs.is_empty()
    }

    fn angle_from_centre(&self, at: (f64, f64)) -> f64 {
        let c = self.centre();
        (at.1 - c.1).atan2(at.0 - c.0)
    }

    fn pick(&mut self, s: &PenSample, all_layers: bool) {
        let (x, y) = self.view.to_canvas(s.x, s.y);
        let layer = (!all_layers).then_some(self.active);
        if let Some(c) = ops::pick_color(&self.doc, layer, x, y, self.tools.pick_radius as f64) {
            self.color = [c[0], c[1], c[2]];
        }
    }

    /// Mouse wheel over the canvas: zoom about the pointer.
    pub fn wheel(&mut self, steps: f64, at: (f64, f64)) {
        if steps != 0.0 && steps.is_finite() {
            self.view.zoom_by(1.2f64.powf(steps.clamp(-10.0, 10.0)), at);
        }
    }

    // ----- view commands ------------------------------------------------------------------

    fn centre(&self) -> (f64, f64) {
        let (x, y, w, h) = self.viewport;
        (x + w / 2.0, y + h / 2.0)
    }

    pub fn zoom_in(&mut self) {
        let c = self.centre();
        self.view.step_zoom(true, c);
    }

    pub fn zoom_out(&mut self) {
        let c = self.centre();
        self.view.step_zoom(false, c);
    }

    pub fn zoom_actual_pixels(&mut self) {
        let c = self.centre();
        self.view.set_zoom_at(1.0, c);
    }

    pub fn fit_to_window(&mut self) {
        let (x, y, w, h) = self.viewport;
        self.view.fit((self.doc.width, self.doc.height), (w, h), 24.0);
        self.view.pan(x, y);
        self.fitted = Some(self.view);
    }

    pub fn rotate_by_degrees(&mut self, degrees: f64) {
        let c = self.centre();
        self.view.rotate_by(degrees.to_radians(), c);
    }

    pub fn reset_rotation(&mut self) {
        let c = self.centre();
        self.view.set_angle_at(0.0, c);
    }

    pub fn toggle_mirror(&mut self) {
        let c = self.centre();
        self.view.toggle_mirror(c);
    }

    /// `[` and `]`: about 10 % smaller or larger (the retouch brush with a retouch tool).
    pub fn resize_brush(&mut self, bigger: bool) {
        let step = |size: f32| {
            let s = if bigger { (size * 1.1).max(size + 1.0) } else { (size / 1.1).min(size - 1.0) };
            s.round().clamp(MIN_BRUSH, MAX_BRUSH)
        };
        if self.tool.retouch(false).is_some() {
            self.tools.retouch.size = step(self.tools.retouch.size);
        } else {
            let b = self.current_brush_mut();
            b.size = step(b.size);
        }
    }

    /// The size the toolbar shows and changes: the retouch brush's with a retouch tool.
    pub fn shown_brush_size(&self) -> f32 {
        if self.tool.retouch(false).is_some() {
            self.tools.retouch.size
        } else if self.tool == Tool::Eraser {
            self.tools.eraser.size
        } else {
            self.tools.brush.size
        }
    }

    /// `E`: switch between the brush and the eraser.
    pub fn toggle_eraser(&mut self) {
        self.tool = if self.tool == Tool::Eraser { Tool::Brush } else { Tool::Eraser };
    }

    // ----- undo -----------------------------------------------------------------------------

    /// Ends what the pen is doing, and puts down a lasso transform's part: commands and
    /// undo act on the painting as it shows.
    fn finish_interaction(&mut self) {
        if self.owner.is_some() {
            self.release(recedo_tablet::clock::now_us());
        }
        self.commit_transform();
    }

    pub fn undo(&mut self) {
        self.finish_interaction();
        self.last_edit = None;
        let before = self.doc.clone();
        match self.history.undo(&mut self.doc) {
            Some(name) => {
                self.after_history_move(&before);
                self.notice(NoticeKind::Info, format!("Undo: {name}"));
            }
            None => self.notice(NoticeKind::Info, "Nothing to undo"),
        }
    }

    pub fn redo(&mut self) {
        self.finish_interaction();
        self.last_edit = None;
        let before = self.doc.clone();
        match self.history.redo(&mut self.doc) {
            Some(name) => {
                self.after_history_move(&before);
                self.notice(NoticeKind::Info, format!("Redo: {name}"));
            }
            None => self.notice(NoticeKind::Info, "Nothing to redo"),
        }
    }

    /// History panel: click a step.
    pub fn jump_to_history(&mut self, applied: usize) {
        self.finish_interaction();
        self.last_edit = None;
        let before = self.doc.clone();
        self.history.jump_to(&mut self.doc, applied);
        self.after_history_move(&before);
    }

    fn after_history_move(&mut self, before: &Document) {
        if (before.width, before.height) != (self.doc.width, self.doc.height) {
            self.crop_rect = None;
            self.fit_pending = true;
            let (x, y, w, h) = self.viewport;
            self.set_viewport(x, y, w, h);
        }
        self.active = self.active.min(self.doc.layers.len().saturating_sub(1));
        self.refresh_from(before);
        self.autosaver.changed(Instant::now());
    }

    // ----- layers ---------------------------------------------------------------------------

    /// Runs a document change as one undo step named `name`.
    fn edit(&mut self, name: &str, f: impl FnOnce(&mut Document, &mut usize) -> Result<(), String>) {
        self.finish_interaction();
        self.edit_now(name, f);
    }

    /// An edit while the pen is down (the fill tool): like `edit`, without ending the pen's
    /// interaction first.
    fn edit_now(&mut self, name: &str, f: impl FnOnce(&mut Document, &mut usize) -> Result<(), String>) {
        self.last_edit = None;
        let before = self.doc.clone();
        let mut active = self.active;
        match f(&mut self.doc, &mut active) {
            Ok(()) => {
                self.history.record(name, &before);
                self.active = active.min(self.doc.layers.len().saturating_sub(1));
                self.refresh_from(&before);
                self.autosaver.changed(Instant::now());
            }
            Err(e) => {
                self.doc = before;
                self.notice(NoticeKind::Warning, e);
            }
        }
    }

    pub fn add_layer(&mut self) {
        self.edit("New layer", |doc, active| {
            *active = ops::add_layer(doc, Some(*active));
            Ok(())
        });
    }

    pub fn delete_layer(&mut self) {
        self.edit("Delete layer", |doc, active| {
            *active = ops::delete_layer(doc, *active)?;
            Ok(())
        });
    }

    pub fn duplicate_layer(&mut self) {
        self.edit("Duplicate layer", |doc, active| {
            *active = ops::duplicate_layer(doc, *active)?;
            Ok(())
        });
    }

    pub fn move_layer(&mut self, up: bool) {
        let name = if up { "Move layer up" } else { "Move layer down" };
        self.edit(name, |doc, active| {
            *active = ops::move_layer(doc, *active, up)?;
            Ok(())
        });
    }

    pub fn merge_down(&mut self) {
        self.edit("Merge down", |doc, active| {
            *active = ops::merge_down(doc, *active)?;
            Ok(())
        });
    }

    /// Delete: clears the selection's pixels on the selected layer, or the whole layer.
    pub fn clear_layer(&mut self) {
        match self.doc.selection.clone() {
            Some(sel) => {
                self.finish_interaction();
                if let Some(why) = self.layer_refusal() {
                    self.notice(NoticeKind::Warning, format!("{why}: nothing was cleared."));
                    return;
                }
                let before = self.doc.clone();
                fill::erase_mask(&mut self.doc.layers[self.active].pixels, &sel);
                self.finish_paint("Clear selection", before);
            }
            None => self.edit("Clear layer", |doc, active| ops::clear_layer(doc, *active)),
        }
    }

    pub fn select_layer(&mut self, index: usize) {
        if index < self.doc.layers.len() {
            if index != self.active {
                self.commit_transform();
            }
            self.active = index;
            self.last_edit = None;
        }
    }

    /// Selects the layer above (`up`) or below the current one (Page Up / Page Down).
    pub fn select_neighbour(&mut self, up: bool) {
        let i = if up { self.active + 1 } else { self.active.wrapping_sub(1) };
        self.select_layer(i);
    }

    /// A property change that a slider may repeat many times: one undo step per drag.
    /// Call [`Editor::end_edit`] when the drag ends.
    fn property_edit(&mut self, key: &'static str, index: usize, name: &str, f: impl FnOnce(&mut recedo_core::doc::Layer)) {
        if index >= self.doc.layers.len() {
            return;
        }
        self.finish_interaction();
        if self.last_edit != Some((key, index)) {
            self.history.record(name, &self.doc);
            self.last_edit = Some((key, index));
        }
        // Tiles of the layer before and after (a blend mode change may move nothing but
        // changes how everything it covers looks).
        self.refresh_layer(index);
        f(&mut self.doc.layers[index]);
        self.refresh_layer(index);
        self.autosaver.changed(Instant::now());
    }

    pub fn end_edit(&mut self) {
        self.last_edit = None;
    }

    pub fn set_layer_opacity(&mut self, index: usize, opacity: u8) {
        if self.doc.layers.get(index).is_some_and(|l| l.opacity != opacity) {
            self.property_edit("opacity", index, "Layer opacity", |l| l.opacity = opacity);
        }
    }

    pub fn set_layer_visible(&mut self, index: usize, visible: bool) {
        if self.doc.layers.get(index).is_some_and(|l| l.visible != visible) {
            self.property_edit("visible", index, if visible { "Show layer" } else { "Hide layer" }, |l| l.visible = visible);
            self.last_edit = None;
        }
    }

    pub fn set_layer_locked(&mut self, index: usize, locked: bool) {
        if self.doc.layers.get(index).is_some_and(|l| l.locked != locked) {
            self.property_edit("locked", index, if locked { "Lock layer" } else { "Unlock layer" }, |l| l.locked = locked);
            self.last_edit = None;
        }
    }

    pub fn set_layer_blend(&mut self, index: usize, mode: BlendMode) {
        if self.doc.layers.get(index).is_some_and(|l| l.blend != mode) {
            self.property_edit("blend", index, "Layer blend mode", |l| l.blend = mode);
            self.last_edit = None;
        }
    }

    pub fn rename_layer(&mut self, index: usize, name: &str) {
        let name = name.trim();
        if !name.is_empty() && self.doc.layers.get(index).is_some_and(|l| l.name != name) {
            let name = name.to_string();
            self.property_edit("name", index, "Rename layer", |l| l.name = name);
        }
    }

    // ----- files ----------------------------------------------------------------------------

    fn replace_document(&mut self, doc: Document) {
        self.transform = None;
        self.finish_interaction();
        self.remove_recovery_copies();
        self.doc = doc;
        self.history.clear();
        self.active = self.doc.layers.len().saturating_sub(1);
        self.text_edit = None;
        self.crop_rect = None;
        self.measure = None;
        self.clone_source = None;
        self.clone_offset = None;
        self.stroke_before = None;
        self.autosaver = Autosaver::new(self.autosaver.interval);
        self.doc_id = new_uuid();
        self.refresh_everything();
        self.fit_pending = true;
        self.view = View::default();
        self.last_edit = None;
        let (x, y, w, h) = self.viewport;
        self.set_viewport(x, y, w, h);
    }

    /// File > New. The caller has asked about unsaved changes first.
    pub fn new_document(&mut self, preset: &CanvasPreset) -> Result<(), String> {
        let doc = preset.create()?;
        self.replace_document(doc);
        self.file = None;
        self.untitled_counter += 1;
        self.untitled_name = format!("Untitled {}", self.untitled_counter);
        Ok(())
    }

    /// File > Open: a `.kra`, PNG or JPEG. Things that could not be opened become warnings.
    pub fn open(&mut self, path: &Path) -> Result<(), String> {
        let loaded = import::open_file(path)?;
        self.replace_document(loaded.doc);
        let is_kra = std::fs::read(path).ok().is_some_and(|b| import::detect(&b) == Some(import::Format::Kra));
        if is_kra {
            self.file = Some(path.to_path_buf());
        } else {
            // A PNG or JPEG becomes a new painting: Save asks where to put the .kra.
            self.file = None;
            self.untitled_name = path.file_stem().map_or_else(|| "Picture".into(), |s| s.to_string_lossy().into_owned());
        }
        for w in loaded.warnings {
            self.notice(NoticeKind::Warning, w);
        }
        Ok(())
    }

    /// Opens a file from its bytes (the browser version: a file the user picked or dropped).
    /// `name` is the file's name; the painting takes it as its title.
    // Only the browser app (recedo-web) calls this.
    #[cfg_attr(windows, allow(dead_code))]
    pub fn open_bytes(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        let loaded = import::open(bytes)?;
        self.replace_document(loaded.doc);
        self.file = None;
        self.untitled_name = Path::new(name).file_stem().map_or_else(|| "Picture".into(), |s| s.to_string_lossy().into_owned());
        for w in loaded.warnings {
            self.notice(NoticeKind::Warning, w);
        }
        Ok(())
    }

    /// The painting as a `.kra` file's bytes (the browser version saves by downloading them).
    // Only the browser app (recedo-web) calls this.
    #[cfg_attr(windows, allow(dead_code))]
    pub fn encode_kra(&mut self) -> Result<Vec<u8>, String> {
        self.finish_interaction();
        kra::save(&self.doc)
    }

    /// The flattened picture as PNG or JPEG bytes.
    // Only the browser app (recedo-web) calls this.
    #[cfg_attr(windows, allow(dead_code))]
    pub fn encode_export(&mut self, format: ExportFormat) -> Result<Vec<u8>, String> {
        self.finish_interaction();
        let flat = self.doc.flatten();
        match format {
            ExportFormat::Png => image::encode_png(&flat, false),
            ExportFormat::Jpeg(q) => image::encode_jpeg(&flat, q, [255, 255, 255], self.doc.dpi),
        }
    }

    /// The bytes from [`Editor::encode_kra`] (taken at `snapshot`) were saved as `name`.
    // Only the browser app (recedo-web) calls this.
    #[cfg_attr(windows, allow(dead_code))]
    pub fn saved_elsewhere(&mut self, name: &str, snapshot: Instant) {
        self.autosaver.saved(snapshot);
        self.untitled_name = Path::new(name).file_stem().map_or_else(|| name.to_string(), |s| s.to_string_lossy().into_owned());
        self.notice(NoticeKind::Info, format!("Saved {name}"));
    }

    /// Opens a crash-recovery copy: its pixels, unsaved, with the original's file name.
    pub fn open_recovery(&mut self, r: &autosave::Recoverable) -> Result<(), String> {
        let loaded = kra::load_file(&r.kra)?;
        self.replace_document(loaded.doc);
        self.file = r.info.original.clone();
        self.autosaver.changed(Instant::now());
        // The recovered copy stays on disk until the painting is saved or closed normally.
        self.recovered_from = Some(r.kra.clone());
        Ok(())
    }

    /// Saves to `path` as `.kra` on a background thread (the painting can go on meanwhile).
    pub fn save_as(&mut self, path: &Path) {
        self.finish_interaction();
        let path = with_extension(path, "kra");
        let (tx, rx) = mpsc::channel();
        let (doc, p) = (self.doc.clone(), path.clone());
        std::thread::spawn(move || {
            let _ = tx.send(kra::save_file(&doc, &p));
        });
        self.jobs.push(Job { kind: JobKind::Save(path), snapshot: Instant::now(), rx });
    }

    /// File > Save. `false` when the painting has no file yet (ask for one, then `save_as`).
    pub fn save(&mut self) -> bool {
        match self.file.clone() {
            Some(p) => {
                self.save_as(&p);
                true
            }
            None => false,
        }
    }

    /// File > Export: the flattened picture as PNG (with transparency) or JPEG.
    pub fn export(&mut self, path: &Path, format: ExportFormat) {
        self.finish_interaction();
        let path = with_extension(path, if matches!(format, ExportFormat::Png) { "png" } else { "jpg" });
        let (tx, rx) = mpsc::channel();
        let (doc, p) = (self.doc.clone(), path.clone());
        std::thread::spawn(move || {
            let flat = doc.flatten();
            let bytes = match format {
                ExportFormat::Png => image::encode_png(&flat, false),
                ExportFormat::Jpeg(q) => image::encode_jpeg(&flat, q, [255, 255, 255], doc.dpi),
            };
            let _ = tx.send(bytes.and_then(|b| write_atomically(&p, &b)));
        });
        self.jobs.push(Job { kind: JobKind::Export(path), snapshot: Instant::now(), rx });
    }

    /// Finishes background saves (call every frame; also from `tick`).
    pub fn poll_jobs(&mut self) {
        let mut finished = Vec::new();
        self.jobs.retain(|j| match j.rx.try_recv() {
            Ok(r) => {
                finished.push((j.kind.clone(), j.snapshot, r));
                false
            }
            Err(mpsc::TryRecvError::Empty) => true,
            Err(mpsc::TryRecvError::Disconnected) => {
                finished.push((j.kind.clone(), j.snapshot, Err("the save stopped unexpectedly".into())));
                false
            }
        });
        for (kind, snapshot, result) in finished {
            match (kind, result) {
                (JobKind::Save(path), Ok(())) => {
                    self.autosaver.saved(snapshot);
                    self.file = Some(path.clone());
                    self.remove_recovery_copies();
                    self.notice(NoticeKind::Info, format!("Saved {}", path.display()));
                }
                (JobKind::Export(path), Ok(())) => self.notice(NoticeKind::Info, format!("Exported {}", path.display())),
                (JobKind::Recovery, Ok(())) => self.autosaver.autosaved(snapshot),
                (JobKind::Save(path) | JobKind::Export(path), Err(e)) => {
                    self.notice(NoticeKind::Error, format!("Could not save {}: {e}", path.display()));
                }
                (JobKind::Recovery, Err(e)) => self.notice(NoticeKind::Warning, format!("Autosave failed: {e}")),
            }
        }
    }

    /// Windows is shutting down: writes the recovery copy now, if it lacks any change, and
    /// waits for it (an autosave that is due in a minute would never happen).
    pub fn emergency_autosave(&mut self) {
        // Saves already running may write the same files; let them finish first.
        while !self.jobs.is_empty() {
            std::thread::sleep(std::time::Duration::from_millis(10));
            self.poll_jobs();
        }
        let Some(dir) = self.recovery_dir.clone() else { return };
        if !self.autosaver.copy_is_behind() {
            return;
        }
        let snapshot = Instant::now();
        match autosave::write_recovery(&dir, &self.doc_id, &self.doc, self.file.as_deref()) {
            Ok(_) => self.autosaver.autosaved(snapshot),
            Err(e) => self.notice(NoticeKind::Warning, format!("Autosave failed: {e}")),
        }
    }

    /// Waits for background saves (before quitting).
    pub fn wait_for_saves(&mut self) {
        while self.is_busy_saving() {
            std::thread::sleep(std::time::Duration::from_millis(10));
            self.poll_jobs();
        }
    }

    /// The painting is being closed normally (after its changes were saved or discarded):
    /// its recovery copy is no longer needed.
    pub fn closing(&mut self) {
        self.wait_for_saves();
        while !self.jobs.is_empty() {
            std::thread::sleep(std::time::Duration::from_millis(10));
            self.poll_jobs();
        }
        self.remove_recovery_copies();
    }

    fn remove_recovery_copies(&mut self) {
        if let Some(dir) = &self.recovery_dir {
            autosave::remove_recovery(dir, &self.doc_id);
        }
        if let Some(kra) = self.recovered_from.take() {
            autosave::remove_recovery_file(&kra);
        }
    }
}

/// A point on the canvas, in canvas pixels.
pub type CanvasPoint = (f64, f64);

/// Shift while dragging a shape: lines and gradients at multiples of 15°, squares and circles.
/// The lasso transform's box and map after dragging `handle` from `start` to `at` (canvas
/// pixels): corners and sides scale in the part's own (turned) frame, inside moves it, the
/// rotate handle turns it about its middle (Shift: in 15° steps of its whole angle), Ctrl +
/// a side slants it (the opposite side stays put).
fn drag_transform(handle: Handle, from: Rect4, from_m: &Affine, start: (f64, f64), at: (f64, f64), shift: bool) -> (Rect4, Affine) {
    let inv = invert(from_m).unwrap_or(IDENTITY);
    let (x0, y0, x1, y1) = from;
    let mid = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    match handle {
        Handle::Inside => {
            let mut m = *from_m;
            m[2] += at.0 - start.0;
            m[5] += at.1 - start.1;
            (from, m)
        }
        Handle::Corner(_) | Handle::Side(_) => (drag_box(handle, from, apply(&inv, start), apply(&inv, at), shift), *from_m),
        Handle::Rotate => {
            let c = apply(from_m, mid);
            let mut by = (at.1 - c.1).atan2(at.0 - c.0) - (start.1 - c.1).atan2(start.0 - c.0);
            if shift {
                let was = from_m[3].atan2(from_m[0]);
                let step = 15f64.to_radians();
                by = ((was + by) / step).round() * step - was;
            }
            (from, compose(&about(rotation(by), c), from_m))
        }
        Handle::Skew(i) => {
            // The pen's movement in the part's own frame.
            let (a, b) = (apply(&inv, start), apply(&inv, at));
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let (w, h) = ((x1 - x0).max(1.0), (y1 - y0).max(1.0));
            // The dragged side moves with the pen; the opposite side stays put.
            let (l, pivot) = match i {
                0 => ([1.0, -dx / h, 0.0, 1.0], (mid.0, y1)),
                2 => ([1.0, dx / h, 0.0, 1.0], (mid.0, y0)),
                1 => ([1.0, 0.0, dy / w, 1.0], (x0, mid.1)),
                _ => ([1.0, 0.0, -dy / w, 1.0], (x1, mid.1)),
            };
            (from, compose(from_m, &about(l, pivot)))
        }
    }
}

/// The lasso transform's box after dragging `handle` from `start` to `at` (canvas pixels).
/// The opposite corner or side stays put; Shift keeps the box's proportions at a corner. The
/// box never gets smaller than a pixel and does not turn inside out.
fn drag_box(handle: Handle, from: Rect4, start: (f64, f64), at: (f64, f64), shift: bool) -> Rect4 {
    let (x0, y0, x1, y1) = from;
    let (dx, dy) = (at.0 - start.0, at.1 - start.1);
    const MIN: f64 = 1.0;
    match handle {
        Handle::Inside | Handle::Rotate | Handle::Skew(_) => (x0 + dx, y0 + dy, x1 + dx, y1 + dy),
        Handle::Side(i) => match i {
            0 => (x0, (y0 + dy).min(y1 - MIN), x1, y1),
            1 => (x0, y0, (x1 + dx).max(x0 + MIN), y1),
            2 => (x0, y0, x1, (y1 + dy).max(y0 + MIN)),
            _ => ((x0 + dx).min(x1 - MIN), y0, x1, y1),
        },
        Handle::Corner(i) => {
            // The fixed corner, and which way the dragged one lies from it.
            let (fx, fy, sx, sy) = match i {
                0 => (x1, y1, -1.0, -1.0),
                1 => (x0, y1, 1.0, -1.0),
                2 => (x0, y0, 1.0, 1.0),
                _ => (x1, y0, -1.0, 1.0),
            };
            let (w0, h0) = (x1 - x0, y1 - y0);
            let corner = |i: u8| match i {
                0 => (x0, y0),
                1 => (x1, y0),
                2 => (x1, y1),
                _ => (x0, y1),
            };
            let c = corner(i);
            let (mut w, mut h) = (((c.0 + dx - fx) * sx).max(MIN), ((c.1 + dy - fy) * sy).max(MIN));
            if shift {
                let k = (w / w0).max(h / h0);
                (w, h) = ((w0 * k).max(MIN), (h0 * k).max(MIN));
            }
            let (ax, bx) = if sx > 0.0 { (fx, fx + w) } else { (fx - w, fx) };
            let (ay, by) = if sy > 0.0 { (fy, fy + h) } else { (fy - h, fy) };
            (ax, ay, bx, by)
        }
    }
}

fn constrain(tool: Tool, a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    match tool {
        Tool::Rectangle | Tool::Ellipse => {
            let d = dx.abs().max(dy.abs());
            (a.0 + d.copysign(dx), a.1 + d.copysign(dy))
        }
        _ => {
            let len = dx.hypot(dy);
            let step = 15f64.to_radians();
            let ang = (dy.atan2(dx) / step).round() * step;
            (a.0 + len * ang.cos(), a.1 + len * ang.sin())
        }
    }
}

/// `path` with `ext` unless it already ends in it (any case).
fn with_extension(path: &Path, ext: &str) -> PathBuf {
    match path.extension() {
        Some(e) if e.eq_ignore_ascii_case(ext) || (ext == "jpg" && e.eq_ignore_ascii_case("jpeg")) => path.to_path_buf(),
        _ => {
            let mut s = path.as_os_str().to_owned();
            s.push(".");
            s.push(ext);
            PathBuf::from(s)
        }
    }
}

/// Writes through a `.part` file renamed into place, so a failed write never leaves half a
/// file behind.
fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".part");
    let tmp = PathBuf::from(tmp);
    let r = std::fs::write(&tmp, bytes).and_then(|()| std::fs::rename(&tmp, path)).map_err(|e| e.to_string());
    if r.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    r
}

/// The tiles whose look may differ between two versions of a document, or `None` when the
/// layer stack itself changed (then everything is redrawn).
pub fn changed_tiles(a: &Document, b: &Document) -> Option<HashSet<TileKey>> {
    if (a.width, a.height) != (b.width, b.height) || a.layers.len() != b.layers.len() {
        return None;
    }
    let mut out = HashSet::new();
    for (la, lb) in a.layers.iter().zip(&b.layers) {
        if la.uuid != lb.uuid || la.visible != lb.visible || la.opacity != lb.opacity || la.blend != lb.blend || la.pixels.default != lb.pixels.default {
            return None;
        }
        for k in la.pixels.tile_keys().chain(lb.pixels.tile_keys()) {
            let same = match (la.pixels.tile(k), lb.pixels.tile(k)) {
                (Some(x), Some(y)) => std::sync::Arc::ptr_eq(x, y),
                (None, None) => true,
                _ => false,
            };
            if !same {
                out.insert(k);
            }
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests;
