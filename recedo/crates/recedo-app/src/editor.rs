//! The editor: everything Recedo does to a painting, independent of Windows and the GPU, so
//! all of it is unit-tested on any OS. The Windows shell feeds it pen samples, keys and menu
//! actions, and shows its projection (the flattened picture) on screen.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Instant;

use recedo_core::autosave::{self, Autosaver};
use recedo_core::blend::BlendMode;
use recedo_core::brush::Brush;
use recedo_core::canvas::CanvasPreset;
use recedo_core::doc::{Document, new_uuid};
use recedo_core::history::History;
use recedo_core::image;
use recedo_core::paint::PaintSession;
use recedo_core::projection::Projection;
use recedo_core::stabilizer::{PaintInfo, StabilizerSettings};
use recedo_core::tiles::TileKey;
use recedo_core::view::View;
use recedo_core::{import, kra, ops};
use recedo_tablet::{Flags, PenSample, Source};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tool {
    #[default]
    Brush,
    Eraser,
    Eyedropper,
    Pan,
}

impl Tool {
    pub const ALL: [Tool; 4] = [Tool::Brush, Tool::Eraser, Tool::Eyedropper, Tool::Pan];

    pub fn name(self) -> &'static str {
        match self {
            Tool::Brush => "Brush",
            Tool::Eraser => "Eraser",
            Tool::Eyedropper => "Eyedropper",
            Tool::Pan => "Pan",
        }
    }
}

/// Settings of the tools, kept between sessions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ToolSettings {
    pub brush: Brush,
    pub eraser: Brush,
    pub stabilizer: StabilizerSettings,
    /// Eyedropper: sample the visible picture (true) or only the current layer.
    pub pick_all_layers: bool,
    /// Eyedropper: average over this radius, in canvas pixels (0 = one pixel).
    pub pick_radius: f32,
}

impl Default for ToolSettings {
    fn default() -> ToolSettings {
        ToolSettings {
            brush: Brush::default(),
            eraser: Brush { size: 40.0, hardness: 0.7, ..Brush::default() },
            stabilizer: StabilizerSettings::default(),
            pick_all_layers: true,
            pick_radius: 0.0,
        }
    }
}

/// Smallest and largest brush size the size keys and sliders allow, in canvas pixels.
pub const MIN_BRUSH: f32 = 1.0;
pub const MAX_BRUSH: f32 = 1000.0;

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
    /// The pen is down but nothing happens until it lifts (e.g. a locked layer).
    Ignore,
}

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
    /// Size of the canvas area on screen, in pixels.
    pub viewport: (f64, f64),
    /// Index of the selected layer (bottom = 0).
    pub active: usize,
    pub tool: Tool,
    pub tools: ToolSettings,
    /// Painting colour, sRGB.
    pub color: [u8; 3],
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
            viewport: (800.0, 600.0),
            active,
            tool: Tool::Brush,
            tools: ToolSettings::default(),
            color: [20, 20, 30],
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
        self.autosaver.has_unsaved_changes()
    }

    pub fn is_busy_saving(&self) -> bool {
        self.jobs.iter().any(|j| matches!(j.kind, JobKind::Save(_) | JobKind::Export(_)))
    }

    pub fn stroke_active(&self) -> bool {
        matches!(self.interaction, Interaction::Stroke(_))
    }

    /// The stabilizer's delay leash on screen, while a stroke uses it.
    pub fn leash(&self) -> Option<Leash> {
        let Interaction::Stroke(s) = &self.interaction else { return None };
        let (b, p, r) = s.leash()?;
        Some(Leash { brush: self.view.to_screen(b.x, b.y), pen: self.view.to_screen(p.x, p.y), radius: r * self.view.zoom })
    }

    /// The brush outline at the hover point: (centre, radius in screen pixels), for the tools
    /// that paint.
    pub fn brush_outline(&self) -> Option<((f64, f64), f64)> {
        let brush = match self.tool {
            Tool::Brush => &self.tools.brush,
            Tool::Eraser => &self.tools.eraser,
            _ => return None,
        };
        let at = self.hover?;
        Some((at, (brush.size as f64 * 0.5 * self.view.zoom).max(1.0)))
    }

    pub fn current_brush(&self) -> &Brush {
        if self.tool == Tool::Eraser { &self.tools.eraser } else { &self.tools.brush }
    }

    pub fn current_brush_mut(&mut self) -> &mut Brush {
        if self.tool == Tool::Eraser { &mut self.tools.eraser } else { &mut self.tools.brush }
    }

    fn notice(&mut self, kind: NoticeKind, text: impl Into<String>) {
        self.notices.push(Notice { kind, text: text.into() });
    }

    // ----- screen -----------------------------------------------------------------------

    /// The canvas area's size on screen; fits a newly opened painting into it.
    pub fn set_viewport(&mut self, w: f64, h: f64) {
        if w < 1.0 || h < 1.0 {
            return;
        }
        self.viewport = (w, h);
        if self.fit_pending {
            self.fit_pending = false;
            self.view.fit((self.doc.width, self.doc.height), self.viewport, 24.0);
        }
    }

    /// Brings the projection up to date and says what the screen must re-upload.
    pub fn take_refresh(&mut self) -> Refresh {
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
    }

    /// After the document changed from `before`: refresh only what differs.
    fn refresh_from(&mut self, before: &Document) {
        match changed_tiles(before, &self.doc) {
            Some(tiles) => self.mark_tiles(tiles),
            None => self.refresh_all = true,
        }
    }

    /// Marks every tile a layer covers (after a change to its opacity, visibility, mode).
    fn refresh_layer(&mut self, index: usize) {
        let Some(l) = self.doc.layers.get(index) else { return };
        if l.pixels.default[3] != 0 {
            self.refresh_all = true;
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
        let at = (s.x, s.y);
        self.interaction = if mods.space && mods.shift {
            Interaction::Rotate { last_angle: self.angle_from_centre(at) }
        } else if mods.space && mods.ctrl {
            Interaction::Zoom { anchor: at, last_y: s.y }
        } else if mods.space || self.tool == Tool::Pan {
            Interaction::Pan { last: at }
        } else if self.tool == Tool::Eyedropper || (mods.ctrl && matches!(self.tool, Tool::Brush | Tool::Eraser)) {
            let all_layers = self.tools.pick_all_layers;
            self.pick(s, all_layers);
            Interaction::Pick { all_layers }
        } else {
            // The eraser end of the pen always erases, whatever the tool.
            let erase = self.tool == Tool::Eraser || s.flags.has(Flags::ERASER);
            let brush = if erase { self.tools.eraser.clone() } else { self.tools.brush.clone() };
            let info = self.paint_info(s);
            match PaintSession::begin(&mut self.doc, &mut self.history, self.active, brush, self.color, erase, self.tools.stabilizer, self.view.zoom, info, s.recv_us) {
                Ok(mut session) => {
                    let d = session.take_dirty();
                    self.mark_tiles(d);
                    Interaction::Stroke(Box::new(session))
                }
                Err(e) => {
                    self.notice(NoticeKind::Warning, format!("{e}: nothing was painted."));
                    Interaction::Ignore
                }
            }
        };
    }

    fn drag(&mut self, s: &PenSample) {
        let at = (s.x, s.y);
        let info = self.paint_info(s);
        let angle = self.angle_from_centre(at);
        let centre = self.centre();
        let mut pick = None;
        match &mut self.interaction {
            Interaction::Stroke(session) => {
                session.add(&mut self.doc, info, s.recv_us);
                self.dirty.extend(session.take_dirty());
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
            Interaction::Pick { all_layers } => pick = Some(*all_layers),
            Interaction::Ignore | Interaction::Idle => {}
        }
        if let Some(all) = pick {
            self.pick(s, all);
        }
    }

    fn release(&mut self, now_us: u64) {
        let interaction = std::mem::replace(&mut self.interaction, Interaction::Idle);
        self.owner = None;
        if let Interaction::Stroke(session) = interaction {
            let d = session.end(&mut self.doc, now_us);
            self.mark_tiles(d);
            self.autosaver.changed(Instant::now());
        }
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
        if let Interaction::Stroke(session) = &mut self.interaction {
            session.tick(&mut self.doc, now_us);
            self.dirty.extend(session.take_dirty());
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
        (at.1 - self.viewport.1 / 2.0).atan2(at.0 - self.viewport.0 / 2.0)
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
        (self.viewport.0 / 2.0, self.viewport.1 / 2.0)
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
        self.view.fit((self.doc.width, self.doc.height), self.viewport, 24.0);
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

    /// `[` and `]`: about 10 % smaller or larger.
    pub fn resize_brush(&mut self, bigger: bool) {
        let b = self.current_brush_mut();
        let s = if bigger { (b.size * 1.1).max(b.size + 1.0) } else { (b.size / 1.1).min(b.size - 1.0) };
        b.size = s.round().clamp(MIN_BRUSH, MAX_BRUSH);
    }

    /// `E`: switch between the brush and the eraser.
    pub fn toggle_eraser(&mut self) {
        self.tool = if self.tool == Tool::Eraser { Tool::Brush } else { Tool::Eraser };
    }

    // ----- undo -----------------------------------------------------------------------------

    fn finish_interaction(&mut self) {
        if self.owner.is_some() {
            self.release(recedo_tablet::clock::now_us());
        }
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
            self.fit_pending = true;
            self.set_viewport(self.viewport.0, self.viewport.1);
        }
        self.active = self.active.min(self.doc.layers.len().saturating_sub(1));
        self.refresh_from(before);
        self.autosaver.changed(Instant::now());
    }

    // ----- layers ---------------------------------------------------------------------------

    /// Runs a document change as one undo step named `name`.
    fn edit(&mut self, name: &str, f: impl FnOnce(&mut Document, &mut usize) -> Result<(), String>) {
        self.finish_interaction();
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

    pub fn clear_layer(&mut self) {
        self.edit("Clear layer", |doc, active| ops::clear_layer(doc, *active));
    }

    pub fn select_layer(&mut self, index: usize) {
        if index < self.doc.layers.len() {
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
        self.finish_interaction();
        self.remove_recovery_copies();
        self.doc = doc;
        self.history.clear();
        self.active = self.doc.layers.len().saturating_sub(1);
        self.autosaver = Autosaver::new(self.autosaver.interval);
        self.doc_id = new_uuid();
        self.refresh_all = true;
        self.fit_pending = true;
        self.view = View::default();
        self.last_edit = None;
        let vp = self.viewport;
        self.set_viewport(vp.0, vp.1);
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
