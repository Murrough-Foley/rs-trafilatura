//! Markdown output example for rs-trafilatura.
//!
//! This example demonstrates how to extract content as GitHub Flavored Markdown.
//!
//! The `output_markdown` option populates `ExtractResult.content_markdown` with
//! markdown generated from the extracted HTML content.
//!
//! ## What Gets Converted
//!
//! - Headings (h1-h6) → # Heading syntax
//! - Unordered lists → - item syntax
//! - Ordered lists → 1. item syntax
//! - Paragraphs → text blocks
//!
//! ## Current Limitations
//!
//! - Code blocks (`<pre><code>`) and inline code (`<code>`) are stripped by
//!   the extraction process and not included in markdown output
//! - Tables: Text content is preserved but table structure is not
//! - Bold/italic: Text is preserved but formatting is stripped
//!
//! These limitations will be addressed in future enhancements to preserve
//! more HTML structure during extraction.
//!
//! Run with: `cargo run --example markdown_output`

use rs_trafilatura::{extract_with_options, Options};

fn main() -> Result<(), rs_trafilatura::Error> {
    let html = r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <title>Getting Started with Rust - Programming Blog</title>
            <meta name="author" content="Alex Developer">
        </head>
        <body>
            <nav>
                <a href="/">Home</a>
                <a href="/rust">Rust</a>
            </nav>

            <article>
                <h1>Getting Started with Rust</h1>

                <p>Rust is a systems programming language that runs blazingly fast,
                prevents segfaults, and guarantees thread safety.</p>

                <h2>Why Rust?</h2>

                <ul>
                    <li>Memory safety without garbage collection</li>
                    <li>Zero-cost abstractions</li>
                    <li>Fearless concurrency</li>
                </ul>

                <h2>Installation</h2>

                <p>Install Rust using rustup:</p>

                <pre><code>curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh</code></pre>

                <h2>Your First Program</h2>

                <p>Create a file named main.rs and run:</p>

                <pre><code>fn main() {
    println!("Hello, world!");
}</code></pre>
            </article>

            <footer>
                <p>&copy; 2024 Programming Blog</p>
            </footer>
        </body>
        </html>
    "#;

    // Enable markdown output
    let options = Options {
        output_markdown: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options)?;

    println!("=== Extracted Metadata ===");
    println!("Title: {:?}", result.metadata.title);
    println!("Author: {:?}", result.metadata.author);

    println!("\n=== Markdown Output ===\n");
    if let Some(ref markdown) = result.content_markdown {
        println!("{}", markdown);
    } else {
        println!("(Markdown output is None - check that output_markdown is enabled)");
    }

    Ok(())
}
