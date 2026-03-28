//! Simple CLI that reads HTML from stdin and outputs JSON to stdout.
//! Used by the text-extraction-benchmark Python wrapper.

use rs_trafilatura::{extract_with_options, Options};
use rs_trafilatura::page_type::PageType;
use serde::Serialize;
use std::io::{self, Read};

#[derive(Serialize)]
struct Output {
    title: Option<String>,
    author: Option<String>,
    date: Option<String>,
    main_content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    page_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    classification_confidence: Option<f64>,
    /// Extraction quality confidence (0.0-1.0). Always present.
    /// Pages below ~0.6 are candidates for LLM fallback.
    confidence: f64,
    // Hybrid-only fields
    #[serde(skip_serializing_if = "Option::is_none")]
    content_html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_markdown: Option<String>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Parse optional flags
    let mut url: Option<String> = None;
    let mut page_type_override: Option<PageType> = None;
    let mut hybrid = false;
    let mut markdown = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--url" => {
                if i + 1 < args.len() {
                    url = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("--url requires a value");
                    std::process::exit(1);
                }
            }
            "--page-type" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<PageType>() {
                        Ok(pt) => page_type_override = Some(pt),
                        Err(e) => {
                            eprintln!("Invalid --page-type: {e}");
                            std::process::exit(1);
                        }
                    }
                    i += 2;
                } else {
                    eprintln!("--page-type requires a value");
                    std::process::exit(1);
                }
            }
            "--hybrid" => {
                hybrid = true;
                i += 1;
            }
            "--markdown" => {
                markdown = true;
                i += 1;
            }
            _ => { i += 1; }
        }
    }

    // Read HTML from stdin
    let mut html = String::new();
    if io::stdin().read_to_string(&mut html).is_err() {
        eprintln!("Failed to read from stdin");
        std::process::exit(1);
    }

    // Build options
    let options = Options {
        url,
        page_type: page_type_override,
        output_markdown: markdown,
        include_tables: if markdown { true } else { Options::default().include_tables },
        include_links: if markdown { true } else { Options::default().include_links },
        include_formatting: if markdown { true } else { Options::default().include_formatting },
        ..Options::default()
    };

    // Extract
    let result = extract_with_options(&html, &options);

    // Output JSON
    let output = match result {
        Ok(r) => Output {
            title: r.metadata.title,
            author: r.metadata.author,
            date: r.metadata.date.map(|d| d.to_rfc3339()),
            main_content: r.content_text,
            page_type: r.metadata.page_type,
            classification_confidence: r.classification_confidence,
            confidence: r.extraction_quality,
            content_html: if hybrid { r.content_html } else { None },
            content_markdown: if markdown { r.content_markdown } else { None },
        },
        Err(_) => Output {
            title: None,
            author: None,
            date: None,
            main_content: String::new(),
            page_type: None,
            classification_confidence: None,
            confidence: 0.0,
            content_html: None,
            content_markdown: None,
        },
    };

    println!("{}", serde_json::to_string(&output).unwrap_or_default());
}
