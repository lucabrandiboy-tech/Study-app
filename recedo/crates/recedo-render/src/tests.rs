//! Rendering tests on whatever GPU (or software rasteriser) is available. As in the tablet
//! test app, a missing GPU skips them unless `RECEDO_REQUIRE_GPU=1`.

use std::collections::HashSet;

use recedo_core::doc::Document;
use recedo_core::projection::Projection;
use recedo_core::view::View;

use super::*;

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

fn gpu() -> Option<Gpu> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor { backends: wgpu::Backends::all(), ..wgpu::InstanceDescriptor::new_without_display_handle() });
    let found = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).map_err(|e| e.to_string()).and_then(|adapter| {
        let desc = wgpu::DeviceDescriptor { required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()), ..Default::default() };
        pollster::block_on(adapter.request_device(&desc)).map_err(|e| e.to_string())
    });
    match found {
        Ok((device, queue)) => Some(Gpu { device, queue }),
        Err(e) if std::env::var("RECEDO_REQUIRE_GPU").as_deref() == Ok("1") => panic!("no GPU for rendering tests: {e}"),
        Err(e) => {
            eprintln!("SKIPPED GPU test (set RECEDO_REQUIRE_GPU=1 to make this a failure): {e}");
            None
        }
    }
}

/// Renders `display` with `view` into a w x h Rgba8Unorm image and reads it back.
fn render(g: &Gpu, display: &mut CanvasDisplay, view: &View, w: u32, h: u32) -> Vec<u8> {
    let target = g.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let tv = target.create_view(&Default::default());
    let mut enc = g.device.create_command_encoder(&Default::default());
    display.draw(&g.queue, &mut enc, &tv, view, [0.2, 0.2, 0.2]);
    let padded = (w * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buf = g.device.create_buffer(&wgpu::BufferDescriptor { label: None, size: (padded * h) as u64, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
    enc.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(padded), rows_per_image: Some(h) } },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
    g.queue.submit([enc.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buf.map_async(wgpu::MapMode::Read, .., move |r| {
        let _ = tx.send(r);
    });
    g.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    rx.recv().unwrap().unwrap();
    let data = buf.get_mapped_range(..).unwrap();
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h as usize {
        out.extend_from_slice(&data[y * padded as usize..][..(w * 4) as usize]);
    }
    out
}

fn px(img: &[u8], w: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * w + x) * 4) as usize;
    [img[i], img[i + 1], img[i + 2], img[i + 3]]
}

fn near(a: [u8; 4], b: [u8; 3], tol: u8) -> bool {
    (0..3).all(|c| a[c].abs_diff(b[c]) <= tol)
}

/// A 200 x 120 painting: four opaque colour blocks, a half-transparent strip and a
/// transparent hole.
fn painting() -> Document {
    let mut doc = Document::new(200, 120, None);
    let l = &mut doc.layers[0].pixels;
    for y in 0..120 {
        for x in 0..200 {
            let c = match (x < 100, y < 60) {
                (true, true) => [255, 0, 0, 255],
                (false, true) => [0, 255, 0, 255],
                (true, false) => [0, 0, 255, 255],
                (false, false) => [255, 255, 0, 255],
            };
            l.set_pixel(x, y, c);
        }
    }
    for y in 0..120 {
        for x in 150..170 {
            l.set_pixel(x, y, [255, 255, 255, 128]);
        }
        for x in 180..190 {
            l.set_pixel(x, y, [0, 0, 0, 0]);
        }
    }
    doc
}

#[test]
fn draws_at_zoom_1_with_offset() {
    let Some(g) = gpu() else { return };
    let doc = painting();
    let proj = Projection::new(&doc);
    let mut d = CanvasDisplay::new(&g.device, doc.width, doc.height, wgpu::TextureFormat::Rgba8Unorm);
    d.upload_all(&g.queue, &proj);
    let view = View { offset: (30.0, 20.0), ..Default::default() };
    let img = render(&g, &mut d, &view, 300, 200);
    assert!(near(px(&img, 300, 30 + 10, 20 + 10), [255, 0, 0], 0));
    assert!(near(px(&img, 300, 30 + 120, 20 + 10), [0, 255, 0], 0));
    assert!(near(px(&img, 300, 30 + 10, 20 + 100), [0, 0, 255], 0));
    assert!(near(px(&img, 300, 30 + 120, 20 + 100), [255, 255, 0], 0));
    // Exact pixel edges: canvas column 99 red, 100 green.
    assert!(near(px(&img, 300, 30 + 99, 25), [255, 0, 0], 0));
    assert!(near(px(&img, 300, 30 + 100, 25), [0, 255, 0], 0));
    // Background outside the canvas (0.2 -> 51).
    assert!(near(px(&img, 300, 10, 10), [51, 51, 51], 1));
    assert!(near(px(&img, 300, 30 + 200, 25), [51, 51, 51], 1));
    // Transparent hole shows the checkerboard (0.6 or 0.75 grey).
    let hole = px(&img, 300, 30 + 185, 20 + 10);
    assert!(near(hole, [153, 153, 153], 1) || near(hole, [191, 191, 191], 1), "{hole:?}");
    // Half-transparent white over the checker: 128 + grey * 127 / 255.
    let strip = px(&img, 300, 30 + 160, 20 + 10);
    assert!(near(strip, [204, 204, 204], 2) || near(strip, [223, 223, 223], 2), "{strip:?}");
}

#[test]
fn rotation_mirror_and_zoom_follow_the_view_math() {
    let Some(g) = gpu() else { return };
    let doc = painting();
    let proj = Projection::new(&doc);
    let mut d = CanvasDisplay::new(&g.device, doc.width, doc.height, wgpu::TextureFormat::Rgba8Unorm);
    d.upload_all(&g.queue, &proj);
    let mut view = View::default();
    view.fit((200, 120), (400.0, 400.0), 10.0);
    view.rotate_by(0.7, (200.0, 200.0));
    view.toggle_mirror((200.0, 200.0));
    view.zoom_by(1.3, (150.0, 220.0));
    let img = render(&g, &mut d, &view, 400, 400);
    // Every block centre lands where View::to_screen says, with its colour.
    for (cx, cy, colour) in [(40.0, 30.0, [255, 0, 0]), (130.0, 30.0, [0, 255, 0]), (40.0, 90.0, [0, 0, 255]), (130.0, 90.0, [255, 255, 0])] {
        let (sx, sy) = view.to_screen(cx, cy);
        let got = px(&img, 400, sx as u32, sy as u32);
        assert!(near(got, colour, 2), "block at canvas ({cx}, {cy}) -> screen ({sx:.0}, {sy:.0}) is {got:?}");
    }
}

#[test]
fn zoomed_out_uses_mipmaps_and_zoomed_in_shows_sharp_pixels() {
    let Some(g) = gpu() else { return };
    // Fine 1-pixel black/white stripes: zoomed out to 25 % they must average to grey, not alias.
    let mut doc = Document::new(256, 256, None);
    for y in 0..256 {
        for x in 0..256 {
            let v = if x % 2 == 0 { 0 } else { 255 };
            doc.layers[0].pixels.set_pixel(x, y, [v, v, v, 255]);
        }
    }
    let proj = Projection::new(&doc);
    let mut d = CanvasDisplay::new(&g.device, 256, 256, wgpu::TextureFormat::Rgba8Unorm);
    d.upload_all(&g.queue, &proj);
    let small = View { zoom: 0.25, ..Default::default() };
    let img = render(&g, &mut d, &small, 64, 64);
    for (x, y) in [(10, 10), (31, 40), (50, 7)] {
        let p = px(&img, 64, x, y);
        assert!(near(p, [128, 128, 128], 6), "zoomed-out stripes should be grey, got {p:?} at {x},{y}");
    }
    // At 400 % each canvas pixel is a sharp 4x4 square.
    let big = View { zoom: 4.0, ..Default::default() };
    let img = render(&g, &mut d, &big, 64, 64);
    for x in 0..64u32 {
        // Screen column x shows canvas column x / 4: even columns black, odd white.
        let want = if (x / 4) % 2 == 0 { [0, 0, 0] } else { [255, 255, 255] };
        assert!(near(px(&img, 64, x, 5), want, 0), "screen x {x}");
    }
}

#[test]
fn tile_updates_show_up() {
    let Some(g) = gpu() else { return };
    let mut doc = painting();
    let mut proj = Projection::new(&doc);
    let mut d = CanvasDisplay::new(&g.device, doc.width, doc.height, wgpu::TextureFormat::Rgba8Unorm);
    d.upload_all(&g.queue, &proj);
    let view = View::default();
    let before = render(&g, &mut d, &view, 200, 120);
    assert!(near(px(&before, 200, 70, 70), [0, 0, 255], 0));
    for y in 64..128 {
        for x in 64..128 {
            doc.layers[0].pixels.set_pixel(x, y, [10, 20, 30, 255]);
        }
    }
    let dirty: HashSet<_> = [(1, 1)].into_iter().collect();
    let up = proj.update(&doc, &dirty);
    d.upload(&g.queue, &proj, &up);
    let after = render(&g, &mut d, &view, 200, 120);
    assert!(near(px(&after, 200, 70, 70), [10, 20, 30], 0));
    assert!(near(px(&after, 200, 50, 70), [0, 0, 255], 0), "untouched tile unchanged");
}

#[test]
fn srgb_targets_get_the_same_look() {
    let Some(g) = gpu() else { return };
    let doc = painting();
    let proj = Projection::new(&doc);
    let mut d = CanvasDisplay::new(&g.device, doc.width, doc.height, wgpu::TextureFormat::Rgba8UnormSrgb);
    d.upload_all(&g.queue, &proj);
    // Render into an sRGB texture, read its raw bytes: they must equal the canvas values.
    let target = g.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d { width: 200, height: 120, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let tv = target.create_view(&Default::default());
    let mut enc = g.device.create_command_encoder(&Default::default());
    d.draw(&g.queue, &mut enc, &tv, &View::default(), [0.2, 0.2, 0.2]);
    let buf = g.device.create_buffer(&wgpu::BufferDescriptor { label: None, size: 256 * 4 * 120, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
    enc.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(256 * 4), rows_per_image: Some(120) } },
        wgpu::Extent3d { width: 200, height: 120, depth_or_array_layers: 1 },
    );
    g.queue.submit([enc.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buf.map_async(wgpu::MapMode::Read, .., move |r| {
        let _ = tx.send(r);
    });
    g.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    rx.recv().unwrap().unwrap();
    let data = buf.get_mapped_range(..).unwrap();
    let at = |x: usize, y: usize| [data[y * 1024 + x * 4], data[y * 1024 + x * 4 + 1], data[y * 1024 + x * 4 + 2], 255];
    assert!(near(at(160, 10), [204, 204, 204], 3) || near(at(160, 10), [223, 223, 223], 3), "{:?}", at(160, 10));
    assert!(near(at(10, 10), [255, 0, 0], 1));
}

/// Reads one mip level of a chunk's texture back (premultiplied RGBA, tightly packed).
fn read_level(g: &Gpu, texture: &wgpu::Texture, level: u32) -> (u32, u32, Vec<u8>) {
    let (w, h) = ((texture.width() >> level).max(1), (texture.height() >> level).max(1));
    let padded = (w * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buf = g.device.create_buffer(&wgpu::BufferDescriptor { label: None, size: (padded * h) as u64, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
    let mut enc = g.device.create_command_encoder(&Default::default());
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo { texture, mip_level: level, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(padded), rows_per_image: Some(h) } },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
    g.queue.submit([enc.finish()]);
    buf.map_async(wgpu::MapMode::Read, .., |_| {});
    g.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let data = buf.get_mapped_range(..).unwrap();
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h as usize {
        out.extend_from_slice(&data[y * padded as usize..][..(w * 4) as usize]);
    }
    (w, h, out)
}

/// Every mip level holds exactly the picture, halved level by level: after a full upload
/// and after a partial one. (On some browsers' WebGL the old GPU-drawn mipmaps wrote a
/// half-size copy of the picture into level 0 instead.)
#[test]
fn every_mip_level_is_the_picture_halved() {
    let Some(g) = gpu() else { return };
    let mut doc = Document::new(300, 200, None);
    let mut seed = 99u32;
    for y in 0..200 {
        for x in 0..300 {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            doc.layers[0].pixels.set_pixel(x, y, [seed as u8, (seed >> 8) as u8, (x as u8) ^ (y as u8), 40 + (seed >> 26) as u8 * 3]);
        }
    }
    let mut proj = Projection::new(&doc);
    let mut d = CanvasDisplay::new(&g.device, doc.width, doc.height, wgpu::TextureFormat::Rgba8Unorm);
    d.upload_all(&g.queue, &proj);
    let check = |d: &CanvasDisplay, proj: &Projection, what: &str| {
        let tex = &d.chunks[0].texture;
        let mut want: Vec<u8> = proj.image.data.as_chunks::<4>().0.iter().flat_map(|p| premultiply(*p)).collect();
        let (mut ww, mut wh) = (tex.width(), tex.height());
        for level in 0..tex.mip_level_count() {
            let (w, h, got) = read_level(&g, tex, level);
            assert_eq!((w, h), (ww, wh));
            assert!(got == want, "{what}: mip level {level} differs");
            // The next level: each pixel the rounded average of its 2x2 pixels.
            let (nw, nh) = ((ww / 2).max(1), (wh / 2).max(1));
            let at = |x: u32, y: u32, c: usize| want[(((y.min(wh - 1)) * ww + x.min(ww - 1)) * 4) as usize + c] as u32;
            let mut next = Vec::with_capacity((nw * nh * 4) as usize);
            for y in 0..nh {
                for x in 0..nw {
                    for c in 0..4 {
                        next.push(((at(2 * x, 2 * y, c) + at(2 * x + 1, 2 * y, c) + at(2 * x, 2 * y + 1, c) + at(2 * x + 1, 2 * y + 1, c) + 2) / 4) as u8);
                    }
                }
            }
            (want, ww, wh) = (next, nw, nh);
        }
    };
    check(&d, &proj, "after the first upload");
    for y in 130..200 {
        for x in 0..90 {
            doc.layers[0].pixels.set_pixel(x, y, [250, 10, 10, 255]);
        }
    }
    let dirty: HashSet<_> = [(0, 2), (1, 2), (0, 3), (1, 3)].into_iter().collect();
    let up = proj.update(&doc, &dirty);
    d.upload(&g.queue, &proj, &up);
    check(&d, &proj, "after a partial update");
    // Scattered tiles: gaps within a row of tiles, and the clipped last column and row.
    let spots = [(0, 0), (2, 0), (4, 0), (4, 1), (1, 3), (3, 3), (4, 3)];
    for &(tx, ty) in &spots {
        for y in (ty * 64)..((ty * 64 + 64).min(200)) {
            for x in (tx * 64)..((tx * 64 + 64).min(300)) {
                doc.layers[0].pixels.set_pixel(x, y, [(x * 3) as u8, (y * 5) as u8, 90, (x + y) as u8 | 1]);
            }
        }
    }
    let dirty: HashSet<_> = spots.into_iter().collect();
    let up = proj.update(&doc, &dirty);
    d.upload(&g.queue, &proj, &up);
    check(&d, &proj, "after scattered updates");
}
