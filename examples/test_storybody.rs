use rs_trafilatura::{extract_with_options, Options};
use std::fs;

fn main() {
    let html_path = "/home/foley/Documents/rs-trafilatura/benchmarks/web-content-extraction-benchmark/datasets/combined/html/l3s-gn1/8c4834d7b7ca686f3fe7003c2f8de5c4231110fed44c9b0f3487b8da35148d1a.html";

    let html = fs::read_to_string(html_path).expect("read file");

    let opts = Options {
        use_readability_fallback: true,
        ..Options::default()
    };

    match extract_with_options(&html, &opts) {
        Ok(result) => {
            let text = &result.content_text;
            println!("Extracted text length: {}", text.len());
            println!("\n=== First 500 chars ===");
            println!("{}", &text.chars().take(500).collect::<String>());
            println!("\n=== Contains 'Boston' (article)? {} ===", text.contains("Boston"));
        }
        Err(e) => {
            println!("Error: {e:?}");
        }
    }
}
