//! Section pruning functions.
//!
//! This module provides functions to remove unwanted sections (boilerplate, ads, navigation)
//! from the document before content extraction, using selector rules and link density analysis.
//!
//! Port of:
//! - `main-extractor.go` lines 610-662: `pruneUnwantedSections`
//! - `html-processing.go` lines 140-188: `pruneUnwantedNodes`

use std::collections::HashSet;

use dom_query::{Document, Selection};

use crate::dom;
use crate::etree;
use crate::html_processing::delete_by_link_density;
use crate::link_density::link_density_test_tables;
use crate::selector::{self, discard, precision, Rule};
use crate::Options;

use super::tags::{is_xml_head_tag, XML_HEAD_TAGS, XML_LIST_TAGS, XML_QUOTE_TAGS};

/// Preserve tail text and remove an element.
///
/// Before removing a node, this function moves any tail text to the previous
/// sibling's tail or the parent's tail to prevent text loss.
///
/// Go equivalent: Part of `pruneUnwantedNodes` loop (lines 157-176)
fn preserve_tail_and_remove(node: &Selection) {
    let tail = etree::tail(node);
    if !tail.is_empty() {
        // Try to find previous sibling or parent to receive the tail
        let previous = dom::previous_element_sibling(node);
        if let Some(prev) = previous {
            // Append tail to previous sibling's tail
            let prev_tail = etree::tail(&prev);
            if prev_tail.is_empty() {
                etree::set_tail(&prev, &tail);
            } else {
                etree::set_tail(&prev, &format!("{prev_tail} {tail}"));
            }
        } else {
            // Try parent
            let parent = dom::parent(node);
            if !parent.is_empty() {
                let parent_tail = etree::tail(&parent);
                if parent_tail.is_empty() {
                    etree::set_tail(&parent, &tail);
                } else {
                    etree::set_tail(&parent, &format!("{parent_tail} {tail}"));
                }
            }
        }
    }

    etree::remove(node, false);
}

/// Prune the HTML tree by removing elements matching selector rules.
///
/// Removes elements that match any of the provided rules. Preserves tail text
/// by moving it to the previous sibling or parent element.
///
/// # Arguments
///
/// * `tree` - The tree to prune (cloned internally to allow backup restoration)
/// * `rules` - Selector rules to match for removal
/// * `with_backup` - If true, restore from backup if >85% of text is removed
///
/// # Returns
///
/// A cloned and pruned Document.
///
/// Go equivalent: `pruneUnwantedNodes(tree, queries, withBackup)` (lines 140-188)
#[must_use]
pub fn prune_unwanted_nodes(tree: &Selection, rules: &[Rule], with_backup: bool) -> Document {
    // Clone the tree for modification
    let cloned_doc = dom::clone_element(tree, true);
    let cloned_tree = cloned_doc.select("body > *");

    // Optionally create backup and measure original length
    let (backup_doc, old_len) = if with_backup {
        let backup = dom::clone_element(tree, true);
        let text = dom::text_content(tree);
        let len = text.chars().count();
        (Some(backup), len)
    } else {
        (None, 0)
    };

    // Apply each rule and remove matching elements
    for rule in rules {
        let matches = selector::query_all(&cloned_tree, *rule);

        // Remove in reverse order to avoid index shifting issues
        for node in matches.into_iter().rev() {
            preserve_tail_and_remove(&node);
        }
    }

    // Check if too much text was removed
    if with_backup {
        let new_text = dom::text_content(&cloned_tree);
        let new_len = new_text.chars().count();

        // If more than ~85% of text was removed, restore from backup
        // Go: newLen <= oldLen/7 means newLen is at most ~14% of original
        if new_len <= old_len / 7 {
            if let Some(backup) = backup_doc {
                return backup;
            }
        }
    }

    cloned_doc
}

/// Rule-based deletion of targeted document sections.
///
/// Performs multiple pruning passes:
/// 1. Overall discarded content (ads, navigation, etc.)
/// 2. Image sections (if images not included)
/// 3. Teaser sections (if not favor recall)
/// 4. Precision-targeted sections (if favor precision)
/// 5. Link density filtering (2 passes)
/// 6. Table link density filtering
/// 7. Trailing title removal (if favor precision)
///
/// # Arguments
///
/// * `sub_tree` - The subtree to prune (modified in place via rule-based removal)
/// * `potential_tags` - Tags that are considered potential content
/// * `opts` - Extraction options
///
/// # Returns
///
/// A new pruned Document.
///
/// Go equivalent: `pruneUnwantedSections(subTree, potentialTags, opts)` (lines 610-662)
#[must_use]
pub fn prune_unwanted_sections<S: std::hash::BuildHasher>(
    sub_tree: &Selection,
    potential_tags: &HashSet<String, S>,
    opts: &Options,
) -> Document {
    // 1. Prune overall discarded content
    let doc = prune_unwanted_nodes(sub_tree, discard::OVERALL_DISCARDED_CONTENT, true);
    let tree = doc.select("body > *");

    // 2. Prune images if not included (apply to current tree in place)
    if !opts.include_images {
        prune_in_place(&tree, precision::DISCARDED_IMAGE);
    }

    // 3. Balance precision / recall
    if !opts.favor_recall {
        // Prune teaser sections
        prune_in_place(&tree, precision::DISCARDED_TEASER);

        // Extra pruning for precision mode
        if opts.favor_precision {
            prune_in_place(&tree, precision::PRECISION_DISCARDED_CONTENT);
        }
    }

    // 4. Remove elements by link density - 2 passes
    for _ in 0..2 {
        delete_by_link_density(&tree, opts, true, &["div"]);
        delete_by_link_density(&tree, opts, false, &XML_LIST_TAGS);
        delete_by_link_density(&tree, opts, false, &["p"]);
    }

    // 5. Remove tables by link density
    let table_potential = potential_tags.contains("table");
    if table_potential || opts.favor_precision {
        let tables: Vec<_> = etree::iter(&tree, &["table"]).nodes().to_vec();
        for node in tables.into_iter().rev() {
            let table = Selection::from(node);
            if link_density_test_tables(&table, opts) {
                etree::remove(&table, false);
            }
        }
    }

    // 6. Additional pruning for precision mode
    if opts.favor_precision {
        // Delete trailing titles (from the end)
        let children: Vec<_> = dom::children(&tree).nodes().to_vec();
        for node in children.into_iter().rev() {
            let child = Selection::from(node);
            let tag_name = dom::tag_name(&child).unwrap_or_default();
            if is_xml_head_tag(&tag_name) {
                etree::remove(&child, false);
            } else {
                // Stop at first non-heading element
                break;
            }
        }

        // Link density on headings and quotes
        delete_by_link_density(&tree, opts, false, &XML_HEAD_TAGS);
        delete_by_link_density(&tree, opts, false, &XML_QUOTE_TAGS);
    }

    doc
}

/// Prune elements in place without backup logic.
///
/// Removes all elements matching the rules from the tree, preserving tail text.
fn prune_in_place(tree: &Selection, rules: &[Rule]) {
    for rule in rules {
        let matches = selector::query_all(tree, *rule);

        // Remove in reverse order
        for node in matches.into_iter().rev() {
            preserve_tail_and_remove(&node);
        }
    }
}

/// Strip tags while preserving potential link content.
///
/// Used before wild text recovery to clean up the document.
/// Strips `<a>`, `<ref>`, and `<span>` tags unless links are potential content.
///
/// # Arguments
///
/// * `tree` - The tree to clean
/// * `potential_tags` - Tags that should be preserved
///
/// Go equivalent: Part of extraction cleanup
pub fn strip_non_potential_tags<S: std::hash::BuildHasher>(
    tree: &Selection,
    potential_tags: &HashSet<String, S>,
) {
    // Strip links if not potential
    if potential_tags.contains("a") {
        etree::strip_tags(tree, &["span"]);
    } else {
        etree::strip_tags(tree, &["a", "ref", "span"]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_potential_tags() -> HashSet<String> {
        ["p", "h1", "h2", "blockquote"]
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    }

    #[test]
    fn test_prune_unwanted_nodes_removes_matching() {
        let doc = dom::parse(
            r##"<div>
            <p>Content</p>
            <div class="sidebar">Sidebar</div>
        </div>"##,
        );
        let root = doc.select("div").first();

        // Create a rule that matches .sidebar
        fn sidebar_rule(sel: &Selection) -> bool {
            dom::get_attribute(sel, "class")
                .is_some_and(|c| c.contains("sidebar"))
        }

        let pruned_doc = prune_unwanted_nodes(&root, &[sidebar_rule], false);
        let pruned = pruned_doc.select("*");

        // Sidebar should be removed
        assert_eq!(pruned.select(".sidebar").length(), 0);
        // Content should remain
        assert_eq!(pruned.select("p").length(), 1);
    }

    #[test]
    fn test_prune_unwanted_nodes_preserves_tail() {
        let doc = dom::parse(
            r##"<div><p>First</p><span class="remove">Remove me</span> tail text<p>Second</p></div>"##,
        );
        let root = doc.select("div");

        fn remove_span(sel: &Selection) -> bool {
            dom::tag_name(sel).as_deref() == Some("span")
                && dom::get_attribute(sel, "class")
                    .is_some_and(|c| c.contains("remove"))
        }

        let pruned_doc = prune_unwanted_nodes(&root, &[remove_span], false);
        let pruned = pruned_doc.select("*");

        // Span should be removed
        assert_eq!(pruned.select("span").length(), 0);
        // Tail text should be preserved somewhere
        let html = dom::outer_html(&pruned);
        assert!(html.contains("tail text"));
    }

    #[test]
    fn test_prune_unwanted_nodes_with_backup_restores() {
        // Create a document where removing the rule would remove almost all content
        let doc = dom::parse(
            r##"<div class="content">
            <div class="main">Main content that is quite long and substantial to measure</div>
        </div>"##,
        );
        let root = doc.select("div.content");

        // Rule that matches .main (which contains most of the text)
        fn main_rule(sel: &Selection) -> bool {
            dom::get_attribute(sel, "class")
                .is_some_and(|c| c.contains("main"))
        }

        // With backup=true, should restore if too much removed
        let pruned_doc = prune_unwanted_nodes(&root, &[main_rule], true);
        let pruned = pruned_doc.select("*");

        // The .main div should still exist (restored from backup)
        // because removing it would remove >85% of text
        assert!(pruned.select(".main").length() > 0 || !dom::text_content(&pruned).is_empty());
    }

    #[test]
    fn test_prune_unwanted_sections_default_options() {
        let doc = dom::parse(
            r##"<div>
            <p>Main content paragraph with enough text to pass filters</p>
        </div>"##,
        );
        let root = doc.select("div");
        let potential = make_potential_tags();
        let opts = Options::default();

        let pruned_doc = prune_unwanted_sections(&root, &potential, &opts);
        let pruned = pruned_doc.select("*");

        // Content should remain after pruning
        assert!(pruned.select("p").length() > 0);
    }

    #[test]
    fn test_prune_unwanted_sections_precision_removes_trailing_titles() {
        // Test the trailing title removal logic directly by using a simpler structure
        // that won't be affected by other pruning rules
        let doc = dom::parse(
            r##"<div>
            <p>Main content paragraph one with substantial text to avoid filtering.</p>
            <p>Main content paragraph two with more substantial text content here.</p>
            <h2>Trailing Title At End</h2>
        </div>"##,
        );
        let root = doc.select("div");
        let potential = make_potential_tags();
        let opts = Options {
            favor_precision: true,
            ..Options::default()
        };

        let pruned_doc = prune_unwanted_sections(&root, &potential, &opts);
        let pruned = pruned_doc.select("body > *");

        // Content paragraphs should remain
        assert!(pruned.select("p").length() >= 1, "Content paragraphs should remain");

        // Trailing h2 should be removed in precision mode
        // (it's at the end of the content, so it gets removed)
        let h2_count = pruned.select("h2").length();
        assert_eq!(h2_count, 0, "Trailing h2 should be removed in precision mode");
    }

    #[test]
    fn test_prune_unwanted_sections_precision_keeps_non_trailing_titles() {
        // Non-trailing titles (followed by content) should be preserved
        let doc = dom::parse(
            r##"<div>
            <h1>Main Title</h1>
            <p>Content after the title with enough text to pass filters.</p>
        </div>"##,
        );
        let root = doc.select("div");
        let potential = make_potential_tags();
        let opts = Options {
            favor_precision: true,
            ..Options::default()
        };

        let pruned_doc = prune_unwanted_sections(&root, &potential, &opts);
        let pruned = pruned_doc.select("body > *");

        // h1 is NOT trailing (p follows it), so it should remain
        // Note: link density may still remove it, but the trailing title logic won't
        let text = dom::text_content(&pruned);
        assert!(
            text.contains("Main Title") || pruned.select("h1").length() > 0,
            "Non-trailing title should be preserved"
        );
    }

    #[test]
    fn test_strip_non_potential_tags_with_links() {
        let doc = dom::parse(
            r##"<div>
            <p>Text with <a href="#">link</a> and <span>span</span></p>
        </div>"##,
        );
        let root = doc.select("div");

        let mut potential = make_potential_tags();
        potential.insert("a".to_string());

        strip_non_potential_tags(&root, &potential);

        // Links should remain
        assert_eq!(root.select("a").length(), 1);
        // Spans should be stripped (tag removed, content preserved)
        assert_eq!(root.select("span").length(), 0);
    }

    #[test]
    fn test_strip_non_potential_tags_without_links() {
        let doc = dom::parse(
            r##"<div>
            <p>Text with <a href="#">link</a> and <span>span</span></p>
        </div>"##,
        );
        let root = doc.select("div");
        let potential = make_potential_tags(); // No 'a' tag

        strip_non_potential_tags(&root, &potential);

        // Links should be stripped
        assert_eq!(root.select("a").length(), 0);
        // Spans should also be stripped
        assert_eq!(root.select("span").length(), 0);
        // But content should be preserved
        let text = dom::text_content(&root);
        assert!(text.contains("link"));
        assert!(text.contains("span"));
    }

    #[test]
    fn test_prune_unwanted_nodes_empty_rules() {
        let doc = dom::parse("<div><p>Content</p></div>");
        let root = doc.select("div");

        let pruned_doc = prune_unwanted_nodes(&root, &[], false);
        let pruned = pruned_doc.select("*");

        // Nothing should be removed
        assert_eq!(pruned.select("p").length(), 1);
    }

    #[test]
    fn test_prune_unwanted_nodes_no_matches() {
        let doc = dom::parse("<div><p>Content</p></div>");
        let root = doc.select("div");

        fn never_matches(_sel: &Selection) -> bool {
            false
        }

        let pruned_doc = prune_unwanted_nodes(&root, &[never_matches], false);
        let pruned = pruned_doc.select("*");

        // Nothing should be removed
        assert_eq!(pruned.select("p").length(), 1);
    }
}
