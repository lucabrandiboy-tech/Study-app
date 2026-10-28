//! Settings kept between sessions (`%APPDATA%\Recedo\recedo.json` on Windows).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::dock::DockLayout;
use crate::editor::{Tool, ToolSettings};

/// Which tablet interface the pen comes through.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TabletApi {
    /// Windows Ink (Windows' own pen support).
    #[default]
    Ink,
    /// The tablet driver's Wintab interface (like Krita's "WinTab" setting).
    Wintab,
}

impl TabletApi {
    pub fn label(self) -> &'static str {
        match self {
            TabletApi::Ink => "Windows Ink",
            TabletApi::Wintab => "Wintab",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub tools: ToolSettings,
    pub color: [u8; 3],
    pub color2: [u8; 3],
    pub recent_colors: Vec<[u8; 3]>,
    pub dock: DockLayout,
    pub tablet: TabletApi,
    /// Interface size multiplier on top of Windows' display scaling.
    pub ui_scale: f32,
    /// Minutes between autosaves of paintings with unsaved changes.
    pub autosave_minutes: u32,
    /// Last folder used in a file dialog.
    pub last_folder: Option<PathBuf>,
    pub recent_files: Vec<PathBuf>,
    /// JPEG export quality, 1..=100.
    pub jpeg_quality: u8,
    /// Index into the new-canvas presets last used.
    pub last_preset: usize,
    /// Saved window position: left, top, right, bottom, maximised.
    pub window: Option<(i32, i32, i32, i32, bool)>,
    /// The tools on the side bar, in order (tool ids, so a newer version's tools never make
    /// the file unreadable).
    pub hotbar: Vec<String>,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            tools: ToolSettings::default(),
            color: [0, 0, 0],
            color2: [255, 255, 255],
            recent_colors: Vec::new(),
            dock: DockLayout::default(),
            tablet: TabletApi::Ink,
            ui_scale: 1.0,
            autosave_minutes: 5,
            last_folder: None,
            recent_files: Vec::new(),
            jpeg_quality: 92,
            last_preset: 0,
            window: None,
            hotbar: Tool::DEFAULT_BAR.iter().map(|t| t.id().to_string()).collect(),
        }
    }
}

pub const MAX_RECENT: usize = 10;

impl Settings {
    /// Loads settings; a missing or damaged file gives the defaults (and the damaged file is
    /// kept next to it as `.broken` so nothing is silently lost).
    pub fn load(path: &Path) -> Settings {
        let Ok(text) = std::fs::read_to_string(path) else { return Settings::default() };
        match serde_json::from_str::<Settings>(&text) {
            Ok(mut s) => {
                s.sanitize();
                s
            }
            Err(_) => {
                let _ = std::fs::rename(path, path.with_extension("json.broken"));
                Settings::default()
            }
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(".part");
        let tmp = PathBuf::from(tmp);
        std::fs::write(&tmp, text).and_then(|()| std::fs::rename(&tmp, path)).map_err(|e| e.to_string())
    }

    /// Keeps hand-edited or old values in range.
    /// The side bar's tools, in order.
    pub fn hotbar_tools(&self) -> Vec<Tool> {
        self.hotbar.iter().filter_map(|id| Tool::from_id(id)).collect()
    }

    pub fn set_hotbar(&mut self, tools: &[Tool]) {
        self.hotbar = tools.iter().map(|t| t.id().to_string()).collect();
    }

    pub fn sanitize(&mut self) {
        // The old standard side bar gets the new standard one (with the lasso transform).
        if self.hotbar.iter().map(String::as_str).eq(Tool::OLD_DEFAULT_BAR) {
            self.set_hotbar(&Tool::DEFAULT_BAR);
        }
        // The old default colour (a very dark blue) becomes the new one, black.
        if self.color == [20, 20, 30] {
            self.color = [0, 0, 0];
        }
        self.ui_scale = if self.ui_scale.is_finite() { self.ui_scale.clamp(0.5, 3.0) } else { 1.0 };
        self.autosave_minutes = self.autosave_minutes.clamp(1, 120);
        self.jpeg_quality = self.jpeg_quality.clamp(1, 100);
        for b in [&mut self.tools.brush, &mut self.tools.eraser] {
            if !b.size.is_finite() {
                b.size = 20.0;
            }
            b.size = b.size.clamp(crate::editor::MIN_BRUSH, crate::editor::MAX_BRUSH);
            b.opacity = if b.opacity.is_finite() { b.opacity.clamp(0.0, 1.0) } else { 1.0 };
            b.flow = if b.flow.is_finite() { b.flow.clamp(0.0, 1.0) } else { 1.0 };
            b.hardness = if b.hardness.is_finite() { b.hardness.clamp(0.0, 1.0) } else { 0.8 };
            b.spacing = if b.spacing.is_finite() { b.spacing.clamp(0.01, 5.0) } else { 0.1 };
            b.min_size = if b.min_size.is_finite() { b.min_size.clamp(0.0, 1.0) } else { 0.02 };
        }
        let st = &mut self.tools.stabilizer;
        st.migrate();
        st.distance = if st.distance.is_finite() { st.distance.clamp(3.0, 500.0) } else { 50.0 };
        st.delay_distance = if st.delay_distance.is_finite() { st.delay_distance.clamp(1.0, 500.0) } else { 50.0 };
        self.tools.pick_radius = if self.tools.pick_radius.is_finite() { self.tools.pick_radius.clamp(0.0, 50.0) } else { 0.0 };
        let r = &mut self.tools.retouch;
        let fix = |v: f32, lo: f32, hi: f32, d: f32| if v.is_finite() { v.clamp(lo, hi) } else { d };
        r.size = fix(r.size, crate::editor::MIN_BRUSH, crate::editor::MAX_BRUSH, 60.0);
        r.strength = fix(r.strength, 0.0, 1.0, 0.5);
        r.hardness = fix(r.hardness, 0.0, 1.0, 0.3);
        r.smudge_length = fix(r.smudge_length, 0.0, 0.99, 0.85);
        r.spray_density = fix(r.spray_density, 1.0, 200.0, 12.0);
        r.spray_dot = fix(r.spray_dot, 0.5, 200.0, 4.0);
        self.tools.wand_tolerance = fix(self.tools.wand_tolerance, 0.0, 1.0, 0.12);
        self.tools.fill_tolerance = fix(self.tools.fill_tolerance, 0.0, 1.0, 0.12);
        let t = &mut self.tools.text;
        t.size = fix(t.size, 4.0, 2000.0, 64.0);
        t.line_spacing = fix(t.line_spacing, 0.5, 4.0, 1.0);
        // Known tools only, each once; an empty bar gets the default back.
        let mut seen = std::collections::HashSet::new();
        let bar: Vec<Tool> = self.hotbar_tools().into_iter().filter(|t| seen.insert(*t)).collect();
        self.set_hotbar(if bar.is_empty() { &Tool::DEFAULT_BAR } else { &bar });
        self.dock.sanitize();
        self.recent_files.truncate(MAX_RECENT);
        self.recent_colors.truncate(crate::editor::MAX_RECENT_COLORS);
    }

    /// Puts a file at the top of the recent list.
    pub fn add_recent(&mut self, path: &Path) {
        self.recent_files.retain(|p| p != path);
        self.recent_files.insert(0, path.to_path_buf());
        self.recent_files.truncate(MAX_RECENT);
        if let Some(dir) = path.parent() {
            self.last_folder = Some(dir.to_path_buf());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_damage() {
        let dir = std::env::temp_dir().join(format!("recedo-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("recedo.json");
        assert_eq!(Settings::load(&path), Settings::default(), "missing file: defaults");
        let mut s = Settings { tablet: TabletApi::Wintab, ui_scale: 1.25, ..Settings::default() };
        s.tools.brush.size = 33.0;
        s.add_recent(Path::new("C:/Art/cat.kra"));
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);
        // Out-of-range values are pulled back; unknown fields are ignored.
        std::fs::write(&path, r#"{"ui_scale": 50, "jpeg_quality": 0, "tools": {"brush": {"size": 99999}}, "future_thing": 1}"#).unwrap();
        let t = Settings::load(&path);
        assert_eq!((t.ui_scale, t.jpeg_quality, t.tools.brush.size), (3.0, 1, crate::editor::MAX_BRUSH));
        // A damaged file gives defaults and is kept aside.
        std::fs::write(&path, "{not json").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
        assert!(dir.join("recedo.json.broken").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_side_bar_keeps_known_tools_once() {
        let mut s = Settings::default();
        assert_eq!(s.hotbar_tools(), Tool::DEFAULT_BAR.to_vec());
        s.hotbar = vec!["smudge".into(), "no_such_tool".into(), "brush".into(), "smudge".into()];
        s.sanitize();
        assert_eq!(s.hotbar_tools(), vec![Tool::Smudge, Tool::Brush]);
        assert_eq!(s.hotbar, vec!["smudge", "brush"]);
        s.hotbar.clear();
        s.sanitize();
        assert_eq!(s.hotbar_tools(), Tool::DEFAULT_BAR.to_vec(), "never empty");
        // Saved with the old standard bar: the new standard one, with the lasso transform.
        s.hotbar = Tool::OLD_DEFAULT_BAR.iter().map(|id| id.to_string()).collect();
        s.sanitize();
        assert!(s.hotbar_tools().contains(&Tool::LassoTransform));
        // A bar the user chose stays as it is.
        s.hotbar = vec!["brush".into(), "move".into()];
        s.sanitize();
        assert_eq!(s.hotbar_tools(), vec![Tool::Brush, Tool::Move]);
        // Every tool has its own id, and it reads back.
        let ids: std::collections::HashSet<&str> = Tool::ALL.iter().map(|t| t.id()).collect();
        assert_eq!(ids.len(), Tool::ALL.len());
        assert!(Tool::ALL.iter().all(|t| Tool::from_id(t.id()) == Some(*t)));
    }

    /// Settings saved before the adaptive stabilizer: it becomes the adaptive one, as strong as
    /// the Stability slider showed it; settings without any stabilizer get the default.
    #[test]
    fn an_older_stabilizer_becomes_the_adaptive_one() {
        use recedo_core::stabilizer::SmoothMode;
        let dir = std::env::temp_dir().join(format!("recedo-stab-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("recedo.json");
        let mut first = recedo_core::StabilizerSettings { mode: SmoothMode::Average, ..Default::default() };
        first.set_stability(37);
        std::fs::write(&path, format!(r#"{{"tools": {{"stabilizer": {{"enabled": true, "distance": {}, "finish_line": true, "scalable_distance": false}}}}}}"#, first.distance)).unwrap();
        let s = Settings::load(&path).tools.stabilizer;
        assert_eq!((s.mode, s.enabled, s.stability()), (SmoothMode::Adaptive, true, 37));
        std::fs::write(&path, "{}").unwrap();
        let s = Settings::load(&path).tools.stabilizer;
        assert_eq!((s.mode, s.enabled), (SmoothMode::Adaptive, false));
        // Krita's average, chosen in the details, stays chosen.
        let mut kept = Settings::default();
        kept.tools.stabilizer.mode = SmoothMode::Average;
        kept.tools.stabilizer.set_stability(60);
        kept.save(&path).unwrap();
        let s = Settings::load(&path).tools.stabilizer;
        assert_eq!((s.mode, s.stability()), (SmoothMode::Average, 60));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn recent_files_are_unique_and_capped() {
        let mut s = Settings::default();
        for i in 0..15 {
            s.add_recent(Path::new(&format!("/a/{i}.kra")));
        }
        s.add_recent(Path::new("/a/3.kra"));
        assert_eq!(s.recent_files.len(), MAX_RECENT);
        assert_eq!(s.recent_files[0], Path::new("/a/3.kra"));
        assert_eq!(s.recent_files.iter().filter(|p| p.ends_with("3.kra")).count(), 1);
    }
}
