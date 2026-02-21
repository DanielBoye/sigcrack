use std::fmt;

pub struct HardwareInfo {
    pub cpu_name: String,
    pub physical_cores: usize,
    pub logical_cores: usize,
    pub has_avx2: bool,
    pub has_avx512f: bool,
    pub gpu_name: Option<String>,
    pub gpu_backend: Option<String>,
}

impl HardwareInfo {
    pub fn detect() -> Self {
        Self {
            cpu_name: Self::detect_cpu_name(),
            physical_cores: num_cpus::get_physical(),
            logical_cores: num_cpus::get(),
            has_avx2: Self::detect_avx2(),
            has_avx512f: Self::detect_avx512f(),
            gpu_name: None,
            gpu_backend: None,
        }
    }

    pub fn has_gpu(&self) -> bool {
        self.gpu_name.is_some()
    }

    pub fn print_devices(&self) {
        println!("Devices:");
        println!("  CPU: {} — {}C/{}T — {}",
            self.cpu_name, self.physical_cores, self.logical_cores, self.best_simd_name());
        match (&self.gpu_name, &self.gpu_backend) {
            (Some(name), Some(backend)) => println!("  GPU #0: {} ({})", name, backend),
            _ => println!("  GPU: none detected"),
        }
    }

    fn detect_cpu_name() -> String {
        #[cfg(target_os = "linux")]
        {
            if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
                for line in cpuinfo.lines() {
                    if line.starts_with("model name") {
                        if let Some(name) = line.split(':').nth(1) {
                            return name.trim().to_string();
                        }
                    }
                }
            }
        }
        "Unknown CPU".to_string()
    }

    #[cfg(target_arch = "x86_64")]
    fn detect_avx2() -> bool { is_x86_feature_detected!("avx2") }
    #[cfg(not(target_arch = "x86_64"))]
    fn detect_avx2() -> bool { false }

    #[cfg(target_arch = "x86_64")]
    fn detect_avx512f() -> bool { is_x86_feature_detected!("avx512f") }
    #[cfg(not(target_arch = "x86_64"))]
    fn detect_avx512f() -> bool { false }

    pub fn best_simd_name(&self) -> &str {
        if self.has_avx512f { "AVX-512 (8-way)" }
        else if self.has_avx2 { "AVX2 (4-way)" }
        else { "scalar" }
    }
}

impl fmt::Display for HardwareInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} — {}C/{}T — {}",
            self.cpu_name, self.physical_cores, self.logical_cores, self.best_simd_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_returns_valid_info() {
        let hw = HardwareInfo::detect();
        assert!(hw.physical_cores > 0);
        assert!(hw.logical_cores >= hw.physical_cores);
    }

    #[test]
    fn test_no_gpu_by_default() {
        let hw = HardwareInfo::detect();
        assert!(!hw.has_gpu());
    }
}
