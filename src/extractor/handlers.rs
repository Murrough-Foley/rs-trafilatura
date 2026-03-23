//! Simple element handlers for titles, formatting, images, and code blocks.
//!
//! This module ports the simpler element handlers from go-trafilatura's main-extractor.go.

use dom_query::{Selection, Document};
use crate::dom;
use crate::etree;
use crate::html_processing::{handle_text_node, is_share_button_text, process_node, text_chars_test};
use crate::Options;
use super::state::ExtractionState;
use super::tags::{
    is_xml_cell_tag, is_xml_graphic_tag, is_xml_head_tag, is_xml_hi_tag,
    is_xml_item_tag, is_xml_list_tag, is_xml_quote_tag,
    XML_ITEM_TAGS, XML_QUOTE_TAGS,
};

/// Check if element contains text content.
///
/// Go equivalent: `isTextElement(element)` (line 119-122)
#[must_use]
pub fn is_text_element(element: &Selection) -> bool {
    if element.is_empty() {
        return false;
    }
    let text = etree::iter_text(element, "");
    text_chars_test(&text)
}

/// Create a new sub-element if processed element is not None.
///
/// Go equivalent: `defineNewElement(processedElement, originalElement)` (lines 124-131)
pub fn define_new_element(processed: Option<&Selection>, original: &Selection) {
    if let Some(processed_el) = processed {
        let tag_name = dom::tag_name(processed_el).unwrap_or_else(|| "span".to_string());
        let child = etree::sub_element(original, &tag_name);
        etree::set_text(&child, &etree::text(processed_el));
        etree::set_tail(&child, &etree::tail(processed_el));
    }
}

/// Add a sub-element to an existing child element.
///
/// Go equivalent: `addSubElement(newChildElement, subElement, processedSubChild)` (lines 91-98)
#[must_use]
pub fn add_sub_element<'a>(
    new_child: &Selection<'a>,
    sub_element: &Selection,
    processed_sub_child: &Selection,
) -> Selection<'a> {
    let tag_name = dom::tag_name(processed_sub_child).unwrap_or_else(|| "span".to_string());
    let sub_child = etree::sub_element(new_child, &tag_name);
    etree::set_text(&sub_child, &etree::text(processed_sub_child));
    etree::set_tail(&sub_child, &etree::tail(processed_sub_child));

    // Copy attributes from sub_element to sub_child
    let attrs = dom::get_all_attributes(sub_element);
    for (key, value) in attrs {
        dom::set_attribute(&sub_child, &key, &value);
    }

    sub_child
}

/// Process head elements (titles: h1-h6, summary).
///
/// In original trafilatura, summary is treated as heading. However, in XML,
/// <h1> to <h6> is treated simply as <head>, which means heading level is
/// not important in XML. Since we work mainly in HTML, we can't simply
/// change the summary into heading because heading level is important here.
/// So, here we just mark the summary as bold to show that it's an important text.
///
/// Returns a Document containing the processed title element.
///
/// Note: Cache parameter omitted from Go equivalent - deduplication is handled at higher level.
///
/// Go equivalent: `handleTitles(element, cache, opts)` (lines 15-58)
#[must_use]
pub fn handle_titles(
    element: &Selection,
    state: &mut ExtractionState,
    opts: &Options,
) -> Option<Document> {
    // Convert summary to bold
    let tag_name = dom::tag_name(element).unwrap_or_default();

    if tag_name == "summary" {
        dom::rename(element, "b");
    }

    let children = dom::children(element);
    let title = if children.is_empty() {
        // No children - process as simple node
        if process_node(element, None, opts) {
            let doc = dom::clone_element(element, true);
            Some(doc)
        } else {
            None
        }
    } else {
        // Has children - clone and process each child
        let title_doc = dom::clone_element(element, false);
        let title_el = title_doc.select("*");

        for child_node in dom::child_nodes(element).nodes() {
            let child = Selection::from(*child_node);
            let cloned_doc = dom::clone_element(&child, true);
            let cloned_child = cloned_doc.select("body > *");

            // Process the child
            let _processed = handle_text_node(&cloned_child, None, false, false, opts);

            // Always append the child regardless of processing result
            dom::append_child(&title_el, &cloned_child);

            // Mark original child as done
            if let Some(node) = child.nodes().first() {
                state.mark_done(node.id);
            }
        }

        Some(title_doc)
    };

    // Verify title has text content and is not boilerplate
    if let Some(ref t) = title {
        let sel = t.select("*");
        let all_text = etree::iter_text(&sel, " ");

        // Check for boilerplate patterns in the title
        // This filters out headings like "Subscribe to Newsletter", "Comments", etc.
        for line in all_text.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && is_share_button_text(trimmed) {
                return None;
            }
        }

        if is_text_element(&sel) {
            return title;
        }
    }

    None
}

/// Process formatting elements (b, i, em, etc.) found outside of paragraphs.
///
/// Returns a Document containing the processed formatting element (possibly wrapped in <p>).
///
/// Note: Cache parameter omitted from Go equivalent - deduplication is handled at higher level.
///
/// Go equivalent: `handleFormatting(element, cache, opts)` (lines 60-89)
#[must_use]
pub fn handle_formatting(
    element: &Selection,
    opts: &Options,
) -> Option<Document> {
    let formatting = if process_node(element, None, opts) {
        Some(dom::clone_element(element, true))
    } else {
        None
    };

    let children = dom::children(element);
    if children.is_empty() && formatting.is_none() {
        return None;
    }

    // Repair orphan elements - wrap in <p> if parent is not suitable
    // Go fallback: if parent is nil, try PrevSibling (lines 69-72)
    let parent = dom::parent(element);
    let effective_parent = if parent.is_empty() {
        dom::prev_sibling(element)
    } else {
        parent
    };
    let parent_tag = if effective_parent.is_empty() {
        String::new()
    } else {
        dom::tag_name(&effective_parent).unwrap_or_default()
    };

    let needs_wrapping = effective_parent.is_empty()
        || (!is_xml_cell_tag(&parent_tag)
            && !is_xml_head_tag(&parent_tag)
            && !is_xml_hi_tag(&parent_tag)
            && !is_xml_item_tag(&parent_tag)
            && !is_xml_quote_tag(&parent_tag)
            && parent_tag != "p");

    if needs_wrapping {
        if let Some(fmt) = formatting {
            // Create a <p> element and append formatting to it
            let doc = Document::from("<p></p>");
            let p = doc.select("p");
            let fmt_sel = fmt.select("*");
            etree::append(&p, &fmt_sel);
            return Some(doc);
        }
    }

    formatting
}

/// Check if element is a code block according to common structural markers.
///
/// Go equivalent: `isCodeBlockElement(element)` (lines 197-217)
#[must_use]
pub fn is_code_block_element(element: &Selection) -> bool {
    // Check for lang attribute (Pip)
    if dom::get_attribute(element, "lang").is_some() {
        return true;
    }

    // Check if element is <code>
    if dom::tag_name(element).as_deref() == Some("code") {
        return true;
    }

    // Check parent for highlight class (GitHub)
    let parent = dom::parent(element);
    if !parent.is_empty() {
        let parent_class = dom::get_attribute(&parent, "class").unwrap_or_default();
        if parent_class.contains("highlight") {
            return true;
        }
    }

    // Check for single <code> child (Highlight.js)
    let code_children = element.select("code");
    let all_children = dom::children(element);
    if code_children.length() > 0 && all_children.length() == 1 {
        return true;
    }

    false
}

/// Turn element into a properly tagged code block.
///
/// Returns a Document containing the processed code element.
///
/// Go equivalent: `handleCodeBlocks(element)` (lines 219-232)
#[must_use]
pub fn handle_code_blocks(element: &Selection, state: &mut ExtractionState) -> Document {
    let processed_doc = dom::clone_element(element, true);
    let processed = processed_doc.select("*");

    // Mark all children in original as done
    for child_node in etree::iter(element, &[]).nodes() {
        state.mark_done(child_node.id);
    }

    // Rename to code and clear attributes
    dom::rename(&processed, "code");
    for child_node in etree::iter(&processed, &[]).nodes() {
        let child = Selection::from(*child_node);
        dom::clear_all_attributes(&child);
    }

    processed_doc
}

/// Handle diverse or unknown elements in the scope of relevant tags.
///
/// Returns a Document containing the processed element.
///
/// Note: Cache parameter omitted from Go equivalent - deduplication is handled at higher level.
///
/// Go equivalent: `handleOtherElements(element, potentialTags, cache, opts)` (lines 256-287)
#[must_use]
pub fn handle_other_elements(
    element: &Selection,
    state: &mut ExtractionState,
    opts: &Options,
) -> Option<Document> {
    let tag_name = dom::tag_name(element).unwrap_or_default();

    // Handle W3Schools Code
    if tag_name == "div" {
        let class = dom::get_attribute(element, "class").unwrap_or_default();
        if class.contains("w3-code") {
            // Mark all children in original as done (matches Go's handleCodeBlocks behavior)
            for child_node in etree::iter(element, &[]).nodes() {
                state.mark_done(child_node.id);
            }
            // Return code block
            let processed_doc = dom::clone_element(element, true);
            let processed = processed_doc.select("*");
            dom::rename(&processed, "code");
            return Some(processed_doc);
        }
    }

    // Delete non-potential element
    if !state.is_potential_tag(&tag_name) {
        return None;
    }

    // Handle div or details
    if (tag_name == "div" || tag_name == "details")
        && handle_text_node(element, None, false, true, opts)
    {
        let text = etree::text(element);
        if text_chars_test(&text) {
            let processed_doc = dom::clone_element(element, true);
            let processed = processed_doc.select("*");
            dom::clear_all_attributes(&processed);

            if tag_name == "div" {
                dom::rename(&processed, "p");
            }

            return Some(processed_doc);
        }
    }

    None
}

/// Process image element and their relevant attributes.
///
/// Returns a Document containing the processed img element.
///
/// Go equivalent: `handleImage(element)` (lines 481-529)
#[must_use]
pub fn handle_image(element: &Selection) -> Option<Document> {
    if element.is_empty() {
        return None;
    }

    let tag_name = dom::tag_name(element).unwrap_or_else(|| "img".to_string());
    // Create the processed element
    let doc = Document::from(format!("<{tag_name}></{tag_name}>"));
    let processed = doc.select(&tag_name);

    // Handle image source
    let src = dom::get_attribute(element, "src");
    let data_src = dom::get_attribute(element, "data-src");

    if let Some(ds) = &data_src {
        if is_image_file(ds) {
            dom::set_attribute(&processed, "src", ds);
        }
    }

    if dom::get_attribute(&processed, "src").is_none() {
        if let Some(s) = &src {
            if is_image_file(s) {
                dom::set_attribute(&processed, "src", s);
            }
        }
    }

    // If still no src, try data-src* attributes
    if dom::get_attribute(&processed, "src").is_none() {
        let attrs = dom::get_all_attributes(element);
        for (key, value) in attrs {
            if key.starts_with("data-src") && is_image_file(&value) {
                dom::set_attribute(&processed, "src", &value);
                break;
            }
        }
    }

    // Handle alt and title
    if let Some(alt) = dom::get_attribute(element, "alt") {
        if !alt.is_empty() {
            dom::set_attribute(&processed, "alt", &alt);
        }
    }

    if let Some(title) = dom::get_attribute(element, "title") {
        if !title.is_empty() {
            dom::set_attribute(&processed, "title", &title);
        }
    }

    // Verify we have src attribute
    let final_src = dom::get_attribute(&processed, "src")?;
    if final_src.is_empty() {
        return None;
    }

    // Post-process URL: convert protocol-relative to http
    if final_src.starts_with("//") {
        let url = format!("http:{final_src}");
        dom::set_attribute(&processed, "src", &url);
    }

    Some(doc)
}

/// Check if a source string points to an image file.
fn is_image_file(src: &str) -> bool {
    if src.is_empty() {
        return false;
    }
    // Remove query string
    let path = src.split('?').next().unwrap_or(src);
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
    matches!(
        ext.as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "bmp" | "ico" | "tiff" | "tif" | "avif"
    )
}

// === Complex Element Handlers ===

/// Iterate through element children and rewire descendants.
///
/// This function processes nested elements within a child element and adds them to the new child.
///
/// Note: Cache parameter omitted from Go equivalent - deduplication is handled at higher level
/// in the extraction pipeline. Rust's borrow rules make passing mutable cache through loops
/// impractical without interior mutability.
///
/// Go equivalent: `processNestedElement(child, newChildElement, cache, opts)` (lines 100-117)
pub fn process_nested_element(
    child: &Selection,
    new_child_element: &Selection,
    state: &mut ExtractionState,
    opts: &Options,
) {
    // Set text from child
    etree::set_text(new_child_element, &etree::text(child));

    // Process descendants
    for sub_node in etree::iter_descendants(child, &[]).nodes() {
        let sub_element = Selection::from(*sub_node);
        let tag_name = dom::tag_name(&sub_element).unwrap_or_default();

        if is_xml_list_tag(&tag_name) {
            // Handle nested lists
            if let Some(processed) = handle_lists(&sub_element, state, opts) {
                // Append the list to new_child_element
                let list_sel = processed.select("*");
                etree::append(new_child_element, &list_sel);
            }
        } else {
            // Handle other nested elements
            if handle_text_node(&sub_element, None, false, false, opts) {
                let _ = add_sub_element(new_child_element, &sub_element, &sub_element);
            }
        }

        // Mark as done
        state.mark_done(sub_node.id);
    }
}

/// Process list elements (ul, ol, dl) including their descendants.
///
/// Returns a Document containing the processed list element.
///
/// Note: Cache parameter omitted from Go equivalent - deduplication is handled at higher level.
///
/// Go equivalent: `handleLists(element, cache, opts)` (lines 133-195)
#[must_use]
pub fn handle_lists(
    element: &Selection,
    state: &mut ExtractionState,
    opts: &Options,
) -> Option<Document> {
    let tag_name = dom::tag_name(element).unwrap_or_else(|| "ul".to_string());
    let processed_doc = etree::element(&tag_name);
    let processed_element = processed_doc.select(&tag_name);

    // Handle text directly in list element
    let text = etree::text(element).trim().to_string();
    if !text.is_empty() {
        let li = etree::sub_element(&processed_element, "li");
        etree::set_text(&li, &text);
    }

    // Process list item descendants (dd, dt, li)
    for child_node in etree::iter_descendants(element, &XML_ITEM_TAGS).nodes() {
        let child = Selection::from(*child_node);
        let child_tag = dom::tag_name(&child).unwrap_or_else(|| "li".to_string());

        let new_child = etree::sub_element(&processed_element, &child_tag);

        let children = dom::children(&child);
        if children.is_empty() {
            // Childless list item - process directly
            if process_node(&child, None, opts) {
                let mut new_text = etree::text(&child);
                let tail = etree::tail(&child).trim().to_string();
                if !tail.is_empty() {
                    new_text = format!("{} {}", new_text.trim(), tail);
                }
                etree::set_text(&new_child, &new_text);
            }
        } else {
            // Has children - process nested elements
            process_nested_element(&child, &new_child, state, opts);

            // Handle tail
            let tail = etree::tail(&child);
            if !tail.trim().is_empty() {
                let new_child_children = dom::children(&new_child);
                if !new_child_children.is_empty() {
                    // Get last child and append tail
                    let last_child_node = new_child_children.nodes().last();
                    if let Some(last_node) = last_child_node {
                        let last_sel = Selection::from(*last_node);
                        let existing_tail = etree::tail(&last_sel);
                        if existing_tail.trim().is_empty() {
                            etree::set_tail(&last_sel, &tail);
                        } else {
                            let new_tail = format!("{existing_tail} {tail}");
                            etree::set_tail(&last_sel, &new_tail);
                        }
                    }
                }
            }
        }

        // Mark as done
        state.mark_done(child_node.id);
    }

    // Mark element as done
    if let Some(node) = element.nodes().first() {
        state.mark_done(node.id);
    }

    // Return if has text content
    if is_text_element(&processed_element) {
        Some(processed_doc)
    } else {
        None
    }
}

/// Process quote elements (blockquote, pre, q).
///
/// Returns a Document containing the processed quote element.
///
/// Note: Cache parameter omitted from Go equivalent - deduplication is handled at higher level.
///
/// Go equivalent: `handleQuotes(element, cache, opts)` (lines 234-254)
#[must_use]
pub fn handle_quotes(
    element: &Selection,
    state: &mut ExtractionState,
    opts: &Options,
) -> Option<Document> {
    // Handle code block first
    if is_code_block_element(element) {
        return Some(handle_code_blocks(element, state));
    }

    let tag_name = dom::tag_name(element).unwrap_or_else(|| "blockquote".to_string());
    let processed_doc = etree::element(&tag_name);
    let processed_element = processed_doc.select(&tag_name);

    // Process element itself (Go's Iter includes self)
    if process_node(element, None, opts) {
        // Set text from original element
        etree::set_text(&processed_element, &etree::text(element));
        etree::set_tail(&processed_element, &etree::tail(element));
    }

    // Process child elements
    for child_node in etree::iter(element, &[]).nodes() {
        let child = Selection::from(*child_node);

        if process_node(&child, None, opts) {
            define_new_element(Some(&child), &processed_element);
        }

        state.mark_done(child_node.id);
    }

    // Return if has text content
    if is_text_element(&processed_element) {
        // Strip nested quote tags
        etree::strip_tags(&processed_element, &XML_QUOTE_TAGS);
        Some(processed_doc)
    } else {
        None
    }
}

/// Process paragraph (p) elements along with their children.
///
/// Returns a Document containing the processed paragraph element.
///
/// Note: Cache parameter omitted from Go equivalent - deduplication is handled at higher level.
///
/// Go equivalent: `handleParagraphs(element, potentialTags, cache, opts)` (lines 289-395)
#[must_use]
pub fn handle_paragraphs(
    element: &Selection,
    state: &mut ExtractionState,
    opts: &Options,
) -> Option<Document> {
    // Clear attributes
    dom::clear_all_attributes(element);

    // Handle paragraph without children
    let children = dom::children(element);
    if children.is_empty() {
        return if process_node(element, None, opts) {
            Some(dom::clone_element(element, true))
        } else {
            None
        };
    }

    // Handle paragraph with children
    let mut unwanted_children = Vec::new();

    for child_node in dom::get_elements_by_tag_name(element, "*").nodes() {
        let child = Selection::from(*child_node);
        let child_tag = dom::tag_name(&child).unwrap_or_default();

        // Check if child is potential element
        if !state.is_potential_tag(&child_tag) && !state.is_done(child_node.id) {
            unwanted_children.push(*child_node);
            continue;
        }

        // Process child
        if !handle_text_node(&child, None, false, true, opts) {
            state.mark_done(child_node.id);
            continue;
        }

        // Handle specific tag types
        match child_tag.as_str() {
            "p" => {
                // Nested <p> - strip but keep content
                let child_text = etree::text(&child);
                let parent_sel = dom::parent(&child);
                let parent_text = if parent_sel.is_empty() {
                    String::new()
                } else {
                    etree::text(&parent_sel)
                };

                if !parent_text.is_empty() && !child_text.is_empty() {
                    etree::set_text(&child, &format!(" {child_text}"));
                }
                etree::strip(&child);
            }
            "a" => {
                // Links - preserve href and target only
                let href = dom::get_attribute(&child, "href")
                    .map(|h| h.trim().to_string())
                    .unwrap_or_default();
                let target = dom::get_attribute(&child, "target")
                    .map(|t| t.trim().to_string())
                    .unwrap_or_default();

                dom::clear_all_attributes(&child);

                if !href.is_empty() {
                    dom::set_attribute(&child, "href", &href);
                }
                if !target.is_empty() {
                    dom::set_attribute(&child, "target", &target);
                }
            }
            _ if is_xml_graphic_tag(&child_tag) => {
                // Image - handle separately
                if let Some(img) = handle_image(&child) {
                    dom::replace_element(&child, &img);
                }
            }
            _ => {}
        }

        state.mark_done(child_node.id);
    }

    // Remove unwanted children (reverse order)
    for node in unwanted_children.into_iter().rev() {
        etree::remove(&Selection::from(node), false);
    }

    // Remove empty elements (reverse order)
    let all_children = dom::get_elements_by_tag_name(element, "*").nodes().to_vec();
    for node in all_children.into_iter().rev() {
        let child = Selection::from(node);
        let is_empty = !text_chars_test(&etree::text(&child));
        let is_void = dom::is_void_element(&child);
        if is_empty && !is_void {
            etree::strip(&child);
        }
    }

    // Clean trailing line breaks
    let line_breaks = element.select("br, hr").nodes().to_vec();
    for node in line_breaks.into_iter().rev() {
        let br = Selection::from(node);
        let has_next_sibling = dom::next_element_sibling(&br).is_some();
        let tail = etree::tail(&br);
        if !has_next_sibling || tail.is_empty() {
            etree::remove(&br, false);
        }
    }

    // Clone element to return
    let processed = dom::clone_element(element, true);
    let processed_sel = processed.select("*");
    etree::set_tail(&processed_sel, &etree::tail(element));

    // Check if has content
    let text = etree::text(&processed_sel);
    let children = dom::children(&processed_sel);
    if !children.is_empty() || !text.is_empty() {
        Some(processed)
    } else {
        None
    }
}

/// Process single table element.
///
/// Returns a Document containing the processed table element.
///
/// Note: Due to HTML5 parsing limitations in `dom_query`, we cannot dynamically
/// append tr/td elements to tables. Instead, we build the table HTML as a string
/// and parse it all at once.
///
/// For cells with children, this processes each child according to its type:
/// - Cell/hi tags: processed via `handle_text_node`
/// - List tags (in `favor_recall` mode): processed via `handle_lists`
/// - Other elements: processed via text extraction
///
/// Cache parameter omitted from Go equivalent - deduplication is handled at higher level.
///
/// Go equivalent: `handleTable(tableElement, potentialTags, cache, opts)` (lines 397-479)
#[must_use]
pub fn handle_table(
    table_element: &Selection,
    state: &mut ExtractionState,
    opts: &Options,
) -> Option<Document> {
    // Build table HTML as string (required due to HTML5 parsing limitations)
    let mut rows: Vec<String> = Vec::new();
    let mut current_row_cells: Vec<String> = Vec::new();

    // Strip structural elements from input
    etree::strip_tags(table_element, &["thead", "tbody", "tfoot"]);

    // Explore sub-elements
    for sub_node in etree::iter_descendants(table_element, &[]).nodes() {
        let sub_element = Selection::from(*sub_node);
        let sub_tag = dom::tag_name(&sub_element).unwrap_or_default();

        match sub_tag.as_str() {
            "tr" => {
                // Save current row if it has cells, start new row
                if !current_row_cells.is_empty() {
                    rows.push(format!("<tr>{}</tr>", current_row_cells.join("")));
                    current_row_cells.clear();
                }
            }
            "td" | "th" => {
                let children = dom::children(&sub_element);
                if children.is_empty() {
                    // Childless cell - process directly
                    if process_node(&sub_element, None, opts) {
                        let text = html_escape(&etree::text(&sub_element));
                        let tail = html_escape(&etree::tail(&sub_element));
                        current_row_cells.push(format!("<{sub_tag}>{text}</{sub_tag}>{tail}"));
                    }
                } else {
                    // Cell with children - process each child
                    let cell_html = process_table_cell(&sub_element, state, opts);
                    if !cell_html.trim().is_empty() {
                        current_row_cells.push(format!("<{sub_tag}>{cell_html}</{sub_tag}>"));
                    }
                    state.mark_done(sub_node.id);

                    // Mark all descendants as done
                    for child_node in etree::iter_descendants(&sub_element, &[]).nodes() {
                        state.mark_done(child_node.id);
                    }
                }
            }
            "table" => {
                // Nested table - stop processing
                break;
            }
            _ => {}
        }

        state.mark_done(sub_node.id);
    }

    // Don't forget the last row
    if !current_row_cells.is_empty() {
        rows.push(format!("<tr>{}</tr>", current_row_cells.join("")));
    }

    // Return table if has content
    if rows.is_empty() {
        None
    } else {
        let table_html = format!("<table><tbody>{}</tbody></table>", rows.join(""));
        Some(dom::parse(&table_html))
    }
}

/// Process table cell children and return HTML content.
///
/// Processes children according to their type:
/// - Cell/hi tags: via `handle_text_node`
/// - List tags (in `favor_recall` mode): via `handle_lists`
/// - Other elements: text extraction
fn process_table_cell(cell: &Selection, state: &mut ExtractionState, opts: &Options) -> String {
    let mut content_parts: Vec<String> = Vec::new();

    // Start with cell's direct text
    let cell_text = etree::text(cell);
    if !cell_text.trim().is_empty() {
        content_parts.push(html_escape(&cell_text));
    }

    // Process each child element
    for child_node in etree::iter_descendants(cell, &[]).nodes() {
        let child = Selection::from(*child_node);
        let child_tag = dom::tag_name(&child).unwrap_or_default();

        // Check tag type and process accordingly
        if is_xml_cell_tag(&child_tag) || is_xml_hi_tag(&child_tag) {
            // Cell or formatting tag - use handle_text_node
            if handle_text_node(&child, None, true, false, opts) {
                let text = etree::text(&child);
                let tail = etree::tail(&child);
                if !text.trim().is_empty() {
                    // Wrap in appropriate tag for formatting
                    if is_xml_hi_tag(&child_tag) {
                        content_parts.push(format!(
                            "<{tag}>{text}</{tag}>",
                            tag = child_tag,
                            text = html_escape(&text)
                        ));
                    } else {
                        content_parts.push(html_escape(&text));
                    }
                }
                if !tail.trim().is_empty() {
                    content_parts.push(html_escape(&tail));
                }
            }
        } else if is_xml_list_tag(&child_tag) && opts.favor_recall {
            // List tag in recall mode - process via handle_lists
            if let Some(list_doc) = handle_lists(&child, state, opts) {
                let list_html = dom::inner_html(&list_doc.select("body")).to_string();
                if !list_html.trim().is_empty() {
                    content_parts.push(list_html);
                }
            }
        } else {
            // Other elements - extract text content
            let text = etree::text(&child);
            let tail = etree::tail(&child);
            if !text.trim().is_empty() {
                content_parts.push(html_escape(&text));
            }
            if !tail.trim().is_empty() {
                content_parts.push(html_escape(&tail));
            }
        }

        state.mark_done(child_node.id);
    }

    content_parts.join(" ")
}

/// Escape HTML special characters
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_text_element_with_text() {
        let doc = dom::parse("<p>Hello world</p>");
        let p = doc.select("p");
        assert!(is_text_element(&p));
    }

    #[test]
    fn test_is_text_element_empty() {
        let doc = dom::parse("<p>   </p>");
        let p = doc.select("p");
        assert!(!is_text_element(&p));
    }

    #[test]
    fn test_is_text_element_no_alphanumeric() {
        let doc = dom::parse("<p>!!!</p>");
        let p = doc.select("p");
        assert!(!is_text_element(&p));
    }

    #[test]
    fn test_is_code_block_with_lang() {
        let doc = dom::parse(r#"<pre lang="python">code</pre>"#);
        let pre = doc.select("pre");
        assert!(is_code_block_element(&pre));
    }

    #[test]
    fn test_is_code_block_with_code_tag() {
        let doc = dom::parse("<code>code</code>");
        let code = doc.select("code");
        assert!(is_code_block_element(&code));
    }

    #[test]
    fn test_is_code_block_github_highlight() {
        let doc = dom::parse(r#"<div class="highlight"><pre>code</pre></div>"#);
        let pre = doc.select("pre");
        assert!(is_code_block_element(&pre));
    }

    #[test]
    fn test_is_code_block_single_code_child() {
        let doc = dom::parse("<pre><code>code</code></pre>");
        let pre = doc.select("pre");
        assert!(is_code_block_element(&pre));
    }

    #[test]
    fn test_is_code_block_not_code() {
        let doc = dom::parse("<pre>text</pre>");
        let pre = doc.select("pre");
        assert!(!is_code_block_element(&pre));
    }

    #[test]
    fn test_handle_image_basic() {
        let doc = dom::parse(r#"<img src="test.jpg" alt="Test">"#);
        let img = doc.select("img");

        let processed = handle_image(&img);
        assert!(processed.is_some());

        let p_doc = processed.unwrap();
        let p = p_doc.select("img");
        assert_eq!(dom::get_attribute(&p, "src"), Some("test.jpg".to_string()));
        assert_eq!(dom::get_attribute(&p, "alt"), Some("Test".to_string()));
    }

    #[test]
    fn test_handle_image_data_src() {
        let doc = dom::parse(r#"<img data-src="lazy.png">"#);
        let img = doc.select("img");

        let processed = handle_image(&img);
        assert!(processed.is_some());
        let p_doc = processed.unwrap();
        let p = p_doc.select("img");
        assert_eq!(
            dom::get_attribute(&p, "src"),
            Some("lazy.png".to_string())
        );
    }

    #[test]
    fn test_handle_image_data_src_lazy() {
        let doc = dom::parse(r#"<img data-src-lazy="lazy2.jpg">"#);
        let img = doc.select("img");

        let processed = handle_image(&img);
        assert!(processed.is_some());
        let p_doc = processed.unwrap();
        let p = p_doc.select("img");
        assert_eq!(
            dom::get_attribute(&p, "src"),
            Some("lazy2.jpg".to_string())
        );
    }

    #[test]
    fn test_handle_image_protocol_relative() {
        let doc = dom::parse(r#"<img src="//example.com/img.jpg">"#);
        let img = doc.select("img");

        let processed = handle_image(&img);
        assert!(processed.is_some());
        let p_doc = processed.unwrap();
        let p = p_doc.select("img");
        assert_eq!(
            dom::get_attribute(&p, "src"),
            Some("http://example.com/img.jpg".to_string())
        );
    }

    #[test]
    fn test_handle_image_no_src() {
        let doc = dom::parse(r#"<img alt="No source">"#);
        let img = doc.select("img");

        let processed = handle_image(&img);
        assert!(processed.is_none());
    }

    #[test]
    fn test_handle_image_non_image_src() {
        let doc = dom::parse(r#"<img src="data.json">"#);
        let img = doc.select("img");

        let processed = handle_image(&img);
        assert!(processed.is_none());
    }

    #[test]
    fn test_handle_image_with_title() {
        let doc = dom::parse(r#"<img src="test.png" title="Image Title">"#);
        let img = doc.select("img");

        let processed = handle_image(&img);
        assert!(processed.is_some());
        let p_doc = processed.unwrap();
        let p = p_doc.select("img");
        assert_eq!(dom::get_attribute(&p, "title"), Some("Image Title".to_string()));
    }

    #[test]
    fn test_handle_titles_simple() {
        let doc = dom::parse("<h1>Title Text</h1>");
        let h1 = doc.select("h1");
        let mut state = ExtractionState::new();
        let opts = Options::default();

        let processed = handle_titles(&h1, &mut state, &opts);
        assert!(processed.is_some());
    }

    #[test]
    fn test_handle_titles_summary_to_bold() {
        let doc = dom::parse("<summary>Summary Text</summary>");
        let summary = doc.select("summary");
        let mut state = ExtractionState::new();
        let opts = Options::default();

        let _ = handle_titles(&summary, &mut state, &opts);
        // Summary should be renamed to b
        assert_eq!(dom::tag_name(&summary), Some("b".to_string()));
    }

    #[test]
    fn test_handle_formatting_orphan() {
        let doc = dom::parse("<div><b>Bold text</b></div>");
        let b = doc.select("b");
        let opts = Options::default();

        let processed = handle_formatting(&b, &opts);
        // Should be wrapped in <p> since parent is div (not suitable)
        assert!(processed.is_some());
        let p_doc = processed.unwrap();
        let p = p_doc.select("p");
        assert_eq!(dom::tag_name(&p), Some("p".to_string()));
    }

    #[test]
    fn test_is_image_file_jpg() {
        assert!(is_image_file("image.jpg"));
        assert!(is_image_file("image.jpeg"));
    }

    #[test]
    fn test_is_image_file_png() {
        assert!(is_image_file("image.png"));
    }

    #[test]
    fn test_is_image_file_with_query() {
        assert!(is_image_file("image.jpg?size=large"));
    }

    #[test]
    fn test_is_image_file_not_image() {
        assert!(!is_image_file("document.pdf"));
        assert!(!is_image_file("data.json"));
    }

    #[test]
    fn test_handle_code_blocks() {
        let doc = dom::parse(r#"<pre class="language-rust">fn main() {}</pre>"#);
        let pre = doc.select("pre");
        let mut state = ExtractionState::new();

        let processed_doc = handle_code_blocks(&pre, &mut state);
        let processed = processed_doc.select("code");
        assert_eq!(dom::tag_name(&processed), Some("code".to_string()));
        // Attributes should be cleared
        assert!(dom::get_attribute(&processed, "class").is_none());
    }

    #[test]
    fn test_handle_other_elements_w3_code() {
        let doc = dom::parse(r#"<div class="w3-code">code here</div>"#);
        let div = doc.select("div");
        let mut state = ExtractionState::new();
        let opts = Options::default();

        let processed = handle_other_elements(&div, &mut state, &opts);
        assert!(processed.is_some());
        let p_doc = processed.unwrap();
        let p = p_doc.select("code");
        assert_eq!(dom::tag_name(&p), Some("code".to_string()));
    }

    // === Complex Handler Tests ===

    #[test]
    fn test_handle_lists_simple() {
        let doc = dom::parse("<ul><li>Item 1</li><li>Item 2</li></ul>");
        let ul = doc.select("ul");
        let mut state = ExtractionState::new();
        let opts = Options::default();

        let processed = handle_lists(&ul, &mut state, &opts);
        assert!(processed.is_some());

        let list_doc = processed.unwrap();
        let list = list_doc.select("ul");
        assert_eq!(dom::tag_name(&list), Some("ul".to_string()));
        assert_eq!(list.select("li").length(), 2);
    }

    #[test]
    fn test_handle_lists_nested() {
        let doc = dom::parse("<ul><li>Item 1<ul><li>Nested</li></ul></li></ul>");
        let ul = doc.select("ul").first();
        let mut state = ExtractionState::new();
        let opts = Options::default();

        let processed = handle_lists(&ul, &mut state, &opts);
        assert!(processed.is_some());
    }

    #[test]
    fn test_handle_lists_with_text() {
        let doc = dom::parse("<ul>Direct text<li>Item</li></ul>");
        let ul = doc.select("ul");
        let mut state = ExtractionState::new();
        let opts = Options::default();

        let processed = handle_lists(&ul, &mut state, &opts);
        assert!(processed.is_some());
    }

    #[test]
    fn test_handle_lists_empty() {
        let doc = dom::parse("<ul></ul>");
        let ul = doc.select("ul");
        let mut state = ExtractionState::new();
        let opts = Options::default();

        let processed = handle_lists(&ul, &mut state, &opts);
        assert!(processed.is_none());
    }

    #[test]
    fn test_handle_quotes_simple() {
        let doc = dom::parse("<blockquote>Quote text</blockquote>");
        let bq = doc.select("blockquote");
        let mut state = ExtractionState::new();
        let opts = Options::default();

        let processed = handle_quotes(&bq, &mut state, &opts);
        assert!(processed.is_some());
    }

    #[test]
    fn test_handle_quotes_code_block() {
        let doc = dom::parse(r#"<pre lang="rust">fn main() {}</pre>"#);
        let pre = doc.select("pre");
        let mut state = ExtractionState::new();

        let processed = handle_quotes(&pre, &mut state, &Options::default());
        assert!(processed.is_some());
        let code_doc = processed.unwrap();
        let code = code_doc.select("code");
        assert_eq!(dom::tag_name(&code), Some("code".to_string()));
    }

    #[test]
    fn test_handle_quotes_empty() {
        let doc = dom::parse("<blockquote></blockquote>");
        let bq = doc.select("blockquote");
        let mut state = ExtractionState::new();
        let opts = Options::default();

        let processed = handle_quotes(&bq, &mut state, &opts);
        assert!(processed.is_none());
    }

    #[test]
    fn test_handle_paragraphs_simple() {
        let doc = dom::parse("<p>Simple paragraph</p>");
        let p = doc.select("p");
        let mut state = ExtractionState::new();
        let opts = Options::default();

        let processed = handle_paragraphs(&p, &mut state, &opts);
        assert!(processed.is_some());
    }

    #[test]
    fn test_handle_paragraphs_with_link() {
        let doc = dom::parse(r#"<p>Text with <a href="http://example.com">link</a></p>"#);
        let p = doc.select("p");
        let mut state = ExtractionState::new();
        state.add_potential_tag("a");
        let opts = Options { include_links: true, ..Options::default() };

        let processed = handle_paragraphs(&p, &mut state, &opts);
        assert!(processed.is_some());

        // Check link preserved
        let p_doc = processed.unwrap();
        let links = p_doc.select("a");
        assert_eq!(links.length(), 1);
    }

    #[test]
    fn test_handle_paragraphs_nested_p() {
        let doc = dom::parse("<p>Outer <p>Inner</p> text</p>");
        let p = doc.select("p").first();
        let mut state = ExtractionState::new();
        let opts = Options::default();

        let processed = handle_paragraphs(&p, &mut state, &opts);
        // Nested p should be stripped
        assert!(processed.is_some());
    }

    #[test]
    fn test_handle_paragraphs_empty() {
        let doc = dom::parse("<p></p>");
        let p = doc.select("p");
        let mut state = ExtractionState::new();
        let opts = Options::default();

        let processed = handle_paragraphs(&p, &mut state, &opts);
        assert!(processed.is_none());
    }

    #[test]
    fn test_handle_table_simple() {
        let doc = dom::parse("<table><tr><td>Cell 1</td><td>Cell 2</td></tr></table>");
        let table = doc.select("table");
        let mut state = ExtractionState::new();
        state.add_potential_tag("table");
        let opts = Options::default();

        let processed = handle_table(&table, &mut state, &opts);
        assert!(processed.is_some());

        let t_doc = processed.unwrap();
        let t = t_doc.select("table");
        assert_eq!(t.select("td").length(), 2);
    }

    #[test]
    fn test_handle_table_strips_structural() {
        let doc = dom::parse("<table><thead><tr><th>Header</th></tr></thead><tbody><tr><td>Data</td></tr></tbody></table>");
        let table = doc.select("table");
        let mut state = ExtractionState::new();
        state.add_potential_tag("table");
        let opts = Options::default();

        let processed = handle_table(&table, &mut state, &opts);
        assert!(processed.is_some());

        // thead is stripped from input, we output a single tbody
        let t_doc = processed.unwrap();
        let t = t_doc.select("table");
        assert_eq!(t.select("thead").length(), 0);
        // We now always output with tbody wrapper
        assert_eq!(t.select("tbody").length(), 1);
    }

    #[test]
    fn test_handle_table_empty() {
        let doc = dom::parse("<table><tr></tr></table>");
        let table = doc.select("table");
        let mut state = ExtractionState::new();
        let opts = Options::default();

        let processed = handle_table(&table, &mut state, &opts);
        assert!(processed.is_none());
    }

    #[test]
    fn test_handle_table_multiple_rows() {
        let doc = dom::parse("<table><tr><td>R1C1</td></tr><tr><td>R2C1</td></tr></table>");
        let table = doc.select("table");
        let mut state = ExtractionState::new();
        state.add_potential_tag("table");
        let opts = Options::default();

        let processed = handle_table(&table, &mut state, &opts);
        assert!(processed.is_some());

        let t_doc = processed.unwrap();
        let t = t_doc.select("table");
        assert_eq!(t.select("tr").length(), 2);
    }

    #[test]
    fn test_handle_table_cell_with_formatting() {
        let doc = dom::parse("<table><tr><td><b>Bold</b> and <em>italic</em> text</td></tr></table>");
        let table = doc.select("table");
        let mut state = ExtractionState::new();
        state.add_potential_tag("table");
        let opts = Options::default();

        let processed = handle_table(&table, &mut state, &opts);
        assert!(processed.is_some());

        let t_doc = processed.unwrap();
        let html = dom::outer_html(&t_doc.select("table"));
        // Formatting should be preserved
        assert!(html.contains("<b>") || html.contains("Bold"));
        assert!(html.contains("<em>") || html.contains("italic"));
    }

    #[test]
    fn test_handle_table_cell_with_list_recall_mode() {
        let doc = dom::parse("<table><tr><td><ul><li>Item 1</li><li>Item 2</li></ul></td></tr></table>");
        let table = doc.select("table");
        let mut state = ExtractionState::new();
        state.add_potential_tag("table");
        let opts = Options {
            favor_recall: true,
            ..Options::default()
        };

        let processed = handle_table(&table, &mut state, &opts);
        assert!(processed.is_some());

        let t_doc = processed.unwrap();
        let html = dom::outer_html(&t_doc.select("table"));
        // In recall mode, list content should be included
        assert!(html.contains("Item 1") || html.contains("ul"));
    }

    #[test]
    fn test_handle_table_cell_with_nested_elements() {
        let doc = dom::parse("<table><tr><td><p>Paragraph in cell</p></td></tr></table>");
        let table = doc.select("table");
        let mut state = ExtractionState::new();
        state.add_potential_tag("table");
        let opts = Options::default();

        let processed = handle_table(&table, &mut state, &opts);
        assert!(processed.is_some());

        let t_doc = processed.unwrap();
        let text = dom::text_content(&t_doc.select("table"));
        assert!(text.contains("Paragraph"));
    }
}
