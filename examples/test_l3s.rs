use rs_trafilatura::{extract_with_options, Options};
use std::fs;

fn main() {
    let html_path = "benchmarks/web-content-extraction-benchmark/datasets/combined/html/l3s-gn1/a239d7841d541cf94478d35dd979eb49e4f068fac61ea1c26fe293ba8db37775.html";

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
            println!("\n=== Contains 'Canberra' (nav)? {} ===", text.contains("Canberra"));
            println!("=== Contains 'Rottweiler' (article)? {} ===", text.contains("Rottweiler"));
        }
        Err(e) => {
            println!("Error: {e:?}");
        }
    }
}
