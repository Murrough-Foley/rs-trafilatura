/// Simple CLI for benchmarking rs-trafilatura
/// Reads HTML from stdin or file, outputs JSON with extracted content
use rs_trafilatura::extract;
use std::env;
use std::fs;
use std::io::{self, Read};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    // Read HTML content
    let html = if args.len() > 1 {
        // Read from file
        fs::read_to_string(&args[1])?
    } else {
        // Read from stdin
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer
    };

    // Extract with default options
    let result = extract(&html)?;

    // Output as JSON
    let output = serde_json::json!({
        "title": result.metadata.title,
        "author": result.metadata.author,
        "date": result.metadata.date.map(|d| d.to_rfc3339()),
        "main_content": result.content_text,
        "hostname": result.metadata.hostname,
        "description": result.metadata.description,
        "sitename": result.metadata.sitename,
        "categories": result.metadata.categories,
        "tags": result.metadata.tags,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);

    Ok(())
}
