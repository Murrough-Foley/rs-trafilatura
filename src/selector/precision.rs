//! Precision, Teaser & Image Discard Patterns
//!
//! Provides additional filtering for:
//! - **Precision mode**: Stricter extraction when `Options.precision = true`
//! - **Teaser removal**: Removes preview sections of other content
//! - **Image captions**: Removes caption elements
//!
//! Port of:
//! - `internal/selector/content-discard-precision.go`
//! - `internal/selector/teaser-discard.go`
//! - `internal/selector/image-discard.go`

use crate::selector::utils::{attr, class, contains, id, lower, tag};
use crate::selector::Rule;
use dom_query::Selection;

// ============================================================
// PRECISION MODE DISCARD RULES
// Used when Options.precision = true for stricter extraction
// ============================================================

/// Precision mode discarded content rules
///
/// Applied **in addition to** `OVERALL_DISCARDED_CONTENT` when precision mode is enabled.
/// Provides stricter filtering for cleaner extraction.
pub static PRECISION_DISCARDED_CONTENT: &[Rule] = &[
    precision_discarded_content_rule_1,
    precision_discarded_content_rule_2,
];

/// Rule 1: Header elements
///
/// In precision mode, `<header>` elements are removed as they typically contain
/// site navigation and branding rather than article content.
///
/// **Tags**: header (ONLY this tag)
///
/// Go equivalent: `precisionDiscardedContentRule1` (lines 35-37)
#[must_use]
pub fn precision_discarded_content_rule_1(sel: &Selection) -> bool {
    tag(sel) == "header"
}

/// Rule 2: Bottom, link, and border elements
///
/// Removes elements with:
/// - "bottom" in id or class (page footers, bottom navigation)
/// - "link" in id or class (link lists, related links)
/// - "border" in style attribute (often used for visual separators/ads)
///
/// **Tags**: div, dd, dt, li, ul, ol, dl, p, section, span (ONLY these tags)
///
/// Go equivalent: `precisionDiscardedContentRule2` (lines 39-65)
#[must_use]
pub fn precision_discarded_content_rule_2(sel: &Selection) -> bool {
    let tag_val = tag(sel);
    let id_val = id(sel);
    let class_val = class(sel);
    let style = attr(sel, "style");
    let id_class = format!("{id_val}{class_val}");

    // Tag filter - ONLY these tags
    if !matches!(
        tag_val.as_str(),
        "div" | "dd" | "dt" | "li" | "ul" | "ol" | "dl" | "p" | "section" | "span"
    ) {
        return false;
    }

    // Pattern matching
    contains(&id_class, "bottom") || contains(&id_class, "link") || contains(&style, "border")
}

// ============================================================
// TEASER DISCARD RULES
// Remove teaser sections that preview other content
// ============================================================

/// Teaser discard rules
///
/// Teasers are preview sections that link to other articles/content.
/// They should be removed to avoid including summaries of other content
/// in the extracted main article.
pub static DISCARDED_TEASER: &[Rule] = &[discarded_teaser_rule_1];

/// Rule 1: Teaser elements
///
/// Removes elements with "teaser" in their id or class attributes.
/// Common patterns: "article-teaser", "news-teaser", "teaser-box"
///
/// **Tags**: div, dd, dt, li, ul, ol, dl, p, section, span (ONLY these tags)
///
/// **Case sensitivity**: Case-insensitive (matches "teaser", "Teaser", "TEASER", etc.)
///
/// Go equivalent: `discardedTeaserRule1` (lines 35-54)
#[must_use]
pub fn discarded_teaser_rule_1(sel: &Selection) -> bool {
    let tag_val = tag(sel);
    let id_val = id(sel);
    let class_val = class(sel);

    // Tag filter - ONLY these tags
    if !matches!(
        tag_val.as_str(),
        "div" | "dd" | "dt" | "li" | "ul" | "ol" | "dl" | "p" | "section" | "span"
    ) {
        return false;
    }

    // Pattern matching (case-insensitive)
    contains(&lower(&id_val), "teaser") || contains(&lower(&class_val), "teaser")
}

// ============================================================
// IMAGE DISCARD RULES
// Remove image captions and related elements
// ============================================================

/// Image discard rules
///
/// Image captions are typically removed because:
/// - They're often boilerplate ("Image: Getty Images", "Photo credit: Reuters")
/// - They may contain ads or attribution text
/// - go-trafilatura removes them for cleaner extraction
pub static DISCARDED_IMAGE: &[Rule] = &[discarded_image_rule_1];

/// Rule 1: Caption elements
///
/// Removes elements with "caption" in their id or class attributes.
/// Common patterns: "image-caption", "photo-caption", "caption-text"
///
/// **Tags**: div, dd, dt, li, ul, ol, dl, p, section, span (ONLY these tags)
///
/// **Case sensitivity**: Case-sensitive (only matches lowercase "caption")
///
/// Go equivalent: `discardedImageRule1` (lines 35-54)
#[must_use]
pub fn discarded_image_rule_1(sel: &Selection) -> bool {
    let tag_val = tag(sel);
    let id_val = id(sel);
    let class_val = class(sel);

    // Tag filter - ONLY these tags
    if !matches!(
        tag_val.as_str(),
        "div" | "dd" | "dt" | "li" | "ul" | "ol" | "dl" | "p" | "section" | "span"
    ) {
        return false;
    }

    // Pattern matching (case-sensitive)
    contains(&id_val, "caption") || contains(&class_val, "caption")
}

// ============================================================
// HELPER FUNCTIONS
// ============================================================

/// Check if element should be discarded in precision mode
///
/// Returns true if the element matches any precision mode discard pattern.
///
/// # Example
///
/// ```rust
/// use rs_trafilatura::selector::precision;
/// use rs_trafilatura::dom;
///
/// let doc = dom::parse("<header>site header</header>");
/// let header = doc.select("header");
///
/// assert!(precision::should_discard_precision(&header));
/// ```
#[must_use]
pub fn should_discard_precision(sel: &Selection) -> bool {
    PRECISION_DISCARDED_CONTENT.iter().any(|rule| rule(sel))
}

/// Check if element is a teaser
///
/// Returns true if the element matches teaser patterns.
///
/// # Example
///
/// ```rust
/// use rs_trafilatura::selector::precision;
/// use rs_trafilatura::dom;
///
/// let doc = dom::parse(r#"<div class="article-teaser">Preview text...</div>"#);
/// let div = doc.select("div");
///
/// assert!(precision::is_teaser(&div));
/// ```
#[must_use]
pub fn is_teaser(sel: &Selection) -> bool {
    DISCARDED_TEASER.iter().any(|rule| rule(sel))
}

/// Check if element is an image caption to discard
///
/// Returns true if the element matches image caption patterns.
///
/// # Example
///
/// ```rust
/// use rs_trafilatura::selector::precision;
/// use rs_trafilatura::dom;
///
/// let doc = dom::parse(r#"<div class="image-caption">Photo: Reuters</div>"#);
/// let div = doc.select("div");
///
/// assert!(precision::is_image_discard(&div));
/// ```
#[must_use]
pub fn is_image_discard(sel: &Selection) -> bool {
    DISCARDED_IMAGE.iter().any(|rule| rule(sel))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom;

    // ===== Precision Rule 1 Tests =====

    #[test]
    fn test_precision_header() {
        let doc = dom::parse("<header>site header</header>");
        assert!(precision_discarded_content_rule_1(&doc.select("header")));
    }

    #[test]
    fn test_precision_not_header() {
        let doc = dom::parse("<div>content</div>");
        assert!(!precision_discarded_content_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_precision_header_with_class() {
        let doc = dom::parse(r#"<header class="site-header">header</header>"#);
        assert!(precision_discarded_content_rule_1(&doc.select("header")));
    }

    // ===== Precision Rule 2 Tests =====

    #[test]
    fn test_precision_bottom() {
        let doc = dom::parse(r#"<div class="page-bottom">content</div>"#);
        assert!(precision_discarded_content_rule_2(&doc.select("div")));
    }

    #[test]
    fn test_precision_bottom_in_id() {
        let doc = dom::parse(r#"<section id="article-bottom">content</section>"#);
        assert!(precision_discarded_content_rule_2(&doc.select("section")));
    }

    #[test]
    fn test_precision_link() {
        let doc = dom::parse(r#"<ul id="related-links">links</ul>"#);
        assert!(precision_discarded_content_rule_2(&doc.select("ul")));
    }

    #[test]
    fn test_precision_link_in_class() {
        let doc = dom::parse(r#"<div class="sidebar-links">links</div>"#);
        assert!(precision_discarded_content_rule_2(&doc.select("div")));
    }

    #[test]
    fn test_precision_border_style() {
        let doc = dom::parse(r#"<p style="border: 1px solid black">content</p>"#);
        assert!(precision_discarded_content_rule_2(&doc.select("p")));
    }

    #[test]
    fn test_precision_border_style_complex() {
        let doc = dom::parse(r#"<div style="padding: 10px; border-top: 2px;">content</div>"#);
        assert!(precision_discarded_content_rule_2(&doc.select("div")));
    }

    #[test]
    fn test_precision_wrong_tag() {
        // article tag not in allowed list
        let doc = dom::parse(r#"<article class="bottom">content</article>"#);
        assert!(!precision_discarded_content_rule_2(&doc.select("article")));
    }

    #[test]
    fn test_precision_no_match() {
        let doc = dom::parse(r#"<div class="article-content">clean content</div>"#);
        assert!(!precision_discarded_content_rule_2(&doc.select("div")));
    }

    // ===== Teaser Tests =====

    #[test]
    fn test_teaser_class() {
        let doc = dom::parse(r#"<div class="article-teaser">preview</div>"#);
        assert!(discarded_teaser_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_teaser_id() {
        let doc = dom::parse(r#"<section id="news-teaser">preview</section>"#);
        assert!(discarded_teaser_rule_1(&doc.select("section")));
    }

    #[test]
    fn test_teaser_case_insensitive() {
        let doc = dom::parse(r#"<section id="MainTeaser">preview</section>"#);
        assert!(discarded_teaser_rule_1(&doc.select("section")));
    }

    #[test]
    fn test_teaser_case_insensitive_class() {
        let doc = dom::parse(r#"<div class="TEASER-box">preview</div>"#);
        assert!(discarded_teaser_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_teaser_wrong_tag() {
        // article tag not in allowed list
        let doc = dom::parse(r#"<article class="teaser">preview</article>"#);
        assert!(!discarded_teaser_rule_1(&doc.select("article")));
    }

    #[test]
    fn test_teaser_wrong_tag_h2() {
        let doc = dom::parse(r#"<h2 class="teaser">preview</h2>"#);
        assert!(!discarded_teaser_rule_1(&doc.select("h2")));
    }

    #[test]
    fn test_teaser_no_match() {
        let doc = dom::parse(r#"<div class="article-content">content</div>"#);
        assert!(!discarded_teaser_rule_1(&doc.select("div")));
    }

    // ===== Image Discard Tests =====

    #[test]
    fn test_image_caption_class() {
        let doc = dom::parse(r#"<div class="image-caption">Photo: Reuters</div>"#);
        assert!(discarded_image_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_image_caption_id() {
        let doc = dom::parse(r#"<p id="photo-caption">description</p>"#);
        assert!(discarded_image_rule_1(&doc.select("p")));
    }

    #[test]
    fn test_image_caption_in_class() {
        let doc = dom::parse(r#"<span class="wp-caption-text">text</span>"#);
        assert!(discarded_image_rule_1(&doc.select("span")));
    }

    #[test]
    fn test_image_caption_case_sensitive() {
        // "Caption" with capital C should NOT match "caption"
        let doc = dom::parse(r#"<div class="image-Caption">text</div>"#);
        // contains(&class, "caption") - "Caption" does NOT contain "caption"
        assert!(!discarded_image_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_image_caption_case_sensitive_id() {
        let doc = dom::parse(r#"<p id="CAPTION-text">text</p>"#);
        // contains(&id, "caption") - "CAPTION" does NOT contain "caption"
        assert!(!discarded_image_rule_1(&doc.select("p")));
    }

    #[test]
    fn test_image_wrong_tag() {
        // figcaption tag not in allowed list
        let doc = dom::parse(r#"<figcaption class="caption">text</figcaption>"#);
        assert!(!discarded_image_rule_1(&doc.select("figcaption")));
    }

    #[test]
    fn test_image_wrong_tag_figure() {
        let doc = dom::parse(r#"<figure id="caption">image</figure>"#);
        assert!(!discarded_image_rule_1(&doc.select("figure")));
    }

    #[test]
    fn test_image_no_match() {
        let doc = dom::parse(r#"<div class="article-image">content</div>"#);
        assert!(!discarded_image_rule_1(&doc.select("div")));
    }

    // ===== Helper Function Tests =====

    #[test]
    fn test_should_discard_precision_rule_1() {
        let doc = dom::parse("<header>header</header>");
        assert!(should_discard_precision(&doc.select("header")));
    }

    #[test]
    fn test_should_discard_precision_rule_2() {
        let doc = dom::parse(r#"<div class="page-bottom">content</div>"#);
        assert!(should_discard_precision(&doc.select("div")));
    }

    #[test]
    fn test_should_not_discard_precision() {
        let doc = dom::parse(r#"<div class="article">content</div>"#);
        assert!(!should_discard_precision(&doc.select("div")));
    }

    #[test]
    fn test_is_teaser_true() {
        let doc = dom::parse(r#"<div class="teaser-box">teaser</div>"#);
        assert!(is_teaser(&doc.select("div")));
    }

    #[test]
    fn test_is_teaser_false() {
        let doc = dom::parse(r#"<div class="article">content</div>"#);
        assert!(!is_teaser(&doc.select("div")));
    }

    #[test]
    fn test_is_image_discard_true() {
        let doc = dom::parse(r#"<span class="caption">caption</span>"#);
        assert!(is_image_discard(&doc.select("span")));
    }

    #[test]
    fn test_is_image_discard_false() {
        let doc = dom::parse(r#"<div class="image">content</div>"#);
        assert!(!is_image_discard(&doc.select("div")));
    }

    // ===== Combined Tests =====

    #[test]
    fn test_all_rules_independent() {
        // Verify that rules don't interfere with each other
        let doc = dom::parse(
            r#"
            <div>
                <header>header</header>
                <div class="bottom">bottom</div>
                <div class="teaser">teaser</div>
                <span class="caption">caption</span>
                <div class="content">keep this</div>
            </div>
        "#,
        );

        assert!(should_discard_precision(&doc.select("header")));
        assert!(should_discard_precision(&doc.select("div.bottom")));
        assert!(is_teaser(&doc.select("div.teaser")));
        assert!(is_image_discard(&doc.select("span.caption")));
        assert!(!should_discard_precision(&doc.select("div.content")));
        assert!(!is_teaser(&doc.select("div.content")));
        assert!(!is_image_discard(&doc.select("div.content")));
    }
}
