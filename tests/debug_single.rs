use rs_trafilatura::extract;
use std::fs;

#[test]
fn debug_single_file() {
    let file = "c582d3b772578e8feaa3cfd8f5ae8100bb6f0bc66048204a9a398395841c1164";
    let html_path = format!("tests/benchmark_suite/{file}.html");
    let html = fs::read_to_string(&html_path).expect("failed to read HTML");

    let result = extract(&html).expect("extraction failed");

    println!("\n=== Extraction Results ===");
    println!("Content length: {}", result.content_text.len());
    println!("Content (first 500 chars):\n{}", &result.content_text.chars().take(500).collect::<String>());
    println!("\n=== Metadata ===");
    println!("Title: {:?}", result.metadata.title);
    println!("Author: {:?}", result.metadata.author);
    println!("Language: {:?}", result.metadata.language);
    println!("\n=== Warnings ===");
    for warning in &result.warnings {
        println!("  - {warning}");
    }

    // This will fail if content is empty
    assert!(!result.content_text.trim().is_empty(), "Content should not be empty");
}
