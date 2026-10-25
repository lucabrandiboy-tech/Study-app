//! Recedo's GPU canvas display.
//!
//! The CPU keeps the picture (`recedo_core::projection::Projection`); this uploads the
//! tiles that changed into GPU textures (premultiplied, with mipmaps for zoomed-out
//! views) and draws them with the view's pan, zoom, rotation and mirror, over a
//! checkerboard where the picture is see-through. Canvases larger than the GPU's texture
//! limit are split into several textures.

pub mod gpu;

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
    /// Per mip level n >= 1: bind group reading level n-1, and the view of level n.
    mips: Vec<(wgpu::BindGroup, wgpu::TextureView)>,
    mips_stale: bool,
}

pub struct CanvasDisplay {
    pub width: u32,
    pub height: u32,
    chunks: Vec<Chunk>,
    pipeline: wgpu::RenderPipeline,
    mip_pipeline: wgpu::RenderPipeline,
    srgb_target: bool,
    /// Scratch for converting a tile to premultiplied alpha.
    scratch: Vec<u8>,
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
        let mip_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("canvas mipmaps"),
            source: wgpu::ShaderSource::Wgsl(include_str!("mip.wgsl").into()),
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
        let mip_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("canvas mipmaps"), entries: &[tex_entry(0), samp_entry(1)] });
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
        let mip_pipeline = pipeline_for("canvas mipmaps", &mip_module, "fs_downsample", &mip_bgl, TEXTURE_FORMAT);
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
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
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
                let level_view = |level| {
                    texture.create_view(&wgpu::TextureViewDescriptor { base_mip_level: level, mip_level_count: Some(1), ..Default::default() })
                };
                let mips = (1..levels)
                    .map(|level| {
                        let src = level_view(level - 1);
                        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("canvas mip"),
                            layout: &mip_bgl,
                            entries: &[
                                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&src) },
                                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
                            ],
                        });
                        (bg, level_view(level))
                    })
                    .collect();
                chunks.push(Chunk { x, y, w, h, texture, uniform, bind_group, mips, mips_stale: true });
            }
        }
        CanvasDisplay { width, height, chunks, pipeline, mip_pipeline, srgb_target: target_format.is_srgb(), scratch: Vec::new() }
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Uploads changed tiles of the projection.
    pub fn upload(&mut self, queue: &wgpu::Queue, projection: &Projection, tiles: &[TileKey]) {
        let img = &projection.image;
        for &key in tiles {
            let Some((x, y, w, h)) = projection.tile_rect(key) else { continue };
            let Some(chunk) = self.chunks.iter_mut().find(|c| x >= c.x && x < c.x + c.w && y >= c.y && y < c.y + c.h) else { continue };
            self.scratch.clear();
            for row in 0..h {
                let start = ((y + row) as usize * img.width as usize + x as usize) * 4;
                for p in img.data[start..start + w as usize * 4].as_chunks::<4>().0 {
                    let a = p[3] as u32;
                    let pm = |c: u8| ((c as u32 * a + 127) / 255) as u8;
                    self.scratch.extend_from_slice(&[pm(p[0]), pm(p[1]), pm(p[2]), p[3]]);
                }
            }
            queue.write_texture(
                wgpu::TexelCopyTextureInfo { texture: &chunk.texture, mip_level: 0, origin: wgpu::Origin3d { x: x - chunk.x, y: y - chunk.y, z: 0 }, aspect: wgpu::TextureAspect::All },
                &self.scratch,
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: Some(h) },
                wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            );
            chunk.mips_stale = true;
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
        // Mipmaps are only needed when zoomed out; rebuild them lazily.
        if zoom < 1.0 {
            for c in self.chunks.iter_mut().filter(|c| c.mips_stale) {
                for (bg, dst) in &c.mips {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("canvas mip"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: dst,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
                        })],
                        ..Default::default()
                    });
                    pass.set_pipeline(&self.mip_pipeline);
                    pass.set_bind_group(0, bg, &[]);
                    pass.draw(0..3, 0..1);
                }
                c.mips_stale = false;
            }
        }
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

fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

#[cfg(test)]
mod tests;
