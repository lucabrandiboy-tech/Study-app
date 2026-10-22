//! Command-line helper for checking `.kra` files against the real Krita
//! (used by `scripts/krita-interop.sh`).
//!
//!   kra_tool sample <out.kra>               write a test painting
//!   kra_tool flatten <in.kra> <out.png>     flatten a .kra the way Recedo does
//!   kra_tool same <a.kra> <b.kra>           do two files hold the same layers and pixels?
//!   kra_tool diff <a.png> <b.png> <max>     are two pictures within <max> per channel?
//!   kra_tool bench <dir>                    time saving and opening a 3840 x 2160, 6-layer painting

use std::path::Path;
use std::process::ExitCode;

use recedo_core::doc::{Document, Layer};
use recedo_core::image::{decode_png, encode_png};
use recedo_core::kra;
use recedo_core::tiles::TiledLayer;

fn sample() -> Document {
    let mut doc = Document::new(300, 200, Some([250, 245, 235, 255]));
    doc.dpi = 300.0;
    let ink = &mut doc.layers[1];
    ink.name = "Ink & \"lines\" <1>".into();
    for i in 0..400 {
        let t = i as f64 / 400.0;
        let (x, y) = ((-20.0 + 340.0 * t) as i32, (100.0 + 70.0 * (t * 9.0).sin()) as i32);
        for dy in -2..=2 {
            ink.pixels.set_pixel(x, y + dy, [20, 20, 30, 255]);
        }
    }
    let mut colour = Layer::new("Colour (60%)");
    colour.opacity = 153;
    for y in 30..170 {
        for x in 40..260 {
            let a = ((x - 40) * 255 / 220) as u8;
            colour.pixels.set_pixel(x, y, [230, (y % 200) as u8, 60, a]);
        }
    }
    doc.layers.insert(1, colour);
    let mut hidden = Layer::new("Hidden sketch");
    hidden.visible = false;
    hidden.pixels = TiledLayer::filled([255, 0, 0, 255]);
    doc.layers.push(hidden);
    let mut locked = Layer::new("Locked highlights");
    locked.locked = true;
    for y in 0..64 {
        for x in 200..300 {
            locked.pixels.set_pixel(x, y, [255, 255, 255, 120]);
        }
    }
    doc.layers.push(locked);
    doc
}

fn run(args: &[String]) -> Result<(), String> {
    match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["sample", out] => kra::save_file(&sample(), Path::new(out)),
        ["flatten", input, out] => {
            let l = kra::load_file(Path::new(input))?;
            for w in &l.warnings {
                eprintln!("warning: {w}");
            }
            std::fs::write(out, encode_png(&l.doc.flatten(), false)?).map_err(|e| e.to_string())
        }
        ["same", a, b] => {
            let (a, b) = (kra::load_file(Path::new(a))?, kra::load_file(Path::new(b))?);
            for w in a.warnings.iter().chain(&b.warnings) {
                eprintln!("warning: {w}");
            }
            let (a, b) = (a.doc, b.doc);
            if (a.width, a.height) != (b.width, b.height) {
                return Err(format!("size {}x{} vs {}x{}", a.width, a.height, b.width, b.height));
            }
            if a.layers.len() != b.layers.len() {
                return Err(format!("{} layers vs {}", a.layers.len(), b.layers.len()));
            }
            for (la, lb) in a.layers.iter().zip(&b.layers) {
                let meta = |l: &Layer| (l.name.clone(), l.visible, l.opacity, l.locked);
                if meta(la) != meta(lb) {
                    return Err(format!("layer {:?} vs {:?}", meta(la), meta(lb)));
                }
                for y in -64..a.height as i32 + 64 {
                    for x in -64..a.width as i32 + 64 {
                        if la.pixels.pixel(x, y) != lb.pixels.pixel(x, y) {
                            return Err(format!("layer \"{}\" differs at {x},{y}: {:?} vs {:?}", la.name, la.pixels.pixel(x, y), lb.pixels.pixel(x, y)));
                        }
                    }
                }
                println!("  layer \"{}\": same (visible {}, opacity {}, locked {})", la.name, la.visible, la.opacity, la.locked);
            }
            Ok(())
        }
        ["diff", a, b, max] => {
            let read = |p: &str| decode_png(&std::fs::read(p).map_err(|e| format!("{p}: {e}"))?);
            let (a, b) = (read(a)?, read(b)?);
            let max: u8 = max.parse().map_err(|_| "bad max")?;
            if (a.width, a.height) != (b.width, b.height) {
                return Err(format!("size {}x{} vs {}x{}", a.width, a.height, b.width, b.height));
            }
            let mut worst = 0;
            for (p, q) in a.data.chunks_exact(4).zip(b.data.chunks_exact(4)) {
                let channels = if p[3] == 0 && q[3] == 0 { 3..4 } else { 0..4 };
                for c in channels {
                    worst = worst.max(p[c].abs_diff(q[c]));
                }
            }
            println!("  largest channel difference: {worst}");
            if worst > max { Err(format!("pictures differ by up to {worst}")) } else { Ok(()) }
        }
        ["bench", dir] => {
            let mut doc = Document::new(3840, 2160, Some([255, 255, 255, 255]));
            doc.layers.clear();
            doc.layers.push(Layer { pixels: TiledLayer::filled([255, 255, 255, 255]), ..Layer::new("Background") });
            for n in 0..5u32 {
                let mut l = Layer::new(&format!("Painted {n}"));
                // Soft gradients over the whole canvas with a little noise, like painted colour.
                let mut seed = 0x9E37_79B9u32.wrapping_mul(n + 1);
                for y in 0..2160u32 {
                    for x in 0..3840u32 {
                        seed ^= seed << 13;
                        seed ^= seed >> 17;
                        seed ^= seed << 5;
                        let noise = (seed & 3) as u8;
                        l.pixels.set_pixel(x as i32, y as i32, [(x / 16) as u8 ^ noise, (y / 9) as u8, (n * 50) as u8, 128 + noise]);
                    }
                }
                doc.layers.push(l);
            }
            let path = Path::new(dir).join("bench.kra");
            let t = std::time::Instant::now();
            kra::save_file(&doc, &path)?;
            let save = t.elapsed();
            let size = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
            let t = std::time::Instant::now();
            let back = kra::load_file(&path)?;
            let load = t.elapsed();
            let t = std::time::Instant::now();
            let flattened = back.doc.flatten();
            let flat = t.elapsed();
            let t = std::time::Instant::now();
            let _ = encode_png(&flattened, true)?;
            println!("PNG of the flattened picture: {:.2?}", t.elapsed());
            let t = std::time::Instant::now();
            for l in &doc.layers {
                let _ = kra::write_tiles(&l.pixels);
            }
            println!("tiles of all layers, one thread: {:.2?}", t.elapsed());
            let t = std::time::Instant::now();
            let mut h = recedo_core::history::History::default();
            for _ in 0..50 {
                h.record("Brush stroke", &doc);
            }
            println!("undo snapshot of this painting: {:.2?} each", t.elapsed() / 50);
            if back.doc != doc {
                return Err("bench file did not read back the same".into());
            }
            println!("3840x2160, 6 layers: save {save:.2?} ({:.1} MB), open {load:.2?}, flatten {flat:.2?}", size as f64 / 1e6);
            Ok(())
        }
        _ => Err("usage: kra_tool sample <out.kra> | flatten <in.kra> <out.png> | same <a.kra> <b.kra> | diff <a.png> <b.png> <max>".into()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
