//! The plain-text diagnostics report ("Copy report" / saved next to recordings).

use std::fmt::Write;

use recedo_tablet::Source;
use recedo_tablet::stats::SourceStats;

use crate::model::{Model, WintabState};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GIT_HASH: &str = env!("RECEDO_GIT_HASH");

fn source_block(out: &mut String, name: &str, st: &mut SourceStats, now_us: u64) {
    let _ = writeln!(out, "[{name}]");
    if st.samples == 0 {
        let _ = writeln!(out, "  no samples");
        return;
    }
    let rate_now = st.rate_now(now_us);
    let iv = &st.intervals;
    let ms = |v: Option<u32>| v.map(|u| format!("{:.2} ms", u as f64 / 1000.0)).unwrap_or_else(|| "-".into());
    let _ = writeln!(out, "  samples {} (touching {}), messages {}", st.samples, st.contact_samples, st.messages);
    let _ = writeln!(out, "  rate now {rate_now:.0}/s, peak {:.0}/s, device rate (median interval) {}", st.peak_rate,
        st.median_rate().map(|r| format!("{r:.0}/s")).unwrap_or_else(|| "-".into()));
    let _ = writeln!(out, "  interval median {}, p95 {}, max {} ({} intervals)", ms(iv.percentile(50.0)), ms(iv.percentile(95.0)), ms(iv.max()), iv.len());
    let _ = writeln!(out, "  samples per message avg {:.2}, max {}", st.samples_per_message(), st.batch_max);
    let _ = writeln!(out, "  lost by serial {}, queue overflows {}, whole-pixel fallbacks {}", st.serial_gaps, st.queue_errors, st.coarse_positions);
    if st.pressure.seen {
        let _ = writeln!(out, "  pressure {:.4} .. {:.4}, {} distinct raw levels", st.pressure.min, st.pressure.max, st.pressure_levels());
    } else {
        let _ = writeln!(out, "  pressure: none seen while touching");
    }
    if st.tilt_x.seen {
        let _ = writeln!(out, "  tilt X {:+.1} .. {:+.1}, tilt Y {:+.1} .. {:+.1}", st.tilt_x.min, st.tilt_x.max, st.tilt_y.min, st.tilt_y.max);
    } else {
        let _ = writeln!(out, "  tilt: not reported");
    }
    if st.twist.seen {
        let _ = writeln!(out, "  rotation {:.1} .. {:.1}", st.twist.min, st.twist.max);
    }
    let _ = writeln!(out, "  eraser end seen: {}, side buttons seen: 0x{:X}", st.eraser_seen, st.barrel_seen >> 2);
}

pub fn build(m: &mut Model, now_us: u64) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Recedo tablet test report");
    let _ = writeln!(out, "version {VERSION} ({GIT_HASH})");
    let _ = writeln!(out, "os: {}", m.os);
    let _ = writeln!(out, "gpu: {}", m.gpu);
    let _ = writeln!(out, "window dpi: {}", m.window_dpi);
    for (i, mon) in m.monitors.iter().enumerate() {
        let (l, t, r, b) = mon.rect;
        let _ = writeln!(out, "monitor {}: {} {}x{} at ({l},{t}) dpi {}{}{}", i + 1, mon.name, r - l, b - t, mon.dpi,
            if mon.primary { " primary" } else { "" }, if mon.has_window { " [window]" } else { "" });
    }
    let s = &m.settings;
    let b = &s.brush;
    let _ = writeln!(out, "api: {}, wintab mapping: {}", s.api.label(), s.mapping_mode().label());
    let _ = writeln!(out, "brush: size {:.1}, opacity {:.2}, hardness {:.2}, spacing {:.2}, pressure->size {}, pressure->opacity {}, tilt {}",
        b.size, b.opacity, b.hardness, b.spacing, b.pressure_size, b.pressure_opacity, b.tilt_shape);
    let _ = writeln!(out, "presentation: {}, low latency {}", m.frame.present_mode, s.low_latency);
    let _ = writeln!(out);

    let _ = writeln!(out, "[Wintab driver]");
    match &m.wintab.state {
        WintabState::Open => {
            let _ = writeln!(out, "  state: open");
        }
        WintabState::Closed => {
            let _ = writeln!(out, "  state: not opened (Wintab not selected this session)");
        }
        WintabState::Failed(e) => {
            let _ = writeln!(out, "  state: FAILED: {e}");
        }
    }
    for (k, v) in &m.wintab.info {
        let _ = writeln!(out, "  {k}: {v}");
    }
    if !m.wintab.mapping_active.is_empty() {
        let _ = writeln!(out, "  mapping in use: {}{}", m.wintab.mapping_active, if m.wintab.mismatch { " (DOES NOT MATCH THE CURSOR)" } else { "" });
        for (label, score) in &m.wintab.mapping_rows {
            match score {
                Some(sc) if sc.observations > 0 => {
                    let _ = writeln!(out, "    {label}: off by {:.2} px ({} checks)", sc.error, sc.observations);
                }
                _ => {
                    let _ = writeln!(out, "    {label}: no checks");
                }
            }
        }
        if let Some(f) = &m.wintab.fit {
            let _ = writeln!(out, "    fitted: x = {:.3}*u + {:.2}, y = {:.3}*v + {:.2}, rms {:.2} px over {} checks", f.ax, f.bx, f.ay, f.by, f.rms, f.samples);
        }
    }
    for c in &m.wintab.cursors {
        let _ = writeln!(out, "  cursor: {c}");
    }
    let _ = writeln!(out, "[Windows Ink]");
    if m.ink.devices.is_empty() {
        let _ = writeln!(out, "  no pen devices registered");
    }
    for d in &m.ink.devices {
        let _ = writeln!(out, "  device: {d}");
    }
    let _ = writeln!(out, "  most samples in one message: {}, whole-pixel fallbacks: {}", m.ink.max_history, m.ink.coarse_fallbacks);
    let _ = writeln!(out);
    source_block(&mut out, Source::Wintab.label(), &mut m.stats.wintab, now_us);
    source_block(&mut out, Source::Ink.label(), &mut m.stats.ink, now_us);
    source_block(&mut out, Source::Mouse.label(), &mut m.stats.mouse, now_us);
    let f = &m.frame;
    let _ = writeln!(out, "[Screen]");
    let _ = writeln!(out, "  {} frames; while drawing (last second): {:.0} fps, frame {:.2} ms avg / {:.2} ms worst, {:.2} samples per frame; newest sample to present {:.2} ms",
        f.frames, f.fps, f.frame_ms_avg, f.frame_ms_max, f.samples_per_frame, f.input_age_ms);
    let _ = writeln!(out, "  strokes {}, dabs {}", m.strokes, m.dabs);
    let _ = writeln!(out);
    let _ = writeln!(out, "[Log]");
    for l in m.log.iter().rev().take(80).collect::<Vec<_>>().into_iter().rev() {
        let _ = writeln!(out, "  {l}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use recedo_tablet::PenSample;

    #[test]
    fn report_mentions_every_section() {
        let mut m = Model::default();
        let batch: Vec<PenSample> = (0..20).map(|i| PenSample::synthetic(i * 5000, i as f64, 0.0, 0.5, true)).collect();
        m.stats.ink.record_batch(&batch, 100_000, false);
        let r = build(&mut m, 100_000);
        for needle in ["[Wintab driver]", "[Windows Ink]", "[Windows Ink]", "samples 20", "pressure 0.5000", "[Screen]", "[Log]"] {
            assert!(r.contains(needle), "missing {needle:?} in\n{r}");
        }
    }
}
