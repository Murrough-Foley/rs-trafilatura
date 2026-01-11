//! Selector Infrastructure
//!
//! Provides the foundation for selector rules used in content finding and boilerplate detection.
//! Rules are simple functions that test if a Selection matches certain criteria.
//!
//! Port of `internal/selector/selector.go` and `internal/selector/utils.go`.

use dom_query::Selection;

// Future story modules (placeholders)
pub mod content;    // Story 2.2: Content selector rules
pub mod discard;    // Story 2.3: Overall discard patterns
pub mod precision;  // Story 2.4: Precision/teaser/image discard
pub mod comments;   // Story 2.5: Comment selectors
pub mod meta;       // Story 2.6: Metadata selectors

pub mod utils;

/// A selector rule that tests if a selection matches certain criteria
///
/// Rules are simple predicate functions used throughout the extraction pipeline
/// to identify content, boilerplate, comments, and metadata.
///
/// Go equivalent: `type Rule func(*html.Node) bool`
pub type Rule = fn(&Selection) -> bool;

/// Query for first element matching the rule
///
/// Iterates through all descendants in document order and returns the first
/// element for which the rule returns true.
///
/// Go equivalent: `selector.Query(root, rule)`
///
/// # Example
///
/// ```rust
/// use rs_trafilatura::selector::{self, utils};
/// use rs_trafilatura::dom;
///
/// let doc = dom::parse(r#"<div><p class="content">text</p></div>"#);
/// let root = doc.select("div");
///
/// fn has_content_class(sel: &dom_query::Selection) -> bool {
///     utils::class(sel).contains("content")
/// }
///
/// let result = selector::query(&root, has_content_class);
/// assert!(result.is_some());
/// ```
#[must_use]
pub fn query<'a>(root: &Selection<'a>, rule: Rule) -> Option<Selection<'a>> {
    // Iterate all descendants in document order
    for node in root.select("*").nodes() {
        let sel = Selection::from(*node);
        if rule(&sel) {
            return Some(sel);
        }
    }
    None
}

/// Query for all elements matching the rule
///
/// Iterates through all descendants in document order and collects all
/// elements for which the rule returns true.
///
/// Go equivalent: `selector.QueryAll(root, rule)`
///
/// # Example
///
/// ```rust
/// use rs_trafilatura::selector::{self, utils};
/// use rs_trafilatura::dom;
///
/// let doc = dom::parse(r#"<div><p class="item">1</p><p class="item">2</p></div>"#);
/// let root = doc.select("div");
///
/// fn has_item_class(sel: &dom_query::Selection) -> bool {
///     utils::class(sel).contains("item")
/// }
///
/// let results = selector::query_all(&root, has_item_class);
/// assert_eq!(results.len(), 2);
/// ```
#[must_use]
pub fn query_all<'a>(root: &Selection<'a>, rule: Rule) -> Vec<Selection<'a>> {
    let mut matches = Vec::new();

    // Iterate all descendants in document order
    for node in root.select("*").nodes() {
        let sel = Selection::from(*node);
        if rule(&sel) {
            matches.push(sel);
        }
    }

    matches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom;

    #[test]
    fn test_query_finds_first_match() {
        let doc = dom::parse(r#"
            <div>
                <p class="target">First</p>
                <p class="target">Second</p>
            </div>
        "#);
        let root = doc.select("div");

        fn is_target(sel: &Selection) -> bool {
            utils::class(sel).contains("target")
        }

        let result = query(&root, is_target);
        assert!(result.is_some());
        assert_eq!(dom::text_content(&result.unwrap()), "First".into());
    }

    #[test]
    fn test_query_all_finds_all_matches() {
        let doc = dom::parse(r#"
            <div>
                <p class="target">First</p>
                <span>Not target</span>
                <p class="target">Second</p>
            </div>
        "#);
        let root = doc.select("div");

        fn is_target(sel: &Selection) -> bool {
            utils::class(sel).contains("target")
        }

        let results = query_all(&root, is_target);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_query_returns_none_when_no_match() {
        let doc = dom::parse("<div><p>content</p></div>");
        let root = doc.select("div");

        fn never_matches(_sel: &Selection) -> bool {
            false
        }

        assert!(query(&root, never_matches).is_none());
    }

    #[test]
    fn test_query_all_returns_empty_when_no_matches() {
        let doc = dom::parse("<div><p>content</p></div>");
        let root = doc.select("div");

        fn never_matches(_sel: &Selection) -> bool {
            false
        }

        let results = query_all(&root, never_matches);
        assert!(results.is_empty());
    }

    #[test]
    fn test_query_document_order() {
        let doc = dom::parse(r#"
            <div>
                <section>
                    <p class="match">Deep first</p>
                </section>
                <p class="match">Shallow second</p>
            </div>
        "#);
        let root = doc.select("div");

        fn is_match(sel: &Selection) -> bool {
            utils::class(sel).contains("match")
        }

        // Should return first in document order (deep first)
        let result = query(&root, is_match);
        assert!(result.is_some());
        let text = dom::text_content(&result.unwrap());
        assert!(text.contains("Deep first"));
    }

    #[test]
    fn test_query_all_preserves_document_order() {
        let doc = dom::parse(r#"
            <div>
                <p class="item">1</p>
                <section>
                    <p class="item">2</p>
                </section>
                <p class="item">3</p>
            </div>
        "#);
        let root = doc.select("div");

        fn is_item(sel: &Selection) -> bool {
            utils::class(sel).contains("item")
        }

        let results = query_all(&root, is_item);
        assert_eq!(results.len(), 3);

        // Verify document order
        assert_eq!(dom::text_content(&results[0]), "1".into());
        assert_eq!(dom::text_content(&results[1]), "2".into());
        assert_eq!(dom::text_content(&results[2]), "3".into());
    }

    #[test]
    fn test_rule_can_check_multiple_conditions() {
        let doc = dom::parse(r#"
            <div>
                <article id="main" class="content">Match</article>
                <article class="content">No ID</article>
                <article id="sidebar">No class</article>
            </div>
        "#);
        let root = doc.select("div");

        fn has_both_id_and_class(sel: &Selection) -> bool {
            utils::is_tag(sel, "article")
                && !utils::id(sel).is_empty()
                && !utils::class(sel).is_empty()
        }

        let result = query(&root, has_both_id_and_class);
        assert!(result.is_some());
        assert_eq!(dom::text_content(&result.unwrap()), "Match".into());
    }

    #[test]
    fn test_query_with_tag_filter() {
        let doc = dom::parse(r#"
            <div>
                <p>paragraph</p>
                <article>article element</article>
                <section>section element</section>
            </div>
        "#);
        let root = doc.select("div");

        fn is_article_or_section(sel: &Selection) -> bool {
            utils::is_one_of_tags(sel, &["article", "section"])
        }

        let results = query_all(&root, is_article_or_section);
        assert_eq!(results.len(), 2);
    }
}
