//! Layer operations and the eyedropper. Each is one undo step for the app: call
//! `History::record` with the document before calling these.

use std::collections::HashSet;

use crate::blend::{BlendMode, blend};
use crate::doc::{Document, Layer, new_uuid};
use crate::tiles::{Tile, TiledLayer};

/// Adds an empty layer above `index` (or at the bottom of an empty document); returns its index.
pub fn add_layer(doc: &mut Document, index: Option<usize>) -> usize {
    let n = (1..).find(|n| !doc.layers.iter().any(|l| l.name == format!("Layer {n}"))).unwrap_or(1);
    let at = index.map_or(doc.layers.len(), |i| (i + 1).min(doc.layers.len()));
    doc.layers.insert(at, Layer::new(&format!("Layer {n}")));
    at
}

/// Removes a layer; the document always keeps at least one. Returns the index to select.
pub fn delete_layer(doc: &mut Document, index: usize) -> Result<usize, String> {
    if index >= doc.layers.len() {
        return Err("No such layer".into());
    }
    if doc.layers.len() == 1 {
        return Err("A painting needs at least one layer".into());
    }
    doc.layers.remove(index);
    Ok(index.min(doc.layers.len() - 1))
}

/// Copies a layer above itself (tiles are shared until either is painted on).
pub fn duplicate_layer(doc: &mut Document, index: usize) -> Result<usize, String> {
    let mut copy = doc.layers.get(index).ok_or("No such layer")?.clone();
    copy.name = format!("{} copy", copy.name);
    copy.uuid = new_uuid();
    doc.layers.insert(index + 1, copy);
    Ok(index + 1)
}

/// Moves a layer up (towards the top) or down one place. Returns its new index.
pub fn move_layer(doc: &mut Document, index: usize, up: bool) -> Result<usize, String> {
    if index >= doc.layers.len() {
        return Err("No such layer".into());
    }
    let to = if up { index + 1 } else { index.wrapping_sub(1) };
    if to >= doc.layers.len() {
        return Ok(index);
    }
    doc.layers.swap(index, to);
    Ok(to)
}

/// Merges a layer into the one below it: both opacities and the upper layer's blend mode
/// are baked in, and the result is a Normal layer at full opacity. The picture stays exactly
/// the same when the upper layer is Normal (or the lower layer is opaque under it). A lower
/// layer with a blend mode other than Normal is refused, as the result could not keep it.
/// Returns the merged layer's index.
pub fn merge_down(doc: &mut Document, index: usize) -> Result<usize, String> {
    if index == 0 || index >= doc.layers.len() {
        return Err("There is no layer below to merge into".into());
    }
    let (upper, lower) = (&doc.layers[index], &doc.layers[index - 1]);
    if !upper.visible || !lower.visible {
        return Err("Show both layers before merging them".into());
    }
    if lower.blend != BlendMode::Normal {
        return Err(format!("Set \"{}\" to the Normal blend mode before merging into it", lower.name));
    }
    let merged = merge_layers(&[lower, upper]);
    let result = Layer { name: lower.name.clone(), visible: true, opacity: 255, locked: lower.locked, blend: BlendMode::Normal, uuid: lower.uuid.clone(), pixels: merged };
    doc.layers[index - 1] = result;
    doc.layers.remove(index);
    Ok(index - 1)
}

/// Composites layers (bottom first; hidden ones skipped) onto transparency, with their
/// opacity and blend modes, into one set of pixels.
pub fn merge_layers(layers: &[&Layer]) -> TiledLayer {
    let layers: Vec<&&Layer> = layers.iter().filter(|l| l.shows()).collect();
    let mut default = [0u8; 4];
    for l in &layers {
        blend(&mut default, l.pixels.default, l.opacity, l.blend);
    }
    let mut merged = TiledLayer::filled(default);
    let keys: HashSet<_> = layers.iter().flat_map(|l| l.pixels.tile_keys()).collect();
    let defaults: Vec<Tile> = layers.iter().map(|l| Tile::filled(l.pixels.default)).collect();
    for k in keys {
        let out = merged.tile_mut(k);
        out.0.fill(0);
        for (l, d) in layers.iter().zip(&defaults) {
            // Like Krita, a layer only acts where it has tiles (or a visible default pixel).
            let src = match l.pixels.tile(k) {
                Some(t) => &**t,
                None if l.pixels.default[3] != 0 => d,
                None => continue,
            };
            let normal = l.blend == BlendMode::Normal;
            for (o, s) in out.0.as_chunks_mut::<4>().0.iter_mut().zip(src.0.as_chunks::<4>().0) {
                if s[3] != 0 || !normal {
                    blend(o, *s, l.opacity, l.blend);
                }
            }
        }
    }
    merged.compact();
    merged
}

/// Clears a layer to transparent.
pub fn clear_layer(doc: &mut Document, index: usize) -> Result<(), String> {
    let l = doc.layers.get_mut(index).ok_or("No such layer")?;
    l.pixels = TiledLayer::new();
    Ok(())
}

/// The colour under the eyedropper: the average of the pixels within `radius` canvas
/// pixels of (x, y), from the visible picture or (with `layer`) from one layer.
/// `None` outside the canvas or where everything is transparent.
pub fn pick_color(doc: &Document, layer: Option<usize>, x: f64, y: f64, radius: f64) -> Option<[u8; 4]> {
    if !(x >= 0.0 && y >= 0.0 && x < doc.width as f64 && y < doc.height as f64) {
        return None;
    }
    let r = radius.clamp(0.0, 200.0);
    let (x0, x1) = ((x - r).floor().max(0.0) as i32, (x + r).floor().min(doc.width as f64 - 1.0) as i32);
    let (y0, y1) = ((y - r).floor().max(0.0) as i32, (y + r).floor().min(doc.height as f64 - 1.0) as i32);
    let mut sum = [0u64; 4];
    let mut n = 0u64;
    for py in y0..=y1 {
        for px in x0..=x1 {
            let (dx, dy) = (px as f64 + 0.5 - x, py as f64 + 0.5 - y);
            if r > 0.0 && dx * dx + dy * dy > r * r && (px, py) != (x as i32, y as i32) {
                continue;
            }
            let p = match layer {
                Some(i) => doc.layers.get(i)?.pixels.pixel(px, py),
                None => {
                    let mut p = [0u8; 4];
                    for l in doc.layers.iter().filter(|l| l.visible) {
                        blend(&mut p, l.pixels.pixel(px, py), l.opacity, l.blend);
                    }
                    p
                }
            };
            let a = p[3] as u64;
            sum[0] += p[0] as u64 * a;
            sum[1] += p[1] as u64 * a;
            sum[2] += p[2] as u64 * a;
            sum[3] += a;
            n += 1;
        }
    }
    if sum[3] == 0 || n == 0 {
        return None;
    }
    let c = |v: u64| ((v + sum[3] / 2) / sum[3]) as u8;
    Some([c(sum[0]), c(sum[1]), c(sum[2]), ((sum[3] + n / 2) / n) as u8])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::over;

    fn doc3() -> Document {
        let mut d = Document::new(128, 64, Some([255, 255, 255, 255]));
        d.layers[1].pixels.set_pixel(5, 5, [255, 0, 0, 255]);
        let mut top = Layer::new("Top");
        top.pixels.set_pixel(5, 5, [0, 0, 255, 128]);
        top.pixels.set_pixel(100, 40, [0, 255, 0, 200]);
        top.opacity = 200;
        d.layers.push(top);
        d
    }

    #[test]
    fn add_delete_duplicate_move() {
        let mut d = doc3();
        let i = add_layer(&mut d, Some(0));
        assert_eq!(i, 1);
        assert_eq!(d.layers[1].name, "Layer 2", "Layer 1 already exists");
        assert_eq!(add_layer(&mut d, None), 4);
        assert_eq!(d.layers.len(), 5);
        assert_eq!(delete_layer(&mut d, 4).unwrap(), 3);
        let dup = duplicate_layer(&mut d, 3).unwrap();
        assert_eq!(d.layers[dup].name, "Top copy");
        assert_ne!(d.layers[dup].uuid, d.layers[3].uuid);
        assert!(std::sync::Arc::ptr_eq(d.layers[dup].pixels.tile((0, 0)).unwrap(), d.layers[3].pixels.tile((0, 0)).unwrap()), "tiles shared");
        assert_eq!(move_layer(&mut d, 0, false).unwrap(), 0, "bottom cannot go lower");
        assert_eq!(move_layer(&mut d, 0, true).unwrap(), 1);
        assert_eq!(d.layers[1].name, "Background");
        let last = d.layers.len() - 1;
        assert_eq!(move_layer(&mut d, last, true).unwrap(), last);
        let mut one = Document::new(10, 10, None);
        assert!(delete_layer(&mut one, 0).is_err());
        assert!(delete_layer(&mut one, 5).is_err());
    }

    #[test]
    fn merge_down_keeps_the_picture() {
        let mut d = doc3();
        d.layers[1].opacity = 150;
        let before = d.flatten();
        let i = merge_down(&mut d, 2).unwrap();
        assert_eq!(i, 1);
        assert_eq!(d.layers.len(), 2);
        assert_eq!(d.layers[1].name, "Layer 1");
        assert_eq!(d.layers[1].opacity, 255);
        let after = d.flatten();
        let worst = before.data.iter().zip(&after.data).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
        assert!(worst <= 1, "merging changed the picture by {worst}");
        // Merging into the opaque background.
        merge_down(&mut d, 1).unwrap();
        assert_eq!(d.layers.len(), 1);
        let flat = d.flatten();
        let worst = before.data.iter().zip(&flat.data).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
        assert!(worst <= 2, "{worst}");
        assert!(merge_down(&mut d, 0).is_err());
        let mut hidden = doc3();
        hidden.layers[2].visible = false;
        assert!(merge_down(&mut hidden, 2).is_err());
    }

    #[test]
    fn merge_down_with_a_blend_mode() {
        let mut d = doc3();
        d.layers[2].blend = BlendMode::Multiply;
        d.layers[2].pixels.set_pixel(50, 50, [128, 128, 128, 255]);
        d.layers[1].pixels.set_pixel(50, 50, [200, 100, 0, 255]);
        let before = d.flatten();
        merge_down(&mut d, 2).unwrap();
        assert_eq!(d.layers[1].blend, BlendMode::Normal);
        assert_eq!(d.flatten().pixel(50, 50), before.pixel(50, 50), "opaque lower layer: unchanged");
        let mut e = doc3();
        e.layers[1].blend = BlendMode::Screen;
        assert!(merge_down(&mut e, 2).unwrap_err().contains("Normal"));
    }

    #[test]
    fn eyedropper() {
        let d = doc3();
        assert_eq!(pick_color(&d, Some(1), 5.5, 5.5, 0.0), Some([255, 0, 0, 255]));
        assert_eq!(pick_color(&d, Some(1), 50.5, 5.5, 0.0), None, "transparent");
        assert_eq!(pick_color(&d, None, 50.5, 5.5, 0.0), Some([255, 255, 255, 255]));
        let merged = pick_color(&d, None, 5.5, 5.5, 0.0).unwrap();
        let mut expected = [255, 255, 255, 255];
        over(&mut expected, [255, 0, 0, 255], 255);
        over(&mut expected, [0, 0, 255, 128], 200);
        assert_eq!(merged, expected);
        assert_eq!(pick_color(&d, None, -1.0, 5.0, 0.0), None);
        assert_eq!(pick_color(&d, None, 128.0, 5.0, 0.0), None);
        assert_eq!(pick_color(&d, None, f64::NAN, 5.0, 0.0), None);
        // Averaging: half red, half white around an edge, alpha-weighted.
        let mut e = Document::new(10, 10, None);
        for y in 0..10 {
            for x in 0..5 {
                e.layers[0].pixels.set_pixel(x, y, [255, 0, 0, 255]);
            }
        }
        let avg = pick_color(&e, Some(0), 5.0, 5.0, 3.0).unwrap();
        assert_eq!(&avg[..3], &[255, 0, 0], "transparent pixels do not dilute the colour");
        assert!((100..160).contains(&avg[3]), "{avg:?}");
    }

    #[test]
    fn clearing() {
        let mut d = doc3();
        clear_layer(&mut d, 2).unwrap();
        assert_eq!(d.layers[2].pixels.tile_count(), 0);
        assert!(clear_layer(&mut d, 9).is_err());
    }
}
