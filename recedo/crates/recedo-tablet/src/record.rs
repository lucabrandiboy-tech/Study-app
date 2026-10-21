//! Saving and loading pen recordings as CSV.
//!
//! Recordings made on the real tablet are replayed by tests (stroke rendering, stabilizer)
//! and are how a problem the user sees can be reproduced exactly.

use crate::sample::{Flags, PenSample, Source};

pub const HEADER: &str =
    "source,t_us,recv_us,x,y,pressure,tilt_x,tilt_y,twist,flags,raw_pressure,raw_buttons,serial,cursor";

pub fn to_csv(samples: &[PenSample]) -> String {
    let mut out = String::with_capacity(64 + samples.len() * 96);
    out.push_str(HEADER);
    out.push('\n');
    for s in samples {
        use std::fmt::Write;
        let _ = writeln!(
            out,
            "{},{},{},{:.4},{:.4},{:.5},{:.2},{:.2},{:.2},{},{},{},{},{}",
            s.source.as_str(),
            s.t_us,
            s.recv_us,
            s.x,
            s.y,
            s.pressure,
            s.tilt_x,
            s.tilt_y,
            s.twist,
            s.flags.0,
            s.raw_pressure,
            s.raw_buttons,
            s.serial,
            s.cursor
        );
    }
    out
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Parses a recording. Lines starting with `#` and blank lines are ignored.
pub fn from_csv(text: &str) -> Result<Vec<PenSample>, ParseError> {
    let mut out = Vec::new();
    let mut header_seen = false;
    for (i, line) in text.lines().enumerate() {
        let line_no = i + 1;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if !header_seen {
            if line != HEADER {
                return Err(ParseError { line: line_no, message: "not a Recedo pen recording (unexpected header)".into() });
            }
            header_seen = true;
            continue;
        }
        let f: Vec<&str> = line.split(',').collect();
        if f.len() != 14 {
            return Err(ParseError { line: line_no, message: format!("expected 14 fields, found {}", f.len()) });
        }
        let err = |what: &str| ParseError { line: line_no, message: format!("bad {what}") };
        let num_u64 = |s: &str, what: &str| s.parse::<u64>().map_err(|_| err(what));
        let num_u32 = |s: &str, what: &str| s.parse::<u32>().map_err(|_| err(what));
        let num_f64 = |s: &str, what: &str| s.parse::<f64>().ok().filter(|v| v.is_finite()).ok_or_else(|| err(what));
        out.push(PenSample {
            source: Source::parse(f[0]).ok_or_else(|| err("source"))?,
            t_us: num_u64(f[1], "t_us")?,
            recv_us: num_u64(f[2], "recv_us")?,
            x: num_f64(f[3], "x")?,
            y: num_f64(f[4], "y")?,
            pressure: num_f64(f[5], "pressure")? as f32,
            tilt_x: num_f64(f[6], "tilt_x")? as f32,
            tilt_y: num_f64(f[7], "tilt_y")? as f32,
            twist: num_f64(f[8], "twist")? as f32,
            flags: Flags(num_u32(f[9], "flags")?),
            raw_pressure: num_u32(f[10], "raw_pressure")?,
            raw_buttons: num_u32(f[11], "raw_buttons")?,
            serial: num_u32(f[12], "serial")?,
            cursor: num_u32(f[13], "cursor")?,
        });
    }
    if !header_seen {
        return Err(ParseError { line: 0, message: "empty file".into() });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let mut a = PenSample::synthetic(1_000, 10.25, 20.5, 0.75, true);
        a.source = Source::Wintab;
        a.tilt_x = -32.5;
        a.tilt_y = 12.0;
        a.flags.set(Flags::HAS_TILT | Flags::ERASER, true);
        a.serial = 42;
        a.raw_buttons = 5;
        let b = PenSample::synthetic(6_000, 11.0, 21.0, 0.0, false);
        let text = to_csv(&[a, b]);
        let back = from_csv(&text).unwrap();
        assert_eq!(back, vec![a, b]);
    }

    #[test]
    fn rejects_garbage() {
        assert!(from_csv("hello\n1,2,3").is_err());
        assert!(from_csv("").is_err());
        let bad = format!("{HEADER}\nwintab,1,2,3\n");
        assert_eq!(from_csv(&bad).unwrap_err().line, 2);
        let nan = format!("{HEADER}\nink,1,1,NaN,0,0,0,0,0,0,0,0,0,0\n");
        assert!(from_csv(&nan).is_err());
    }

    #[test]
    fn comments_and_blank_lines_are_skipped() {
        let text = format!("# recorded on test rig\n\n{HEADER}\n# a comment\nmouse,1,1,0,0,1,0,0,0,17,1024,0,0,0\n");
        let v = from_csv(&text).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].source, Source::Mouse);
    }
}
