//! Comment Selectors
//!
//! Selector rules for finding, cleaning, and removing comment sections.
//!
//! Port of:
//! - `internal/selector/comments.go` - Find comment sections
//! - `internal/selector/comments-discard.go` - Discard comment debris
//! - `internal/selector/comments-removed.go` - Remove comment sections

use dom_query::Selection;

use crate::selector::utils::{attr, class, contains, id, lower, starts_with, tag};
use crate::selector::Rule;

// ============================================================
// COMMENT FINDING RULES
// Used when Options.include_comments = true
// ============================================================

/// Comment section selector rules
pub static COMMENTS: &[Rule] = &[
    comments_rule_1,
    comments_rule_2,
    comments_rule_3,
    comments_rule_4,
];

/// Rule 1: Comment list containers
///
/// Tags: div, ol, ul, dl, section
/// Patterns: commentlist, comment-page, comment-list, comments-content, post-comments
///
/// Go: commentsRule1 (lines 41-64)
#[must_use]
pub fn comments_rule_1(sel: &Selection) -> bool {
    let tag_val = tag(sel);
    let id_val = id(sel);
    let class_val = class(sel);
    let id_class = format!("{id_val}{class_val}");

    // Tag filter
    if !matches!(
        tag_val.as_str(),
        "div" | "ol" | "ul" | "dl" | "section"
    ) {
        return false;
    }

    // Pattern matching
    contains(&id_class, "commentlist")
        || contains(&class_val, "comment-page")
        || contains(&id_class, "comment-list")
        || contains(&class_val, "comments-content")
        || contains(&class_val, "post-comments")
}

/// Rule 2: Comment section containers
///
/// Tags: div, section, ol, ul, dl
/// Patterns: comments*, Comments*, comment-*, article-comments
///
/// Go: commentsRule2 (lines 66-93)
#[must_use]
pub fn comments_rule_2(sel: &Selection) -> bool {
    let tag_val = tag(sel);
    let id_val = id(sel);
    let class_val = class(sel);
    let id_class = format!("{id_val}{class_val}");

    // Tag filter
    if !matches!(
        tag_val.as_str(),
        "div" | "section" | "ol" | "ul" | "dl"
    ) {
        return false;
    }

    // Pattern matching
    starts_with(&id_class, "comments")
        || starts_with(&class_val, "Comments")
        || starts_with(&id_class, "comment-")
        || contains(&class_val, "article-comments")
}

/// Rule 3: Third-party comment systems
///
/// Tags: div, section, ol, ul, dl
/// Patterns: `comol*`, `disqus_thread*`, `dsq_comments*`
///
/// Go: commentsRule3 (lines 95-116)
#[must_use]
pub fn comments_rule_3(sel: &Selection) -> bool {
    let tag_val = tag(sel);
    let id_val = id(sel);

    // Tag filter
    if !matches!(
        tag_val.as_str(),
        "div" | "section" | "ol" | "ul" | "dl"
    ) {
        return false;
    }

    // Pattern matching (id-based for third-party systems)
    starts_with(&id_val, "comol")
        || starts_with(&id_val, "disqus_thread")
        || starts_with(&id_val, "dsq_comments")
}

/// Rule 4: Generic comment markers
///
/// Tags: div, section
/// Patterns: social*, *comment*
///
/// Go: commentsRule4 (lines 118-138)
#[must_use]
pub fn comments_rule_4(sel: &Selection) -> bool {
    let tag_val = tag(sel);
    let id_val = id(sel);
    let class_val = class(sel);

    // Tag filter
    if !matches!(tag_val.as_str(), "div" | "section") {
        return false;
    }

    // Pattern matching
    starts_with(&id_val, "social") || contains(&class_val, "comment")
}

// ============================================================
// COMMENT DISCARD RULES
// Remove debris within comment sections
// ============================================================

/// Comment discard rules (debris within comments)
pub static DISCARDED_COMMENTS: &[Rule] = &[
    discarded_comments_rule_1,
    discarded_comments_rule_2,
    discarded_comments_rule_3,
];

/// Rule 1: Respond sections
///
/// Tags: div, section
/// Patterns: respond*
///
/// Go: discardedCommentsRule1 (lines 36-53)
#[must_use]
pub fn discarded_comments_rule_1(sel: &Selection) -> bool {
    let tag_val = tag(sel);
    let id_val = id(sel);

    // Tag filter
    if !matches!(tag_val.as_str(), "div" | "section") {
        return false;
    }

    // Pattern matching
    starts_with(&id_val, "respond")
}

/// Rule 2: Cite and quote elements
///
/// Tags: cite, quote
///
/// Go: discardedCommentsRule2 (lines 55-59)
#[must_use]
pub fn discarded_comments_rule_2(sel: &Selection) -> bool {
    let tag_val = tag(sel);
    tag_val == "cite" || tag_val == "quote"
}

/// Rule 3: Comment UI elements
///
/// Any tag
/// Patterns: comments-title, nocomments, reply-*, message, signin, akismet, display:none
///
/// Go: discardedCommentsRule3 (lines 61-87)
#[must_use]
pub fn discarded_comments_rule_3(sel: &Selection) -> bool {
    let id_val = id(sel);
    let class_val = class(sel);
    let style = attr(sel, "style");
    let id_class = format!("{id_val}{class_val}");

    // No tag filter - applies to any element

    // Pattern matching
    class_val == "comments-title"
        || contains(&class_val, "comments-title")
        || contains(&class_val, "nocomments")
        || starts_with(&id_class, "reply-")
        || contains(&class_val, "-reply-")
        || contains(&class_val, "message")
        || contains(&class_val, "signin")
        || contains(&id_class, "akismet")
        || contains(&style, "display:none")
}

// ============================================================
// COMMENT REMOVAL RULES
// Remove entire comment sections (when include_comments = false)
// ============================================================

/// Comment removal rules
pub static REMOVED_COMMENTS: &[Rule] = &[removed_comments_rule_1];

/// Rule 1: Remove comment sections entirely
///
/// Tags: div, ol, ul, dl, section
/// Patterns: comment* (case-insensitive), article-comments, post-comments, comol*, disqus*, dsq-*
///
/// Go: removedCommentsRule1 (lines 39-63)
#[must_use]
pub fn removed_comments_rule_1(sel: &Selection) -> bool {
    let tag_val = tag(sel);
    let id_val = id(sel);
    let class_val = class(sel);

    // Tag filter
    if !matches!(
        tag_val.as_str(),
        "div" | "ol" | "ul" | "dl" | "section"
    ) {
        return false;
    }

    // Pattern matching
    starts_with(&lower(&id_val), "comment")
        || starts_with(&lower(&class_val), "comment")
        || contains(&class_val, "article-comments")
        || contains(&class_val, "post-comments")
        || starts_with(&id_val, "comol")
        || starts_with(&id_val, "disqus_thread")
        || starts_with(&id_val, "dsq-comments")
}

// ============================================================
// HELPER FUNCTIONS
// ============================================================

/// Find comment sections in the document
#[must_use]
pub fn find_comments<'a>(root: &Selection<'a>) -> Vec<Selection<'a>> {
    use crate::selector::query_all;

    let mut all_matches = Vec::new();
    for rule in COMMENTS {
        all_matches.extend(query_all(root, *rule));
    }
    all_matches
}

/// Check if element is a comment section
#[must_use]
pub fn is_comment_section(sel: &Selection) -> bool {
    COMMENTS.iter().any(|rule| rule(sel))
}

/// Check if element is comment debris to discard
#[must_use]
pub fn is_comment_debris(sel: &Selection) -> bool {
    DISCARDED_COMMENTS.iter().any(|rule| rule(sel))
}

/// Check if element is a comment section to remove
#[must_use]
pub fn should_remove_comments(sel: &Selection) -> bool {
    REMOVED_COMMENTS.iter().any(|rule| rule(sel))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom;

    // ============================================================
    // COMMENT FINDING TESTS
    // ============================================================

    #[test]
    fn test_comments_rule_1_commentlist() {
        let doc = dom::parse(r#"<div id="commentlist">comments</div>"#);
        assert!(comments_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_comments_rule_1_comment_list() {
        let doc = dom::parse(r#"<ul class="comment-list">comments</ul>"#);
        assert!(comments_rule_1(&doc.select("ul")));
    }

    #[test]
    fn test_comments_rule_1_post_comments() {
        let doc = dom::parse(r#"<section class="post-comments">comments</section>"#);
        assert!(comments_rule_1(&doc.select("section")));
    }

    #[test]
    fn test_comments_rule_1_comment_page() {
        let doc = dom::parse(r#"<div class="comment-page">page</div>"#);
        assert!(comments_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_comments_rule_1_comments_content() {
        let doc = dom::parse(r#"<section class="comments-content">content</section>"#);
        assert!(comments_rule_1(&doc.select("section")));
    }

    #[test]
    fn test_comments_rule_1_wrong_tag() {
        let doc = dom::parse(r#"<article id="commentlist">comments</article>"#);
        assert!(!comments_rule_1(&doc.select("article")));
    }

    #[test]
    fn test_comments_rule_2_comments_prefix() {
        let doc = dom::parse(r#"<div id="comments-section">comments</div>"#);
        assert!(comments_rule_2(&doc.select("div")));
    }

    #[test]
    fn test_comments_rule_2_comments_class() {
        let doc = dom::parse(r#"<section class="Comments">comments</section>"#);
        assert!(comments_rule_2(&doc.select("section")));
    }

    #[test]
    fn test_comments_rule_2_comment_dash() {
        let doc = dom::parse(r#"<div class="comment-area">area</div>"#);
        assert!(comments_rule_2(&doc.select("div")));
    }

    #[test]
    fn test_comments_rule_2_article_comments() {
        let doc = dom::parse(r#"<ul class="article-comments">comments</ul>"#);
        assert!(comments_rule_2(&doc.select("ul")));
    }

    #[test]
    fn test_comments_rule_2_wrong_tag() {
        let doc = dom::parse(r#"<article id="comments">comments</article>"#);
        assert!(!comments_rule_2(&doc.select("article")));
    }

    #[test]
    fn test_comments_rule_3_disqus() {
        let doc = dom::parse(r#"<div id="disqus_thread">disqus</div>"#);
        assert!(comments_rule_3(&doc.select("div")));
    }

    #[test]
    fn test_comments_rule_3_dsq_comments() {
        let doc = dom::parse(r#"<section id="dsq_comments">dsq</section>"#);
        assert!(comments_rule_3(&doc.select("section")));
    }

    #[test]
    fn test_comments_rule_3_comol() {
        let doc = dom::parse(r#"<div id="comol-comments">comol</div>"#);
        assert!(comments_rule_3(&doc.select("div")));
    }

    #[test]
    fn test_comments_rule_3_wrong_tag() {
        let doc = dom::parse(r#"<article id="disqus_thread">disqus</article>"#);
        assert!(!comments_rule_3(&doc.select("article")));
    }

    #[test]
    fn test_comments_rule_4_social() {
        let doc = dom::parse(r#"<div id="social-comments">social</div>"#);
        assert!(comments_rule_4(&doc.select("div")));
    }

    #[test]
    fn test_comments_rule_4_comment_class() {
        let doc = dom::parse(r#"<section class="user-comment">comment</section>"#);
        assert!(comments_rule_4(&doc.select("section")));
    }

    #[test]
    fn test_comments_rule_4_wrong_tag() {
        let doc = dom::parse(r#"<article class="comment">comment</article>"#);
        assert!(!comments_rule_4(&doc.select("article")));
    }

    // ============================================================
    // COMMENT DISCARD TESTS
    // ============================================================

    #[test]
    fn test_discarded_respond() {
        let doc = dom::parse(r#"<div id="respond">form</div>"#);
        assert!(discarded_comments_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_discarded_respond_section() {
        let doc = dom::parse(r#"<section id="respond-form">form</section>"#);
        assert!(discarded_comments_rule_1(&doc.select("section")));
    }

    #[test]
    fn test_discarded_respond_wrong_tag() {
        let doc = dom::parse(r#"<article id="respond">respond</article>"#);
        assert!(!discarded_comments_rule_1(&doc.select("article")));
    }

    #[test]
    fn test_discarded_cite() {
        let doc = dom::parse("<cite>quoted text</cite>");
        assert!(discarded_comments_rule_2(&doc.select("cite")));
    }

    #[test]
    fn test_discarded_quote() {
        let doc = dom::parse("<quote>quoted text</quote>");
        assert!(discarded_comments_rule_2(&doc.select("quote")));
    }

    #[test]
    fn test_discarded_not_cite_or_quote() {
        let doc = dom::parse("<blockquote>quoted</blockquote>");
        assert!(!discarded_comments_rule_2(&doc.select("blockquote")));
    }

    #[test]
    fn test_discarded_comments_title_exact() {
        let doc = dom::parse(r#"<div class="comments-title">title</div>"#);
        assert!(discarded_comments_rule_3(&doc.select("div")));
    }

    #[test]
    fn test_discarded_comments_title_contains() {
        let doc = dom::parse(r#"<h3 class="entry-comments-title">title</h3>"#);
        assert!(discarded_comments_rule_3(&doc.select("h3")));
    }

    #[test]
    fn test_discarded_nocomments() {
        let doc = dom::parse(r#"<div class="nocomments">no comments</div>"#);
        assert!(discarded_comments_rule_3(&doc.select("div")));
    }

    #[test]
    fn test_discarded_reply_prefix() {
        let doc = dom::parse(r#"<span id="reply-link">reply</span>"#);
        assert!(discarded_comments_rule_3(&doc.select("span")));
    }

    #[test]
    fn test_discarded_reply_dash() {
        let doc = dom::parse(r#"<a class="comment-reply-link">reply</a>"#);
        assert!(discarded_comments_rule_3(&doc.select("a")));
    }

    #[test]
    fn test_discarded_message() {
        let doc = dom::parse(r#"<div class="message">message</div>"#);
        assert!(discarded_comments_rule_3(&doc.select("div")));
    }

    #[test]
    fn test_discarded_signin() {
        let doc = dom::parse(r#"<div class="signin-prompt">sign in</div>"#);
        assert!(discarded_comments_rule_3(&doc.select("div")));
    }

    #[test]
    fn test_discarded_akismet() {
        let doc = dom::parse(r#"<div id="akismet-form">spam check</div>"#);
        assert!(discarded_comments_rule_3(&doc.select("div")));
    }

    #[test]
    fn test_discarded_display_none() {
        let doc = dom::parse(r#"<div style="display:none">hidden</div>"#);
        assert!(discarded_comments_rule_3(&doc.select("div")));
    }

    // ============================================================
    // COMMENT REMOVAL TESTS
    // ============================================================

    #[test]
    fn test_removed_comments_prefix() {
        let doc = dom::parse(r#"<div id="comments">comments</div>"#);
        assert!(removed_comments_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_removed_comments_case_insensitive() {
        let doc = dom::parse(r#"<section class="Comments-section">comments</section>"#);
        assert!(removed_comments_rule_1(&doc.select("section")));
    }

    #[test]
    fn test_removed_comment_capital_c() {
        let doc = dom::parse(r#"<div id="Comment-area">comments</div>"#);
        assert!(removed_comments_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_removed_article_comments() {
        let doc = dom::parse(r#"<ul class="article-comments">comments</ul>"#);
        assert!(removed_comments_rule_1(&doc.select("ul")));
    }

    #[test]
    fn test_removed_post_comments() {
        let doc = dom::parse(r#"<dl class="post-comments">comments</dl>"#);
        assert!(removed_comments_rule_1(&doc.select("dl")));
    }

    #[test]
    fn test_removed_disqus() {
        let doc = dom::parse(r#"<div id="disqus_thread">disqus</div>"#);
        assert!(removed_comments_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_removed_dsq_comments() {
        let doc = dom::parse(r#"<section id="dsq-comments">dsq</section>"#);
        assert!(removed_comments_rule_1(&doc.select("section")));
    }

    #[test]
    fn test_removed_comol() {
        let doc = dom::parse(r#"<div id="comol">comol</div>"#);
        assert!(removed_comments_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_removed_wrong_tag() {
        let doc = dom::parse(r#"<article class="comments">comments</article>"#);
        assert!(!removed_comments_rule_1(&doc.select("article")));
    }

    // ============================================================
    // HELPER FUNCTION TESTS
    // ============================================================

    #[test]
    fn test_find_comments() {
        let doc = dom::parse(
            r#"
            <div>
                <article>content</article>
                <div id="comments">comments</div>
                <div id="disqus_thread">disqus</div>
            </div>
        "#,
        );
        let root = doc.select("div").first();
        let comments = find_comments(&root);
        assert_eq!(comments.len(), 2);
    }

    #[test]
    fn test_find_comments_empty() {
        let doc = dom::parse(
            r#"
            <div>
                <article>content</article>
                <p>text</p>
            </div>
        "#,
        );
        let root = doc.select("div").first();
        let comments = find_comments(&root);
        assert_eq!(comments.len(), 0);
    }

    #[test]
    fn test_is_comment_section() {
        let doc = dom::parse(r#"<div class="post-comments">comments</div>"#);
        assert!(is_comment_section(&doc.select("div")));
    }

    #[test]
    fn test_is_not_comment_section() {
        let doc = dom::parse(r#"<article>content</article>"#);
        assert!(!is_comment_section(&doc.select("article")));
    }

    #[test]
    fn test_is_comment_debris() {
        let doc = dom::parse(r#"<div id="respond">form</div>"#);
        assert!(is_comment_debris(&doc.select("div")));
    }

    #[test]
    fn test_is_not_comment_debris() {
        let doc = dom::parse(r#"<div>normal content</div>"#);
        assert!(!is_comment_debris(&doc.select("div")));
    }

    #[test]
    fn test_should_remove_comments() {
        let doc = dom::parse(r#"<section id="comment-section">comments</section>"#);
        assert!(should_remove_comments(&doc.select("section")));
    }

    #[test]
    fn test_should_not_remove_comments() {
        let doc = dom::parse(r#"<article>content</article>"#);
        assert!(!should_remove_comments(&doc.select("article")));
    }
}
