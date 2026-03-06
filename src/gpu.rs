pub struct GpuInfo {
    pub name: String,
    pub backend: String,
    pub adapter: wgpu::Adapter,
}

/// Hardware GPUs in adapter order. Software rasterizers are skipped.
pub fn detect() -> Vec<GpuInfo> {
    let instance = wgpu::Instance::default();
    instance
        .enumerate_adapters(wgpu::Backends::PRIMARY)
        .into_iter()
        .filter(|a| a.get_info().device_type != wgpu::DeviceType::Cpu)
        .map(|adapter| {
            let info = adapter.get_info();
            GpuInfo {
                name: info.name,
                backend: info.backend.to_string(),
                adapter,
            }
        })
        .collect()
}
