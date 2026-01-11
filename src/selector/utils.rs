//! Utility functions for selector pattern matching
//!
//! Provides helper functions used throughout selector rules for common operations
//! like attribute access, string matching, and DOM traversal.
//!
//! Port of `internal/selector/utils.go`.

use crate::dom;
use dom_query::Selection;

// === DOM Traversal ===

/// Get ancestors of a node with a specific tag name
///
/// Returns all ancestors (parents, grandparents, etc.) that have the specified tag name,
/// ordered from nearest to furthest.
///
/// Go equivalent: `getNodeAncestors(node, ancestorTag)`
///
/// # Example
///
/// ```rust
/// use rs_trafilatura::selector::utils;
/// use rs_trafilatura::dom;
///
/// let doc = dom::parse(r#"
///     <div>
///         <article>
///             <div>
///                 <p id="target">text</p>
///             </div>
///         </article>
///     </div>
/// "#);
/// let p = doc.select("#target");
///
/// let div_ancestors = utils::get_node_ancestors(&p, "div");
/// assert_eq!(div_ancestors.len(), 2); // Two divs above p
/// ```
#[must_use]
pub fn get_node_ancestors<'a>(sel: &Selection<'a>, ancestor_tag: &str) -> Vec<Selection<'a>> {
    let mut ancestors = Vec::new();
    let mut current = dom::parent(sel);

    while current.exists() {
        if let Some(tag) = dom::tag_name(&current) {
            if tag == ancestor_tag {
                ancestors.push(current.clone());
            }
        }
        current = dom::parent(&current);
    }

    ancestors
}

// === String Utilities ===

/// Case-sensitive contains check
///
/// Go equivalent: `contains(s, substr)`
#[inline]
#[must_use]
pub fn contains(haystack: &str, needle: &str) -> bool {
    haystack.contains(needle)
}

/// Case-sensitive starts-with check
///
/// Go equivalent: `startsWith(s, prefix)`
#[inline]
#[must_use]
pub fn starts_with(s: &str, prefix: &str) -> bool {
    s.starts_with(prefix)
}

/// Convert to lowercase
///
/// Go equivalent: `lower(s)`
#[inline]
#[must_use]
pub fn lower(s: &str) -> String {
    s.to_lowercase()
}

// === Element Attribute Helpers ===

/// Get element ID attribute (empty string if missing)
///
/// Go pattern: `id := getAttr(node, "id"); if id == "" { ... }`
#[inline]
#[must_use]
pub fn id(sel: &Selection) -> String {
    dom::id(sel).unwrap_or_default()
}

/// Get element class attribute (empty string if missing)
///
/// Go pattern: `class := getAttr(node, "class"); if class == "" { ... }`
#[inline]
#[must_use]
pub fn class(sel: &Selection) -> String {
    dom::class_name(sel).unwrap_or_default()
}

/// Get any attribute (empty string if missing)
///
/// Go equivalent: `getAttr(node, name)`
#[inline]
#[must_use]
pub fn attr(sel: &Selection, name: &str) -> String {
    dom::get_attribute(sel, name).unwrap_or_default()
}

/// Get tag name (empty string if missing)
///
/// Go pattern: `tagName := dom.TagName(node)`
#[inline]
#[must_use]
pub fn tag(sel: &Selection) -> String {
    dom::tag_name(sel).unwrap_or_default()
}

/// Combine id and class for efficient multi-attribute checks
///
/// Many go-trafilatura rules use this pattern:
/// ```go
/// idClass := id + class
/// if contains(idClass, "article") || contains(idClass, "post") { ... }
/// ```
///
/// # Example
///
/// ```rust
/// use rs_trafilatura::selector::utils;
/// use rs_trafilatura::dom;
///
/// let doc = dom::parse(r#"<div id="main" class="content">text</div>"#);
/// let div = doc.select("div");
///
/// let combined = utils::id_class(&div);
/// assert!(utils::contains(&combined, "main"));
/// assert!(utils::contains(&combined, "content"));
/// ```
#[inline]
#[must_use]
pub fn id_class(sel: &Selection) -> String {
    format!("{}{}", id(sel), class(sel))
}

// === Element Type Checks ===

/// Check if element has a specific tag name
///
/// Go pattern: `if dom.TagName(node) == "article" { ... }`
#[inline]
#[must_use]
pub fn is_tag(sel: &Selection, expected: &str) -> bool {
    tag(sel) == expected
}

/// Check if element is one of the specified tags
///
/// Go pattern: `if tagName == "article" || tagName == "div" || tagName == "section" { ... }`
///
/// # Example
///
/// ```rust
/// use rs_trafilatura::selector::utils;
/// use rs_trafilatura::dom;
///
/// let doc = dom::parse("<article>content</article>");
/// let article = doc.select("article");
///
/// assert!(utils::is_one_of_tags(&article, &["article", "div", "section"]));
/// assert!(!utils::is_one_of_tags(&article, &["div", "span", "p"]));
/// ```
#[inline]
#[must_use]
pub fn is_one_of_tags(sel: &Selection, tags: &[&str]) -> bool {
    let t = tag(sel);
    tags.contains(&t.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom;

    // === String Utilities Tests ===

    #[test]
    fn test_contains_case_sensitive() {
        assert!(contains("hello world", "world"));
        assert!(!contains("hello world", "WORLD")); // case-sensitive
        assert!(contains("article-content", "content"));
        assert!(!contains("article", "Article"));
    }

    #[test]
    fn test_starts_with_case_sensitive() {
        assert!(starts_with("article-content", "article"));
        assert!(!starts_with("my-article", "article"));
        assert!(starts_with("Content", "Content"));
        assert!(!starts_with("content", "Content"));
    }

    #[test]
    fn test_lower() {
        assert_eq!(lower("HELLO"), "hello");
        assert_eq!(lower("MiXeD"), "mixed");
        assert_eq!(lower("already lowercase"), "already lowercase");
    }

    // === Attribute Access Tests ===

    #[test]
    fn test_id_returns_value_or_empty() {
        let doc = dom::parse(r#"<div id="main">text</div>"#);
        let with_id = doc.select("div");
        assert_eq!(id(&with_id), "main");

        let doc2 = dom::parse("<div>no id</div>");
        let no_id = doc2.select("div");
        assert_eq!(id(&no_id), "");
    }

    #[test]
    fn test_class_returns_value_or_empty() {
        let doc = dom::parse(r#"<div class="content main">text</div>"#);
        let with_class = doc.select("div");
        assert_eq!(class(&with_class), "content main");

        let doc2 = dom::parse("<div>no class</div>");
        let no_class = doc2.select("div");
        assert_eq!(class(&no_class), "");
    }

    #[test]
    fn test_attr_returns_value_or_empty() {
        let doc = dom::parse(r#"<div data-role="article">text</div>"#);
        let div = doc.select("div");

        assert_eq!(attr(&div, "data-role"), "article");
        assert_eq!(attr(&div, "nonexistent"), "");
    }

    #[test]
    fn test_tag_returns_lowercase_tag_name() {
        let doc = dom::parse("<ARTICLE>content</ARTICLE>");
        let article = doc.select("article");
        assert_eq!(tag(&article), "article");

        let doc2 = dom::parse("<div>text</div>");
        let div = doc2.select("div");
        assert_eq!(tag(&div), "div");
    }

    #[test]
    fn test_id_class_combines_both_attributes() {
        let doc = dom::parse(r#"<div id="main" class="content">test</div>"#);
        let div = doc.select("div");

        let combined = id_class(&div);
        assert!(contains(&combined, "main"));
        assert!(contains(&combined, "content"));
        assert_eq!(combined, "maincontent");
    }

    #[test]
    fn test_id_class_handles_missing_attributes() {
        let doc = dom::parse(r#"<div id="only-id">test</div>"#);
        let div = doc.select("div");
        assert_eq!(id_class(&div), "only-id");

        let doc2 = dom::parse(r#"<div class="only-class">test</div>"#);
        let div2 = doc2.select("div");
        assert_eq!(id_class(&div2), "only-class");

        let doc3 = dom::parse("<div>neither</div>");
        let div3 = doc3.select("div");
        assert_eq!(id_class(&div3), "");
    }

    // === Element Type Tests ===

    #[test]
    fn test_is_tag() {
        let doc = dom::parse("<article>content</article>");
        let article = doc.select("article");

        assert!(is_tag(&article, "article"));
        assert!(!is_tag(&article, "div"));
        assert!(!is_tag(&article, "ARTICLE")); // case-sensitive
    }

    #[test]
    fn test_is_one_of_tags() {
        let doc = dom::parse("<article>content</article>");
        let article = doc.select("article");

        assert!(is_one_of_tags(&article, &["article", "div", "section"]));
        assert!(is_one_of_tags(&article, &["article"]));
        assert!(!is_one_of_tags(&article, &["div", "span", "p"]));
        assert!(!is_one_of_tags(&article, &[])); // Empty list
    }

    // === Ancestor Traversal Tests ===

    #[test]
    fn test_get_node_ancestors_finds_all_matching() {
        let doc = dom::parse(r#"
            <div>
                <article>
                    <div>
                        <p id="target">content</p>
                    </div>
                </article>
            </div>
        "#);
        let p = doc.select("#target");

        let div_ancestors = get_node_ancestors(&p, "div");
        assert_eq!(div_ancestors.len(), 2); // Two divs above p
    }

    #[test]
    fn test_get_node_ancestors_preserves_order() {
        let doc = dom::parse(r#"
            <div id="outer">
                <section>
                    <div id="inner">
                        <p id="target">content</p>
                    </div>
                </section>
            </div>
        "#);
        let p = doc.select("#target");

        let div_ancestors = get_node_ancestors(&p, "div");
        assert_eq!(div_ancestors.len(), 2);

        // First ancestor should be nearest (inner)
        assert_eq!(id(&div_ancestors[0]), "inner");
        // Second ancestor should be furthest (outer)
        assert_eq!(id(&div_ancestors[1]), "outer");
    }

    #[test]
    fn test_get_node_ancestors_empty_when_no_matches() {
        let doc = dom::parse(r#"
            <section>
                <article>
                    <p id="target">content</p>
                </article>
            </section>
        "#);
        let p = doc.select("#target");

        let div_ancestors = get_node_ancestors(&p, "div");
        assert_eq!(div_ancestors.len(), 0); // No div ancestors
    }

    #[test]
    fn test_get_node_ancestors_stops_at_root() {
        let doc = dom::parse(r#"<p id="target">content</p>"#);
        let p = doc.select("#target");

        let ancestors = get_node_ancestors(&p, "div");
        assert_eq!(ancestors.len(), 0); // No ancestors
    }

    // === Integration Tests ===

    #[test]
    fn test_combined_pattern_matching() {
        let doc = dom::parse(r#"
            <div>
                <article id="main-article" class="post-content">Match 1</article>
                <div id="sidebar-widget" class="advertisement">No match</div>
                <section id="related-posts" class="post-list">Match 2</section>
            </div>
        "#);
        let root = doc.select("div").first();

        // Simulate a typical go-trafilatura selector rule pattern
        let matches: Vec<Selection> = root
            .select("*")
            .nodes()
            .iter()
            .map(|node| Selection::from(*node))
            .filter(|sel| {
                let id_class_combined = id_class(sel);
                contains(&id_class_combined, "post") || contains(&id_class_combined, "article")
            })
            .collect();

        assert_eq!(matches.len(), 2); // article and section
    }
}
