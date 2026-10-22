//! Autosave and crash recovery.
//!
//! While a painting has unsaved changes, a copy is written every few minutes (between
//! strokes, on a background thread, from a snapshot that shares tiles with the live
//! document) to the recovery folder as `<id>.recovery.kra`, with a small
//! `<id>.recovery.json` naming the original file. Saving the painting or closing it
//! normally deletes the copy. A copy still there at start-up means Recedo did not close
//! normally, and is offered back.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};

use crate::doc::Document;
use crate::kra;

pub const DEFAULT_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Decides when to autosave.
#[derive(Clone, Debug)]
pub struct Autosaver {
    pub interval: Duration,
    /// When the oldest unsaved change was made (`None` = nothing unsaved).
    dirty_since: Option<Instant>,
    last_save: Option<Instant>,
}

impl Autosaver {
    pub fn new(interval: Duration) -> Autosaver {
        Autosaver { interval, dirty_since: None, last_save: None }
    }

    /// Call after every change to the painting.
    pub fn changed(&mut self, now: Instant) {
        self.dirty_since.get_or_insert(now);
    }

    /// Call when the painting was saved (by the user or by an autosave).
    pub fn saved(&mut self, now: Instant) {
        self.dirty_since = None;
        self.last_save = Some(now);
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.dirty_since.is_some()
    }

    /// True when an autosave should start now. `busy` (a stroke in progress, or a save
    /// already running) postpones it.
    pub fn due(&self, now: Instant, busy: bool) -> bool {
        let Some(since) = self.dirty_since else { return false };
        if busy {
            return false;
        }
        // Wait a full interval after the first unsaved change, and after the last save.
        let base = self.last_save.map_or(since, |l| l.max(since));
        now.duration_since(base) >= self.interval
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecoveryInfo {
    /// The file the painting belongs to (`None` = never saved).
    pub original: Option<PathBuf>,
    /// Seconds since 1970 when the copy was written.
    pub saved_at: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Recoverable {
    pub kra: PathBuf,
    pub info: RecoveryInfo,
}

fn paths(dir: &Path, id: &str) -> (PathBuf, PathBuf) {
    let safe: String = id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    (dir.join(format!("{safe}.recovery.kra")), dir.join(format!("{safe}.recovery.json")))
}

/// Writes the recovery copy of a painting (`id` names it, e.g. the document's uuid).
pub fn write_recovery(dir: &Path, id: &str, doc: &Document, original: Option<&Path>) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let (kra_path, json_path) = paths(dir, id);
    kra::save_file(doc, &kra_path)?;
    let saved_at = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let info = RecoveryInfo { original: original.map(Path::to_path_buf), saved_at };
    let json = serde_json::to_vec_pretty(&info).map_err(|e| e.to_string())?;
    std::fs::write(&json_path, json).map_err(|e| format!("{}: {e}", json_path.display()))?;
    Ok(kra_path)
}

/// Runs [`write_recovery`] on a background thread. The document is a snapshot (cloning
/// shares tiles, so it is cheap and the live painting can keep changing).
pub fn spawn_recovery(dir: PathBuf, id: String, doc: Document, original: Option<PathBuf>) -> std::thread::JoinHandle<Result<PathBuf, String>> {
    std::thread::Builder::new()
        .name("recedo-autosave".into())
        .spawn(move || write_recovery(&dir, &id, &doc, original.as_deref()))
        .expect("can start the autosave thread")
}

/// Deletes a painting's recovery copy (after a save or a normal close).
pub fn remove_recovery(dir: &Path, id: &str) {
    let (k, j) = paths(dir, id);
    let _ = std::fs::remove_file(k);
    let _ = std::fs::remove_file(j);
}

/// Recovery copies left behind, newest first.
pub fn find_recoverable(dir: &Path) -> Vec<Recoverable> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<Recoverable> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.ends_with(".recovery.kra")))
        .map(|kra| {
            let json = kra.with_extension("json");
            let info = std::fs::read(&json).ok().and_then(|b| serde_json::from_slice::<RecoveryInfo>(&b).ok()).unwrap_or_else(|| {
                let modified = std::fs::metadata(&kra).and_then(|m| m.modified()).ok();
                RecoveryInfo { original: None, saved_at: modified.and_then(|m| m.duration_since(SystemTime::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs()) }
            });
            Recoverable { kra, info }
        })
        .collect();
    out.sort_by_key(|r| std::cmp::Reverse(r.info.saved_at));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autosave_timing() {
        let t0 = Instant::now();
        let s = |secs| t0 + Duration::from_secs(secs);
        let mut a = Autosaver::new(Duration::from_secs(300));
        assert!(!a.due(s(10_000), false), "nothing to save");
        a.changed(s(0));
        a.changed(s(100));
        assert!(!a.due(s(299), false));
        assert!(a.due(s(300), false), "5 minutes after the first unsaved change");
        assert!(!a.due(s(300), true), "not during a stroke");
        a.saved(s(301));
        assert!(!a.has_unsaved_changes());
        assert!(!a.due(s(2000), false));
        a.changed(s(302));
        assert!(!a.due(s(500), false));
        assert!(a.due(s(602), false));
    }

    #[test]
    fn recovery_roundtrip() {
        let dir = std::env::temp_dir().join(format!("recedo-recovery-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(find_recoverable(&dir).is_empty(), "missing folder is fine");
        let mut doc = Document::new(100, 80, Some([255; 4]));
        doc.layers[1].pixels.set_pixel(3, 4, [1, 2, 3, 255]);
        let original = PathBuf::from("C:\\Art\\cat.kra");
        let handle = spawn_recovery(dir.clone(), "{abc-123}".into(), doc.clone(), Some(original.clone()));
        let path = handle.join().unwrap().unwrap();
        assert_eq!(path.file_name().unwrap(), "abc-123.recovery.kra");
        write_recovery(&dir, "other", &Document::new(5, 5, None), None).unwrap();
        // A half-written file from a crash during the save is not offered.
        std::fs::write(dir.join("x.recovery.kra.part"), b"junk").unwrap();
        let found = find_recoverable(&dir);
        assert_eq!(found.len(), 2);
        let cat = found.iter().find(|r| r.info.original.is_some()).unwrap();
        assert_eq!(cat.info.original.as_deref(), Some(original.as_path()));
        assert!(cat.info.saved_at > 1_700_000_000);
        let back = kra::load_file(&cat.kra).unwrap().doc;
        assert_eq!(back.layers[1].pixels.pixel(3, 4), [1, 2, 3, 255]);
        remove_recovery(&dir, "{abc-123}");
        remove_recovery(&dir, "other");
        assert!(find_recoverable(&dir).is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
