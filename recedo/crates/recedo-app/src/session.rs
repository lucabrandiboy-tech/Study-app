//! One open window's worth of Recedo, independent of Windows: the editor, the interface, and
//! drawing both into a GPU texture. The Windows shell feeds it input and presents the
//! texture; the tests run it off-screen.

use std::path::PathBuf;

use recedo_core::doc::Document;
use recedo_render::CanvasDisplay;

use crate::editor::{Editor, Refresh};
use crate::settings::Settings;
use crate::ui::{AppUi, FrameOutput};

/// Grey around the canvas (sRGB 0..1).
pub const BACKGROUND: [f64; 3] = [0.20, 0.20, 0.22];

pub struct Session {
    pub ed: Editor,
    pub ui: AppUi,
    pub egui: egui::Context,
    /// The canvas area from the last frame, in points.
    pub canvas_rect: egui::Rect,
}

impl Session {
    pub fn new(settings: Settings, recovery_dir: Option<PathBuf>) -> Session {
        let preset =
            recedo_core::canvas::builtin_presets().into_iter().nth(settings.last_preset).unwrap_or_else(|| recedo_core::canvas::builtin_presets().remove(0));
        let doc = preset.create().unwrap_or_else(|_| Document::new(1920, 1080, Some([255; 4])));
        let mut ed = Editor::new(doc, recovery_dir);
        ed.tools = settings.tools.clone();
        ed.color = settings.color;
        ed.autosaver.interval = std::time::Duration::from_secs(settings.autosave_minutes as u64 * 60);
        let egui = egui::Context::default();
        egui.set_theme(egui::Theme::Dark);
        Session { ed, ui: AppUi::new(settings), egui, canvas_rect: egui::Rect::NOTHING }
    }

    /// The settings to save: the interface's, with the editor's tools and colour.
    pub fn settings(&self) -> Settings {
        let mut s = self.ui.settings.clone();
        s.tools = self.ed.tools.clone();
        s.color = self.ed.color;
        s
    }

    /// Runs the interface for one frame and tells the editor where the canvas area is.
    pub fn ui_frame(&mut self, raw: egui::RawInput) -> (egui::FullOutput, FrameOutput) {
        self.egui.set_zoom_factor(self.ui.settings.ui_scale.clamp(0.5, 3.0));
        self.ed.autosaver.interval = std::time::Duration::from_secs(self.ui.settings.autosave_minutes.max(1) as u64 * 60);
        let mut frame = None;
        let (ui, ed) = (&mut self.ui, &mut self.ed);
        let full = self.egui.run_ui(raw, |root| frame = Some(ui.frame(root, ed)));
        let frame = frame.unwrap_or(FrameOutput { canvas_rect: egui::Rect::NOTHING, requests: Vec::new() });
        let ppp = full.pixels_per_point as f64;
        let r = frame.canvas_rect;
        self.canvas_rect = r;
        self.ed.set_viewport(r.min.x as f64 * ppp, r.min.y as f64 * ppp, r.width() as f64 * ppp, r.height() as f64 * ppp);
        (full, frame)
    }

    /// Is this window pixel on the canvas (not on a panel, menu, dialog or popup)?
    pub fn over_canvas(&self, x: f64, y: f64) -> bool {
        let ppp = self.egui.pixels_per_point();
        let p = egui::pos2(x as f32 / ppp, y as f32 / ppp);
        if !self.canvas_rect.contains(p) || self.ui.dialog_open() {
            return false;
        }
        // Menus, popups and floating panels are areas on layers above the background (the
        // canvas overlay is only painted, never an area, so it does not count).
        self.egui.layer_id_at(p).is_none_or(|l| l.order == egui::Order::Background)
    }
}

/// Where a frame is drawn: a texture view and its size in pixels.
pub struct Target<'a> {
    pub view: &'a wgpu::TextureView,
    pub size: [u32; 2],
}

/// Draws the canvas and the interface into a texture.
pub struct Renderer {
    display: CanvasDisplay,
    egui: egui_wgpu::Renderer,
    canvas_size: (u32, u32),
    format: wgpu::TextureFormat,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, ed: &Editor) -> Renderer {
        Renderer {
            display: CanvasDisplay::new(device, ed.doc.width, ed.doc.height, format),
            egui: egui_wgpu::Renderer::new(device, format, egui_wgpu::RendererOptions::default()),
            canvas_size: (ed.doc.width, ed.doc.height),
            format,
        }
    }

    /// Uploads what changed in the painting since the last frame.
    pub fn sync(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, ed: &mut Editor) {
        let refresh = ed.take_refresh();
        if (ed.doc.width, ed.doc.height) != self.canvas_size {
            // A painting of another size: new textures.
            self.display = CanvasDisplay::new(device, ed.doc.width, ed.doc.height, self.format);
            self.canvas_size = (ed.doc.width, ed.doc.height);
            self.display.upload_all(queue, &ed.projection);
            return;
        }
        match refresh {
            Refresh::None => {}
            Refresh::Tiles(t) => self.display.upload(queue, &ed.projection, &t),
            Refresh::All => self.display.upload_all(queue, &ed.projection),
        }
    }

    /// Applies a frame's font and image updates without drawing it (a frame that is not
    /// shown, e.g. while the window is minimised, must still hand these over).
    pub fn skip_frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, out: &mut egui::FullOutput) {
        let mut textures = std::mem::take(&mut out.textures_delta);
        for (id, deltas) in textures.set.drain() {
            for delta in deltas {
                self.egui.update_texture(device, queue, id, &delta);
            }
        }
        for id in textures.free.drain() {
            self.egui.free_texture(&id);
        }
    }

    /// Draws one frame into `target` and submits it.
    pub fn render(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, target: Target<'_>, ed: &Editor, ctx: &egui::Context, mut out: egui::FullOutput) {
        let Target { view: target, size } = target;
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("recedo frame") });
        self.display.draw(queue, &mut enc, target, &ed.view, BACKGROUND);
        let prims = ctx.tessellate(std::mem::take(&mut out.shapes), out.pixels_per_point);
        let mut textures = std::mem::take(&mut out.textures_delta);
        for (id, deltas) in textures.set.drain() {
            for delta in deltas {
                self.egui.update_texture(device, queue, id, &delta);
            }
        }
        let sd = egui_wgpu::ScreenDescriptor { size_in_pixels: size, pixels_per_point: out.pixels_per_point };
        let extra = self.egui.update_buffers(device, queue, &mut enc, &prims, &sd);
        {
            let mut pass = enc
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                    })],
                    ..Default::default()
                })
                .forget_lifetime();
            self.egui.render(&mut pass, &prims, &sd);
        }
        for id in textures.free.drain() {
            self.egui.free_texture(&id);
        }
        queue.submit(extra.into_iter().chain(std::iter::once(enc.finish())));
    }
}
