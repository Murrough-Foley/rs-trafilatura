//! Integration with the [spider](https://crates.io/crates/spider) web crawler.
//!
//! Enable with the `spider` feature flag:
//!
//! ```toml
//! [dependencies]
//! rs-trafilatura = { version = "0.2", features = ["spider"] }
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use spider::website::Website;
//! use rs_trafilatura::spider_integration::extract_page;
//!
//! # async fn example() {
//! let mut website = Website::new("https://example.com");
//! website.crawl().await;
//!
//! for page in website.get_pages().unwrap_or_default().iter() {
//!     if let Ok(result) = extract_page(page) {
//!         println!("{}: {}", result.metadata.title.unwrap_or_default(), result.extraction_quality);
//!     }
//! }
//! # }
//! ```

use spider::page::Page;

use crate::{ExtractResult, Options, Result};

/// Extracts main content from a spider [`Page`] using default options.
///
/// The page URL is automatically passed to the extraction pipeline for
/// URL-based page type classification.
pub fn extract_page(page: &Page) -> Result<ExtractResult> {
    extract_page_with_options(page, &Options::default())
}

/// Extracts main content from a spider [`Page`] with custom options.
///
/// If `options.url` is `None`, the page URL is used automatically.
pub fn extract_page_with_options(page: &Page, options: &Options) -> Result<ExtractResult> {
    let html = page.get_html_bytes_u8();
    let mut opts = options.clone();
    if opts.url.is_none() {
        opts.url = Some(page.get_url().to_string());
    }
    crate::extract_bytes_with_options(html, &opts)
}
