//! Overall Discard Patterns
//!
//! Identifies boilerplate elements (navigation, footers, sidebars, ads, paywalls) that should
//! be removed during content extraction.
//!
//! Port of `internal/selector/content-discard-overall.go`.

use crate::dom;
use crate::selector::utils::{attr, class, contains, id, lower, starts_with, tag};
use crate::selector::Rule;
use dom_query::Selection;

/// Overall discarded content rules
///
/// Elements matching these rules should be removed during extraction.
/// Includes navigation, footers, ads, social widgets, paywalls, and hidden elements.
pub static OVERALL_DISCARDED_CONTENT: &[Rule] = &[
    overall_discarded_content_rule_1,
    overall_discarded_content_rule_2,
    overall_discarded_content_rule_3,
];

/// Rule 1: Navigation, footers, social, ads, sidebars, paywalls (~75 patterns)
///
/// **Tags**: div, dd, dt, li, ul, ol, dl, p, section, span (ONLY these tags)
///
/// Matches elements with classes/IDs related to:
/// - Footer content
/// - Related content widgets
/// - Social sharing buttons
/// - Navigation menus
/// - Sidebars and banners
/// - Advertisements
/// - Author bylines
/// - Widgets and UI elements
/// - Paywalls and premium content
/// - Chinese site-specific patterns
///
/// Go equivalent: `overallDiscardedContentRule1` (lines 83-180)
#[must_use]
pub fn overall_discarded_content_rule_1(sel: &Selection) -> bool {
    let tag_val = tag(sel);
    let id_val = id(sel);
    let class_val = class(sel);
    let role = attr(sel, "role");
    let data_component = attr(sel, "data-component");
    let id_class = format!("{id_val}{class_val}");

    // Tag filter - ONLY these tags
    // Note: header and nav are always discarded (see overall_discarded_content_rule_3)
    if !matches!(
        tag_val.as_str(),
        "div" | "dd" | "dt" | "li" | "ul" | "ol" | "dl" | "p" | "section" | "span"
    ) {
        return false;
    }

    // Pattern matching - any match returns true
    // Footer patterns
    contains(&lower(&id_val), "footer")
        || contains(&lower(&class_val), "footer")
        // Related content widgets (but NOT blog detail pages like "related_post" singular)
        // Match: related-articles, related-posts, relatedstories, related_content
        // Skip: related_post, related-post (singular - often the main blog detail container)
        || (contains(&id_val, "related") && !id_val.ends_with("_post") && !id_val.ends_with("-post"))
        || (contains(&class_val, "related")
            && !contains(&class_val, "related_post")
            && !contains(&class_val, "related-post"))
        // Viral/sharing
        || contains(&id_class, "viral")
        || starts_with(&id_class, "shar")
        || contains(&class_val, "share-")
        || contains(&lower(&id_val), "share")
        // Social
        || contains(&id_class, "social")
        || contains(&class_val, "sociable")
        || contains(&id_class, "syndication")
        // WordPress specific
        || starts_with(&id_val, "jp-")
        || starts_with(&id_val, "dpsp-content")
        // Embeds
        || contains(&class_val, "embedded")
        || contains(&class_val, "embed")
        // Newsletter
        || contains(&id_class, "newsletter")
        // Navigation
        || contains(&class_val, "subnav")
        || contains(&id_class, "cookie")
        || contains(&id_class, "tags")
        || contains(&class_val, "tag-list")
        // Sidebar/banner (but NOT layout wrappers like "sidebar-left__wrapper")
        // Exception: "with-sidebar" means main content displayed WITH a sidebar, not the sidebar itself
        || (contains(&id_class, "sidebar")
            && !contains(&class_val, "sidebar-left")
            && !contains(&class_val, "sidebar__wrapper")
            && !contains(&class_val, "sidebar-wrapper")
            && !contains(&class_val, "with-sidebar"))
        || contains(&id_class, "banner")
        // Toolbar/navbar patterns (but not via "bar" substring which is too broad)
        || contains(&class_val, "toolbar")
        || contains(&class_val, "navbar")
        || contains(&class_val, "topbar")
        // Meta/menu (case-insensitive for menu to catch MenuItem, menuItem, etc.)
        // Exception: "contextmenu" is CSS for right-click styling, not navigation
        || contains(&class_val, "meta")
        || contains(&lower(&id_val), "menu")
        || (contains(&lower(&class_val), "menu") && !contains(&lower(&class_val), "contextmenu"))
        // Navigation (case-insensitive)
        || contains(&lower(&id_val), "nav")
        || contains(&lower(&role), "nav")
        || starts_with(&lower(&class_val), "nav")
        || contains(&lower(&class_val), "avigation") // catches "navigation", "Navigation"
        || contains(&lower(&class_val), "navbar")
        || contains(&lower(&class_val), "navbox")
        || starts_with(&lower(&class_val), "post-nav")
        // Breadcrumbs
        || contains(&id_class, "breadcrumb")
        || contains(&id_class, "bread-crumb")
        // Author/byline
        || contains(&id_class, "author")
        || contains(&id_class, "button")
        || contains(&lower(&class_val), "byline")
        // Widgets (but NOT Elementor/HubSpot content widgets)
        || contains(&class_val, "rating")
        || (contains(&class_val, "widget")
            && !contains(&class_val, "elementor-widget")
            && !contains(&class_val, "hs_cos_wrapper"))
        || contains(&class_val, "attachment")
        || contains(&class_val, "timestamp")
        || contains(&class_val, "user-info")
        || contains(&class_val, "user-profile")
        // Ads
        || contains(&class_val, "-ad-")
        || contains(&class_val, "-icon")
        || contains(&class_val, "article-infos")
        || contains(&class_val, "nfoline")
        // Third-party
        || contains(&data_component, "MostPopularStories")
        || contains(&class_val, "outbrain")
        || contains(&class_val, "taboola")
        || contains(&class_val, "criteo")
        // UI elements
        || contains(&class_val, "options")
        || contains(&class_val, "expand")
        || contains(&class_val, "consent")
        || contains(&class_val, "modal-content")
        || contains(&class_val, " ad ")
        || contains(&class_val, "permission")
        // Related stories
        || contains(&class_val, "next-")
        || contains(&class_val, "-stories")
        || contains(&class_val, "most-popular")
        || contains(&class_val, "mol-factbox")
        // Forms
        || starts_with(&class_val, "ZendeskForm")
        || contains(&id_class, "message-container")
        // Chinese site patterns
        || contains(&class_val, "yin")
        || contains(&class_val, "zlylin")
        || contains(&class_val, "xg1")
        || contains(&id_val, "bmdh")
        // UI elements and carousels
        || contains(&class_val, "slide")
        || contains(&class_val, "slick-")
        || contains(&class_val, "carousel")
        || contains(&class_val, "swiper")
        || contains(&class_val, "viewport")
        // Data attributes
        || dom::has_attribute(sel, "data-lp-replacement-content")
        // Paywalls
        || contains(&id_val, "premium")
        || contains(&class_val, "overlay")
        || contains(&class_val, "paid-content")
        || contains(&class_val, "paidcontent")
        || contains(&class_val, "obfuscated")
        || contains(&class_val, "blurred")
        // Login/subscribe prompts
        || contains(&class_val, "login")
        || contains(&class_val, "signin")
        || contains(&class_val, "sign-in")
        || contains(&class_val, "signup")
        || contains(&class_val, "sign-up")
        || contains(&class_val, "subscribe")
        || contains(&class_val, "subscription")
        || contains(&id_class, "snippet")
        // Trending/popular sections
        || contains(&class_val, "trending")
        || contains(&class_val, "popular")
        || contains(&class_val, "most-read")
        || contains(&class_val, "top-stories")
        // WikiHow specific boilerplate
        || contains(&class_val, "article_byline")
        || contains(&class_val, "coauthor")
        || contains(&class_val, "wh_ad")
        || contains(&id_val, "social_proof")
        || contains(&class_val, "social_proof")
        || contains(&class_val, "sp_inner")
        || contains(&class_val, "sp_box")
        || contains(&class_val, "sp_helpful")
        || contains(&class_val, "sp_expert")
        || contains(&id_val, "sp_expert")
        || contains(&class_val, "wh_thumb_helpful")
        || contains(&class_val, "wh_thumb_unhelpful")
        || contains(&class_val, "aboutthisarticle")
        || contains(&id_val, "aboutthisarticle")
        || contains(&class_val, "reader_tips")
        || contains(&id_val, "reader_tips")
        || contains(&class_val, "helpfulness")
        || contains(&id_val, "byline_hover")
        || contains(&class_val, "byline_hover")
}

/// Rule 2: Comment debris and hidden elements (~20 patterns)
///
/// **Tags**: ANY element (no tag filter)
///
/// Matches elements with:
/// - Comment-related classes/IDs
/// - Hidden visibility (style, class, aria-hidden)
/// - Print-only content markers
/// - Akismet spam filter markers
///
/// Go equivalent: `overallDiscardedContentRule2` (lines 182-227)
#[must_use]
pub fn overall_discarded_content_rule_2(sel: &Selection) -> bool {
    let id_val = id(sel);
    let class_val = class(sel);
    let style = attr(sel, "style");
    let aria_hidden = attr(sel, "aria-hidden");
    let id_class = format!("{id_val}{class_val}");
    let id_style = format!("{id_val}{style}");

    // No tag filter - applies to any element

    // Pattern matching
    class_val == "comments-title"
        || contains(&class_val, "comments-title")
        || contains(&class_val, "nocomments")
        || starts_with(&id_class, "reply-")
        || contains(&class_val, "-reply-")
        || contains(&class_val, "message")
        || contains(&id_val, "reader-comments")
        || contains(&id_val, "akismet")
        || contains(&class_val, "akismet")
        || contains(&class_val, "suggest-links")
        // Hidden elements
        || starts_with(&class_val, "hide-")
        || contains(&class_val, "-hide-")
        || contains(&class_val, "hide-print")
        || contains(&id_style, "hidden")
        || contains(&class_val, " hidden")
        || contains(&class_val, " hide")
        || contains(&class_val, "noprint")
        || contains(&style, "display:none")
        || contains(&style, "display: none")
        || aria_hidden == "true"
        || dom::has_attribute(sel, "hidden") // HTML5 hidden attribute
        || contains(&class_val, "notloaded")
}

/// Rule 3: Header and navigation elements
///
/// **Tags**: header, nav, aside (structural boilerplate elements)
///
/// These elements are always considered boilerplate on modern web pages.
/// Headers contain site branding, navigation menus, and user account links.
/// Nav elements contain navigation links.
/// Aside elements contain sidebars and related content.
///
/// This rule was added to handle modern web pages (2025+) that use semantic HTML
/// with complex navigation structures inside header elements.
#[must_use]
pub fn overall_discarded_content_rule_3(sel: &Selection) -> bool {
    let tag_val = tag(sel);

    // Match structural boilerplate elements
    matches!(tag_val.as_str(), "header" | "nav" | "aside")
}

// ... (previous imports)

/// Precision discarded content rules
///
/// Toggled by `options.favor_precision`. Removes more potential noise.
pub static PRECISION_DISCARDED_CONTENT: &[Rule] = &[
    precision_discard_rule_1,
];

/// Rule 1: Precision discard patterns
///
/// Go equivalent: `precisionDiscardedContentRule1`
#[must_use]
pub fn precision_discard_rule_1(sel: &Selection) -> bool {
    let id_val = id(sel);
    let class_val = class(sel);
    let id_class = format!("{id_val}{class_val}");

    // Pattern matching
    contains(&id_class, "fs-headline")
        || contains(&class_val, "read-more")
        || contains(&class_val, "bottom")
        || contains(&class_val, "generic")
        || contains(&class_val, "jumbotron")
}

/// Teaser content rules
///
/// Identifies "teaser" elements that are just summaries/links to other content.
pub static TEASER_DISCARDED_CONTENT: &[Rule] = &[
    teaser_rule_1,
];

/// Rule 1: Teaser patterns
///
/// Go equivalent: `teaserRule1`
#[must_use]
pub fn teaser_rule_1(sel: &Selection) -> bool {
    let id_val = id(sel);
    let class_val = class(sel);
    let id_class = format!("{id_val}{class_val}");

    contains(&class_val, "teaser")
        || contains(&class_val, "excerpt")
        || contains(&class_val, "summary")
        || contains(&id_class, "context")
        || contains(&class_val, "promoted")
        || contains(&class_val, "sponsored")
        || contains(&class_val, "paid")
        || contains(&id_class, "cta") // Call to action
        || contains(&id_class, "promo")
}

/// Check if element should be discarded (matches any discard rule)
///
/// Returns true if the element matches any of the overall discard patterns.
///
/// # Example
///
/// ```rust
/// use rs_trafilatura::selector::discard;
/// use rs_trafilatura::dom;
///
/// let doc = dom::parse(r#"<div class="footer">footer content</div>"#);
/// let div = doc.select("div");
///
/// assert!(discard::should_discard(&div));
/// ```
#[must_use]
pub fn should_discard(sel: &Selection) -> bool {
    OVERALL_DISCARDED_CONTENT.iter().any(|rule| rule(sel))
}

/// Get all elements that should be discarded from a tree
///
/// Finds all elements in the tree that match any of the overall discard patterns.
///
/// # Example
///
/// ```rust
/// use rs_trafilatura::selector::discard;
/// use rs_trafilatura::dom;
///
/// let doc = dom::parse(r#"
///     <div>
///         <div class="content">keep this</div>
///         <div class="footer">discard this</div>
///         <div class="sidebar">discard this</div>
///     </div>
/// "#);
/// let root = doc.select("div").first();
///
/// let discardable = discard::find_discardable(&root);
/// assert_eq!(discardable.len(), 2); // footer and sidebar
/// ```
pub fn find_discardable<'a>(root: &Selection<'a>) -> Vec<Selection<'a>> {
    use crate::selector::query_all;

    let mut all_matches = Vec::new();
    for rule in OVERALL_DISCARDED_CONTENT {
        all_matches.extend(query_all(root, *rule));
    }
    all_matches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom;

    // ===== Rule 1 Tests =====

    #[test]
    fn test_discard_footer() {
        let doc = dom::parse(r#"<div class="footer">content</div>"#);
        assert!(overall_discarded_content_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_discard_footer_case_insensitive() {
        let doc = dom::parse(r#"<div id="pageFooter">content</div>"#);
        assert!(overall_discarded_content_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_discard_navigation() {
        let doc = dom::parse(r#"<ul class="navbar">items</ul>"#);
        assert!(overall_discarded_content_rule_1(&doc.select("ul")));
    }

    #[test]
    fn test_discard_navigation_avigation_pattern() {
        // "avigation" pattern catches "navigation"
        let doc = dom::parse(r#"<div class="main-navigation">nav</div>"#);
        assert!(overall_discarded_content_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_discard_sidebar() {
        let doc = dom::parse(r#"<div id="sidebar">content</div>"#);
        assert!(overall_discarded_content_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_discard_ad() {
        let doc = dom::parse(r#"<div class="content-ad-wrapper">ad</div>"#);
        assert!(overall_discarded_content_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_discard_ad_with_spaces() {
        let doc = dom::parse(r#"<div class="box ad slot">ad</div>"#);
        assert!(overall_discarded_content_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_discard_paywall() {
        let doc = dom::parse(r#"<div class="paid-content">subscribe</div>"#);
        assert!(overall_discarded_content_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_discard_paywall_obfuscated() {
        let doc = dom::parse(r#"<div class="obfuscated">premium</div>"#);
        assert!(overall_discarded_content_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_discard_social() {
        let doc = dom::parse(r#"<span class="social-share">share</span>"#);
        assert!(overall_discarded_content_rule_1(&doc.select("span")));
    }

    #[test]
    fn test_discard_sharing_shar_prefix() {
        let doc = dom::parse(r#"<div id="share-buttons">share</div>"#);
        assert!(overall_discarded_content_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_discard_related_elated_pattern() {
        // "elated" pattern catches "related" without 'r'
        let doc = dom::parse(r#"<div class="related-articles">related</div>"#);
        assert!(overall_discarded_content_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_discard_breadcrumb() {
        let doc = dom::parse(r#"<ul class="breadcrumb">path</ul>"#);
        assert!(overall_discarded_content_rule_1(&doc.select("ul")));
    }

    #[test]
    fn test_discard_author() {
        let doc = dom::parse(r#"<div class="author-info">by John</div>"#);
        assert!(overall_discarded_content_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_discard_widget() {
        let doc = dom::parse(r#"<div class="widget-recent">posts</div>"#);
        assert!(overall_discarded_content_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_discard_third_party_outbrain() {
        let doc = dom::parse(r#"<div class="outbrain-widget">recommended</div>"#);
        assert!(overall_discarded_content_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_discard_wordpress_jetpack() {
        let doc = dom::parse(r#"<div id="jp-post-flair">jetpack</div>"#);
        assert!(overall_discarded_content_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_discard_modal() {
        let doc = dom::parse(r#"<div class="modal-content">popup</div>"#);
        assert!(overall_discarded_content_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_discard_chinese_pattern() {
        let doc = dom::parse(r#"<div class="xg1">content</div>"#);
        assert!(overall_discarded_content_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_discard_wrong_tag() {
        // article tag not in allowed list
        let doc = dom::parse(r#"<article class="footer">content</article>"#);
        assert!(!overall_discarded_content_rule_1(&doc.select("article")));
    }

    #[test]
    fn test_discard_wrong_tag_h1() {
        // h1 tag not in allowed list
        let doc = dom::parse(r#"<h1 class="footer">heading</h1>"#);
        assert!(!overall_discarded_content_rule_1(&doc.select("h1")));
    }

    #[test]
    fn test_no_discard_clean_content() {
        let doc = dom::parse(r#"<div class="article-content">clean content</div>"#);
        assert!(!overall_discarded_content_rule_1(&doc.select("div")));
    }

    // ===== Rule 2 Tests =====

    #[test]
    fn test_discard_hidden_style() {
        let doc = dom::parse(r#"<div style="display:none">hidden</div>"#);
        assert!(overall_discarded_content_rule_2(&doc.select("div")));
    }

    #[test]
    fn test_discard_hidden_style_with_space() {
        let doc = dom::parse(r#"<div style="display: none">hidden</div>"#);
        assert!(overall_discarded_content_rule_2(&doc.select("div")));
    }

    #[test]
    fn test_discard_aria_hidden() {
        let doc = dom::parse(r#"<span aria-hidden="true">hidden</span>"#);
        assert!(overall_discarded_content_rule_2(&doc.select("span")));
    }

    #[test]
    fn test_discard_hidden_class() {
        let doc = dom::parse(r#"<div class="is hidden">content</div>"#);
        assert!(overall_discarded_content_rule_2(&doc.select("div")));
    }

    #[test]
    fn test_discard_hide_class() {
        let doc = dom::parse(r#"<div class="content hide">content</div>"#);
        assert!(overall_discarded_content_rule_2(&doc.select("div")));
    }

    #[test]
    fn test_discard_noprint() {
        let doc = dom::parse(r#"<div class="noprint">print only</div>"#);
        assert!(overall_discarded_content_rule_2(&doc.select("div")));
    }

    #[test]
    fn test_discard_comments_title() {
        let doc = dom::parse(r#"<h2 class="comments-title">Comments</h2>"#);
        assert!(overall_discarded_content_rule_2(&doc.select("h2")));
    }

    #[test]
    fn test_discard_comments_title_exact() {
        let doc = dom::parse(r#"<div class="comments-title">Comments</div>"#);
        assert!(overall_discarded_content_rule_2(&doc.select("div")));
    }

    #[test]
    fn test_discard_nocomments() {
        let doc = dom::parse(r#"<div class="nocomments">No comments</div>"#);
        assert!(overall_discarded_content_rule_2(&doc.select("div")));
    }

    #[test]
    fn test_discard_reply() {
        let doc = dom::parse(r#"<div id="reply-123">reply</div>"#);
        assert!(overall_discarded_content_rule_2(&doc.select("div")));
    }

    #[test]
    fn test_discard_akismet() {
        let doc = dom::parse(r#"<div class="akismet-info">spam filter</div>"#);
        assert!(overall_discarded_content_rule_2(&doc.select("div")));
    }

    #[test]
    fn test_discard_hide_prefix() {
        let doc = dom::parse(r#"<div class="hide-mobile">hidden on mobile</div>"#);
        assert!(overall_discarded_content_rule_2(&doc.select("div")));
    }

    #[test]
    fn test_no_discard_visible_content_rule_2() {
        let doc = dom::parse(r#"<div class="article-text">visible content</div>"#);
        assert!(!overall_discarded_content_rule_2(&doc.select("div")));
    }

    #[test]
    fn test_rule_2_applies_to_any_tag() {
        // Rule 2 has no tag filter - works on any element
        let doc = dom::parse(r#"<article style="display:none">hidden</article>"#);
        assert!(overall_discarded_content_rule_2(&doc.select("article")));
    }

    // ===== Helper Function Tests =====

    #[test]
    fn test_should_discard_rule_1() {
        let doc = dom::parse(r#"<div class="footer">footer</div>"#);
        assert!(should_discard(&doc.select("div")));
    }

    #[test]
    fn test_should_discard_rule_2() {
        let doc = dom::parse(r#"<div style="display:none">hidden</div>"#);
        assert!(should_discard(&doc.select("div")));
    }

    #[test]
    fn test_should_not_discard_content() {
        let doc = dom::parse(r#"<div class="article-content">content</div>"#);
        assert!(!should_discard(&doc.select("div")));
    }

    #[test]
    fn test_find_discardable() {
        let doc = dom::parse(
            r#"
            <div>
                <div class="content">keep</div>
                <div class="footer">discard</div>
                <div class="sidebar">discard</div>
                <p class="text">keep</p>
            </div>
        "#,
        );
        let root = doc.select("div").first();
        let discardable = find_discardable(&root);
        assert_eq!(discardable.len(), 2); // footer and sidebar
    }

    #[test]
    fn test_find_discardable_mixed_rules() {
        let doc = dom::parse(
            r#"
            <div>
                <div class="article">keep</div>
                <div class="footer">rule 1</div>
                <span style="display:none">rule 2</span>
                <p>keep</p>
            </div>
        "#,
        );
        let root = doc.select("div").first();
        let discardable = find_discardable(&root);
        assert_eq!(discardable.len(), 2); // footer (rule 1) and span (rule 2)
    }

    #[test]
    fn test_find_discardable_none() {
        let doc = dom::parse(
            r#"
            <div>
                <div class="content">keep</div>
                <p class="paragraph">keep</p>
            </div>
        "#,
        );
        let root = doc.select("div").first();
        let discardable = find_discardable(&root);
        assert_eq!(discardable.len(), 0);
    }
}

