//! Renders a pen recording off-screen, exactly as the live app would draw it.

use recedo_tablet::{PenSample, Source};

use crate::brush::{BrushSettings, DabBatch, sample_dot};
use crate::gpu::{CanvasRenderer, Gpu, Layer};
use crate::strokes::StrokeTracker;

pub struct Rendered {
    pub width: u32,
    pub height: u32,
    /// Premultiplied RGBA8.
    pub canvas: Vec<u8>,
    pub strokes: usize,
    pub dabs: usize,
}

/// Draws every sample of `source` (or of every source, if `None`).
pub fn render(gpu: &Gpu, samples: &[PenSample], brush: &BrushSettings, source: Option<Source>, width: u32, height: u32) -> Result<Rendered, String> {
    let mut canvas = CanvasRenderer::new(gpu, width, height, None);
    let mut tracker = StrokeTracker::default();
    let mut batches: Vec<DabBatch> = Vec::new();
    let mut dots = Vec::new();
    for s in samples.iter().filter(|s| source.is_none_or(|src| s.source == src)) {
        tracker.feed(s, brush, |_, _| true, &mut batches);
        if s.contact() {
            dots.push(sample_dot(s));
        }
    }
    let mut enc = gpu.device.create_command_encoder(&Default::default());
    canvas.draw(gpu, &mut enc, Layer::Canvas, &batches);
    canvas.draw(gpu, &mut enc, Layer::Dots, &[DabBatch { erase: false, dabs: dots }]);
    gpu.queue.submit([enc.finish()]);
    Ok(Rendered {
        width: canvas.width,
        height: canvas.height,
        canvas: canvas.read_canvas(gpu)?,
        strokes: tracker.strokes_started,
        dabs: tracker.dabs_made,
    })
}

/// Simple measurements used by tests and the self-test.
pub mod measure {
    /// Is the pixel noticeably darker than the white canvas?
    pub fn inked(rgba: &[u8], w: u32, x: u32, y: u32) -> bool {
        let i = ((y * w + x) * 4) as usize;
        let (r, g, b, a) = (rgba[i] as u32, rgba[i + 1] as u32, rgba[i + 2] as u32, rgba[i + 3] as u32);
        // Premultiplied over white: what the pixel looks like on the white page.
        let lum = (r + g + b) / 3 + (255 - a);
        lum < 128
    }

    /// Is the pixel see-through (erased)?
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn erased(rgba: &[u8], w: u32, x: u32, y: u32) -> bool {
        rgba[((y * w + x) * 4 + 3) as usize] < 128
    }

    /// Number of inked pixels in a column, between `y0` and `y1`.
    pub fn column_ink(rgba: &[u8], w: u32, x: u32, y0: u32, y1: u32) -> u32 {
        (y0..y1).filter(|&y| inked(rgba, w, x, y)).count() as u32
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn count_inked(rgba: &[u8], w: u32, h: u32) -> u64 {
        let mut n = 0;
        for y in 0..h {
            for x in 0..w {
                if inked(rgba, w, x, y) {
                    n += 1;
                }
            }
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::measure::*;
    use super::*;
    use recedo_tablet::Flags;

    /// These tests need a GPU or a software rasteriser (lavapipe/llvmpipe). Without one
    /// they are skipped with a message, unless `RECEDO_REQUIRE_GPU=1` (set by the build
    /// scripts and CI), where a missing GPU is a failure.
    fn gpu() -> Option<Gpu> {
        match Gpu::new_headless() {
            Ok(g) => Some(g),
            Err(e) if std::env::var("RECEDO_REQUIRE_GPU").as_deref() == Ok("1") => panic!("no GPU for rendering tests: {e}"),
            Err(e) => {
                eprintln!("SKIPPED GPU test (set RECEDO_REQUIRE_GPU=1 to make this a failure): {e}");
                None
            }
        }
    }

    fn line(y: f64, x0: f64, x1: f64, p0: f32, p1: f32, n: usize) -> Vec<PenSample> {
        let mut v: Vec<PenSample> = (0..=n)
            .map(|i| {
                let t = i as f64 / n as f64;
                PenSample::synthetic(i as u64 * 5000, x0 + (x1 - x0) * t, y, p0 + (p1 - p0) * t as f32, true)
            })
            .collect();
        let last = *v.last().unwrap();
        v.push(PenSample { flags: Flags(0), ..last });
        v
    }

    #[test]
    fn pressure_ramp_renders_thin_to_thick() {
        let Some(gpu) = gpu() else { return };
        let brush = BrushSettings { size: 30.0, min_size: 0.05, hardness: 1.0, ..Default::default() };
        let samples = line(50.0, 20.0, 380.0, 0.1, 1.0, 60);
        let r = render(&gpu, &samples, &brush, None, 400, 100).unwrap();
        assert_eq!(r.strokes, 1);
        let thin = column_ink(&r.canvas, r.width, 60, 0, 100);
        let thick = column_ink(&r.canvas, r.width, 360, 0, 100);
        // Diameter at x=60 is about 30 * (0.05 + 0.95 * 0.2) = 7 px; at x=360 about 28.6 px.
        assert!((4..=10).contains(&thin), "thin {thin}");
        assert!((25..=32).contains(&thick), "thick {thick}");
        // Nothing outside the stroke.
        assert!(!inked(&r.canvas, r.width, 200, 5));
        assert!(!inked(&r.canvas, r.width, 5, 50));
    }

    #[test]
    fn fast_sparse_samples_leave_no_gaps() {
        let Some(gpu) = gpu() else { return };
        let brush = BrushSettings { size: 6.0, pressure_size: false, hardness: 1.0, ..Default::default() };
        // 8 samples across 360 px: 50 px between samples, far more than the brush size.
        let samples = line(30.0, 20.0, 380.0, 1.0, 1.0, 7);
        let r = render(&gpu, &samples, &brush, None, 400, 60).unwrap();
        for x in 20..=380 {
            assert!(inked(&r.canvas, r.width, x, 30), "gap at x={x}");
        }
    }

    #[test]
    fn eraser_end_erases() {
        let Some(gpu) = gpu() else { return };
        let brush = BrushSettings { size: 20.0, pressure_size: false, ..Default::default() };
        let mut samples = line(50.0, 50.0, 350.0, 1.0, 1.0, 30);
        let mut erase = line(50.0, 150.0, 250.0, 1.0, 1.0, 10);
        for s in &mut erase {
            s.flags.set(Flags::ERASER, true);
        }
        samples.extend(erase);
        let r = render(&gpu, &samples, &brush, None, 400, 100).unwrap();
        assert_eq!(r.strokes, 2);
        assert!(inked(&r.canvas, r.width, 100, 50));
        assert!(erased(&r.canvas, r.width, 200, 50));
        assert!(inked(&r.canvas, r.width, 300, 50));
        // The eraser also clears the white page outside the ink.
        assert!(erased(&r.canvas, r.width, 200, 58));
    }

    #[test]
    fn source_filter() {
        let Some(gpu) = gpu() else { return };
        let mut samples = line(50.0, 20.0, 380.0, 1.0, 1.0, 30);
        for s in &mut samples {
            s.source = Source::Wintab;
        }
        let r = render(&gpu, &samples, &BrushSettings::default(), Some(Source::Ink), 400, 100).unwrap();
        assert_eq!(r.strokes, 0);
        assert_eq!(count_inked(&r.canvas, r.width, r.height), 0);
    }
}
