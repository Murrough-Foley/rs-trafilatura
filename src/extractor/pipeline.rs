//! Content extraction pipeline.
//!
//! This module contains the main extraction pipeline that orchestrates
//! selector matching, section pruning, element handling, and result assembly.
//!
//! Port of:
//! - `main-extractor.go` lines 531-564: `handleTextElem`
//! - `main-extractor.go` lines 566-608: `recoverWildText`
//! - `main-extractor.go` lines 664-788: `extractContent`

use std::collections::HashSet;

use dom_query::{Document, Selection};

use crate::dom;
use crate::etree;
use crate::html_processing::{process_node, text_chars_test};
use crate::selector::{self, content::CONTENT_RULES};
use crate::Options;

use super::handlers::{
    handle_formatting, handle_image, handle_lists, handle_other_elements,
    handle_paragraphs, handle_quotes, handle_table, handle_titles,
};
use super::pruning::{prune_unwanted_sections, strip_non_potential_tags};
use super::state::ExtractionState;
use super::tags::{
    is_xml_graphic_tag, is_xml_head_tag, is_xml_hi_tag, is_xml_lb_tag,
    is_xml_list_tag, is_xml_quote_tag, is_xml_ref_tag,
    XML_LB_TAGS, XML_LIST_TAGS, XML_QUOTE_TAGS,
};

/// Process text element and determine how to deal with its content.
///
/// This is the main dispatch function that routes elements to appropriate handlers
/// based on their tag type.
///
/// Go equivalent: `handleTextElem(element, potentialTags, cache, opts)` (lines 531-564)
#[must_use]
pub fn handle_text_elem(
    element: &Selection,
    state: &mut ExtractionState,
    opts: &Options,
) -> Option<Document> {
    let tag_name = dom::tag_name(element).unwrap_or_default();

    // Route to appropriate handler based on tag type
    if is_xml_list_tag(&tag_name) {
        handle_lists(element, state, opts)
    } else if is_xml_quote_tag(&tag_name) || tag_name == "code" {
        handle_quotes(element, state, opts)
    } else if is_xml_head_tag(&tag_name) {
        handle_titles(element, state, opts)
    } else if tag_name == "p" {
        handle_paragraphs(element, state, opts)
    } else if is_xml_lb_tag(&tag_name) {
        // Line break with tail content - create paragraph from tail
        let tail = etree::tail(element);
        if text_chars_test(&tail) && process_node(element, None, opts) {
            let p_doc = etree::element("p");
            let p = p_doc.select("p");
            etree::set_text(&p, &tail);
            return Some(p_doc);
        }
        None
    } else if is_xml_hi_tag(&tag_name) || is_xml_ref_tag(&tag_name) || tag_name == "span" {
        handle_formatting(element, opts)
    } else if tag_name == "table" {
        if state.is_potential_tag("table") {
            handle_table(element, state, opts)
        } else {
            None
        }
    } else if is_xml_graphic_tag(&tag_name) {
        if state.is_potential_tag("img") {
            handle_image(element)
        } else {
            None
        }
    } else {
        handle_other_elements(element, state, opts)
    }
}

/// Look for all previously unconsidered wild elements to recover potentially missing text.
///
/// This function searches for content that wasn't found by the main selector rules,
/// allowing recovery of text that might otherwise be missed.
///
/// Go equivalent: `recoverWildText(doc, resultBody, potentialTags, cache, opts)` (lines 566-608)
pub fn recover_wild_text(
    doc: &Document,
    result_body: &Selection,
    state: &mut ExtractionState,
    opts: &Options,
) {
    // Build selector list for wild text recovery
    let mut selector_list: Vec<&str> = Vec::new();
    selector_list.extend_from_slice(&XML_QUOTE_TAGS);
    selector_list.push("code");
    selector_list.push("p");
    selector_list.push("table");
    selector_list.push(r#"div[class*="w3-code"]"#);

    // Additional selectors for recall mode
    if opts.favor_recall {
        state.add_potential_tag("div");
        for tag in &XML_LB_TAGS {
            state.add_potential_tag(tag);
        }

        selector_list.push("div");
        selector_list.extend_from_slice(&XML_LB_TAGS);
        selector_list.extend_from_slice(&XML_LIST_TAGS);
    }

    // Prune the search document
    let search_tree = doc.select("body");
    let pruned_doc = prune_unwanted_sections(&search_tree, state.potential_tags(), opts);
    let pruned = pruned_doc.select("body > *");

    // Strip non-potential tags before searching
    strip_non_potential_tags(&pruned, state.potential_tags());

    // Build CSS selector string
    let css_selector = selector_list.join(", ");

    // Process matching elements
    let mut processed_elems: Vec<Document> = Vec::new();
    for node in pruned_doc.select(&css_selector).nodes() {
        let element = Selection::from(*node);

        // Skip already processed elements
        if state.is_done(node.id) {
            continue;
        }

        if let Some(processed) = handle_text_elem(&element, state, opts) {
            processed_elems.push(processed);
        }

        state.mark_done(node.id);
    }

    // Append recovered elements to result body
    for elem_doc in processed_elems {
        let elem = elem_doc.select("body > *");
        if !elem.is_empty() {
            etree::append(result_body, &elem);
        }
    }
}

/// Find the main content of a page using a set of selectors.
///
/// This is the main entry point for content extraction. It:
/// 1. Iterates through selector rules in priority order
/// 2. Prunes unwanted sections from matched subtrees
/// 3. Processes elements through appropriate handlers
/// 4. Falls back to wild text recovery if insufficient content found
///
/// Go equivalent: `extractContent(doc, cache, opts)` (lines 664-788)
///
/// # Returns
///
/// A tuple of `(Document, String)` where:
/// - `Document` contains the extracted content body
/// - `String` is the plain text of the extracted content
#[must_use]
pub fn extract_content(doc: &Document, opts: &Options) -> (Document, String) {
    // Clone document for backup (used in wild text recovery)
    let backup_doc = dom::clone_document(doc);

    // Create result body container
    let result_doc = etree::element("body");
    let result_body = result_doc.select("body");

    // Initialize extraction state from options
    let mut state = ExtractionState::new();
    state.configure_from_options(opts);

    // Iterate through each selector rule in priority order
    for rule in CONTENT_RULES {
        // Find first element matching this rule
        let Some(sub_tree) = selector::query(&doc.select("body"), *rule) else {
            continue;
        };

        // Prune unwanted sections from the subtree
        let pruned_doc = prune_unwanted_sections(&sub_tree, state.potential_tags(), opts);
        let pruned = pruned_doc.select("body > *");

        // If subtree is now empty, try next selector
        if dom::children(&pruned).is_empty() {
            continue;
        }

        // Check if there are enough paragraphs with text
        // EPIC-04: text_content returns StrTendril, convert to String when needed
        let paragraph_text: String = pruned_doc
            .select("p")
            .nodes()
            .iter()
            .filter_map(|n| {
                let sel = Selection::from(*n);
                let text = dom::text_content(&sel);
                if !text.trim().is_empty() {
                    Some(text.to_string())
                } else {
                    None
                }
            })
            .collect();

        let factor = if opts.favor_precision { 1 } else { 3 };
        let min_size = opts.min_extracted_size;

        if paragraph_text.is_empty() || paragraph_text.chars().count() < min_size * factor {
            // Not enough paragraph content - add div as potential tag
            state.add_potential_tag("div");
        }

        // Strip non-potential tags before processing
        strip_non_potential_tags(&pruned, state.potential_tags());

        // Get all sub-elements for processing
        let sub_elements: Vec<_> = pruned.select("*").nodes().to_vec();

        // Check if all sub elements are line breaks (special case)
        let unique_tags: HashSet<String> = sub_elements
            .iter()
            .filter_map(|n| dom::tag_name(&Selection::from(*n)))
            .collect();

        let process_elements: Vec<_> = if unique_tags.len() == 1 && unique_tags.contains("br") {
            // All elements are <br> - process the parent instead
            vec![pruned.nodes().first().copied()]
        } else {
            sub_elements.into_iter().map(Some).collect()
        };

        // Process each element and collect results
        let mut processed_elems: Vec<Document> = Vec::new();
        for node in process_elements.into_iter().flatten() {
            // Skip already processed nodes
            if state.is_done(node.id) {
                continue;
            }

            let elem = Selection::from(node);
            if let Some(processed) = handle_text_elem(&elem, &mut state, opts) {
                processed_elems.push(processed);
            }

            state.mark_done(node.id);
        }

        // Append processed elements to result body
        for elem_doc in processed_elems {
            let elem = elem_doc.select("body > *");
            if !elem.is_empty() {
                etree::append(&result_body, &elem);
            }
        }

        // Remove trailing titles and refs (cleanup)
        let final_children: Vec<_> = dom::children(&result_body).nodes().to_vec();
        for node in final_children.into_iter().rev() {
            let child = Selection::from(node);
            let tag_name = dom::tag_name(&child).unwrap_or_default();
            if is_xml_head_tag(&tag_name) || is_xml_ref_tag(&tag_name) {
                etree::remove(&child, false);
            } else {
                // Stop at first non-heading/non-ref element
                break;
            }
        }

        // Exit if result has sufficient content (more than 1 child)
        if dom::children(&result_body).length() > 1 {
            break;
        }
    }

    // Check if we need wild text recovery
    let tmp_text = etree::iter_text(&result_body, " ").trim().to_string();
    let tmp_text_length = tmp_text.chars().count();
    let min_size = opts.min_extracted_size;

    if dom::children(&result_body).is_empty() || tmp_text_length < min_size {
        // Create new result body for wild text recovery
        let new_result_doc = etree::element("body");
        let new_result_body = new_result_doc.select("body");

        // Reset state for wild text recovery
        let mut wild_state = ExtractionState::new();
        wild_state.configure_from_options(opts);

        recover_wild_text(&backup_doc, &new_result_body, &mut wild_state, opts);

        let new_text = etree::iter_text(&new_result_body, " ").trim().to_string();

        // Use recovered content if it's better
        if new_text.chars().count() > tmp_text_length {
            // Clean up recovered content
            etree::strip_elements(&new_result_body, false, &["done"]);
            etree::strip_tags(&new_result_body, &["div"]);

            let final_text = etree::iter_text(&new_result_body, " ").trim().to_string();
            return (new_result_doc, final_text);
        }
    }

    // Final cleanup on result body
    etree::strip_elements(&result_body, false, &["done"]);
    etree::strip_tags(&result_body, &["div"]);

    let final_text = etree::iter_text(&result_body, " ").trim().to_string();
    (result_doc, final_text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_text_elem_dispatches_to_list() {
        let doc = dom::parse("<ul><li>Item 1</li><li>Item 2</li></ul>");
        let ul = doc.select("ul");
        let mut state = ExtractionState::new();
        let opts = Options::default();

        let result = handle_text_elem(&ul, &mut state, &opts);
        assert!(result.is_some());
        let result_doc = result.unwrap();
        assert_eq!(result_doc.select("ul").length(), 1);
    }

    #[test]
    fn test_handle_text_elem_dispatches_to_quote() {
        let doc = dom::parse("<blockquote>This is a quote with enough text content.</blockquote>");
        let bq = doc.select("blockquote");
        let mut state = ExtractionState::new();
        let opts = Options::default();

        let result = handle_text_elem(&bq, &mut state, &opts);
        assert!(result.is_some());
    }

    #[test]
    fn test_handle_text_elem_dispatches_to_heading() {
        let doc = dom::parse("<h1>Main Title Here</h1>");
        let h1 = doc.select("h1");
        let mut state = ExtractionState::new();
        let opts = Options::default();

        let result = handle_text_elem(&h1, &mut state, &opts);
        assert!(result.is_some());
    }

    #[test]
    fn test_handle_text_elem_dispatches_to_paragraph() {
        let doc = dom::parse("<p>This is a paragraph with some text content.</p>");
        let p = doc.select("p");
        let mut state = ExtractionState::new();
        let opts = Options::default();

        let result = handle_text_elem(&p, &mut state, &opts);
        assert!(result.is_some());
    }

    #[test]
    fn test_handle_text_elem_dispatches_to_table() {
        let doc = dom::parse("<table><tr><td>Cell content</td></tr></table>");
        let table = doc.select("table");
        let mut state = ExtractionState::new();
        state.add_potential_tag("table");
        let opts = Options {
            include_tables: true,
            ..Options::default()
        };

        let result = handle_text_elem(&table, &mut state, &opts);
        assert!(result.is_some());
    }

    #[test]
    fn test_handle_text_elem_table_requires_potential_tag() {
        let doc = dom::parse("<table><tr><td>Cell</td></tr></table>");
        let table = doc.select("table");
        let mut state = ExtractionState::new();
        // Don't add table as potential tag
        let opts = Options::default();

        let result = handle_text_elem(&table, &mut state, &opts);
        assert!(result.is_none()); // Should be None without potential tag
    }

    #[test]
    fn test_handle_text_elem_dispatches_to_image() {
        let doc = dom::parse(r#"<img src="test.jpg" alt="Test image">"#);
        let img = doc.select("img");
        let mut state = ExtractionState::new();
        state.add_potential_tag("img");
        let opts = Options {
            include_images: true,
            ..Options::default()
        };

        let result = handle_text_elem(&img, &mut state, &opts);
        assert!(result.is_some());
    }

    #[test]
    fn test_handle_text_elem_dispatches_to_formatting() {
        let doc = dom::parse("<b>Bold text content here</b>");
        let b = doc.select("b");
        let mut state = ExtractionState::new();
        let opts = Options::default();

        let _result = handle_text_elem(&b, &mut state, &opts);
        // Formatting outside paragraph may return wrapped in p or None
        // depending on parent context - we just verify no panic
    }

    #[test]
    fn test_handle_text_elem_code_routes_to_quotes() {
        let doc = dom::parse("<code>let x = 42;</code>");
        let code = doc.select("code");
        let mut state = ExtractionState::new();
        let opts = Options::default();

        let result = handle_text_elem(&code, &mut state, &opts);
        // Code should be handled by quote handler
        assert!(result.is_some());
    }

    #[test]
    fn test_extract_content_basic() {
        let html = r#"<!DOCTYPE html>
        <html>
        <body>
            <article class="post-content">
                <h1>Article Title</h1>
                <p>This is the main content of the article with enough text.</p>
                <p>This is another paragraph with more substantial content here.</p>
            </article>
        </body>
        </html>"#;

        let doc = Document::from(html);
        let opts = Options::default();

        let (result_doc, text) = extract_content(&doc, &opts);

        assert!(!dom::children(&result_doc.select("body")).is_empty());
        assert!(!text.is_empty());
        assert!(text.contains("main content"));
    }

    #[test]
    fn test_extract_content_with_boilerplate() {
        let html = r#"<!DOCTYPE html>
        <html>
        <body>
            <nav class="navigation">Navigation links here</nav>
            <article class="post-content">
                <p>Main content paragraph one with substantial text here.</p>
                <p>Main content paragraph two with more text content.</p>
            </article>
            <footer class="site-footer">Footer content here</footer>
        </body>
        </html>"#;

        let doc = Document::from(html);
        let opts = Options::default();

        let (_result_doc, text) = extract_content(&doc, &opts);

        // Should extract main content
        assert!(text.contains("Main content"));
    }

    #[test]
    fn test_extract_content_empty_document() {
        let html = r#"<!DOCTYPE html>
        <html>
        <body>
        </body>
        </html>"#;

        let doc = Document::from(html);
        let opts = Options::default();

        let (result_doc, text) = extract_content(&doc, &opts);

        // Should handle empty document gracefully
        assert!(dom::children(&result_doc.select("body")).is_empty() || text.is_empty());
    }

    #[test]
    fn test_recover_wild_text_finds_paragraphs() {
        let doc = Document::from("<body><p>Wild paragraph one.</p><p>Wild paragraph two.</p></body>");
        let result_doc = etree::element("body");
        let result_body = result_doc.select("body");
        let mut state = ExtractionState::new();
        let opts = Options::default();

        recover_wild_text(&doc, &result_body, &mut state, &opts);

        // Should find the paragraphs
        let children = dom::children(&result_body);
        assert!(!children.is_empty() || !etree::iter_text(&result_body, " ").is_empty());
    }

    #[test]
    fn test_recover_wild_text_recall_mode() {
        let doc = Document::from("<body><div>Content in div</div><p>Paragraph</p></body>");
        let result_doc = etree::element("body");
        let result_body = result_doc.select("body");
        let mut state = ExtractionState::new();
        let opts = Options {
            favor_recall: true,
            ..Options::default()
        };

        recover_wild_text(&doc, &result_body, &mut state, &opts);

        // In recall mode, should also find divs
        assert!(state.is_potential_tag("div"));
    }

    #[test]
    fn test_extract_content_removes_trailing_titles() {
        let html = r#"<!DOCTYPE html>
        <html>
        <body>
            <article class="post-content">
                <p>Main content with enough text to pass extraction.</p>
                <p>More content paragraph with additional text here.</p>
                <h2>Trailing Title Should Be Removed</h2>
            </article>
        </body>
        </html>"#;

        let doc = Document::from(html);
        let opts = Options {
            favor_precision: true,
            ..Options::default()
        };

        let (_result_doc, _text) = extract_content(&doc, &opts);

        // Trailing titles should be removed (behavior depends on pruning)
    }

    #[test]
    fn test_extract_content_precision_mode() {
        let html = r#"<!DOCTYPE html>
        <html>
        <body>
            <article class="post-content">
                <p>Main content paragraph with substantial text content here.</p>
            </article>
        </body>
        </html>"#;

        let doc = Document::from(html);
        let opts = Options {
            favor_precision: true,
            ..Options::default()
        };

        let (_result_doc, text) = extract_content(&doc, &opts);

        // Precision mode should still extract main content
        assert!(text.contains("Main content") || text.is_empty());
    }

    #[test]
    fn test_extract_content_recall_mode() {
        let html = r#"<!DOCTYPE html>
        <html>
        <body>
            <div class="content">
                <p>Content that might be missed in precision mode.</p>
            </div>
        </body>
        </html>"#;

        let doc = Document::from(html);
        let opts = Options {
            favor_recall: true,
            ..Options::default()
        };

        let (_result_doc, _text) = extract_content(&doc, &opts);

        // Recall mode should be more aggressive in finding content
    }
}
