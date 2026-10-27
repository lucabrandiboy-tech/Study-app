//! Recedo's interface (egui): menu bar, docked panels, status bar, dialogs, keyboard
//! shortcuts, and the brush outline over the canvas. Portable: the Windows shell runs it every
//! frame, and the tests run it headless.

use std::path::PathBuf;
use std::time::Duration;

use egui::{Color32, Id, Key, KeyboardShortcut, Modifiers as EguiMods, RichText, Stroke, Ui, vec2};
use recedo_core::autosave::Recoverable;
use recedo_core::blend::BlendMode;
use recedo_core::brush::{Brush, PaintMode};
use recedo_core::canvas::{Background, CanvasPreset, builtin_presets};
use recedo_core::curve::Curve;
use recedo_core::select::Combine;
use recedo_core::text::Font;
use web_time::Instant;

use crate::dock::Panel;
use crate::editor::{Editor, ExportFormat, MAX_BRUSH, MIN_BRUSH, NoticeKind, Tool, ToolGroup};
use crate::icons::{self, Action};
use crate::previews::{self, Previews};
use crate::settings::{Settings, TabletApi};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The toolbar's and tool strip's background (darker than the panels, as in Clip Studio).
const TOOLBAR_FILL: Color32 = Color32::from_gray(38);
/// The tool strip: icons with their names under them.
const STRIP_WIDTH: f32 = 60.0;
/// The font family of the text tool's preview: Ubuntu Light, which the text is painted in
/// (the interface itself is in Inter).
pub const TEXT_SANS: &str = "text sans";

/// Clip Studio-like greys: mid-grey panels, darker bars, a blue-grey selection.
pub fn apply_theme(ctx: &egui::Context) {
    let g = Color32::from_gray;
    let mut v = egui::Visuals::dark();
    v.panel_fill = g(58);
    v.window_fill = g(58);
    v.faint_bg_color = g(64);
    v.extreme_bg_color = g(40);
    v.window_stroke = Stroke::new(1.0, g(28));
    v.selection.bg_fill = Color32::from_rgb(92, 108, 142);
    v.selection.stroke = Stroke::new(1.0, g(245));
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = g(58);
    w.noninteractive.weak_bg_fill = g(58);
    w.noninteractive.bg_stroke = Stroke::new(1.0, g(44));
    w.noninteractive.fg_stroke = Stroke::new(1.0, g(210));
    for (state, fill, text) in [(&mut w.inactive, 76, 220), (&mut w.hovered, 92, 245), (&mut w.active, 104, 255), (&mut w.open, 84, 235)] {
        state.bg_fill = g(fill);
        state.weak_bg_fill = g(fill);
        state.fg_stroke = Stroke::new(1.0, g(text));
    }
    w.hovered.bg_stroke = Stroke::new(1.0, g(120));
    ctx.set_visuals_of(egui::Theme::Dark, v);
    ctx.set_theme(egui::ThemePreference::Dark);
    // Inter: a typeface drawn for small interface text (sharper than egui's Ubuntu Light).
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("Inter".into(), std::sync::Arc::new(egui::FontData::from_static(include_bytes!("../assets/Inter-Regular.otf"))));
    let rest: Vec<String> = fonts.families.get(&egui::FontFamily::Proportional).cloned().unwrap_or_default();
    let mut ui_fonts = vec!["Inter".to_string()];
    ui_fonts.extend(rest.iter().cloned());
    fonts.families.insert(egui::FontFamily::Proportional, ui_fonts);
    fonts.families.insert(egui::FontFamily::Name(TEXT_SANS.into()), rest);
    ctx.set_fonts(fonts);
    // Compact: text and spacing about three quarters of egui's.
    ctx.all_styles_mut(|s| {
        use egui::{FontFamily, FontId, TextStyle};
        s.text_styles = [
            (TextStyle::Small, FontId::new(9.0, FontFamily::Proportional)),
            (TextStyle::Body, FontId::new(11.0, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(11.0, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(10.5, FontFamily::Monospace)),
            (TextStyle::Heading, FontId::new(14.0, FontFamily::Proportional)),
        ]
        .into();
        let sp = &mut s.spacing;
        sp.item_spacing = vec2(5.0, 3.0);
        sp.button_padding = vec2(4.0, 1.0);
        sp.interact_size = vec2(32.0, 16.0);
        sp.slider_width = 80.0;
        sp.combo_width = 80.0;
        sp.icon_width = 11.0;
        sp.icon_width_inner = 6.0;
        sp.icon_spacing = 3.0;
        sp.indent = 12.0;
        sp.menu_margin = egui::Margin::same(4);
        sp.window_margin = egui::Margin::same(6);
    });
}

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
    /// Browser version: show or hide the reference panel (search, pictures, videos).
    ToggleReference,
    /// Browser version: G hid (true) or brought back the interface; the reference panel
    /// hides with it.
    CanvasOnly(bool),
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
    GettingStarted,
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
    /// The window with every tool, where the side bar's tools are chosen.
    pub tool_library: bool,
    /// Browser version: the reference panel is open (the shell keeps this up to date).
    pub reference_open: bool,
    /// G: only the painting is shown.
    pub canvas_only: bool,
    /// The tool in the last frame, and whether text was being written then.
    last_tool: Tool,
    writing: bool,
    /// The brush and eraser presets last picked in the Sub tool panel.
    brush_preset: Option<usize>,
    eraser_preset: Option<usize>,
    /// Stroke samples and layer thumbnails.
    previews: Previews,
    /// Where the tool strip's buttons were last drawn (`None`: All tools), for the tests.
    pub strip: Vec<(Option<Tool>, egui::Rect)>,
}

const TOAST_TIME: Duration = Duration::from_secs(5);

impl AppUi {
    pub fn new(settings: Settings) -> AppUi {
        // The presets the saved brush and eraser are (if they were not changed since).
        let brush_preset = presets().iter().position(|p| p.2 == settings.tools.brush);
        let eraser_preset = eraser_presets().iter().position(|p| p.2 == settings.tools.eraser);
        AppUi {
            settings,
            dialog: None,
            then_after_save: None,
            discard_ok: false,
            toasts: Vec::new(),
            rename: None,
            presets: builtin_presets(),
            hex: String::new(),
            hsv: crate::widgets::Hsv(0.0, 0.0, 0.0),
            tablet_status: String::new(),
            web: false,
            tool_library: false,
            reference_open: false,
            canvas_only: false,
            last_tool: Tool::Brush,
            writing: false,
            brush_preset,
            eraser_preset,
            previews: Previews::default(),
            strip: Vec::new(),
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

    /// G: hides (or brings back) everything but the painting: menus, toolbar, panels, and in
    /// the browser the reference panel.
    pub fn set_canvas_only(&mut self, on: bool, requests: &mut Vec<Request>) {
        if self.canvas_only == on {
            return;
        }
        self.canvas_only = on;
        if self.web {
            requests.push(Request::CanvasOnly(on));
        }
        // The hint goes with it: once everything is back it has nothing more to say.
        self.toasts.retain(|(_, _, text)| text != CANVAS_ONLY_HINT);
        if on {
            self.toasts.push((Instant::now(), NoticeKind::Info, CANVAS_ONLY_HINT.into()));
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
        let result = ed.open(path);
        if result.is_ok() {
            self.settings.add_recent(path);
        }
        self.after_open(ed, before, result);
    }

    /// Lists in a dialog what the file had that Recedo could not open, or why it failed.
    fn after_open(&mut self, ed: &mut Editor, before: usize, result: Result<(), String>) {
        match result {
            Ok(()) => {
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

    /// The browser version: a file the user picked, as bytes (unsaved changes were asked
    /// about before the file picker opened).
    // Only the browser app (recedo-web) calls this.
    #[cfg_attr(windows, allow(dead_code))]
    pub fn open_bytes(&mut self, ed: &mut Editor, name: &str, bytes: &[u8]) {
        let before = ed.notices.len();
        let result = ed.open_bytes(name, bytes);
        self.after_open(ed, before, result);
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
            // The same message again (Undo pressed with nothing left, say): shown once, anew.
            self.toasts.retain(|(_, _, text)| *text != n.text);
            self.toasts.push((Instant::now(), n.kind, n.text));
        }
        self.toasts.retain(|(t, kind, _)| t.elapsed() < if *kind == NoticeKind::Info { TOAST_TIME } else { TOAST_TIME * 2 });
        if self.toasts.len() > 6 {
            let extra = self.toasts.len() - 6;
            self.toasts.drain(..extra);
        }

        // While text is being written, letters are text, never shortcuts (also those that come
        // before its box has the keys, in the frame of the click that started it).
        let writing = ed.tool == Tool::Text && ed.text_edit.is_some();
        if self.dialog.is_none() && !ctx.egui_wants_keyboard_input() && !writing {
            self.shortcuts(&ctx, ed, &mut requests);
        }

        let canvas_rect = if self.canvas_only {
            // G: nothing but the painting.
            root.available_rect_before_wrap()
        } else {
            egui::Panel::top("menu").show(root, |ui| {
                egui::MenuBar::new().ui(ui, |ui| self.menus(ui, ed, &mut requests));
            });
            egui::Panel::top("toolbar").frame(egui::Frame::NONE.fill(TOOLBAR_FILL).inner_margin(egui::Margin::symmetric(5, 2))).show(root, |ui| self.toolbar(ui, ed, &mut requests));
            egui::Panel::bottom("status").show(root, |ui| self.status_bar(ui, ed));
            // Clip Studio's tool strip: the side bar's tools as icons, at the far left.
            egui::Panel::left("tool strip")
                .resizable(false)
                .exact_size(STRIP_WIDTH)
                .frame(egui::Frame::NONE.fill(TOOLBAR_FILL).inner_margin(egui::Margin::symmetric(0, 3)))
                .show(root, |ui| self.tool_strip(ui, ed));
            let titles: Vec<String> = Panel::ALL.iter().map(|&p| self.panel_title(p, ed)).collect();
            let title = |p: Panel| titles[Panel::ALL.iter().position(|&q| q == p).unwrap_or(0)].clone();
            let mut dock = std::mem::take(&mut self.settings.dock);
            let rect = dock.ui(root, &title, &mut |ui, panel| self.panel(ui, panel, ed));
            self.settings.dock = dock;
            rect
        };
        // The canvas itself is drawn by the GPU underneath; here only what goes over it, on the
        // panels' own layer after them (nothing else paints the canvas area), so every window
        // and floating panel stays on top of it.
        let painter = ctx.layer_painter(egui::LayerId::background()).with_clip_rect(canvas_rect);
        ed.update_outline();
        self.overlay(&painter, &ctx, ed, canvas_rect);
        if !self.canvas_only {
            self.tool_library_window(&ctx, ed);
        }
        self.toasts_ui(&ctx, canvas_rect);
        self.dialogs(&ctx, ed, &mut requests);
        // Text being written: its box (in Tool options) comes to the front when writing
        // starts, and picking another tool places the text, as in other painting apps (it
        // would otherwise stay on the canvas as a preview, never painted).
        if ed.text_edit.is_some() {
            if !self.writing {
                self.settings.dock.show(Panel::ToolOptions);
            }
            if ed.tool != Tool::Text && self.last_tool == Tool::Text {
                ed.place_text();
            }
        }
        self.writing = ed.text_edit.is_some();
        // Another tool puts the lasso transform's part down.
        if ed.transform.is_some() && ed.tool != Tool::LassoTransform {
            ed.commit_transform();
        }
        self.last_tool = ed.tool;
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
        // The more specific combinations first: a shortcut ignores extra Shift or Alt.
        let ctrl_alt = EguiMods::COMMAND | EguiMods::ALT;
        if hit(ctrl_alt, Key::J) {
            ed.selection_to_layer(false);
        } else if hit(ctrl_shift, Key::J) {
            ed.selection_to_layer(true);
        } else if hit(ctrl, Key::J) {
            ed.duplicate_layer();
        }
        if hit(ctrl_shift, Key::A) || hit(ctrl, Key::D) {
            ed.deselect();
        } else if hit(ctrl, Key::A) {
            ed.select_all();
        }
        // Ctrl+I too (hit() also takes Ctrl+Shift+I): a browser may open its developer tools
        // on Ctrl+Shift+I.
        if hit(ctrl, Key::I) {
            ed.invert_selection();
        }
        if hit(none, Key::Backspace) {
            ed.fill_selection();
        }
        if hit(none, Key::Enter) {
            ed.apply_crop();
            ed.commit_transform();
        }
        if hit(none, Key::X) {
            ed.swap_colors();
        }
        if hit(none, Key::Escape) {
            ed.cancel_crop();
            ed.cancel_text();
            ed.cancel_transform();
            if self.canvas_only {
                self.set_canvas_only(false, requests);
            }
        }
        if ed.tool == Tool::Move {
            for (key, dx, dy) in [(Key::ArrowLeft, -1, 0), (Key::ArrowRight, 1, 0), (Key::ArrowUp, 0, -1), (Key::ArrowDown, 0, 1)] {
                if hit(shift, key) {
                    ed.nudge(dx * 10, dy * 10);
                } else if hit(none, key) {
                    ed.nudge(dx, dy);
                }
            }
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
        if hit(none, Key::F) {
            ed.tool = Tool::Fill;
        }
        // G: only the canvas (again, or Escape: everything back); Shift+G: gradient.
        if hit(shift, Key::G) {
            ed.tool = Tool::Gradient;
        } else if hit(none, Key::G) {
            self.set_canvas_only(!self.canvas_only, requests);
        }
        if hit(none, Key::V) {
            ed.tool = Tool::Line;
        }
        for (key, tool) in [(Key::S, Tool::Smudge), (Key::R, Tool::SelectRect), (Key::L, Tool::SelectLasso), (Key::W, Tool::MagicWand), (Key::C, Tool::Crop), (Key::T, Tool::Text), (Key::Z, Tool::Zoom)] {
            if hit(none, key) {
                ed.tool = tool;
            }
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
            // A browser page cannot reopen files by their path, so it has no recent list.
            if !self.web {
                let recent = self.settings.recent_files.clone();
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
            }
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
            if item(ui, "Only the canvas", "G") {
                self.set_canvas_only(true, requests);
                ui.close();
            }
            ui.separator();
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
        ui.menu_button("Select", |ui| {
            if item(ui, "Select all", "Ctrl+A") {
                ed.select_all();
                ui.close();
            }
            let has = ed.has_selection();
            ui.add_enabled_ui(has, |ui| {
                if item(ui, "Deselect", "Ctrl+D") {
                    ed.deselect();
                    ui.close();
                }
                if item(ui, "Invert", "Ctrl+I") {
                    ed.invert_selection();
                    ui.close();
                }
            });
            ui.separator();
            if item(ui, if has { "Fill the selection with the colour" } else { "Fill the layer with the colour" }, "Backspace") {
                ed.fill_selection();
                ui.close();
            }
            if item(ui, if has { "Clear the selection" } else { "Clear the layer" }, "Delete") {
                ed.clear_layer();
                ui.close();
            }
            ui.add_enabled_ui(has, |ui| {
                ui.separator();
                if item(ui, "Copy to a new layer", "Ctrl+Alt+J") {
                    ed.selection_to_layer(false);
                    ui.close();
                }
                if item(ui, "Cut to a new layer", "Ctrl+Shift+J") {
                    ed.selection_to_layer(true);
                    ui.close();
                }
                if item(ui, "Crop the painting to it", "") {
                    ed.crop_to_selection();
                    ui.close();
                }
            });
        });
        ui.menu_button("Window", |ui| {
            for p in Panel::ALL {
                let mut open = self.settings.dock.is_open(p);
                if ui.checkbox(&mut open, p.title()).changed() {
                    self.settings.dock.toggle(p);
                }
            }
            if self.web {
                let mut open = self.reference_open;
                if ui.checkbox(&mut open, "Reference (search, pictures, videos)").changed() {
                    requests.push(Request::ToggleReference);
                }
            }
            ui.separator();
            if ui.button("Reset panels").clicked() {
                self.settings.dock = Default::default();
                ui.close();
            }
        });
        ui.menu_button("Help", |ui| {
            if ui.button("Getting started").clicked() {
                self.dialog = Some(Dialog::GettingStarted);
                ui.close();
            }
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
            Panel::Tools => self.sub_tool_panel(ui, ed),
            Panel::ToolOptions => {
                let name = self.sub_tool_name(ed);
                tool_options_panel(ui, ed, self.dialog.is_none(), &mut self.previews, &name);
            }
            Panel::Color => self.color_panel(ui, ed),
            Panel::Layers => self.layers_panel(ui, ed),
            Panel::History => history_panel(ui, ed),
        }
    }

    /// A panel's tab title; Sub tool and Tool property say what they show, as in Clip Studio.
    fn panel_title(&self, p: Panel, ed: &Editor) -> String {
        match p {
            Panel::Tools => match ed.tool {
                Tool::Brush | Tool::Eraser => format!("Sub tool [{}]", ed.tool.name()),
                t => format!("Sub tool [{}]", t.group().name()),
            },
            Panel::ToolOptions => format!("Tool property [{}]", self.sub_tool_name(ed)),
            other => other.title().to_string(),
        }
    }

    /// The brush or eraser preset picked last, or the tool's name.
    fn sub_tool_name(&self, ed: &Editor) -> String {
        let pick = |list: Vec<(&'static str, &'static str, Brush)>, i: Option<usize>| i.and_then(|i| list.get(i).map(|p| p.0.to_string()));
        match ed.tool {
            Tool::Brush => pick(presets(), self.brush_preset),
            Tool::Eraser => pick(eraser_presets(), self.eraser_preset),
            _ => None,
        }
        .unwrap_or_else(|| ed.tool.name().to_string())
    }

    /// The tool strip: the side bar's tools as icons (point at one for its name and key), the
    /// button for all the others at the top, and the colours at the bottom.
    fn tool_strip(&mut self, ui: &mut Ui, ed: &mut Editor) {
        self.strip.clear();
        let size = 26.0;
        ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            let r = icons::button(ui, size, self.tool_library, true, |p, r, c| icons::action(p, r, Action::AllTools, c))
                .on_hover_text(if self.tool_library { "Close the list of all tools" } else { "All tools: every tool, and which ones are on this bar" });
            self.strip.push((None, r.rect));
            if r.clicked() {
                self.tool_library = !self.tool_library;
            }
            ui.separator();
            let mut bar = self.settings.hotbar_tools();
            // A tool picked from the full list that is not on the bar still shows here.
            if !bar.contains(&ed.tool) {
                bar.push(ed.tool);
            }
            // Smaller icons when they would not all fit above the colours (a long bar on a
            // short screen); past the smallest size the strip scrolls.
            let swatch = size + 10.0;
            let room = ui.available_height() - swatch;
            // Each tool: its icon and name; shorter when they would not all fit.
            let icon = (room / bar.len().max(1) as f32 - 2.0).clamp(25.0, 33.0);
            egui::ScrollArea::vertical().id_salt("tool strip").max_height(room.max(icon)).scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
                for t in bar {
                    let tip = match t.key() {
                        Some(k) => format!("{} ({k})\n{}", t.name(), t.hint()),
                        None => format!("{}\n{}", t.name(), t.hint()),
                    };
                    let r = icons::tool_button(ui, STRIP_WIDTH - 4.0, icon, ed.tool == t, t.label(), |p, r, c| icons::tool(p, r, t, c)).on_hover_text(tip);
                    self.strip.push((Some(t), r.rect));
                    if r.clicked() {
                        ed.tool = t;
                    }
                }
            });
            ui.add_space(6.0);
            if crate::widgets::swatches(ui, size, ed.color, ed.color2).on_hover_text("The colour and the sub colour: click (or X) to swap them").clicked() {
                ed.swap_colors();
            }
        });
    }

    /// Clip Studio's Sub tool panel: for the brush and the eraser their presets, each with a
    /// stroke painted by it; for the other tools, the tools of the same kind.
    fn sub_tool_panel(&mut self, ui: &mut Ui, ed: &mut Editor) {
        let ppp = ui.ctx().pixels_per_point();
        match ed.tool {
            Tool::Brush | Tool::Eraser => {
                let eraser = ed.tool == Tool::Eraser;
                let list = if eraser { eraser_presets() } else { presets() };
                let chosen = if eraser { self.eraser_preset } else { self.brush_preset };
                ui.spacing_mut().item_spacing.y = 0.0;
                for (i, (name, tip, brush)) in list.into_iter().enumerate() {
                    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 23.0), egui::Sense::click());
                    let resp = resp.on_hover_text(tip);
                    let v = ui.visuals();
                    let selected = chosen == Some(i);
                    let fill = if selected { v.selection.bg_fill } else if resp.hovered() { v.widgets.hovered.weak_bg_fill } else { Color32::TRANSPARENT };
                    let text = if selected { v.selection.stroke.color } else { v.widgets.inactive.fg_stroke.color };
                    ui.painter().rect_filled(rect, 0.0, fill);
                    ui.painter().hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, Color32::from_gray(46)));
                    // The stroke on the left, the name on the right.
                    let pic = egui::Rect::from_min_max(rect.min + vec2(4.0, 2.0), egui::pos2(rect.left() + rect.width() * 0.58, rect.bottom() - 2.0));
                    let (w, h) = (((pic.width() * ppp) as u32).max(8), ((pic.height() * ppp) as u32).max(8));
                    let target = (brush.size.sqrt() * 2.2 * ppp).clamp(2.5 * ppp, h as f32 * 0.55);
                    let tex = self.previews.stroke(ui.ctx(), &previews::sized(&brush, target), [236, 236, 236], w, h);
                    ui.painter().image(tex.id(), pic, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                    ui.painter().text(egui::pos2(rect.right() - 6.0, rect.center().y), egui::Align2::RIGHT_CENTER, name, egui::FontId::proportional(11.0), text);
                    if resp.clicked() {
                        if eraser {
                            ed.tools.eraser = brush;
                            self.eraser_preset = Some(i);
                        } else {
                            ed.tools.brush = brush;
                            self.brush_preset = Some(i);
                        }
                    }
                }
            }
            current => {
                ui.spacing_mut().item_spacing.y = 0.0;
                for t in Tool::ALL.into_iter().filter(|t| t.group() == current.group()) {
                    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 23.0), egui::Sense::click());
                    let resp = resp.on_hover_text(t.hint());
                    let v = ui.visuals();
                    let selected = t == ed.tool;
                    let fill = if selected { v.selection.bg_fill } else if resp.hovered() { v.widgets.hovered.weak_bg_fill } else { Color32::TRANSPARENT };
                    let col = if selected { v.selection.stroke.color } else { v.widgets.inactive.fg_stroke.color };
                    let weak = v.weak_text_color();
                    ui.painter().rect_filled(rect, 0.0, fill);
                    ui.painter().hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, Color32::from_gray(46)));
                    icons::tool(ui.painter(), egui::Rect::from_center_size(egui::pos2(rect.left() + 13.0, rect.center().y), vec2(15.0, 15.0)), t, col);
                    ui.painter().text(egui::pos2(rect.left() + 27.0, rect.center().y), egui::Align2::LEFT_CENTER, t.name(), egui::FontId::proportional(11.0), col);
                    if let Some(k) = t.key() {
                        ui.painter().text(egui::pos2(rect.right() - 6.0, rect.center().y), egui::Align2::RIGHT_CENTER, k, egui::FontId::proportional(9.5), weak);
                    }
                    if resp.clicked() {
                        ed.tool = t;
                    }
                }
            }
        }
    }

    /// The icon toolbar under the menus.
    fn toolbar(&mut self, ui: &mut Ui, ed: &mut Editor, requests: &mut Vec<Request>) {
        ui.horizontal(|ui| {
            let size = 22.0;
            let act = |ui: &mut Ui, a: Action, enabled: bool, selected: bool, tip: &str| icons::button(ui, size, selected, enabled, |p, r, c| icons::action(p, r, a, c)).on_hover_text(tip).clicked();
            if act(ui, Action::New, true, false, "New painting (Ctrl+N)") {
                self.guarded(ed, Then::NewCanvas, requests);
            }
            if act(ui, Action::Open, true, false, "Open… (Ctrl+O)") {
                self.guarded(ed, Then::OpenDialog, requests);
            }
            if act(ui, Action::Save, true, false, "Save (Ctrl+S)") && !ed.save() {
                requests.push(Request::SaveAsDialog);
            }
            ui.separator();
            let (_, applied) = ed.history.entries();
            let redo = ed.history.can_redo();
            if act(ui, Action::Undo, applied > 0, false, "Undo (Ctrl+Z)") {
                ed.undo();
            }
            if act(ui, Action::Redo, redo, false, "Redo (Ctrl+Shift+Z)") {
                ed.redo();
            }
            ui.separator();
            if act(ui, Action::Smaller, true, false, "Smaller brush ([)") {
                ed.resize_brush(false);
            }
            ui.add_sized(vec2(38.0, size), egui::Label::new(format!("{:.0} px", ed.shown_brush_size()))).on_hover_text("Brush size");
            if act(ui, Action::Bigger, true, false, "Bigger brush (])") {
                ed.resize_brush(true);
            }
            ui.separator();
            if act(ui, Action::ZoomOut, true, false, "Zoom out (-)") {
                ed.zoom_out();
            }
            if act(ui, Action::ZoomIn, true, false, "Zoom in (+)") {
                ed.zoom_in();
            }
            if act(ui, Action::Fit, true, false, "Fit the painting to the window (2)") {
                ed.fit_to_window();
                ed.reset_rotation();
            }
            if self.web {
                ui.separator();
                if act(ui, Action::Reference, true, self.reference_open, "Reference: search pictures, videos and articles, or watch a YouTube link, beside the canvas") {
                    requests.push(Request::ToggleReference);
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(6.0);
                ui.label(RichText::new(ed.tool.name()).strong()).on_hover_text(ed.tool.hint());
                let (rect, _) = ui.allocate_exact_size(vec2(15.0, 15.0), egui::Sense::hover());
                icons::tool(ui.painter(), rect, ed.tool, ui.visuals().widgets.inactive.fg_stroke.color);
            });
        });
    }

    /// Every tool, by group: click one to use it, tick it to keep it on the side bar. On the
    /// right, the side bar's order.
    fn tool_library_window(&mut self, ctx: &egui::Context, ed: &mut Editor) {
        if !self.tool_library {
            return;
        }
        let mut open = true;
        let mut bar = self.settings.hotbar_tools();
        let before = bar.clone();
        // Nothing inside takes its width from the window (content as wide as the space it is
        // given makes a resizable window grow a little every frame); the lists fill its height.
        egui::Window::new("All tools").id(Id::unique("recedo tool library")).open(&mut open).collapsible(false).default_size(vec2(820.0, 520.0)).show(ctx, |ui| {
            ui.label("Click a tool to use it. Tick \"On bar\" to keep it on your side bar (point at a tool to see what it does).");
            ui.add_space(4.0);
            ui.horizontal_top(|ui| {
                // Top to bottom (a scroll area inside a row would lay the groups out in a row).
                ui.vertical(|ui| {
                    egui::ScrollArea::vertical().id_salt("all tools").auto_shrink([true, false]).show(ui, |ui| {
                        for group in ToolGroup::ALL {
                            ui.add_space(4.0);
                            ui.strong(group.name());
                            let tools: Vec<Tool> = Tool::ALL.into_iter().filter(|t| t.group() == group).collect();
                            egui::Grid::new(("tool group", group.name())).num_columns(6).spacing(vec2(6.0, 4.0)).min_col_width(56.0).show(ui, |ui| {
                                for row in tools.chunks(3) {
                                    for &t in row {
                                        let name = match t.key() {
                                            Some(k) => format!("{}  ({k})", t.name()),
                                            None => t.name().to_string(),
                                        };
                                        // A light outline, so each tool stands out as a box.
                                        let button = egui::Button::new(name).selected(ed.tool == t).stroke(Stroke::new(1.0, Color32::from_gray(220))).min_size(vec2(118.0, 20.0));
                                        if ui.add(button).on_hover_text(t.hint()).clicked() {
                                            ed.tool = t;
                                        }
                                        let mut on = bar.contains(&t);
                                        if ui.checkbox(&mut on, "On bar").changed() {
                                            if on {
                                                bar.push(t);
                                            } else if bar.len() > 1 {
                                                bar.retain(|b| *b != t);
                                            }
                                        }
                                    }
                                    ui.end_row();
                                }
                            });
                        }
                    });
                });
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    ui.strong("Your side bar");
                    ui.weak("Top to bottom");
                    egui::ScrollArea::vertical().id_salt("bar order").auto_shrink([true, false]).show(ui, |ui| {
                        let mut action = None;
                        for (i, t) in bar.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.add_sized(vec2(120.0, 20.0), egui::Label::new(t.name()).truncate());
                                if ui.add_enabled(i > 0, egui::Button::new("Up").small()).clicked() {
                                    action = Some((i, -1));
                                }
                                if ui.add_enabled(i + 1 < bar.len(), egui::Button::new("Down").small()).clicked() {
                                    action = Some((i, 1));
                                }
                                if ui.add_enabled(bar.len() > 1, egui::Button::new("Remove").small()).clicked() {
                                    action = Some((i, 0));
                                }
                            });
                        }
                        match action {
                            Some((i, 0)) => {
                                bar.remove(i);
                            }
                            Some((i, d)) => bar.swap(i, (i as i32 + d) as usize),
                            None => {}
                        }
                        ui.add_space(6.0);
                        if ui.button("Back to the standard tools").clicked() {
                            bar = Tool::DEFAULT_BAR.to_vec();
                        }
                    });
                });
            });
        });
        if bar != before {
            self.settings.set_hotbar(&bar);
        }
        if !open {
            self.tool_library = false;
        }
    }

    fn color_panel(&mut self, ui: &mut Ui, ed: &mut Editor) {
        ui.vertical_centered(|ui| crate::widgets::color_wheel(ui, &mut ed.color, &mut self.hsv));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if crate::widgets::swatches(ui, 22.0, ed.color, ed.color2).on_hover_text("The colour and the sub colour: click (or X) to swap them").clicked() {
                ed.swap_colors();
            }
            let [r, g, b] = ed.color;
            ui.label(RichText::new(format!("R {r}   G {g}   B {b}")).monospace());
        });
        ui.add_space(4.0);
        if !ed.recent_colors.is_empty() {
            // Colours painted with, newest first: tap one to paint with it again.
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
                for c in ed.recent_colors.clone() {
                    let (rect, resp) = ui.allocate_exact_size(vec2(16.0, 16.0), egui::Sense::click());
                    ui.painter().rect_filled(rect, 3.0, Color32::from_rgb(c[0], c[1], c[2]));
                    let stroke = if c == ed.color { Stroke::new(2.0, ui.visuals().selection.stroke.color) } else { Stroke::new(1.0, ui.visuals().widgets.inactive.fg_stroke.color) };
                    ui.painter().rect_stroke(rect, 3.0, stroke, egui::StrokeKind::Inside);
                    if resp.on_hover_text(format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])).clicked() {
                        ed.color = c;
                    }
                }
            });
        }
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
        // The selected layer's blend mode and opacity (how see-through it is), on top as in
        // Clip Studio.
        let i = ed.active;
        if let Some(l) = ed.doc.layers.get(i) {
            let mut opacity = (l.opacity as f32 / 255.0 * 100.0).round();
            let blend = l.blend;
            let mut mode = blend;
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("blend mode").width(ui.available_width() - 4.0).selected_text(mode.name()).show_ui(ui, |ui| {
                    for m in BlendMode::ALL {
                        ui.selectable_value(&mut mode, m, m.name());
                    }
                });
            });
            if mode != blend {
                ed.set_layer_blend(i, mode);
            }
            ui.horizontal(|ui| {
                let tip = "How see-through the selected layer is: 100 % solid, 0 % invisible";
                ui.label("Opacity").on_hover_text(tip);
                ui.spacing_mut().slider_width = (ui.available_width() - 70.0).max(50.0);
                let r = ui.add(egui::Slider::new(&mut opacity, 0.0..=100.0).suffix(" %").integer()).on_hover_text(tip);
                if r.changed() {
                    ed.set_layer_opacity(i, (opacity / 100.0 * 255.0).round() as u8);
                }
                if r.drag_stopped() || r.lost_focus() {
                    ed.end_edit();
                }
            });
        }
        ui.horizontal(|ui| {
            let act = |ui: &mut Ui, a: Action, enabled: bool, tip: &str| icons::button(ui, 20.0, false, enabled, |p, r, c| icons::action(p, r, a, c)).on_hover_text(tip).clicked();
            let n = ed.doc.layers.len();
            if act(ui, Action::LayerNew, true, "New layer above this one (Insert)") {
                ed.add_layer();
            }
            if act(ui, Action::LayerDuplicate, true, "Duplicate the layer (Ctrl+J)") {
                ed.duplicate_layer();
            }
            if act(ui, Action::LayerDelete, n > 1, "Delete the layer (Shift+Delete)") {
                ed.delete_layer();
            }
            if act(ui, Action::LayerUp, i + 1 < n, "Move the layer up (Ctrl+Page Up)") {
                ed.move_layer(true);
            }
            if act(ui, Action::LayerDown, i > 0, "Move the layer down (Ctrl+Page Down)") {
                ed.move_layer(false);
            }
            if act(ui, Action::LayerMerge, i > 0, "Merge into the layer below (Ctrl+E)") {
                ed.merge_down();
            }
        });
        ui.separator();
        // Top layer first, like Krita and Clip Studio: eye, lock, a small picture, the name,
        // and its opacity and blend mode.
        let ppp = ui.ctx().pixels_per_point();
        let (tw, th) = previews::thumbnail_size(&ed.doc, 28);
        let thumb = vec2(tw as f32, th as f32) * (28.0 / tw.max(th) as f32);
        let n = ed.doc.layers.len();
        ui.spacing_mut().item_spacing.y = 2.0;
        for i in (0..n).rev() {
            let (name, visible, locked, opacity, blend) = {
                let l = &ed.doc.layers[i];
                (l.name.clone(), l.visible, l.locked, l.opacity, l.blend)
            };
            // The row takes clicks first; the eye, lock and name field on it come after, so
            // they are on top and keep theirs.
            let (rect, row) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), egui::Sense::click());
            let v = ui.visuals();
            let selected = ed.active == i;
            let fill = if selected { v.selection.bg_fill } else if row.hovered() { v.widgets.hovered.weak_bg_fill } else { Color32::from_gray(66) };
            let text_col = if selected { v.selection.stroke.color } else { v.widgets.inactive.fg_stroke.color };
            ui.painter().rect_filled(rect, 2.0, fill);
            let mut row_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink2(vec2(3.0, 2.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
            use crate::widgets::{Icon, icon_toggle};
            if icon_toggle(&mut row_ui, visible, Icon::Eye).on_hover_text(if visible { "Visible (click to hide)" } else { "Hidden (click to show)" }).clicked() {
                ed.set_layer_visible(i, !visible);
            }
            if icon_toggle(&mut row_ui, locked, Icon::Lock).on_hover_text(if locked { "Locked: cannot be painted on (click to unlock)" } else { "Unlocked (click to lock)" }).clicked() {
                ed.set_layer_locked(i, !locked);
            }
            // The picture, on a checkerboard where the layer is empty.
            let (pic, _) = row_ui.allocate_exact_size(vec2(28.0, 28.0), egui::Sense::hover());
            let pic = egui::Rect::from_center_size(pic.center(), thumb);
            checkerboard(row_ui.painter(), pic, 4.0);
            if let Some(tex) = self.previews.thumbnail(row_ui.ctx(), &ed.doc, i, ed.pixels_rev, (28.0 * ppp) as u32) {
                row_ui.painter().image(tex.id(), pic, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
            }
            row_ui.painter().rect_stroke(pic, 0.0, Stroke::new(1.0, Color32::from_gray(30)), egui::StrokeKind::Outside);
            row_ui.add_space(4.0);
            row_ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                let pct = (opacity as f32 / 2.55).round();
                ui.label(RichText::new(format!("{pct} %  {}", blend.name())).size(9.0).color(text_col.gamma_multiply(0.75)));
                if let Some((ri, text)) = &mut self.rename
                    && *ri == i
                {
                    let r = ui.text_edit_singleline(text);
                    // Enter, Escape or a click elsewhere ends it. (Asking for the keys again
                    // every frame kept the box open for good, with every shortcut dead.)
                    if r.lost_focus() {
                        let (idx, new) = self.rename.take().expect("renaming");
                        if !ui.input(|i| i.key_pressed(Key::Escape)) {
                            ed.rename_layer(idx, &new);
                        }
                    } else if !r.has_focus() {
                        r.request_focus();
                    }
                } else {
                    let label = if visible { RichText::new(&name).color(text_col) } else { RichText::new(&name).color(text_col.gamma_multiply(0.5)) };
                    ui.add(egui::Label::new(label).truncate().selectable(false));
                }
            });
            let row = row.on_hover_text("Click to select, double-click to rename");
            if row.clicked() {
                ed.select_layer(i);
            }
            if row.double_clicked() {
                self.rename = Some((i, name));
            }
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
        let to = |p: (f64, f64)| pt(ed.view.to_screen(p.0, p.1));
        // Lines drawn dark under light, so they show on any colour.
        let ants = |shapes: &mut Vec<egui::Shape>, light: &mut Vec<egui::Shape>, a: egui::Pos2, b: egui::Pos2| {
            shapes.push(egui::Shape::line_segment([a, b], Stroke::new(2.5, Color32::from_black_alpha(190))));
            light.push(egui::Shape::line_segment([a, b], Stroke::new(1.0, Color32::from_gray(250))));
        };
        let (mut dark, mut light) = (Vec::new(), Vec::new());
        // The selection's outline (while moving it, where it goes).
        let moving = match ed.shape_preview() {
            Some((Tool::Move, a, b)) => Some((b.0 - a.0, b.1 - a.1)),
            _ => None,
        };
        let shift = moving.unwrap_or((0.0, 0.0));
        let to_moved = |p: (f64, f64)| to((p.0 + shift.0, p.1 + shift.1));
        match (ed.selection_outline(), &ed.doc.selection) {
            (Some(Some(segments)), _) => {
                for [a, b] in segments {
                    let (sa, sb) = (to_moved(*a), to_moved(*b));
                    if canvas.intersects(egui::Rect::from_two_pos(sa, sb).expand(2.0)) {
                        ants(&mut dark, &mut light, sa, sb);
                    }
                }
            }
            (Some(None), Some(m)) => {
                // Too intricate to outline: its bounding box instead.
                let (x0, y0, x1, y1) = (m.bounds.0 as f64, m.bounds.1 as f64, m.bounds.2 as f64, m.bounds.3 as f64);
                let c = [to_moved((x0, y0)), to_moved((x1, y0)), to_moved((x1, y1)), to_moved((x0, y1))];
                for i in 0..4 {
                    ants(&mut dark, &mut light, c[i], c[(i + 1) % 4]);
                }
            }
            _ => {}
        }
        // Moving a whole layer: the outline of what it holds.
        if let (Some(d), None) = (moving, &ed.doc.selection)
            && let Some(l) = ed.doc.layers.get(ed.active)
        {
            let keys: Vec<_> = l.pixels.tile_keys().collect();
            if !keys.is_empty() {
                let t = recedo_core::tiles::TILE as f64;
                let (x0, y0) = (keys.iter().map(|k| k.0).min().unwrap_or(0) as f64 * t, keys.iter().map(|k| k.1).min().unwrap_or(0) as f64 * t);
                let (x1, y1) = ((keys.iter().map(|k| k.0).max().unwrap_or(0) + 1) as f64 * t, (keys.iter().map(|k| k.1).max().unwrap_or(0) + 1) as f64 * t);
                let c = [to((x0 + d.0, y0 + d.1)), to((x1 + d.0, y0 + d.1)), to((x1 + d.0, y1 + d.1)), to((x0 + d.0, y1 + d.1))];
                for i in 0..4 {
                    ants(&mut dark, &mut light, c[i], c[(i + 1) % 4]);
                }
            }
        }
        // A lasso being drawn, closed back to its start.
        if let Some(points) = ed.lasso_preview()
            && points.len() >= 2
        {
            for w in points.windows(2) {
                ants(&mut dark, &mut light, to(w[0]), to(w[1]));
            }
            light.push(egui::Shape::line_segment([to(points[points.len() - 1]), to(points[0])], Stroke::new(1.0, Color32::from_white_alpha(110))));
        }
        // The lasso transform: the outline drawn, scaled with the part, its box and handles.
        if let Some(t) = &ed.transform {
            let (s, r) = (t.src, t.rect);
            let map = |p: (f64, f64)| {
                let u = if s.2 > s.0 { (p.0 - s.0) / (s.2 - s.0) } else { 0.0 };
                let v = if s.3 > s.1 { (p.1 - s.1) / (s.3 - s.1) } else { 0.0 };
                to((r.0 + u * (r.2 - r.0), r.1 + v * (r.3 - r.1)))
            };
            let n = t.outline.len();
            for i in 0..n {
                light.push(egui::Shape::line_segment([map(t.outline[i]), map(t.outline[(i + 1) % n])], Stroke::new(1.0, Color32::from_white_alpha(140))));
            }
            let c = [to((r.0, r.1)), to((r.2, r.1)), to((r.2, r.3)), to((r.0, r.3))];
            for i in 0..4 {
                ants(&mut dark, &mut light, c[i], c[(i + 1) % 4]);
            }
            let (mx, my) = ((r.0 + r.2) / 2.0, (r.1 + r.3) / 2.0);
            for h in c.into_iter().chain([to((mx, r.1)), to((r.2, my)), to((mx, r.3)), to((r.0, my))]) {
                let b = egui::Rect::from_center_size(h, vec2(7.0, 7.0));
                light.push(egui::Shape::rect_filled(b, 1.0, Color32::WHITE));
                light.push(egui::Shape::rect_stroke(b, 1.0, Stroke::new(1.0, Color32::from_gray(30)), egui::StrokeKind::Outside));
            }
        }
        // The crop rectangle, and the rest of the canvas dimmed.
        let crop = match ed.shape_preview() {
            Some((Tool::Crop, a, b)) => Some((a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1))),
            _ => ed.crop_rect.map(|(x, y, w, h)| (x as f64, y as f64, (x + w as i32) as f64, (y + h as i32) as f64)),
        };
        if let Some((x0, y0, x1, y1)) = crop {
            let (w, h) = (ed.doc.width as f64, ed.doc.height as f64);
            let c = [to((x0, y0)), to((x1, y0)), to((x1, y1)), to((x0, y1))];
            // One mesh for the dimmed ring between the canvas edge and the rectangle (separate
            // anti-aliased pieces would leave light seams where they meet).
            let dim = Color32::from_black_alpha(130);
            let mut ring = egui::Mesh::default();
            for p in [to((0.0, 0.0)), to((w, 0.0)), to((w, h)), to((0.0, h))].into_iter().chain(c) {
                ring.colored_vertex(p, dim);
            }
            for i in 0..4 {
                let j = (i + 1) % 4;
                ring.add_triangle(i, j, 4 + j);
                ring.add_triangle(i, 4 + j, 4 + i);
            }
            painter.add(egui::Shape::mesh(ring));
            for i in 0..4 {
                ants(&mut dark, &mut light, c[i], c[(i + 1) % 4]);
            }
            label_on_canvas(painter, c[0] + vec2(4.0, 4.0), format!("{:.0} × {:.0}", x1 - x0, y1 - y0));
        }
        // The measuring line, while dragging and afterwards.
        let measured = match ed.shape_preview() {
            Some((Tool::Measure, a, b)) => Some((a, b)),
            _ if ed.tool == Tool::Measure => ed.measure,
            _ => None,
        };
        if let Some((a, b)) = measured {
            ants(&mut dark, &mut light, to(a), to(b));
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let label = format!("{:.1} px, {:.1}°", dx.hypot(dy), (-dy).atan2(dx).to_degrees());
            label_on_canvas(painter, to(b) + vec2(10.0, 10.0), label);
        }
        // The clone tool's source.
        if ed.tool == Tool::Clone
            && let Some(src) = ed.clone_source
        {
            let c = to(src);
            for (r, col) in [(9.0, Color32::from_black_alpha(200)), (8.0, Color32::WHITE)] {
                painter.circle_stroke(c, r, Stroke::new(1.5, col));
            }
            painter.line_segment([c - vec2(12.0, 0.0), c + vec2(12.0, 0.0)], Stroke::new(1.0, Color32::WHITE));
            painter.line_segment([c - vec2(0.0, 12.0), c + vec2(0.0, 12.0)], Stroke::new(1.0, Color32::WHITE));
        }
        // The text being written, roughly as it will look (the same fonts), turned and
        // mirrored with the view like the painting under it.
        if let Some(t) = &ed.text_edit {
            let colour = Color32::from_rgb(ed.color[0], ed.color[1], ed.color[2]);
            let family = if ed.tools.text.font == Font::Mono { egui::FontFamily::Monospace } else { egui::FontFamily::Name(TEXT_SANS.into()) };
            let size = ((ed.tools.text.size as f64 * ed.view.zoom) as f32 / ppp).max(1.0);
            let at = to(t.at);
            // Where the canvas's right and down directions point on screen.
            let (sin, cos) = (ed.view.angle as f32).sin_cos();
            let m = if ed.view.mirror { -1.0 } else { 1.0 };
            let (right, down) = (vec2(cos * m, sin * m), vec2(-sin, cos));
            if t.text.is_empty() {
                painter.line_segment([at, at + down * size], Stroke::new(2.0, colour));
            } else {
                // Laid out upright at `at`, then each corner of its triangles moved.
                let galley = painter.layout_no_wrap(t.text.clone(), egui::FontId::new(size, family), colour);
                let shape = egui::epaint::ClippedShape::new(egui::Rect::EVERYTHING, egui::Shape::galley(at, galley, colour));
                for p in ctx.tessellate(vec![shape], ctx.pixels_per_point()) {
                    if let egui::epaint::Primitive::Mesh(mut mesh) = p.primitive {
                        for v in &mut mesh.vertices {
                            let d = v.pos - at;
                            v.pos = at + right * d.x + down * d.y;
                        }
                        painter.add(egui::Shape::mesh(mesh));
                    }
                }
            }
            painter.circle_filled(at, 3.0, Color32::from_rgb(80, 140, 230));
        }
        painter.extend(dark);
        painter.extend(light);
        if let Some((tool, a, b)) = ed.shape_preview() {
            let [r, g, bl] = ed.color;
            let width = if tool == Tool::Gradient { 1.5 } else { (ed.tools.brush.size as f64 * ed.view.zoom / ppp as f64).max(1.0) as f32 };
            let s = Stroke::new(width, Color32::from_rgb(r, g, bl));
            let guide = Stroke::new(1.0, Color32::from_white_alpha(170));
            match tool {
                Tool::Line | Tool::Gradient => {
                    painter.line_segment([to(a), to(b)], s);
                    if tool == Tool::Gradient {
                        painter.circle_stroke(to(a), 5.0, guide);
                        painter.circle_stroke(to(b), 5.0, guide);
                    }
                }
                Tool::Rectangle => {
                    painter.add(egui::Shape::closed_line(vec![to(a), to((b.0, a.1)), to(b), to((a.0, b.1))], s));
                }
                Tool::SelectRect => {
                    let c = [to(a), to((b.0, a.1)), to(b), to((a.0, b.1))];
                    painter.add(egui::Shape::closed_line(c.to_vec(), Stroke::new(2.5, Color32::from_black_alpha(190))));
                    painter.add(egui::Shape::closed_line(c.to_vec(), guide));
                }
                Tool::SelectEllipse => {
                    let (cx, cy, rx, ry) = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0, (a.0 - b.0).abs() / 2.0, (a.1 - b.1).abs() / 2.0);
                    let pts: Vec<egui::Pos2> = (0..96).map(|i| i as f64 / 96.0 * std::f64::consts::TAU).map(|t| to((cx + rx * t.cos(), cy + ry * t.sin()))).collect();
                    painter.add(egui::Shape::closed_line(pts.clone(), Stroke::new(2.5, Color32::from_black_alpha(190))));
                    painter.add(egui::Shape::closed_line(pts, guide));
                }
                Tool::Crop | Tool::Measure | Tool::Move => {}
                _ => {
                    let (cx, cy, rx, ry) = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0, (a.0 - b.0).abs() / 2.0, (a.1 - b.1).abs() / 2.0);
                    let pts = (0..96).map(|i| i as f64 / 96.0 * std::f64::consts::TAU).map(|t| to((cx + rx * t.cos(), cy + ry * t.sin()))).collect();
                    painter.add(egui::Shape::closed_line(pts, s));
                }
            }
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
                        // As wide as the message, up to 560 points (the area keeps the width it
                        // had and would wrap a longer message into a narrow column).
                        let font = egui::TextStyle::Body.resolve(ui.style());
                        ui.set_width(ui.painter().layout_no_wrap(text.clone(), font, color).size().x.min(560.0));
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
                Dialog::GettingStarted => {
                    ui.heading("Getting started");
                    let tall = (ctx.content_rect().height() - 140.0).max(160.0);
                    egui::ScrollArea::vertical().max_height(tall).show(ui, |ui| {
                        ui.set_max_width(520.0);
                        let tip = |ui: &mut Ui, head: &str, text: &str| {
                            ui.add_space(6.0);
                            ui.strong(head);
                            ui.label(text);
                        };
                        tip(ui, "Draw", "Paint with the pen or mouse. [ and ] change the brush size, E switches to the eraser, Ctrl+Z undoes. Pressing harder paints thicker.");
                        tip(ui, "Tools", "The icons on the far left are your tools (point at one for its name and key). The grid button at the top lists all 32: click one to use it, tick \"On bar\" to keep it there. Sub tool shows the brushes (or the tools of the same kind), Tool property their settings.");
                        tip(ui, "Move around", "Space + drag pans, the mouse wheel zooms, 2 fits the painting to the window, 4 and 6 turn it, 5 straightens it. G hides everything but the painting (G or Escape brings it back).");
                        tip(ui, "Layers and colour", "The colour wheel is on the left: the ring picks the colour, the square how light and strong it is; X swaps it with the second colour. Layers are on the right: the Opacity slider there makes the selected layer see-through. Ctrl+click on the painting picks a colour.");
                        tip(ui, "Selections", "Box select (R), the magic wand (W) and the others keep painting inside the selection. The Select menu has the rest.");
                        tip(ui, "Scaling part of a layer", "The lasso transform: draw round what to change, then drag a corner to make it bigger or smaller (Shift keeps its shape), or drag inside to move it. Enter puts it down, Escape cancels.");
                        if self.web {
                            tip(ui, "Reference", "\"Reference\" in the toolbar opens a box on the right to search pictures, videos and articles, or play a YouTube link, while you draw.");
                            tip(ui, "Saving", "File > Save as… downloads a .kra file (Krita opens it too); File > Export as PNG or JPEG for a picture. Your painting is also kept in this browser, so it comes back if the tab closes.");
                        } else {
                            tip(ui, "Saving", "File > Save (.kra, Krita opens it too); File > Export as PNG or JPEG for a picture. A copy is saved every few minutes, so a crash loses little.");
                        }
                        tip(ui, "Panels", "Drag a panel's tab to move it; the Window menu shows or hides panels.");
                    });
                    ui.add_space(6.0);
                    if ui.button("Close").clicked() {
                        keep = false;
                    }
                }
                Dialog::Shortcuts => {
                    ui.heading("Keyboard shortcuts");
                    // Scrolls when the screen is not tall enough for the whole list.
                    let tall = (ctx.content_rect().height() - 140.0).max(160.0);
                    egui::ScrollArea::vertical().max_height(tall).show(ui, |ui| {
                        egui::Grid::new("keys").striped(true).show(ui, |ui| {
                            for (k, what) in SHORTCUTS {
                                ui.label(RichText::new(*k).monospace());
                                ui.label(*what);
                                ui.end_row();
                            }
                        });
                    });
                    if ui.button("Close").clicked() {
                        keep = false;
                    }
                }
                Dialog::About => {
                    ui.heading("Recedo");
                    ui.label(format!("Version {VERSION} ({})", option_env!("RECEDO_GIT_HASH").unwrap_or("dev")));
                    ui.label("A painting app for pen tablets, compatible with Krita's .kra files.");
                    ui.label(if self.web {
                        "Licence: GPL-3.0-or-later. Runs in your browser; your paintings stay on this computer."
                    } else {
                        "Licence: GPL-3.0-or-later. See LICENCES.txt for the parts it uses."
                    });
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

/// Big buttons for what a pen or a finger cannot reach by keyboard: undo, redo, brush or
/// eraser, brush size, fit, zoom.
/// `reference`: in the browser version, whether the reference panel is open. Returns true
/// when its button was clicked.
/// Ready-made erasers for the Sub tool panel.
fn eraser_presets() -> Vec<(&'static str, &'static str, Brush)> {
    let base = Brush::default();
    vec![
        ("Hard eraser", "Hard edge; pressing harder erases wider", Brush { size: 20.0, hardness: 1.0, ..base.clone() }),
        ("Soft eraser", "Soft edge, for fading paint out", Brush { size: 40.0, hardness: 0.2, ..base.clone() }),
        ("Fine eraser", "Thin and even, for details", Brush { size: 4.0, hardness: 1.0, size_curve: None, ..base.clone() }),
        ("Big soft eraser", "Wide and very soft; takes paint away a little at a time", Brush { size: 150.0, hardness: 0.0, flow: 0.15, mode: PaintMode::BuildUp, size_curve: None, flow_curve: Some(Curve::linear()), ..base }),
    ]
}

/// Ready-made brushes (Krita has a library of these; these are the basics).
fn presets() -> Vec<(&'static str, &'static str, Brush)> {
    let base = Brush::default();
    vec![
        ("Paint brush", "The default round brush", base.clone()),
        ("Pencil", "Thin, a little soft, lighter when you press lightly", Brush { size: 4.0, hardness: 0.6, opacity_curve: Some(Curve::linear()), min_size: 0.5, ..base.clone() }),
        ("Ink pen", "Hard edge, thin to thick with pressure", Brush { size: 10.0, hardness: 1.0, min_size: 0.1, ..base.clone() }),
        ("Fine liner", "Thin line of one even width", Brush { size: 3.0, hardness: 1.0, size_curve: None, ..base.clone() }),
        ("Felt-tip", "Even strokes, slightly soft", Brush { size: 14.0, hardness: 0.85, opacity: 0.9, size_curve: None, ..base.clone() }),
        ("Marker", "Even, see-through strokes that do not darken where they cross", Brush { size: 28.0, hardness: 0.75, opacity: 0.55, size_curve: None, ..base.clone() }),
        ("Highlighter", "Wide flat tip, see-through, even", Brush { size: 40.0, hardness: 0.95, opacity: 0.35, size_curve: None, roundness: 0.3, angle: 70.0, ..base.clone() }),
        ("Calligraphy", "Flat nib at an angle: thick and thin strokes by direction", Brush { size: 26.0, hardness: 1.0, roundness: 0.22, angle: 35.0, min_size: 0.35, ..base.clone() }),
        ("Watercolour", "Soft and see-through; builds up where you go over it", Brush { size: 70.0, hardness: 0.15, opacity: 0.5, flow: 0.12, mode: PaintMode::BuildUp, size_curve: Some(Curve::linear()), min_size: 0.4, ..base.clone() }),
        ("Airbrush", "Big and soft; builds up colour as you go over it", Brush { size: 150.0, hardness: 0.0, flow: 0.08, mode: PaintMode::BuildUp, size_curve: None, flow_curve: Some(Curve::linear()), ..base }),
    ]
}

/// Small grey and white squares, under pictures with see-through parts.
fn checkerboard(p: &egui::Painter, rect: egui::Rect, cell: f32) {
    p.rect_filled(rect, 0.0, Color32::from_gray(235));
    let (nx, ny) = ((rect.width() / cell).ceil() as i32, (rect.height() / cell).ceil() as i32);
    for y in 0..ny {
        for x in 0..nx {
            if (x + y) % 2 == 1 {
                let r = egui::Rect::from_min_size(rect.min + vec2(x as f32, y as f32) * cell, vec2(cell, cell)).intersect(rect);
                p.rect_filled(r, 0.0, Color32::from_gray(200));
            }
        }
    }
}

/// The current brush's stroke on a light box, in the colour, at its real size when it fits
/// (Clip Studio shows one at the top of Tool property).
fn brush_preview(ui: &mut Ui, ed: &Editor, previews: &mut Previews, name: &str) {
    let ppp = ui.ctx().pixels_per_point();
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 48.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 3.0, Color32::from_gray(226));
    let eraser = ed.tool == Tool::Eraser;
    let brush = if eraser { &ed.tools.eraser } else { &ed.tools.brush };
    // The stroke under the name.
    let area = egui::Rect::from_min_max(rect.min + vec2(4.0, 12.0), rect.max - vec2(4.0, 1.0));
    let (w, h) = ((area.width() * ppp) as u32, (area.height() * ppp) as u32);
    let size = (brush.size * ppp).min(h as f32 * 0.7).max(1.0);
    let color = if eraser { [90, 90, 90] } else { ed.color };
    let tex = previews.stroke(ui.ctx(), &previews::sized(brush, size), color, w.max(8), h.max(8));
    let pic = egui::Rect::from_min_size(area.min, vec2(w as f32, h as f32) / ppp);
    ui.painter().image(tex.id(), pic, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
    ui.painter().text(rect.min + vec2(4.0, 2.0), egui::Align2::LEFT_TOP, name, egui::FontId::proportional(9.5), Color32::from_gray(70));
    ui.add_space(4.0);
}

/// `may_focus`: no dialog is open, so the text tool's box may take the keyboard. `name`: the
/// preset picked in Sub tool (or the tool's name).
fn tool_options_panel(ui: &mut Ui, ed: &mut Editor, may_focus: bool, previews: &mut Previews, name: &str) {
    match ed.tool {
        Tool::Brush | Tool::Eraser => {
            brush_preview(ui, ed, previews, name);
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
            let mut round = (b.roundness * 100.0).round();
            if ui.add(egui::Slider::new(&mut round, 5.0..=100.0).text("Roundness").suffix(" %").integer()).on_hover_text("100 % is round; less is a flat tip, like a highlighter or a calligraphy nib").changed() {
                b.roundness = round / 100.0;
            }
            if b.roundness < 1.0 {
                ui.add(egui::Slider::new(&mut b.angle, -90.0..=90.0).text("Tip angle").suffix("°").max_decimals(0));
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
        Tool::Fill => {
            ui.strong("Fill");
            let mut tol = (ed.tools.fill_tolerance * 100.0).round();
            if ui.add(egui::Slider::new(&mut tol, 0.0..=100.0).text("Tolerance").suffix(" %").integer()).on_hover_text("How different a colour may be and still be filled").changed() {
                ed.tools.fill_tolerance = tol / 100.0;
            }
            ui.radio_value(&mut ed.tools.fill_all_layers, true, "Find the area in the visible picture");
            ui.radio_value(&mut ed.tools.fill_all_layers, false, "Find the area in this layer only");
            ui.weak("Fills the selected layer with the colour, a pixel under the edges of lines so no gap shows.");
        }
        Tool::Line | Tool::Rectangle | Tool::Ellipse => {
            ui.strong(ed.tool.name());
            if ed.tool != Tool::Line {
                ui.checkbox(&mut ed.tools.shape_filled, "Filled with the colour");
            }
            ui.add(egui::Slider::new(&mut ed.tools.brush.size, MIN_BRUSH..=MAX_BRUSH).logarithmic(true).text("Line width").suffix(" px").max_decimals(0));
            ui.weak(match ed.tool {
                Tool::Line => "Drag from start to end. Hold Shift for angles in 15° steps. Uses the brush.",
                Tool::Rectangle => "Drag from corner to corner. Hold Shift for a square. Uses the brush.",
                _ => "Drag across the box it fits in. Hold Shift for a circle. Uses the brush.",
            });
        }
        Tool::Gradient => {
            ui.strong("Gradient");
            ui.weak("Drag from where the colour is full to where it fades out. Hold Shift for angles in 15° steps.");
        }
        Tool::LassoTransform => {
            ui.strong("Lasso transform");
            let layer = ed.doc.layers.get(ed.active).map_or(String::new(), |l| l.name.clone());
            match &ed.transform {
                None => {
                    ui.label(format!("Draw around part of the selected layer (\"{layer}\"). Only that layer changes."));
                }
                Some(t) => {
                    let (w0, h0) = (t.src.2 - t.src.0, t.src.3 - t.src.1);
                    let (w, h) = (t.rect.2 - t.rect.0, t.rect.3 - t.rect.1);
                    ui.label(format!("{w0:.0} × {h0:.0} px, now {w:.0} × {h:.0} px"));
                    let mut pct = (w / w0 * 100.0).round();
                    if ui.add(egui::Slider::new(&mut pct, 5.0..=400.0).logarithmic(true).text("Size").suffix(" %").max_decimals(0)).on_hover_text("Scales it evenly about its middle").changed() {
                        ed.scale_transform(pct);
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Put down (Enter)").clicked() {
                            ed.commit_transform();
                        }
                        if ui.button("Cancel (Escape)").clicked() {
                            ed.cancel_transform();
                        }
                    });
                }
            }
            ui.weak("Drag a corner to scale (Shift keeps the shape), the middle of a side to stretch one way, inside to move. Another tool puts it down too.");
        }
        Tool::Pan => {
            ui.strong("Pan");
            ui.label("Drag to move the view.");
            ui.weak("With any tool: Space+drag pans, Shift+Space+drag rotates, Ctrl+Space+drag zooms.");
        }
        Tool::Smudge | Tool::Blur | Tool::Sharpen | Tool::Dodge | Tool::Burn | Tool::Sponge | Tool::Recolour | Tool::Clone | Tool::Push | Tool::Spray => {
            ui.strong(ed.tool.name());
            ui.weak(ed.tool.hint());
            let r = &mut ed.tools.retouch;
            ui.add(egui::Slider::new(&mut r.size, MIN_BRUSH..=MAX_BRUSH).logarithmic(true).text("Size").suffix(" px").max_decimals(0)).on_hover_text("[ and ] change the size");
            percent(ui, &mut r.strength, "Strength", "How strong one pass of the brush is; pressing lightly makes it weaker");
            percent(ui, &mut r.hardness, "Hardness", "0 % fades out from the middle, 100 % works evenly up to the edge");
            match ed.tool {
                Tool::Smudge => percent(ui, &mut r.smudge_length, "Length", "How far the paint is dragged"),
                Tool::Spray => {
                    ui.add(egui::Slider::new(&mut r.spray_density, 1.0..=100.0).text("Dots").max_decimals(0)).on_hover_text("Dots per step of the brush");
                    ui.add(egui::Slider::new(&mut r.spray_dot, 0.5..=50.0).logarithmic(true).text("Dot size").suffix(" px").max_decimals(1));
                    ui.weak("Sprays the colour.");
                }
                Tool::Sponge => {
                    ui.radio_value(&mut ed.tools.sponge_saturates, false, "Make colours more grey");
                    ui.radio_value(&mut ed.tools.sponge_saturates, true, "Make colours more vivid");
                }
                Tool::Recolour => {
                    ui.weak("Uses the colour (Ctrl+click picks one).");
                }
                Tool::Clone => match ed.clone_source {
                    Some((x, y)) => {
                        ui.label(format!("Copying from {:.0}, {:.0}", x, y));
                        if ui.button("Choose another source").on_hover_text("Or Ctrl+click on the canvas").clicked() {
                            ed.forget_clone_source();
                        }
                    }
                    None => {
                        ui.label(RichText::new("Ctrl+click where to copy from first.").strong());
                    }
                },
                _ => {}
            }
        }
        Tool::MagicEraser | Tool::MagicWand => {
            ui.strong(ed.tool.name());
            ui.weak(ed.tool.hint());
            percent(ui, &mut ed.tools.wand_tolerance, "Tolerance", "How different a colour may be and still count");
            ui.radio_value(&mut ed.tools.wand_all_layers, true, "Find the area in the visible picture");
            ui.radio_value(&mut ed.tools.wand_all_layers, false, "Find the area in this layer only");
            if ed.tool == Tool::MagicWand {
                selection_controls(ui, ed);
            }
        }
        Tool::LassoFill => {
            ui.strong("Lasso fill");
            ui.weak(ed.tool.hint());
            ui.label("Draw around the area; lifting the pen closes the outline and fills it with the colour.");
        }
        Tool::SelectRect | Tool::SelectEllipse | Tool::SelectLasso => {
            ui.strong(ed.tool.name());
            ui.weak(ed.tool.hint());
            selection_controls(ui, ed);
        }
        Tool::Move => {
            ui.strong("Move");
            ui.label(if ed.has_selection() { "Moves the selected pixels (and the selection)." } else { "Moves the whole selected layer." });
            ui.weak("Drag on the canvas. Arrow keys move 1 px, Shift+arrow 10 px. Shift while dragging keeps it level.");
        }
        Tool::Crop => {
            ui.strong("Crop");
            match ed.crop_rect {
                Some((x, y, w, h)) => {
                    ui.label(format!("Keep {w} × {h} px from {x}, {y}"));
                    ui.horizontal(|ui| {
                        if ui.button("Crop").on_hover_text("Enter").clicked() {
                            ed.apply_crop();
                        }
                        if ui.button("Cancel").on_hover_text("Escape").clicked() {
                            ed.cancel_crop();
                        }
                    });
                }
                None => {
                    ui.weak(ed.tool.hint());
                }
            }
            if ed.has_selection() && ui.button("Crop to the selection").clicked() {
                ed.crop_to_selection();
            }
        }
        Tool::Text => text_options(ui, ed, may_focus),
        Tool::Zoom => {
            ui.strong("Zoom");
            ui.weak(ed.tool.hint());
        }
        Tool::Rotate => {
            ui.strong("Rotate view");
            ui.weak(ed.tool.hint());
            if ui.button("Straighten").on_hover_text("5").clicked() {
                ed.reset_rotation();
            }
        }
        Tool::Measure => {
            ui.strong("Measure");
            ui.weak(ed.tool.hint());
            if let Some((a, b)) = ed.measure {
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                ui.label(format!("Length: {:.1} px", dx.hypot(dy)));
                ui.label(format!("Angle: {:.1}°", (-dy).atan2(dx).to_degrees()));
                ui.label(format!("Across {:.1} px, down {:.1} px", dx, dy));
            }
        }
    }
}

/// Text over the canvas, light on a dark patch so it shows on any picture.
fn label_on_canvas(painter: &egui::Painter, at: egui::Pos2, text: String) {
    let galley = painter.layout_no_wrap(text, egui::FontId::proportional(13.0), Color32::WHITE);
    let rect = egui::Rect::from_min_size(at, galley.size()).expand(3.0);
    painter.rect_filled(rect, 3.0, Color32::from_black_alpha(170));
    painter.galley(at, galley, Color32::WHITE);
}

/// A 0..=1 value as a percentage slider.
fn percent(ui: &mut Ui, v: &mut f32, label: &str, tip: &str) {
    let mut p = (*v * 100.0).round();
    if ui.add(egui::Slider::new(&mut p, 0.0..=100.0).text(label).suffix(" %").integer()).on_hover_text(tip).changed() {
        *v = p / 100.0;
    }
}

/// How new selections combine, and what can be done with the selection.
fn selection_controls(ui: &mut Ui, ed: &mut Editor) {
    ui.add_space(4.0);
    ui.label("A new selection:");
    ui.horizontal_wrapped(|ui| {
        ui.radio_value(&mut ed.tools.select_mode, Combine::Replace, "Replaces").on_hover_text("Click without dragging to deselect");
        ui.radio_value(&mut ed.tools.select_mode, Combine::Add, "Adds").on_hover_text("Or hold Shift when you start");
        ui.radio_value(&mut ed.tools.select_mode, Combine::Subtract, "Takes away").on_hover_text("Or hold Alt when you start");
        ui.radio_value(&mut ed.tools.select_mode, Combine::Intersect, "Keeps the overlap").on_hover_text("Or hold Shift and Alt");
    });
    let has = ed.has_selection();
    ui.horizontal_wrapped(|ui| {
        if ui.button("Select all").on_hover_text("Ctrl+A").clicked() {
            ed.select_all();
        }
        if ui.add_enabled(has, egui::Button::new("Deselect")).on_hover_text("Ctrl+Shift+A").clicked() {
            ed.deselect();
        }
        if ui.add_enabled(has, egui::Button::new("Invert")).on_hover_text("Ctrl+I").clicked() {
            ed.invert_selection();
        }
    });
    if has {
        ui.horizontal_wrapped(|ui| {
            if ui.button("Fill with colour").on_hover_text("Backspace").clicked() {
                ed.fill_selection();
            }
            if ui.button("Clear").on_hover_text("Delete").clicked() {
                ed.clear_layer();
            }
            if ui.button("Copy to new layer").on_hover_text("Ctrl+Alt+J").clicked() {
                ed.selection_to_layer(false);
            }
            if ui.button("Cut to new layer").on_hover_text("Ctrl+Shift+J").clicked() {
                ed.selection_to_layer(true);
            }
            if ui.button("Crop to it").clicked() {
                ed.crop_to_selection();
            }
        });
        ui.weak("Painting and filling stay inside the selection.");
    }
}

fn text_options(ui: &mut Ui, ed: &mut Editor, may_focus: bool) {
    ui.strong("Text");
    let st = &mut ed.tools.text;
    ui.horizontal(|ui| {
        for f in Font::ALL {
            ui.radio_value(&mut st.font, f, f.name());
        }
        ui.checkbox(&mut st.bold, "Bold");
    });
    ui.add(egui::Slider::new(&mut st.size, 4.0..=600.0).logarithmic(true).text("Size").suffix(" px").max_decimals(0));
    ui.add(egui::Slider::new(&mut st.line_spacing, 0.5..=3.0).text("Line spacing").max_decimals(2));
    let (mut place, mut cancel) = (false, false);
    match &mut ed.text_edit {
        None => {
            ui.weak("Click on the canvas where the text should start.");
        }
        Some(t) => {
            let edit = ui.add(egui::TextEdit::multiline(&mut t.text).desired_rows(3).desired_width(f32::INFINITY).hint_text("Type the text here"));
            // Escape cancels. Otherwise the keys go to the text while it is being written,
            // unless another field has them: a click on the canvas (to move the text) takes
            // them away.
            cancel = may_focus && ui.input(|i| i.key_pressed(Key::Escape));
            let elsewhere = ui.ctx().memory(|m| m.focused()).is_some_and(|id| id != edit.id);
            if may_focus && !cancel && !edit.has_focus() && !elsewhere {
                edit.request_focus();
            }
            // Ctrl+Enter, read from the key press itself.
            let ctrl_enter = |e: &egui::Event| matches!(e, egui::Event::Key { key: Key::Enter, pressed: true, modifiers, .. } if modifiers.command);
            place = edit.has_focus() && ui.input(|i| i.events.iter().any(ctrl_enter));
            ui.horizontal(|ui| {
                if ui.button("Place text").on_hover_text("Ctrl+Enter").clicked() {
                    place = true;
                }
                if ui.button("Cancel").on_hover_text("Escape").clicked() {
                    cancel = true;
                }
            });
            ui.weak("Uses the colour. Click on the canvas again to move the text.");
        }
    }
    if place {
        ed.place_text();
    } else if cancel {
        ed.cancel_text();
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
/// Shown when G hides everything but the painting.
const CANVAS_ONLY_HINT: &str = "Only the canvas: G or Escape brings everything back.";

pub const SHORTCUTS: &[(&str, &str)] = &[
    ("B", "Brush"),
    ("E", "Eraser on/off"),
    ("P", "Eyedropper"),
    ("F", "Fill"),
    ("G", "Only the canvas (G or Escape: everything back)"),
    ("Shift + G", "Gradient"),
    ("X", "Swap the colour and the sub colour"),
    ("V", "Line"),
    ("S", "Smudge"),
    ("R  L  W", "Rectangle select, lasso select, magic wand"),
    ("C", "Crop (Enter crops, Escape cancels)"),
    ("T", "Text (Ctrl+Enter places it)"),
    ("Z", "Zoom tool"),
    ("Shift while dragging a shape", "Straight angles, squares, circles"),
    ("Shift / Alt when starting a selection", "Add to it / take away from it"),
    ("Ctrl + A", "Select all"),
    ("Ctrl + Shift + A, Ctrl + D", "Deselect"),
    ("Ctrl + I, Ctrl + Shift + I", "Invert the selection"),
    ("Backspace", "Fill the selection (or the layer) with the colour"),
    ("Ctrl + Alt + J", "Copy the selection to a new layer"),
    ("Ctrl + Shift + J", "Cut the selection to a new layer"),
    ("Arrow keys (Move tool)", "Move 1 px; with Shift 10 px"),
    ("Ctrl + click (Clone tool)", "Choose where to copy from"),
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
    ("Delete", "Clear the selection, or the layer"),
    ("Ctrl + E", "Merge down"),
    ("Page Up / Down", "Select the layer above / below"),
    ("Ctrl + Page Up / Down", "Move the layer up / down"),
    ("Ctrl + N, O, S", "New, open, save"),
    ("Ctrl + Shift + S", "Save as"),
    ("Ctrl + Q", "Quit"),
];

#[cfg(test)]
mod tests;
