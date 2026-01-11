use rs_trafilatura::{extract_with_options, Options};
use std::fs::File;
use std::io::Read;
use flate2::read::GzDecoder;

fn main() {
    let file = File::open("/home/foley/Documents/rs-trafilatura/benchmarks/article-extraction-benchmark/html/6a72de37e8f98f4eee6c0821e593b35ce536cef6c8b424c5e1dd747ebe6621ba.html.gz").unwrap();
    let mut decoder = GzDecoder::new(file);
    let mut html = String::new();
    decoder.read_to_string(&mut html).unwrap();
    
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
    println!("\n=== FULL CONTENT ===\n{}", result.content_text);
}
