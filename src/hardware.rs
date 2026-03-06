use std::fmt;

use crate::gpu::{self, GpuInfo};
use crate::hasher::SimdWidth;

pub struct Hardware {
    pub cpu_name: String,
    pub physical_cores: usize,
    pub logical_cores: usize,
    pub avx2: bool,
    pub avx512f: bool,
    pub gpus: Vec<GpuInfo>,
}

impl Hardware {
    pub fn detect() -> Self {
        let (avx2, avx512f) = simd_support();
        Self {
            cpu_name: cpu_name(),
            physical_cores: num_cpus::get_physical(),
            logical_cores: num_cpus::get(),
            avx2,
            avx512f,
            gpus: gpu::detect(),
        }
    }

    pub fn best_simd(&self) -> SimdWidth {
        if self.avx512f {
            SimdWidth::Avx512
        } else if self.avx2 {
            SimdWidth::Avx2
        } else {
            SimdWidth::Scalar
        }
    }

    pub fn print_devices(&self) {
        println!("cpu: {self}");
        if self.gpus.is_empty() {
            println!("gpu: none");
        }
        for (i, gpu) in self.gpus.iter().enumerate() {
            println!("gpu {i}: {} ({})", gpu.name, gpu.backend);
        }
    }
}

impl fmt::Display for Hardware {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}C/{}T, {})",
            self.cpu_name,
            self.physical_cores,
            self.logical_cores,
            self.best_simd().name()
        )
    }
}

#[cfg(target_arch = "x86_64")]
fn simd_support() -> (bool, bool) {
    (
        is_x86_feature_detected!("avx2"),
        is_x86_feature_detected!("avx512f"),
    )
}

#[cfg(not(target_arch = "x86_64"))]
fn simd_support() -> (bool, bool) {
    (false, false)
}

fn cpu_name() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(info) = std::fs::read_to_string("/proc/cpuinfo") {
            let name = info
                .lines()
                .find(|l| l.starts_with("model name"))
                .and_then(|l| l.split_once(':'))
                .map(|(_, v)| v.trim().to_string());
            if let Some(name) = name {
                return name;
            }
        }
    }
    "unknown".to_string()
}
