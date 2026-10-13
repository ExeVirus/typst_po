use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use typst_po::extract::extract_chunks;
use typst_po::po::PotCatalog;

#[derive(Parser, Debug)]
#[command(name = "typ2pot", about = "Extract gettext .pot templates from Typst files")]
struct Args {
    /// Input .typ file or directory to scan
    #[arg(required = true)]
    input: PathBuf,

    /// Output .pot file path
    #[arg(short, long, default_value = "messages.pot")]
    output: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut catalog = PotCatalog::new();

    let files: Vec<PathBuf> = if args.input.is_dir() {
        WalkDir::new(&args.input)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "typ"))
            .map(|e| e.into_path())
            .collect()
    } else {
        vec![args.input.clone()]
    };

    println!("Scanning {} Typst file(s)...", files.len());

    for file_path in files {
        let source = fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read {}", file_path.display()))?;

        let chunks = extract_chunks(&source);
        let rel_display = file_path.display().to_string();

        for chunk in chunks {
            catalog.add(chunk.text, format!("{}:{}", rel_display, chunk.line), chunk.line);
        }
    }

    catalog
        .write_to_file(&args.output)
        .with_context(|| format!("Failed to write {}", args.output.display()))?;

    println!("Generated template at {}", args.output.display());
    Ok(())
}