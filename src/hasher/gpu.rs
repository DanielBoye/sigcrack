use wgpu::util::DeviceExt;

const SHADER_SOURCE: &str = include_str!("../shaders/keccak256.wgsl");
const SLOT_SIZE_U32: u32 = 64; // 256 bytes per slot
const MAX_MATCHES: u32 = 1024;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    target_b0: u32,
    target_b1: u32,
    target_b2: u32,
    target_b3: u32,
    count: u32,
    _pad: [u32; 3],
}

pub struct GpuHasher {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    batch_size: u32,
}

impl GpuHasher {
    pub fn new(_gpu_device_index: usize) -> Result<Self, String> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        }))
        .ok_or_else(|| "no GPU adapter found".to_string())?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("sigcrack"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            },
            None,
        ))
        .map_err(|e| format!("failed to create device: {}", e))?;

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("keccak256"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("keccak_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("keccak_pl"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("keccak_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let batch_size = 65536u32;

        Ok(Self {
            device,
            queue,
            pipeline,
            bind_group_layout,
            batch_size,
        })
    }

    pub fn batch_size(&self) -> u32 {
        self.batch_size
    }

    pub fn hash_batch(&self, candidates: &[&[u8]], target: [u8; 4]) -> Vec<usize> {
        let count = candidates.len().min(self.batch_size as usize) as u32;
        if count == 0 {
            return vec![];
        }

        let input_size = (self.batch_size * SLOT_SIZE_U32) as usize;
        let mut input_data = vec![0u32; input_size];

        for (i, candidate) in candidates.iter().enumerate().take(count as usize) {
            let slot_off = (i as u32 * SLOT_SIZE_U32) as usize;
            input_data[slot_off] = candidate.len() as u32;
            let data_off = slot_off + 1;
            for (j, &byte) in candidate.iter().enumerate() {
                let word_idx = data_off + j / 4;
                let shift = (j % 4) * 8;
                input_data[word_idx] |= (byte as u32) << shift;
            }
        }

        let input_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("input"),
            contents: bytemuck::cast_slice(&input_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let output_size = ((1 + MAX_MATCHES) * 4) as u64;
        let output_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("output"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging"),
            size: output_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let params = Params {
            target_b0: target[0] as u32,
            target_b1: target[1] as u32,
            target_b2: target[2] as u32,
            target_b3: target[3] as u32,
            count,
            _pad: [0; 3],
        };

        let params_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("keccak_bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buf.as_entire_binding(),
                },
            ],
        });

        let workgroups = (count + 255) / 256;

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("keccak_enc"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("keccak_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output_buf, 0, &staging_buf, 0, output_size);
        self.queue.submit(Some(encoder.finish()));

        let slice = staging_buf.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device.poll(wgpu::Maintain::Wait);
        receiver
            .recv()
            .map_err(|_| "map recv failed")
            .and_then(|r| r.map_err(|_| "map failed"))
            .expect("GPU readback failed");

        let data = slice.get_mapped_range();
        let result: &[u32] = bytemuck::cast_slice(&data);
        let match_count = result[0].min(MAX_MATCHES) as usize;
        let mut matched_indices = Vec::with_capacity(match_count);
        for i in 0..match_count {
            matched_indices.push(result[1 + i] as usize);
        }
        drop(data);
        staging_buf.unmap();

        matched_indices
    }
}
