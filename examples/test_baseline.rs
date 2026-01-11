use rs_trafilatura::extractor::fallback::baseline;
use dom_query::Document;
use std::fs;

fn main() {
    let html = fs::read_to_string(
        "/home/foley/Documents/rs-trafilatura/benchmarks/web-content-extraction-benchmark/datasets/combined/html/cleaneval/85cf432edad31057cbd92ed29ba23a268fbe2077f8e28610f40070224ffaf9c1.html"
    ).unwrap();

    let doc = Document::from(html);
    let (_body_doc, text) = baseline(&doc);
    
    println!("Baseline extracted {} chars", text.len());
    println!("\nFirst 1000 chars:\n{}", &text[..text.len().min(1000)]);
}
