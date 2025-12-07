//! Tag Comparison Binary
//!
//! Command-line tool to compare tags extracted by OxiDex vs ExifTool

use clap::Parser;
use std::path::PathBuf;

mod models;
mod extraction;
mod comparison;

use models::ComparisonReport;
use extraction::{OxiDexExtractor, ExifToolExtractor};
use comparison::{ComparisonEngine, generate_markdown_reports};

#[derive(Parser, Debug)]
#[command(name = "tag-comparison")]
#[command(about = "Compare tags extracted by OxiDex vs ExifTool", long_about = None)]
struct Args {
    /// Path to test fixtures directory
    #[arg(long, default_value = "tests/fixtures")]
    fixture_path: PathBuf,

    /// Specific format to process (if not specified, all formats)
    #[arg(long)]
    format: Option<String>,

    /// Output file for JSON results
    #[arg(short, long, default_value = "comparison.json")]
    output: PathBuf,

    /// Path to exiftool executable
    #[arg(long, default_value = "exiftool")]
    exiftool: String,

    /// Output directory for markdown reports
    #[arg(long, default_value = "docs/compatibility")]
    markdown_dir: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("🏷️  Tag Comparison Tool");
    println!("=======================\n");

    // Create report
    let mut report = ComparisonReport::new();

    // Determine which formats to process
    let formats = if let Some(format) = args.format {
        vec![format]
    } else {
        // TODO: Auto-detect formats from fixture directory
        vec![
            "JPEG".to_string(),
            "PNG".to_string(),
            "TIFF".to_string(),
            "PDF".to_string(),
            "MP4".to_string(),
        ]
    };

    // Process each format
    for format in formats {
        println!("Processing format: {}", format);

        // Extract OxiDex tags
        let mut oxidex_extractor = OxiDexExtractor::new(args.fixture_path.clone());
        match oxidex_extractor.extract_format_tags(&format).await {
            Ok(oxidex_tags) => {
                println!("  OxiDex found {} tags", oxidex_tags.len());

                // Extract ExifTool tags
                let mut exiftool_extractor = ExifToolExtractor::new(args.exiftool.clone());
                match exiftool_extractor.extract_format_tags(&format, &args.fixture_path).await {
                    Ok(exiftool_tags) => {
                        println!("  ExifTool found {} tags", exiftool_tags.len());

                        // Compare
                        let previous = None; // TODO: Load from baseline
                        let comparison = ComparisonEngine::compare(oxidex_tags, exiftool_tags, &format, previous);
                        println!("  Result: {}", comparison.summary());

                        report.add_format(format, comparison);
                    }
                    Err(e) => eprintln!("  Error extracting ExifTool tags: {}", e),
                }
            }
            Err(e) => eprintln!("  Error extracting OxiDex tags: {}", e),
        }
    }

    // Calculate overall coverage
    report.calculate_overall_coverage();

    println!("\n📊 Overall Results");
    println!("==================");
    println!("{}", report.summary);

    // Output results
    let json = serde_json::to_string_pretty(&report)?;
    std::fs::write(&args.output, json)?;
    println!("\n✅ Results saved to: {}", args.output.display());

    // Generate markdown reports
    println!("\n📝 Generating markdown reports...");
    generate_markdown_reports(&report, &args.markdown_dir)?;
    println!("✅ Markdown reports saved to: {}", args.markdown_dir.display());

    Ok(())
}
