//! Metadata extraction module.
//!
//! This module provides functions for extracting metadata from HTML documents,
//! including JSON-LD parsing, HTML meta tags, Open Graph, and other sources.

pub mod dom_extraction;
pub mod json_ld;
pub mod meta_tags;

use dom_query::Document;
use crate::result::Metadata;
use crate::url_utils;
use crate::Options;

pub use dom_extraction::{
    examine_title_element, extract_dom_author, extract_dom_categories,
    extract_dom_license, extract_dom_sitename, extract_dom_tags,
    extract_dom_title, extract_dom_url,
};
pub use json_ld::extract_json_ld;
pub use meta_tags::{examine_meta, extract_open_graph, validate_metadata_name};

/// Extract all metadata from a document.
///
/// Go equivalent: `extractMetadata(doc, opts)` (metadata.go lines 70-153)
///
/// Orchestrates metadata extraction from multiple sources:
/// 1. JSON-LD (Schema.org structured data)
/// 2. HTML meta tags (og:, twitter:, etc.)
/// 3. DOM extraction (selectors and heuristics)
///
/// # Arguments
/// * `doc` - The HTML document
/// * `opts` - Extraction options (includes author blacklist, URL)
///
/// # Returns
/// * Complete metadata with all available fields filled
#[must_use]
pub fn extract_metadata(doc: &Document, opts: &Options) -> Metadata {
    let mut metadata = Metadata::default();

    // Set URL from options if provided
    if let Some(ref url) = opts.url {
        metadata.url = Some(url.clone());
        metadata.hostname = url_utils::extract_hostname(url);
    }

    // 1. Extract from JSON-LD (highest priority for structured data)
    metadata = json_ld::extract_json_ld(doc, metadata, opts);

    // 2. Extract from HTML meta tags
    metadata = meta_tags::examine_meta(doc, metadata, opts);

    // 3. Extract from DOM (fallback for missing fields)
    metadata = dom_extraction::extract_dom_title(doc, metadata, opts);
    metadata = dom_extraction::extract_dom_author(doc, metadata, opts);
    metadata = dom_extraction::extract_dom_date(doc, metadata, opts);
    metadata = dom_extraction::extract_dom_url(doc, metadata, opts);
    metadata = dom_extraction::extract_dom_sitename(doc, metadata, opts);
    metadata = dom_extraction::extract_dom_categories(doc, metadata, opts);
    metadata = dom_extraction::extract_dom_tags(doc, metadata, opts);
    metadata = dom_extraction::extract_dom_license(doc, metadata, opts);

    // 4. Post-processing
    metadata = post_process_metadata(metadata, opts);

    // 5. Apply author blacklist
    if let Some(ref author) = metadata.author {
        if is_blacklisted_author(author, opts) {
            metadata.author = None;
        }
    }

    // 6. Ensure hostname is set if we have a URL
    if metadata.hostname.is_none() {
        if let Some(ref url) = metadata.url {
            metadata.hostname = url_utils::extract_hostname(url);
        }
    }

    metadata
}

/// Post-process metadata to clean and validate.
fn post_process_metadata(mut metadata: Metadata, _opts: &Options) -> Metadata {
    // Trim all string fields
    if let Some(ref mut title) = metadata.title {
        *title = title.trim().to_string();
        if title.is_empty() {
            metadata.title = None;
        }
    }

    if let Some(ref mut author) = metadata.author {
        *author = author.trim().to_string();
        if author.is_empty() {
            metadata.author = None;
        }
    }

    if let Some(ref mut description) = metadata.description {
        *description = description.trim().to_string();
        if description.is_empty() {
            metadata.description = None;
        }
    }

    if let Some(ref mut sitename) = metadata.sitename {
        *sitename = sitename.trim().to_string();
        if sitename.is_empty() {
            metadata.sitename = None;
        }
    }

    // Clean categories and tags
    metadata.categories = metadata.categories
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    metadata.tags = metadata.tags
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Validate URL
    if let Some(ref url) = metadata.url {
        let (_, is_valid) = url_utils::validate_url(url, None);
        if !is_valid && !url.starts_with('/') {
            metadata.url = None;
        }
    }

    // Validate image URL
    if let Some(ref image) = metadata.image {
        let (_, is_valid) = url_utils::validate_url(image, None);
        if !is_valid && !image.starts_with('/') && !image.starts_with("data:") {
            metadata.image = None;
        }
    }

    metadata
}

/// Check if an author name is in the blacklist.
///
/// Go equivalent: `removeBlacklistedAuthors(current, opts)` (metadata.go lines 822-850)
fn is_blacklisted_author(author: &str, opts: &Options) -> bool {
    if let Some(ref blacklist) = opts.author_blacklist {
        let author_lower = author.to_lowercase();
        for blocked in blacklist {
            if author_lower.contains(&blocked.to_lowercase()) {
                return true;
            }
        }
    }
    false
}

/// Light metadata extraction (title and date only).
///
/// Used for performance-sensitive scenarios where full metadata isn't needed.
#[must_use]
pub fn extract_metadata_light(doc: &Document, opts: &Options) -> Metadata {
    let mut metadata = Metadata::default();

    // URL from options
    if let Some(ref url) = opts.url {
        metadata.url = Some(url.clone());
        metadata.hostname = url_utils::extract_hostname(url);
    }

    // Title from meta tags or DOM
    metadata = meta_tags::examine_meta(doc, metadata, opts);
    if metadata.title.is_none() {
        metadata = dom_extraction::extract_dom_title(doc, metadata, opts);
    }

    metadata
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_metadata_priority() {
        // JSON-LD should take priority
        let html = r#"<!DOCTYPE html>
        <html>
        <head>
            <meta property="og:title" content="OG Title">
            <script type="application/ld+json">
            {"@type": "Article", "headline": "JSON-LD Title"}
            </script>
        </head>
        <body><h1>DOM Title</h1></body>
        </html>"#;

        let doc = Document::from(html);
        let metadata = extract_metadata(&doc, &Options::default());

        // JSON-LD headline should win
        assert_eq!(metadata.title, Some("JSON-LD Title".to_string()));
    }

    #[test]
    fn test_extract_metadata_fallback_chain() {
        // Test fallback when higher priority sources are empty
        let html = r#"<!DOCTYPE html>
        <html>
        <head>
            <meta property="og:description" content="OG Description">
        </head>
        <body>
            <h1>Article Title</h1>
        </body>
        </html>"#;

        let doc = Document::from(html);
        let metadata = extract_metadata(&doc, &Options::default());

        // Title from DOM, description from meta
        assert_eq!(metadata.title, Some("Article Title".to_string()));
        assert_eq!(metadata.description, Some("OG Description".to_string()));
    }

    #[test]
    fn test_extract_metadata_with_url_option() {
        let html = "<html><body></body></html>";

        let opts = Options {
            url: Some("https://example.com/article".to_string()),
            ..Options::default()
        };

        let doc = Document::from(html);
        let metadata = extract_metadata(&doc, &opts);

        assert_eq!(metadata.url, Some("https://example.com/article".to_string()));
        assert_eq!(metadata.hostname, Some("example.com".to_string()));
    }

    #[test]
    fn test_author_blacklist() {
        let html = r#"<html>
        <head><meta name="author" content="Staff Writer"></head>
        <body></body>
        </html>"#;

        let opts = Options {
            author_blacklist: Some(vec!["Staff Writer".to_string()]),
            ..Options::default()
        };

        let doc = Document::from(html);
        let metadata = extract_metadata(&doc, &opts);

        // Author should be filtered out
        assert!(metadata.author.is_none());
    }

    #[test]
    fn test_post_process_trims_fields() {
        let metadata = Metadata {
            title: Some("  Spaced Title  ".to_string()),
            author: Some(String::new()),  // Empty after trim
            categories: vec!["cat1".to_string(), String::new(), "cat2".to_string()],
            ..Metadata::default()
        };

        let result = post_process_metadata(metadata, &Options::default());

        assert_eq!(result.title, Some("Spaced Title".to_string()));
        assert!(result.author.is_none());
        assert_eq!(result.categories, vec!["cat1", "cat2"]);
    }

    #[test]
    fn test_is_blacklisted_author() {
        let opts = Options {
            author_blacklist: Some(vec![
                "staff".to_string(),
                "admin".to_string(),
            ]),
            ..Options::default()
        };

        assert!(is_blacklisted_author("Staff Writer", &opts));
        assert!(is_blacklisted_author("Site Admin", &opts));
        assert!(!is_blacklisted_author("John Smith", &opts));
    }

    #[test]
    fn test_extract_metadata_light() {
        let html = r#"<!DOCTYPE html>
        <html>
        <head>
            <meta property="og:title" content="Article Title">
            <meta name="author" content="John Doe">
        </head>
        <body></body>
        </html>"#;

        let doc = Document::from(html);
        let metadata = extract_metadata_light(&doc, &Options::default());

        // Should have title but not full metadata extraction
        assert_eq!(metadata.title, Some("Article Title".to_string()));
    }
}
