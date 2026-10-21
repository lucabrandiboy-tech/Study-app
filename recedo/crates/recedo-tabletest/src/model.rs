//! The test app's state that the panel shows and edits. Portable: the Windows layer fills
//! in the platform parts (Wintab/Ink details, monitors) as plain data.

use std::collections::VecDeque;

use recedo_tablet::mapping::{Fit, MappingMode, Score};
use recedo_tablet::stats::SourceStats;
use recedo_tablet::{PenSample, Source};
use serde::{Deserialize, Serialize};

use crate::brush::BrushSettings;

/// Which tablet API draws on the canvas (Krita: Settings > Tablet settings > Tablet Input API).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Api {
    WindowsInk,
    Wintab,
    MouseOnly,
}

impl Api {
    pub const ALL: [Api; 3] = [Api::WindowsInk, Api::Wintab, Api::MouseOnly];

    pub fn label(self) -> &'static str {
        match self {
            Api::WindowsInk => "Windows Ink",
            Api::Wintab => "Wintab",
            Api::MouseOnly => "Mouse only (no tablet)",
        }
    }

    pub fn source(self) -> Source {
        match self {
            Api::WindowsInk => Source::Ink,
            Api::Wintab => Source::Wintab,
            Api::MouseOnly => Source::Mouse,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub api: Api,
    /// [`MappingMode::key`].
    pub mapping: String,
    pub brush: BrushSettings,
    pub show_dots: bool,
    pub show_hover_dots: bool,
    pub show_crosshair: bool,
    pub low_latency: bool,
    pub ui_scale: f32,
    pub panel_right: bool,
    /// Window placement: left, top, right, bottom of the restored window, and maximised.
    pub window: Option<(i32, i32, i32, i32, bool)>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            api: Api::WindowsInk,
            mapping: MappingMode::Auto.key().to_string(),
            brush: BrushSettings::default(),
            show_dots: false,
            show_hover_dots: false,
            show_crosshair: true,
            low_latency: false,
            ui_scale: 1.0,
            panel_right: false,
            window: None,
        }
    }
}

impl Settings {
    pub fn load(path: &std::path::Path) -> Settings {
        std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn mapping_mode(&self) -> MappingMode {
        MappingMode::from_key(&self.mapping)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub wintab: SourceStats,
    pub ink: SourceStats,
    pub mouse: SourceStats,
}

impl Stats {
    pub fn get(&self, s: Source) -> &SourceStats {
        match s {
            Source::Wintab => &self.wintab,
            Source::Ink => &self.ink,
            Source::Mouse | Source::Synthetic => &self.mouse,
        }
    }

    pub fn get_mut(&mut self, s: Source) -> &mut SourceStats {
        match s {
            Source::Wintab => &mut self.wintab,
            Source::Ink => &mut self.ink,
            Source::Mouse | Source::Synthetic => &mut self.mouse,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct FrameStats {
    pub fps: f64,
    pub frame_ms_avg: f64,
    pub frame_ms_max: f64,
    pub samples_per_frame: f64,
    /// Age of the newest drawn sample when its frame was handed to the GPU, in ms.
    pub input_age_ms: f64,
    pub present_mode: String,
    pub frames: u64,
    /// The app's own work per frame (UI, tessellation, recording GPU commands), in ms,
    /// smoothed. This is what the app controls.
    pub work_ms: f64,
    /// Time per frame spent waiting for the GPU or the display (getting a free image and
    /// presenting), in ms, smoothed. Mostly the display's refresh rate and the GPU's speed.
    pub wait_ms: f64,
    /// The last frame's unsmoothed timings (for measurements over a set of frames).
    pub last_work_ms: f64,
    pub last_wait_ms: f64,
}

impl FrameStats {
    /// Folds one frame's timings into the smoothed values.
    pub fn add_timing(&mut self, work_ms: f64, wait_ms: f64) {
        self.last_work_ms = work_ms;
        self.last_wait_ms = wait_ms;
        if self.frames <= 1 {
            self.work_ms = work_ms;
            self.wait_ms = wait_ms;
        } else {
            self.work_ms += 0.1 * (work_ms - self.work_ms);
            self.wait_ms += 0.1 * (wait_ms - self.wait_ms);
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum WintabState {
    #[default]
    Closed,
    Open,
    Failed(String),
}

#[derive(Clone, Debug, Default)]
pub struct WintabView {
    pub state: WintabState,
    pub device_name: String,
    pub info: Vec<(String, String)>,
    pub mapping_active: String,
    pub mapping_rows: Vec<(String, Option<Score>)>,
    pub fit: Option<Fit>,
    pub mismatch: bool,
    pub cursors: Vec<String>,
    pub in_proximity: bool,
    pub last_raw: String,
}

#[derive(Clone, Debug, Default)]
pub struct InkView {
    pub devices: Vec<String>,
    pub max_history: u32,
    pub coarse_fallbacks: u64,
    pub repeats_dropped: u64,
    pub in_range: bool,
}

#[derive(Clone, Debug, Default)]
pub struct MonitorView {
    pub name: String,
    pub rect: (i32, i32, i32, i32),
    pub dpi: u32,
    pub primary: bool,
    pub has_window: bool,
}

/// Things the panel asks the platform layer to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    SetApi(Api),
    SetMapping(MappingMode),
    ResetCalibration,
    Clear,
    ClearDots,
    SaveRecording,
    CopyReport,
    OpenFolder,
    MoveToMonitor(usize),
    ResetStats,
    PresentModeChanged,
}

pub const LOG_LINES: usize = 200;

#[derive(Default)]
pub struct Model {
    pub settings: Settings,
    pub stats: Stats,
    pub frame: FrameStats,
    pub wintab: WintabView,
    pub ink: InkView,
    pub monitors: Vec<MonitorView>,
    pub recording: bool,
    pub recorded: Vec<PenSample>,
    pub log: VecDeque<String>,
    /// Newest sample from the API that draws.
    pub latest: Option<PenSample>,
    pub strokes: usize,
    pub dabs: u64,
    pub saved_to: Option<String>,
    pub gpu: String,
    pub window_dpi: u32,
    pub os: String,
}

impl Model {
    pub fn log(&mut self, line: impl Into<String>) {
        let line = line.into();
        if self.log.len() == LOG_LINES {
            self.log.pop_front();
        }
        self.log.push_back(line);
    }

    /// Recordings are capped so a forgotten "Record" cannot eat all memory (about 2 hours
    /// at 266 samples per second).
    pub const MAX_RECORDED: usize = 2_000_000;

    pub fn record(&mut self, samples: &[PenSample]) {
        if self.recording {
            let room = Self::MAX_RECORDED.saturating_sub(self.recorded.len());
            self.recorded.extend_from_slice(&samples[..samples.len().min(room)]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_roundtrip_and_defaults_for_missing_fields() {
        let dir = std::env::temp_dir().join(format!("recedo-settings-test-{}", std::process::id()));
        let path = dir.join("s.json");
        let s = Settings { api: Api::Wintab, brush: BrushSettings { size: 77.0, ..Default::default() }, ..Default::default() };
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);
        // An older file with fewer fields still loads.
        std::fs::write(&path, r#"{"api":"Wintab"}"#).unwrap();
        let old = Settings::load(&path);
        assert_eq!(old.api, Api::Wintab);
        assert_eq!(old.brush, BrushSettings::default());
        // Garbage gives defaults.
        std::fs::write(&path, "not json").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn recording_is_capped() {
        let mut m = Model { recording: true, ..Default::default() };
        let s = PenSample::synthetic(0, 0.0, 0.0, 0.5, true);
        m.recorded = vec![s; Model::MAX_RECORDED - 1];
        m.record(&[s, s, s]);
        assert_eq!(m.recorded.len(), Model::MAX_RECORDED);
    }
}
