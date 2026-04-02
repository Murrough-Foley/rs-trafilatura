# rs-trafilatura

Fast and accurate web content extraction in Rust.

A high-performance Rust port of [trafilatura](https://github.com/adbar/trafilatura) / [go-trafilatura](https://github.com/markusmobius/go-trafilatura), extracting clean, readable content from web pages while removing boilerplate, navigation, and advertisements.

## Features

- **Fast**: 71 files/s for articles, 46 files/s overall on a 1,497-page benchmark (pure Rust, compile-time regex)
- **Accurate**: F1 0.966 on ScrapingHub benchmark, F1 0.859 across 7 page types
- **Page Type Classification**: XGBoost classifier (200 trees, 181 features) detects 7 page types: article, forum, product, collection, listing, documentation, service
- **Per-Type Extraction**: Specialized extraction profiles tuned for each page type (12 forum platforms, 4 documentation frameworks, JSON-LD product fallback)
- **Extraction Quality Predictor**: ML-based confidence scoring (0.0-1.0) using a 27-feature XGBoost model that predicts extraction F1 — pages below 0.80 are candidates for LLM fallback
- **Markdown Output**: GitHub Flavored Markdown preserving headings, lists, tables, bold/italic, code blocks
- **Rich Metadata**: Title, author, date, description, categories, tags, license, images from JSON-LD, Open Graph, Dublin Core, and HTML meta tags
- **Configurable**: 28 options to tune precision/recall tradeoff, content selection, and output format
- **Robust**: Handles malformed HTML gracefully with automatic character encoding detection (UTF-8, ISO-8859-1, Windows-1252)

## Quick Start

```rust
use rs_trafilatura::extract;

fn main() -> Result<(), rs_trafilatura::Error> {
    let html = r#"
        <html>
        <head><title>My Article</title></head>
        <body>
            <nav>Home | About | Contact</nav>
            <article>
                <h1>Welcome</h1>
                <p>This is the main content of the article.</p>
            </article>
            <footer>Copyright 2024</footer>
        </body>
        </html>
    "#;

    let result = extract(html)?;

    println!("Title: {:?}", result.metadata.title);
    println!("Content: {}", result.content_text);
    println!("Page type: {:?}", result.metadata.page_type);
    println!("Confidence: {:.2}", result.extraction_quality);

    Ok(())
}
```

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
rs-trafilatura = "0.2"
```

## Usage

### Basic Extraction

```rust
use rs_trafilatura::extract;

let result = extract(html)?;
println!("Content: {}", result.content_text);
println!("Title: {:?}", result.metadata.title);
println!("Author: {:?}", result.metadata.author);
println!("Page type: {:?}", result.metadata.page_type);
println!("Extraction quality: {:.2}", result.extraction_quality);
```

### Custom Options

```rust
use rs_trafilatura::{extract_with_options, Options};

let options = Options {
    include_comments: true,
    include_tables: true,
    include_images: true,
    include_links: true,
    favor_precision: true,  // Stricter filtering, less noise
    // favor_recall: true,  // More inclusive, may include some noise
    url: Some("https://example.com/article".to_string()),
    ..Options::default()
};

let result = extract_with_options(html, &options)?;
```

### Markdown Output

```rust
use rs_trafilatura::{extract_with_options, Options};

let options = Options {
    output_markdown: true,
    ..Options::default()
};

let result = extract_with_options(html, &options)?;
if let Some(markdown) = &result.content_markdown {
    println!("{}", markdown);
}
```

### Page Type Override

```rust
use rs_trafilatura::{extract_with_options, Options};
use rs_trafilatura::page_type::PageType;

let options = Options {
    page_type: Some(PageType::Product),
    ..Options::default()
};

let result = extract_with_options(html, &options)?;
```

### Working with Extracted Images

```rust
use rs_trafilatura::{extract_with_options, Options};

let options = Options {
    include_images: true,
    ..Options::default()
};

let result = extract_with_options(html, &options)?;

for image in &result.images {
    println!("URL: {}", image.src);
    println!("Filename: {}", image.filename);

    if let Some(alt) = &image.alt {
        println!("Alt text: {}", alt);
    }
    if let Some(caption) = &image.caption {
        println!("Caption: {}", caption);
    }
    if image.is_hero {
        println!("This is the hero image!");
    }
}
```

### Extracting from Bytes

For HTML with unknown encoding:

```rust
use rs_trafilatura::extract_bytes;

let html_bytes: &[u8] = /* ... */;
let result = extract_bytes(html_bytes)?;
```

## CLI

The included `extract_stdin` binary reads HTML from stdin and outputs JSON:

```bash
echo '<html><body><h1>Test</h1><p>Hello world</p></body></html>' | cargo run --bin extract_stdin

# With URL context and page type override
cat page.html | cargo run --bin extract_stdin -- --url https://example.com --page-type product
```

## Extracted Data

The `ExtractResult` struct contains:

| Field | Type | Description |
|-------|------|-------------|
| `content_text` | `String` | Main article content as plain text |
| `content_html` | `Option<String>` | Main content as HTML (if available) |
| `content_markdown` | `Option<String>` | Main content as Markdown (if `output_markdown` enabled) |
| `comments_text` | `Option<String>` | Comments section text |
| `comments_html` | `Option<String>` | Comments section HTML |
| `metadata` | `Metadata` | Extracted metadata |
| `images` | `Vec<ImageData>` | Extracted images with metadata |
| `classification_confidence` | `Option<f64>` | ML classifier confidence (0.0-1.0) |
| `extraction_quality` | `f64` | Extraction quality confidence (0.0-1.0) |

## Benchmarks

### Performance

Benchmarked on 1,497 HTML files (462 MB total) on Linux x86_64:

| Page Type | Count | ms/file | Avg Size | files/s |
|-----------|-------|---------|----------|---------|
| article | 782 | 14.1 | 225 KB | 71.0 |
| service | 164 | 22.3 | 286 KB | 44.8 |
| product | 124 | 34.9 | 718 KB | 28.7 |
| collection | 116 | 43.8 | 656 KB | 22.8 |
| forum | 113 | 29.4 | 260 KB | 34.1 |
| listing | 107 | 26.7 | 339 KB | 37.5 |
| documentation | 91 | 26.7 | 209 KB | 37.5 |
| **Overall** | **1,497** | **21.8** | **316 KB** | **45.8** |

Extraction speed scales with page size. Articles (the most common page type) process at 71 files/s.

### ScrapingHub Article Extraction Benchmark

Tested on [scrapinghub/article-extraction-benchmark](https://github.com/scrapinghub/article-extraction-benchmark) (181 article pages):

| Implementation | F1 | Precision | Recall |
|----------------|------|-----------|--------|
| **rs-trafilatura (Rust)** | **0.966** | 0.942 | **0.991** |
| go-trafilatura (Go) | 0.960 | 0.940 | 0.980 |
| trafilatura (Python) | 0.958 | 0.938 | 0.978 |

### Multi-Type Benchmark (WCEB)

Tested on the [Web Content Extraction Benchmark](https://webcontentextraction.org) ([GitHub](https://github.com/Murrough-Foley/web-content-extraction-benchmark)) — 1,497 pages across 7 page types:

| Dataset | F1 |
|---------|------|
| Development set (1,497 pages) | **0.859** |
| Held-out test set (511 pages) | **0.893** |
| + MinerU-HTML fallback (hybrid) | **0.910** |

### Per-Page-Type F1

| Page Type | Count | F1 |
|-----------|-------|------|
| Article | 793 | 0.932 |
| Documentation | 91 | 0.931 |
| Service | 165 | 0.843 |
| Forum | 113 | 0.792 |
| Collection | 117 | 0.713 |
| Listing | 99 | 0.704 |
| Product | 119 | 0.670 |

## Examples

See the [`examples/`](examples/) directory:

```bash
# Basic extraction demo
cargo run --example basic

# Markdown output
cargo run --example markdown_output

# Metadata extraction
cargo run --example metadata
```

## License

MIT OR Apache-2.0

## Citation

If you use rs-trafilatura in academic work, please cite:

```bibtex
@software{rs_trafilatura,
  title = {rs-trafilatura: Fast Web Content Extraction in Rust},
  author = {Foley, Murrough},
  url = {https://github.com/Murrough-Foley/rs-trafilatura},
  year = {2026}
}
```

## Acknowledgments

- [trafilatura](https://github.com/adbar/trafilatura) - Original Python implementation by Adrien Barbaresi
- [go-trafilatura](https://github.com/markusmobius/go-trafilatura) - Go port by Markus Mobius
- [dom_query](https://github.com/niklak/dom_query) - DOM manipulation library
