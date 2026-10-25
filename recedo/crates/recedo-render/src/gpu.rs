//! Picking a graphics adapter and opening a device (shared by Recedo and the tablet test app).

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
