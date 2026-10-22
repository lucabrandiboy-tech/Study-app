//! Compares Recedo's blend modes with pictures the real Krita flattened
//! (used by hand; see `scripts/krita-interop.sh` for the automatic checks).
//!
//!   blend_check make <dir>                      write bottom.png and top.png test layers
//!   blend_check compare <dir> <mode-id> <krita.png>   flatten them with the mode and compare

use recedo_core::blend::{BlendMode, blend};
use recedo_core::image::{Rgba8, decode_png, encode_png};

fn layers() -> (Rgba8, Rgba8) {
    let (w, h) = (64u32, 64u32);
    let mut bottom = Rgba8::new(w, h);
    let mut top = Rgba8::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            // Bottom: colour gradients, opaque except the last 8 rows (half) and columns (clear).
            let ba = if x >= 56 { 0 } else if y >= 56 { 128 } else { 255 };
            bottom.data[i..i + 4].copy_from_slice(&[(x * 4) as u8, (y * 4) as u8, (255 - x * 2) as u8, ba]);
            // Top: different gradients, alpha falling off to the right half.
            let ta = if x < 32 { 255 } else { (255 - (x - 32) * 8) as u8 };
            top.data[i..i + 4].copy_from_slice(&[(y * 4) as u8, (255 - y * 3) as u8, (x * 3 + y) as u8, ta]);
        }
    }
    (bottom, top)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (bottom, top) = layers();
    match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["make", dir] => {
            std::fs::write(format!("{dir}/bottom.png"), encode_png(&bottom, false).unwrap()).unwrap();
            std::fs::write(format!("{dir}/top.png"), encode_png(&top, false).unwrap()).unwrap();
        }
        ["compare", _dir, mode, krita] => {
            let mode = BlendMode::from_krita_id(mode).expect("known mode id");
            let k = decode_png(&std::fs::read(krita).unwrap()).unwrap();
            let mut worst = (0u8, 0u32, 0u32, [0u8; 4], [0u8; 4]);
            let mut diffs = 0;
            for y in 0..64u32 {
                for x in 0..64u32 {
                    // Krita's OpenRaster import goes through premultiplied 8-bit, which
                    // loses colour on nearly transparent pixels; compare where it cannot.
                    let (ta, ba) = (top.pixel(x, y)[3], bottom.pixel(x, y)[3]);
                    if (1..128).contains(&ta) || (1..128).contains(&ba) {
                        continue;
                    }
                    let mut px = bottom.pixel(x, y);
                    blend(&mut px, top.pixel(x, y), 255, mode);
                    let kp = k.pixel(x, y);
                    let channels = if px[3] == 0 && kp[3] == 0 { 3..4 } else { 0..4 };
                    let d = channels.map(|c| px[c].abs_diff(kp[c])).max().unwrap();
                    if d > 1 {
                        diffs += 1;
                    }
                    if d > worst.0 {
                        worst = (d, x, y, px, kp);
                    }
                }
            }
            let (d, x, y, ours, theirs) = worst;
            println!("{:<13} worst {d:>3} at ({x:>2},{y:>2}) ours {ours:?} krita {theirs:?}; pixels off by >1: {diffs}", mode.krita_id());
        }
        _ => eprintln!("usage: blend_check make <dir> | compare <dir> <mode-id> <krita.png>"),
    }
}
