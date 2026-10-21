//! Mapping Wintab tablet coordinates to screen pixels.
//!
//! Wintab hands out tablet coordinates; turning them into screen positions needs the
//! screen area the pen covers. Drivers describe that differently (the system context's
//! `lcSys*` rectangle, the whole virtual desktop, one monitor), and on multi-monitor PCs it
//! is often wrong, which shows up as strokes landing away from the pen tip. Qt falls back
//! to the integer mouse position in that case; Krita asks the user.
//!
//! The driver always moves the system cursor to the pen tip, so the cursor is ground truth
//! (to the pixel). [`Calibrator`] scores every candidate mapping against the cursor while
//! the pen moves slowly, and in `Auto` mode uses the best one. It also fits a per-axis
//! linear mapping by least squares, which covers any sub-area or offset mapping the
//! driver may use. Only the choice of mapping comes from the cursor: positions still come
//! from the packets, so they keep sub-pixel precision and every packet.

/// A rectangle in virtual-desktop physical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect { x, y, w, h }
    }

    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && py >= self.y && px < self.x + self.w && py < self.y + self.h
    }
}

/// A tablet position normalised to the context's output area: `u` 0..1 left to right,
/// `v` 0..1 top to bottom (assuming the driver honoured the flipped Y output we ask for).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Norm {
    pub u: f64,
    pub v: f64,
}

/// The context's output coordinate space (`lcOutOrg*`, `lcOutExt*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputSpace {
    pub org_x: i32,
    pub org_y: i32,
    pub ext_x: i32,
    pub ext_y: i32,
}

impl OutputSpace {
    pub fn normalize(&self, x: i32, y: i32) -> Norm {
        let ex = (self.ext_x.unsigned_abs().max(1)) as f64;
        let ey = (self.ext_y.unsigned_abs().max(1)) as f64;
        let u = (x - self.org_x) as f64 / ex;
        let mut v = (y - self.org_y) as f64 / ey;
        // A positive Y extent is Wintab's native "Y up" orientation.
        if self.ext_y > 0 {
            v = 1.0 - v;
        }
        Norm { u, v }
    }
}

/// Which screen area a fixed candidate maps the tablet onto.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    /// The rectangle in the driver's default system context (`lcSys*`).
    SystemContext,
    /// The whole virtual desktop (all monitors).
    VirtualDesktop,
    /// The monitor the cursor is on.
    Monitor,
}

impl Target {
    pub const ALL: [Target; 3] = [Target::SystemContext, Target::VirtualDesktop, Target::Monitor];

    pub fn label(self) -> &'static str {
        match self {
            Target::SystemContext => "Tablet-provided area",
            Target::VirtualDesktop => "Whole virtual screen",
            Target::Monitor => "Monitor under the pen",
        }
    }
}

/// The screen rectangles the candidates map to, refreshed by the platform layer.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Targets {
    pub system_context: Rect,
    pub virtual_desktop: Rect,
    pub monitor: Option<Rect>,
}

impl Targets {
    pub fn rect(&self, t: Target) -> Option<Rect> {
        match t {
            Target::SystemContext => Some(self.system_context).filter(|r| r.w > 0.0 && r.h > 0.0),
            Target::VirtualDesktop => Some(self.virtual_desktop).filter(|r| r.w > 0.0 && r.h > 0.0),
            Target::Monitor => self.monitor,
        }
    }
}

/// A way of turning a [`Norm`] into screen pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Candidate {
    Fixed { target: Target, flip_y: bool },
    /// Least-squares fit against the cursor.
    Fitted,
}

impl Candidate {
    /// The six fixed candidates, in a stable order.
    pub const FIXED: [Candidate; 6] = [
        Candidate::Fixed { target: Target::SystemContext, flip_y: false },
        Candidate::Fixed { target: Target::VirtualDesktop, flip_y: false },
        Candidate::Fixed { target: Target::Monitor, flip_y: false },
        Candidate::Fixed { target: Target::SystemContext, flip_y: true },
        Candidate::Fixed { target: Target::VirtualDesktop, flip_y: true },
        Candidate::Fixed { target: Target::Monitor, flip_y: true },
    ];

    pub fn label(self) -> String {
        match self {
            Candidate::Fixed { target, flip_y: false } => target.label().to_string(),
            Candidate::Fixed { target, flip_y: true } => format!("{} (Y flipped)", target.label()),
            Candidate::Fitted => "Calibrated from the cursor".to_string(),
        }
    }

    fn index(self) -> Option<usize> {
        Candidate::FIXED.iter().position(|&c| c == self)
    }
}

/// What the user picked in settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum MappingMode {
    /// Pick automatically by comparing with the system cursor.
    #[default]
    Auto,
    Fixed(Target),
    Calibrated,
}

impl MappingMode {
    pub const ALL: [MappingMode; 5] = [
        MappingMode::Auto,
        MappingMode::Fixed(Target::SystemContext),
        MappingMode::Fixed(Target::VirtualDesktop),
        MappingMode::Fixed(Target::Monitor),
        MappingMode::Calibrated,
    ];

    pub fn label(self) -> &'static str {
        match self {
            MappingMode::Auto => "Automatic (match the cursor)",
            MappingMode::Fixed(t) => t.label(),
            MappingMode::Calibrated => "Calibrated from the cursor",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            MappingMode::Auto => "auto",
            MappingMode::Fixed(Target::SystemContext) => "system",
            MappingMode::Fixed(Target::VirtualDesktop) => "virtual",
            MappingMode::Fixed(Target::Monitor) => "monitor",
            MappingMode::Calibrated => "calibrated",
        }
    }

    pub fn from_key(s: &str) -> MappingMode {
        MappingMode::ALL.into_iter().find(|m| m.key() == s).unwrap_or_default()
    }
}

/// Maps a [`Norm`] with a fixed candidate.
pub fn map_fixed(n: Norm, rect: Rect, flip_y: bool) -> (f64, f64) {
    let v = if flip_y { 1.0 - n.v } else { n.v };
    (rect.x + n.u * rect.w, rect.y + v * rect.h)
}

/// Running least-squares fit of `screen = a * t + b` for one axis.
#[derive(Clone, Copy, Debug, Default)]
struct AxisFit {
    n: f64,
    st: f64,
    stt: f64,
    ss: f64,
    sts: f64,
    sss: f64,
    t_min: f64,
    t_max: f64,
}

impl AxisFit {
    fn add(&mut self, t: f64, s: f64) {
        if self.n == 0.0 {
            self.t_min = t;
            self.t_max = t;
        }
        self.n += 1.0;
        self.st += t;
        self.stt += t * t;
        self.ss += s;
        self.sts += t * s;
        self.sss += s * s;
        self.t_min = self.t_min.min(t);
        self.t_max = self.t_max.max(t);
    }

    fn solve(&self) -> Option<(f64, f64, f64)> {
        let det = self.n * self.stt - self.st * self.st;
        if self.n < 2.0 || det.abs() < 1e-12 {
            return None;
        }
        let a = (self.n * self.sts - self.st * self.ss) / det;
        let b = (self.ss - a * self.st) / self.n;
        // Residual sum of squares from the accumulated sums.
        let rss = self.sss - 2.0 * a * self.sts - 2.0 * b * self.ss + a * a * self.stt + 2.0 * a * b * self.st + b * b * self.n;
        let rms = (rss.max(0.0) / self.n).sqrt();
        Some((a, b, rms))
    }

    fn span(&self) -> f64 {
        if self.n == 0.0 { 0.0 } else { self.t_max - self.t_min }
    }
}

/// Fitted linear mapping per axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fit {
    pub ax: f64,
    pub bx: f64,
    pub ay: f64,
    pub by: f64,
    /// RMS distance between the fit and the cursor positions, in pixels.
    pub rms: f64,
    pub samples: u32,
}

/// Score of one candidate against the cursor.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Score {
    /// Exponential moving average of the distance to the cursor, in pixels.
    pub error: f64,
    pub observations: u32,
}

const EMA_ALPHA: f64 = 0.15;
const MIN_OBSERVATIONS: u32 = 8;
/// Below this average distance (pixels) a fixed candidate counts as matching.
const GOOD_ERROR: f64 = 6.0;
/// A candidate must beat the current one by this much to take over.
const HYSTERESIS: f64 = 2.0;
const FIT_MIN_SAMPLES: u32 = 40;
const FIT_MIN_SPAN: f64 = 0.05;
const FIT_MAX_RMS: f64 = 3.0;
/// Packets that moved more than this (pixels) since the last observation are skipped,
/// because the cursor lags behind a fast-moving pen.
const MAX_STEP_FOR_OBSERVATION: f64 = 4.0;

/// Chooses and applies the tablet-to-screen mapping.
#[derive(Clone, Debug)]
pub struct Calibrator {
    pub mode: MappingMode,
    scores: [Score; 6],
    fit_x: AxisFit,
    fit_y: AxisFit,
    chosen: Candidate,
    last_observed: Option<(f64, f64)>,
}

impl Default for Calibrator {
    fn default() -> Self {
        Calibrator::new(MappingMode::Auto)
    }
}

impl Calibrator {
    pub fn new(mode: MappingMode) -> Calibrator {
        Calibrator {
            mode,
            scores: [Score::default(); 6],
            fit_x: AxisFit::default(),
            fit_y: AxisFit::default(),
            chosen: Candidate::FIXED[0],
            last_observed: None,
        }
    }

    /// Forgets everything learned (e.g. after the display layout changed).
    pub fn reset(&mut self) {
        *self = Calibrator::new(self.mode);
    }

    pub fn score(&self, c: Candidate) -> Option<Score> {
        c.index().map(|i| self.scores[i])
    }

    pub fn fit(&self) -> Option<Fit> {
        let (ax, bx, rx) = self.fit_x.solve()?;
        let (ay, by, ry) = self.fit_y.solve()?;
        Some(Fit { ax, bx, ay, by, rms: rx.hypot(ry), samples: self.fit_x.n as u32 })
    }

    fn fit_usable(&self) -> Option<Fit> {
        let f = self.fit()?;
        let ok = f.samples >= FIT_MIN_SAMPLES
            && self.fit_x.span() >= FIT_MIN_SPAN
            && self.fit_y.span() >= FIT_MIN_SPAN
            && f.rms <= FIT_MAX_RMS;
        ok.then_some(f)
    }

    /// The candidate currently in use.
    pub fn active(&self) -> Candidate {
        match self.mode {
            MappingMode::Auto => self.chosen,
            MappingMode::Fixed(target) => Candidate::Fixed { target, flip_y: false },
            MappingMode::Calibrated => {
                if self.fit_usable().is_some() {
                    Candidate::Fitted
                } else {
                    Candidate::FIXED[0]
                }
            }
        }
    }

    /// True when automatic mode found nothing that lines up with the cursor.
    pub fn mismatch(&self) -> bool {
        if self.mode != MappingMode::Auto {
            return false;
        }
        match self.chosen {
            Candidate::Fitted => false,
            c => self.score(c).is_some_and(|s| s.observations >= MIN_OBSERVATIONS && s.error > GOOD_ERROR),
        }
    }

    /// Maps with a specific candidate.
    pub fn map_with(&self, c: Candidate, n: Norm, targets: &Targets) -> Option<(f64, f64)> {
        match c {
            Candidate::Fixed { target, flip_y } => targets.rect(target).map(|r| map_fixed(n, r, flip_y)),
            Candidate::Fitted => self.fit().map(|f| (f.ax * n.u + f.bx, f.ay * n.v + f.by)),
        }
    }

    /// Maps with the active candidate, falling back to the system context rectangle.
    pub fn map(&self, n: Norm, targets: &Targets) -> (f64, f64) {
        self.map_with(self.active(), n, targets)
            .or_else(|| targets.rect(Target::VirtualDesktop).map(|r| map_fixed(n, r, false)))
            .unwrap_or((n.u, n.v))
    }

    /// Feeds one packet position with the cursor position read right after it.
    ///
    /// Call this for the newest packet of each batch only, since the cursor matches the
    /// newest packet.
    pub fn observe(&mut self, n: Norm, cursor: (f64, f64), targets: &Targets) {
        let here = self.map(n, targets);
        let slow = match self.last_observed {
            Some(prev) => (here.0 - prev.0).hypot(here.1 - prev.1) <= MAX_STEP_FOR_OBSERVATION,
            None => false,
        };
        self.last_observed = Some(here);
        if !slow {
            return;
        }
        for (i, c) in Candidate::FIXED.iter().enumerate() {
            if let Some(p) = self.map_with(*c, n, targets) {
                let err = (p.0 - cursor.0).hypot(p.1 - cursor.1);
                let s = &mut self.scores[i];
                s.error = if s.observations == 0 { err } else { s.error + EMA_ALPHA * (err - s.error) };
                s.observations = s.observations.saturating_add(1);
            }
        }
        // The cursor sits on whole pixels; its centre is half a pixel in.
        self.fit_x.add(n.u, cursor.0 + 0.5);
        self.fit_y.add(n.v, cursor.1 + 0.5);
        self.choose();
    }

    fn choose(&mut self) {
        let best = Candidate::FIXED
            .iter()
            .copied()
            .filter_map(|c| self.score(c).map(|s| (c, s)))
            .filter(|(_, s)| s.observations >= MIN_OBSERVATIONS)
            .min_by(|a, b| a.1.error.total_cmp(&b.1.error));
        let Some((best_c, best_s)) = best else { return };
        let current_error = match self.chosen {
            Candidate::Fitted => self.fit_usable().map(|f| f.rms),
            c => self.score(c).filter(|s| s.observations >= MIN_OBSERVATIONS).map(|s| s.error),
        };
        if best_s.error <= GOOD_ERROR {
            let switch = match current_error {
                None => true,
                Some(e) => self.chosen != best_c && best_s.error + HYSTERESIS < e,
            };
            if switch {
                self.chosen = best_c;
            }
        } else if let Some(f) = self.fit_usable() {
            if current_error.is_none_or(|e| f.rms + HYSTERESIS < e) {
                self.chosen = Candidate::Fitted;
            }
        } else if current_error.is_none_or(|e| best_s.error + HYSTERESIS < e) {
            self.chosen = best_c;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake driver: maps the tablet onto `area` and reports the cursor there.
    struct Driver {
        area: Rect,
        flip: bool,
    }

    impl Driver {
        fn norm_for(&self, sx: f64, sy: f64) -> Norm {
            let u = (sx - self.area.x) / self.area.w;
            let v = (sy - self.area.y) / self.area.h;
            Norm { u, v: if self.flip { 1.0 - v } else { v } }
        }
    }

    fn two_monitors() -> Targets {
        Targets {
            // Many drivers report the whole desktop here.
            system_context: Rect::new(0.0, 0.0, 3840.0, 1080.0),
            virtual_desktop: Rect::new(0.0, 0.0, 3840.0, 1080.0),
            monitor: Some(Rect::new(1920.0, 0.0, 1920.0, 1080.0)),
        }
    }

    /// Slow hover path (steps of about 1.5 px) over the middle of the given area. The
    /// cursor is reported on whole pixels, like the real one.
    fn hover(cal: &mut Calibrator, d: &Driver, targets: &Targets) {
        let steps = 1500;
        for i in 0..steps {
            let t = i as f64 / steps as f64;
            let sx = d.area.x + d.area.w * (0.3 + 0.4 * t);
            let sy = d.area.y + d.area.h * (0.4 + 0.15 * (std::f64::consts::TAU * 2.0 * t).sin());
            cal.observe(d.norm_for(sx, sy), (sx.floor(), sy.floor()), targets);
        }
    }

    #[test]
    fn output_space_normalisation() {
        let flipped = OutputSpace { org_x: 0, org_y: 0, ext_x: 50_000, ext_y: -30_000 };
        assert_eq!(flipped.normalize(25_000, 0), Norm { u: 0.5, v: 0.0 });
        let native = OutputSpace { org_x: 0, org_y: 0, ext_x: 50_000, ext_y: 30_000 };
        assert_eq!(native.normalize(0, 0), Norm { u: 0.0, v: 1.0 });
    }

    #[test]
    fn auto_finds_the_pen_display_on_a_two_monitor_pc() {
        let targets = two_monitors();
        let d = Driver { area: targets.monitor.unwrap(), flip: false };
        let mut cal = Calibrator::default();
        // Before any observation the tablet-provided area is used.
        assert_eq!(cal.active(), Candidate::FIXED[0]);
        hover(&mut cal, &d, &targets);
        assert_eq!(cal.active(), Candidate::Fixed { target: Target::Monitor, flip_y: false });
        assert!(!cal.mismatch());
        let (x, y) = cal.map(d.norm_for(2500.25, 600.75), &targets);
        assert!((x - 2500.25).abs() < 1e-9 && (y - 600.75).abs() < 1e-9);
    }

    #[test]
    fn auto_keeps_a_correct_system_context() {
        let mut targets = two_monitors();
        targets.system_context = targets.monitor.unwrap();
        let d = Driver { area: targets.system_context, flip: false };
        let mut cal = Calibrator::default();
        hover(&mut cal, &d, &targets);
        // Monitor and system context are the same rectangle here; either is right, and the
        // first one found must not flip-flop.
        let a = cal.active();
        assert!(matches!(a, Candidate::Fixed { flip_y: false, .. }), "{a:?}");
        let (x, _) = cal.map(d.norm_for(2000.0, 500.0), &targets);
        assert!((x - 2000.0).abs() < 1e-9);
    }

    #[test]
    fn auto_detects_a_driver_that_ignores_the_y_flip() {
        let targets = two_monitors();
        let d = Driver { area: targets.monitor.unwrap(), flip: true };
        let mut cal = Calibrator::default();
        hover(&mut cal, &d, &targets);
        assert_eq!(cal.active(), Candidate::Fixed { target: Target::Monitor, flip_y: true });
    }

    #[test]
    fn auto_falls_back_to_the_fit_for_a_sub_area_mapping() {
        // The driver maps the tablet to a region no candidate describes.
        let targets = two_monitors();
        let d = Driver { area: Rect::new(2100.0, 90.0, 1500.0, 844.0), flip: false };
        let mut cal = Calibrator::default();
        hover(&mut cal, &d, &targets);
        assert_eq!(cal.active(), Candidate::Fitted);
        let f = cal.fit().unwrap();
        assert!(f.rms < 1.0, "rms {}", f.rms);
        let (x, y) = cal.map(d.norm_for(3000.0, 500.0), &targets);
        assert!((x - 3000.0).abs() < 1.0 && (y - 500.0).abs() < 1.0, "{x} {y}");
    }

    #[test]
    fn fast_moves_are_not_used_for_scoring() {
        let targets = two_monitors();
        let d = Driver { area: targets.monitor.unwrap(), flip: false };
        let mut cal = Calibrator::default();
        // Every step jumps 50 px: the cursor would lag, so nothing is observed.
        for i in 0..100 {
            let sx = 2000.0 + (i % 30) as f64 * 50.0;
            cal.observe(d.norm_for(sx, 500.0), (sx, 500.0), &targets);
        }
        assert_eq!(cal.score(Candidate::FIXED[2]).unwrap().observations, 0);
    }

    #[test]
    fn forced_mode_is_respected() {
        let targets = two_monitors();
        let d = Driver { area: targets.monitor.unwrap(), flip: false };
        let mut cal = Calibrator::new(MappingMode::Fixed(Target::VirtualDesktop));
        hover(&mut cal, &d, &targets);
        assert_eq!(cal.active(), Candidate::Fixed { target: Target::VirtualDesktop, flip_y: false });
        assert!(!cal.mismatch());
    }

    #[test]
    fn mode_keys_roundtrip() {
        for m in MappingMode::ALL {
            assert_eq!(MappingMode::from_key(m.key()), m);
        }
        assert_eq!(MappingMode::from_key("nonsense"), MappingMode::Auto);
    }
}
