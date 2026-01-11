//! Extract article bodies from the article-extraction-benchmark HTML files
//! and produce output in the same format as other extractors.
//!
//! Usage: cargo run --release --example benchmark_extract

use flate2::read::GzDecoder;
use rs_trafilatura::{extract_with_options, Options};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

// Set to true to enable fallback extraction (baseline + readability)
const USE_FALLBACK: bool = true;

#[derive(Serialize)]
struct OutputEntry {
    #[serde(rename = "articleBody")]
    article_body: String,
}

#[derive(Deserialize)]
struct GroundTruthEntry {
    #[serde(rename = "articleBody")]
    article_body: String,
    url: Option<String>,
}

fn main() {
    let benchmark_dir = Path::new("benchmarks/article-extraction-benchmark");
    let html_dir = benchmark_dir.join("html");
    let output_path = benchmark_dir.join("output/rs_trafilatura.json");

    // Read ground truth to get list of files
    let ground_truth_path = benchmark_dir.join("ground-truth.json");
    let ground_truth: HashMap<String, GroundTruthEntry> =
        serde_json::from_str(&fs::read_to_string(&ground_truth_path).expect("read ground-truth"))
            .expect("parse ground-truth");

    let mut output: HashMap<String, OutputEntry> = HashMap::new();
    let mut success = 0;
    let mut failed = 0;

    for file_id in ground_truth.keys() {
        let html_path = html_dir.join(format!("{file_id}.html.gz"));

        // Read and decompress gzipped HTML
        let html = match read_gzipped_file(&html_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Failed to read {file_id}: {e}");
                failed += 1;
                continue;
            }
        };

        // Extract content with configurable fallback
        let opts = Options {
            use_readability_fallback: USE_FALLBACK,
            ..Options::default()
        };
        match extract_with_options(&html, &opts) {
            Ok(result) => {
                output.insert(
                    file_id.clone(),
                    OutputEntry {
                        article_body: result.content_text,
                    },
                );
                success += 1;
            }
            Err(e) => {
                eprintln!("Extraction failed for {file_id}: {e:?}");
                output.insert(
                    file_id.clone(),
                    OutputEntry {
                        article_body: String::new(),
                    },
                );
                failed += 1;
            }
        }
    }

    // Write output
    let output_json = serde_json::to_string_pretty(&output).expect("serialize output");
    fs::write(&output_path, output_json).expect("write output");

    println!("\n=== Benchmark Extraction Complete ===");
    println!("Fallback enabled: {USE_FALLBACK}");
    println!("Successful: {success}");
    println!("Failed: {failed}");
    println!("Output written to: {}", output_path.display());
}

fn read_gzipped_file(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut decoder = GzDecoder::new(file);
    let mut content = String::new();
    decoder.read_to_string(&mut content)?;
    Ok(content)
}
