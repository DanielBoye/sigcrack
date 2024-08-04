use clap::Parser;

#[derive(Parser)]
#[command(name = "sigcrack")]
#[command(about = "A tool to find Solidity function signature collisions", long_about = None)]
pub struct Cli {
    #[arg(short, long, help = "The target hash to find a collision for")]
    pub target_hash: String,

    #[arg(short, long, help = "Optional prefix for the function name")]
    pub prefix: Option<String>,

    #[arg(short, long, help = "Optional function parameter")]
    pub suffix: Option<String>,

    #[arg(short, long, help = "Length of the function name")]
    pub length: Option<usize>,
}