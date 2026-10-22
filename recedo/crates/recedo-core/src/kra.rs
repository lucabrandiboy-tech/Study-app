//! Krita `.kra` files.
//!
//! Read: canvas size and resolution, and 8-bit RGBA paint layers (also those inside
//! groups, which are flattened into the list) with name, visibility, opacity, lock and
//! position. Everything else (vector, filter, fill, file and clone layers, masks, blend modes
//! other than Normal, other colour depths) is left out and listed in `warnings`. If no layer
//! can be read at all, the flattened picture Krita stores in the file (`mergedimage.png`) is
//! opened as a single layer instead.
//!
//! Written: the same, in the layout Krita 5.2 writes, plus `mergedimage.png` and
//! `preview.png` so file browsers and other apps can show the picture.
//!
//! A `.kra` is a zip: `mimetype` (stored first, uncompressed), `maindoc.xml` (the layer
//! tree), `<image>/layers/<file>` (each layer's tiles) and `<file>.defaultpixel`. A tile
//! file is a text header (`VERSION 2`, `TILEWIDTH 64`, `TILEHEIGHT 64`, `PIXELSIZE 4`,
//! `DATA <count>`) followed by, per tile, a line `x,y,LZF,<size>` and `size` bytes: a flag
//! byte (1 = LZF-compressed with the colour channels stored one after the other, 0 = raw)
//! and the B, G, R, A pixels.

use std::fs::File;
use std::io::{BufWriter, Cursor, Read, Seek, Write};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use quick_xml::escape::escape;
use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::blend::BlendMode;
use crate::doc::{Document, Layer, MAX_SIDE, new_uuid};
use crate::image;
use crate::lzf;
use crate::tiles::{TILE, TILE_BYTES, TILE_PIXELS, Tile, TiledLayer};

pub const MIMETYPE: &str = "application/x-krita";
/// Name of the image (and its folder inside the zip) in files Recedo writes.
const IMAGE_NAME: &str = "Unnamed";
/// The profile Krita gives 8-bit sRGB images; Krita ships it, so the file need not.
const SRGB_PROFILE: &str = "sRGB-elle-V2-srgbtrc.icc";
/// Farthest a layer or tile may sit from the canvas origin, in pixels (keeps all the
/// position arithmetic far from overflowing).
const MAX_POSITION: i32 = 1 << 24;
/// Biggest single file accepted from inside a `.kra` (a 30000 x 30000 layer is 3.6 GB raw,
/// but compressed tiles of real paintings are far smaller).
const MAX_ENTRY: u64 = 4 << 30;

pub struct Loaded {
    pub doc: Document,
    /// Things in the file that Recedo could not open, in plain words.
    pub warnings: Vec<String>,
}

pub fn load_file(path: &Path) -> Result<Loaded, String> {
    let f = File::open(path).map_err(|e| format!("Could not open {}: {e}", path.display()))?;
    load_from(std::io::BufReader::new(f))
}

pub fn load(bytes: &[u8]) -> Result<Loaded, String> {
    load_from(Cursor::new(bytes))
}

pub fn load_from<R: Read + Seek>(r: R) -> Result<Loaded, String> {
    let mut zip = ZipArchive::new(r).map_err(|e| format!("Not a Krita file (not a zip archive: {e})"))?;
    let mut warnings = Vec::new();
    if let Some(m) = entry(&mut zip, "mimetype", 1024)? {
        let m = String::from_utf8_lossy(&m);
        if m.trim() != MIMETYPE {
            warnings.push(format!("The file says it is \"{}\", not a Krita document; opening it anyway.", m.trim()));
        }
    }
    let xml = entry(&mut zip, "maindoc.xml", 64 << 20)?.ok_or("Not a Krita file (maindoc.xml is missing)")?;
    let xml = String::from_utf8(xml).map_err(|_| "maindoc.xml is not valid UTF-8")?;
    let root = parse_xml(&xml)?;
    let img = root.child("IMAGE").ok_or("maindoc.xml has no IMAGE element")?;
    let width: u32 = img.attr("width").and_then(|v| v.trim().parse().ok()).ok_or("The image width is missing")?;
    let height: u32 = img.attr("height").and_then(|v| v.trim().parse().ok()).ok_or("The image height is missing")?;
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
        return Err(format!("Canvas size {width} x {height} is not supported (1 to {MAX_SIDE} pixels a side)"));
    }
    let dpi = img.attr("x-res").and_then(|v| v.trim().parse::<f64>().ok()).filter(|d| d.is_finite() && *d > 0.0).unwrap_or(72.0);
    if let Some(p) = img.attr("profile")
        && !p.to_ascii_lowercase().contains("srgb")
    {
        warnings.push(format!("The image's colour profile \"{p}\" is treated as sRGB."));
    }
    let image_name = img.attr("name").unwrap_or(IMAGE_NAME).to_string();
    let names: Vec<String> = zip.file_names().map(String::from).collect();

    // At most 4 canvases' worth of tiles per layer (plus some), so a small hostile file
    // cannot make Recedo allocate gigabytes.
    let max_tiles = 4 * (width.div_ceil(TILE as u32) as usize * height.div_ceil(TILE as u32) as usize) + 1024;
    let mut ctx = Ctx { zip: &mut zip, names: &names, image_name: &image_name, warnings: &mut warnings, max_tiles };
    let mut pending = Vec::new();
    let tree = match img.child("layers") {
        Some(layers) => read_layers(&mut ctx, layers, &mut pending),
        None => Vec::new(),
    };
    // Decoding the tiles is the slow part: spread the layers over the CPU cores.
    let mut decoded: Vec<Option<Layer>> = Vec::with_capacity(pending.len());
    for (p, result) in pending.iter().zip(decode_parallel(&pending)) {
        match result {
            Ok(pixels) => decoded.push(Some(Layer { pixels, ..p.layer.clone() })),
            Err(e) => {
                warnings.push(format!("Layer \"{}\" could not be opened: {e}", p.layer.name));
                decoded.push(None);
            }
        }
    }
    let mut layers = resolve(tree, &mut decoded, &mut warnings);
    layers.reverse();

    if layers.is_empty() {
        if let Some(png) = entry(&mut zip, "mergedimage.png", MAX_ENTRY)? {
            let merged = image::decode_png(&png)?;
            let mut l = Layer::new("Flattened picture");
            l.pixels.write_rect(0, 0, merged.width, merged.height, &merged.data);
            layers.push(l);
            warnings.push("No layer could be opened, so the picture was opened flattened, as one layer.".into());
        } else {
            let mut e = String::from("The file has no layers Recedo can open.");
            for w in &warnings {
                e.push_str("\n- ");
                e.push_str(w);
            }
            return Err(e);
        }
    }
    Ok(Loaded { doc: Document { width, height, dpi, layers }, warnings })
}

/// A paint layer whose tiles are not decoded yet.
struct Pending {
    /// Everything but the pixels (which hold only the default pixel).
    layer: Layer,
    data: Vec<u8>,
    offset: (i32, i32),
    max_tiles: usize,
}

/// The layer tree from maindoc.xml (top layer first), before the pixels are decoded.
enum Entry {
    /// Index into the pending paint layers.
    Paint(usize),
    Group {
        /// The group's own name, visibility, opacity and blend mode (no pixels).
        meta: Layer,
        passthrough: bool,
        children: Vec<Entry>,
    },
}

/// Turns the tree into a flat layer list (top first). A group whose layers can simply be
/// listed without changing the picture is listed (Normal at full opacity with only Normal
/// layers inside, or pass-through at full opacity); any other group is merged into one
/// layer exactly as Krita composites it (its layers together first, then the group with its
/// opacity and blend mode), since Recedo has no groups yet.
fn resolve(tree: Vec<Entry>, decoded: &mut [Option<Layer>], warnings: &mut Vec<String>) -> Vec<Layer> {
    let mut out = Vec::new();
    for e in tree {
        match e {
            Entry::Paint(i) => out.extend(decoded[i].take()),
            Entry::Group { meta, passthrough, children } => {
                let mut kids = resolve(children, decoded, warnings);
                let listable = meta.opacity == 255 && (passthrough || (meta.blend == BlendMode::Normal && kids.iter().all(|k| k.blend == BlendMode::Normal)));
                if listable {
                    push_warning(warnings, "Layer groups were opened as plain layers (the groups themselves are not kept).".into());
                    for k in &mut kids {
                        k.visible &= meta.visible;
                    }
                    out.extend(kids);
                } else {
                    push_warning(warnings, format!("Group \"{}\" was merged into one layer to keep its look (Recedo has no layer groups yet).", meta.name));
                    let bottom_first: Vec<&Layer> = kids.iter().rev().collect();
                    out.push(Layer { pixels: crate::ops::merge_layers(&bottom_first), ..meta });
                }
            }
        }
    }
    out
}

fn push_warning(warnings: &mut Vec<String>, w: String) {
    if !warnings.contains(&w) {
        warnings.push(w);
    }
}

fn decode_parallel(pending: &[Pending]) -> Vec<Result<TiledLayer, String>> {
    let next = AtomicUsize::new(0);
    let done: Mutex<Vec<Option<Result<TiledLayer, String>>>> = Mutex::new((0..pending.len()).map(|_| None).collect());
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get()).clamp(1, 16).min(pending.len());
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(p) = pending.get(i) else { break };
                    let mut pixels = TiledLayer::filled(p.layer.pixels.default);
                    let r = read_tiles_limited(&p.data, &mut pixels, p.offset.0, p.offset.1, p.max_tiles).map(|()| pixels);
                    done.lock().unwrap_or_else(|e| e.into_inner())[i] = Some(r);
                }
            });
        }
    });
    done.into_inner()
        .unwrap_or_else(|e| e.into_inner())
        .into_iter()
        .map(|r| r.unwrap_or_else(|| Err("could not be decoded".to_string())))
        .collect()
}

struct Ctx<'a, R: Read + Seek> {
    zip: &'a mut ZipArchive<R>,
    names: &'a [String],
    image_name: &'a str,
    warnings: &'a mut Vec<String>,
    max_tiles: usize,
}

impl<R: Read + Seek> Ctx<'_, R> {
    fn warn(&mut self, w: String) {
        push_warning(self.warnings, w);
    }

    /// The zip entry for a layer file: `<image>/layers/<file>`, or any `*/layers/<file>`.
    fn layer_entry(&mut self, file: &str, suffix: &str, limit: u64) -> Result<Option<Vec<u8>>, String> {
        let want = format!("{}/layers/{file}{suffix}", self.image_name);
        let tail = format!("/layers/{file}{suffix}");
        let name = if self.names.contains(&want) { Some(want) } else { self.names.iter().find(|n| n.ends_with(&tail)).cloned() };
        match name {
            Some(n) => entry(self.zip, &n, limit),
            None => Ok(None),
        }
    }
}

fn kind_name(nodetype: &str) -> &str {
    match nodetype {
        "shapelayer" => "vector layer",
        "adjustmentlayer" => "filter layer",
        "generatorlayer" => "fill layer",
        "filelayer" => "file layer",
        "clonelayer" => "clone layer",
        other => other,
    }
}

/// Reads `<layers>` (top layer first): paint layers go to `pending` for decoding.
fn read_layers<R: Read + Seek>(ctx: &mut Ctx<'_, R>, node: &Node, pending: &mut Vec<Pending>) -> Vec<Entry> {
    let mut out = Vec::new();
    for el in node.children.iter().filter(|c| c.name == "layer") {
        let name = el.attr("name").unwrap_or("Layer").to_string();
        let nodetype = el.attr("nodetype").unwrap_or("");
        let visible = el.attr("visible").is_none_or(|v| v.trim() != "0");
        let opacity = el.attr("opacity").and_then(|v| v.trim().parse::<f64>().ok()).filter(|o| o.is_finite()).unwrap_or(255.0).clamp(0.0, 255.0).round() as u8;
        if el.child("masks").is_some_and(|m| !m.children.is_empty()) {
            ctx.warn(format!("Masks on layer \"{name}\" were left out."));
        }
        let op = el.attr("compositeop").unwrap_or("normal");
        let mode = BlendMode::from_krita_id(op);
        if mode.is_none() {
            ctx.warn(format!("Layer \"{name}\" uses the blend mode \"{op}\", which Recedo does not have yet; it is shown as Normal."));
        }
        let meta = Layer {
            name: name.clone(),
            visible,
            opacity,
            locked: el.attr("locked").is_some_and(|v| v.trim() == "1"),
            blend: mode.unwrap_or_default(),
            uuid: el.attr("uuid").map_or_else(new_uuid, String::from),
            pixels: TiledLayer::new(),
        };
        match nodetype {
            "paintlayer" => match read_paint_layer(ctx, el) {
                Ok((default, data, offset)) => {
                    out.push(Entry::Paint(pending.len()));
                    pending.push(Pending { layer: Layer { pixels: TiledLayer::filled(default), ..meta }, data, offset, max_tiles: ctx.max_tiles });
                }
                Err(e) => ctx.warn(format!("Layer \"{name}\" could not be opened: {e}")),
            },
            "grouplayer" => {
                let passthrough = el.attr("passthrough").is_some_and(|v| matches!(v.trim(), "1" | "true"));
                let children = el.child("layers").map_or_else(Vec::new, |inner| read_layers(ctx, inner, pending));
                out.push(Entry::Group { meta, passthrough, children });
            }
            other => ctx.warn(format!("Layer \"{name}\" was left out: Recedo cannot open {}s yet.", kind_name(other))),
        }
    }
    out
}

/// The layer's default pixel, tile data and position.
type PaintLayerData = ([u8; 4], Vec<u8>, (i32, i32));

fn read_paint_layer<R: Read + Seek>(ctx: &mut Ctx<'_, R>, el: &Node) -> Result<PaintLayerData, String> {
    let cs = el.attr("colorspacename").unwrap_or("RGBA");
    if cs != "RGBA" {
        return Err(format!("its colour model \"{cs}\" is not supported yet (only 8-bit RGB)"));
    }
    let file = el.attr("filename").ok_or("no pixel data named")?.to_string();
    let ox: i32 = el.attr("x").and_then(|v| v.trim().parse().ok()).unwrap_or(0);
    let oy: i32 = el.attr("y").and_then(|v| v.trim().parse().ok()).unwrap_or(0);
    if ox.abs() > MAX_POSITION || oy.abs() > MAX_POSITION {
        return Err(format!("its position ({ox}, {oy}) is out of range"));
    }
    let mut default = [0; 4];
    if let Some(d) = ctx.layer_entry(&file, ".defaultpixel", 64)?
        && let [b, g, r, a] = d[..]
    {
        default = [r, g, b, a];
    }
    let data = ctx.layer_entry(&file, "", MAX_ENTRY)?.ok_or("its pixel data is missing")?;
    Ok((default, data, (ox, oy)))
}

fn next_line<'a>(data: &'a [u8], pos: &mut usize) -> Result<&'a str, String> {
    let rest = data.get(*pos..).unwrap_or(&[]);
    let n = rest.iter().position(|&b| b == b'\n').ok_or("layer data is cut short")?;
    *pos += n + 1;
    std::str::from_utf8(&rest[..n]).map_err(|_| "layer data header is garbled".to_string())
}

/// Parses a tile file into `layer`, offset by the layer's position.
pub fn read_tiles(data: &[u8], layer: &mut TiledLayer, ox: i32, oy: i32) -> Result<(), String> {
    read_tiles_limited(data, layer, ox, oy, usize::MAX)
}

/// [`read_tiles`], refusing files with more than `max_tiles` tiles.
fn read_tiles_limited(data: &[u8], layer: &mut TiledLayer, ox: i32, oy: i32, max_tiles: usize) -> Result<(), String> {
    if ox.abs() > MAX_POSITION || oy.abs() > MAX_POSITION {
        return Err("layer position out of range".into());
    }
    let mut pos = 0;
    let (mut version, mut tw, mut th, mut ps) = ("", 0, 0, 0);
    let count: usize = loop {
        let l = next_line(data, &mut pos)?;
        let (k, v) = l.split_once(' ').unwrap_or((l, ""));
        match k {
            "VERSION" => version = v,
            "TILEWIDTH" => tw = v.trim().parse().unwrap_or(0),
            "TILEHEIGHT" => th = v.trim().parse().unwrap_or(0),
            "PIXELSIZE" => ps = v.trim().parse().unwrap_or(0),
            "DATA" => break v.trim().parse().map_err(|_| "bad tile count")?,
            _ => {}
        }
    };
    if version.trim() != "2" || tw != TILE || th != TILE || ps != 4 {
        return Err(format!("unsupported layer data (version {version}, tiles {tw}x{th}, {ps} bytes per pixel)"));
    }
    if count > max_tiles {
        return Err(format!("it has {count} tiles, far more than its canvas can hold ({max_tiles})"));
    }
    for _ in 0..count {
        let l = next_line(data, &mut pos)?;
        let mut it = l.split(',');
        let (Some(x), Some(y), Some(method), Some(size), None) = (it.next(), it.next(), it.next(), it.next(), it.next()) else {
            return Err(format!("bad tile header \"{l}\""));
        };
        let x: i32 = x.trim().parse().map_err(|_| format!("bad tile header \"{l}\""))?;
        let y: i32 = y.trim().parse().map_err(|_| format!("bad tile header \"{l}\""))?;
        let size: usize = size.trim().parse().map_err(|_| format!("bad tile header \"{l}\""))?;
        if method != "LZF" {
            return Err(format!("tile compression \"{method}\" is not supported"));
        }
        let payload = data.get(pos..pos.checked_add(size).ok_or("bad tile size")?).ok_or("layer data is cut short")?;
        pos += size;
        let (&flag, body) = payload.split_first().ok_or("empty tile")?;
        let mut tile = Tile::filled([0; 4]);
        match flag {
            1 => {
                let planar = lzf::decompress(body, TILE_BYTES).ok_or("a tile is corrupt")?;
                let (b, g, r, a) = (&planar[..TILE_PIXELS], &planar[TILE_PIXELS..2 * TILE_PIXELS], &planar[2 * TILE_PIXELS..3 * TILE_PIXELS], &planar[3 * TILE_PIXELS..]);
                for (i, px) in tile.0.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                    *px = [r[i], g[i], b[i], a[i]];
                }
            }
            0 => {
                if body.len() != TILE_BYTES {
                    return Err("a raw tile has the wrong size".into());
                }
                for (px, &[b, g, r, a]) in tile.0.as_chunks_mut::<4>().0.iter_mut().zip(body.as_chunks::<4>().0) {
                    *px = [r, g, b, a];
                }
            }
            f => return Err(format!("unknown tile flag {f}")),
        }
        if x.abs() > MAX_POSITION || y.abs() > MAX_POSITION {
            return Err("a tile is out of range".into());
        }
        // Krita rounds a tile's position down to the 64-pixel grid.
        let (px, py) = (x.div_euclid(TILE) * TILE + ox, y.div_euclid(TILE) * TILE + oy);
        if px.rem_euclid(TILE) == 0 && py.rem_euclid(TILE) == 0 {
            layer.put_tile((px.div_euclid(TILE), py.div_euclid(TILE)), Some(Arc::new(tile)));
        } else {
            for ty in 0..TILE {
                for tx in 0..TILE {
                    layer.set_pixel(px + tx, py + ty, tile.get(tx, ty));
                }
            }
        }
    }
    layer.compact();
    Ok(())
}

/// Encodes a layer's tiles in Krita's tile file format.
pub fn write_tiles(layer: &TiledLayer) -> Vec<u8> {
    let mut keys: Vec<_> = layer.tile_keys().filter(|k| !layer.tile(*k).is_some_and(|t| t.is_uniform(layer.default))).collect();
    keys.sort_by_key(|&(c, r)| (r, c));
    let mut out = format!("VERSION 2\nTILEWIDTH {TILE}\nTILEHEIGHT {TILE}\nPIXELSIZE 4\nDATA {}\n", keys.len()).into_bytes();
    let mut planar = vec![0u8; TILE_BYTES];
    for (c, r) in keys {
        let tile = layer.tile((c, r)).expect("key from the layer");
        for (i, &[red, g, b, a]) in tile.0.as_chunks::<4>().0.iter().enumerate() {
            planar[i] = b;
            planar[TILE_PIXELS + i] = g;
            planar[2 * TILE_PIXELS + i] = red;
            planar[3 * TILE_PIXELS + i] = a;
        }
        let (flag, body) = match lzf::compress(&planar, TILE_BYTES - 1) {
            Some(z) => (1u8, z),
            None => (0u8, tile.0.as_chunks::<4>().0.iter().flat_map(|&[red, g, b, a]| [b, g, red, a]).collect()),
        };
        out.extend_from_slice(format!("{},{},LZF,{}\n", c * TILE, r * TILE, body.len() + 1).as_bytes());
        out.push(flag);
        out.extend_from_slice(&body);
    }
    out
}

/// Saves in memory.
pub fn save(doc: &Document) -> Result<Vec<u8>, String> {
    let mut c = Cursor::new(Vec::new());
    save_to(doc, &mut c)?;
    Ok(c.into_inner())
}

/// Saves to `path` safely: written to `<path>.part` first and renamed over the old file only
/// once complete, so a crash or full disk never leaves a half-written painting.
pub fn save_file(doc: &Document, path: &Path) -> Result<(), String> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".part");
    let tmp = std::path::PathBuf::from(tmp);
    let result = (|| {
        let f = File::create(&tmp).map_err(|e| format!("Could not write {}: {e}", tmp.display()))?;
        let mut w = BufWriter::new(f);
        save_to(doc, &mut w)?;
        let f = w.into_inner().map_err(|e| format!("Could not write {}: {}", tmp.display(), e.error()))?;
        f.sync_all().map_err(|e| format!("Could not write {}: {e}", tmp.display()))?;
        drop(f);
        std::fs::rename(&tmp, path).map_err(|e| format!("Could not replace {}: {e}", path.display()))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

pub fn save_to<W: Write + Seek>(doc: &Document, w: W) -> Result<(), String> {
    let err = |e: zip::result::ZipError| format!("Could not write the file: {e}");
    let ioerr = |e: std::io::Error| format!("Could not write the file: {e}");
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut zip = ZipWriter::new(w);
    zip.start_file("mimetype", stored).map_err(err)?;
    zip.write_all(MIMETYPE.as_bytes()).map_err(ioerr)?;

    zip.start_file("maindoc.xml", deflated).map_err(err)?;
    zip.write_all(maindoc_xml(doc).as_bytes()).map_err(ioerr)?;
    zip.start_file("documentinfo.xml", deflated).map_err(err)?;
    zip.write_all(documentinfo_xml().as_bytes()).map_err(ioerr)?;

    let encoded = encode_parallel(doc)?;
    zip.start_file("preview.png", stored).map_err(err)?;
    zip.write_all(&encoded.preview_png).map_err(ioerr)?;

    for (i, (l, data)) in doc.layers.iter().zip(&encoded.tiles).enumerate() {
        let file = layer_file(i);
        zip.start_file(format!("{IMAGE_NAME}/layers/{file}"), stored).map_err(err)?;
        zip.write_all(data).map_err(ioerr)?;
        zip.start_file(format!("{IMAGE_NAME}/layers/{file}.defaultpixel"), stored).map_err(err)?;
        let [r, g, b, a] = l.pixels.default;
        zip.write_all(&[b, g, r, a]).map_err(ioerr)?;
    }

    zip.start_file("mergedimage.png", stored).map_err(err)?;
    zip.write_all(&encoded.merged_png).map_err(ioerr)?;
    zip.finish().map_err(err)?;
    Ok(())
}

struct Encoded {
    merged_png: Vec<u8>,
    preview_png: Vec<u8>,
    /// Each layer's tile file, in layer order.
    tiles: Vec<Vec<u8>>,
}

/// The slow parts of saving, spread over the CPU cores: the flattened picture and its
/// preview, and each layer's compressed tiles.
fn encode_parallel(doc: &Document) -> Result<Encoded, String> {
    let next = AtomicUsize::new(0);
    let done: Mutex<Vec<Option<Vec<u8>>>> = Mutex::new(vec![None; doc.layers.len()]);
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get()).clamp(1, 16);
    let pngs = std::thread::scope(|s| {
        let merged = s.spawn(|| -> Result<(Vec<u8>, Vec<u8>), String> {
            let m = doc.flatten();
            Ok((image::encode_png(&m, true)?, image::encode_png(&image::thumbnail(&m, 256), false)?))
        });
        for _ in 0..workers.min(doc.layers.len()) {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(l) = doc.layers.get(i) else { break };
                    let data = write_tiles(&l.pixels);
                    done.lock().unwrap_or_else(|e| e.into_inner())[i] = Some(data);
                }
            });
        }
        merged.join().map_err(|_| "Saving failed (picture encoder crashed)".to_string())?
    })?;
    let tiles = done.into_inner().unwrap_or_else(|e| e.into_inner());
    let tiles = tiles.into_iter().collect::<Option<Vec<_>>>().ok_or("Saving failed (a layer could not be encoded)")?;
    Ok(Encoded { merged_png: pngs.0, preview_png: pngs.1, tiles })
}

fn layer_file(index: usize) -> String {
    format!("layer{}", index + 1)
}

fn maindoc_xml(doc: &Document) -> String {
    let mut s = String::new();
    s.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    s.push_str("<!DOCTYPE DOC PUBLIC '-//KDE//DTD krita 2.0//EN' 'http://www.calligra.org/DTD/krita-2.0.dtd'>\n");
    s.push_str("<DOC xmlns=\"http://www.calligra.org/DTD/krita\" editor=\"Recedo\" kritaVersion=\"5.2.2\" syntaxVersion=\"2.0\">\n");
    s.push_str(&format!(
        " <IMAGE mime=\"application/x-kra\" name=\"{IMAGE_NAME}\" width=\"{}\" height=\"{}\" colorspacename=\"RGBA\" profile=\"{SRGB_PROFILE}\" x-res=\"{}\" y-res=\"{}\" description=\"\">\n",
        doc.width, doc.height, doc.dpi, doc.dpi
    ));
    s.push_str("  <layers>\n");
    for (i, l) in doc.layers.iter().enumerate().rev() {
        s.push_str(&format!(
            "   <layer nodetype=\"paintlayer\" name=\"{}\" filename=\"{}\" uuid=\"{}\" visible=\"{}\" opacity=\"{}\" locked=\"{}\" compositeop=\"{}\" colorspacename=\"RGBA\" x=\"0\" y=\"0\" collapsed=\"0\" colorlabel=\"0\" channelflags=\"\" channellockflags=\"\" intimeline=\"0\" onionskin=\"0\"/>\n",
            escape(l.name.as_str()),
            layer_file(i),
            escape(l.uuid.as_str()),
            u8::from(l.visible),
            l.opacity,
            u8::from(l.locked),
            l.blend.krita_id(),
        ));
    }
    s.push_str("  </layers>\n");
    s.push_str("  <ProjectionBackgroundColor ColorData=\"AAAAAA==\"/>\n");
    s.push_str(" </IMAGE>\n</DOC>\n");
    s
}

fn documentinfo_xml() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let now = iso_utc(secs);
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE document-info PUBLIC '-//KDE//DTD document-info 1.1//EN' 'http://www.calligra.org/DTD/document-info-1.1.dtd'>\n<document-info xmlns=\"http://www.calligra.org/DTD/document-info\">\n <about>\n  <title></title>\n  <editing-cycles>1</editing-cycles>\n  <date>{now}</date>\n  <creation-date>{now}</creation-date>\n </about>\n <author>\n  <full-name></full-name>\n </author>\n</document-info>\n"
    )
}

/// `YYYY-MM-DDTHH:MM:SS` for seconds since 1970 (UTC).
fn iso_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Civil-from-days (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}", rem / 3600, rem / 60 % 60, rem % 60)
}

/// Reads one zip entry, at most `limit` bytes; `None` if it does not exist.
fn entry<R: Read + Seek>(zip: &mut ZipArchive<R>, name: &str, limit: u64) -> Result<Option<Vec<u8>>, String> {
    match zip.by_name(name) {
        Ok(f) => {
            if f.size() > limit {
                return Err(format!("{name} is too large"));
            }
            // The size in the zip header is only a claim: grow as data really arrives.
            let mut v = Vec::with_capacity(f.size().min(16 << 20) as usize);
            f.take(limit + 1).read_to_end(&mut v).map_err(|e| format!("Could not read {name}: {e}"))?;
            if v.len() as u64 > limit {
                return Err(format!("{name} is too large"));
            }
            Ok(Some(v))
        }
        Err(zip::result::ZipError::FileNotFound) => Ok(None),
        Err(e) => Err(format!("Could not read {name}: {e}")),
    }
}

/// Deepest element nesting accepted in maindoc.xml (Krita files nest a few levels per
/// layer group; this allows groups about 40 deep and keeps the recursive code safe).
const MAX_XML_DEPTH: usize = 128;

/// Just enough XML tree for maindoc.xml.
#[derive(Debug, Default)]
struct Node {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<Node>,
}

impl Node {
    fn attr(&self, k: &str) -> Option<&str> {
        self.attrs.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str())
    }

    fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == name)
    }
}

fn node_from(e: &BytesStart) -> Result<Node, String> {
    let bad = |e: &dyn std::fmt::Display| format!("maindoc.xml is damaged: {e}");
    let mut n = Node { name: e.local_name().into_inner().to_string(), ..Default::default() };
    for a in e.attributes() {
        let a = a.map_err(|e| bad(&e))?;
        let key = a.key.local_name().into_inner().to_string();
        let value = a.normalized_value(XmlVersion::Implicit1_0).map_err(|e| bad(&e))?.into_owned();
        n.attrs.push((key, value));
    }
    Ok(n)
}

fn parse_xml(s: &str) -> Result<Node, String> {
    let mut r = Reader::from_str(s);
    r.config_mut().trim_text(true);
    let mut stack = vec![Node::default()];
    loop {
        match r.read_event().map_err(|e| format!("maindoc.xml is damaged: {e}"))? {
            Event::Start(e) => {
                if stack.len() > MAX_XML_DEPTH {
                    return Err("maindoc.xml is nested too deeply".into());
                }
                stack.push(node_from(&e)?);
            }
            Event::Empty(e) => {
                let n = node_from(&e)?;
                stack.last_mut().expect("root").children.push(n);
            }
            Event::End(_) => {
                let n = stack.pop().expect("root");
                stack.last_mut().ok_or("maindoc.xml is damaged: unbalanced tags")?.children.push(n);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if stack.len() != 1 {
        return Err("maindoc.xml is damaged: unclosed tags".into());
    }
    let doc = stack.pop().expect("root");
    doc.children.into_iter().find(|c| c.name == "DOC").ok_or_else(|| "maindoc.xml has no DOC element".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::Rgba8;

    fn sample_doc() -> Document {
        let mut doc = Document::new(200, 130, Some([255, 255, 255, 255]));
        let l1 = &mut doc.layers[1];
        l1.name = "Ink & <lines> \"1\"".into();
        for x in -70..150 {
            l1.pixels.set_pixel(x, 40, [10, 20, 30, 255]);
            l1.pixels.set_pixel(x, -3, [200, 100, 0, 77]);
        }
        let mut top = Layer::new("Colour");
        top.opacity = 128;
        top.locked = true;
        for y in 60..125 {
            for x in 100..199 {
                top.pixels.set_pixel(x, y, [(x % 256) as u8, (y % 256) as u8, 7, (x * y % 256) as u8]);
            }
        }
        doc.layers.push(top);
        let mut hidden = Layer::new("Hidden");
        hidden.visible = false;
        hidden.pixels = TiledLayer::filled([1, 2, 3, 4]);
        hidden.pixels.set_pixel(5, 5, [9, 9, 9, 9]);
        doc.layers.push(hidden);
        doc.dpi = 300.0;
        doc
    }

    #[test]
    fn roundtrip_keeps_everything() {
        let doc = sample_doc();
        let bytes = save(&doc).unwrap();
        let back = load(&bytes).unwrap();
        assert!(back.warnings.is_empty(), "{:?}", back.warnings);
        let b = back.doc;
        assert_eq!((b.width, b.height, b.dpi), (200, 130, 300.0));
        assert_eq!(b.layers.len(), 4);
        for (a, b) in doc.layers.iter().zip(&b.layers) {
            assert_eq!(a.name, b.name);
            assert_eq!(a.visible, b.visible);
            assert_eq!(a.opacity, b.opacity);
            assert_eq!(a.locked, b.locked);
            assert_eq!(a.uuid, b.uuid);
            assert_eq!(a.pixels.default, b.pixels.default);
            for y in -80..140 {
                for x in -80..210 {
                    assert_eq!(a.pixels.pixel(x, y), b.pixels.pixel(x, y), "layer {} at {x},{y}", a.name);
                }
            }
        }
        assert_eq!(doc.flatten(), b.flatten());
    }

    #[test]
    fn mimetype_comes_first_uncompressed() {
        let bytes = save(&Document::new(10, 10, None)).unwrap();
        assert_eq!(&bytes[..4], b"PK\x03\x04");
        assert_eq!(u16::from_le_bytes([bytes[8], bytes[9]]), 0, "stored");
        let name_len = u16::from_le_bytes([bytes[26], bytes[27]]) as usize;
        let extra_len = u16::from_le_bytes([bytes[28], bytes[29]]) as usize;
        assert_eq!(&bytes[30..30 + name_len], b"mimetype");
        let data = 30 + name_len + extra_len;
        assert_eq!(&bytes[data..data + MIMETYPE.len()], MIMETYPE.as_bytes());
    }

    #[test]
    fn incompressible_tiles_are_stored_raw() {
        let mut l = TiledLayer::new();
        let mut x = 7u32;
        for py in 0..TILE {
            for px in 0..TILE {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                l.set_pixel(px, py, x.to_le_bytes());
            }
        }
        let data = write_tiles(&l);
        let header_end = data.windows(4).position(|w| w == b",LZF").unwrap();
        let line_end = header_end + data[header_end..].iter().position(|&b| b == b'\n').unwrap();
        assert_eq!(data[line_end + 1], 0, "raw flag");
        let mut back = TiledLayer::new();
        read_tiles(&data, &mut back, 0, 0).unwrap();
        assert_eq!(back.read_rect(0, 0, 64, 64), l.read_rect(0, 0, 64, 64));
    }

    #[test]
    fn unaligned_layer_offsets() {
        let mut l = TiledLayer::new();
        l.set_pixel(0, 0, [1, 2, 3, 255]);
        l.set_pixel(63, 63, [4, 5, 6, 255]);
        let data = write_tiles(&l);
        let mut back = TiledLayer::new();
        read_tiles(&data, &mut back, 10, -20).unwrap();
        assert_eq!(back.pixel(10, -20), [1, 2, 3, 255]);
        assert_eq!(back.pixel(73, 43), [4, 5, 6, 255]);
        assert_eq!(back.pixel(0, 0), [0, 0, 0, 0]);
    }

    #[test]
    fn damaged_files_give_errors_not_crashes() {
        assert!(load(b"hello").is_err());
        let good = save(&sample_doc()).unwrap();
        // Truncated at many points: every one is an error or a clean load, never a panic.
        for cut in (0..good.len()).step_by(997) {
            let _ = load(&good[..cut]);
        }
        // Flipped bytes throughout.
        for i in (0..good.len()).step_by(331) {
            let mut bad = good.clone();
            bad[i] ^= 0x5A;
            let _ = load(&bad);
        }
        // Broken tile data.
        let mut l = TiledLayer::new();
        assert!(read_tiles(b"VERSION 2\nTILEWIDTH 64\nTILEHEIGHT 64\nPIXELSIZE 4\nDATA 1\n0,0,LZF,5\n\x01abcd", &mut l, 0, 0).is_err());
        assert!(read_tiles(b"VERSION 2\nTILEWIDTH 64\nTILEHEIGHT 64\nPIXELSIZE 4\nDATA 2\n0,0,LZF,1\n\x01", &mut l, 0, 0).is_err());
        assert!(read_tiles(b"VERSION 1\nTILEWIDTH 64\nTILEHEIGHT 64\nPIXELSIZE 4\nDATA 0\n", &mut l, 0, 0).is_err());
        assert!(read_tiles(b"VERSION 2\nTILEWIDTH 64\nTILEHEIGHT 64\nPIXELSIZE 8\nDATA 0\n", &mut l, 0, 0).is_err());
        assert!(read_tiles(b"VERSION 2\nDATA 1\n", &mut l, 0, 0).is_err());
        assert!(read_tiles(b"VERSION 2\nTILEWIDTH 64\nTILEHEIGHT 64\nPIXELSIZE 4\nDATA 1\n0,0,LZF,99999999999999999999\n", &mut l, 0, 0).is_err());
    }

    fn kra_with(maindoc: &str, files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut c = Cursor::new(Vec::new());
        let mut z = ZipWriter::new(&mut c);
        let o = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        z.start_file("mimetype", o).unwrap();
        z.write_all(MIMETYPE.as_bytes()).unwrap();
        z.start_file("maindoc.xml", o).unwrap();
        z.write_all(maindoc.as_bytes()).unwrap();
        for (n, d) in files {
            z.start_file(*n, o).unwrap();
            z.write_all(d).unwrap();
        }
        z.finish().unwrap();
        c.into_inner()
    }

    #[test]
    fn unsupported_content_is_reported() {
        let mut paint = TiledLayer::new();
        paint.set_pixel(1, 1, [255, 0, 0, 255]);
        let tiles = write_tiles(&paint);
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<DOC xmlns="http://www.calligra.org/DTD/krita"><IMAGE name="Pic" width="64" height="64" x-res="100" profile="Rec2020-elle-V4-g10.icc">
<layers>
 <layer nodetype="shapelayer" name="Lines" filename="layer9"/>
 <layer nodetype="grouplayer" name="G" opacity="128" visible="1" compositeop="multiply">
  <layers>
   <layer nodetype="paintlayer" name="Inside" filename="layer3" opacity="255" compositeop="hue" colorspacename="RGBA">
    <masks><mask nodetype="transparencymask" name="m"/></masks>
   </layer>
  </layers>
 </layer>
 <layer nodetype="paintlayer" name="Deep" filename="layer4" colorspacename="RGBA16"/>
 <layer nodetype="paintlayer" name="Gone" filename="layer5" colorspacename="RGBA"/>
</layers></IMAGE></DOC>"#;
        let bytes = kra_with(xml, &[("Pic/layers/layer3", &tiles)]);
        let l = load(&bytes).unwrap();
        assert_eq!(l.doc.layers.len(), 1);
        let g = &l.doc.layers[0];
        assert_eq!(g.name, "G", "a faded group becomes one layer");
        assert_eq!((g.opacity, g.blend), (128, BlendMode::Multiply));
        assert_eq!(g.pixels.pixel(1, 1), [255, 0, 0, 255]);
        assert_eq!(l.doc.dpi, 100.0);
        let all = l.warnings.join("\n");
        for needle in ["Rec2020", "Lines", "vector layer", "\"G\" was merged", "Masks on layer \"Inside\"", "\"hue\"", "Deep", "RGBA16", "Gone"] {
            assert!(all.contains(needle), "missing {needle:?} in:\n{all}");
        }
    }

    fn group_file(group_attrs: &str) -> Vec<u8> {
        let (mut red, mut blue) = (TiledLayer::new(), TiledLayer::new());
        for y in 0..8 {
            for x in 0..8 {
                red.set_pixel(x, y, [255, 0, 0, 255]);
                blue.set_pixel(x + 4, y, [0, 0, 255, 255]);
            }
        }
        let xml = format!(
            r#"<DOC><IMAGE name="P" width="64" height="64"><layers>
 <layer nodetype="grouplayer" name="G" {group_attrs}><layers>
  <layer nodetype="paintlayer" name="Blue" filename="b" colorspacename="RGBA"/>
  <layer nodetype="paintlayer" name="Red" filename="r" colorspacename="RGBA" visible="1"/>
 </layers></layer>
 <layer nodetype="paintlayer" name="Under" filename="u" colorspacename="RGBA"/>
</layers></IMAGE></DOC>"#
        );
        kra_with(&xml, &[("P/layers/r", &write_tiles(&red)), ("P/layers/b", &write_tiles(&blue)), ("P/layers/u", &write_tiles(&TiledLayer::new()))])
    }

    #[test]
    fn groups_keep_their_look() {
        // A faded group: its layers are composited together first, then faded, as in Krita
        // (checked with Krita 5.2.2: the overlap is [0, 0, 255, 128], not a blend of both).
        let l = load(&group_file(r#"opacity="128" visible="1" compositeop="normal""#)).unwrap();
        let names: Vec<_> = l.doc.layers.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, ["Under", "G"]);
        let flat = l.doc.flatten();
        assert_eq!(flat.pixel(5, 3), [0, 0, 255, 128], "overlap");
        assert_eq!(flat.pixel(1, 3), [255, 0, 0, 128]);
        // A plain group (Normal, full opacity, Normal layers) is opened as its layers.
        let l = load(&group_file(r#"opacity="255" visible="0" compositeop="normal""#)).unwrap();
        let names: Vec<_> = l.doc.layers.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, ["Under", "Red", "Blue"]);
        assert!(l.doc.layers[1..].iter().all(|l| !l.visible), "hidden with their group");
        assert!(l.warnings.iter().any(|w| w.contains("plain layers")));
        // Pass-through at full opacity: listed too.
        let l = load(&group_file(r#"opacity="255" passthrough="1" compositeop="normal""#)).unwrap();
        assert_eq!(l.doc.layers.len(), 3);
    }

    #[test]
    fn hostile_structure_is_refused_cleanly() {
        // Groups nested thousands deep: an error, not a stack overflow.
        let deep = format!(
            "<DOC><IMAGE width=\"8\" height=\"8\"><layers>{}{}</layers></IMAGE></DOC>",
            "<layer nodetype=\"grouplayer\" name=\"g\"><layers>".repeat(5000),
            "</layers></layer>".repeat(5000)
        );
        assert!(load(&kra_with(&deep, &[])).err().unwrap().contains("nested too deeply"));
        // More tiles than the canvas could hold: the layer is refused, not loaded.
        let mut many = TiledLayer::new();
        for i in 0..2000 {
            many.set_pixel(i * 64, 0, [1, 1, 1, 255]);
        }
        let xml = r#"<DOC><IMAGE name="P" width="64" height="64"><layers>
 <layer nodetype="paintlayer" name="Many" filename="m" colorspacename="RGBA"/>
 <layer nodetype="paintlayer" name="Far" filename="f" colorspacename="RGBA" x="2147483600" y="0"/>
</layers></IMAGE></DOC>"#;
        let small = write_tiles(&TiledLayer::filled([0; 4]));
        let r = load(&kra_with(xml, &[("P/layers/m", &write_tiles(&many)), ("P/layers/f", &small)]));
        let all = match r {
            Ok(l) => l.warnings.join("\n"),
            Err(e) => e,
        };
        assert!(all.contains("far more than its canvas"), "{all}");
        assert!(all.contains("out of range"), "{all}");
    }

    #[test]
    fn tile_positions_snap_to_the_grid_like_krita() {
        let mut l = TiledLayer::new();
        let data = b"VERSION 2\nTILEWIDTH 64\nTILEHEIGHT 64\nPIXELSIZE 4\nDATA 1\n10,-5,LZF,16385\n\x00";
        let mut v = data.to_vec();
        v.extend(std::iter::repeat_n([9u8, 8, 7, 255], TILE_PIXELS).flatten());
        read_tiles(&v, &mut l, 0, 0).unwrap();
        assert_eq!(l.pixel(0, -64), [7, 8, 9, 255], "10,-5 lands on tile (0, -1)");
        assert_eq!(l.pixel(0, 0), [0, 0, 0, 0]);
    }

    #[test]
    fn falls_back_to_the_flattened_picture() {
        let mut merged = Rgba8::new(3, 2);
        merged.data[0..4].copy_from_slice(&[1, 2, 3, 255]);
        let png = image::encode_png(&merged, true).unwrap();
        let xml = r#"<DOC><IMAGE width="3" height="2" colorspacename="CMYK"><layers><layer nodetype="paintlayer" name="C" filename="layer2" colorspacename="CMYK"/></layers></IMAGE></DOC>"#;
        let l = load(&kra_with(xml, &[("mergedimage.png", &png)])).unwrap();
        assert_eq!(l.doc.layers.len(), 1);
        assert_eq!(l.doc.layers[0].pixels.pixel(0, 0), [1, 2, 3, 255]);
        assert!(l.warnings.iter().any(|w| w.contains("flattened")), "{:?}", l.warnings);
        // Without a flattened picture it is an error.
        assert!(load(&kra_with(xml, &[])).is_err());
        // Absurd sizes are refused before anything is allocated.
        let huge = r#"<DOC><IMAGE width="900000" height="900000"><layers/></IMAGE></DOC>"#;
        assert!(load(&kra_with(huge, &[])).err().unwrap().contains("not supported"));
    }

    #[test]
    fn dates() {
        assert_eq!(iso_utc(0), "1970-01-01T00:00:00");
        assert_eq!(iso_utc(951_782_400), "2000-02-29T00:00:00");
        assert_eq!(iso_utc(1_792_643_045), "2026-10-22T04:24:05");
    }

    #[test]
    fn save_file_replaces_atomically() {
        let dir = std::env::temp_dir().join(format!("recedo-kra-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.kra");
        save_file(&sample_doc(), &path).unwrap();
        save_file(&Document::new(5, 5, None), &path).unwrap();
        assert_eq!(load_file(&path).unwrap().doc.width, 5);
        assert!(!dir.join("a.kra.part").exists());
        // A folder that does not exist fails cleanly.
        assert!(save_file(&sample_doc(), &dir.join("missing").join("b.kra")).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
