//! Recedo's interface (egui): menu bar, docked panels, status bar, dialogs, keyboard
//! shortcuts, and the brush outline over the canvas. Portable: the Windows shell runs it every
//! frame, and the tests run it headless.

use std::path::PathBuf;
use std::time::Duration;

use egui::{Color32, Id, Key, KeyboardShortcut, Modifiers as EguiMods, RichText, Stroke, Ui, vec2};
use recedo_core::autosave::Recoverable;
use recedo_core::blend::BlendMode;
use recedo_core::brush::PaintMode;
use recedo_core::canvas::{Background, CanvasPreset, builtin_presets};
use recedo_core::curve::Curve;

use crate::dock::Panel;
use crate::editor::{Editor, ExportFormat, MAX_BRUSH, MIN_BRUSH, NoticeKind, Tool};
use crate::settings::{Settings, TabletApi};
use web_time::Instant;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Something only the platform can do (file dialogs, quitting, devices).
#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    /// Show an Open dialog, then call [`AppUi::open_path`].
    OpenDialog,
    /// Show a Save dialog for a `.kra`, then call [`AppUi::save_as_path`].
    SaveAsDialog,
    /// Show a Save dialog for an exported picture, then `Editor::export`.
    ExportDialog(ExportFormat),
    /// Close the app (unsaved changes were dealt with).
    Quit,
    /// Switch the tablet interface.
    SetTabletApi(TabletApi),
}

/// What to do once unsaved changes are saved or discarded.
#[derive(Clone, Debug, PartialEq)]
pub enum Then {
    NewCanvas,
    OpenDialog,
    OpenPath(PathBuf),
    Quit,
    Recover(Box<Recoverable>),
}

enum Dialog {
    NewCanvas(NewCanvasForm),
    Unsaved(Then),
    Recovery(Vec<Recoverable>),
    JpegQuality,
    Messages { title: String, lines: Vec<String> },
    Settings,
    Shortcuts,
    About,
}

#[derive(Clone, Debug)]
struct NewCanvasForm {
    preset: usize,
    width: u32,
    height: u32,
    dpi: f64,
    background: Background,
}

impl NewCanvasForm {
    fn from_preset(presets: &[CanvasPreset], i: usize) -> NewCanvasForm {
        let p = &presets[i.min(presets.len() - 1)];
        NewCanvasForm { preset: i, width: p.width, height: p.height, dpi: p.dpi, background: p.background }
    }

    fn preset(&self) -> CanvasPreset {
        CanvasPreset { name: "Custom".into(), width: self.width, height: self.height, dpi: self.dpi, background: self.background }
    }
}

/// What the shell needs from one UI frame.
pub struct FrameOutput {
    /// The canvas area, in points.
    pub canvas_rect: egui::Rect,
    pub requests: Vec<Request>,
}

pub struct AppUi {
    pub settings: Settings,
    dialog: Option<Dialog>,
    /// After "Save" in the unsaved-changes dialog: what to do once the save finished.
    then_after_save: Option<Then>,
    /// The user chose "Don't save" before an Open dialog: don't ask again for that file.
    discard_ok: bool,
    /// Recent messages shown at the bottom of the canvas.
    toasts: Vec<(Instant, NoticeKind, String)>,
    rename: Option<(usize, String)>,
    presets: Vec<CanvasPreset>,
    hex: String,
    hsv: crate::widgets::Hsv,
    /// Shown in the status bar: the tablet interface and whether it works.
    pub tablet_status: String,
    /// Running in a web browser: no Quit, no recent files (no file paths), no choice of
    /// tablet interface (the browser has its own).
    pub web: bool,
}

const TOAST_TIME: Duration = Duration::from_secs(5);

impl AppUi {
    pub fn new(settings: Settings) -> AppUi {
        AppUi {
            settings,
            dialog: None,
            then_after_save: None,
            discard_ok: false,
            toasts: Vec::new(),
            rename: None,
            presets: builtin_presets(),
            hex: String::new(),
            hsv: crate::widgets::Hsv(0.6, 0.8, 0.6),
            tablet_status: String::new(),
            web: false,
        }
    }

    pub fn dialog_open(&self) -> bool {
        self.dialog.is_some()
    }

    /// Crash-recovery copies found at start-up.
    pub fn offer_recovery(&mut self, found: Vec<Recoverable>) {
        if !found.is_empty() {
            self.dialog = Some(Dialog::Recovery(found));
        }
    }

    /// The window's close button.
    pub fn request_quit(&mut self, ed: &Editor) -> bool {
        if ed.has_unsaved_changes() {
            self.dialog = Some(Dialog::Unsaved(Then::Quit));
            false
        } else {
            true
        }
    }

    /// The user picked a file in the Open dialog (or dropped one on the window).
    pub fn open_path(&mut self, ed: &mut Editor, path: PathBuf) {
        if ed.has_unsaved_changes() && !std::mem::take(&mut self.discard_ok) {
            self.dialog = Some(Dialog::Unsaved(Then::OpenPath(path)));
            return;
        }
        self.do_open(ed, &path);
    }

    fn do_open(&mut self, ed: &mut Editor, path: &std::path::Path) {
        let before = ed.notices.len();
        match ed.open(path) {
            Ok(()) => {
                self.settings.add_recent(path);
                // Things the file had that Recedo could not open: list them in a dialog.
                let warnings: Vec<String> = ed.notices[before..].iter().filter(|n| n.kind == NoticeKind::Warning).map(|n| n.text.clone()).collect();
                if !warnings.is_empty() {
                    ed.notices.truncate(before);
                    self.dialog = Some(Dialog::Messages { title: format!("Opened {}, with some changes", ed.title()), lines: warnings });
                }
            }
            Err(e) => self.dialog = Some(Dialog::Messages { title: "Could not open the file".into(), lines: vec![e] }),
        }
    }

    /// The user picked where to save in the Save dialog.
    pub fn save_as_path(&mut self, ed: &mut Editor, path: PathBuf) {
        ed.save_as(&path);
        self.settings.add_recent(&path.with_extension("kra"));
    }

    /// The Save dialog was cancelled: forget what was to follow the save.
    pub fn save_cancelled(&mut self) {
        self.then_after_save = None;
    }

    fn continue_with(&mut self, ed: &mut Editor, then: Then, requests: &mut Vec<Request>) {
        match then {
            Then::NewCanvas => self.dialog = Some(Dialog::NewCanvas(NewCanvasForm::from_preset(&self.presets, self.settings.last_preset))),
            Then::OpenDialog => requests.push(Request::OpenDialog),
            Then::OpenPath(p) => self.do_open(ed, &p),
            Then::Quit => requests.push(Request::Quit),
            Then::Recover(r) => {
                if let Err(e) = ed.open_recovery(&r) {
                    self.dialog = Some(Dialog::Messages { title: "Could not recover the painting".into(), lines: vec![e] });
                }
            }
        }
    }

    /// Runs a command that would replace the painting, asking about unsaved changes first.
    fn guarded(&mut self, ed: &mut Editor, then: Then, requests: &mut Vec<Request>) {
        if ed.has_unsaved_changes() {
            self.dialog = Some(Dialog::Unsaved(then));
        } else {
            self.continue_with(ed, then, requests);
        }
    }

    /// One frame of the interface.
    pub fn frame(&mut self, root: &mut Ui, ed: &mut Editor) -> FrameOutput {
        let mut requests = Vec::new();
        let ctx = root.ctx().clone();
        // A save started from the unsaved-changes dialog: carry on when it is done.
        if self.then_after_save.is_some() && !ed.is_busy_saving() {
            let then = self.then_after_save.take().expect("checked");
            if !ed.has_unsaved_changes() {
                self.continue_with(ed, then, &mut requests);
            }
        }
        for n in ed.notices.drain(..) {
            self.toasts.push((Instant::now(), n.kind, n.text));
        }
        self.toasts.retain(|(t, kind, _)| t.elapsed() < if *kind == NoticeKind::Info { TOAST_TIME } else { TOAST_TIME * 2 });
        if self.toasts.len() > 6 {
            let extra = self.toasts.len() - 6;
            self.toasts.drain(..extra);
        }

        if self.dialog.is_none() && !ctx.egui_wants_keyboard_input() {
            self.shortcuts(&ctx, ed, &mut requests);
        }

        egui::Panel::top("menu").show(root, |ui| {
            egui::MenuBar::new().ui(ui, |ui| self.menus(ui, ed, &mut requests));
        });
        egui::Panel::bottom("status").show(root, |ui| self.status_bar(ui, ed));
        let mut dock = std::mem::take(&mut self.settings.dock);
        let canvas_rect = dock.ui(root, &mut |ui, panel| self.panel(ui, panel, ed));
        self.settings.dock = dock;
        // The canvas itself is drawn by the GPU underneath; here only what goes over it.
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, Id::unique("recedo canvas overlay"))).with_clip_rect(canvas_rect);
        self.overlay(&painter, &ctx, ed, canvas_rect);
        self.toasts_ui(&ctx, canvas_rect);
        self.dialogs(&ctx, ed, &mut requests);
        FrameOutput { canvas_rect, requests }
    }

    // ----- shortcuts (Krita's defaults where Krita has one) -----------------------------

    fn shortcuts(&mut self, ctx: &egui::Context, ed: &mut Editor, requests: &mut Vec<Request>) {
        let none = EguiMods::NONE;
        let ctrl = EguiMods::COMMAND;
        let ctrl_shift = EguiMods::COMMAND | EguiMods::SHIFT;
        let shift = EguiMods::SHIFT;
        let hit = |m: EguiMods, k: Key| ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(m, k)));
        if hit(ctrl_shift, Key::Z) || hit(ctrl, Key::Y) {
            ed.redo();
        } else if hit(ctrl, Key::Z) {
            ed.undo();
        }
        if hit(ctrl, Key::N) {
            self.guarded(ed, Then::NewCanvas, requests);
        }
        if hit(ctrl, Key::O) {
            self.guarded(ed, Then::OpenDialog, requests);
        }
        // Save as, or Save when the painting has no file yet.
        if hit(ctrl_shift, Key::S) || (hit(ctrl, Key::S) && !ed.save()) {
            requests.push(Request::SaveAsDialog);
        }
        if !self.web && hit(ctrl, Key::Q) && self.request_quit(ed) {
            requests.push(Request::Quit);
        }
        if hit(ctrl, Key::J) {
            ed.duplicate_layer();
        }
        if hit(ctrl, Key::E) {
            ed.merge_down();
        }
        if hit(ctrl, Key::PageUp) {
            ed.move_layer(true);
        }
        if hit(ctrl, Key::PageDown) {
            ed.move_layer(false);
        }
        if hit(shift, Key::Delete) {
            ed.delete_layer();
        } else if hit(none, Key::Delete) {
            ed.clear_layer();
        }
        if hit(none, Key::Insert) {
            ed.add_layer();
        }
        if hit(none, Key::PageUp) {
            ed.select_neighbour(true);
        }
        if hit(none, Key::PageDown) {
            ed.select_neighbour(false);
        }
        if hit(none, Key::B) {
            ed.tool = Tool::Brush;
        }
        if hit(none, Key::E) {
            ed.toggle_eraser();
        }
        if hit(none, Key::P) {
            ed.tool = Tool::Eyedropper;
        }
        if hit(none, Key::OpenBracket) {
            ed.resize_brush(false);
        }
        if hit(none, Key::CloseBracket) {
            ed.resize_brush(true);
        }
        if hit(none, Key::I) {
            let b = ed.current_brush_mut();
            b.opacity = ((b.opacity - 0.1) * 10.0).round().clamp(0.0, 10.0) / 10.0;
        }
        if hit(none, Key::O) {
            let b = ed.current_brush_mut();
            b.opacity = ((b.opacity + 0.1) * 10.0).round().clamp(0.0, 10.0) / 10.0;
        }
        if hit(none, Key::Plus) || hit(none, Key::Equals) || hit(shift, Key::Equals) {
            ed.zoom_in();
        }
        if hit(none, Key::Minus) {
            ed.zoom_out();
        }
        if hit(none, Key::Num1) {
            ed.zoom_actual_pixels();
        }
        if hit(none, Key::Num2) {
            ed.fit_to_window();
        }
        if hit(none, Key::Num4) {
            ed.rotate_by_degrees(-15.0);
        }
        if hit(none, Key::Num6) {
            ed.rotate_by_degrees(15.0);
        }
        if hit(none, Key::Num5) {
            ed.reset_rotation();
        }
        if hit(none, Key::M) {
            ed.toggle_mirror();
        }
    }

    // ----- menus --------------------------------------------------------------------------

    fn menus(&mut self, ui: &mut Ui, ed: &mut Editor, requests: &mut Vec<Request>) {
        let item = |ui: &mut Ui, text: &str, keys: &str| ui.add(egui::Button::new(text).shortcut_text(keys)).clicked();
        ui.menu_button("File", |ui| {
            if item(ui, "New…", "Ctrl+N") {
                self.guarded(ed, Then::NewCanvas, requests);
                ui.close();
            }
            if item(ui, "Open…", "Ctrl+O") {
                self.guarded(ed, Then::OpenDialog, requests);
                ui.close();
            }
            let recent = if self.web { Vec::new() } else { self.settings.recent_files.clone() };
            ui.add_enabled_ui(!recent.is_empty(), |ui| {
                ui.menu_button("Open recent", |ui| {
                    for p in recent {
                        if ui.button(p.display().to_string()).clicked() {
                            self.guarded(ed, Then::OpenPath(p), requests);
                            ui.close();
                        }
                    }
                });
            });
            ui.separator();
            if item(ui, "Save", "Ctrl+S") {
                if !ed.save() {
                    requests.push(Request::SaveAsDialog);
                }
                ui.close();
            }
            if item(ui, "Save as…", "Ctrl+Shift+S") {
                requests.push(Request::SaveAsDialog);
                ui.close();
            }
            ui.separator();
            if item(ui, "Export as PNG…", "") {
                requests.push(Request::ExportDialog(ExportFormat::Png));
                ui.close();
            }
            if item(ui, "Export as JPEG…", "") {
                self.dialog = Some(Dialog::JpegQuality);
                ui.close();
            }
            if !self.web {
                ui.separator();
                if item(ui, "Quit", "Ctrl+Q") {
                    if self.request_quit(ed) {
                        requests.push(Request::Quit);
                    }
                    ui.close();
                }
            }
        });
        ui.menu_button("Edit", |ui| {
            let (names, applied) = ed.history.entries();
            let undo = applied.checked_sub(1).and_then(|i| names.get(i)).map(|n| format!("Undo {n}"));
            let redo = names.get(applied).map(|n| format!("Redo {n}"));
            if ui.add_enabled(undo.is_some(), egui::Button::new(undo.as_deref().unwrap_or("Undo")).shortcut_text("Ctrl+Z")).clicked() {
                ed.undo();
                ui.close();
            }
            if ui.add_enabled(redo.is_some(), egui::Button::new(redo.as_deref().unwrap_or("Redo")).shortcut_text("Ctrl+Shift+Z")).clicked() {
                ed.redo();
                ui.close();
            }
            ui.separator();
            if item(ui, "Settings…", "") {
                self.dialog = Some(Dialog::Settings);
                ui.close();
            }
        });
        ui.menu_button("View", |ui| {
            if item(ui, "Zoom in", "+") {
                ed.zoom_in();
            }
            if item(ui, "Zoom out", "-") {
                ed.zoom_out();
            }
            if item(ui, "Actual pixels (100 %)", "1") {
                ed.zoom_actual_pixels();
            }
            if item(ui, "Fit to window", "2") {
                ed.fit_to_window();
            }
            ui.separator();
            if item(ui, "Rotate left 15°", "4") {
                ed.rotate_by_degrees(-15.0);
            }
            if item(ui, "Rotate right 15°", "6") {
                ed.rotate_by_degrees(15.0);
            }
            if item(ui, "Reset rotation", "5") {
                ed.reset_rotation();
            }
            if ui.add(egui::Button::selectable(ed.view.mirror, "Mirror view").shortcut_text("M")).clicked() {
                ed.toggle_mirror();
            }
        });
        ui.menu_button("Layer", |ui| {
            if item(ui, "New layer", "Insert") {
                ed.add_layer();
                ui.close();
            }
            if item(ui, "Duplicate layer", "Ctrl+J") {
                ed.duplicate_layer();
                ui.close();
            }
            if item(ui, "Delete layer", "Shift+Delete") {
                ed.delete_layer();
                ui.close();
            }
            ui.separator();
            if item(ui, "Move layer up", "Ctrl+Page Up") {
                ed.move_layer(true);
            }
            if item(ui, "Move layer down", "Ctrl+Page Down") {
                ed.move_layer(false);
            }
            if item(ui, "Merge down", "Ctrl+E") {
                ed.merge_down();
                ui.close();
            }
            ui.separator();
            if item(ui, "Clear layer", "Delete") {
                ed.clear_layer();
                ui.close();
            }
        });
        ui.menu_button("Window", |ui| {
            for p in Panel::ALL {
                let mut open = self.settings.dock.is_open(p);
                if ui.checkbox(&mut open, p.title()).changed() {
                    self.settings.dock.toggle(p);
                }
            }
            ui.separator();
            if ui.button("Reset panels").clicked() {
                self.settings.dock = Default::default();
                ui.close();
            }
        });
        ui.menu_button("Help", |ui| {
            if ui.button("Keyboard shortcuts").clicked() {
                self.dialog = Some(Dialog::Shortcuts);
                ui.close();
            }
            if ui.button("About Recedo").clicked() {
                self.dialog = Some(Dialog::About);
                ui.close();
            }
        });
    }

    // ----- panels -------------------------------------------------------------------------

    fn panel(&mut self, ui: &mut Ui, panel: Panel, ed: &mut Editor) {
        match panel {
            Panel::Tools => tools_panel(ui, ed),
            Panel::ToolOptions => tool_options_panel(ui, ed),
            Panel::Color => self.color_panel(ui, ed),
            Panel::Layers => self.layers_panel(ui, ed),
            Panel::History => history_panel(ui, ed),
        }
    }

    fn color_panel(&mut self, ui: &mut Ui, ed: &mut Editor) {
        crate::widgets::color_selector(ui, &mut ed.color, &mut self.hsv);
        ui.add_space(2.0);
        // The numbers fold away: the square and the hue bar are what a pen uses.
        egui::CollapsingHeader::new("Numbers (RGB, hex)").id_salt("colour numbers").default_open(false).show(ui, |ui| {
            for (i, name) in ["Red", "Green", "Blue"].into_iter().enumerate() {
                ui.add(egui::Slider::new(&mut ed.color[i], 0..=255).text(name));
            }
            ui.horizontal(|ui| {
                ui.label("Hex");
                let current = format!("{:02X}{:02X}{:02X}", ed.color[0], ed.color[1], ed.color[2]);
                let resp = ui.add(egui::TextEdit::singleline(&mut self.hex).desired_width(70.0).hint_text(&current));
                if resp.lost_focus() {
                    if let Some(rgb) = parse_hex(&self.hex) {
                        ed.color = rgb;
                    }
                    self.hex.clear();
                }
            });
        });
    }

    fn layers_panel(&mut self, ui: &mut Ui, ed: &mut Editor) {
        ui.horizontal_wrapped(|ui| {
            if ui.button("New").on_hover_text("New layer above this one (Insert)").clicked() {
                ed.add_layer();
            }
            if ui.button("Duplicate").on_hover_text("Ctrl+J").clicked() {
                ed.duplicate_layer();
            }
            if ui.button("Delete").on_hover_text("Shift+Delete").clicked() {
                ed.delete_layer();
            }
            if ui.button("Up").on_hover_text("Ctrl+Page Up").clicked() {
                ed.move_layer(true);
            }
            if ui.button("Down").on_hover_text("Ctrl+Page Down").clicked() {
                ed.move_layer(false);
            }
            if ui.button("Merge down").on_hover_text("Ctrl+E").clicked() {
                ed.merge_down();
            }
        });
        ui.separator();
        // The selected layer's settings.
        let i = ed.active;
        if let Some(l) = ed.doc.layers.get(i) {
            let mut opacity = (l.opacity as f32 / 255.0 * 100.0).round();
            let blend = l.blend;
            let r = ui.add(egui::Slider::new(&mut opacity, 0.0..=100.0).text("Opacity").suffix(" %").integer());
            if r.changed() {
                ed.set_layer_opacity(i, (opacity / 100.0 * 255.0).round() as u8);
            }
            if r.drag_stopped() || r.lost_focus() {
                ed.end_edit();
            }
            let mut mode = blend;
            egui::ComboBox::from_id_salt("blend mode").selected_text(mode.name()).show_ui(ui, |ui| {
                for m in BlendMode::ALL {
                    ui.selectable_value(&mut mode, m, m.name());
                }
            });
            if mode != blend {
                ed.set_layer_blend(i, mode);
            }
        }
        ui.separator();
        // Top layer first, like Krita.
        let n = ed.doc.layers.len();
        for i in (0..n).rev() {
            let (name, visible, locked, opacity) = {
                let l = &ed.doc.layers[i];
                (l.name.clone(), l.visible, l.locked, l.opacity)
            };
            ui.horizontal(|ui| {
                use crate::widgets::{Icon, icon_toggle};
                if icon_toggle(ui, visible, Icon::Eye).on_hover_text(if visible { "Visible (click to hide)" } else { "Hidden (click to show)" }).clicked() {
                    ed.set_layer_visible(i, !visible);
                }
                if icon_toggle(ui, locked, Icon::Lock)
                    .on_hover_text(if locked { "Locked: cannot be painted on (click to unlock)" } else { "Unlocked (click to lock)" })
                    .clicked()
                {
                    ed.set_layer_locked(i, !locked);
                }
                if let Some((ri, text)) = &mut self.rename
                    && *ri == i
                {
                    let r = ui.text_edit_singleline(text);
                    r.request_focus();
                    if r.lost_focus() {
                        let (idx, new) = self.rename.take().expect("renaming");
                        if !ui.input(|i| i.key_pressed(Key::Escape)) {
                            ed.rename_layer(idx, &new);
                        }
                    }
                } else {
                    let label = if opacity < 255 { format!("{name}  ({}%)", (opacity as f32 / 2.55).round()) } else { name.clone() };
                    let text = if visible { RichText::new(label) } else { RichText::new(label).weak() };
                    let r = ui
                        .add(egui::Button::selectable(ed.active == i, text).min_size(vec2(ui.available_width(), 0.0)))
                        .on_hover_text("Click to select, double-click to rename");
                    if r.clicked() {
                        ed.select_layer(i);
                    }
                    if r.double_clicked() {
                        self.rename = Some((i, name));
                    }
                }
            });
        }
    }

    // ----- status bar, overlay, messages -----------------------------------------------------

    fn status_bar(&mut self, ui: &mut Ui, ed: &Editor) {
        ui.horizontal(|ui| {
            let star = if ed.has_unsaved_changes() { " *" } else { "" };
            ui.label(RichText::new(format!("{}{star}", ed.title())).strong());
            ui.separator();
            ui.label(format!("{} × {} px", ed.doc.width, ed.doc.height));
            ui.separator();
            ui.label(format!("{:.1} %", ed.view.zoom * 100.0));
            let deg = ed.view.angle.to_degrees();
            let deg = if deg > 180.0 { deg - 360.0 } else { deg };
            if deg.abs() > 0.05 {
                ui.label(format!("rotated {deg:.0}°"));
            }
            if ed.view.mirror {
                ui.label("mirrored");
            }
            ui.separator();
            ui.label(ed.tool.name());
            if let Some((x, y)) = ed.hover {
                let (cx, cy) = ed.view.to_canvas(x, y);
                ui.separator();
                ui.label(format!("{:.0}, {:.0}", cx.floor(), cy.floor()));
            }
            if ed.is_busy_saving() {
                ui.separator();
                ui.label("Saving…");
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.weak(&self.tablet_status);
            });
        });
    }

    fn overlay(&self, painter: &egui::Painter, ctx: &egui::Context, ed: &Editor, canvas: egui::Rect) {
        let ppp = ctx.pixels_per_point();
        let pt = |(x, y): (f64, f64)| egui::pos2(x as f32 / ppp, y as f32 / ppp);
        if let Some(l) = ed.leash() {
            let s = Stroke::new(1.0, Color32::from_rgb(80, 140, 230));
            painter.circle_stroke(pt(l.pen), l.radius as f32 / ppp, s);
            painter.line_segment([pt(l.brush), pt(l.pen)], s);
        }
        if let Some((c, r)) = ed.brush_outline() {
            let c = pt(c);
            if canvas.contains(c) {
                let r = r as f32 / ppp;
                // Dark and light rings, visible on any colour.
                painter.circle_stroke(c, r, Stroke::new(1.0, Color32::from_black_alpha(160)));
                painter.circle_stroke(c, r + 1.0, Stroke::new(1.0, Color32::from_white_alpha(160)));
                if r < 4.0 {
                    painter.line_segment([c - vec2(6.0, 0.0), c + vec2(6.0, 0.0)], Stroke::new(1.0, Color32::from_white_alpha(200)));
                    painter.line_segment([c - vec2(0.0, 6.0), c + vec2(0.0, 6.0)], Stroke::new(1.0, Color32::from_white_alpha(200)));
                }
            }
        }
    }

    fn toasts_ui(&self, ctx: &egui::Context, canvas: egui::Rect) {
        if self.toasts.is_empty() {
            return;
        }
        egui::Area::new(Id::unique("recedo toasts"))
            .order(egui::Order::Foreground)
            .anchor(
                egui::Align2::CENTER_BOTTOM,
                vec2(canvas.center().x - ctx.content_rect().center().x, -(ctx.content_rect().bottom() - canvas.bottom()) - 12.0),
            )
            .interactable(false)
            .show(ctx, |ui| {
                for (_, kind, text) in &self.toasts {
                    let color = match kind {
                        NoticeKind::Info => ui.visuals().text_color(),
                        NoticeKind::Warning => Color32::from_rgb(240, 190, 60),
                        NoticeKind::Error => Color32::from_rgb(240, 90, 80),
                    };
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.label(RichText::new(text).color(color));
                    });
                }
            });
    }

    // ----- dialogs ------------------------------------------------------------------------

    fn dialogs(&mut self, ctx: &egui::Context, ed: &mut Editor, requests: &mut Vec<Request>) {
        let Some(mut dialog) = self.dialog.take() else { return };
        let mut keep = true;
        let modal = egui::Modal::new(Id::unique("recedo dialog")).show(ctx, |ui| {
            ui.set_max_width(460.0);
            match &mut dialog {
                Dialog::NewCanvas(form) => keep = self.new_canvas_ui(ui, ed, form),
                Dialog::Unsaved(then) => {
                    let then = then.clone();
                    ui.heading("Save changes?");
                    ui.label(format!("\"{}\" has changes that are not saved.", ed.title()));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            keep = false;
                            self.then_after_save = Some(then.clone());
                            if !ed.save() {
                                requests.push(Request::SaveAsDialog);
                            }
                        }
                        if ui.button("Don't save").clicked() {
                            keep = false;
                            self.discard_ok = matches!(then, Then::OpenDialog);
                            self.continue_with(ed, then.clone(), requests);
                        }
                        if ui.button("Cancel").clicked() {
                            keep = false;
                        }
                    });
                }
                Dialog::Recovery(list) => {
                    ui.heading("Recover your work?");
                    ui.label("Recedo did not close normally last time. These autosaved copies were kept:");
                    ui.add_space(4.0);
                    let mut chosen = None;
                    for (i, r) in list.iter().enumerate() {
                        let name = r
                            .info
                            .original
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .map_or_else(|| "Unsaved painting".to_string(), |n| n.to_string_lossy().into_owned());
                        ui.horizontal(|ui| {
                            ui.label(format!("{name}, autosaved {}", ago(r.info.saved_at)));
                            if ui.button("Open").clicked() {
                                chosen = Some(i);
                            }
                        });
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Delete all and start fresh").clicked() {
                            for r in list.iter() {
                                recedo_core::autosave::remove_recovery_file(&r.kra);
                            }
                            keep = false;
                        }
                        if ui.button("Decide later").on_hover_text("Keep the copies; you will be asked next time").clicked() {
                            keep = false;
                        }
                    });
                    if let Some(i) = chosen {
                        keep = false;
                        self.guarded(ed, Then::Recover(Box::new(list[i].clone())), requests);
                    }
                }
                Dialog::JpegQuality => {
                    ui.heading("Export as JPEG");
                    ui.add(egui::Slider::new(&mut self.settings.jpeg_quality, 1..=100).text("Quality"));
                    ui.weak("Transparent areas become white. Full colour detail is kept (4:4:4).");
                    ui.horizontal(|ui| {
                        if ui.button("Choose file…").clicked() {
                            requests.push(Request::ExportDialog(ExportFormat::Jpeg(self.settings.jpeg_quality)));
                            keep = false;
                        }
                        if ui.button("Cancel").clicked() {
                            keep = false;
                        }
                    });
                }
                Dialog::Messages { title, lines } => {
                    ui.heading(title.as_str());
                    for l in lines.iter() {
                        ui.label(format!("• {l}"));
                    }
                    if ui.button("OK").clicked() {
                        keep = false;
                    }
                }
                Dialog::Settings => keep = self.settings_ui(ui, requests),
                Dialog::Shortcuts => {
                    ui.heading("Keyboard shortcuts");
                    egui::Grid::new("keys").striped(true).show(ui, |ui| {
                        for (k, what) in SHORTCUTS {
                            ui.label(RichText::new(*k).monospace());
                            ui.label(*what);
                            ui.end_row();
                        }
                    });
                    if ui.button("Close").clicked() {
                        keep = false;
                    }
                }
                Dialog::About => {
                    ui.heading("Recedo");
                    ui.label(format!("Version {VERSION} ({})", option_env!("RECEDO_GIT_HASH").unwrap_or("dev")));
                    ui.label("A painting app for pen tablets, compatible with Krita's .kra files.");
                    ui.label("Licence: GPL-3.0-or-later. See LICENCES.txt for the parts it uses.");
                    if ui.button("Close").clicked() {
                        keep = false;
                    }
                }
            }
        });
        // Escape or a click outside closes informational dialogs; questions need an answer.
        // Escape or a click outside closes any dialog: Cancel for "Save changes?", "Decide
        // later" for recovery (the copies are kept).
        if modal.should_close() {
            keep = false;
        }
        if keep && self.dialog.is_none() {
            self.dialog = Some(dialog);
        }
    }

    fn new_canvas_ui(&mut self, ui: &mut Ui, ed: &mut Editor, form: &mut NewCanvasForm) -> bool {
        ui.heading("New painting");
        let mut keep = true;
        egui::ComboBox::from_id_salt("preset").width(320.0).selected_text(self.presets[form.preset.min(self.presets.len() - 1)].name.clone()).show_ui(
            ui,
            |ui| {
                for (i, p) in self.presets.iter().enumerate() {
                    if ui.selectable_label(form.preset == i, &p.name).clicked() {
                        *form = NewCanvasForm::from_preset(&self.presets, i);
                    }
                }
            },
        );
        egui::Grid::new("new canvas").num_columns(2).show(ui, |ui| {
            ui.label("Width");
            ui.add(egui::DragValue::new(&mut form.width).range(1..=recedo_core::doc::MAX_SIDE).suffix(" px"));
            ui.end_row();
            ui.label("Height");
            ui.add(egui::DragValue::new(&mut form.height).range(1..=recedo_core::doc::MAX_SIDE).suffix(" px"));
            ui.end_row();
            ui.label("Resolution");
            ui.add(egui::DragValue::new(&mut form.dpi).range(1.0..=10_000.0).suffix(" ppi"));
            ui.end_row();
            ui.label("Background");
            ui.horizontal(|ui| {
                ui.radio_value(&mut form.background, Background::White, "White");
                ui.radio_value(&mut form.background, Background::Black, "Black");
                ui.radio_value(&mut form.background, Background::Transparent, "Transparent");
            });
            ui.end_row();
        });
        let preset = form.preset();
        let mb = preset.layer_bytes() as f64 / 1_048_576.0;
        ui.weak(format!("Each fully painted layer uses about {mb:.0} MB of memory."));
        match preset.validate() {
            Err(e) => {
                ui.colored_label(Color32::from_rgb(240, 90, 80), e);
            }
            Ok(()) => {
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        match ed.new_document(&preset) {
                            Ok(()) => {
                                self.settings.last_preset = form.preset;
                                keep = false;
                            }
                            Err(e) => {
                                ed.notices.push(crate::editor::Notice { kind: NoticeKind::Error, text: e });
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        keep = false;
                    }
                });
            }
        }
        keep
    }

    fn settings_ui(&mut self, ui: &mut Ui, requests: &mut Vec<Request>) -> bool {
        ui.heading("Settings");
        let mut keep = true;
        let web = self.web;
        egui::Grid::new("settings").num_columns(2).show(ui, |ui| {
            if web {
                ui.label("Pen");
                ui.weak("The browser's pen input (Windows Ink: pressure and tilt).");
                ui.end_row();
            } else {
                ui.label("Tablet");
                ui.horizontal(|ui| {
                    for api in [TabletApi::Ink, TabletApi::Wintab] {
                        if ui.radio(self.settings.tablet == api, api.label()).clicked() && self.settings.tablet != api {
                            self.settings.tablet = api;
                            requests.push(Request::SetTabletApi(api));
                        }
                    }
                });
                ui.end_row();
                ui.label("");
                ui.weak("Windows Ink works with most drivers. Try Wintab if pressure or tilt is missing.");
                ui.end_row();
            }
            ui.label("Interface size");
            ui.add(egui::Slider::new(&mut self.settings.ui_scale, 0.75..=2.0).step_by(0.05).custom_formatter(|v, _| format!("{:.0} %", v * 100.0)));
            ui.end_row();
            ui.label("Autosave every");
            ui.add(egui::Slider::new(&mut self.settings.autosave_minutes, 1..=60).suffix(" min"));
            ui.end_row();
        });
        if ui.button("Close").clicked() {
            keep = false;
        }
        keep
    }
}

fn tools_panel(ui: &mut Ui, ed: &mut Editor) {
    let keys = |t: Tool| match t {
        Tool::Brush => "B",
        Tool::Eraser => "E",
        Tool::Eyedropper => "P (or Ctrl-click)",
        Tool::Pan => "or hold Space",
    };
    ui.vertical_centered_justified(|ui| {
        for t in Tool::ALL {
            if ui.add(egui::Button::selectable(ed.tool == t, t.name()).min_size(vec2(0.0, 28.0))).on_hover_text(keys(t)).clicked() {
                ed.tool = t;
            }
        }
        ui.add_space(8.0);
        let [r, g, b] = ed.color;
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 4.0, Color32::from_rgb(r, g, b));
        // A light outline, so a dark colour still reads as a swatch on the dark panel.
        ui.painter().rect_stroke(rect, 4.0, Stroke::new(1.0, ui.visuals().widgets.inactive.fg_stroke.color), egui::StrokeKind::Inside);
    });
}

fn tool_options_panel(ui: &mut Ui, ed: &mut Editor) {
    match ed.tool {
        Tool::Brush | Tool::Eraser => {
            ui.strong(if ed.tool == Tool::Eraser { "Eraser" } else { "Brush" });
            let b = ed.current_brush_mut();
            ui.add(egui::Slider::new(&mut b.size, MIN_BRUSH..=MAX_BRUSH).logarithmic(true).text("Size").suffix(" px").max_decimals(0))
                .on_hover_text("[ and ] change the size");
            let pct = |ui: &mut Ui, v: &mut f32, label: &str, tip: &str| {
                let mut p = (*v * 100.0).round();
                if ui.add(egui::Slider::new(&mut p, 0.0..=100.0).text(label).suffix(" %").integer()).on_hover_text(tip).changed() {
                    *v = p / 100.0;
                }
            };
            pct(ui, &mut b.opacity, "Opacity", "How much a stroke covers at most (I and O change it)");
            pct(ui, &mut b.flow, "Flow", "How much each dab adds");
            pct(ui, &mut b.hardness, "Hardness", "0 % is a soft airbrush edge, 100 % a hard edge");
            let mut spacing = (b.spacing * 100.0).round();
            if ui
                .add(egui::Slider::new(&mut spacing, 1.0..=100.0).text("Spacing").suffix(" %").integer())
                .on_hover_text("Distance between dabs, as a share of the size")
                .changed()
            {
                b.spacing = spacing / 100.0;
            }
            ui.horizontal(|ui| {
                ui.label("Mode");
                ui.radio_value(&mut b.mode, PaintMode::Wash, "Wash").on_hover_text("A stroke never gets darker than its opacity where it crosses itself");
                ui.radio_value(&mut b.mode, PaintMode::BuildUp, "Build up").on_hover_text("Overlapping parts of a stroke get darker");
            });
            // Fold-out sections, so the panel stays short on a small screen.
            egui::CollapsingHeader::new("Pen").id_salt("pen options").default_open(true).show(ui, |ui| {
                let mut size_p = b.size_curve.is_some();
                if ui.checkbox(&mut size_p, "Pressure changes the size").changed() {
                    b.size_curve = size_p.then(Curve::linear);
                }
                if size_p {
                    let mut m = (b.min_size * 100.0).round();
                    if ui.add(egui::Slider::new(&mut m, 0.0..=100.0).text("Smallest size").suffix(" %").integer()).changed() {
                        b.min_size = m / 100.0;
                    }
                }
                let mut op_p = b.opacity_curve.is_some();
                if ui.checkbox(&mut op_p, "Pressure changes the opacity").changed() {
                    b.opacity_curve = op_p.then(Curve::linear);
                }
                let mut flow_p = b.flow_curve.is_some();
                if ui.checkbox(&mut flow_p, "Pressure changes the flow").changed() {
                    b.flow_curve = flow_p.then(Curve::linear);
                }
                ui.checkbox(&mut b.tilt_shape, "Tilt leans the brush");
            });
            let s = &mut ed.tools.stabilizer;
            let title = if s.enabled { "Stabilizer (on)" } else { "Stabilizer (off)" };
            egui::CollapsingHeader::new(title).id_salt("stabilizer options").default_open(false).show(ui, |ui| {
                ui.checkbox(&mut s.enabled, "Smooth the line (Krita's stabilizer)");
                ui.add_enabled_ui(s.enabled, |ui| {
                    ui.add(egui::Slider::new(&mut s.distance, 3.0..=500.0).logarithmic(true).text("Distance").max_decimals(0));
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut s.delay, "Delay");
                        ui.add_enabled(s.delay, egui::Slider::new(&mut s.delay_distance, 1.0..=500.0).logarithmic(true).suffix(" px").max_decimals(0));
                    });
                    ui.checkbox(&mut s.finish_line, "Finish line (catch up when the pen lifts)");
                    ui.checkbox(&mut s.stabilize_sensors, "Smooth pressure and tilt too");
                    ui.checkbox(&mut s.scalable_distance, "Distance follows the zoom");
                });
            });
        }
        Tool::Eyedropper => {
            ui.strong("Eyedropper");
            ui.radio_value(&mut ed.tools.pick_all_layers, true, "The visible picture");
            ui.radio_value(&mut ed.tools.pick_all_layers, false, "The selected layer only");
            ui.add(egui::Slider::new(&mut ed.tools.pick_radius, 0.0..=20.0).text("Average over").suffix(" px").max_decimals(0));
            ui.weak("Tip: hold Ctrl with the brush to pick a colour.");
        }
        Tool::Pan => {
            ui.strong("Pan");
            ui.label("Drag to move the view.");
            ui.weak("With any tool: Space+drag pans, Shift+Space+drag rotates, Ctrl+Space+drag zooms.");
        }
    }
}

fn history_panel(ui: &mut Ui, ed: &mut Editor) {
    let (names, applied) = ed.history.entries();
    let names: Vec<String> = names.into_iter().map(String::from).collect();
    ui.horizontal(|ui| {
        if ui.add_enabled(applied > 0, egui::Button::new("Undo")).clicked() {
            ed.undo();
        }
        if ui.add_enabled(applied < names.len(), egui::Button::new("Redo")).clicked() {
            ed.redo();
        }
    });
    ui.separator();
    if ui.selectable_label(applied == 0, "Opened / new").clicked() {
        ed.jump_to_history(0);
    }
    for (i, name) in names.iter().enumerate() {
        let done = i < applied;
        let text = if done { RichText::new(name) } else { RichText::new(name).weak() };
        if ui.selectable_label(applied == i + 1, text).clicked() {
            ed.jump_to_history(i + 1);
        }
    }
}

fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let h = s.trim().trim_start_matches('#');
    let full: String = match h.len() {
        3 => h.chars().flat_map(|c| [c, c]).collect(),
        6 => h.to_string(),
        _ => return None,
    };
    let v = u32::from_str_radix(&full, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

fn ago(secs_since_1970: u64) -> String {
    let now = web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let d = now.saturating_sub(secs_since_1970);
    match d {
        0..60 => "just now".into(),
        60..3600 => format!("{} min ago", d / 60),
        3600..86_400 => format!("{} h ago", d / 3600),
        _ => format!("{} days ago", d / 86_400),
    }
}

/// Shown in Help > Keyboard shortcuts.
pub const SHORTCUTS: &[(&str, &str)] = &[
    ("B", "Brush"),
    ("E", "Eraser on/off"),
    ("P", "Eyedropper"),
    ("Ctrl + click", "Pick a colour while painting"),
    ("[  ]", "Brush smaller / bigger"),
    ("I  O", "Brush opacity down / up"),
    ("Space + drag", "Pan"),
    ("Shift + Space + drag", "Rotate the view"),
    ("Ctrl + Space + drag", "Zoom"),
    ("Mouse wheel", "Zoom"),
    ("Middle mouse button + drag", "Pan"),
    ("+  -", "Zoom in / out"),
    ("1", "Actual pixels (100 %)"),
    ("2", "Fit to window"),
    ("4  6", "Rotate 15° left / right"),
    ("5", "Reset rotation"),
    ("M", "Mirror the view"),
    ("Ctrl + Z", "Undo"),
    ("Ctrl + Shift + Z, Ctrl + Y", "Redo"),
    ("Insert", "New layer"),
    ("Ctrl + J", "Duplicate layer"),
    ("Shift + Delete", "Delete layer"),
    ("Delete", "Clear layer"),
    ("Ctrl + E", "Merge down"),
    ("Page Up / Down", "Select the layer above / below"),
    ("Ctrl + Page Up / Down", "Move the layer up / down"),
    ("Ctrl + N, O, S", "New, open, save"),
    ("Ctrl + Shift + S", "Save as"),
    ("Ctrl + Q", "Quit"),
];

#[cfg(test)]
mod tests;
