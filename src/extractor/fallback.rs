//! Fallback Extraction
//!
//! This module ports fallback extraction from go-trafilatura's baseline.go and external.go.
//! It provides baseline extraction (JSON-LD, paragraph scraping) and external fallback using `dom_smoothie`.

use dom_query::{Document, Selection};
use serde_json::Value;
use crate::dom;
use crate::etree;
use crate::selector::discard::should_discard;
use crate::Options;
use super::pruning::prune_unwanted_nodes;
use super::tags::VALID_TAG_CATALOG;

// === Baseline Extraction ===

static BASIC_CLEANING_SELECTOR: &str = "aside, footer, nav, header, div[id*=\"footer\"], div[class*=\"footer\"], div[class*=\"consent\"], div[class*=\"cookie\"], div[class*=\"privacy\"], div[class*=\"gdpr\"], div[class*=\"banner\"], div[class*=\"modal\"], div[class*=\"popup\"], div[class*=\"newsletter\"], script, style, noscript";

/// Basic document cleaning for baseline extraction.
///
/// Go equivalent: `basicCleaning(doc)` (lines 22-28)
fn basic_cleaning(doc: &Document) {
    let discarded = doc.select(BASIC_CLEANING_SELECTOR).nodes().to_vec();
    for node in discarded.into_iter().rev() {
        etree::remove(&Selection::from(node), false);
    }

    let all_elements = doc.select("*").nodes().to_vec();
    for node in all_elements.into_iter().rev() {
        let sel = Selection::from(node);
        let tag = dom::tag_name(&sel).unwrap_or_default().to_ascii_lowercase();
        if matches!(tag.as_str(), "script" | "style" | "noscript") {
            etree::remove(&sel, false);
        }
    }
}

/// Extract content from Discourse forum's data-preloaded attribute.
///
/// Discourse forums use client-side rendering and embed the actual post content
/// in a hidden div with id="data-preloaded". This function extracts the "cooked"
/// HTML content from each post and converts it to plain text.
#[must_use]
pub fn extract_discourse_content(doc: &Document) -> Option<String> {
    // Look for the data-preloaded div
    let preloaded_selection = doc.select("#data-preloaded");
    let preloaded = preloaded_selection.nodes();
    if preloaded.is_empty() {
        return None;
    }

    let preloaded_sel = Selection::from(*preloaded.first()?);
    let data_attr = preloaded_sel.attr("data-preloaded")?;
    let data_str = data_attr.to_string();

    if data_str.is_empty() {
        return None;
    }

    // The data is HTML-entity encoded JSON
    let decoded = data_str
        .replace("&quot;", "\"")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&#39;", "'");

    // Parse the outer JSON object
    let outer: Value = serde_json::from_str(&decoded).ok()?;

    // Look for topic data (usually in a key like "topic_NNNNNN")
    let topic_data = outer.as_object()?.iter()
        .find(|(k, _)| k.starts_with("topic_"))
        .map(|(_, v)| v)?;

    // The value is a JSON string that needs to be parsed again
    let topic_str = topic_data.as_str()?;
    let topic: Value = serde_json::from_str(topic_str).ok()?;

    // Extract posts from post_stream.posts
    let posts = topic.get("post_stream")?.get("posts")?.as_array()?;

    let mut content_parts = Vec::new();
    for post in posts {
        if let Some(cooked) = post.get("cooked").and_then(|v| v.as_str()) {
            // The "cooked" field contains HTML - extract text
            // First unescape the unicode escapes
            let unescaped = cooked
                .replace("\\u003c", "<")
                .replace("\\u003e", ">")
                .replace("\\u0026", "&")
                .replace("\\n", "\n")
                .replace("\\\"", "\"");

            // Parse as HTML and extract text
            let temp_doc = Document::from(format!("<div>{unescaped}</div>"));
            let text = dom::text_content(&temp_doc.select("div")).trim().to_string();

            if !text.is_empty() {
                content_parts.push(text);
            }
        }
    }

    if content_parts.is_empty() {
        return None;
    }

    Some(content_parts.join("\n\n"))
}

/// Recursively find articleBody in JSON-LD data.
fn find_article_body(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            for (key, val) in map {
                if key.to_lowercase() == "articlebody" {
                    if let Value::String(s) = val {
                        return Some(s.clone());
                    }
                }

                // Recurse into nested objects
                if let Some(found) = find_article_body(val) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(arr) => {
            for item in arr {
                if let Some(found) = find_article_body(item) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

/// Extract article body from JSON-LD script tags.
///
/// This function looks for `articleBody` fields in JSON-LD structured data,
/// which often contains the complete article text. Many modern sites include
/// full article content in JSON-LD for SEO/accessibility purposes.
///
/// Go equivalent: Part of `baseline(doc)` (lines 39-94)
#[must_use]
pub fn extract_json_ld_article_body(doc: &Document) -> Option<String> {
    for script in doc.select(r#"script[type="application/ld+json"]"#).nodes() {
        let script_sel = Selection::from(*script);
        let json_text = dom::text_content(&script_sel).trim().to_string();

        if json_text.is_empty() {
            continue;
        }

        // Parse JSON
        let data: Value = match serde_json::from_str(&json_text) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Recursively search for articleBody
        if let Some(article_body) = find_article_body(&data) {
            let body = article_body.trim().to_string();
            if !body.is_empty() {
                // If contains HTML, extract text
                if body.contains("<p>") {
                    let temp_doc = Document::from(format!("<div>{body}</div>"));
                    return Some(dom::text_content(&temp_doc.select("div")).trim().to_string());
                }
                return Some(body);
            }
        }
    }

    None
}

/// Baseline extraction function targeting text paragraphs and/or JSON metadata.
///
/// Go equivalent: `baseline(doc)` (lines 30-152)
///
/// # Returns
/// * `(post_body, text)` - Extracted body document and plain text
#[must_use]
pub fn baseline(doc: &Document) -> (Document, String) {
    let post_body_doc = etree::element("body");
    let post_body = post_body_doc.select("body");
    let mut tmp_text = String::new();

    // 1. Try JSON-LD article body first
    if let Some(article_body) = extract_json_ld_article_body(doc) {
        let p = etree::sub_element(&post_body, "p");
        etree::set_text(&p, &article_body);
        tmp_text = article_body;

        // If substantial content found, return
        if tmp_text.chars().count() > 100 {
            return (post_body_doc, tmp_text.trim().to_string());
        }
    }

    // 2. Basic tree cleaning
    basic_cleaning(doc);

    // 3. Try <article> or <story> tags (some news sites use <story>)
    let article_selector = "article, story, story bodytext";
    if let Some(article_node) = doc.select(article_selector).nodes().first() {
        let article = Selection::from(*article_node);
        let article_text = dom::text_content(&article).trim().to_string();

        if article_text.chars().count() > 100 {
            let p = etree::sub_element(&post_body, "p");
            etree::set_text(&p, &article_text);
            tmp_text = format!("{tmp_text} {article_text}");
        }
    }

    if !dom::children(&post_body).is_empty() {
        return (post_body_doc, tmp_text.trim().to_string());
    }

    // 4. Scrape from text paragraphs
    let mut seen = std::collections::HashSet::new();
    for node in etree::iter(
        &doc.select("body"),
        &["blockquote", "pre", "q", "code", "p"],
    ).nodes() {
        let elem = Selection::from(*node);

        // Skip elements that match discard rules (e.g., MenuItem, navigation)
        if should_discard(&elem) {
            continue;
        }

        // Also check parent elements for discard patterns
        let parent = elem.parent();
        if !parent.is_empty() && should_discard(&parent) {
            continue;
        }

        let entry = dom::text_content(&elem).trim().to_string();

        // Skip cookie consent and tracking-related paragraphs
        let entry_lower = entry.to_lowercase();
        if entry_lower.contains("cookie") && entry_lower.contains("consent") {
            continue;
        }
        if entry_lower.contains("tracking technolog") {
            continue;
        }
        // Skip paragraphs that are mostly navigation/menu text (high ratio of newlines)
        let newline_count = entry.matches('\n').count();
        let word_count = entry.split_whitespace().count();
        if word_count > 0 && newline_count > word_count / 2 {
            continue;
        }

        if !seen.contains(&entry) && !entry.is_empty() {
            let p = etree::sub_element(&post_body, "p");
            etree::set_text(&p, &entry);
            tmp_text = format!("{tmp_text} {entry}");
            seen.insert(entry);
        }
    }

    let tmp_text = tmp_text.trim().to_string();
    if tmp_text.chars().count() > 100 {
        return (post_body_doc, tmp_text);
    }

    // If we have some paragraphs from step 4, return them even if < 100 chars
    if !dom::children(&post_body).is_empty() {
        return (post_body_doc, tmp_text);
    }

    // 5. Default strategy: take everything from body
    if let Some(body_node) = doc.select("body").nodes().first() {
        let body = Selection::from(*body_node);
        let text = etree::iter_text(&body, "\n").trim().to_string();

        if text.chars().count() > 100 {
            let elem = etree::sub_element(&post_body, "p");
            etree::set_text(&elem, &text);
            return (post_body_doc, text);
        }
    }

    // 6. Final fallback: entire document text
    let text = dom::text_content(&doc.select("*")).trim().to_string();
    let elem = etree::sub_element(&post_body, "p");
    etree::set_text(&elem, &text);

    (post_body_doc, text)
}

// === External Fallback (dom_smoothie) ===

static TAGS_TO_SANITIZE: &[&str] = &[
    "aside", "audio", "button", "fieldset", "figure", "footer", "iframe",
    "input", "label", "link", "nav", "noindex", "noscript",
    "object", "option", "select", "source", "svg", "time",
    "script", "style",
];

/// CSS selector for common social share plugin elements to remove from fallback output.
/// These are specific plugin classes that inject share buttons into article content.
static SHARE_PLUGIN_SELECTOR: &str = "[class*=\"dpsp-\"], [class*=\"wabtn\"], [class*=\"addtoany\"], [class*=\"shareaholic\"], [class*=\"share-wrapper\"], [class*=\"social-share\"], [class*=\"share-buttons\"], [id*=\"share-buttons\"], [class*=\"post-share\"], [class*=\"entry-share\"]";

/// Check if fallback candidate is good enough to use.
///
/// This function implements go-trafilatura's `candidateIsUsable` heuristics which determine
/// whether a fallback candidate should replace the current extraction. The key heuristics are:
///
/// 1. **Ratio checks**: Candidate must not shrink content by >50% (protects good extractions)
/// 2. **2x bigger**: Accept candidates that are 2x+ larger (significant improvement)
/// 3. **Borderline cases**: Structural analysis for similar-length content:
///    - No paragraph text in extracted → use candidate (handles table-based layouts)
///    - More tables than paragraphs → use candidate (table-heavy pages)
///    - FavorRecall with headings → use candidate
///
/// Go equivalent: `candidateIsUsable(candidateDoc, extractedDoc, lenCandidate, lenExtracted, opts)` (lines 163-202)
#[must_use]
pub fn candidate_is_usable(
    candidate_doc: &Selection,
    extracted_doc: &Selection,
    len_candidate: usize,
    len_extracted: usize,
    opts: &Options,
) -> bool {
    let min_size = opts.min_extracted_size;

    if len_candidate == 0 || len_candidate == len_extracted {
        return false;
    }

    if len_extracted == 0 && len_candidate > 0 {
        return true;
    }

    // Fix 5: Extreme over-extraction sanity check
    // If we extracted >5x more than Readability found, we almost certainly grabbed boilerplate
    // Trust Readability in this case (no paragraph density check needed)
    if len_extracted > 5 * len_candidate && len_candidate >= min_size {
        return true;
    }

    // If our extraction is much larger than candidate (2-5x), check if it's low quality
    // (mostly non-paragraph content, indicating we grabbed boilerplate/navigation)
    if len_extracted > 2 * len_candidate {
        // Check paragraph text ratio in our extraction
        let p_text_length: usize = extracted_doc
            .select("p")
            .nodes()
            .iter()
            .map(|n| etree::iter_text(&Selection::from(*n), " ").trim().chars().count())
            .sum();

        // Fix 6: Tighter paragraph density check (raised from 30% to 40%)
        // If less than 40% of our extracted text is in paragraphs, we likely grabbed boilerplate
        if len_extracted > 0 && p_text_length * 100 / len_extracted < 40 && len_candidate >= min_size {
            return true;
        }

        // Also accept if ratio is >3x even with decent paragraph density
        // (we're still probably grabbing extra boilerplate)
        if len_extracted > 3 * len_candidate && len_candidate >= min_size {
            return true;
        }

        return false;
    }

    if len_candidate > 2 * len_extracted {
        return true;
    }

    // Borderline case - check structure
    let extracted_tables = extracted_doc.select("table").length();
    let extracted_paragraphs = extracted_doc.select("p").length();
    let candidate_headings = candidate_doc.select("h2, h3, h4").length();

    // Calculate paragraph text length for quality assessment
    // (reuse value if already calculated above)
    let p_text_length: usize = extracted_doc
        .select("p")
        .nodes()
        .iter()
        .map(|n| etree::iter_text(&Selection::from(*n), " ").trim().chars().count())
        .sum();

    if p_text_length == 0 && len_candidate > min_size * 2 {
        return true;
    }

    if extracted_tables > extracted_paragraphs && len_candidate > min_size * 2 {
        return true;
    }

    if opts.favor_recall {
        let extracted_heads = extracted_doc.select("head").length();
        if extracted_heads == 0 && candidate_headings > 0 && len_candidate > len_extracted {
            return true;
        }
    }

    // Check if must favor recall due to insufficient content
    len_extracted < min_size && opts.favor_recall
}

/// Sanitize tree from external extraction (post-processing).
///
/// Go equivalent: `sanitizeTree(tree, opts)` (lines 204-242)
fn sanitize_tree(tree: &Selection, opts: &Options) {
    // 1. Clean
    // doc_cleaning equivalent for external output
    let sub_elements = tree.select("*").nodes().to_vec();
    for node in sub_elements.into_iter().rev() {
        let elem = Selection::from(node);
        let tag = dom::tag_name(&elem).unwrap_or_default().to_ascii_lowercase();
        if TAGS_TO_SANITIZE.contains(&tag.as_str()) {
            etree::remove(&elem, false);
        }
    }

    // 1b. Remove social share plugin elements
    // These are commonly injected into article content by WordPress plugins
    let share_elements = tree.select(SHARE_PLUGIN_SELECTOR).nodes().to_vec();
    for node in share_elements.into_iter().rev() {
        etree::remove(&Selection::from(node), false);
    }

    // Strip links if not included
    if !opts.include_links {
        etree::strip_tags(tree, &["a"]);
    }

    etree::strip_tags(tree, &["span"]);

    // 2. Sanitize - remove invalid tags
    let mut tags_to_strip = Vec::new();
    let mut seen_tags = std::collections::HashSet::new();

    for node in tree.select("*").nodes() {
        let elem = Selection::from(*node);
        let tag = dom::tag_name(&elem).unwrap_or_default();

        if seen_tags.contains(&tag) {
            continue;
        }
        seen_tags.insert(tag.clone());

        if !VALID_TAG_CATALOG.contains(tag.as_str()) {
            tags_to_strip.push(tag);
        }
    }

    if !tags_to_strip.is_empty() {
        let tags_refs: Vec<&str> = tags_to_strip.iter().map(std::string::String::as_str).collect();
        etree::strip_tags(tree, &tags_refs);
    }
}

/// Compare and choose between own extraction and external extraction.
///
/// Go equivalent: `compareExternalExtraction(originalDoc, extractedDoc, opts)` (lines 50-101)
///
/// # Arguments
/// * `original_doc` - The original document
/// * `extracted_doc` - Our extraction result
/// * `opts` - Extraction options
///
/// # Returns
/// * `(final_doc, final_text)` - The best extraction result
#[must_use]
pub fn compare_external_extraction(
    original_doc: &Document,
    extracted_doc: &Selection,
    opts: &Options,
) -> (Document, String) {
    let extracted_text = etree::iter_text(extracted_doc, " ").trim().to_string();
    let len_extracted = extracted_text.chars().count();
    let min_size = opts.min_extracted_size;

    // Bypass for favor recall with substantial content
    if opts.favor_recall && len_extracted > min_size * 10 {
        // Clone extracted_doc's owning document
        let extracted_html = dom::outer_html(extracted_doc);
        let result_doc = Document::from(extracted_html);
        return (result_doc, extracted_text);
    }

    // Prior cleaning for precision mode
    let cleaned_doc = if opts.favor_precision {
        let cloned = dom::clone_document(original_doc);
        let _ = prune_unwanted_nodes(
            &cloned.select("body"),
            crate::selector::discard::OVERALL_DISCARDED_CONTENT,
            true,
        );
        cloned
    } else {
        dom::clone_document(original_doc)
    };

    // Remove social share plugin elements BEFORE calling Readability
    // This prevents share buttons from being included in the article content
    let share_elements = cleaned_doc.select(SHARE_PLUGIN_SELECTOR).nodes().to_vec();
    for node in share_elements.into_iter().rev() {
        etree::remove(&Selection::from(node), false);
    }

    // Track best result as Document
    let mut best_result: Option<(Document, String, usize)> = None;

    // Attempt Readability extraction via dom_smoothie
    #[cfg(feature = "readability")]
    {
        use dom_smoothie::Readability;

        if let Ok(mut reader) = Readability::with_document(cleaned_doc.clone(), None, None) {
            if let Ok(article) = reader.parse() {
                // Parse the content HTML to get a Document
                let content_html = article.content.to_string();
                let content_doc = Document::from(content_html);
                let content_node = content_doc.select("body");

                let candidate_text = etree::iter_text(&content_node, " ").trim().to_string();
                let len_candidate = candidate_text.chars().count();

                if candidate_is_usable(&content_node, extracted_doc, len_candidate, len_extracted, opts) {
                    if len_candidate >= min_size {
                        sanitize_tree(&content_node, opts);
                        let final_text = etree::iter_text(&content_node, " ").trim().to_string();
                        return (content_doc, final_text);
                    }
                    best_result = Some((content_doc, candidate_text.clone(), len_candidate));
                }
            }
        }
    }

    // If no feature, just use our extraction
    #[cfg(not(feature = "readability"))]
    {
        // No external fallback available, use our extraction
        let _ = cleaned_doc; // Suppress unused variable warning
    }

    // Use best result if available, otherwise use our extraction
    let result_doc = if let Some((doc, _text, _)) = best_result {
        doc
    } else {
        let extracted_html = dom::outer_html(extracted_doc);
        Document::from(extracted_html)
    };

    // Final sanitization
    sanitize_tree(&result_doc.select("body"), opts);
    let final_text = etree::iter_text(&result_doc.select("body"), " ").trim().to_string();
    (result_doc, final_text)
}

/// Main fallback extraction entry point.
///
/// Tries baseline extraction first, then external fallback if needed.
#[must_use]
pub fn extract_with_fallback(
    doc: &Document,
    our_extraction: &Selection,
    our_text: &str,
    opts: &Options,
) -> (Document, String) {
    let our_len = our_text.chars().count();
    let min_size = opts.min_extracted_size;

    // If we have sufficient content, try external comparison
    if our_len >= min_size {
        return compare_external_extraction(doc, our_extraction, opts);
    }

    // Try baseline extraction
    let (baseline_body_doc, baseline_text) = baseline(doc);
    let baseline_len = baseline_text.chars().count();

    // Compare baseline with our extraction
    if baseline_len > our_len && baseline_len >= min_size {
        return (baseline_body_doc, baseline_text);
    }

    // If still insufficient, try external
    if our_len < min_size {
        return compare_external_extraction(doc, our_extraction, opts);
    }

    // Return our extraction as a Document
    let extracted_html = dom::outer_html(our_extraction);
    let result_doc = Document::from(extracted_html);
    (result_doc, our_text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_ld_simple() {
        let html = r#"<!DOCTYPE html>
        <html>
        <head>
            <script type="application/ld+json">
            {
                "@type": "Article",
                "articleBody": "This is the article body content."
            }
            </script>
        </head>
        <body></body>
        </html>"#;

        let doc = Document::from(html);
        let result = extract_json_ld_article_body(&doc);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), "This is the article body content.");
    }

    #[test]
    fn test_extract_json_ld_nested() {
        let html = r#"<!DOCTYPE html>
        <html>
        <head>
            <script type="application/ld+json">
            {
                "@graph": [{
                    "@type": "Article",
                    "articleBody": "Nested article body."
                }]
            }
            </script>
        </head>
        <body></body>
        </html>"#;

        let doc = Document::from(html);
        let result = extract_json_ld_article_body(&doc);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), "Nested article body.");
    }

    #[test]
    fn test_extract_json_ld_with_html() {
        let html = r#"<!DOCTYPE html>
        <html>
        <head>
            <script type="application/ld+json">
            {
                "articleBody": "<p>Paragraph one.</p><p>Paragraph two.</p>"
            }
            </script>
        </head>
        <body></body>
        </html>"#;

        let doc = Document::from(html);
        let result = extract_json_ld_article_body(&doc);

        assert!(result.is_some());
        // HTML should be stripped
        let text = result.unwrap();
        assert!(text.contains("Paragraph one"));
        assert!(!text.contains("<p>"));
    }

    #[test]
    fn test_baseline_with_article_tag() {
        let html = r#"<!DOCTYPE html>
        <html>
        <body>
            <nav>Navigation</nav>
            <article>
                This is a long article content that exceeds the minimum threshold
                for extraction. It contains multiple sentences to ensure we have
                enough text content.
            </article>
            <footer>Footer</footer>
        </body>
        </html>"#;

        let doc = Document::from(html);
        let (body_doc, text) = baseline(&doc);
        let body = body_doc.select("body");

        assert!(!dom::children(&body).is_empty());
        assert!(text.contains("article content"));
    }

    #[test]
    fn test_baseline_paragraph_scraping() {
        let html = r#"<!DOCTYPE html>
        <html>
        <body>
            <p>First paragraph with sufficient content for extraction.</p>
            <p>Second paragraph also with content.</p>
            <p>Third paragraph completing the text.</p>
        </body>
        </html>"#;

        let doc = Document::from(html);
        let (_body_doc, text) = baseline(&doc);

        assert!(!text.is_empty());
        assert!(text.contains("First paragraph"));
    }

    #[test]
    fn test_baseline_deduplication() {
        let html = r#"<!DOCTYPE html>
        <html>
        <body>
            <p>Duplicate text</p>
            <p>Duplicate text</p>
            <p>Unique text</p>
        </body>
        </html>"#;

        let doc = Document::from(html);
        let (body_doc, _text) = baseline(&doc);
        let body = body_doc.select("body");

        // Should only have 2 paragraphs (duplicates removed)
        assert_eq!(body.select("p").length(), 2);
    }

    #[test]
    fn test_candidate_is_usable_empty_extracted() {
        let doc1 = dom::parse("<div></div>");
        let doc2 = dom::parse("<div><p>Content</p></div>");
        let opts = Options::default();

        let result = candidate_is_usable(
            &doc2.select("div"),
            &doc1.select("div"),
            100,  // candidate length
            0,    // extracted length
            &opts,
        );

        assert!(result);
    }

    #[test]
    fn test_candidate_is_usable_much_longer() {
        let doc1 = dom::parse("<div><p>Short</p></div>");
        let doc2 = dom::parse("<div><p>Much longer content here</p></div>");
        let opts = Options::default();

        let result = candidate_is_usable(
            &doc2.select("div"),
            &doc1.select("div"),
            500,   // candidate length
            100,   // extracted length
            &opts,
        );

        assert!(result);
    }

    #[test]
    fn test_sanitize_tree_removes_invalid() {
        let doc = dom::parse("<div><aside>Aside</aside><p>Keep</p><nav>Nav</nav></div>");
        let root = doc.select("div");
        let opts = Options::default();

        sanitize_tree(&root, &opts);

        // aside and nav should be removed
        assert_eq!(root.select("aside").length(), 0);
        assert_eq!(root.select("nav").length(), 0);
        // p should remain
        assert_eq!(root.select("p").length(), 1);
    }

    #[test]
    fn test_sanitize_tree_strips_links_when_disabled() {
        let doc = dom::parse(r##"<div><p>Text with <a href="#">link</a></p></div>"##);
        let root = doc.select("div");
        let opts = Options { include_links: false, ..Options::default() };

        sanitize_tree(&root, &opts);

        assert_eq!(root.select("a").length(), 0);
    }
}

#[cfg(test)]
mod share_plugin_tests {
    use super::*;
    use crate::dom;
    
    #[test]
    fn test_share_plugin_selector_matches() {
        let html = r#"
        <html><body>
        <p class="dpsp-share-text">Sharing is caring!</p>
        <div id="dpsp-content-top" class="dpsp-content-wrapper">Share buttons</div>
        <p>Real content here</p>
        </body></html>
        "#;
        
        let doc = dom::parse(html);
        let matches = doc.select(SHARE_PLUGIN_SELECTOR);
        
        assert!(matches.length() >= 2, "Expected at least 2 matches, got {}", matches.length());
    }
}
