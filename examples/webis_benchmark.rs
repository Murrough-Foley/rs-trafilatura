//! Extract article bodies from the Webis web-content-extraction-benchmark HTML files
//! and produce output in the same JSONL format as other extractors.
//!
//! Usage: cargo run --release --example webis_benchmark
//!
//! This produces output files in outputs/model-outputs/<dataset>/rs_trafilatura.jsonl

use rs_trafilatura::{extract_with_options, Options};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::panic;
use std::path::Path;

// Set to true to enable fallback extraction (baseline + readability)
const USE_FALLBACK: bool = true;

const DATASETS: &[&str] = &[
    "cetd",
    "cleaneval",
    "cleanportaleval",
    "dragnet",
    "google-trends-2017",
    "l3s-gn1",
    "readability",
    "scrapinghub",
];

#[derive(Serialize)]
struct OutputEntry {
    page_id: String,
    plaintext: String,
}

#[derive(Deserialize)]
struct GroundTruthEntry {
    page_id: String,
    #[allow(dead_code)]
    plaintext: String,
    #[allow(dead_code)]
    url: Option<String>,
}

fn main() {
    let benchmark_dir = Path::new("benchmarks/web-content-extraction-benchmark");
    let html_base = benchmark_dir.join("datasets/combined/html");
    let gt_base = benchmark_dir.join("datasets/combined/ground-truth");
    let output_base = benchmark_dir.join("outputs/model-outputs");

    let opts = Options {
        use_readability_fallback: USE_FALLBACK,
        ..Options::default()
    };

    let mut total_success = 0;
    let mut total_failed = 0;

    for dataset in DATASETS {
        let html_dir = html_base.join(dataset);
        let gt_path = gt_base.join(format!("{dataset}.jsonl"));
        let output_dir = output_base.join(dataset);
        let output_path = output_dir.join("rs_trafilatura.jsonl");

        if !html_dir.exists() {
            eprintln!("Skipping {dataset}: HTML directory not found");
            continue;
        }

        if !gt_path.exists() {
            eprintln!("Skipping {dataset}: ground truth not found");
            continue;
        }

        // Create output directory
        fs::create_dir_all(&output_dir).expect("create output dir");

        // Read ground truth to get list of page IDs
        let gt_file = File::open(&gt_path).expect("open ground truth");
        let reader = BufReader::new(gt_file);

        let mut output_file = File::create(&output_path).expect("create output file");
        let mut success = 0;
        let mut failed = 0;

        for line in reader.lines() {
            let line = line.expect("read line");
            let entry: GroundTruthEntry = match serde_json::from_str(&line) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("Failed to parse ground truth entry: {e}");
                    continue;
                }
            };

            let html_path = html_dir.join(format!("{}.html", entry.page_id));

            // Read HTML file
            let html = match fs::read_to_string(&html_path) {
                Ok(content) => content,
                Err(e) => {
                    eprintln!("[{}] Failed to read {}: {}", dataset, entry.page_id, e);
                    failed += 1;
                    // Write empty entry
                    let output_entry = OutputEntry {
                        page_id: entry.page_id,
                        plaintext: String::new(),
                    };
                    writeln!(output_file, "{}", serde_json::to_string(&output_entry).unwrap()).unwrap();
                    continue;
                }
            };

            // Extract content (with panic catching for malformed HTML)
            let html_clone = html.clone();
            let opts_clone = opts.clone();
            let plaintext = match panic::catch_unwind(move || extract_with_options(&html_clone, &opts_clone)) {
                Ok(Ok(result)) => {
                    success += 1;
                    result.content_text
                }
                Ok(Err(e)) => {
                    eprintln!("[{}] Extraction failed for {}: {:?}", dataset, entry.page_id, e);
                    failed += 1;
                    String::new()
                }
                Err(_) => {
                    eprintln!("[{}] Panic during extraction for {}", dataset, entry.page_id);
                    failed += 1;
                    String::new()
                }
            };

            let output_entry = OutputEntry {
                page_id: entry.page_id,
                plaintext,
            };
            writeln!(output_file, "{}", serde_json::to_string(&output_entry).unwrap()).unwrap();
        }

        println!("{dataset}: {success} success, {failed} failed");
        total_success += success;
        total_failed += failed;
    }

    println!("\n=== Webis Benchmark Extraction Complete ===");
    println!("Fallback enabled: {USE_FALLBACK}");
    println!("Total successful: {total_success}");
    println!("Total failed: {total_failed}");
    println!("Output written to: {}/*/rs_trafilatura.jsonl", output_base.display());
}
