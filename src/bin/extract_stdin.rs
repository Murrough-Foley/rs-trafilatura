//! Simple CLI that reads HTML from stdin and outputs JSON to stdout.
//! Used by the text-extraction-benchmark Python wrapper.

use rs_trafilatura::extract;
use serde::Serialize;
use std::io::{self, Read};

#[derive(Serialize)]
struct Output {
    title: Option<String>,
    author: Option<String>,
    date: Option<String>,
    main_content: String,
}

fn main() {
    // Read HTML from stdin
    let mut html = String::new();
    if io::stdin().read_to_string(&mut html).is_err() {
        eprintln!("Failed to read from stdin");
        std::process::exit(1);
    }

    // Extract with default options
    let result = extract(&html);

    // Output JSON
    let output = match result {
        Ok(r) => Output {
            title: r.metadata.title,
            author: r.metadata.author,
            date: r.metadata.date.map(|d| d.to_rfc3339()),
            main_content: r.content_text,
        },
        Err(_) => Output {
            title: None,
            author: None,
            date: None,
            main_content: String::new(),
        },
    };

    println!("{}", serde_json::to_string(&output).unwrap_or_default());
}
