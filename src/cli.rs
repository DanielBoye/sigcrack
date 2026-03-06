use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum NameMode {
    /// Every identifier, shortest first
    Random,
    /// camelCase names from a word list
    Readable,
}

impl NameMode {
    pub fn name(self) -> &'static str {
        match self {
            NameMode::Random => "random",
            NameMode::Readable => "readable",
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SimdMode {
    Auto,
    Avx512,
    Avx2,
    Scalar,
}

#[derive(Parser, Debug)]
#[command(
    name = "sigcrack",
    version,
    about = "Brute-force Solidity function selector collisions"
)]
pub struct Args {
    /// Selector or signature, e.g. 0xa9059cbb or "transfer(address,uint256)"
    #[arg(required_unless_present = "list_devices")]
    pub target: Option<String>,

    /// Collisions to find
    #[arg(short = 'n', long, default_value_t = 1, value_name = "N")]
    pub count: u32,

    /// Name style
    #[arg(short, long, default_value = "random")]
    pub mode: NameMode,

    /// CPU only
    #[arg(long, conflicts_with = "gpu")]
    pub cpu: bool,

    /// GPU only, fail if none is found
    #[arg(long)]
    pub gpu: bool,

    /// Show detected hardware and exit
    #[arg(long)]
    pub list_devices: bool,

    /// Plain output instead of the TUI
    #[arg(long)]
    pub no_tui: bool,

    /// CPU threads, 0 = physical cores
    #[arg(short, long, default_value_t = 0, value_name = "N")]
    pub threads: usize,

    /// Max parameters per signature, 0 to 6
    #[arg(short = 'p', long, default_value_t = 4, value_name = "N", value_parser = clap::value_parser!(u8).range(0..=6))]
    pub max_params: u8,

    /// SIMD backend
    #[arg(long, default_value = "auto")]
    pub simd: SimdMode,

    /// GPU index from --list-devices
    #[arg(long, default_value_t = 0, value_name = "ID")]
    pub gpu_device: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_required_unless_listing_devices() {
        assert!(Args::try_parse_from(["sigcrack"]).is_err());
        assert!(Args::try_parse_from(["sigcrack", "--list-devices"]).is_ok());
        let args = Args::parse_from(["sigcrack", "0xa9059cbb", "-n", "5", "-m", "readable"]);
        assert_eq!(args.target.as_deref(), Some("0xa9059cbb"));
        assert_eq!(args.count, 5);
        assert_eq!(args.mode, NameMode::Readable);
    }

    #[test]
    fn cpu_and_gpu_conflict() {
        assert!(Args::try_parse_from(["sigcrack", "0xa9059cbb", "--cpu", "--gpu"]).is_err());
    }

    #[test]
    fn max_params_is_capped() {
        assert!(Args::try_parse_from(["sigcrack", "0xa9059cbb", "-p", "6"]).is_ok());
        assert!(Args::try_parse_from(["sigcrack", "0xa9059cbb", "-p", "7"]).is_err());
    }
}
