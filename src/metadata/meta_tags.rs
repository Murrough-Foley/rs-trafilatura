//! HTML Meta Tag Extraction
//!
//! This module ports metadata extraction from go-trafilatura's metadata.go.
//! It extracts metadata from standard HTML meta tags, Open Graph tags,
//! Twitter cards, Dublin Core, and other common metadata formats.

use dom_query::{Document, Selection};
use chrono::{DateTime, Utc};
use crate::dom;
use crate::result::Metadata;
use crate::Options;

/// Examine HTML meta tags for metadata.
///
/// Go equivalent: `examineMeta(doc)` (lines 155-260)
///
/// Extracts metadata from:
/// - Standard meta tags (name/content)
/// - Open Graph tags (property/content)
/// - Twitter cards (name/content)
/// - Schema.org itemprops
/// - Dublin Core tags
#[must_use]
pub fn examine_meta(doc: &Document, original: Metadata, _opts: &Options) -> Metadata {
    let mut result = original;

    // Collect all meta tags
    for node in doc.select("meta").nodes() {
        let meta = Selection::from(*node);

        let name = dom::get_attribute(&meta, "name")
            .or_else(|| dom::get_attribute(&meta, "property"))
            .or_else(|| dom::get_attribute(&meta, "itemprop"))
            .unwrap_or_default()
            .to_lowercase();

        let content = dom::get_attribute(&meta, "content").unwrap_or_default();

        if name.is_empty() || content.is_empty() {
            continue;
        }

        // Route to appropriate handler based on name
        match name.as_str() {
            // Author
            "author" | "article:author" | "dc.creator" | "dc.contributor"
            | "byl" | "sailthru.author" | "parsely-author" => {
                if result.author.is_none() && validate_metadata_name(&content) {
                    result.author = Some(content.clone());
                }
            }

            // Title
            "og:title" | "twitter:title" | "dc.title" | "sailthru.title"
            | "parsely-title" | "title" => {
                if result.title.is_none() {
                    result.title = Some(content.clone());
                }
            }

            // Description
            "description" | "og:description" | "twitter:description"
            | "dc.description" | "excerpt" => {
                if result.description.is_none() {
                    result.description = Some(content.clone());
                }
            }

            // Site name
            "og:site_name" | "application-name" | "publisher"
            | "dc.publisher" | "twitter:site" => {
                if result.sitename.is_none() {
                    result.sitename = Some(content.clone());
                }
            }

            // URL
            "og:url" | "twitter:url" => {
                if result.url.is_none() {
                    result.url = Some(content.clone());
                }
            }

            // Image
            "og:image" | "twitter:image" | "twitter:image:src" | "thumbnail" => {
                if result.image.is_none() {
                    result.image = Some(content.clone());
                }
            }

            // Date - comprehensive list from go-trafilatura
            "article:published_time" | "article:modified_time" | "og:article:published_time"
            | "article:published" | "article.published" | "article:created" | "article.created"
            | "date" | "dc.date" | "dc.date.issued" | "dcterms.date" | "dcterms.created"
            | "datepublished" | "datemodified" | "og:updated_time"
            | "sailthru.date" | "parsely-pub-date" | "datelastpubbed"
            | "pubdate" | "publish_date" | "publishdate"
            | "timestamp" | "pdate" | "cxenseparse:recs:publishtime" => {
                if result.date.is_none() {
                    if let Some(date) = parse_meta_date(&content) {
                        result.date = Some(date);
                    }
                }
            }

            // Tags
            "article:tag" | "keywords" | "parsely-tags" | "sailthru.tags" => {
                if result.tags.is_empty() {
                    result.tags = parse_tag_list(&content);
                }
            }

            // Categories
            "article:section" | "category" | "parsely-section" => {
                if result.categories.is_empty() {
                    result.categories = parse_tag_list(&content);
                }
            }

            // Page type
            "og:type" => {
                if result.page_type.is_none() {
                    result.page_type = Some(content.clone());
                }
            }

            // Language
            "og:locale" | "language" | "dc.language" | "content-language" => {
                if result.language.is_none() {
                    // Extract primary language code
                    let lang = content.split('_').next()
                        .or_else(|| content.split('-').next())
                        .unwrap_or(&content);
                    result.language = Some(lang.to_lowercase());
                }
            }

            _ => {}
        }
    }

    // Also check <html lang="...">
    if result.language.is_none() {
        if let Some(node) = doc.select("html").nodes().first() {
            let html = Selection::from(*node);
            if let Some(lang) = dom::get_attribute(&html, "lang") {
                let lang = lang.split('-').next().unwrap_or(&lang);
                result.language = Some(lang.to_lowercase());
            }
        }
    }

    result
}

/// Extract Open Graph metadata specifically.
///
/// Go equivalent: `extractOpenGraphMeta(doc)` (lines 262-330)
///
/// This is a more focused extraction for OG tags, useful as a secondary pass.
#[must_use]
pub fn extract_open_graph(doc: &Document, original: Metadata) -> Metadata {
    let mut result = original;

    for node in doc.select("meta[property^='og:']").nodes() {
        let meta = Selection::from(*node);
        let property = dom::get_attribute(&meta, "property").unwrap_or_default();
        let content = dom::get_attribute(&meta, "content").unwrap_or_default();

        if content.is_empty() {
            continue;
        }

        match property.as_str() {
            "og:title" => {
                if result.title.is_none() {
                    result.title = Some(content);
                }
            }
            "og:description" => {
                if result.description.is_none() {
                    result.description = Some(content);
                }
            }
            "og:site_name" => {
                if result.sitename.is_none() {
                    result.sitename = Some(content);
                }
            }
            "og:url" => {
                if result.url.is_none() {
                    result.url = Some(content);
                }
            }
            "og:image" => {
                if result.image.is_none() {
                    result.image = Some(content);
                }
            }
            "og:type" => {
                if result.page_type.is_none() {
                    result.page_type = Some(content);
                }
            }
            "og:locale" => {
                if result.language.is_none() {
                    let lang = content.split('_').next().unwrap_or(&content);
                    result.language = Some(lang.to_lowercase());
                }
            }
            _ => {}
        }
    }

    result
}

/// Validate that a metadata name looks like a real author name.
///
/// Go equivalent: `validateMetadataName(name)` (lines 332-398)
///
/// Filters out:
/// - Empty or very short names
/// - Names that look like URLs
/// - Names with too many special characters
/// - Names that look like JSON
#[must_use]
pub fn validate_metadata_name(name: &str) -> bool {
    let name = name.trim();

    // Too short
    if name.len() < 2 {
        return false;
    }

    // Too long (likely not a name)
    if name.len() > 120 {
        return false;
    }

    // Must contain at least one space (first + last name)
    // unless very short
    if name.len() > 20 && !name.contains(' ') {
        return false;
    }

    // Looks like URL
    if name.starts_with("http://") || name.starts_with("https://") || name.starts_with("www.") {
        return false;
    }

    // Contains URL-like patterns
    if name.contains(".com") || name.contains(".org") || name.contains(".net") {
        return false;
    }

    // Looks like JSON
    if name.starts_with('{') || name.starts_with('[') {
        return false;
    }

    // Too many digits (likely an ID)
    let digit_count = name.chars().filter(char::is_ascii_digit).count();
    if digit_count > 3 {
        return false;
    }

    // Too many special characters
    let special_count = name.chars().filter(|c| {
        !c.is_alphanumeric() && !c.is_whitespace() && *c != '-' && *c != '\'' && *c != '.'
    }).count();

    if special_count > 2 {
        return false;
    }

    true
}

/// Parse a date string from meta tags or DOM elements.
///
/// Supports ISO 8601, RFC 3339, and common date formats.
#[must_use]
pub fn parse_meta_date(date_str: &str) -> Option<DateTime<Utc>> {
    let date_str = date_str.trim();

    // ISO 8601 with timezone
    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return Some(dt.with_timezone(&Utc));
    }

    // ISO 8601 without timezone
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt.and_utc());
    }

    // Date only
    if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        return Some(date.and_hms_opt(0, 0, 0)?.and_utc());
    }

    // Common variations
    let formats = [
        "%Y/%m/%d",
        "%d/%m/%Y",
        "%m/%d/%Y",
        "%B %d, %Y",  // January 15, 2024
        "%b %d, %Y",  // Jan 15, 2024
        "%d %B %Y",   // 15 January 2024
    ];

    for fmt in formats {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, fmt) {
            return Some(date.and_hms_opt(0, 0, 0)?.and_utc());
        }
    }

    None
}

/// Parse a comma or semicolon-separated list of tags.
fn parse_tag_list(content: &str) -> Vec<String> {
    content
        .split([',', ';'])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_meta_tags() {
        let html = r#"<!DOCTYPE html>
        <html>
        <head>
            <meta name="author" content="John Smith">
            <meta name="description" content="A test article.">
            <meta name="keywords" content="test, article, example">
        </head>
        <body></body>
        </html>"#;

        let doc = Document::from(html);
        let metadata = examine_meta(&doc, Metadata::default(), &Options::default());

        assert_eq!(metadata.author, Some("John Smith".to_string()));
        assert_eq!(metadata.description, Some("A test article.".to_string()));
        assert_eq!(metadata.tags, vec!["test", "article", "example"]);
    }

    #[test]
    fn test_open_graph_tags() {
        let html = r#"<!DOCTYPE html>
        <html>
        <head>
            <meta property="og:title" content="OG Title">
            <meta property="og:description" content="OG Description">
            <meta property="og:site_name" content="Example Site">
            <meta property="og:image" content="https://example.com/image.jpg">
            <meta property="og:url" content="https://example.com/article">
        </head>
        <body></body>
        </html>"#;

        let doc = Document::from(html);
        let metadata = examine_meta(&doc, Metadata::default(), &Options::default());

        assert_eq!(metadata.title, Some("OG Title".to_string()));
        assert_eq!(metadata.description, Some("OG Description".to_string()));
        assert_eq!(metadata.sitename, Some("Example Site".to_string()));
        assert_eq!(metadata.image, Some("https://example.com/image.jpg".to_string()));
        assert_eq!(metadata.url, Some("https://example.com/article".to_string()));
    }

    #[test]
    fn test_twitter_cards() {
        let html = r#"<!DOCTYPE html>
        <html>
        <head>
            <meta name="twitter:title" content="Twitter Title">
            <meta name="twitter:creator" content="@johndoe">
        </head>
        <body></body>
        </html>"#;

        let doc = Document::from(html);
        let metadata = examine_meta(&doc, Metadata::default(), &Options::default());

        assert_eq!(metadata.title, Some("Twitter Title".to_string()));
        // Note: twitter:creator is not currently mapped to author in the implementation
    }

    #[test]
    fn test_dublin_core() {
        let html = r#"<!DOCTYPE html>
        <html>
        <head>
            <meta name="DC.creator" content="Jane Doe">
            <meta name="DC.title" content="DC Title">
            <meta name="DC.date" content="2024-03-15">
        </head>
        <body></body>
        </html>"#;

        let doc = Document::from(html);
        let metadata = examine_meta(&doc, Metadata::default(), &Options::default());

        assert_eq!(metadata.author, Some("Jane Doe".to_string()));
        assert_eq!(metadata.title, Some("DC Title".to_string()));
        assert!(metadata.date.is_some());
    }

    #[test]
    fn test_validate_metadata_name_valid() {
        assert!(validate_metadata_name("John Smith"));
        assert!(validate_metadata_name("Jean-Pierre"));
        assert!(validate_metadata_name("O'Connor"));
        assert!(validate_metadata_name("Dr. Smith"));
    }

    #[test]
    fn test_validate_metadata_name_invalid() {
        assert!(!validate_metadata_name(""));
        assert!(!validate_metadata_name("x"));
        assert!(!validate_metadata_name("https://example.com"));
        assert!(!validate_metadata_name("user@example.com"));
        assert!(!validate_metadata_name("{\"name\": \"test\"}"));
        assert!(!validate_metadata_name("1234567890"));
    }

    #[test]
    fn test_language_from_html_lang() {
        let html = r#"<!DOCTYPE html>
        <html lang="en-US">
        <head></head>
        <body></body>
        </html>"#;

        let doc = Document::from(html);
        let metadata = examine_meta(&doc, Metadata::default(), &Options::default());

        assert_eq!(metadata.language, Some("en".to_string()));
    }

    #[test]
    fn test_date_parsing_formats() {
        assert!(parse_meta_date("2024-03-15").is_some());
        assert!(parse_meta_date("2024-03-15T10:30:00Z").is_some());
        assert!(parse_meta_date("2024-03-15T10:30:00+00:00").is_some());
        assert!(parse_meta_date("invalid date").is_none());
    }

    #[test]
    fn test_preserves_existing_metadata() {
        let html = r#"<meta name="author" content="New Author">"#;

        let original = Metadata {
            author: Some("Original Author".to_string()),
            ..Metadata::default()
        };

        let doc = Document::from(html);
        let metadata = examine_meta(&doc, original, &Options::default());

        // Should preserve original
        assert_eq!(metadata.author, Some("Original Author".to_string()));
    }
}
