//! The side panel: input choice, live pen readout, event rates, brush, diagnostics.

use egui::{Color32, RichText, Stroke, Ui, vec2};
use recedo_tablet::mapping::MappingMode;
use recedo_tablet::stats::SourceStats;
use recedo_tablet::{Flags, Source};

use crate::model::{Action, Api, Model, WintabState};

const WARN: Color32 = Color32::from_rgb(255, 170, 60);
const BAD: Color32 = Color32::from_rgb(255, 95, 95);
const GOOD: Color32 = Color32::from_rgb(110, 210, 120);

pub const PANEL_WIDTH: f32 = 340.0;

/// Draws the panel contents. Returns what the platform layer should do.
pub fn panel(ui: &mut Ui, m: &mut Model, now_us: u64) -> Vec<Action> {
    let mut actions = Vec::new();
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.add_space(4.0);
        ui.heading("Recedo tablet test");
        ui.label(RichText::new("Phase 0: checks pressure, tilt and event rate through Wintab and Windows Ink. Draw on the white area.").small());
        ui.add_space(6.0);
        input_section(ui, m, &mut actions);
        live_section(ui, m);
        rate_section(ui, m, now_us, &mut actions);
        brush_section(ui, m, &mut actions);
        stabilizer_section(ui, m);
        wintab_section(ui, m, &mut actions);
        ink_section(ui, m);
        screen_section(ui, m, &mut actions);
        record_section(ui, m, &mut actions);
        log_section(ui, m);
        ui.add_space(20.0);
    });
    actions
}

fn input_section(ui: &mut Ui, m: &mut Model, actions: &mut Vec<Action>) {
    egui::CollapsingHeader::new(RichText::new("Tablet input API").strong()).default_open(true).show(ui, |ui| {
        let before = m.settings.api;
        for api in Api::ALL {
            ui.radio_value(&mut m.settings.api, api, api.label());
        }
        if m.settings.api != before {
            actions.push(Action::SetApi(m.settings.api));
        }
        match &m.wintab.state {
            WintabState::Open => {
                ui.colored_label(GOOD, format!("Wintab: open ({})", if m.wintab.device_name.is_empty() { "unnamed device" } else { &m.wintab.device_name }));
            }
            WintabState::Failed(e) => {
                ui.colored_label(BAD, format!("Wintab: {e}"));
            }
            WintabState::Closed => {
                ui.weak("Wintab: not in use");
            }
        }
        if m.ink.devices.is_empty() {
            ui.colored_label(WARN, "Windows Ink: no pen device found. In the XP-Pen driver, turn on \"Windows Ink\".");
        } else {
            ui.label(format!("Windows Ink pen: {}", m.ink.devices.join(", ")));
        }
        if m.settings.api == Api::Wintab && m.wintab.mismatch {
            ui.colored_label(
                BAD,
                "The Wintab position does not line up with the cursor. Open \"Wintab details\" and try another screen mapping.",
            );
        }
    });
}

fn yes_no(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}

fn live_section(ui: &mut Ui, m: &mut Model) {
    egui::CollapsingHeader::new(RichText::new("Live pen").strong()).default_open(true).show(ui, |ui| {
        let Some(s) = m.latest else {
            ui.weak("Hover the pen over the window.");
            return;
        };
        let state = if s.contact() { "touching" } else { "hovering" };
        ui.label(format!("{}: {state}{}", s.source.label(), if s.eraser() { ", ERASER end" } else { "" }));
        egui::Grid::new("live").num_columns(2).spacing(vec2(10.0, 2.0)).show(ui, |ui| {
            ui.label("Position");
            ui.monospace(format!("{:8.2}, {:8.2} px", s.x, s.y));
            ui.end_row();
            ui.label("Pressure");
            if s.flags.has(Flags::HAS_PRESSURE) {
                let raw = match s.source {
                    Source::Ink => format!("{} / 1024", s.raw_pressure),
                    _ => format!("raw {}", s.raw_pressure),
                };
                ui.add(egui::ProgressBar::new(s.pressure).desired_width(170.0).text(format!("{:.3}  ({raw})", s.pressure)));
            } else {
                ui.colored_label(WARN, "not reported");
            }
            ui.end_row();
            ui.label("Tilt X / Y");
            if s.flags.has(Flags::HAS_TILT) {
                ui.monospace(format!("{:+6.1}° / {:+6.1}°", s.tilt_x, s.tilt_y));
            } else {
                ui.colored_label(WARN, "not reported");
            }
            ui.end_row();
            ui.label("Rotation");
            ui.monospace(if s.flags.has(Flags::HAS_TWIST) { format!("{:.1}°", s.twist) } else { "not reported".into() });
            ui.end_row();
            ui.label("Buttons");
            let mut b = Vec::new();
            if s.flags.has(Flags::BARREL1) {
                b.push("lower side button");
            }
            if s.flags.has(Flags::BARREL2) {
                b.push("upper side button");
            }
            ui.monospace(format!("{}  (raw 0x{:X})", if b.is_empty() { "none".to_string() } else { b.join(" + ") }, s.raw_buttons));
            ui.end_row();
        });
        if s.flags.has(Flags::HAS_TILT) {
            tilt_widget(ui, s.tilt_x, s.tilt_y);
        }
    });
}

/// A circle with a line showing which way the pen leans (as seen from above).
fn tilt_widget(ui: &mut Ui, tx: f32, ty: f32) {
    let (resp, painter) = ui.allocate_painter(vec2(90.0, 90.0), egui::Sense::hover());
    let c = resp.rect.center();
    let r = 40.0;
    painter.circle_stroke(c, r, Stroke::new(1.0, Color32::GRAY));
    painter.circle_stroke(c, r * 0.5, Stroke::new(1.0, Color32::from_gray(70)));
    let end = c + vec2(tx / 90.0 * r, ty / 90.0 * r);
    painter.line_segment([c, end], Stroke::new(3.0, Color32::from_rgb(90, 170, 255)));
    painter.circle_filled(end, 4.0, Color32::from_rgb(90, 170, 255));
    resp.on_hover_text("Seen from above: the line points the way the top of the pen leans. Right = +X, down (toward you) = +Y.");
}

fn rate_row(ui: &mut Ui, name: &str, st: &mut SourceStats, now_us: u64) {
    let now = st.rate_now(now_us);
    ui.label(name);
    ui.monospace(format!("{now:5.0}"));
    ui.monospace(format!("{:5.0}", st.peak_rate));
    ui.monospace(st.median_rate().map(|r| format!("{r:5.0}")).unwrap_or_else(|| "    –".into()));
    ui.monospace(format!("{:4.1}/{}", st.samples_per_message(), st.batch_max));
    let lost = st.serial_gaps + st.queue_errors;
    if lost > 0 {
        ui.colored_label(BAD, format!("{lost}"));
    } else {
        ui.monospace("0");
    }
    ui.end_row();
}

fn rate_section(ui: &mut Ui, m: &mut Model, now_us: u64, actions: &mut Vec<Action>) {
    egui::CollapsingHeader::new(RichText::new("Event rate (samples per second)").strong()).default_open(true).show(ui, |ui| {
        egui::Grid::new("rates").num_columns(6).striped(true).spacing(vec2(8.0, 2.0)).show(ui, |ui| {
            for h in ["", "now", "peak", "device", "per msg", "lost"] {
                ui.small(h);
            }
            ui.end_row();
            rate_row(ui, "Wintab", &mut m.stats.wintab, now_us);
            rate_row(ui, "Ink", &mut m.stats.ink, now_us);
            rate_row(ui, "Mouse", &mut m.stats.mouse, now_us);
        });
        ui.small("\"device\" = rate from the time between samples. \"per msg\" = samples per message (average/most): above 1 means coalesced samples are being collected. \"lost\" = Wintab packets missing by serial number.");
        let f = &m.frame;
        if f.fps > 0.0 {
            ui.label(format!(
                "Screen while drawing: {:.0} fps, frame {:.1} ms (worst {:.1}), {:.1} samples per frame",
                f.fps, f.frame_ms_avg, f.frame_ms_max, f.samples_per_frame
            ));
        } else {
            ui.label("Screen while drawing: move the pen to measure");
        }
        ui.label(format!("Newest sample to screen: {:.1} ms (plus display scan-out)", f.input_age_ms));
        ui.label(format!("Per frame: {:.1} ms of app work, {:.1} ms waiting for the GPU and display", f.work_ms, f.wait_ms));
        let st = m.stats.get(m.settings.api.source());
        if st.pressure.seen {
            ui.label(format!(
                "{}: pressure seen {:.3} – {:.3}, {} distinct levels",
                m.settings.api.source().label(),
                st.pressure.min,
                st.pressure.max,
                st.pressure_levels()
            ));
        }
        if st.tilt_x.seen {
            ui.label(format!(
                "Tilt seen: X {:+.0}° … {:+.0}°, Y {:+.0}° … {:+.0}°",
                st.tilt_x.min, st.tilt_x.max, st.tilt_y.min, st.tilt_y.max
            ));
        }
        if ui.button("Reset numbers").clicked() {
            actions.push(Action::ResetStats);
        }
    });
}

fn brush_section(ui: &mut Ui, m: &mut Model, actions: &mut Vec<Action>) {
    egui::CollapsingHeader::new(RichText::new("Test brush").strong()).default_open(true).show(ui, |ui| {
        let b = &mut m.settings.brush;
        ui.add(egui::Slider::new(&mut b.size, 1.0..=300.0).logarithmic(true).text("size px"));
        ui.add(egui::Slider::new(&mut b.opacity, 0.02..=1.0).text("opacity"));
        ui.add(egui::Slider::new(&mut b.hardness, 0.0..=1.0).text("hardness"));
        ui.add(egui::Slider::new(&mut b.spacing, 0.02..=1.0).text("spacing"));
        ui.checkbox(&mut b.pressure_size, "Pressure controls size");
        ui.checkbox(&mut b.pressure_opacity, "Pressure controls opacity");
        ui.checkbox(&mut b.tilt_shape, "Tilt stretches the brush");
        ui.checkbox(&mut b.eraser_end_erases, "Eraser end of the pen erases");
        ui.horizontal(|ui| {
            for (name, rgb) in [("Black", [0.08, 0.08, 0.1]), ("Blue", [0.1, 0.3, 0.85]), ("Red", [0.85, 0.15, 0.1]), ("Green", [0.1, 0.6, 0.25])] {
                // Black text would vanish on the dark panel.
                let c = if name == "Black" {
                    Color32::from_gray(220)
                } else {
                    Color32::from_rgb((rgb[0] * 255.0) as u8, (rgb[1] * 255.0) as u8, (rgb[2] * 255.0) as u8)
                };
                let selected = b.color == rgb;
                let text = RichText::new(if selected { format!("• {name}") } else { name.to_string() }).color(c);
                if ui.button(text).clicked() {
                    b.color = rgb;
                }
            }
        });
        ui.small("Lines are drawn exactly as the tablet sends them, with no smoothing, so you can see the raw input.");
        let before = m.settings.show_dots;
        ui.checkbox(&mut m.settings.show_dots, "Show every sample as a red dot");
        ui.add_enabled_ui(m.settings.show_dots, |ui| {
            ui.checkbox(&mut m.settings.show_hover_dots, "…and hover samples (blue)");
        });
        if before && !m.settings.show_dots {
            actions.push(Action::ClearDots);
        }
        ui.checkbox(&mut m.settings.show_crosshair, "Red cross where the tablet says the pen is");
        ui.horizontal(|ui| {
            if ui.button("Clear canvas").clicked() {
                actions.push(Action::Clear);
            }
            ui.weak(format!("{} strokes, {} dabs", m.strokes, m.dabs));
        });
    });
}

fn stabilizer_section(ui: &mut Ui, m: &mut Model) {
    egui::CollapsingHeader::new(RichText::new("Stabilizer (Phase 1 preview)").strong()).default_open(false).show(ui, |ui| {
        let s = &mut m.settings.stabilizer;
        ui.checkbox(&mut s.enabled, "Use the stabilizer");
        ui.small("Krita's stabilizer: the line follows the pen smoothly. Changes apply to the next stroke.");
        ui.add_enabled_ui(s.enabled, |ui| {
            ui.add(egui::Slider::new(&mut s.distance, 3.0..=500.0).logarithmic(true).text("distance"));
            ui.horizontal(|ui| {
                ui.checkbox(&mut s.delay, "Delay");
                ui.add_enabled(s.delay, egui::Slider::new(&mut s.delay_distance, 1.0..=500.0).logarithmic(true).text("px"));
            });
            ui.checkbox(&mut s.finish_line, "Finish line (catch up on lift)");
            ui.checkbox(&mut s.stabilize_sensors, "Stabilize pressure and tilt");
        });
    });
}

fn wintab_section(ui: &mut Ui, m: &mut Model, actions: &mut Vec<Action>) {
    egui::CollapsingHeader::new(RichText::new("Wintab details").strong()).default_open(false).show(ui, |ui| {
        let w = &m.wintab;
        if w.info.is_empty() {
            ui.weak("Choose Wintab above to open the driver.");
        }
        egui::Grid::new("wtinfo").num_columns(2).spacing(vec2(8.0, 1.0)).show(ui, |ui| {
            for (k, v) in &w.info {
                ui.small(k.as_str());
                ui.small(v.as_str());
                ui.end_row();
            }
        });
        if !w.cursors.is_empty() {
            ui.small(format!("Cursors seen: {}", w.cursors.join("; ")));
        }
        if !w.last_raw.is_empty() {
            ui.small(RichText::new(format!("Last packet: {}", w.last_raw)).monospace());
        }
        ui.separator();
        ui.label("Screen mapping");
        let mut mode = m.settings.mapping_mode();
        egui::ComboBox::from_id_salt("mapping").width(240.0).selected_text(mode.label()).show_ui(ui, |ui| {
            for opt in MappingMode::ALL {
                ui.selectable_value(&mut mode, opt, opt.label());
            }
        });
        if mode != m.settings.mapping_mode() {
            m.settings.mapping = mode.key().to_string();
            actions.push(Action::SetMapping(mode));
        }
        if !w.mapping_active.is_empty() {
            ui.label(format!("In use: {}", w.mapping_active));
        }
        egui::Grid::new("mapping").num_columns(3).striped(true).spacing(vec2(8.0, 1.0)).show(ui, |ui| {
            ui.small("candidate");
            ui.small("off by (px)");
            ui.small("checks");
            ui.end_row();
            for (label, score) in &w.mapping_rows {
                ui.small(label.as_str());
                match score {
                    Some(s) if s.observations > 0 => {
                        ui.small(format!("{:.1}", s.error));
                        ui.small(format!("{}", s.observations));
                    }
                    _ => {
                        ui.small("–");
                        ui.small("0");
                    }
                }
                ui.end_row();
            }
            if let Some(f) = &w.fit {
                ui.small("Calibrated from the cursor");
                ui.small(format!("{:.2}", f.rms));
                ui.small(format!("{}", f.samples));
                ui.end_row();
            }
        });
        ui.small("\"off by\" compares each way of placing the tablet on screen with the system cursor while the pen moves slowly. Automatic picks the closest.");
        if ui.button("Forget calibration").clicked() {
            actions.push(Action::ResetCalibration);
        }
    });
}

fn ink_section(ui: &mut Ui, m: &mut Model) {
    egui::CollapsingHeader::new(RichText::new("Windows Ink details").strong()).default_open(false).show(ui, |ui| {
        if m.ink.devices.is_empty() {
            ui.weak("No pen digitizer registered with Windows.");
        }
        ui.small(format!("Pen in range right now: {}", yes_no(m.ink.in_range)));
        for d in &m.ink.devices {
            ui.small(d.as_str());
        }
        ui.small(format!("Most samples carried by one message: {}", m.ink.max_history));
        ui.small(format!("Samples that fell back to whole-pixel positions: {}", m.ink.coarse_fallbacks));
        ui.small(format!("Repeated frames dropped: {}", m.ink.repeats_dropped));
        ui.small("Windows Ink always reports pressure as 0…1024 (1025 levels), whatever the pen's own resolution.");
    });
}

fn screen_section(ui: &mut Ui, m: &mut Model, actions: &mut Vec<Action>) {
    egui::CollapsingHeader::new(RichText::new("Screen and display").strong()).default_open(false).show(ui, |ui| {
        ui.label(format!("This window: {} dpi ({}% scaling)", m.window_dpi, m.window_dpi * 100 / 96));
        for (i, mon) in m.monitors.iter().enumerate() {
            ui.horizontal(|ui| {
                let (l, t, r, b) = mon.rect;
                let text = format!(
                    "{}{} {}×{} at ({l},{t}), {}%",
                    if mon.has_window { "• " } else { "" },
                    if mon.name.is_empty() { format!("Monitor {}", i + 1) } else { mon.name.clone() },
                    r - l,
                    b - t,
                    mon.dpi * 100 / 96
                );
                ui.small(text);
                if !mon.has_window && ui.small_button("Move here").clicked() {
                    actions.push(Action::MoveToMonitor(i));
                }
            });
        }
        ui.add(egui::Slider::new(&mut m.settings.ui_scale, 0.75..=2.0).step_by(0.05).text("panel size"));
        let before = m.settings.low_latency;
        ui.checkbox(&mut m.settings.low_latency, "Low-latency presentation (may tear)");
        if before != m.settings.low_latency {
            actions.push(Action::PresentModeChanged);
        }
        ui.small(format!("Presentation: {}", m.frame.present_mode));
        ui.checkbox(&mut m.settings.panel_right, "Panel on the right");
        ui.small(format!("GPU: {}", m.gpu));
    });
}

fn record_section(ui: &mut Ui, m: &mut Model, actions: &mut Vec<Action>) {
    egui::CollapsingHeader::new(RichText::new("Recording and report").strong()).default_open(true).show(ui, |ui| {
        ui.checkbox(&mut m.recording, format!("Record pen data ({} samples)", m.recorded.len()));
        ui.horizontal(|ui| {
            if ui.button("Save recording + report").clicked() {
                actions.push(Action::SaveRecording);
            }
            if ui.button("Copy report").clicked() {
                actions.push(Action::CopyReport);
            }
        });
        if ui.button("Save drawing (.kra for Krita + .png)").clicked() {
            actions.push(Action::SaveDrawing);
        }
        if let Some(p) = &m.saved_to {
            ui.small(format!("Saved: {p}"));
            if ui.small_button("Open folder").clicked() {
                actions.push(Action::OpenFolder);
            }
        }
        ui.small("The report lists your driver, rates and ranges. Paste it (or attach the saved files) in the chat so I can check the results.");
    });
}

fn log_section(ui: &mut Ui, m: &mut Model) {
    egui::CollapsingHeader::new(RichText::new("Log").strong()).default_open(false).show(ui, |ui| {
        for line in m.log.iter().rev().take(60) {
            ui.small(line.as_str());
        }
    });
}

/// The whole window's UI for one frame: the side panel and the overlay on the canvas.
/// Returns the panel's actions and its rectangle (in points).
pub fn frame_ui(ui: &mut Ui, m: &mut Model, now_us: u64) -> (Vec<Action>, egui::Rect) {
    let side = if m.settings.panel_right { egui::Panel::right("panel") } else { egui::Panel::left("panel") };
    let mut actions = Vec::new();
    let resp = side.resizable(false).exact_size(PANEL_WIDTH).show(ui, |ui| {
        actions = panel(ui, m, now_us);
    });
    let ppp = ui.ctx().pixels_per_point();
    crosshair(ui.ctx(), m, ppp);
    (actions, resp.response.rect)
}

/// May a pen stroke start at this window pixel? Not outside the window, not on the panel,
/// not on a popup (an open list, a tooltip). Plain overlays such as the red cross, which
/// sits right under the pen, do not count.
pub fn stroke_may_start(ctx: &egui::Context, panel: egui::Rect, ppp: f32, size_px: (f64, f64), x: f64, y: f64) -> bool {
    if x < 0.0 || y < 0.0 || x >= size_px.0 || y >= size_px.1 {
        return false;
    }
    let p = egui::pos2(x as f32 / ppp, y as f32 / ppp);
    !panel.contains(p) && !ctx.layer_id_at(p).is_some_and(|l| l.order != egui::Order::Background)
}

/// Draws the red cross at the newest tablet position (canvas overlay, in points).
pub fn crosshair(ctx: &egui::Context, m: &Model, pixels_per_point: f32) {
    if !m.settings.show_crosshair {
        return;
    }
    // Stabilizer delay: the dead-zone circle around the pen and the leash to the brush.
    if let Some(crate::model::Leash { brush: (bx, by), pen: (px, py), radius: r }) = m.leash {
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("recedo leash")));
        let to_pt = |x: f64, y: f64| egui::pos2(x as f32 / pixels_per_point, y as f32 / pixels_per_point);
        let stroke = Stroke::new(1.0, Color32::from_rgb(80, 140, 230));
        painter.circle_stroke(to_pt(px, py), r as f32 / pixels_per_point, stroke);
        painter.line_segment([to_pt(bx, by), to_pt(px, py)], stroke);
    }
    let Some(s) = m.latest else { return };
    if s.source == Source::Mouse {
        return;
    }
    let p = egui::pos2(s.x as f32 / pixels_per_point, s.y as f32 / pixels_per_point);
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("recedo crosshair")));
    let color = if s.contact() { Color32::from_rgb(230, 30, 30) } else { Color32::from_rgb(230, 60, 60) };
    let r = 9.0;
    painter.line_segment([p - vec2(r, 0.0), p - vec2(2.5, 0.0)], Stroke::new(1.5, color));
    painter.line_segment([p + vec2(2.5, 0.0), p + vec2(r, 0.0)], Stroke::new(1.5, color));
    painter.line_segment([p - vec2(0.0, r), p - vec2(0.0, 2.5)], Stroke::new(1.5, color));
    painter.line_segment([p + vec2(0.0, 2.5), p + vec2(0.0, r)], Stroke::new(1.5, color));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every character the panel uses must exist in the bundled fonts. (egui 0.37 ships
    /// with no fonts unless its `default_fonts` feature is on; without it every letter
    /// rendered as a box.)
    #[test]
    fn panel_text_has_real_glyphs() {
        let ctx = egui::Context::default();
        let mut m = Model::default();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            panel(ui, &mut m, 0);
        })
        .drop_without_applying_deltas();
        let sample = "Recedo tablet test 0123456789 ABCxyz ()[]/%:,.+-";
        let special = "…°×•–";
        ctx.fonts_mut(|f| {
            for font in [egui::FontId::proportional(14.0), egui::FontId::monospace(12.0)] {
                assert!(f.has_glyphs(&font, sample), "missing basic glyphs in {font:?}");
                for c in special.chars() {
                    assert!(f.has_glyph(&font, c), "missing {c:?} in {font:?}");
                }
            }
        });
    }

    fn input(w: f32, h: f32) -> egui::RawInput {
        egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h))), ..Default::default() }
    }

    #[test]
    fn strokes_start_on_the_canvas_even_under_the_red_cross() {
        let ctx = egui::Context::default();
        let mut m = Model::default();
        assert!(m.settings.show_crosshair, "the cross is on by default");
        let mut s = recedo_tablet::PenSample::synthetic(0, 800.0, 300.0, 0.5, false);
        s.source = Source::Ink;
        m.latest = Some(s);
        let mut panel_rect = egui::Rect::NOTHING;
        for _ in 0..2 {
            ctx.run_ui(input(1280.0, 800.0), |ui| {
                panel_rect = frame_ui(ui, &mut m, 0).1;
            })
            .drop_without_applying_deltas();
        }
        let size = (1280.0, 800.0);
        assert!((panel_rect.width() - PANEL_WIDTH).abs() < 1.0, "{panel_rect:?}");
        assert!(stroke_may_start(&ctx, panel_rect, 1.0, size, 800.0, 300.0), "the cross must not block the pen");
        assert!(stroke_may_start(&ctx, panel_rect, 1.0, size, 400.0, 10.0));
        assert!(!stroke_may_start(&ctx, panel_rect, 1.0, size, 100.0, 300.0), "panel");
        assert!(!stroke_may_start(&ctx, panel_rect, 1.0, size, 1280.0, 300.0), "outside");
        assert!(!stroke_may_start(&ctx, panel_rect, 1.0, size, -1.0, 300.0), "outside");
    }

    #[test]
    fn strokes_do_not_start_on_a_popup() {
        let ctx = egui::Context::default();
        let mut m = Model::default();
        let mut panel_rect = egui::Rect::NOTHING;
        for _ in 0..2 {
            ctx.run_ui(input(1280.0, 800.0), |ui| {
                panel_rect = frame_ui(ui, &mut m, 0).1;
                // Stands in for an open drop-down list over the canvas.
                egui::Area::new(egui::Id::unique("test popup")).order(egui::Order::Foreground).fixed_pos(egui::pos2(600.0, 200.0)).show(ui.ctx(), |ui| {
                    ui.set_min_size(egui::vec2(200.0, 100.0));
                    ui.label("popup");
                });
            })
            .drop_without_applying_deltas();
        }
        assert!(!stroke_may_start(&ctx, panel_rect, 1.0, (1280.0, 800.0), 650.0, 250.0));
        assert!(stroke_may_start(&ctx, panel_rect, 1.0, (1280.0, 800.0), 1000.0, 600.0));
    }

    #[test]
    fn panel_on_the_right_and_high_dpi() {
        let ctx = egui::Context::default();
        ctx.set_zoom_factor(1.0);
        let mut m = Model::default();
        m.settings.panel_right = true;
        let mut panel_rect = egui::Rect::NOTHING;
        // 1920x1080 pixels at 150% = 1280x720 points.
        let mut raw = input(1280.0, 720.0);
        raw.viewports.entry(raw.viewport_id).or_default().native_pixels_per_point = Some(1.5);
        ctx.run_ui(raw, |ui| {
            panel_rect = frame_ui(ui, &mut m, 0).1;
        })
        .drop_without_applying_deltas();
        let size = (1920.0, 1080.0);
        // Pixel 1900 is point 1266.7: on the right-hand panel.
        assert!(!stroke_may_start(&ctx, panel_rect, 1.5, size, 1900.0, 500.0), "{panel_rect:?}");
        assert!(stroke_may_start(&ctx, panel_rect, 1.5, size, 100.0, 500.0));
    }

    #[test]
    fn panel_runs_with_live_data() {
        let ctx = egui::Context::default();
        let mut m = Model::default();
        let mut s = recedo_tablet::PenSample::synthetic(0, 100.0, 50.0, 0.4, true);
        s.flags.set(Flags::HAS_TILT, true);
        s.tilt_x = 30.0;
        m.latest = Some(s);
        m.stats.ink.record_batch(&[s], 1000, false);
        m.wintab.state = WintabState::Failed("not installed".into());
        let mut actions = Vec::new();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            actions = panel(ui, &mut m, 2000);
        })
        .drop_without_applying_deltas();
        assert!(actions.is_empty());
    }
}
