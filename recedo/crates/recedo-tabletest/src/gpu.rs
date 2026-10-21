//! GPU setup and the canvas renderer (wgpu: DirectX 12 or Vulkan on Windows).

use wgpu::util::DeviceExt;

use crate::brush::{Dab, DabBatch};

pub const CANVAS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

pub struct Gpu {
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl Gpu {
    /// Backends to try, best first. `RECEDO_GPU_BACKEND` (dx12, vulkan, gl) forces one.
    pub fn backends() -> wgpu::Backends {
        match std::env::var("RECEDO_GPU_BACKEND").ok().as_deref() {
            Some("dx12") => wgpu::Backends::DX12,
            Some("vulkan") => wgpu::Backends::VULKAN,
            Some("gl") => wgpu::Backends::GL,
            _ if cfg!(windows) => wgpu::Backends::DX12 | wgpu::Backends::VULKAN | wgpu::Backends::GL,
            _ => wgpu::Backends::VULKAN | wgpu::Backends::GL,
        }
    }

    pub fn new_instance() -> wgpu::Instance {
        wgpu::Instance::new(wgpu::InstanceDescriptor { backends: Gpu::backends(), ..wgpu::InstanceDescriptor::new_without_display_handle() })
    }

    /// Picks the fastest adapter that can present to `surface` (or any adapter if none).
    ///
    /// Backends are tried in order (DX12, then Vulkan, then GL) and the first that offers
    /// an adapter wins, so a working DX12 is never passed over for another API.
    pub fn new(instance: wgpu::Instance, surface: Option<&wgpu::Surface<'_>>) -> Result<Gpu, String> {
        let mut errors = Vec::new();
        let order = [wgpu::Backends::DX12, wgpu::Backends::VULKAN, wgpu::Backends::GL];
        for backend in order.into_iter().filter(|b| Gpu::backends().contains(*b)) {
            let adapters = pollster::block_on(instance.enumerate_adapters(backend));
            let mut candidates: Vec<wgpu::Adapter> =
                adapters.into_iter().filter(|a| surface.is_none_or(|s| a.is_surface_supported(s))).collect();
            // Discrete GPU first, then integrated, then software.
            candidates.sort_by_key(|a| match a.get_info().device_type {
                wgpu::DeviceType::DiscreteGpu => 0,
                wgpu::DeviceType::IntegratedGpu => 1,
                wgpu::DeviceType::VirtualGpu => 2,
                wgpu::DeviceType::Other => 3,
                wgpu::DeviceType::Cpu => 4,
            });
            for adapter in candidates {
                let desc = wgpu::DeviceDescriptor {
                    label: Some("recedo"),
                    required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
                    ..Default::default()
                };
                match pollster::block_on(adapter.request_device(&desc)) {
                    Ok((device, queue)) => return Ok(Gpu { adapter, device, queue }),
                    Err(e) => errors.push(format!("{}: {e}", adapter.get_info().name)),
                }
            }
        }
        Err(if errors.is_empty() { "no graphics adapter found".to_string() } else { errors.join("; ") })
    }

    pub fn new_headless() -> Result<Gpu, String> {
        Gpu::new(Gpu::new_instance(), None)
    }

    pub fn describe(&self) -> String {
        let i = self.adapter.get_info();
        format!("{} ({:?}, {:?}, driver {} {})", i.name, i.backend, i.device_type, i.driver, i.driver_info)
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    size: [f32; 2],
    _pad: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CompositeUniform {
    show_dots: u32,
    _pad: [u32; 3],
}

/// Which texture dabs go to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    Canvas,
    Dots,
}

struct Target {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

fn make_target(device: &wgpu::Device, w: u32, h: u32, label: &str) -> Target {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: CANVAS_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    Target { texture, view }
}

/// The canvas (white, erasable to transparent), the sample-dot overlay, and the pipelines
/// that draw dabs into them and composite them to the screen.
pub struct CanvasRenderer {
    pub width: u32,
    pub height: u32,
    canvas: Target,
    dots: Target,
    paint: wgpu::RenderPipeline,
    erase: wgpu::RenderPipeline,
    globals: wgpu::Buffer,
    globals_bg: wgpu::BindGroup,
    composite: Option<(wgpu::RenderPipeline, wgpu::BindGroupLayout)>,
    composite_bg: Option<wgpu::BindGroup>,
    composite_u: wgpu::Buffer,
    pub dabs_drawn: u64,
}

const DAB_ATTRS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
    0 => Float32x2,
    1 => Float32x2,
    2 => Float32x4,
    3 => Float32x4,
];

impl CanvasRenderer {
    /// `surface_format`: the window's format, or `None` when rendering off-screen only.
    pub fn new(gpu: &Gpu, width: u32, height: u32, surface_format: Option<wgpu::TextureFormat>) -> CanvasRenderer {
        let device = &gpu.device;
        let width = width.max(1);
        let height = height.max(1);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("tabletest shaders"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders.wgsl").into()),
        });
        let globals_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("dab globals"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let globals = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("dab globals"),
            contents: bytemuck::bytes_of(&Globals { size: [width as f32, height as f32], _pad: [0.0; 2] }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let globals_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("dab globals"),
            layout: &globals_bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals.as_entire_binding() }],
        });
        let dab_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("dab"),
            bind_group_layouts: &[Some(&globals_bgl)],
            immediate_size: 0,
        });
        let dab_pipeline = |label: &str, blend: wgpu::BlendState| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&dab_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_dab"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Dab>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &DAB_ATTRS,
                    })],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs_dab"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState { format: CANVAS_FORMAT, blend: Some(blend), write_mask: wgpu::ColorWrites::ALL })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let paint = dab_pipeline("paint dabs", wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING);
        // Destination-out: what the dab covers becomes transparent.
        let erase_component =
            wgpu::BlendComponent { src_factor: wgpu::BlendFactor::Zero, dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha, operation: wgpu::BlendOperation::Add };
        let erase = dab_pipeline("erase dabs", wgpu::BlendState { color: erase_component, alpha: erase_component });

        let composite = surface_format.map(|format| {
            let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("composite"),
                entries: &[
                    tex_entry(0),
                    tex_entry(1),
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                        count: None,
                    },
                ],
            });
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("composite"),
                bind_group_layouts: &[Some(&bgl)],
                immediate_size: 0,
            });
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("composite"),
                layout: Some(&layout),
                vertex: wgpu::VertexState { module: &module, entry_point: Some("vs_full"), compilation_options: Default::default(), buffers: &[] },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs_composite"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
                }),
                multiview_mask: None,
                cache: None,
            });
            (pipeline, bgl)
        });
        let composite_u = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("composite uniform"),
            size: std::mem::size_of::<CompositeUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut r = CanvasRenderer {
            width,
            height,
            canvas: make_target(device, width, height, "canvas"),
            dots: make_target(device, width, height, "sample dots"),
            paint,
            erase,
            globals,
            globals_bg,
            composite,
            composite_bg: None,
            composite_u,
            dabs_drawn: 0,
        };
        r.rebuild_composite_bg(gpu);
        let mut enc = device.create_command_encoder(&Default::default());
        r.clear(&mut enc);
        gpu.queue.submit([enc.finish()]);
        r
    }

    fn rebuild_composite_bg(&mut self, gpu: &Gpu) {
        self.composite_bg = self.composite.as_ref().map(|(_, bgl)| {
            gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("composite"),
                layout: bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&self.canvas.view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&self.dots.view) },
                    wgpu::BindGroupEntry { binding: 2, resource: self.composite_u.as_entire_binding() },
                ],
            })
        });
    }

    /// Grows the canvas to at least `w`×`h`, keeping what is drawn. Never shrinks, so
    /// making the window smaller and larger again loses nothing.
    pub fn ensure_size(&mut self, gpu: &Gpu, w: u32, h: u32) {
        if w <= self.width && h <= self.height {
            return;
        }
        let nw = w.max(self.width);
        let nh = h.max(self.height);
        let canvas = make_target(&gpu.device, nw, nh, "canvas");
        let dots = make_target(&gpu.device, nw, nh, "sample dots");
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        clear_target(&mut enc, &canvas.view, wgpu::Color::WHITE);
        clear_target(&mut enc, &dots.view, wgpu::Color::TRANSPARENT);
        let extent = wgpu::Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 };
        enc.copy_texture_to_texture(self.canvas.texture.as_image_copy(), canvas.texture.as_image_copy(), extent);
        enc.copy_texture_to_texture(self.dots.texture.as_image_copy(), dots.texture.as_image_copy(), extent);
        gpu.queue.write_buffer(&self.globals, 0, bytemuck::bytes_of(&Globals { size: [nw as f32, nh as f32], _pad: [0.0; 2] }));
        gpu.queue.submit([enc.finish()]);
        self.canvas = canvas;
        self.dots = dots;
        self.width = nw;
        self.height = nh;
        self.rebuild_composite_bg(gpu);
    }

    /// Canvas back to white, dots overlay to transparent.
    pub fn clear(&mut self, enc: &mut wgpu::CommandEncoder) {
        clear_target(enc, &self.canvas.view, wgpu::Color::WHITE);
        clear_target(enc, &self.dots.view, wgpu::Color::TRANSPARENT);
    }

    pub fn clear_dots(&mut self, enc: &mut wgpu::CommandEncoder) {
        clear_target(enc, &self.dots.view, wgpu::Color::TRANSPARENT);
    }

    /// Draws dab batches in order. Each call uploads its own instance buffer, so several
    /// calls may share one encoder.
    pub fn draw(&mut self, gpu: &Gpu, enc: &mut wgpu::CommandEncoder, layer: Layer, batches: &[DabBatch]) {
        let total: usize = batches.iter().map(|b| b.dabs.len()).sum();
        if total == 0 {
            return;
        }
        let mut all: Vec<Dab> = Vec::with_capacity(total);
        for b in batches {
            all.extend_from_slice(&b.dabs);
        }
        let buf = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("dabs"),
            contents: bytemuck::cast_slice(&all),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let view = match layer {
            Layer::Canvas => &self.canvas.view,
            Layer::Dots => &self.dots.view,
        };
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("dabs"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            ..Default::default()
        });
        pass.set_bind_group(0, &self.globals_bg, &[]);
        pass.set_vertex_buffer(0, buf.slice(..));
        let mut start = 0u32;
        for b in batches {
            let n = b.dabs.len() as u32;
            if n == 0 {
                continue;
            }
            pass.set_pipeline(if b.erase { &self.erase } else { &self.paint });
            pass.draw(0..6, start..start + n);
            start += n;
        }
        self.dabs_drawn += total as u64;
    }

    /// Draws the canvas (over a checkerboard) and, optionally, the dots to `view`.
    pub fn composite(&self, gpu: &Gpu, enc: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, show_dots: bool) {
        let (Some((pipeline, _)), Some(bg)) = (&self.composite, &self.composite_bg) else { return };
        gpu.queue.write_buffer(&self.composite_u, 0, bytemuck::bytes_of(&CompositeUniform { show_dots: show_dots as u32, _pad: [0; 3] }));
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
            })],
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bg, &[]);
        pass.draw(0..3, 0..1);
    }

    /// Reads the canvas back as tightly packed RGBA8 (premultiplied), `width`×`height`.
    pub fn read_canvas(&self, gpu: &Gpu) -> Result<Vec<u8>, String> {
        read_texture(gpu, &self.canvas.texture, self.width, self.height)
    }

}

fn tex_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn clear_target(enc: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, color: wgpu::Color) {
    let _pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("clear"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations { load: wgpu::LoadOp::Clear(color), store: wgpu::StoreOp::Store },
        })],
        ..Default::default()
    });
}

fn read_texture(gpu: &Gpu, texture: &wgpu::Texture, w: u32, h: u32) -> Result<Vec<u8>, String> {
    let row = w * 4;
    let padded = row.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: padded as u64 * h as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = gpu.device.create_command_encoder(&Default::default());
    enc.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(padded), rows_per_image: Some(h) } },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
    gpu.queue.submit([enc.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buf.map_async(wgpu::MapMode::Read, .., move |r| {
        let _ = tx.send(r);
    });
    gpu.device.poll(wgpu::PollType::wait_indefinitely()).map_err(|e| format!("GPU poll failed: {e}"))?;
    rx.recv().map_err(|e| e.to_string())?.map_err(|e| format!("readback failed: {e}"))?;
    let data = buf.get_mapped_range(..).map_err(|e| format!("readback failed: {e:?}"))?;
    let mut out = Vec::with_capacity((row * h) as usize);
    for y in 0..h as usize {
        out.extend_from_slice(&data[y * padded as usize..y * padded as usize + row as usize]);
    }
    drop(data);
    buf.unmap();
    Ok(out)
}

/// Writes RGBA8 pixels as a PNG.
pub fn write_png(path: &std::path::Path, w: u32, h: u32, rgba: &[u8]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(|e| e.to_string())?;
    wr.write_image_data(rgba).map_err(|e| e.to_string())?;
    Ok(())
}

/// Un-premultiplies RGBA8 in place (for saving PNGs, which are straight alpha).
pub fn unpremultiply(rgba: &mut [u8]) {
    for px in rgba.chunks_exact_mut(4) {
        let a = px[3] as u32;
        if a != 0 && a != 255 {
            for c in &mut px[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
}
