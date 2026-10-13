use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use typst_po::extract::translate_source;
use typst_po::po::parse_po_file;

#[derive(Parser, Debug)]
#[command(name = "typ-translate", about = "Inject .po translations into a mirrored build folder")]
struct Args {
    /// Master .typ file or root source directory
    #[arg(required = true)]
    input: PathBuf,

    /// Translated .po file (e.g. locales/de.po)
    #[arg(required = true)]
    po: PathBuf,

    /// Target build directory (e.g. build/de/)
    #[arg(short, long, default_value = "build/translated")]
    output_dir: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("Loading translations from {}...", args.po.display());
    let translations = parse_po_file(&args.po)
        .with_context(|| format!("Failed to parse PO file {}", args.po.display()))?;

    if args.input.is_file() {
        let source = fs::read_to_string(&args.input)
            .with_context(|| format!("Failed to read {}", args.input.display()))?;
        let translated = translate_source(&source, &translations);

        fs::create_dir_all(&args.output_dir)?;
        let file_name = args.input.file_name().unwrap();
        let target_path = args.output_dir.join(file_name);

        fs::write(&target_path, translated)?;
        println!("Output written to {}", target_path.display());
    } else {
        println!("Mirroring tree from {} to {}...", args.input.display(), args.output_dir.display());

        for entry in WalkDir::new(&args.input).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            let rel = path.strip_prefix(&args.input)?;
            let dest = args.output_dir.join(rel);

            if entry.file_type().is_dir() {
                fs::create_dir_all(&dest)?;
            } else if path.extension().is_some_and(|ext| ext == "typ") {
                let source = fs::read_to_string(path)?;
                let translated = translate_source(&source, &translations);
                fs::write(&dest, translated)?;
            } else {
                // Copy assets (.png, .svg, data) so relative includes don't break
                fs::copy(path, &dest)?;
            }
        }
        println!("Translation build complete in {}", args.output_dir.display());
    }

    Ok(())
}