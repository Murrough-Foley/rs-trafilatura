//! Compiled regex patterns and CSS selectors for content extraction.
//!
//! All patterns are compiled once at startup using `LazyLock` for efficiency.
//! Patterns are organized by their purpose in the extraction pipeline.

#![allow(clippy::expect_used)]
#![allow(dead_code)]

use std::sync::LazyLock;

use regex::Regex;

// =============================================================================
// Boilerplate Detection Patterns
// =============================================================================

/// Matches class/id names indicating navigation elements.
/// Note: We match site-header/site-footer specifically but NOT generic "header"/"footer"
/// because compound classes like "article-header" are content, not navigation.
///
/// IMPORTANT: "nav" patterns use word boundaries or position anchors to avoid matching
/// layout containers like "usa-in-page-nav-container" where "nav" appears in the middle
/// of a compound name. We match:
/// - `nav` at word boundary (start or end of token)
/// - `navbar`, `navigation` (nav at start)
/// - `main-nav`, `site-nav`, `top-nav` (nav at end after separator)
///
/// But NOT: `usa-in-page-nav-container`, `in-page-nav-wrapper` (nav in middle).
/// Note: "sidebar" removed - handled separately with position-aware matching
/// to avoid false positives like "newspaper-x-sidebar" (theme namespace, not actual sidebar).
pub static NAVIGATION_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    // Note: \bmenu\b uses word boundary to avoid matching "contextmenu" (CSS right-click property)
    // while still matching "menu", "main-menu", "nav-menu", "menu-item", etc.
    Regex::new(
        r"(?i)(^nav$|^nav[-_]|[-_]nav$|navbar|navigation|top[-_]?nav|main[-_]?menu|site[-_]?nav|\bmenu\b|site[-_]?footer|site[-_]?header|page[-_]?header|page[-_]?footer|breadcrumb(?:s)?|crumb(?:s)?)",
    )
    .expect("NAVIGATION_CLASS regex")
});

/// Matches class/id names indicating advertisement elements.
pub static ADVERTISEMENT_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(ad|ads|advert|advertisement|sponsor|sponsored|promo)$")
        .expect("ADVERTISEMENT_CLASS regex")
});

/// Matches class/id names indicating boilerplate content.
/// Note: "footer" uses word boundary to avoid matching `article-footer` type classes
/// CAUTION: Be careful not to match content container classes like "article-inner-content-breaking-news"
/// Fixed: login/signin/signup now use word boundaries to avoid matching "blogInner" etc.
/// NOTE: "sidebar" removed from regex - handled separately with position-aware matching
/// to avoid false positives like "newspaper-x-sidebar" (theme namespace, not actual sidebar)
/// NOTE: "\bauthor\b" removed - handled separately to avoid matching WordPress author taxonomy
/// classes like "author-risc-v-international-staff" (author name, not author box/bio section)
pub static BOILERPLATE_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    // NOTE: Changed `entry[-_]?cat` to `entry[-_]?cats?\b` and `post[-_]?cat` to `post[-_]?cats?\b`
    // to avoid false positives with WordPress taxonomy classes like `entry-category-game_design`
    // which indicate the article's category, not a category widget/sidebar section.
    // NOTE: "widget" is handled separately in is_boilerplate() to exclude Elementor content widgets
    // (elementor-widget-text-editor, elementor-widget-container are content containers)
    Regex::new(
        r"(?i)(comment|shar(?:e|ing)|social|related|recommend(?:ed)?|\bfooter\b|site[-_]?footer|\bwell\b|copyright|legal|disclaimer|more[-_]?from|you[-_]?may[-_]?like|taboola|outbrain|mgid|revcontent|zergnet|cookie[-_]?consent|privacy[-_]?consent|gdpr[-_]?consent|cookie[-_]?notice|privacy[-_]?notice|cookie[-_]?banner|consent[-_]?banner|\blogin\b|\bsignin\b|\bsign[-_]?in\b|\bsignup\b|\bsign[-_]?up\b|\bsubscribe\b|subscription|newsletter|snippet[-_]?login|snippet[-_]?action|trending|popular|most[-_]?read|top[-_]?stories|\bbyline\b|article[-_]byline|timestamp|dateline|print[-_]?header|photo[-_]?credit|img[-_]?credit|image[-_]?credit|\bcredit\b|caption|penci[-_]?cat|cat[-_]?name|post[-_]?cats?\b|entry[-_]?cats?\b|dpsp[-_]|addtoany|shareaholic|share[-_]?btn|social[-_]?btn|crumb|post[-_]?meta|entry[-_]?meta|meta[-_]?info|tag[-_]?cloud|category[-_]?list|filed[-_]?under|posted[-_]?in|wabtn|coauthor|pdf[-_]?link|article[-_]?info|story[-_]?info)",
    )
    .expect("BOILERPLATE_CLASS regex")
});

// =============================================================================
// Content Identification Patterns
// =============================================================================

/// Matches class/id names likely to contain main content.
pub static CONTENT_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(article|content|post|entry|story|text|body|main)").expect("CONTENT_CLASS regex")
});

/// Matches class/id names indicating article content.
pub static ARTICLE_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(article|post|entry|story|news|blog)").expect("ARTICLE_CLASS regex")
});

pub static COMMENT_CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(comment|comments|comment[-_]?list|respond|reply|replies|discussion|disqus|fb[-_]?comments)\b")
        .expect("COMMENT_CLASS regex")
});

pub static COMMENT_ID: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(comments|comment-section|disqus_thread|respond|discussion)$")
        .expect("COMMENT_ID regex")
});

// =============================================================================
// Metadata Extraction Patterns
// =============================================================================

/// Matches author patterns in text.
pub static AUTHOR_TEXT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:by|author|written by|posted by)\s*:?\s*([^,\n]+)").expect("AUTHOR_TEXT regex")
});

/// Matches date patterns in various formats.
pub static DATE_TEXT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(\d{4}[-/]\d{1,2}[-/]\d{1,2}|\d{1,2}[-/]\d{1,2}[-/]\d{4}|\w+\s+\d{1,2},?\s+\d{4})",
    )
    .expect("DATE_TEXT regex")
});

// =============================================================================
// Text Cleaning Patterns
// =============================================================================

/// Matches multiple whitespace characters for normalization.
pub static WHITESPACE_NORMALIZE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\s+").expect("WHITESPACE_NORMALIZE regex")
});

/// Matches leading/trailing whitespace on lines.
pub static LINE_WHITESPACE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]+|[ \t]+$").expect("LINE_WHITESPACE regex")
});

/// Matches multiple consecutive newlines.
pub static MULTIPLE_NEWLINES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\n{3,}").expect("MULTIPLE_NEWLINES regex")
});

/// Matches common separators used between article title and site name.
pub static TITLE_SEPARATOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\s*[\|–—\-:]\s*").expect("TITLE_SEPARATOR regex")
});

// =============================================================================
// CSS Selectors (as strings for use with scraper)
// =============================================================================

/// Selector for article elements.
pub const ARTICLE_SELECTOR: &str = "article, [role='article'], .article, .post, .entry";

/// Selector for main content areas.
pub const MAIN_SELECTOR: &str = "main, [role='main'], #main, .main, #content, .content";

/// Selector for title elements.
pub const TITLE_SELECTOR: &str = "title, h1, [class*='title'], [id*='title']";

/// Selector for author metadata.
pub const AUTHOR_SELECTOR: &str =
    "[rel='author'], .author, .byline, [class*='author'], [itemprop='author']";

/// Selector for date metadata.
pub const DATE_SELECTOR: &str =
    "time, [datetime], .date, [class*='date'], [itemprop='datePublished']";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_class_matches_nav_elements() {
        assert!(NAVIGATION_CLASS.is_match("main-nav"));
        assert!(NAVIGATION_CLASS.is_match("sidebar-menu"));
        assert!(NAVIGATION_CLASS.is_match("site-footer"));
        assert!(!NAVIGATION_CLASS.is_match("article-content"));
    }

    #[test]
    fn content_class_matches_article_elements() {
        assert!(CONTENT_CLASS.is_match("article-body"));
        assert!(CONTENT_CLASS.is_match("post-content"));
        assert!(CONTENT_CLASS.is_match("main-text"));
        assert!(!CONTENT_CLASS.is_match("sidebar-widget"));
    }

    #[test]
    fn whitespace_normalize_collapses_spaces() {
        let result = WHITESPACE_NORMALIZE.replace_all("hello   world", " ");
        assert_eq!(result, "hello world");
    }
}
