use clap::Parser;

#[derive(Parser)]
#[command(name = "sigcrack")]
#[command(about = "A tool to find Solidity function signature collisions", long_about = None)]
pub struct Cli {
    #[arg(short, long)]
    pub target_hash: String,

    #[arg(short, long)]
    pub prefix: Option<String>,

    #[arg(short, long)]
    pub suffix: Option<String>,

    #[arg(short, long)]
    pub length: Option<usize>,
}