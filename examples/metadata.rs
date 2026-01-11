//! Metadata extraction example for rs-trafilatura.
//!
//! This example demonstrates all available metadata fields.
//!
//! Run with: `cargo run --example metadata`

use rs_trafilatura::extract;

fn main() -> Result<(), rs_trafilatura::Error> {
    // HTML with comprehensive metadata
    let html = r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <title>Breaking: Major Discovery in Science - Science Daily</title>

            <!-- Open Graph metadata -->
            <meta property="og:title" content="Major Discovery in Science">
            <meta property="og:description" content="Scientists announce breakthrough findings.">
            <meta property="og:site_name" content="Science Daily">
            <meta property="og:type" content="article">
            <meta property="og:image" content="https://example.com/image.jpg">
            <meta property="og:url" content="https://example.com/science/discovery">
            <meta property="article:published_time" content="2024-01-15T10:30:00Z">

            <!-- Standard meta tags -->
            <meta name="author" content="Dr. Jane Smith">
            <meta name="description" content="A comprehensive look at the latest scientific breakthrough.">
            <meta name="keywords" content="science, discovery, research, breakthrough">

            <!-- JSON-LD structured data -->
            <script type="application/ld+json">
            {
                "@context": "https://schema.org",
                "@type": "NewsArticle",
                "headline": "Major Discovery in Science",
                "author": {
                    "@type": "Person",
                    "name": "Dr. Jane Smith"
                },
                "datePublished": "2024-01-15T10:30:00Z",
                "publisher": {
                    "@type": "Organization",
                    "name": "Science Daily"
                },
                "articleSection": "Science",
                "keywords": ["science", "discovery", "research"]
            }
            </script>
        </head>
        <body>
            <article>
                <h1>Major Discovery in Science</h1>
                <p class="byline">By Dr. Jane Smith | January 15, 2024</p>
                <p>Scientists have announced a major breakthrough in their research.</p>
                <p>The discovery could have far-reaching implications for the field.</p>
            </article>
        </body>
        </html>
    "#;

    let result = extract(html)?;
    let m = &result.metadata;

    println!("=== Extracted Metadata ===\n");

    // Core metadata
    println!("Title:       {:?}", m.title);
    println!("Author:      {:?}", m.author);
    println!("Date:        {:?}", m.date);
    println!("Description: {:?}", m.description);
    println!();

    // Site information
    println!("Site Name:   {:?}", m.sitename);
    println!("URL:         {:?}", m.url);
    println!("Hostname:    {:?}", m.hostname);
    println!();

    // Media and type
    println!("Image:       {:?}", m.image);
    println!("Page Type:   {:?}", m.page_type);
    println!("Language:    {:?}", m.language);
    println!();

    // Categorization
    if !m.categories.is_empty() {
        println!("Categories:  {:?}", m.categories);
    }
    if !m.tags.is_empty() {
        println!("Tags:        {:?}", m.tags);
    }

    // Additional fields
    if m.license.is_some() {
        println!("License:     {:?}", m.license);
    }
    if m.id.is_some() {
        println!("ID:          {:?}", m.id);
    }
    if m.fingerprint.is_some() {
        println!("Fingerprint: {:?}", m.fingerprint);
    }

    println!("\n=== Content Preview ===\n");
    println!(
        "{}...",
        result.content_text.chars().take(200).collect::<String>()
    );

    Ok(())
}
