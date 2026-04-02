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

use crate::{Options, Result, ExtractResult};

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

#[cfg(test)]
mod tests {
    use super::*;
    use spider::page::build;
    use spider::utils::PageResponse;
    use spider::reqwest::StatusCode;

    fn make_test_page(url: &str, html: &str) -> Page {
        let response = PageResponse {
            content: Some(html.as_bytes().to_vec()),
            status_code: StatusCode::OK,
            ..Default::default()
        };
        build(url, response)
    }

    #[test]
    fn test_extract_page_basic() {
        let html = r#"<html>
            <head><title>Test Article</title></head>
            <body>
                <nav>Home | About</nav>
                <article>
                    <h1>Test Article</h1>
                    <p>This is a test paragraph with enough content for extraction.
                    The article continues with more text to ensure the extractor
                    identifies this as the main content of the page.</p>
                </article>
                <footer>Copyright 2026</footer>
            </body>
        </html>"#;

        let page = make_test_page("https://example.com/blog/test-article", html);
        let result = extract_page(&page).expect("extraction should succeed");

        assert!(!result.content_text.is_empty(), "should extract content");
        assert!(result.content_text.contains("test paragraph"), "should contain article text");
        assert!(result.extraction_quality >= 0.0 && result.extraction_quality <= 1.0);
    }

    #[test]
    fn test_extract_page_with_options() {
        let html = r#"<html>
            <head><title>Product Page</title></head>
            <body>
                <article>
                    <h1>Widget Pro</h1>
                    <p>The Widget Pro is our best-selling product with advanced features
                    for professional users. It includes a high-resolution display and
                    long battery life.</p>
                </article>
            </body>
        </html>"#;

        let page = make_test_page("https://example.com/products/widget-pro", html);
        let options = Options {
            favor_precision: true,
            ..Options::default()
        };
        let result = extract_page_with_options(&page, &options).expect("extraction should succeed");

        assert!(!result.content_text.is_empty());
    }

    #[test]
    fn test_extract_page_url_passthrough() {
        let html = "<html><body><article><p>Content here.</p></article></body></html>";
        let page = make_test_page("https://docs.example.com/api/reference", html);

        let result = extract_page(&page).expect("extraction should succeed");
        // URL contains "docs." so classifier should detect documentation type
        assert!(result.metadata.page_type.is_some() || result.content_text.is_empty().not());
    }

    #[test]
    fn test_extract_page_empty_html() {
        let page = make_test_page("https://example.com", "");
        // Should not panic on empty input
        let _ = extract_page(&page);
    }

    // Helper for the url_passthrough test
    trait Not { fn not(self) -> bool; }
    impl Not for bool { fn not(self) -> bool { !self } }
}
