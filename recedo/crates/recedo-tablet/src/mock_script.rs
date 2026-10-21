//! The mock Wintab driver's tablet geometry and stroke script.
//!
//! Lives here (not in the mock crate) so the self-test can know what the mock sends
//! without linking the mock driver itself into the app.
use crate::orientation::tilt_to_azimuth_altitude;

pub const X_MAX: i32 = 50_799;
pub const Y_MAX: i32 = 28_574;
pub const PRESSURE_MAX: i32 = 8191;

/// One scripted pen state, in screen pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step {
    pub sx: f64,
    pub sy: f64,
    pub pressure: f64,
    pub tilt_x: f64,
    pub tilt_y: f64,
    pub tip: bool,
    /// Leave out this packet's serial number (simulates a lost packet).
    pub drop: bool,
}

/// The test stroke: a slow hover (lets the app calibrate its mapping), then a stroke
/// left to right with pressure rising 0.05 → 1 and tilt X sweeping -40° → +40°, one
/// deliberately lost packet, then lift-off and a short hover.
pub fn stroke(left: f64, top: f64, width: f64, height: f64) -> Vec<Step> {
    let mut v = Vec::new();
    let y = top + height * 0.5;
    // Hover: 60 steps of ~1.5 px across, wobbling vertically so both axes move.
    for i in 0..60 {
        let t = i as f64 / 60.0;
        v.push(Step {
            sx: left + width * 0.1 + t * 90.0,
            sy: y - 40.0 + (t * std::f64::consts::TAU).sin() * 20.0,
            pressure: 0.0,
            tilt_x: 0.0,
            tilt_y: 0.0,
            tip: false,
            drop: false,
        });
    }
    let n = 150;
    for i in 0..=n {
        let t = i as f64 / n as f64;
        v.push(Step {
            sx: left + width * (0.1 + 0.8 * t),
            sy: y + (t * std::f64::consts::PI * 2.0).sin() * height * 0.15,
            pressure: 0.05 + 0.95 * t,
            tilt_x: -40.0 + 80.0 * t,
            tilt_y: 20.0,
            tip: true,
            drop: i == 75,
        });
    }
    for i in 0..10 {
        v.push(Step { sx: left + width * 0.9 + i as f64, sy: y, pressure: 0.0, tilt_x: 0.0, tilt_y: 0.0, tip: false, drop: false });
    }
    v
}

/// Tablet output coordinates for a screen position, given the context's mapping.
pub fn to_tablet(sx: f64, sy: f64, sys: (f64, f64, f64, f64), out: (i32, i32, i32, i32)) -> (i32, i32) {
    let (ox, oy, ex, ey) = out;
    let u = ((sx - sys.0) / sys.2).clamp(0.0, 1.0);
    let v = ((sy - sys.1) / sys.3).clamp(0.0, 1.0);
    let x = ox as f64 + u * ex.unsigned_abs() as f64;
    // Negative Y extent = Y grows downward (what Recedo asks for).
    let y = if ey < 0 { oy as f64 + v * ey.unsigned_abs() as f64 } else { oy as f64 + (1.0 - v) * ey as f64 };
    (x.round() as i32, y.round() as i32)
}

/// Raw orientation (tenths of a degree) for a tilt.
pub fn orientation(tilt_x: f64, tilt_y: f64) -> (i32, i32) {
    let (az, alt) = tilt_to_azimuth_altitude(tilt_x, tilt_y);
    ((az * 10.0).round() as i32, (alt * 10.0).round() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_shape() {
        let s = stroke(0.0, 0.0, 1000.0, 600.0);
        assert_eq!(s.iter().filter(|s| s.tip).count(), 151);
        assert_eq!(s.iter().filter(|s| s.drop).count(), 1);
        // Hover steps are small enough for the app's calibration to use them.
        for w in s[..60].windows(2) {
            assert!((w[1].sx - w[0].sx).hypot(w[1].sy - w[0].sy) < 4.0);
        }
    }

    #[test]
    fn tablet_mapping() {
        let sys = (0.0, 0.0, 1920.0, 1080.0);
        assert_eq!(to_tablet(960.0, 0.0, sys, (0, 0, 50_800, -28_575)), (25_400, 0));
        assert_eq!(to_tablet(0.0, 1080.0, sys, (0, 0, 50_800, 28_575)), (0, 0));
    }
}
