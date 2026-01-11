use rs_trafilatura::{extract_with_options, Options};
use std::fs;

fn main() {
    let html = fs::read_to_string(
        "/home/foley/Documents/rs-trafilatura/benchmarks/web-content-extraction-benchmark/datasets/combined/html/readability/6d2da8dabacaab99f5b5bbdf3721197d2d40bf606879db6156cd06f88e8ba928.html"
    ).unwrap();

    let options = Options {
        include_tables: true,
        favor_recall: false,
        favor_precision: false,
        use_readability_fallback: true,
        ..Options::default()
    };

    let result = extract_with_options(&html, &options).unwrap();

    println!("Content length: {} chars", result.content_text.len());
    println!("Warnings: {:?}", result.warnings);
    println!("\nFirst 500 chars:\n{}", &result.content_text[..result.content_text.len().min(500)]);
}
