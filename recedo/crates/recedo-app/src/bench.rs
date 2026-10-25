//! `recedo --benchmark [folder]`: the spec's performance targets, measured through the app's
//! own code on this computer: a 3840 x 2160 painting with 15 layers, strokes with 20, 200
//! and 1000 px brushes fed through the editor at a fast pen's rate, each frame's update of
//! the picture and its GPU display, undo and redo, saving and opening the `.kra`.
//!
//! No window: the display is drawn into an off-screen texture of a typical canvas area
//! (1600 x 900) and each frame waits for the GPU to finish, so the times include its work.

use std::fmt::Write as _;
use std::path::Path;
use std::time::{Duration, Instant};

use recedo_core::brush::Brush;
use recedo_core::doc::{Document, Layer};
use recedo_core::tiles::TiledLayer;
use recedo_render::gpu::Gpu;
use recedo_tablet::PenSample;

use crate::editor::{Editor, Modifiers};
use crate::session::{Renderer, Target};

const W: u32 = 3840;
const H: u32 = 2160;
/// The canvas area on screen.
const VIEW: (u32, u32) = (1600, 900);
/// A fast pen: 200 samples a second, shown at 60 frames a second.
const SAMPLES_PER_FRAME: usize = 4;
const FRAME_BUDGET_MS: f64 = 1000.0 / 60.0;

/// Background plus `painted` layers covered in soft gradients with a little noise (no tile
/// is empty or uniform, the worst case for compositing and saving).
fn painting(painted: u32) -> Document {
    let mut doc = Document::new(W, H, Some([255, 255, 255, 255]));
    doc.layers.clear();
    doc.layers.push(Layer { pixels: TiledLayer::filled([255, 255, 255, 255]), ..Layer::new("Background") });
    let mut rgba = vec![0u8; (W * H * 4) as usize];
    for n in 0..painted {
        let mut seed = 0x9E37_79B9u32.wrapping_mul(n + 1);
        for (i, p) in rgba.chunks_exact_mut(4).enumerate() {
            let (x, y) = (i as u32 % W, i as u32 / W);
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let noise = (seed & 3) as u8;
            p.copy_from_slice(&[(x / 16) as u8 ^ noise, (y / 9) as u8, (n * 50 % 256) as u8, 40 + noise]);
        }
        let mut l = Layer::new(&format!("Painted {n}"));
        l.pixels.write_rect(0, 0, W, H, &rgba);
        doc.layers.push(l);
    }
    doc
}

struct Display {
    gpu: Gpu,
    renderer: Renderer,
    target: wgpu::Texture,
    egui: egui::Context,
}

impl Display {
    /// Brings the screen up to date: the picture's changed tiles, then a frame drawn and
    /// finished by the GPU. Returns the time the picture update took (the CPU part).
    fn frame(&mut self, ed: &mut Editor) -> Duration {
        let g = &self.gpu;
        let t = Instant::now();
        let refresh = ed.take_refresh();
        let picture = t.elapsed();
        self.renderer.apply(&g.device, &g.queue, ed, refresh);
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(VIEW.0 as f32, VIEW.1 as f32))),
            ..Default::default()
        };
        let out = self.egui.run_ui(raw, |_| {});
        let view = self.target.create_view(&Default::default());
        self.renderer.render(&g.device, &g.queue, Target { view: &view, size: [VIEW.0, VIEW.1] }, ed, &self.egui, out);
        let _ = g.device.poll(wgpu::PollType::wait_indefinitely());
        picture
    }
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// Median, 95th percentile and worst of a list of times (ms).
fn spread(mut v: Vec<f64>) -> (f64, f64, f64) {
    v.sort_by(f64::total_cmp);
    let at = |q: f64| v.get(((v.len() as f64 - 1.0) * q).round() as usize).copied().unwrap_or(0.0);
    (at(0.5), at(0.95), v.last().copied().unwrap_or(0.0))
}

/// One fast S-shaped stroke across the canvas area (about 1.2 s of pen time), fed to the
/// editor a frame's worth of samples at a time, with the screen updated after each.
fn stroke(ed: &mut Editor, display: &mut Display, size: f32, out: &mut String) {
    ed.tools.brush = Brush { size, hardness: 0.5, ..Brush::default() };
    let n = 240;
    let (vw, vh) = (VIEW.0 as f64, VIEW.1 as f64);
    let samples: Vec<PenSample> = (0..=n + 1)
        .map(|i| {
            let t = (i.min(n)) as f64 / n as f64;
            let x = vw * (0.08 + 0.84 * t);
            let y = vh * (0.5 + 0.3 * (t * std::f64::consts::TAU).sin());
            PenSample::synthetic(i as u64 * 5000, x, y, 0.6 + 0.4 * t as f32, i <= n)
        })
        .collect();
    let (mut pen, mut picture, mut gpu, mut total) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for chunk in samples.chunks(SAMPLES_PER_FRAME) {
        let t0 = Instant::now();
        for s in chunk {
            ed.pen(s, true, Modifiers::default());
        }
        let t1 = Instant::now();
        let pic = display.frame(ed);
        let t2 = Instant::now();
        pen.push(ms(t1 - t0));
        picture.push(ms(pic));
        gpu.push(ms(t2 - t1 - pic));
        total.push(ms(t2 - t0));
    }
    let frames = total.len();
    let on_time = total.iter().filter(|&&t| t <= FRAME_BUDGET_MS).count();
    let cpu_on_time = pen.iter().zip(&picture).filter(|(a, b)| *a + *b <= FRAME_BUDGET_MS).count();
    let cell = |v: Vec<f64>| {
        let (m, p95, max) = spread(v);
        format!("{m:.1} / {p95:.1} / {max:.1}")
    };
    let _ = writeln!(
        out,
        "| {size} px | {} | {} | {} | {} | {:.0} % | {:.0} % |",
        cell(pen),
        cell(picture),
        cell(gpu),
        cell(total),
        on_time as f64 * 100.0 / frames as f64,
        cpu_on_time as f64 * 100.0 / frames as f64
    );
}

/// Runs everything and returns the report (Markdown). `dir` receives the test `.kra`.
pub fn run(dir: &Path) -> Result<String, String> {
    let mut out = String::new();
    let _ = writeln!(out, "# Recedo benchmark\n");
    let _ = writeln!(out, "recedo {} ({})  ", crate::ui::VERSION, crate::GIT_HASH);
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let gpu = Gpu::new_headless()?;
    let _ = writeln!(out, "CPU threads: {threads}  ");
    let _ = writeln!(out, "GPU: {}  ", gpu.describe());

    let t = Instant::now();
    let doc = painting(14);
    let _ = writeln!(out, "Painting: {W} x {H}, {} layers, every pixel painted (made in {:.1} s)\n", doc.layers.len(), t.elapsed().as_secs_f64());
    let mut ed = Editor::new(doc, None);
    ed.set_viewport(0.0, 0.0, VIEW.0 as f64, VIEW.1 as f64);
    ed.active = 8;
    ed.color = [30, 60, 160];
    let renderer = Renderer::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm, &ed);
    let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("benchmark screen"),
        size: wgpu::Extent3d { width: VIEW.0, height: VIEW.1, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let mut display = Display { gpu, renderer, target, egui: egui::Context::default() };
    let t = Instant::now();
    display.frame(&mut ed);
    let _ = writeln!(out, "First display of the whole painting (upload to the GPU): {:.0} ms\n", ms(t.elapsed()));
    let t = Instant::now();
    for _ in 0..30 {
        display.frame(&mut ed);
    }
    let _ = writeln!(out, "Redrawing the screen with nothing changed: {:.1} ms per frame\n", ms(t.elapsed()) / 30.0);

    let _ = writeln!(
        out,
        "## Painting\n\nA fast S-shaped stroke across the canvas on layer 9 of 15, soft brush, the pen at 200 samples/s and \
         the screen at 60 frames/s ({SAMPLES_PER_FRAME} samples per frame), shown fitted to a {} x {} canvas area. Times per frame in ms, \
         median / 95 % / worst. \"Painting\" = the brush on the layer; \"Picture\" = the changed tiles recombined from all 15 \
         layers (CPU); \"Display\" = uploading them and drawing the frame until the GPU is done.\n",
        VIEW.0, VIEW.1
    );
    let _ = writeln!(
        out,
        "| Brush | Painting | Picture | Display | Total | Frames within 16.7 ms | Painting + picture within 16.7 ms |\n|---|---|---|---|---|---|---|"
    );
    for size in [20.0, 200.0, 1000.0] {
        stroke(&mut ed, &mut display, size, &mut out);
    }

    let _ = writeln!(out, "\n## Undo\n");
    for what in ["Undo", "Redo"] {
        let t = Instant::now();
        if what == "Undo" { ed.undo() } else { ed.redo() }
        let edit = t.elapsed();
        let t = Instant::now();
        let picture = display.frame(&mut ed);
        let _ = writeln!(
            out,
            "{what} the 1000 px stroke: {:.1} ms, then {:.0} ms to update the screen ({:.0} ms picture, {:.0} ms display)  ",
            ms(edit),
            ms(t.elapsed()),
            ms(picture),
            ms(t.elapsed() - picture)
        );
    }

    let _ = writeln!(out, "\n## Files\n");
    let path = dir.join("recedo-benchmark.kra");
    let t = Instant::now();
    ed.save_as(&path);
    ed.wait_for_saves();
    let save = t.elapsed();
    let size = std::fs::metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?.len();
    let t = Instant::now();
    let mut back = Editor::new(Document::new(1, 1, None), None);
    back.open(&path)?;
    let open = t.elapsed();
    let same = back.doc == ed.doc;
    let _ = std::fs::remove_file(&path);
    let _ = writeln!(out, "Save .kra: {:.2} s ({:.0} MB)  ", save.as_secs_f64(), size as f64 / 1e6);
    let _ = writeln!(out, "Open .kra: {:.2} s{}  ", open.as_secs_f64(), if same { "" } else { " (WARNING: did not read back the same)" });
    let mem: usize = ed.doc.layers.iter().map(|l| l.pixels.memory_bytes()).sum();
    let _ = writeln!(out, "Layer memory: {:.0} MB", mem as f64 / 1e6);
    if !same {
        return Err(format!("{out}\nthe saved painting did not read back the same"));
    }
    Ok(out)
}
