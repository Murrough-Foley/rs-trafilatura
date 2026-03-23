//! Basic usage example for rs-trafilatura.
//!
//! Run with: `cargo run --example basic`

use rs_trafilatura::{extract, extract_with_options, Options};

fn main() -> Result<(), rs_trafilatura::Error> {
    let html = r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <title>Example Article - My Blog</title>
            <meta name="author" content="Jane Smith">
            <meta name="description" content="An example article demonstrating content extraction.">
            <meta property="og:site_name" content="My Blog">
        </head>
        <body>
            <nav>
                <a href="/">Home</a>
                <a href="/about">About</a>
                <a href="/contact">Contact</a>
            </nav>

            <article>
                <h1>Example Article Title</h1>
                <p class="byline">By Jane Smith | January 15, 2024</p>

                <p>This is the first paragraph of the article. It contains meaningful
                content that demonstrates how rs-trafilatura extracts the main article
                text while filtering out navigation, sidebars, and other boilerplate.</p>

                <p>The second paragraph continues with more content. Notice how the
                extraction preserves the text structure while removing irrelevant
                page elements.</p>

                <p>A third paragraph provides additional context. The trafilatura
                algorithm analyzes page structure to identify the primary content
                region.</p>
            </article>

            <aside>
                <h3>Related Posts</h3>
                <ul>
                    <li>Another article</li>
                    <li>Yet another article</li>
                </ul>
            </aside>

            <footer>
                <p>© 2024 My Blog. All rights reserved.</p>
            </footer>
        </body>
        </html>
    "#;

    // Simple extraction with defaults
    println!("=== Simple Extraction ===\n");
    let result = extract(html)?;

    println!("Title: {:?}", result.metadata.title);
    println!("Author: {:?}", result.metadata.author);
    println!("Site: {:?}", result.metadata.sitename);
    println!("Language: {:?}", result.metadata.language);
    println!("Description: {:?}", result.metadata.description);
    println!("\nContent:\n{}", result.content_text);

    // Extraction with custom options
    println!("\n=== Extraction with Options ===\n");
    let options = Options {
        include_tables: true,
        favor_precision: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options)?;
    println!("Content length: {} characters", result.content_text.len());

    Ok(())
}
