use anyhow::Result;
use sigcrack::collision_finder::CollisionFinder;
use clap::Parser;

mod cli;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    let target_hash = cli.target_hash.trim_start_matches("0x");
    let target_hash = hex::decode(target_hash)
        .expect("Invalid target hash format. Please provide a valid hex string.");

    if target_hash.len() != 4 {
        eprintln!("Invalid hash length. Please provide a 4-byte hex string.");
        std::process::exit(1);
    }

    let length = cli.length.unwrap_or(6);
    let finder = CollisionFinder::new();

    if let Some(prefix) = &cli.prefix {
        if let Some(suffix) = &cli.suffix {
            finder.find_matching_signature_with_prefix_and_suffix(&target_hash, prefix, length, suffix)?;
        } else {
            finder.find_matching_signature(&target_hash, prefix, length, None)?;
        }
    } else if let Some(suffix) = &cli.suffix {
        finder.find_matching_signature_with_suffix(&target_hash, length, suffix)?;
    } else {
        finder.find_matching_signature_with_length(&target_hash, length)?;
    }

    Ok(())
}