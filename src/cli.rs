use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, ValueEnum)]
pub enum NameMode {
    Random,
    Readable,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum SimdMode {
    Auto,
    Avx512,
    Avx2,
    Scalar,
}

#[derive(Parser, Debug)]
#[command(name = "sigcrack", version, about = "Brute-force Solidity function selector collisions")]
pub struct Args {
    /// Selector or signature, e.g. "transfer(address,uint256)" or "0xa9059cbb"
    pub target: Option<String>,

    /// Collisions to find
    #[arg(short = 'n', long, default_value = "1")]
    pub count: u32,

    /// Name style
    #[arg(short, long, default_value = "random")]
    pub mode: NameMode,

    /// Force CPU-only (skip GPU)
    #[arg(long)]
    pub cpu: bool,

    /// Force GPU-only (skip CPU)
    #[arg(long)]
    pub gpu: bool,

    /// Show detected hardware and exit
    #[arg(long)]
    pub list_devices: bool,

    /// Print to stdout instead of TUI
    #[arg(long)]
    pub no_tui: bool,

    /// CPU worker threads [0 = physical cores]
    #[arg(short = 't', long, default_value = "0")]
    pub threads: usize,

    /// Max parameters per generated signature
    #[arg(short = 'p', long, default_value = "4")]
    pub max_params: usize,

    /// Force SIMD: auto, avx512, avx2, scalar
    #[arg(long, default_value = "auto")]
    pub simd: SimdMode,

    /// GPU device index for multi-GPU systems
    #[arg(long, default_value = "0")]
    pub gpu_device: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_parse_signature_target() {
        let args = Args::parse_from(["sigcrack", "transfer(address,uint256)"]);
        assert_eq!(args.target.as_deref(), Some("transfer(address,uint256)"));
        assert_eq!(args.count, 1);
    }

    #[test]
    fn test_parse_hex_target() {
        let args = Args::parse_from(["sigcrack", "0xa9059cbb"]);
        assert_eq!(args.target.as_deref(), Some("0xa9059cbb"));
    }

    #[test]
    fn test_count_flag() {
        let args = Args::parse_from(["sigcrack", "0xa9059cbb", "-n", "5"]);
        assert_eq!(args.count, 5);
    }

    #[test]
    fn test_cpu_flag() {
        let args = Args::parse_from(["sigcrack", "0xa9059cbb", "--cpu"]);
        assert!(args.cpu);
        assert!(!args.gpu);
    }

    #[test]
    fn test_gpu_flag() {
        let args = Args::parse_from(["sigcrack", "0xa9059cbb", "--gpu"]);
        assert!(args.gpu);
        assert!(!args.cpu);
    }

    #[test]
    fn test_list_devices_no_target() {
        let args = Args::parse_from(["sigcrack", "--list-devices"]);
        assert!(args.list_devices);
        assert!(args.target.is_none());
    }

    #[test]
    fn test_hidden_flags_still_work() {
        let args = Args::parse_from([
            "sigcrack", "0xa9059cbb",
            "--threads", "8",
            "--simd", "avx2",
            "--max-params", "3",
        ]);
        assert_eq!(args.threads, 8);
        assert_eq!(args.max_params, 3);
    }
}
