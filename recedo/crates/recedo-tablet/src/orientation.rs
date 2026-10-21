//! Converting Wintab pen orientation (azimuth/altitude) to tilt X/Y.

use crate::wintab_abi::{AXIS, TU_CIRCLE, fix32_to_f64};

/// How many raw units make one degree on a Wintab orientation axis.
///
/// Wintab describes angles with `axUnits == TU_CIRCLE` and `axResolution` = units per full
/// circle (16.16 fixed point), usually 3600 (tenths of a degree). If a driver leaves that
/// out, tenths of a degree are assumed, which is what the spec's examples and every
/// driver we know of use.
pub fn units_per_degree(axis: &AXIS) -> f64 {
    if axis.axUnits == TU_CIRCLE && axis.axResolution != 0 {
        let per_circle = fix32_to_f64(axis.axResolution);
        if per_circle > 0.0 {
            return per_circle / 360.0;
        }
    }
    10.0
}

/// Converts azimuth (clockwise from the top of the tablet, degrees) and altitude (angle
/// above the tablet surface, degrees) to tilt X/Y in degrees, W3C convention: X positive
/// leaning right, Y positive leaning toward the user.
///
/// For altitudes above 0 this equals Qt's Wintab formula
/// (`atan(sin(az) / tan(alt))`, `-atan(cos(az) / tan(alt))`). It is computed from the pen's
/// direction vector instead so a pen lying flat gives (±90, 0) rather than NaN or a
/// spurious ±90 on the other axis from rounding noise.
pub fn azimuth_altitude_to_tilt(azimuth_deg: f64, altitude_deg: f64) -> (f64, f64) {
    let az = azimuth_deg.to_radians();
    let alt = altitude_deg.abs().min(90.0).to_radians();
    let snap = |v: f64| if v.abs() < 1e-12 { 0.0 } else { v };
    let dx = snap(alt.cos() * az.sin());
    let dy = snap(-alt.cos() * az.cos());
    let dz = alt.sin();
    (clamp_tilt(dx.atan2(dz).to_degrees()), clamp_tilt(dy.atan2(dz).to_degrees()))
}

/// The inverse of [`azimuth_altitude_to_tilt`]; used by tests and the mock driver.
pub fn tilt_to_azimuth_altitude(tilt_x_deg: f64, tilt_y_deg: f64) -> (f64, f64) {
    let tx = tilt_x_deg.to_radians().tan();
    let ty = tilt_y_deg.to_radians().tan();
    let mut azimuth = tx.atan2(-ty).to_degrees();
    if azimuth < 0.0 {
        azimuth += 360.0;
    }
    let altitude = 1.0f64.atan2(tx.hypot(ty)).to_degrees();
    (azimuth, altitude)
}

fn clamp_tilt(v: f64) -> f64 {
    // Rounding in the atan math can produce -0.0 or a hair past 90.
    let v = v.clamp(-90.0, 90.0);
    if v == 0.0 { 0.0 } else { v }
}

/// Orientation of one packet in degrees.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Orientation {
    pub azimuth: f64,
    pub altitude: f64,
    pub twist: f64,
}

/// Converts a raw packet orientation using the device's axis descriptions.
pub fn decode_orientation(raw: [i32; 3], axes: &[AXIS; 3]) -> Orientation {
    Orientation {
        azimuth: raw[0] as f64 / units_per_degree(&axes[0]),
        altitude: raw[1] as f64 / units_per_degree(&axes[1]),
        twist: raw[2] as f64 / units_per_degree(&axes[2]),
    }
}

/// Whether the device reports usable tilt (altitude axis present).
pub fn has_tilt(axes: &[AXIS; 3]) -> bool {
    axes[0].axMax > axes[0].axMin && axes[1].axMax != axes[1].axMin
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn upright_pen_has_no_tilt() {
        for az in [0.0, 45.0, 90.0, 270.0] {
            let (x, y) = azimuth_altitude_to_tilt(az, 90.0);
            assert!(close(x, 0.0) && close(y, 0.0), "az {az}: {x} {y}");
        }
    }

    #[test]
    fn directions_follow_w3c_convention() {
        // Pen top pointing right (east, azimuth 90) at 45° above the surface.
        let (x, y) = azimuth_altitude_to_tilt(90.0, 45.0);
        assert!(close(x, 45.0) && close(y, 0.0), "{x} {y}");
        // Toward the user (south, azimuth 180): tilt Y positive.
        let (x, y) = azimuth_altitude_to_tilt(180.0, 30.0);
        assert!(close(x, 0.0) && close(y, 60.0), "{x} {y}");
        // Away from the user (north): tilt Y negative.
        let (_, y) = azimuth_altitude_to_tilt(0.0, 60.0);
        assert!(close(y, -30.0), "{y}");
        // Left (west).
        let (x, _) = azimuth_altitude_to_tilt(270.0, 60.0);
        assert!(close(x, -30.0), "{x}");
    }

    #[test]
    fn flat_pen_does_not_produce_nan() {
        let (x, y) = azimuth_altitude_to_tilt(90.0, 0.0);
        assert!(close(x, 90.0), "{x}");
        assert!(close(y, 0.0), "{y}");
    }

    #[test]
    fn inverse_roundtrips() {
        for &(tx, ty) in &[(0.0, 0.0), (30.0, 0.0), (-40.0, 25.0), (10.0, -55.0), (59.0, 59.0)] {
            let (az, alt) = tilt_to_azimuth_altitude(tx, ty);
            let (x, y) = azimuth_altitude_to_tilt(az, alt);
            assert!((x - tx).abs() < 1e-6 && (y - ty).abs() < 1e-6, "({tx},{ty}) -> ({az},{alt}) -> ({x},{y})");
        }
    }

    #[test]
    fn unit_scaling() {
        let tenths = AXIS { axMin: 0, axMax: 3599, axUnits: TU_CIRCLE, axResolution: 3600 << 16 };
        assert!(close(units_per_degree(&tenths), 10.0));
        let degrees = AXIS { axMin: 0, axMax: 359, axUnits: TU_CIRCLE, axResolution: 360 << 16 };
        assert!(close(units_per_degree(&degrees), 1.0));
        let unknown = AXIS { axMin: 0, axMax: 900, axUnits: 0, axResolution: 0 };
        assert!(close(units_per_degree(&unknown), 10.0));
        let o = decode_orientation([900, 450, 0], &[tenths, tenths, tenths]);
        assert!(close(o.azimuth, 90.0) && close(o.altitude, 45.0));
    }
}
