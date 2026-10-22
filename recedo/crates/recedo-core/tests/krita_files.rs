//! Files written by the real Krita 5.2.2 open correctly.
//!
//! `krita-5.2.2-one-layer.kra`: Krita's own conversion of `krita-5.2.2-one-layer.png`
//! (a half-transparent red circle and an opaque blue square on a transparent canvas).
//! `krita-5.2.2-two-layers.kra`: Krita's conversion of a two-layer OpenRaster file: a
//! red layer at 50 % opacity on top of a hidden blue layer, both 64 x 64.

use recedo_core::image::{Rgba8, decode_png};
use recedo_core::kra;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

/// The `mergedimage.png` Krita stored in the file: its own flattening of the layers.
fn krita_merged(kra: &[u8]) -> Rgba8 {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(kra)).unwrap();
    let mut png = Vec::new();
    std::io::Read::read_to_end(&mut z.by_name("mergedimage.png").unwrap(), &mut png).unwrap();
    decode_png(&png).unwrap()
}

/// Largest channel difference, ignoring the colour of fully transparent pixels.
fn max_diff(a: &Rgba8, b: &Rgba8) -> u8 {
    assert_eq!((a.width, a.height), (b.width, b.height));
    let mut worst = 0;
    for (p, q) in a.data.chunks_exact(4).zip(b.data.chunks_exact(4)) {
        let channels = if p[3] == 0 && q[3] == 0 { 3..4 } else { 0..4 };
        for c in channels {
            worst = worst.max(p[c].abs_diff(q[c]));
        }
    }
    worst
}

#[test]
fn one_layer_file_matches_the_source_picture() {
    let kra_bytes = fixture("krita-5.2.2-one-layer.kra");
    let loaded = kra::load(&kra_bytes).unwrap();
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    let doc = &loaded.doc;
    assert_eq!((doc.width, doc.height, doc.dpi), (200, 120, 72.0));
    assert_eq!(doc.layers.len(), 1);
    assert_eq!(doc.layers[0].name, "Background");
    assert_eq!(doc.layers[0].uuid, "{51c7d8d6-fa82-49e1-af53-0a06f0ad74db}");
    let source = decode_png(&fixture("krita-5.2.2-one-layer.png")).unwrap();
    let ours = doc.flatten();
    assert_eq!(max_diff(&ours, &source), 0, "layer pixels differ from the picture Krita was given");
    assert_eq!(max_diff(&ours, &krita_merged(&kra_bytes)), 0, "flattening differs from Krita's");
    // The picture really has the expected content (guards against comparing two blanks).
    assert_eq!(ours.pixel(150, 60), [20, 40, 200, 255]);
    let circle = ours.pixel(60, 60);
    assert_eq!(&circle[..3], &[200, 30, 40]);
    assert!((150..=155).contains(&circle[3]), "{circle:?}");
    assert_eq!(ours.pixel(5, 5)[3], 0);
}

#[test]
fn two_layer_file_has_order_opacity_and_visibility() {
    let kra_bytes = fixture("krita-5.2.2-two-layers.kra");
    let loaded = kra::load(&kra_bytes).unwrap();
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    let layers = &loaded.doc.layers;
    assert_eq!(layers.len(), 2);
    // Bottom first.
    assert_eq!(layers[0].name, "BottomBlue");
    assert!(!layers[0].visible);
    assert_eq!(layers[0].opacity, 255);
    assert_eq!(layers[0].pixels.pixel(10, 10), [0, 0, 255, 255]);
    assert_eq!(layers[1].name, "TopRed");
    assert!(layers[1].visible);
    assert_eq!(layers[1].opacity, 127);
    assert_eq!(layers[1].pixels.pixel(63, 63), [255, 0, 0, 255]);
    let ours = loaded.doc.flatten();
    assert_eq!(ours.pixel(0, 0), [255, 0, 0, 127]);
    assert_eq!(max_diff(&ours, &krita_merged(&kra_bytes)), 0, "flattening differs from Krita's");
}

#[test]
fn resaved_krita_file_keeps_its_pixels() {
    for name in ["krita-5.2.2-one-layer.kra", "krita-5.2.2-two-layers.kra"] {
        let first = kra::load(&fixture(name)).unwrap().doc;
        let again = kra::load(&kra::save(&first).unwrap()).unwrap();
        assert!(again.warnings.is_empty());
        assert_eq!(again.doc, first, "{name}");
    }
}

/// `krita-5.2.2-16bit.kra`: Krita's conversion of a 16-bit PNG (a red-to-blue gradient
/// fading to half transparency); `krita-5.2.2-16bit-export.png` is Krita's own 16-bit
/// export of that file. Recedo opens 16-bit layers as 8-bit, with a warning.
#[test]
fn sixteen_bit_file_opens_as_8_bit() {
    let loaded = kra::load(&fixture("krita-5.2.2-16bit.kra")).unwrap();
    assert!(loaded.warnings.iter().any(|w| w.contains("16-bit")), "{:?}", loaded.warnings);
    let ours = loaded.doc.flatten();
    let krita = decode_png(&fixture("krita-5.2.2-16bit-export.png")).unwrap();
    // Krita's 16-bit export, cut to 8 bits by the PNG reader, vs Recedo's rounding: within 1.
    assert!(max_diff(&ours, &krita) <= 1, "differs by {}", max_diff(&ours, &krita));
    let top = ours.pixel(10, 0);
    assert!(top[0] > 240 && top[2] < 20 && top[3] == 255, "{top:?}");
    let bottom = ours.pixel(10, 69);
    assert!(bottom[2] > 230 && (125..=130).contains(&bottom[3]), "{bottom:?}");
}

/// Each `.kra` in `krita-groups/` (layer groups: pass-through, isolated, nested, hidden,
/// zero opacity, with blend modes) and `krita-blend/` (all 16 blend modes over a grid of
/// alpha values, top layer at opacity 77) next to the `.png` the real Krita 5.2.2 flattened
/// it to. Recedo must flatten them identically, except where it opens an isolated group of
/// Normal layers as separate layers (to keep them editable): 8-bit Normal compositing is not
/// associative, so that differs by 1 on some semi-transparent pixels.
#[test]
fn group_and_blend_cases_match_krita() {
    let mut checked = 0;
    for dir in ["krita-groups", "krita-blend"] {
        let path = format!("{}/tests/fixtures/{dir}", env!("CARGO_MANIFEST_DIR"));
        let mut names: Vec<String> = std::fs::read_dir(&path)
            .unwrap()
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .filter_map(|n| n.strip_suffix(".kra").map(String::from))
            .collect();
        names.sort();
        for name in names {
            let doc = kra::load(&fixture(&format!("{dir}/{name}.kra"))).unwrap().doc;
            let krita = decode_png(&fixture(&format!("{dir}/{name}.png"))).unwrap();
            let allowed = if name == "iso_outer_pt_inner_normal" { 1 } else { 0 };
            let d = max_diff(&doc.flatten(), &krita);
            assert!(d <= allowed, "{dir}/{name}: differs from Krita by {d}");
            checked += 1;
        }
    }
    assert_eq!(checked, 26 + 16);
}
