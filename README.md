# rs-trafilatura

Fast and accurate web content extraction in Rust.

A high-performance Rust port of [trafilatura](https://github.com/adbar/trafilatura) / [go-trafilatura](https://github.com/markusmobius/go-trafilatura), extracting clean, readable content from web pages while removing boilerplate, navigation, and advertisements.

## Features

- **Fast**: 7x faster than go-trafilatura, pure Rust with compile-time regex optimization
- **Accurate**: F1 0.860 on a 1,502-page multi-type benchmark
- **Page Type Classification**: ML classifier (Random Forest, 200 trees, 163 features) detects 7 page types: article, forum, product, collection, listing, documentation, service
- **Per-Type Extraction**: Specialized extraction profiles tuned for each page type
- **Extraction Confidence**: Quality scoring (0.0-1.0) for each extraction, enabling hybrid pipelines with LLM fallback
- **Markdown Output**: GitHub Flavored Markdown output preserving headings, lists, tables, formatting
- **Rich Metadata**: Extracts title, author, date, description, tags, and more from JSON-LD, Open Graph, Dublin Core, and HTML meta tags
- **Configurable**: 20+ options to tune precision/recall tradeoff
- **Robust**: Handles malformed HTML gracefully with automatic encoding detection

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
rs-trafilatura = { git = "https://github.com/Murrough-Foley/rs-trafilatura" }
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

Benchmarked on 2,339 HTML files (669 MB total) on Linux x86_64:

| Implementation | Total Time | Per File | Throughput |
|----------------|------------|----------|------------|
| **rs-trafilatura (Rust)** | 39.3s | 16.8ms | 59.6 files/s |
| go-trafilatura (Go) | 282.3s | 120.7ms | 8.3 files/s |

**rs-trafilatura is 7.2x faster than go-trafilatura.**

### ScrapingHub Article Extraction Benchmark

Tested on [scrapinghub/article-extraction-benchmark](https://github.com/scrapinghub/article-extraction-benchmark) (181 article pages):

| Implementation | F1 | Precision | Recall |
|----------------|------|-----------|--------|
| **rs-trafilatura (Rust)** | **0.966** | 0.942 | **0.991** |
| go-trafilatura (Go) | 0.960 | 0.940 | 0.980 |
| trafilatura (Python) | 0.958 | 0.938 | 0.978 |

### Multi-Type Benchmark

Tested on 1,502 pages across 7 page types (articles, forums, products, collections, listings, documentation, services):

| Implementation | F1 |
|----------------|------|
| **rs-trafilatura** | **0.860** |

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
  year = {2025}
}
```

## Acknowledgments

- [trafilatura](https://github.com/adbar/trafilatura) - Original Python implementation by Adrien Barbaresi
- [go-trafilatura](https://github.com/markusmobius/go-trafilatura) - Go port by Markus Mobius
- [dom_query](https://github.com/niklak/dom_query) - DOM manipulation library
