//! Recedo's GPU canvas display.
//!
//! The CPU keeps the picture (`recedo_core::projection::Projection`); this uploads the
//! tiles that changed into GPU textures (premultiplied, with mipmaps for zoomed-out
//! views) and draws them with the view's pan, zoom, rotation and mirror, over a
//! checkerboard where the picture is see-through. Canvases larger than the GPU's texture
//! limit are split into several textures.
//!
//! The mipmaps are computed on the CPU, only where tiles changed, and uploaded level by
//! level. (Drawing each level from the one above inside the same texture went wrong on
//! some browsers' WebGL: a half-size copy of the picture appeared in its top-left corner.)

pub mod gpu;

use rayon::prelude::*;
use recedo_core::image::Rgba8;
use recedo_core::projection::Projection;
use recedo_core::tiles::{TILE, TileKey};
use recedo_core::view::View;

pub const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// Zoom at or above which canvas pixels are shown as sharp squares.
pub const NEAREST_FROM_ZOOM: f64 = 2.0;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    inv0: [f32; 4],
    inv1: [f32; 4],
    chunk: [f32; 4],
    params: [f32; 4],
}

struct Chunk {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    texture: wgpu::Texture,
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    /// CPU copies of mip levels 1, 2, … (premultiplied RGBA), to compute the next level from.
    levels: Vec<Level>,
}

struct Level {
    w: u32,
    h: u32,
    data: Vec<u8>,
}

/// Mip blocks are this many pixels square (in their own level's pixels).
const BLOCK: u32 = 64;

pub struct CanvasDisplay {
    pub width: u32,
    pub height: u32,
    chunks: Vec<Chunk>,
    pipeline: wgpu::RenderPipeline,
    srgb_target: bool,
}

fn mip_count(w: u32, h: u32) -> u32 {
    32 - w.max(h).max(1).leading_zeros()
}

impl CanvasDisplay {
    /// A display for a `width` x `height` canvas, drawing into targets of `target_format`.
    pub fn new(device: &wgpu::Device, width: u32, height: u32, target_format: wgpu::TextureFormat) -> CanvasDisplay {
        let (width, height) = (width.max(1), height.max(1));
        let canvas_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("canvas display"),
            source: wgpu::ShaderSource::Wgsl(include_str!("canvas.wgsl").into()),
        });
        let tex_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
            count: None,
        };
        let samp_entry = |binding| wgpu::BindGroupLayoutEntry { binding, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None };
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("canvas display"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                tex_entry(1),
                samp_entry(2),
            ],
        });
        let pipeline_for = |label, module: &wgpu::ShaderModule, entry, layout: &wgpu::BindGroupLayout, format| {
            let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some(label), bind_group_layouts: &[Some(layout)], immediate_size: 0 });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pl),
                vertex: wgpu::VertexState { module, entry_point: Some("vs_full"), compilation_options: Default::default(), buffers: &[] },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let pipeline = pipeline_for("canvas display", &canvas_module, "fs_canvas", &bgl, target_format);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("canvas"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        // Chunk size: the GPU's limit, rounded down to whole tiles.
        let max = device.limits().max_texture_dimension_2d.min(8192);
        let step = (max / TILE as u32).max(1) * TILE as u32;
        let mut chunks = Vec::new();
        for y in (0..height).step_by(step as usize) {
            for x in (0..width).step_by(step as usize) {
                let (w, h) = (step.min(width - x), step.min(height - y));
                let levels = mip_count(w, h);
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("canvas chunk"),
                    size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                    mip_level_count: levels,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: TEXTURE_FORMAT,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                });
                let uniform = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("canvas view"),
                    size: std::mem::size_of::<Uniform>() as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let full_view = texture.create_view(&Default::default());
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("canvas chunk"),
                    layout: &bgl,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: uniform.as_entire_binding() },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&full_view) },
                        wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
                    ],
                });
                let levels = (1..levels)
                    .map(|n| {
                        let (lw, lh) = ((w >> n).max(1), (h >> n).max(1));
                        Level { w: lw, h: lh, data: vec![0; (lw * lh * 4) as usize] }
                    })
                    .collect();
                chunks.push(Chunk { x, y, w, h, texture, uniform, bind_group, levels });
            }
        }
        CanvasDisplay { width, height, chunks, pipeline, srgb_target: target_format.is_srgb() }
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Uploads changed tiles of the projection, and the parts of the mip levels they change.
    /// Neighbouring changed tiles in a row of tiles go up as one rectangle (one GPU copy
    /// instead of many small ones).
    pub fn upload(&mut self, queue: &wgpu::Queue, projection: &Projection, tiles: &[TileKey]) {
        let img = &projection.image;
        let t = TILE as u32;
        for chunk in &mut self.chunks {
            // This chunk's changed tiles: columns per tile row, in the chunk's own tiles.
            let mut rows: std::collections::BTreeMap<u32, Vec<u32>> = Default::default();
            for &key in tiles {
                let Some((x, y, _, _)) = projection.tile_rect(key) else { continue };
                if x >= chunk.x && x < chunk.x + chunk.w && y >= chunk.y && y < chunk.y + chunk.h {
                    rows.entry((y - chunk.y) / t).or_default().push((x - chunk.x) / t);
                }
            }
            if rows.is_empty() {
                continue;
            }
            // Level 0, and the level-1 blocks under it (a 64 px block of level 1 is 2 x 2 tiles).
            let mut rects = Vec::new();
            let mut blocks = std::collections::HashSet::new();
            for (&ty, cols) in &mut rows {
                for (a, b) in runs(cols) {
                    rects.push((a * t, ty * t, ((b + 1) * t).min(chunk.w), ((ty + 1) * t).min(chunk.h)));
                    blocks.extend((a / 2..=b / 2).map(|bx| (bx, ty / 2)));
                }
            }
            let (cx, cy) = (chunk.x, chunk.y);
            let pixels: Vec<Vec<u8>> = rects.par_iter().map(|&(x0, y0, x1, y1)| premultiplied(img, cx + x0, cy + y0, x1 - x0, y1 - y0)).collect();
            for (&rect, data) in rects.iter().zip(&pixels) {
                write_level(queue, &chunk.texture, 0, rect, data);
            }
            // Each mip level: the changed blocks, in runs along each row of blocks.
            for n in 0..chunk.levels.len() {
                if blocks.is_empty() {
                    break;
                }
                let (prev, rest) = chunk.levels.split_at_mut(n);
                let level = &mut rest[0];
                let above = match n {
                    0 => Above::Picture { img, x: chunk.x, y: chunk.y, w: chunk.w, h: chunk.h },
                    _ => Above::Level(&prev[n - 1]),
                };
                let mut by_row: std::collections::BTreeMap<u32, Vec<u32>> = Default::default();
                for &(bx, by) in &blocks {
                    by_row.entry(by).or_default().push(bx);
                }
                let mut rects = Vec::new();
                for (&by, cols) in &mut by_row {
                    for (a, b) in runs(cols) {
                        let (x0, y0) = (a * BLOCK, by * BLOCK);
                        if x0 < level.w && y0 < level.h {
                            rects.push((x0, y0, ((b + 1) * BLOCK).min(level.w), ((by + 1) * BLOCK).min(level.h)));
                        }
                    }
                }
                // Independent rectangles: computed on all cores, then stored and uploaded.
                let done: Vec<Vec<u8>> = rects.par_iter().map(|&r| downsample(above, r)).collect();
                for (&(x0, y0, x1, y1), data) in rects.iter().zip(&done) {
                    let row = ((x1 - x0) * 4) as usize;
                    for (py, src) in (y0..y1).zip(data.chunks_exact(row)) {
                        let i = ((py * level.w + x0) * 4) as usize;
                        level.data[i..i + row].copy_from_slice(src);
                    }
                    write_level(queue, &chunk.texture, n as u32 + 1, (x0, y0, x1, y1), data);
                }
                blocks = blocks.iter().map(|&(bx, by)| (bx / 2, by / 2)).collect();
            }
        }
    }

    /// Uploads the whole projection (after opening a file or rebuilding it).
    pub fn upload_all(&mut self, queue: &wgpu::Queue, projection: &Projection) {
        let all = projection.all_tiles();
        self.upload(queue, projection, &all);
    }

    /// Draws the canvas into `target`, clearing the rest to `background` (sRGB-encoded 0..1,
    /// like the canvas). Call at most once per queue submission (the view is written to
    /// per-chunk uniform buffers that the submission reads).
    pub fn draw(&mut self, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView, view: &View, background: [f64; 3]) {
        let zoom = view.zoom;
        let nearest = zoom >= NEAREST_FROM_ZOOM;
        // Screen to canvas: the inverse of the view's matrix.
        let [a, b, tx, c, d, ty] = view.matrix();
        let det = a * d - b * c;
        let (ia, ib, ic, id) = (d / det, -b / det, -c / det, a / det);
        let (itx, ity) = (-(ia * tx + ib * ty), -(ic * tx + id * ty));
        let lod = if zoom < 1.0 { (1.0 / zoom).log2() } else { 0.0 };
        let bg = if self.srgb_target { background.map(srgb_to_linear) } else { background };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("canvas display"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: bg[0], g: bg[1], b: bg[2], a: 1.0 }), store: wgpu::StoreOp::Store },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        for ch in &self.chunks {
            let u = Uniform {
                inv0: [ia as f32, ib as f32, itx as f32, 0.0],
                inv1: [ic as f32, id as f32, ity as f32, 0.0],
                chunk: [ch.x as f32, ch.y as f32, ch.w as f32, ch.h as f32],
                params: [lod as f32, if nearest { 1.0 } else { 0.0 }, 8.0, if self.srgb_target { 1.0 } else { 0.0 }],
            };
            queue.write_buffer(&ch.uniform, 0, bytemuck::bytes_of(&u));
            pass.set_bind_group(0, &ch.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}

/// Where a mip level's pixels come from: the level above it.
#[derive(Clone, Copy)]
enum Above<'a> {
    /// Level 0: the chunk's rectangle of the projection (straight alpha).
    Picture { img: &'a Rgba8, x: u32, y: u32, w: u32, h: u32 },
    /// A level computed before (premultiplied).
    Level(&'a Level),
}

impl Above<'_> {
    fn size(&self) -> (u32, u32) {
        match self {
            Above::Picture { w, h, .. } => (*w, *h),
            Above::Level(l) => (l.w, l.h),
        }
    }

    /// `n` pixels of row `sy` from column `sx0`, premultiplied, into `out`. Rows and columns
    /// past the edge repeat the last one (levels of odd sizes).
    fn row(&self, sy: u32, sx0: u32, n: u32, out: &mut Vec<[u8; 4]>) {
        out.clear();
        let (w, h) = self.size();
        let sy = sy.min(h - 1);
        let sx0 = sx0.min(w - 1);
        let len = ((sx0 + n).min(w) - sx0) as usize * 4;
        match *self {
            Above::Picture { img, x, y, .. } => {
                let start = ((y + sy) as usize * img.width as usize + (x + sx0) as usize) * 4;
                out.extend(img.data[start..start + len].as_chunks::<4>().0.iter().map(|&p| premultiply(p)));
            }
            Above::Level(l) => {
                let start = (sy as usize * l.w as usize + sx0 as usize) * 4;
                out.extend_from_slice(l.data[start..start + len].as_chunks::<4>().0);
            }
        }
        let last = *out.last().expect("at least one column");
        out.resize(n as usize, last);
    }
}

/// One block (x0..x1, y0..y1) of a mip level: each pixel the rounded average of its 2x2
/// pixels in the level above. Returns its rows, premultiplied RGBA.
fn downsample(above: Above, (x0, y0, x1, y1): (u32, u32, u32, u32)) -> Vec<u8> {
    let n = 2 * (x1 - x0);
    let (mut top, mut bottom) = (Vec::with_capacity(n as usize), Vec::with_capacity(n as usize));
    let row = (x1 - x0) as usize * 4;
    let mut out = vec![0u8; row * (y1 - y0) as usize];
    for (py, dst) in (y0..y1).zip(out.chunks_exact_mut(row)) {
        above.row(2 * py, 2 * x0, n, &mut top);
        above.row(2 * py + 1, 2 * x0, n, &mut bottom);
        for ((d, t), b) in dst.as_chunks_mut::<4>().0.iter_mut().zip(top.as_chunks::<2>().0).zip(bottom.as_chunks::<2>().0) {
            for c in 0..4 {
                d[c] = ((t[0][c] as u32 + t[1][c] as u32 + b[0][c] as u32 + b[1][c] as u32 + 2) / 4) as u8;
            }
        }
    }
    out
}

/// Runs of consecutive numbers in `v` (sorted and deduplicated here), as (first, last).
fn runs(v: &mut Vec<u32>) -> Vec<(u32, u32)> {
    v.sort_unstable();
    v.dedup();
    let mut out: Vec<(u32, u32)> = Vec::new();
    for &x in v.iter() {
        match out.last_mut() {
            Some(r) if r.1 + 1 == x => r.1 = x,
            _ => out.push((x, x)),
        }
    }
    out
}

/// A rectangle of the projection, premultiplied (what the textures hold).
fn premultiplied(img: &Rgba8, x: u32, y: u32, w: u32, h: u32) -> Vec<u8> {
    let row = w as usize * 4;
    let mut out = vec![0u8; row * h as usize];
    for (r, dst) in out.chunks_exact_mut(row).enumerate() {
        let start = ((y as usize + r) * img.width as usize + x as usize) * 4;
        for (d, s) in dst.as_chunks_mut::<4>().0.iter_mut().zip(img.data[start..start + row].as_chunks::<4>().0) {
            *d = premultiply(*s);
        }
    }
    out
}

/// Copies `data` (rows of the rectangle x0..x1, y0..y1) into mip level `level`.
fn write_level(queue: &wgpu::Queue, texture: &wgpu::Texture, level: u32, (x0, y0, x1, y1): (u32, u32, u32, u32), data: &[u8]) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo { texture, mip_level: level, origin: wgpu::Origin3d { x: x0, y: y0, z: 0 }, aspect: wgpu::TextureAspect::All },
        data,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some((x1 - x0) * 4), rows_per_image: Some(y1 - y0) },
        wgpu::Extent3d { width: x1 - x0, height: y1 - y0, depth_or_array_layers: 1 },
    );
}

/// Straight to premultiplied alpha, rounded (what the display textures hold).
fn premultiply(p: [u8; 4]) -> [u8; 4] {
    match p[3] {
        // What the formula gives for these, without the arithmetic.
        255 => p,
        0 => [0; 4],
        a => {
            let pm = |c: u8| ((c as u32 * a as u32 + 127) / 255) as u8;
            [pm(p[0]), pm(p[1]), pm(p[2]), a]
        }
    }
}

fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

#[cfg(test)]
mod tests;
