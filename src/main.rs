use anyhow::{Context, Result};
use sigcrack::collision_finder::CollisionFinder;
use clap::Parser;

mod cli;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    let target_hash = cli.target_hash.trim_start_matches("0x");
    let target_hash = hex::decode(target_hash)
        .context("Invalid target hash format. Please provide a valid hex string.")?;

    if target_hash.len() != 4 {
        eprintln!("Invalid hash length. Please provide a 4-byte hex string.");
        std::process::exit(1);
    }

    let length = cli.length.unwrap_or(6);
    let finder = CollisionFinder::new();

    match (&cli.prefix, &cli.suffix) {
        (Some(prefix), Some(suffix)) => {
            finder.find_matching_signature_with_prefix_and_suffix(&target_hash, prefix, length, suffix)?;
        }
        (Some(prefix), None) => {
            finder.find_matching_signature(&target_hash, prefix, length, None)?;
        }
        (None, Some(suffix)) => {
            finder.find_matching_signature_with_suffix(&target_hash, length, suffix)?;
        }
        (None, None) => {
            finder.find_matching_signature_with_length(&target_hash, length)?;
        }
    }

    Ok(())
}