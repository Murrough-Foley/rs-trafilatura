//! Content selector rules
//!
//! Port of `internal/selector/content.go`.
//! These rules identify the main content container on a web page.

use std::collections::HashSet;
use dom_query::{NodeId, Selection};
use crate::selector::utils::{contains, starts_with, lower, id, class, attr, tag};
use crate::selector::Rule;

/// Content selector rules in priority order
/// First match wins - check in order
pub static CONTENT_RULES: &[Rule] = &[
    content_rule_1,
    content_rule_2,
    content_rule_3,
    content_rule_4,
    content_rule_5,
    content_rule_6,
];

/// Rule 1: Most specific article body markers
///
/// Matches: `itemprop="articleBody"`, `class*="post-content"`, `class*="article-body"`, etc.
/// Tags: article, div, main, section (and td for storybody class)
///
/// Go: contentRule1 (lines 55-109)
#[must_use]
pub fn content_rule_1(sel: &Selection) -> bool {
    let tag = tag(sel);
    let id = id(sel);
    let class = class(sel);
    let item_prop = attr(sel, "itemprop");

    // Special case: td elements with storybody class (common in BBC and other news sites)
    // These are table-based layouts where the main content is in a td.storybody element.
    if tag == "td" {
        return contains(&lower(&id), "storybody") || contains(&lower(&class), "storybody");
    }

    // Tag filter for non-td elements
    if !matches!(tag.as_str(), "article" | "div" | "main" | "section") {
        return false;
    }

    // Pattern matching
    // Snippet body (WSJ paywall content)
    contains(&class, "snippet-body")
        || class == "post"
        || class == "entry"
        || contains(&class, "post-text")
        || contains(&class, "post_text")
        || contains(&class, "post-body")
        || contains(&class, "post-entry")
        || contains(&class, "postentry")
        || contains(&class, "post-content")
        || contains(&class, "post_content")
        || contains(&lower(&class), "postcontent")
        || contains(&class, "post_inner_wrapper")
        || contains(&class, "article-text")
        || contains(&lower(&class), "articletext")
        || contains(&id, "entry-content")
        || contains(&class, "entry-content")
        || contains(&id, "article-content")
        || contains(&class, "article-content")
        || contains(&id, "article__content")
        || contains(&class, "article__content")
        || contains(&id, "article-body")
        || contains(&class, "article-body")
        || contains(&id, "article__body")
        || contains(&class, "article__body")
        || item_prop == "articleBody"
        || contains(&lower(&id), "articlebody")
        || contains(&lower(&class), "articlebody")
        || id == "articleContent"
        || contains(&class, "ArticleContent")
        || contains(&class, "page-content")
        || contains(&class, "text-content")
        || contains(&id, "body-text")
        || contains(&class, "body-text")
        || contains(&class, "article__container")
        || contains(&id, "art-content")
        || contains(&class, "art-content")
        // Story body patterns (WorldNow CMS and others)
        || contains(&lower(&id), "storybody")
        || contains(&lower(&class), "storybody")
        // Additional body patterns found in l3s-gn1
        || contains(&id, "article_body")
        || contains(&class, "article_body")
        || contains(&id, "va-bodytext")
        || contains(&class, "va-bodytext")
        // Content body (case-insensitive)
        || lower(&id) == "contentbody"
        || contains(&lower(&class), "contentbody")
        // Blog content patterns (modern CMS and blog platforms)
        || contains(&class, "blog-content")
        || contains(&class, "blog_content")
        || contains(&lower(&class), "blogcontent")
        || contains(&class, "blogInner__content")
        || contains(&class, "blog-article-content")
        || contains(&class, "blog-post-content")
        || contains(&class, "blog_post_content")
        || contains(&class, "blog-main-content")
        || class == "only-content"
        // WYSIWYG editor content (common in CMS like WordPress, Tailwind)
        // Note: Use contains since class may have leading whitespace in HTML
        || contains(&class, "wysiwyg")
        // Next.js/React blog patterns (camelCase naming)
        || contains(&class, "blogPostBody")
        || contains(&class, "blogPostContent")
        || contains(&class, "postBody")
        || contains(&class, "postContent")
        // MediaWiki/WikiHow patterns
        || contains(&class, "mw-parser-output")
        || contains(&id, "mw-content-text")
        || contains(&class, "mw-content-text")
        || contains(&id, "bodyContent")
}

/// Rule 2: Article/Story tag
///
/// Matches `<article>` and `<story>` elements (story is used by some news sites)
///
/// Go: contentRule2 (lines 111-114) - extended to include story tag
#[must_use]
pub fn content_rule_2(sel: &Selection) -> bool {
    let t = tag(sel);
    t == "article" || t == "story"
}

/// Rule 3: Story content markers
///
/// Matches: `class*="story-content"`, `role="article"`, `class="story"`, etc.
/// Tags: article, div, main, section
///
/// Go: contentRule3 (lines 116-171)
#[must_use]
pub fn content_rule_3(sel: &Selection) -> bool {
    let tag = tag(sel);
    let id = id(sel);
    let class = class(sel);
    let role = attr(sel, "role");

    // Tag filter
    if !matches!(tag.as_str(), "article" | "div" | "main" | "section") {
        return false;
    }

    // Pattern matching
    contains(&class, "post-bodycopy")
        || contains(&class, "storycontent")
        || contains(&class, "story-content")
        || class == "postarea"
        || class == "art-postcontent"
        || contains(&class, "theme-content")
        || contains(&class, "blog-content")
        || contains(&class, "section-content")
        || contains(&class, "single-content")
        || contains(&class, "single-post")
        || contains(&class, "main-column")
        || contains(&class, "wpb_text_column")
        || starts_with(&id, "primary")
        || starts_with(&class, "article")
        || class == "text"
        || id == "article"
        || class == "cell"
        || id == "story"
        || class == "story"
        || contains(&class, "story-body")
        || contains(&id, "story-body")
        || contains(&class, "field-body")
        || contains(&lower(&class), "fulltext")
        || role == "article"
}

/// Rule 4: Generic content markers
///
/// Matches: `id="content"`, `class="content"`, `class*="main-content"`, etc.
/// Tags: article, div, main, section
///
/// Go: contentRule4 (lines 173-208)
#[must_use]
pub fn content_rule_4(sel: &Selection) -> bool {
    let tag = tag(sel);
    let id = id(sel);
    let class = class(sel);

    // Tag filter
    if !matches!(tag.as_str(), "article" | "div" | "main" | "section") {
        return false;
    }

    // Pattern matching
    contains(&id, "content-main")
        || contains(&class, "content-main")
        || contains(&class, "content_main")
        || contains(&id, "content-body")
        || contains(&class, "content-body")
        || contains(&id, "contentBody")
        || contains(&class, "content__body")
        || contains(&lower(&id), "main-content")
        || contains(&lower(&class), "main-content")
        || contains(&lower(&class), "page-content")
        || lower(&id) == "content"
        || lower(&class) == "content"
}

/// Rule 5: Main element and main markers
///
/// Matches: `<main>`, `class^="main"`, `id^="main"`, `role^="main"`
/// Tags: article, div, section, main
///
/// Go: contentRule5 (lines 210-234)
#[must_use]
pub fn content_rule_5(sel: &Selection) -> bool {
    let tag = tag(sel);
    let id = id(sel);
    let class = class(sel);
    let role = attr(sel, "role");

    // <main> always matches
    if tag == "main" {
        return true;
    }

    // Tag filter for other elements
    if !matches!(tag.as_str(), "article" | "div" | "section") {
        return false;
    }

    // Pattern matching
    starts_with(&class, "main")
        || starts_with(&id, "main")
        || starts_with(&role, "main")
}

/// Rule 6: Generic IDs/classes containing "content" (low priority fallback)
///
/// Matches: IDs or classes that contain "content" as a substring, excluding
/// obvious boilerplate patterns. Used for legacy table-based layouts that
/// have IDs like `centercontentnarrow`, `rightcontent`, etc.
///
/// This is a low-priority fallback to catch content containers that don't
/// match the more specific patterns in rules 1-5.
#[must_use]
pub fn content_rule_6(sel: &Selection) -> bool {
    let tag_name = tag(sel);
    let id_val = id(sel);
    let class_val = class(sel);
    let id_lower = lower(&id_val);
    let class_lower = lower(&class_val);

    // Tag filter - only divs and sections
    if !matches!(tag_name.as_str(), "div" | "section" | "td") {
        return false;
    }

    // Must contain "content" in ID or class
    if !id_lower.contains("content") && !class_lower.contains("content") {
        return false;
    }

    // Exclude obvious boilerplate patterns
    // Extended for modern web (2025+) with complex navigation structures
    let boilerplate_patterns = [
        "footer", "header", "sidebar", "comment", "share", "social",
        "related", "nav", "menu", "ad", "promo", "widget", "meta",
        // Note: Removed "right" and "left" - too broad, catches legitimate
        // column layouts like "col-9-left", "col_left", "content-left"
        // Modern web patterns (added for 2025+ sites)
        "dropdown", "popup", "modal", "banner", "cookie", "newsletter",
        "subscribe", "signup", "login", "signin", "cta", // Call-to-action
        "ddcards", "cards", // Navigation card patterns (like ct-ddCards)
        "featured", "trending", "popular", "recommended",
        "toolbar", "topbar", "bottombar",
    ];

    for pattern in boilerplate_patterns {
        if id_lower.contains(pattern) || class_lower.contains(pattern) {
            return false;
        }
    }

    true
}

/// Minimum text length for a content container to be considered valid.
/// If a matched element has less text than this, we try its parent.
/// Set to 1000 to better distinguish navigation menus from actual article content.
/// (Tested with 500/750 but 1000 gave best overall F1)
const MIN_CONTENT_TEXT_LEN: usize = 1000;

/// Class patterns that indicate boilerplate containers.
/// These are used with word-boundary matching to avoid false positives
/// (e.g., "js-modal-gallery" should NOT match as a modal popup).
const BOILERPLATE_CLASS_PATTERNS: &[&str] = &[
    "mega-menu", "navigation", "navbar",
    "toolbar",
    "accordion", "popup", "overlay",
    // Listing/related patterns - indicate related/recent article lists, not main content
    "listing", "latest", "recent", "related",
    // Hero section patterns - intro/header sections, not main content
    "hero",
];

/// Sidebar patterns that need position-aware matching.
/// "sidebar" should only match when it's at the START or immediately after a position word
/// (like "left-sidebar", "right-sidebar"), NOT when it's at the end of a long namespace
/// prefix (like "newspaper-x-sidebar" which is a theme class, not an actual sidebar).
const SIDEBAR_POSITION_WORDS: &[&str] = &["left", "right", "primary", "secondary", "main", "widget"];

/// Check if a class string contains a boilerplate pattern with word boundaries.
/// Uses simple heuristic: pattern must be at start/end or surrounded by non-alphanumeric chars.
/// Optimized to avoid string allocations in hot path.
fn class_contains_boilerplate(class_str: &str) -> bool {
    // Simple word-boundary patterns (pattern at word boundary)
    for pattern in BOILERPLATE_CLASS_PATTERNS {
        if has_word_boundary_match(class_str, pattern) {
            return true;
        }
    }

    // Special handling for patterns that need exact word matching
    // to avoid matching "js-modal-gallery" for "modal"
    static EXACT_WORD_PATTERNS: &[&str] = &["menu", "nav", "modal", "footer", "header", "banner"];
    for pattern in EXACT_WORD_PATTERNS {
        if has_exact_word_match(class_str, pattern) {
            return true;
        }
    }

    // Sidebar-specific matching: avoid false positives like "newspaper-x-sidebar"
    has_sidebar_match(class_str)
}

/// Check if class contains pattern with word boundaries.
#[inline]
fn has_word_boundary_match(class_str: &str, pattern: &str) -> bool {
    let class_lower = class_str.to_lowercase();
    let class_bytes = class_lower.as_bytes();
    let pattern_len = pattern.len();

    // Exact match
    if class_lower == pattern {
        return true;
    }

    // Find all occurrences and check boundaries
    let mut start = 0;
    while let Some(pos) = class_lower[start..].find(pattern) {
        let abs_pos = start + pos;
        let end_pos = abs_pos + pattern_len;

        // Check left boundary (start of string or separator)
        let left_ok = abs_pos == 0 || is_separator(class_bytes[abs_pos - 1]);

        // Check right boundary (end of string or separator)
        let right_ok = end_pos >= class_bytes.len() || is_separator(class_bytes[end_pos]);

        if left_ok && right_ok {
            return true;
        }

        start = abs_pos + 1;
        if start >= class_lower.len() {
            break;
        }
    }
    false
}

/// Check if byte is a word separator.
#[inline]
fn is_separator(b: u8) -> bool {
    matches!(b, b' ' | b'-' | b'_' | b'.')
}

/// Check for exact word match, excluding js- prefixed patterns.
#[inline]
fn has_exact_word_match(class_str: &str, pattern: &str) -> bool {
    let class_lower = class_str.to_lowercase();

    // Split into words and check each
    for word in class_lower.split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_') {
        if word.is_empty() {
            continue;
        }

        // Match if word equals pattern or starts with pattern- or pattern_
        let matches = word == pattern
            || (word.len() > pattern.len()
                && word.starts_with(pattern)
                && matches!(word.as_bytes().get(pattern.len()), Some(b'-') | Some(b'_')));

        if matches {
            // Skip js- prefixed classes (check without allocation)
            if !has_js_prefix(&class_lower, pattern) {
                return true;
            }
        }
    }
    false
}

/// Check if pattern is js-prefixed in the class string.
/// Checks for js-{pattern} or js_{pattern} anywhere in the string.
#[inline]
fn has_js_prefix(class_lower: &str, pattern: &str) -> bool {
    // Check all occurrences of js- and js_ prefixes
    for prefix in ["js-", "js_"] {
        let mut search_start = 0;
        while let Some(pos) = class_lower[search_start..].find(prefix) {
            let abs_pos = search_start + pos;
            let after_prefix = &class_lower[abs_pos + 3..];
            if after_prefix.starts_with(pattern) {
                // Verify pattern is followed by word boundary or end
                let pattern_end = pattern.len();
                if pattern_end >= after_prefix.len()
                    || !after_prefix.as_bytes()[pattern_end].is_ascii_alphanumeric()
                {
                    return true;
                }
            }
            search_start = abs_pos + 1;
            if search_start >= class_lower.len() {
                break;
            }
        }
    }
    false
}

/// Check for sidebar with position-aware matching.
#[inline]
fn has_sidebar_match(class_str: &str) -> bool {
    let class_lower = class_str.to_lowercase();

    for class_part in class_lower.split_whitespace() {
        let tokens: Vec<&str> = class_part.split(['-', '_']).collect();

        for (i, token) in tokens.iter().enumerate() {
            if *token == "sidebar" {
                // Match if first token or preceded by position word
                if tokens.len() == 1 || i == 0 {
                    return true;
                }
                if i > 0 && SIDEBAR_POSITION_WORDS.contains(&tokens[i - 1]) {
                    return true;
                }
            }
        }
    }

    false
}

/// Cache of boilerplate element NodeIds for O(1) ancestor lookup.
/// Precomputed once per document to avoid repeated DOM traversals.
struct BoilerplateCache {
    /// NodeIds of all boilerplate elements (header, nav, aside, footer, boilerplate-classed)
    boilerplate_ids: HashSet<NodeId>,
}

impl BoilerplateCache {
    /// Build the boilerplate cache by scanning the document once.
    /// EPIC-05: Combined selectors - 4 tree scans → 1 tree scan
    fn new(root: &Selection) -> Self {
        let mut boilerplate_ids = HashSet::new();

        // Find all structural boilerplate elements with single combined selector
        // Before: 4 separate tree scans for ["header", "nav", "aside", "footer"]
        // After: 1 tree scan with combined selector
        for node in root.select("header, nav, aside, footer").nodes() {
            boilerplate_ids.insert(node.id);
        }

        // Find all elements with boilerplate class patterns
        // Only scan elements that have a class attribute (much faster than select("*"))
        for node in root.select("[class]").nodes() {
            let sel = Selection::from(node.clone());
            let class_val = class(&sel);
            if class_contains_boilerplate(&class_val) {
                boilerplate_ids.insert(node.id);
            }
        }

        Self { boilerplate_ids }
    }

    /// Check if an element has a boilerplate ancestor.
    /// O(depth) with O(1) lookups instead of O(depth × patterns).
    fn is_inside_boilerplate(&self, element: &Selection) -> bool {
        use crate::dom;

        let mut current = element.parent();
        while current.length() > 0 {
            // O(1) HashSet lookup instead of pattern matching
            if let Some(node) = current.nodes().first() {
                if self.boilerplate_ids.contains(&node.id) {
                    return true;
                }
            }

            // Stop at body/html
            if let Some(tag) = dom::tag_name(&current) {
                if tag == "body" || tag == "html" {
                    break;
                }
            }

            current = current.parent();
        }
        false
    }
}

/// Check if an element contains sidebar/navigation children that suggest it's a wrapper.
/// Elements that wrap both content and sidebar should be skipped in favor of inner content.
/// EPIC-05: Combined selectors - 12 tree scans → 2 tree scans
fn contains_boilerplate_child(element: &Selection) -> bool {
    // Check for structural boilerplate tags as direct or nested children
    // Before: 2 separate queries for ["aside", "nav"]
    // After: 1 combined query
    if element.select("aside, nav").length() > 0 {
        return true;
    }

    // Check for boilerplate class patterns in children
    // These patterns indicate sidebars, social sharing, author boxes, etc.
    // Before: 10 separate queries in loop
    // After: 1 combined query
    let boilerplate_selector = "[class*='sidebar'], [class*='social'], \
        [class*='share-'], [class*='-share'], \
        [class*='author-'], [class*='sticky-'], \
        [class*='toc-'], [class*='-toc'], \
        [class*='related-'], [class*='widget']";

    if element.select(boilerplate_selector).length() > 0 {
        return true;
    }

    false
}

/// Minimum text length for a nested content element to be considered substantial.
/// Prevents false positives from image captions, metadata blocks, etc.
/// This MUST match MIN_CONTENT_TEXT_LEN to avoid situations where we skip a wrapper
/// in favor of nested elements that ultimately don't pass the content threshold.
const MIN_NESTED_CONTENT_LEN: usize = MIN_CONTENT_TEXT_LEN;

/// Check if an element has nested content elements that might be more specific.
/// Used to determine if we should skip a wrapper in favor of inner content.
/// Only returns true if nested elements have substantial content (>=MIN_NESTED_CONTENT_LEN chars).
fn has_nested_content_element(element: &Selection) -> bool {
    use crate::dom;

    // Look for nested article, main, or elements with content-indicating classes
    let nested_articles = element.select("article");
    if nested_articles.length() > 1 {
        // Check if any nested article has substantial content
        // (Skip wrapper only if there's meaningful content to fall back to)
        for i in 0..nested_articles.length() {
            if let Some(node) = nested_articles.get(i) {
                let nested = dom_query::Selection::from(*node);
                let text = dom::text_content(&nested);
                if text.trim().len() >= MIN_NESTED_CONTENT_LEN {
                    return true;
                }
            }
        }
        // No nested article has substantial content, don't skip wrapper
        return false;
    }

    // Check for content-indicating class patterns in descendants
    // EPIC-05: Combined selectors - 9+ tree scans → 1 tree scan
    // Before: Loop over 9+ content_patterns
    // After: Single combined selector
    let content_selector = "[class*='content_main'], [class*='content-main'], \
        [class*='article-content'], [class*='article_content'], \
        [class*='post-content'], [class*='post_content'], \
        [class*='story-content'], [itemprop='articleBody'], \
        [class*='blogInner__content'], [class*='blog-content'], \
        [class*='blog_content'], [class*='blogContent'], \
        [class*='entry-content']";

    let matches = element.select(content_selector);
    if matches.length() > 0 {
        // Verify at least one match has substantial content
        for i in 0..matches.length() {
            if let Some(node) = matches.get(i) {
                let matched = dom_query::Selection::from(*node);
                let text = dom::text_content(&matched);
                if text.trim().len() >= MIN_NESTED_CONTENT_LEN {
                    return true;
                }
            }
        }
    }

    false
}

/// Find content element using prioritized rules
///
/// Returns the first element matching any rule, checked in priority order.
/// This is the main entry point for content finding.
///
/// If a matched element has very little text content (e.g., a metadata-only
/// `itemprop="articleBody"` div), we check if its parent has more content
/// and use that instead.
///
/// Elements inside `<header>`, `<nav>`, or `<aside>` are skipped as they
/// typically contain navigation/boilerplate content.
///
/// Wrapper elements that contain both sidebar AND nested content are skipped
/// in favor of the more specific inner content element.
pub fn find_content<'a>(root: &Selection<'a>) -> Option<Selection<'a>> {
    use crate::selector::query_all;
    use crate::dom;

    // Precompute boilerplate element IDs once for O(1) ancestor lookups
    let boilerplate_cache = BoilerplateCache::new(root);

    for rule in CONTENT_RULES {
        // Get ALL elements matching this rule, not just the first
        let matches = query_all(root, *rule);

        for element in matches {
            // Skip elements inside header/nav/aside (O(1) lookup per ancestor)
            if boilerplate_cache.is_inside_boilerplate(&element) {
                continue;
            }

            // Skip wrapper elements that contain more specific nested content elements.
            // These are layout wrappers (e.g., outer <div> wrapping sidebar + inner
            // <article class="content_main">).
            //
            // IMPORTANT: We are MORE CONSERVATIVE with <article> tags because they are
            // strong semantic signals. Many valid articles contain sidebars as children
            // but the article itself IS the content container.
            let element_tag = tag(&element);
            let has_nested = has_nested_content_element(&element);

            // Only skip generic wrapper tags (div, section) that have boilerplate + nested content.
            // Don't skip <article> or <main> tags - they are semantic content containers.
            if matches!(element_tag.as_str(), "div" | "section") {
                let has_boilerplate = contains_boilerplate_child(&element);
                if has_boilerplate && has_nested {
                    continue;
                }
            }

            // Only skip an <article> if there's a MORE SPECIFIC nested <article> with
            // content-indicating classes (e.g., article.content_main inside plain article)
            if element_tag == "article" && class(&element).is_empty() && has_nested {
                let nested_specific = element.select("article[class*='content_main'], article[class*='content-main'], article[class*='article-body'], article[class*='article-content'], article[class*='entry-content'], article[class*='post-content']");
                if nested_specific.length() > 0 {
                    continue;
                }
            }

            // Check if this element has enough content
            let text = dom::text_content(&element);
            let text_len = text.trim().len();

            // If the matched element has minimal content, it might be a metadata
            // container (e.g., itemprop="articleBody" with just meta tags).
            // In this case, try the parent element which may contain the actual content.
            if text_len < MIN_CONTENT_TEXT_LEN {
                let parent = element.parent();
                if parent.length() > 0 {
                    // Skip if parent is also in boilerplate
                    if boilerplate_cache.is_inside_boilerplate(&parent) {
                        continue;
                    }

                    let parent_text = dom::text_content(&parent);
                    let parent_text_len = parent_text.trim().len();

                    // Use parent if it has significantly more content
                    if parent_text_len > text_len * 2 && parent_text_len >= MIN_CONTENT_TEXT_LEN {
                        return Some(parent);
                    }
                }
                // Continue searching if not enough content
                continue;
            }

            return Some(element);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom;

    #[test]
    fn test_content_rule_1_article_body() {
        let doc = dom::parse(r#"<div itemprop="articleBody">content</div>"#);
        let div = doc.select("div");
        assert!(content_rule_1(&div));
    }

    #[test]
    fn test_content_rule_1_post_content() {
        let doc = dom::parse(r#"<section class="post-content">content</section>"#);
        let section = doc.select("section");
        assert!(content_rule_1(&section));
    }

    #[test]
    fn test_content_rule_1_wrong_tag() {
        let doc = dom::parse(r#"<span class="post-content">content</span>"#);
        let span = doc.select("span");
        assert!(!content_rule_1(&span)); // span not in allowed tags
    }

    #[test]
    fn test_content_rule_2_article() {
        let doc = dom::parse("<article>content</article>");
        let article = doc.select("article");
        assert!(content_rule_2(&article));
    }

    #[test]
    fn test_content_rule_3_story_content() {
        let doc = dom::parse(r#"<div class="story-content">content</div>"#);
        let div = doc.select("div");
        assert!(content_rule_3(&div));
    }

    #[test]
    fn test_content_rule_3_role_article() {
        let doc = dom::parse(r#"<section role="article">content</section>"#);
        let section = doc.select("section");
        assert!(content_rule_3(&section));
    }

    #[test]
    fn test_content_rule_4_id_content() {
        let doc = dom::parse(r#"<div id="content">content</div>"#);
        let div = doc.select("div");
        assert!(content_rule_4(&div));
    }

    #[test]
    fn test_content_rule_4_main_content() {
        let doc = dom::parse(r#"<div class="main-content">content</div>"#);
        let div = doc.select("div");
        assert!(content_rule_4(&div));
    }

    #[test]
    fn test_content_rule_5_main_tag() {
        let doc = dom::parse("<main>content</main>");
        let main = doc.select("main");
        assert!(content_rule_5(&main));
    }

    #[test]
    fn test_content_rule_5_main_class() {
        let doc = dom::parse(r#"<div class="main-wrapper">content</div>"#);
        let div = doc.select("div");
        assert!(content_rule_5(&div));
    }

    #[test]
    fn test_find_content_priority_order() {
        // Rule 1 should match before Rule 2
        // Generate enough content to pass MIN_CONTENT_TEXT_LEN (1000 chars)
        let long_content = "This is substantial article content. ".repeat(50);
        let doc = dom::parse(&format!(r#"
            <div>
                <article>generic article with short text</article>
                <div class="post-content">{long_content}</div>
            </div>
        "#));
        let root = doc.select("div").first();

        let content = find_content(&root).unwrap();
        // Should find post-content (Rule 1) not article (Rule 2)
        assert!(class(&content).contains("post-content"));
    }

    #[test]
    fn test_find_content_fallback() {
        // Only Rule 5 should match
        // Generate enough content to pass MIN_CONTENT_TEXT_LEN (1000 chars)
        let long_content = "This is the main content of the page with substantial text. ".repeat(30);
        let doc = dom::parse(&format!(r#"
            <div>
                <main>{long_content}</main>
            </div>
        "#));
        let root = doc.select("div").first();

        let content = find_content(&root).unwrap();
        assert_eq!(tag(&content), "main");
    }

    #[test]
    fn test_find_content_skips_header() {
        // Content inside header should be skipped
        let long_content = "This is substantial article content that should be found. ".repeat(30);
        let doc = dom::parse(&format!(r#"
            <div>
                <header>
                    <div class="post-content">Navigation content in header</div>
                </header>
                <article class="post-content">{long_content}</article>
            </div>
        "#));
        let root = doc.select("div").first();

        let content = find_content(&root);
        assert!(content.is_some());
        // Should find the article, not the header content
        let found = content.unwrap();
        assert_eq!(tag(&found), "article");
    }

    #[test]
    fn test_find_content_skips_nav() {
        // Content inside nav should be skipped
        let long_content = "This is the main article content with substantial text to extract. ".repeat(25);
        let doc = dom::parse(&format!(r#"
            <div>
                <nav>
                    <div class="content">Navigation links</div>
                </nav>
                <main>{long_content}</main>
            </div>
        "#));
        let root = doc.select("div").first();

        let content = find_content(&root);
        assert!(content.is_some());
        // Should find main, not nav content
        let found = content.unwrap();
        assert_eq!(tag(&found), "main");
    }

    #[test]
    fn test_find_content_skips_wrapper_with_sidebar() {
        // Outer article wrapping sidebar + inner article should prefer inner article
        // This is the NIH pattern: outer <article> contains sidebar, inner <article class="content_main"> has content
        let long_content = "This is the main article content from the inner content container. ".repeat(25);
        let sidebar_content = "Sidebar navigation links and menu items. ".repeat(10);
        let doc = dom::parse(&format!(r#"
            <body>
                <article>
                    <div class="sidebar">{sidebar_content}</div>
                    <article class="content_main">{long_content}</article>
                </article>
            </body>
        "#));
        let root = doc.select("body");

        let content = find_content(&root);
        assert!(content.is_some());
        // Should find the inner article with content_main class, not the outer wrapper
        let found = content.unwrap();
        assert!(class(&found).contains("content_main"), "Should find inner article with content_main class");
    }
}
