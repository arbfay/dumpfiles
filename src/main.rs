extern crate walkdir;
extern crate glob;
extern crate anyhow;
extern crate log;
extern crate dumpfiles;
extern crate clap;

use std::path::PathBuf;
use anyhow::{Context, Result};
use clap::Parser;
use dumpfiles::write_directory_contents;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the directory to process
    #[arg(value_name = "DIRECTORY")]
    directory: PathBuf,

    /// Path to the output file
    #[arg(short, long, default_value = "output.txt")]
    output: PathBuf,

    /// Patterns to ignore (can be used multiple times)
    #[arg(short, long, default_values = [".git*"])]
    ignore: Vec<String>,

    #[arg(short, long, value_name = "GITIGNORE", default_value = ".gitignore")]
    gitignore: Option<PathBuf>,

    #[arg(long)]
    no_gitignore: bool,
}

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    let ignore_patterns: Vec<String> = cli.ignore.iter()
        .map(|p| p.replace("\\", "/"))
        .map(|p| p.trim_end_matches('/').to_string())
        .collect();

    let gitignore_path = if cli.no_gitignore {
        None
    } else {
        cli.gitignore.as_deref()
    };

    log::info!("Starting to process directory: {}", cli.directory.display());
    write_directory_contents(&cli.directory, &cli.output, &ignore_patterns, gitignore_path)
        .context("Failed to write directory contents")?;

    log::info!("Directory contents have been written to {}", cli.output.display());
    Ok(())
}
