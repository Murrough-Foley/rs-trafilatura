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
/// Decode common HTML entities in text.
fn decode_html_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#8211;", "\u{2013}") // en dash
        .replace("&#8212;", "\u{2014}") // em dash
        .replace("&#8217;", "\u{2019}") // right single quote
        .replace("&#8216;", "\u{2018}") // left single quote
        .replace("&#8220;", "\u{201c}") // left double quote
        .replace("&#8221;", "\u{201d}") // right double quote
        .replace("&bull;", "\u{2022}")  // bullet
        .replace("&#187;", "\u{00bb}")  // »
        .replace("&raquo;", "\u{00bb}") // »
        .replace("&ndash;", "\u{2013}") // en dash
        .replace("&mdash;", "\u{2014}") // em dash
        .replace("&rsquo;", "\u{2019}") // right single quote
        .replace("&lsquo;", "\u{2018}") // left single quote
}

/// Strip site name suffix (or prefix) from a title.
///
/// Common patterns: "Article Title | Site Name", "Article Title - Site Name"
/// Uses heuristics: the suffix/prefix is typically short (≤5 words) while the
/// actual title is substantial (>10 chars). If `sitename` is available, it's
/// used to confirm the suffix is indeed a site name.
fn strip_site_suffix(title: &str, sitename: Option<&str>) -> String {
    // Separators to check — ordered by specificity
    let separators: &[&str] = &[" | ", " \u{2022} ", " \u{00bb} ", " - ", " \u{2013} ", " \u{2014} "];

    for sep in separators {
        // Try all split points for this separator (handles chained suffixes)
        let positions: Vec<usize> = title.match_indices(sep).map(|(i, _)| i).collect();

        for &pos in &positions {
            let before = title[..pos].trim();
            let after = title[pos + sep.len()..].trim();

            // If one side matches the known site name, strip it
            if let Some(site) = sitename {
                let site_lower = site.to_lowercase();
                if after.to_lowercase().starts_with(&site_lower) && !before.is_empty() {
                    return before.to_string();
                }
                if before.to_lowercase() == site_lower && !after.is_empty() {
                    return after.to_string();
                }
            }

            let before_words = before.split_whitespace().count();
            let after_words = after.split_whitespace().count();

            // Suffix pattern: short tail (≤5 words, <35 chars) after a substantial title
            if after_words <= 5 && after.len() < 35 && before.len() > 10 {
                return before.to_string();
            }

            // Prefix pattern: short head before a substantially longer title
            if before_words <= 2 && before.len() < 20 && after.len() > before.len() {
                return after.to_string();
            }
        }
    }

    title.to_string()
}

fn post_process_metadata(mut metadata: Metadata, _opts: &Options) -> Metadata {
    // Trim and clean title
    if let Some(ref mut title) = metadata.title {
        *title = title.trim().to_string();
        // Decode HTML entities that may appear in meta tag content
        *title = decode_html_entities(title);
        // Strip site name suffixes (e.g., "Article | Site Name" → "Article")
        *title = strip_site_suffix(title, metadata.sitename.as_deref());
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

    // ==================== strip_site_suffix tests ====================

    #[test]
    fn test_strip_suffix_pipe() {
        assert_eq!(
            strip_site_suffix("What is Cloud Computing? | Google Cloud", None),
            "What is Cloud Computing?"
        );
    }

    #[test]
    fn test_strip_suffix_dash() {
        assert_eq!(
            strip_site_suffix("10 Easy Steps for Interior Design - Tarkett", None),
            "10 Easy Steps for Interior Design"
        );
    }

    #[test]
    fn test_strip_suffix_em_dash() {
        assert_eq!(
            strip_site_suffix("Wedding Planning Tools — The Knot", None),
            "Wedding Planning Tools"
        );
    }

    #[test]
    fn test_strip_prefix_pattern() {
        assert_eq!(
            strip_site_suffix("BBC | World News Today Is Happening", None),
            "World News Today Is Happening"
        );
    }

    #[test]
    fn test_no_strip_subtitle() {
        // Both sides are long — the dash is a subtitle separator, not a site name
        let title = "The Complete Guide - Everything You Need to Know About Rust";
        assert_eq!(strip_site_suffix(title, None), title);
    }

    #[test]
    fn test_no_separator() {
        assert_eq!(strip_site_suffix("Simple Title", None), "Simple Title");
    }

    #[test]
    fn test_strip_with_known_sitename() {
        assert_eq!(
            strip_site_suffix("Article Title | Example Site", Some("Example Site")),
            "Article Title"
        );
    }

    #[test]
    fn test_strip_prefix_with_known_sitename() {
        assert_eq!(
            strip_site_suffix("Example Site | Article Title Here", Some("Example Site")),
            "Article Title Here"
        );
    }

    #[test]
    fn test_strip_og_title_with_suffix() {
        // Simulates og:title going through post_process_metadata
        let html = r#"<!DOCTYPE html>
        <html>
        <head>
            <meta property="og:title" content="Article Title | Site Name">
            <meta property="og:site_name" content="Site Name">
        </head>
        <body><h1>Article Title</h1></body>
        </html>"#;

        let doc = Document::from(html);
        let metadata = extract_metadata(&doc, &Options::default());
        assert_eq!(metadata.title, Some("Article Title".to_string()));
    }

    #[test]
    fn test_h1_preferred_when_contained_in_title() {
        // <title> = "Cloud Computing | Google Cloud", <h1> = "Cloud Computing"
        // H1 is contained in <title>, so prefer H1
        let html = r#"<!DOCTYPE html>
        <html>
        <head><title>Cloud Computing | Google Cloud</title></head>
        <body><h1>Cloud Computing</h1></body>
        </html>"#;

        let doc = Document::from(html);
        let metadata = extract_metadata(&doc, &Options::default());
        assert_eq!(metadata.title, Some("Cloud Computing".to_string()));
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
