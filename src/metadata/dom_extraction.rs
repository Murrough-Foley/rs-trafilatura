//! DOM-based Metadata Extraction
//!
//! This module provides fallback metadata extraction by searching the document DOM
//! using CSS selectors and heuristics, used when meta tags are insufficient.
//!
//! Port of go-trafilatura's metadata.go (lines 400-820).

use dom_query::{Document, Selection};
use regex::Regex;
use std::sync::LazyLock;

use crate::dom;
use crate::etree;
use crate::result::Metadata;
use crate::selector::{self, meta as meta_selectors};
use crate::Options;

// ============================================================
// REGEX PATTERNS
// ============================================================

/// Regex pattern for splitting titles by common separators
#[allow(clippy::expect_used)]
static TITLE_SEPARATOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\s*[\|–—·]\s*|\s+-\s+|\s*:\s+").expect("valid regex")
});

/// Regex pattern for splitting sitename from title
#[allow(clippy::expect_used)]
static SITENAME_SEPARATOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\s*[\|–—]\s*|\s+-\s+").expect("valid regex")
});

/// Regex pattern for email addresses
#[allow(clippy::expect_used)]
static EMAIL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\S+@\S+\.\S+").expect("valid regex")
});

/// Regex pattern for Twitter handles
#[allow(clippy::expect_used)]
static TWITTER_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"@\w+").expect("valid regex")
});

/// Regex pattern for "and X more" patterns
#[allow(clippy::expect_used)]
static MORE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\s+and\s+\d+\s+more.*$").expect("valid regex")
});

/// Regex pattern for Creative Commons licenses
#[allow(clippy::expect_used)]
static CC_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"creativecommons\.org/licenses/([a-z-]+)/").expect("valid regex")
});

// ============================================================
// TITLE EXTRACTION
// ============================================================

/// Extract title from <title> element, parsing out site suffixes.
///
/// Go equivalent: `examineTitleElement(doc)` (lines 362-398)
#[must_use]
pub fn examine_title_element(doc: &Document) -> Option<String> {
    let title_elem = doc.select("title");
    if title_elem.is_empty() {
        return None;
    }

    let title = dom::text_content(&title_elem).trim().to_string();
    if title.is_empty() {
        return None;
    }

    // Try to extract main title before separator
    // Common separators: | - – — · :
    let parts: Vec<&str> = TITLE_SEPARATOR.split(&title).collect();

    if parts.len() > 1 {
        // Usually the main title is the longest part or the first substantial part
        let main_part = parts.iter()
            .max_by_key(|p| p.len())
            .map(|s| s.trim().to_string());

        if let Some(part) = main_part {
            if part.len() > 10 {
                return Some(part);
            }
        }
    }

    Some(title)
}

/// Extract title from H1 elements or DOM selectors.
///
/// Go equivalent: `extractDomTitle(doc)` (lines 400-480)
#[must_use]
pub fn extract_dom_title(doc: &Document, original: Metadata, _opts: &Options) -> Metadata {
    let mut result = original;

    if result.title.is_some() {
        return result;
    }

    // Try H1 first
    for h1 in doc.select("h1").nodes() {
        let h1_sel = Selection::from(*h1);
        let text = etree::iter_text(&h1_sel, " ").trim().to_string();

        if !text.is_empty() && text.len() > 5 && text.len() < 200 {
            result.title = Some(text);
            return result;
        }
    }

    // Try title selectors
    for rule in meta_selectors::META_TITLE {
        if let Some(elem) = selector::query(&doc.select("body"), *rule) {
            let text = etree::iter_text(&elem, " ").trim().to_string();
            if !text.is_empty() && text.len() > 5 {
                result.title = Some(text);
                return result;
            }
        }
    }

    // Fallback to <title> element
    if let Some(title) = examine_title_element(doc) {
        result.title = Some(title);
    }

    result
}

// ============================================================
// AUTHOR EXTRACTION
// ============================================================

/// Extract author from DOM using selectors.
///
/// Go equivalent: `extractDomAuthor(doc)` (lines 482-530)
#[must_use]
pub fn extract_dom_author(doc: &Document, original: Metadata, opts: &Options) -> Metadata {
    let mut result = original;

    if result.author.is_some() {
        return result;
    }

    // Use meta author selectors
    for rule in meta_selectors::META_AUTHOR {
        if let Some(elem) = selector::query(&doc.select("body"), *rule) {
            // Check if element should be discarded
            if meta_selectors::META_AUTHOR_DISCARD.iter().any(|r| r(&elem)) {
                continue;
            }

            let text = extract_author_text(&elem);
            if !text.is_empty() && super::meta_tags::validate_metadata_name(&text) {
                result.author = normalize_author(&text, opts);
                return result;
            }
        }
    }

    result
}

/// Extract text from author element, handling nested structures.
fn extract_author_text(elem: &Selection) -> String {
    // Try direct text first
    let text = etree::iter_text(elem, " ").trim().to_string();

    // Clean up common prefixes
    text
        .strip_prefix("By ")
        .or_else(|| text.strip_prefix("by "))
        .or_else(|| text.strip_prefix("Written by "))
        .unwrap_or(&text)
        .trim()
        .to_string()
}

/// Normalize author names.
///
/// Go equivalent: `normalizeAuthors(authors, input)` (lines 742-820)
fn normalize_author(name: &str, _opts: &Options) -> Option<String> {
    let name = name.trim();

    if name.is_empty() {
        return None;
    }

    // Remove email addresses
    let name = EMAIL_PATTERN.replace_all(name, "").trim().to_string();

    // Remove Twitter handles
    let name = TWITTER_PATTERN.replace_all(&name, "").trim().to_string();

    // Remove "and X more" patterns
    let name = MORE_PATTERN.replace(&name, "").trim().to_string();

    if name.is_empty() || name.len() < 2 {
        None
    } else {
        Some(name)
    }
}

// ============================================================
// URL EXTRACTION
// ============================================================

/// Extract canonical URL from DOM.
///
/// Go equivalent: `extractDomURL(doc)` (lines 532-565)
#[must_use]
pub fn extract_dom_url(doc: &Document, original: Metadata, _opts: &Options) -> Metadata {
    let mut result = original;

    if result.url.is_some() {
        return result;
    }

    // Try canonical link
    if let Some(node) = doc.select("link[rel='canonical']").nodes().first() {
        let link = Selection::from(*node);
        if let Some(href) = dom::get_attribute(&link, "href") {
            let href = href.trim();
            if !href.is_empty() && (href.starts_with("http://") || href.starts_with("https://")) {
                result.url = Some(href.to_string());
                return result;
            }
        }
    }

    // Try og:url (already handled in meta_tags, but as fallback)
    if let Some(node) = doc.select("meta[property='og:url']").nodes().first() {
        let meta = Selection::from(*node);
        if let Some(content) = dom::get_attribute(&meta, "content") {
            let content = content.trim();
            if !content.is_empty() {
                result.url = Some(content.to_string());
                return result;
            }
        }
    }

    result
}

// ============================================================
// SITENAME EXTRACTION
// ============================================================

/// Extract site name from DOM elements or title suffix.
///
/// Go equivalent: `extractDomSitename(doc)` (lines 567-620)
#[must_use]
pub fn extract_dom_sitename(doc: &Document, original: Metadata, _opts: &Options) -> Metadata {
    let mut result = original;

    if result.sitename.is_some() {
        return result;
    }

    // Try to extract from title suffix
    // Get the raw title text (not the parsed main title)
    let title_elem = doc.select("title");
    if !title_elem.is_empty() {
        let title = dom::text_content(&title_elem).trim().to_string();

        if !title.is_empty() {
            let parts: Vec<&str> = SITENAME_SEPARATOR.split(&title).collect();
            if parts.len() > 1 {
                // Site name is usually the shorter part at the end
                if let Some(sitename) = parts.last() {
                    let sitename = sitename.trim();
                    if sitename.len() > 2 && sitename.len() < 50 {
                        result.sitename = Some(sitename.to_string());
                        return result;
                    }
                }
            }
        }
    }

    // Try meta selectors for sitename
    for rule in meta_selectors::META_SITENAME {
        if let Some(elem) = selector::query(&doc.select("body"), *rule) {
            let text = etree::iter_text(&elem, " ").trim().to_string();
            if !text.is_empty() && text.len() < 100 {
                result.sitename = Some(text);
                return result;
            }
        }
    }

    result
}

// ============================================================
// CATEGORIES AND TAGS EXTRACTION
// ============================================================

/// Extract categories from DOM.
///
/// Go equivalent: `extractDomCategories(doc)` (lines 622-680)
#[must_use]
pub fn extract_dom_categories(doc: &Document, original: Metadata, _opts: &Options) -> Metadata {
    let mut result = original;

    if !result.categories.is_empty() {
        return result;
    }

    let mut categories = Vec::new();

    // Look for category links
    for rule in meta_selectors::META_CATEGORIES {
        for node in selector::query_all(&doc.select("body"), *rule) {
            let text = etree::iter_text(&node, " ").trim().to_string();
            if !text.is_empty() && !categories.contains(&text) {
                categories.push(text);
            }

            // Limit categories
            if categories.len() >= 5 {
                break;
            }
        }
    }

    if !categories.is_empty() {
        result.categories = clean_cat_tags(categories);
    }

    result
}

/// Extract tags from DOM.
///
/// Go equivalent: `extractDomTags(doc)` (lines 682-740)
#[must_use]
pub fn extract_dom_tags(doc: &Document, original: Metadata, _opts: &Options) -> Metadata {
    let mut result = original;

    if !result.tags.is_empty() {
        return result;
    }

    let mut tags = Vec::new();

    // Look for tag links
    for rule in meta_selectors::META_TAGS {
        for node in selector::query_all(&doc.select("body"), *rule) {
            let text = etree::iter_text(&node, " ").trim().to_string();
            if !text.is_empty() && !tags.contains(&text) {
                tags.push(text);
            }

            // Limit tags
            if tags.len() >= 10 {
                break;
            }
        }
    }

    if !tags.is_empty() {
        result.tags = clean_cat_tags(tags);
    }

    result
}

/// Clean and normalize category/tag lists.
///
/// Go equivalent: `cleanCatTags(catTags)` (lines 740-760)
fn clean_cat_tags(items: Vec<String>) -> Vec<String> {
    items
        .into_iter()
        .flat_map(|s| {
            // Split on common separators
            s.split([',', ';', '/'])
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty() && s.len() < 100)
                .collect::<Vec<_>>()
        })
        .collect()
}

// ============================================================
// DATE EXTRACTION
// ============================================================

/// Extract date from DOM elements.
///
/// Searches for dates in:
/// 1. `<time>` elements with datetime attribute
/// 2. `<time>` elements with text content
/// 3. Elements with date-related classes (publish-date, post-date, etc.)
/// 4. Elements with itemprop="datePublished"
#[must_use]
pub fn extract_dom_date(doc: &Document, original: Metadata, _opts: &Options) -> Metadata {
    let mut result = original;

    if result.date.is_some() {
        return result;
    }

    // Priority 1: <time> elements with datetime attribute
    for node in doc.select("time[datetime]").nodes() {
        let el = Selection::from(*node);
        if let Some(dt_attr) = dom::get_attribute(&el, "datetime") {
            if let Some(date) = super::meta_tags::parse_meta_date(&dt_attr) {
                result.date = Some(date);
                return result;
            }
        }
    }

    // Priority 2: <time> elements with text content
    for node in doc.select("time").nodes() {
        let el = Selection::from(*node);
        let text = dom::text_content(&el).trim().to_string();
        if !text.is_empty() {
            if let Some(date) = super::meta_tags::parse_meta_date(&text) {
                result.date = Some(date);
                return result;
            }
        }
    }

    // Priority 3: Elements with date-related classes or itemprop
    let date_selectors = [
        "[class*='publish-date']",
        "[class*='date-publish']",
        "[class*='post-date']",
        "[class*='entry-date']",
        "[class*='article-date']",
        "[itemprop='datePublished']",
        "[itemprop='dateCreated']",
    ];

    for selector in date_selectors {
        for (count, node) in doc.select(selector).nodes().iter().enumerate() {
            if count >= 3 {
                break;
            }

            let el = Selection::from(*node);

            // Check datetime attribute first
            if let Some(dt_attr) = dom::get_attribute(&el, "datetime") {
                if let Some(date) = super::meta_tags::parse_meta_date(&dt_attr) {
                    result.date = Some(date);
                    return result;
                }
            }

            // Check content attribute (for meta-like elements)
            if let Some(content) = dom::get_attribute(&el, "content") {
                if let Some(date) = super::meta_tags::parse_meta_date(&content) {
                    result.date = Some(date);
                    return result;
                }
            }

            // Check text content
            let text = dom::text_content(&el).trim().to_string();
            if !text.is_empty() && text.len() < 100 {
                if let Some(date) = super::meta_tags::parse_meta_date(&text) {
                    result.date = Some(date);
                    return result;
                }
            }
        }
    }

    result
}

// ============================================================
// LICENSE EXTRACTION
// ============================================================

/// Extract license information from DOM.
///
/// Go equivalent: `extractLicense(doc)` (lines 760-820)
#[must_use]
pub fn extract_dom_license(doc: &Document, original: Metadata, _opts: &Options) -> Metadata {
    let mut result = original;

    if result.license.is_some() {
        return result;
    }

    // Check footer area first
    for selector in ["footer", ".footer", "#footer", "[class*='license']", "[class*='copyright']"] {
        for node in doc.select(selector).nodes() {
            let elem = Selection::from(*node);

            // Check links
            for link in elem.select("a").nodes() {
                let a = Selection::from(*link);
                if let Some(href) = dom::get_attribute(&a, "href") {
                    if let Some(caps) = CC_PATTERN.captures(&href) {
                        if let Some(license_type) = caps.get(1) {
                            result.license = Some(format!("CC {}", license_type.as_str().to_uppercase()));
                            return result;
                        }
                    }
                }
            }
        }
    }

    // Check rel="license" links anywhere
    for node in doc.select("a[rel='license']").nodes() {
        let a = Selection::from(*node);
        if let Some(href) = dom::get_attribute(&a, "href") {
            if let Some(caps) = CC_PATTERN.captures(&href) {
                if let Some(license_type) = caps.get(1) {
                    result.license = Some(format!("CC {}", license_type.as_str().to_uppercase()));
                    return result;
                }
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examine_title_element_simple() {
        let html = "<html><head><title>Article Title</title></head><body></body></html>";
        let doc = Document::from(html);
        let title = examine_title_element(&doc);
        assert_eq!(title, Some("Article Title".to_string()));
    }

    #[test]
    fn test_examine_title_element_with_separator() {
        let html = "<html><head><title>Article Title | Site Name</title></head><body></body></html>";
        let doc = Document::from(html);
        let title = examine_title_element(&doc);
        // Should extract the longer part
        assert!(title.unwrap().contains("Article Title"));
    }

    #[test]
    fn test_extract_dom_title_h1() {
        let html = r#"<!DOCTYPE html>
        <html>
        <head><title>Page Title | Site</title></head>
        <body>
            <h1>Main Article Heading</h1>
        </body>
        </html>"#;

        let doc = Document::from(html);
        let metadata = extract_dom_title(&doc, Metadata::default(), &Options::default());
        assert_eq!(metadata.title, Some("Main Article Heading".to_string()));
    }

    #[test]
    fn test_extract_dom_url_canonical() {
        let html = r#"<!DOCTYPE html>
        <html>
        <head>
            <link rel="canonical" href="https://example.com/article">
        </head>
        <body></body>
        </html>"#;

        let doc = Document::from(html);
        let metadata = extract_dom_url(&doc, Metadata::default(), &Options::default());
        assert_eq!(metadata.url, Some("https://example.com/article".to_string()));
    }

    #[test]
    fn test_extract_dom_sitename_from_title() {
        let html = r#"<!DOCTYPE html>
        <html>
        <head><title>Great Article - Example News</title></head>
        <body></body>
        </html>"#;

        let doc = Document::from(html);
        let metadata = extract_dom_sitename(&doc, Metadata::default(), &Options::default());
        assert_eq!(metadata.sitename, Some("Example News".to_string()));
    }

    #[test]
    fn test_extract_dom_license_cc() {
        let html = r#"<!DOCTYPE html>
        <html>
        <body>
            <footer>
                <a href="https://creativecommons.org/licenses/by-sa/4.0/">CC BY-SA</a>
            </footer>
        </body>
        </html>"#;

        let doc = Document::from(html);
        let metadata = extract_dom_license(&doc, Metadata::default(), &Options::default());
        assert_eq!(metadata.license, Some("CC BY-SA".to_string()));
    }

    #[test]
    fn test_normalize_author() {
        assert_eq!(
            normalize_author("John Smith", &Options::default()),
            Some("John Smith".to_string())
        );

        assert_eq!(
            normalize_author("John Smith john@example.com", &Options::default()),
            Some("John Smith".to_string())
        );

        assert_eq!(
            normalize_author("@johndoe", &Options::default()),
            None // Empty after removing handle
        );
    }

    #[test]
    fn test_clean_cat_tags() {
        let input = vec![
            "Technology".to_string(),
            "Science, Innovation".to_string(),
        ];
        let result = clean_cat_tags(input);
        assert_eq!(result, vec!["Technology", "Science", "Innovation"]);
    }
}
