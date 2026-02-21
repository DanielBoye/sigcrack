pub struct GpuInfo {
    pub name: String,
    pub backend: String,
}

pub fn detect_gpu() -> Option<GpuInfo> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    }))?;
    let info = adapter.get_info();
    let backend = match info.backend {
        wgpu::Backend::Vulkan => "Vulkan",
        wgpu::Backend::Metal => "Metal",
        wgpu::Backend::Dx12 => "DX12",
        wgpu::Backend::Gl => "OpenGL",
        _ => "Unknown",
    };
    Some(GpuInfo {
        name: info.name.clone(),
        backend: backend.to_string(),
    })
}
