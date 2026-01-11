# rs-trafilatura

Fast and accurate web content extraction in Rust.

A high-performance Rust port of [trafilatura](https://github.com/adbar/trafilatura) / [go-trafilatura](https://github.com/markusmobius/go-trafilatura), extracting clean, readable content from web pages while removing boilerplate, navigation, and advertisements.

## Features

- **Fast**: 7x faster than go-trafilatura, pure Rust with compile-time regex optimization
- **Accurate**: F1 0.966 on ScrapingHub benchmark - highest among trafilatura implementations
- **Clean Output**: Removes boilerplate, navigation, ads, and page chrome
- **Rich Metadata**: Extracts title, author, date, description, tags, and more
- **Configurable**: 20+ options to tune precision/recall tradeoff
- **Multi-format**: Supports JSON-LD, Open Graph, Dublin Core, and HTML meta tags
- **Robust**: Handles malformed HTML gracefully
- **Encoding Detection**: Automatic character encoding detection for byte input

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

    Ok(())
}
```

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
rs-trafilatura = "0.1"
```

### Feature Flags

- `readability` (default): Enables fallback extraction using Mozilla Readability algorithm via `dom_smoothie`

To disable the readability fallback:

```toml
[dependencies]
rs-trafilatura = { version = "0.1", default-features = false }
```

## Usage

### Basic Extraction

```rust
use rs_trafilatura::extract;

let result = extract(html)?;
println!("Content: {}", result.content_text);
println!("Title: {:?}", result.metadata.title);
println!("Author: {:?}", result.metadata.author);
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

### With Author Blacklist

Filter out generic author names:

```rust
let options = Options {
    author_blacklist: Some(vec![
        "Staff Writer".to_string(),
        "Editorial Team".to_string(),
        "Admin".to_string(),
    ]),
    ..Options::default()
};
```

### Working with Extracted Images

Access rich image metadata from extracted content:

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

## Extracted Data

The `ExtractResult` struct contains:

| Field | Type | Description |
|-------|------|-------------|
| `content_text` | `String` | Main article content as plain text |
| `content_html` | `Option<String>` | Main content as HTML (if available) |
| `comments_text` | `Option<String>` | Comments section text |
| `comments_html` | `Option<String>` | Comments section HTML |
| `metadata` | `Metadata` | Extracted metadata |
| `images` | `Vec<ImageData>` | Extracted images with metadata (when `include_images` is true) |

The `ImageData` struct includes:

| Field | Type | Description |
|-------|------|-------------|
| `src` | `String` | Full image URL |
| `filename` | `String` | Filename extracted from URL (query params stripped) |
| `alt` | `Option<String>` | Alt text from `<img alt="...">` attribute |
| `caption` | `Option<String>` | Caption text from `<figcaption>` element |
| `is_hero` | `bool` | Whether this is the main/hero image |

The `Metadata` struct includes:

| Field | Type | Description |
|-------|------|-------------|
| `title` | `Option<String>` | Article title |
| `author` | `Option<String>` | Author name |
| `date` | `Option<String>` | Publication date |
| `description` | `Option<String>` | Article description/summary |
| `sitename` | `Option<String>` | Website name |
| `url` | `Option<String>` | Canonical URL |
| `hostname` | `Option<String>` | Domain name |
| `image` | `Option<String>` | Featured image URL |
| `language` | `Option<String>` | Content language |
| `categories` | `Vec<String>` | Article categories |
| `tags` | `Vec<String>` | Article tags |
| `license` | `Option<String>` | Content license |
| `page_type` | `Option<String>` | Page type (article, blog, etc.) |

## Benchmarks

### Performance

Benchmarked on 2,339 HTML files (669 MB total) on Linux x86_64:

| Implementation | Total Time | Per File | Throughput |
|----------------|------------|----------|------------|
| **rs-trafilatura (Rust)** | 39.3s | 16.8ms | 59.6 files/s |
| go-trafilatura (Go) | 282.3s | 120.7ms | 8.3 files/s |

**rs-trafilatura is 7.2x faster than go-trafilatura.**

### Accuracy Comparison

Tested on 1,193 modern web pages with AI-generated ground truth:

| Implementation | Precision | Recall | F1 Score | Title Match |
|----------------|-----------|--------|----------|-------------|
| **rs-trafilatura (Rust)** | 0.897 | **0.938** | **0.899** | **61.2%** |
| trafilatura (Python) | **0.907** | 0.921 | 0.897 | 0.0% |
| go-trafilatura (Go) | 0.898 | 0.924 | 0.896 | 55.3% |

All three implementations achieve comparable accuracy (~0.90 F1), with the Rust version having the highest recall and best title extraction.

### ScrapingHub Article Extraction Benchmark

Tested on [scrapinghub/article-extraction-benchmark](https://github.com/scrapinghub/article-extraction-benchmark) (181 pages):

| Implementation | F1 | Precision | Recall | Accuracy |
|----------------|------|-----------|--------|----------|
| **rs-trafilatura (Rust)** | **0.966** | 0.942 | **0.991** | 0.227 |
| go-trafilatura (Go) | 0.960 | 0.940 | 0.980 | 0.287 |
| trafilatura (Python) | 0.958 | 0.938 | 0.978 | 0.293 |

rs-trafilatura achieves the **highest F1 score** and **highest recall** among all trafilatura implementations on this benchmark.

## Examples

See the [`examples/`](examples/) directory:

```bash
# Basic extraction demo
cargo run --example basic

# Run extraction benchmark
cargo run --release --example benchmark_extract
```

## Evaluation

To evaluate extraction quality:

```bash
./scripts/evaluate.sh
```

This runs the extraction benchmark and computes precision, recall, and F-score metrics.

## Roadmap

Planned features for future releases:

- **Markdown Output**: Option to output extracted content as Markdown instead of plain text
- **Link Preservation**: Feature flags to preserve links in extracted content
  - `include_internal_links`: Keep links to the same domain
  - `include_external_links`: Keep links to external domains
- **`html-cleaning` Crate**: Extract HTML cleaning utilities into a standalone crate for reuse in other projects

Contributions welcome! See [issues](https://github.com/Murrough-Foley/rs-trafilatura/issues) for discussion.

## License

MIT OR Apache-2.0

## Acknowledgments

- [trafilatura](https://github.com/adbar/trafilatura) - Original Python implementation by Adrien Barbaresi
- [go-trafilatura](https://github.com/markusmobius/go-trafilatura) - Go port by Markus Mobius
- [dom_query](https://github.com/niklak/dom_query) - DOM manipulation library
- [dom_smoothie](https://github.com/niklak/dom_smoothie) - Readability fallback
