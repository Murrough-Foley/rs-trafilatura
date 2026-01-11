//! Core content extraction algorithm.
//!
//! This module contains the main extraction logic ported from go-trafilatura.
//! It handles HTML parsing, content identification, boilerplate removal,
//! and metadata extraction.

use crate::dom::{self, Document, Selection};
use crate::error::{Error, Result};
use crate::etree;
use crate::extractor::fallback;
use crate::html_processing;
use crate::link_density::{link_density_test, link_density_test_tables};
use crate::metadata;
use crate::options::Options;
use crate::patterns::{
    ADVERTISEMENT_CLASS, ARTICLE_SELECTOR, BOILERPLATE_CLASS, COMMENT_CLASS,
    COMMENT_ID, LINE_WHITESPACE, MAIN_SELECTOR, MULTIPLE_NEWLINES,
    NAVIGATION_CLASS, WHITESPACE_NORMALIZE,
};
use crate::selector;
use crate::result::{ExtractResult, ImageData};
use crate::url_utils::{extract_filename, filenames_match};

/// Main entry point for content extraction.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn extract_content(html: &str, options: &Options) -> Result<ExtractResult> {
    if cfg!(debug_assertions) {
        eprintln!("DEBUG: Starting content extraction (HTML length: {} chars)", html.len());
    }

    // Parse HTML document
    let document = Document::from(html);

    let mut warnings = Vec::new();

    // Extract metadata first (works on full document before cleaning)
    // Uses the metadata module which provides:
    // - JSON-LD parsing with proper schema handling
    // - Meta tag extraction (og:, twitter:, dublin core)
    // - DOM fallback extraction
    // - Author blacklist filtering
    let metadata = metadata::extract_metadata(&document, options);

    if cfg!(debug_assertions) {
        if let Some(ref title) = metadata.title {
            eprintln!("DEBUG: Extracted metadata - Title: {} chars", title.len());
        } else {
            eprintln!("DEBUG: No title found in metadata");
        }
    }

    // Create document backup BEFORE cleaning for fallback extraction
    // Go-trafilatura pattern: docBackup is used by baseline() and recoverWildText()
    // when main extraction fails. Without this, content inside <form> tags
    // (common in legacy pages) would be lost after doc_cleaning removes them.
    let doc_backup = dom::clone_document(&document);

    // Fix 9: Try JSON-LD articleBody extraction FIRST (before cleaning removes scripts)
    // Many modern sites include full article content in JSON-LD structured data.
    // This is more reliable than DOM-based extraction for sites that use it.
    const MIN_STRUCTURED_BODY_LEN: usize = 500; // Require substantial content
    let json_ld_body = fallback::extract_json_ld_article_body(&document);
    let use_json_ld = json_ld_body.as_ref().is_some_and(|body| body.chars().count() >= MIN_STRUCTURED_BODY_LEN);

    // Fix 10: Try Discourse forum extraction (data-preloaded attribute)
    // Discourse forums use client-side rendering and embed content in a hidden div.
    let discourse_body = fallback::extract_discourse_content(&document);
    let use_discourse = discourse_body.as_ref().is_some_and(|body| body.chars().count() >= MIN_STRUCTURED_BODY_LEN);

    // Clean document before content extraction (go-trafilatura: docCleaning)
    // This removes noise elements (script, style, nav, footer, form, etc.) that could
    // affect content selection. Must happen AFTER metadata extraction since
    // metadata relies on elements that get cleaned (head, meta tags).
    //
    // Note: This may cause temporary regressions on some legacy datasets (l3s-gn1,
    // google-trends-2017) as we align with go-trafilatura's extraction flow.
    // These will be addressed by subsequent epics (link density, element handlers).
    html_processing::doc_cleaning(&document, options);

    // Find and extract main content (graceful degradation on failure)
    // If we have substantial JSON-LD content, still run DOM extraction but compare results
    let page_title = metadata.title.as_deref();
    let (mut content_text, mut content_html) = match extract_main_content(&document, options, page_title) {
        Ok((text, html)) => (text, html),
        Err(Error::NoContent) => {
            warnings.push("Content extraction failed - no main content found".to_string());
            (String::new(), None)
        }
        Err(e) => {
            warnings.push(format!("Content extraction failed: {e}"));
            (String::new(), None)
        }
    };

    // Try fallback extraction when main extraction may be insufficient
    // Only trigger when content is potentially under-extracted, following original RS logic.
    // Go-trafilatura always calls fallback but has different main extraction results.
    // We use conditional triggering + candidateIsUsable for better precision.
    let content_len = content_text.chars().count();
    let min_extracted_len = options.min_extracted_len;
    let word_count = count_words(&content_text, options.min_word_length);

    // Detect potential under-extraction: no paragraphs or table-heavy content
    // suggests wrong content was selected (e.g., footer, navigation, data table).
    let under_extracted = if let Some(ref html) = content_html {
        let doc = Document::from(html.as_str());
        let p_count = doc.select("p").length();
        let table_count = doc.select("table").length();
        p_count == 0 || (table_count > 0 && table_count >= p_count)
    } else {
        true // No HTML means definitely under-extracted
    };

    // Also check word count - navigation/footer often has few words but many chars
    // (e.g., "Home | About | Contact" passes char check but has low word count)
    let insufficient_words = word_count < options.min_output_size;

    // Detect navigation-like content: starts with common nav links or has repeated text
    // Navigation often starts with "Home About Contact..." pattern
    let looks_like_navigation = {
        let lower = content_text.to_lowercase();
        // Get first ~100 chars safely (on char boundary)
        let first_100: String = lower.chars().take(100).collect();
        // Count navigation keywords in first 100 chars
        let nav_keywords = ["home", "about", "contact", "links", "menu", "search", "login"];
        let nav_count = nav_keywords.iter().filter(|k| first_100.contains(*k)).count();
        nav_count >= 3 // 3+ nav keywords at start suggests wrong content
    };

    if options.use_readability_fallback && (content_len < min_extracted_len || under_extracted || insufficient_words || looks_like_navigation) {
        // Use doc_backup (pre-cleaning) for fallback - critical for pages where
        // content is inside <form> tags that get removed by doc_cleaning
        // Pass content_html for proper structural comparison in candidate_is_usable
        let (fallback_text, fallback_html) =
            try_fallback_extraction(&doc_backup, &content_text, content_html.as_deref(), options);

        // try_fallback_extraction uses candidate_is_usable heuristics internally:
        // - Won't accept candidates that shrink content by >50% (protects good extractions)
        // - Will accept candidates that are 2x+ larger (significant improvement)
        // - Uses structural analysis for borderline cases (p text, tables vs paragraphs)
        // If it returns Some(html), the result has been validated as an improvement
        if let Some(ref html) = fallback_html {
            let fallback_len = fallback_text.chars().count();
            warnings.push(format!(
                "Used fallback extraction: {fallback_len} chars (was {content_len} chars)"
            ));
            content_text = fallback_text;
            content_html = Some(html.clone());
        }
    }

    // Fix 9 & 10: Prefer structured data (JSON-LD or Discourse) when substantially better
    // Compare structured content with DOM extraction result
    let structured_body = if use_discourse {
        discourse_body.as_ref()
    } else if use_json_ld {
        json_ld_body.as_ref()
    } else {
        None
    };
    let structured_source = if use_discourse { "Discourse" } else { "JSON-LD" };

    if let Some(structured_text) = structured_body {
        let structured_len = structured_text.chars().count();
        let dom_len = content_text.chars().count();

        // Use structured data if:
        // 1. DOM extraction failed or is very short (<200 chars)
        // 2. Structured data is at least 2x larger than DOM extraction
        // 3. DOM extraction looks like navigation/boilerplate (low word ratio)
        let dom_failed = dom_len < 200;
        let structured_much_larger = structured_len > dom_len * 2;
        let dom_looks_like_boilerplate = {
            let lower = content_text.to_lowercase();
            let first_200: String = lower.chars().take(200).collect();
            // Check for cookie/consent/navigation patterns
            first_200.contains("cookie") || first_200.contains("consent")
                || first_200.contains("©") || first_200.contains("copyright")
                || (first_200.matches('\n').count() > first_200.split_whitespace().count() / 3)
        };

        if dom_failed || structured_much_larger || dom_looks_like_boilerplate {
            warnings.push(format!(
                "Using {structured_source} content: {structured_len} chars (DOM was {dom_len} chars)"
            ));
            content_text.clone_from(structured_text);
            // Create minimal HTML wrapper (escape basic HTML entities)
            let escaped = structured_text
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");
            content_html = Some(format!("<p>{escaped}</p>"));
        }
    }

    // Fix 7: Strip navigation patterns from extraction boundaries
    // (Disabled - testing showed marginal impact, may cause edge case regressions)
    // content_text = strip_navigation_boundaries(&content_text);

    // Extract comments if requested
    let (comments_text, comments_html) = if options.include_comments {
        extract_comments(&document, options)
    } else {
        (None, None)
    };

    // Extract images if requested
    let images = if options.include_images {
        extract_images(&document, metadata.image.as_deref())
    } else {
        Vec::new()
    };

    if cfg!(debug_assertions) {
        eprintln!("DEBUG: Extraction summary:");
        eprintln!("  Content text: {} chars", content_text.len());
        eprintln!("  Comments: {} chars", comments_text.as_ref().map_or(0, std::string::String::len));
        eprintln!("  Images: {}", images.len());
        eprintln!("  Warnings: {}", warnings.len());
    }

    // Build initial result
    let mut result = ExtractResult {
        content_text,
        content_html,
        // EPIC-02: Markdown output - populated in Story 3
        content_markdown: None,
        comments_text,
        comments_html,
        images,
        metadata,
        warnings,
    };

    // EPIC-02: Generate Markdown output if enabled
    // Uses quick_html2md for HTML→Markdown conversion with GFM support
    if options.output_markdown {
        if let Some(ref html) = result.content_html {
            use quick_html2md::{html_to_markdown_with_options, MarkdownOptions};

            // Map rs-trafilatura Options to quick_html2md MarkdownOptions
            let md_options = MarkdownOptions::new()
                .include_links(options.include_links)
                .include_images(options.include_images)
                .preserve_tables(options.include_tables);

            // Convert HTML to Markdown (quick_html2md handles tables natively)
            let raw_markdown = html_to_markdown_with_options(html, &md_options);

            // Post-process to escape special characters while preserving formatting
            let processed_markdown = crate::markdown::post_process_markdown(&raw_markdown);

            result.content_markdown = Some(processed_markdown);
        }
    }

    // Apply final validations and return
    let final_result = apply_final_validations(result, &document, options);

    if cfg!(debug_assertions) {
        if let Ok(ref res) = final_result {
            eprintln!("DEBUG: Extraction complete! Final content: {} chars", res.content_text.len());
        }
    }

    final_result
}

/// Counts words in text that meet minimum length requirement.
///
/// Words are split by whitespace. Only words with length >= `min_length` are counted.
fn count_words(text: &str, min_length: usize) -> usize {
    text.split_whitespace()
        .filter(|w| w.len() >= min_length)
        .count()
}

/// Attempts fallback extraction when main extraction produces insufficient content.
///
/// Following go-trafilatura's `compareExternalExtraction` pattern:
/// 1. Try baseline extraction (JSON-LD articleBody, paragraph scraping)
/// 2. Try external fallback (dom_smoothie/Readability)
/// 3. Use `candidate_is_usable` heuristics to choose best result
///
/// # Arguments
/// * `doc_backup` - Document backup created BEFORE doc_cleaning (preserves content in <form> tags)
/// * `current_text` - Text from main extraction attempt
/// * `current_html` - HTML from main extraction attempt (for proper comparison)
/// * `options` - Extraction options
///
/// Returns (text, html) of best extraction result, or None if no improvement.
fn try_fallback_extraction(
    doc_backup: &Document,
    current_text: &str,
    current_html: Option<&str>,
    options: &Options,
) -> (String, Option<String>) {
    let current_len = current_text.chars().count();
    let min_size = options.min_extracted_len;

    // Create Selection from current extraction for proper comparison
    // This allows candidate_is_usable to analyze the structure (p tags, tables, etc.)
    let extracted_doc = if let Some(html) = current_html {
        Document::from(format!("<html><body>{html}</body></html>"))
    } else {
        Document::from("<html><body></body></html>")
    };
    let extracted_sel = extracted_doc.select("body");

    // Clone for modification (remove share plugins) - doc_backup is already pre-cleaning
    let doc_for_fallback = dom::clone_document(doc_backup);

    // Remove social share plugin elements before fallback extraction
    const SHARE_PLUGIN_SELECTOR: &str = "[class*=\"dpsp-\"], [class*=\"wabtn\"], [class*=\"addtoany\"], [class*=\"shareaholic\"], [class*=\"share-wrapper\"], [class*=\"social-share\"], [class*=\"share-buttons\"], [id*=\"share-buttons\"], [class*=\"post-share\"], [class*=\"entry-share\"], [class*=\"shareModal\"], [class*=\"ShareModal\"]";
    for node in doc_for_fallback.select(SHARE_PLUGIN_SELECTOR).nodes() {
        dom::remove(&Selection::from(*node));
    }

    // Go-trafilatura flow (core.go lines 157-165):
    // 1. Try compareExternalExtraction (Readability) with candidateIsUsable
    // 2. Only if still below MinExtractedSize, use baseline as unconditional rescue

    // 1. Try external fallback (Readability) using candidateIsUsable
    // compare_external_extraction uses candidateIsUsable internally
    let (result_doc, result_text) =
        fallback::compare_external_extraction(&doc_for_fallback, &extracted_sel, options);
    let result_len = result_text.chars().count();
    let result_sel = result_doc.select("body");

    // Check if external result is usable using candidateIsUsable heuristics
    if fallback::candidate_is_usable(&result_sel, &extracted_sel, result_len, current_len, options) {
        let html = dom::outer_html(&result_sel).to_string();
        if result_len >= min_size {
            // Substantial improvement, use it
            return (result_text, Some(html));
        }
        // Track as potential result (but may still try baseline rescue)
    }

    // 2. Baseline as LAST RESORT rescue (unconditional, no candidateIsUsable)
    // Go-trafilatura: "Rescue: try to use original/dirty tree"
    // Only triggers when current content is still below min_size
    // Skip if favor_precision mode (go: Focus != FavorPrecision)
    if current_len < min_size && !options.favor_precision {
        let (baseline_doc, baseline_text) = fallback::baseline(&doc_for_fallback);
        let baseline_len = baseline_text.chars().count();

        // Unconditional rescue - just use baseline if it has content
        // Go doesn't apply candidateIsUsable here
        if baseline_len > 0 {
            let baseline_sel = baseline_doc.select("body");
            let html = dom::outer_html(&baseline_sel).to_string();
            return (baseline_text, Some(html));
        }
    }

    // No improvement found
    (current_text.to_string(), None)
}

/// Applies final validations and transformations to extraction result.
///
/// Checks content length and word count thresholds, applies max length limits,
/// and validates comments section. Triggers fallback extraction if content is insufficient.
#[allow(clippy::unnecessary_wraps)]
fn apply_final_validations(
    mut result: ExtractResult,
    _doc: &Document,
    options: &Options,
) -> Result<ExtractResult> {
    // Count words in main content
    let word_count = count_words(&result.content_text, options.min_word_length);

    // Check if content meets minimum thresholds
    let insufficient_content = word_count < options.min_output_size
        || result.content_text.len() < options.min_extracted_len;

    if insufficient_content {
        // Fallback was already attempted in extract_content if enabled
        // This warning indicates content is still insufficient after all attempts
        result.warnings.push(format!(
            "Insufficient content after extraction: {} words (min: {}), {} chars (min: {})",
            word_count,
            options.min_output_size,
            result.content_text.len(),
            options.min_extracted_len
        ));
    }

    // Apply maximum length limit
    if result.content_text.len() > options.max_extracted_len {
        result.content_text.truncate(options.max_extracted_len);
        result.warnings.push(format!(
            "Content truncated to max length: {}",
            options.max_extracted_len
        ));
    }

    // Validate comments section
    if let Some(ref comments) = result.comments_text {
        let comm_word_count = count_words(comments, options.min_word_length);
        if comm_word_count < options.min_output_comm_size {
            result.comments_text = None;
            result.comments_html = None;
            result.warnings.push(format!(
                "Comments section removed: {} words (min: {})",
                comm_word_count, options.min_output_comm_size
            ));
        }
    }

    Ok(result)
}

/// Attempt length-based fallback extraction using alternative selectors
///
/// This function is called when primary extraction yields very short results
/// (< 200 chars). It tries alternative semantic selectors and uses less
/// aggressive filtering to capture more content.
///
/// Currently disabled due to regressions - kept for future improvements.
#[allow(dead_code)]
fn try_length_based_fallback(
    doc: &Document,
    options: &Options,
    primary_text_len: usize,
) -> Option<(String, Option<String>)> {
    // Only trigger fallback for very short extractions
    if primary_text_len >= 200 {
        return None;
    }

    if cfg!(debug_assertions) {
        eprintln!("rs-trafilatura: primary extraction too short ({primary_text_len} chars); trying fallback");
    }

    // Try alternative selectors with relaxed filtering
    let fallback_selectors = [
        "article",
        "main",
        "[role='main']",
        "#content",
        ".content",
        "#main",
        ".main",
        "#main-content",
        ".main-content",
        "article[role='article']",
    ];

    let mut best_text = String::new();
    let mut best_html = String::new();
    let mut best_len = primary_text_len;

    for selector in &fallback_selectors {
        if cfg!(debug_assertions) {
            eprintln!("rs-trafilatura: fallback trying selector '{selector}'");
        }

        // Try to find content with this selector
        let selection = doc.select(selector);
        let fallback_nodes: Vec<_> = selection.nodes().iter().collect();
        let mut best_selection: Option<Selection> = None;
        let mut best_node_len = 0;

        for node in fallback_nodes {
            let sel = Selection::from(*node);

            // Skip nodes that are clearly not main content
            let _class = sel.attr("class").unwrap_or_default();
            let _id = sel.attr("id").unwrap_or_default();
            let text = sel.text();

            // Skip nav, header, footer, aside, etc.
            if let Some(tag) = node.node_name() {
                let tag_lower = tag.to_lowercase();
                if ["nav", "header", "footer", "aside", "script", "style"].contains(&tag_lower.as_str()) {
                    continue;
                }
            }

            // Skip nodes with mostly non-text content
            let trimmed_text = text.trim();
            if trimmed_text.len() < 50 {
                continue;
            }

            if trimmed_text.len() > best_node_len {
                best_node_len = trimmed_text.len();
                best_selection = Some(sel);
            }
        }

        if let Some(sel) = best_selection {
            let text = extract_filtered_text_allow_boilerplate(&sel, options);
            let text_len = text.trim().len();

            if text_len > best_len && text_len >= 200 {
                if cfg!(debug_assertions) {
                    eprintln!("rs-trafilatura: fallback selector '{selector}' found {text_len} chars (better than {best_len})");
                }

                best_text = text;
                best_len = text_len;
                best_html = extract_filtered_html_allow_boilerplate(&sel, options);

                // If we found a good result, use it
                if text_len >= primary_text_len * 2 {
                    break;
                }
            } else if cfg!(debug_assertions) {
                eprintln!("rs-trafilatura: fallback selector '{selector}' only found {text_len} chars (not better)");
            }
        }
    }

    if best_len > primary_text_len {
        if cfg!(debug_assertions) {
            eprintln!("rs-trafilatura: fallback successful! Improved from {primary_text_len} to {best_len} chars");
        }
        Some((best_text, if best_html.is_empty() { None } else { Some(best_html) }))
    } else {
        if cfg!(debug_assertions) {
            eprintln!("rs-trafilatura: fallback did not improve results (best was {best_len} chars)");
        }
        None
    }
}

/// Extracts main content from the document.
fn extract_main_content(doc: &Document, options: &Options, page_title: Option<&str>) -> Result<(String, Option<String>)> {
    if cfg!(debug_assertions) {
        eprintln!("DEBUG: Starting main content extraction");
    }

    // Try semantic selectors first
    let mut content_node = find_main_content_node_with_options(doc, options);

    if cfg!(debug_assertions) {
        if let Some(node) = &content_node {
            if let Some(tag) = dom::tag_name(node) {
                eprintln!("DEBUG: Found content node with tag: {tag}");
            }
        } else {
            eprintln!("DEBUG: No semantic content node found, will use body extraction");
        }
    }

    let (mut text, mut html) = if let Some(node) = &content_node {
        let text = extract_filtered_text_with_title(node, options, page_title);
        let html = extract_filtered_html(node, options);
        if cfg!(debug_assertions) {
            eprintln!("DEBUG: Extracted from content node: {} chars", text.len());
        }
        (text, html)
    } else {
        if cfg!(debug_assertions) {
            eprintln!("DEBUG: Using body extraction fallback");
        }
        (
            extract_body_content(doc, options)?,
            extract_body_content_html(doc, options)?,
        )
    };

    let mut extracted_from_content_node = content_node.is_some();
    let mut used_relaxed_filtering = false;

    if text.is_empty() {
        if cfg!(debug_assertions) {
            eprintln!("rs-trafilatura: selected content node produced empty text; falling back to body extraction");
        }
        text = extract_body_content(doc, options)?;
        html = extract_body_content_html(doc, options)?;
        extracted_from_content_node = false;
    }

    // Second fallback: if still empty, try extracting from semantic content node
    // with less aggressive filtering (allow some boilerplate classes)
    if text.is_empty() {
        if let Some(node) = find_main_content_node_with_options(doc, options) {
            if cfg!(debug_assertions) {
                eprintln!("rs-trafilatura: body extraction empty; trying content node with relaxed filtering");
            }
            text = extract_filtered_text_allow_boilerplate(&node, options);
            if !text.is_empty() {
                html = extract_filtered_html_allow_boilerplate(&node, options);
                content_node = Some(node);
                extracted_from_content_node = true;
                used_relaxed_filtering = true;
            }
        }
    }

    if extracted_from_content_node {
        if let Some(node) = &content_node {
            if let Some((merged_text, merged_html)) =
                maybe_merge_split_article_bodies(node, options, &text, &html, used_relaxed_filtering)
            {
                text = merged_text;
                html = merged_html;
            }
        }
    }

    // Length-based fallback: if extraction is very short, try alternative selectors
    // DISABLED - causing significant regressions by replacing good partial content with bad content
    // let _text_len = text.trim().len();
    // TODO: Re-enable when fallback logic is improved
    // if _text_len > 0 && _text_len < 50 {
    //     if let Some((fallback_text, fallback_html)) =
    //         try_length_based_fallback(doc, options, _text_len)
    //     {
    //         text = fallback_text;
    //         html = fallback_html.unwrap_or_default();
    //     }
    // }

    if text.is_empty() {
        if cfg!(debug_assertions) {
            eprintln!("DEBUG: Extraction failed - no content found");
        }
        return Err(Error::NoContent);
    }

    // TODO: Generate content_html when needed
    let content_html = if html.is_empty() { None } else { Some(html) };

    if cfg!(debug_assertions) {
        eprintln!("DEBUG: Extraction complete! Final text length: {} chars", text.len());
    }

    Ok((text, content_html))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SplitBodySignature {
    ArticleBody,
    BodyContainer,
    EntryContent,
    StoryBody,
}

fn split_body_signature_for_node(node: &Selection) -> Option<SplitBodySignature> {
    // Check class and id separately to avoid format! allocation
    let class = node.attr("class").unwrap_or_default().to_ascii_lowercase();
    let id = node.attr("id").unwrap_or_default().to_ascii_lowercase();

    // Check each signature pattern in both class and id
    for (pattern, signature) in [
        ("article__body", SplitBodySignature::ArticleBody),
        ("body__container", SplitBodySignature::BodyContainer),
        ("entry-content", SplitBodySignature::EntryContent),
        ("storybodycompanioncolumn", SplitBodySignature::StoryBody),
    ] {
        if class.contains(pattern) || id.contains(pattern) {
            return Some(signature);
        }
    }
    None
}

fn split_body_signature_token(signature: SplitBodySignature) -> &'static str {
    match signature {
        SplitBodySignature::ArticleBody => "article__body",
        SplitBodySignature::BodyContainer => "body__container",
        SplitBodySignature::EntryContent => "entry-content",
        SplitBodySignature::StoryBody => "storybodycompanioncolumn",
    }
}

fn find_nearest_article_ancestor<'a>(node: &Selection<'a>) -> Option<Selection<'a>> {
    let mut current = node.nodes().first().copied();
    while let Some(n) = current {
        if n.is_element() {
            if let Some(tag) = n.node_name() {
                if tag.eq_ignore_ascii_case("article") {
                    return Some(Selection::from(n));
                }
            }
        }
        current = n.parent();
    }
    None
}

fn find_split_body_candidates<'a>(article: &Selection<'a>, signature: SplitBodySignature) -> Vec<Selection<'a>> {
    let token = split_body_signature_token(signature);
    let mut out: Vec<Selection<'a>> = Vec::new();
    let mut kept_nodes: Vec<(dom_query::NodeId, usize)> = Vec::new();

    let Some(root) = article.nodes().first().copied() else {
        return out;
    };

    for node in root.descendants() {
        if !node.is_element() {
            continue;
        }

        // Skip candidates that are nested inside a previously selected candidate.
        // This avoids duplicate extraction when wrappers and inner nodes share the same token.
        let mut nested = false;
        for anc in node.ancestors(None) {
            let anc_key = (anc.id, std::ptr::from_ref(anc.tree) as usize);
            if kept_nodes.contains(&anc_key) {
                nested = true;
                break;
            }
        }
        if nested {
            continue;
        }

        let sel = Selection::from(node);
        let Some(class) = sel.attr("class") else {
            continue;
        };
        if class.to_ascii_lowercase().contains(token) {
            out.push(sel);
            kept_nodes.push((node.id, std::ptr::from_ref(node.tree) as usize));
        }
    }

    out
}

fn infer_split_body_signature_from_article(article: &Selection) -> Option<SplitBodySignature> {
    for signature in [
        SplitBodySignature::ArticleBody,
        SplitBodySignature::BodyContainer,
        SplitBodySignature::EntryContent,
        SplitBodySignature::StoryBody,
    ] {
        let candidates = find_split_body_candidates(article, signature);
        if candidates.len() >= 2 {
            return Some(signature);
        }
    }
    None
}

fn is_viable_split_body_chunk(chunk: &Selection) -> bool {
    if let Some(class) = chunk.attr("class") {
        let class = class.to_ascii_lowercase();
        if class.contains("truncation") || class.contains("truncate") {
            return false;
        }
    }

    let p_count = chunk.select("p").length();
    let text_len = dom::text_content(chunk).trim().len();

    if p_count >= 1 {
        return true;
    }
    if text_len >= 200 {
        return true;
    }
    false
}

fn maybe_merge_split_article_bodies(
    content_node: &Selection,
    options: &Options,
    baseline_text: &str,
    baseline_html: &str,
    use_relaxed_filtering: bool,
) -> Option<(String, String)> {
    let baseline_len = baseline_text.trim().len();
    if baseline_len >= 5000 {
        return None;
    }

    let article = find_nearest_article_ancestor(content_node)?;

    let signature = split_body_signature_for_node(content_node)
        .or_else(|| infer_split_body_signature_from_article(&article))?;

    // Entry-content wrappers are common on many sites and often nest other wrappers.
    // Only allow merging them when we already had to fall back to relaxed filtering
    // (a strong signal of under-extraction).
    if signature == SplitBodySignature::EntryContent && !use_relaxed_filtering {
        return None;
    }

    let candidates = find_split_body_candidates(&article, signature);
    if candidates.len() < 2 {
        return None;
    }

    let mut merged_text_parts: Vec<String> = Vec::new();
    let mut merged_html_parts: Vec<String> = Vec::new();

    for chunk in candidates {

        if !is_viable_split_body_chunk(&chunk) {
            continue;
        }

        let part_text = if use_relaxed_filtering {
            extract_filtered_text_allow_boilerplate(&chunk, options)
        } else {
            extract_filtered_text(&chunk, options)
        };
        if part_text.trim().is_empty() {
            continue;
        }
        merged_text_parts.push(part_text);

        let part_html = if use_relaxed_filtering {
            extract_filtered_html_allow_boilerplate(&chunk, options)
        } else {
            extract_filtered_html(&chunk, options)
        };
        if !part_html.trim().is_empty() {
            merged_html_parts.push(part_html);
        }
    }

    if merged_text_parts.len() < 2 {
        return None;
    }

    let merged_text = merged_text_parts.join("\n\n");
    let merged_len = merged_text.trim().len();

    if merged_len <= baseline_len + (baseline_len / 5) {
        return None;
    }

    if merged_len > baseline_len.saturating_mul(4) {
        return None;
    }

    if merged_len > 20000 {
        return None;
    }

    let merged_html = if merged_html_parts.is_empty() {
        baseline_html.to_string()
    } else {
        merged_html_parts.join("\n")
    };

    if merged_text.len() > options.max_extracted_len {
        return None;
    }

    Some((merged_text, merged_html))
}

/// Normalizes a language code to its primary component.
///
/// Converts `en-US` to `en`, `zh_TW` to `zh`, etc.
fn normalize_language(lang: &str) -> String {
    let lang = lang.trim();
    lang.split('-')
        .next()
        .unwrap_or(lang)
        .split('_')
        .next()
        .unwrap_or(lang)
        .to_lowercase()
}

/// Extracts the document's primary language.
fn extract_document_language(doc: &Document) -> Option<String> {
    // Check <html lang="...">
    if let Some(node) = doc.select("html").nodes().first() {
        let html = Selection::from(*node);
        if let Some(lang) = dom::get_attribute(&html, "lang") {
            return Some(normalize_language(&lang));
        }
    }

    // Check <meta http-equiv="content-language">
    for node in doc.select("meta[http-equiv]").nodes() {
        let meta = Selection::from(*node);
        if let Some(equiv) = dom::get_attribute(&meta, "http-equiv") {
            if equiv.eq_ignore_ascii_case("content-language") {
                if let Some(content) = dom::get_attribute(&meta, "content") {
                    let text = clean_text(&content);
                    if !text.is_empty() {
                        return Some(normalize_language(&text));
                    }
                }
            }
        }
    }

    // Check <meta name="language">
    for node in doc.select("meta[name]").nodes() {
        let meta = Selection::from(*node);
        if let Some(name) = dom::get_attribute(&meta, "name") {
            if name.eq_ignore_ascii_case("language") {
                if let Some(content) = dom::get_attribute(&meta, "content") {
                    let text = clean_text(&content);
                    if !text.is_empty() {
                        return Some(normalize_language(&text));
                    }
                }
            }
        }
    }

    None
}

/// Checks if an element matches the target language.
///
/// Returns true if:
/// - No target language specified
/// - Element lang attribute matches target
/// - Document language matches target (when element has no lang)
/// - Language cannot be detected (graceful degradation)
fn matches_target_language(doc: &Document, el: &Selection, target_lang: Option<&String>) -> bool {
    let Some(target) = target_lang else {
        // No target language specified - accept all
        return true;
    };

    let normalized_target = normalize_language(target);

    // Check element's lang attribute
    if let Some(el_lang) = dom::get_attribute(el, "lang") {
        let normalized_el_lang = normalize_language(&el_lang);
        return normalized_el_lang == normalized_target;
    }

    // Check parent elements for lang attribute (up to 5 levels)
    // Note: dom_query doesn't have direct parent traversal like scraper,
    // so we check the document language as fallback

    // Fall back to document language
    if let Some(doc_lang) = extract_document_language(doc) {
        return doc_lang == normalized_target;
    }

    // Unknown language - don't filter (graceful degradation)
    true
}

/// Finds the main content node using semantic selectors.
#[allow(dead_code)]  // Used for backward compatibility
fn find_main_content_node(doc: &Document) -> Option<Selection<'_>> {
    find_main_content_node_with_options(doc, &Options::default())
}

/// Finds the main content node using semantic selectors with options.
fn find_main_content_node_with_options<'a>(doc: &'a Document, options: &Options) -> Option<Selection<'a>> {
    let body = doc.select("body");
    if body.length() == 0 {
        return None;
    }

    // Try sophisticated content selector rules first (handles entry-content, post-content, etc.)
    // These rules check for specific content markers in priority order
    if let Some(content) = selector::content::find_content(&body) {
        // Verify language match if filtering is active
        if options.target_language.is_none()
            || matches_target_language(doc, &content, options.target_language.as_ref())
        {
            return Some(content);
        }
    }

    // Fall back to simple article selector (for pages without specific content markers)
    let article_sel = doc.select(ARTICLE_SELECTOR);
    if article_sel.length() > 0 {
        // If language filtering is active, try to find matching article
        if options.target_language.is_some() {
            for node in article_sel.nodes() {
                let el = Selection::from(*node);
                if matches_target_language(doc, &el, options.target_language.as_ref()) {
                    return Some(el);
                }
            }
            // No language-matching article found, continue to other strategies
        } else {
            // No language filtering, use first article
            return Some(article_sel);
        }
    }

    // Try main content area
    let main_sel = doc.select(MAIN_SELECTOR);
    if main_sel.length() > 0 {
        if options.target_language.is_some() {
            for node in main_sel.nodes() {
                let el = Selection::from(*node);
                if matches_target_language(doc, &el, options.target_language.as_ref()) {
                    return Some(el);
                }
            }
        } else {
            return Some(main_sel);
        }
    }

    find_heuristic_content_node_with_options(doc, options)
}

#[allow(clippy::too_many_lines)]
#[allow(dead_code)]  // Used for backward compatibility
fn find_heuristic_content_node(doc: &Document) -> Option<Selection<'_>> {
    find_heuristic_content_node_with_options(doc, &Options::default())
}

#[allow(clippy::too_many_lines)]
fn find_heuristic_content_node_with_options<'a>(doc: &'a Document, options: &Options) -> Option<Selection<'a>> {
    let body = doc.select("body");
    if body.length() == 0 {
        return None;
    }

    let body_raw_text = dom::text_content(&body);
    let body_cleaned = clean_text(&body_raw_text);
    let body_text_len: i64 = match i64::try_from(body_cleaned.len()) {
        Ok(v) => v,
        Err(_) => i64::MAX,
    };
    // Don't use body as candidate when language filtering is active
    // (body contains all languages, would defeat filtering purpose)
    let allow_body_candidate = body_text_len > 0 && body_text_len <= 500 && options.target_language.is_none();

    let mut best_score: i64 = 0;
    let mut best: Option<Selection> = None;

    if allow_body_candidate {
        let score = score_content_node(&body, &body_cleaned, body_text_len, doc, 0);
        best_score = score;
        best = Some(body.clone());
    }

    // Iterate through all divs, sections, articles, and main elements
    for tag in ["div", "section", "article", "main"] {
        let elements = doc.select(tag);
        for node in elements.nodes() {
            let el = Selection::from(*node);

            if let Some(class) = el.attr("class") {
                if is_boilerplate(&class) {
                    continue;
                }
            }
            if let Some(id) = el.attr("id") {
                if is_boilerplate(&id) {
                    continue;
                }
            }

            // Skip content that doesn't match target language
            if !matches_target_language(doc, &el, options.target_language.as_ref()) {
                continue;
            }

            let raw_text = dom::text_content(&el);
            let cleaned = clean_text(&raw_text);
            let text_len: i64 = match i64::try_from(cleaned.len()) {
                Ok(v) => v,
                Err(_) => i64::MAX,
            };
            if text_len == 0 {
                continue;
            }

            // Calculate depth by counting parent elements
            let mut depth: i64 = 0;
            let mut current = el.parent();
            while current.length() > 0 {
                if let Some(tag_name) = dom::tag_name(&current) {
                    if tag_name == "body" {
                        break;
                    }
                }
                depth = depth.saturating_add(1);
                current = current.parent();
            }

            let score = score_content_node(&el, &cleaned, text_len, doc, depth);
            if score > best_score {
                best_score = score;
                best = Some(el);
            }
        }
    }

    // Apply score threshold based on precision/recall mode
    // Note: If both favor_precision and favor_recall are true,
    // precision takes precedence (stricter threshold wins)
    let min_score = if options.favor_precision {
        5000  // Higher threshold for precision mode
    } else if options.favor_recall {
        500   // Lower threshold for recall mode
    } else {
        1000  // Default threshold
    };

    if best_score >= min_score {
        // Coverage check: if the best element covers less than 30% of body text,
        // it's likely a sibling among many (like tutorialblock divs in documentation).
        // In that case, reject it so body extraction can be used instead.
        if let Some(ref best_sel) = best {
            let best_text = dom::text_content(best_sel);
            let best_len = clean_text(&best_text).len();
            let coverage = if body_text_len > 0 {
                (best_len as f64) / (body_text_len as f64)
            } else {
                1.0
            };
            // If coverage is very low, this is probably not the main content
            if coverage < 0.3 {
                return None;
            }
        }
        best
    } else {
        None
    }
}

/// Scores a content node based on text density, structure, and quality signals.
#[allow(clippy::cast_precision_loss)]
fn score_content_node(
    el: &Selection,
    cleaned_text: &str,
    text_len: i64,
    _doc: &Document,
    depth: i64,
) -> i64 {
    let sentence_count = count_sentences(cleaned_text);

    // Count elements WITHIN the candidate element, not globally
    // This ensures each candidate is scored based on its own structure
    let mut substantive_p_count: i64 = 0;
    let p_elements = el.select("p");
    for node in p_elements.nodes() {
        let p = Selection::from(*node);
        let p_text = dom::text_content(&p);
        let p_clean = clean_text(&p_text);
        let p_len: i64 = match i64::try_from(p_clean.len()) {
            Ok(v) => v,
            Err(_) => i64::MAX,
        };
        if p_len >= 100 {
            substantive_p_count = substantive_p_count.saturating_add(1);
        }
    }

    let p_count: i64 = match i64::try_from(el.select("p").length()) {
        Ok(v) => v,
        Err(_) => i64::MAX,
    };
    let a_count: i64 = match i64::try_from(el.select("a").length()) {
        Ok(v) => v,
        Err(_) => i64::MAX,
    };
    let h_count: i64 = match i64::try_from(el.select("h1, h2, h3, h4, h5, h6").length()) {
        Ok(v) => v,
        Err(_) => i64::MAX,
    };

    let mut link_text_len: i64 = 0;
    let a_elements = el.select("a");
    for node in a_elements.nodes() {
        let a = Selection::from(*node);
        let a_text = dom::text_content(&a);
        let a_clean = clean_text(&a_text);
        let a_len: i64 = match i64::try_from(a_clean.len()) {
            Ok(v) => v,
            Err(_) => i64::MAX,
        };
        link_text_len = link_text_len.saturating_add(a_len);
    }

    let link_density = if text_len > 0 {
        (link_text_len as f64) / (text_len as f64)
    } else {
        1.0
    };

    let effective_text_len = text_len.min(8000);
    let max_counted_sentences = effective_text_len / 50;
    let effective_sentence_count = sentence_count.min(max_counted_sentences);

    let mut score = effective_text_len;
    score = score.saturating_add(p_count.saturating_mul(200));
    score = score.saturating_add(h_count.saturating_mul(100));
    score = score.saturating_add(substantive_p_count.saturating_mul(300));
    score = score.saturating_add(effective_sentence_count.saturating_mul(50));
    score = score.saturating_sub(a_count.saturating_mul(50));
    score = score.saturating_add(depth.saturating_mul(10));

    if link_density > 0.5 {
        score /= 2;
    }

    score
}

fn count_sentences(text: &str) -> i64 {
    let mut count: i64 = 0;
    let mut prev_term = false;

    for ch in text.chars() {
        let is_term = matches!(ch, '.' | '!' | '?');
        if is_term && !prev_term {
            count = count.saturating_add(1);
        }
        prev_term = is_term;
    }

    count
}

/// Extracts content from body with boilerplate filtering.
fn extract_body_content(doc: &Document, options: &Options) -> Result<String> {
    let body = doc.select("body");
    if body.length() == 0 {
        return Err(Error::NoContent);
    }
    Ok(extract_filtered_text(&body, options))
}

fn extract_body_content_html(doc: &Document, options: &Options) -> Result<String> {
    let body = doc.select("body");
    if body.length() == 0 {
        return Err(Error::NoContent);
    }
    Ok(extract_filtered_html(&body, options))
}

fn extract_filtered_text(root: &Selection, options: &Options) -> String {
    extract_filtered_text_inner(root, options, true, None)
}

fn extract_filtered_text_with_title(root: &Selection, options: &Options, page_title: Option<&str>) -> String {
    extract_filtered_text_inner(root, options, true, page_title)
}

fn extract_filtered_text_allow_boilerplate(root: &Selection, options: &Options) -> String {
    extract_filtered_text_inner(root, options, false, None)
}

// === EPIC-06: Hot Path Optimization Helper Functions ===

/// Check if a Tendril tag name matches any of the given targets (case-insensitive).
/// Use this for pre-extracted tag names.
#[inline]
fn tendril_tag_matches(tag_name: &tendril::StrTendril, targets: &[&str]) -> bool {
    targets.iter().any(|t| tag_name.eq_ignore_ascii_case(t))
}

/// Build a static slice of excluded tag names for fast lookup.
/// Using a slice is faster than HashSet for small, fixed tag lists.
#[inline]
fn excluded_tag_names() -> &'static [&'static str] {
    &["script", "style", "noscript", "nav", "aside", "iframe", "svg", "ins"]
}

#[allow(clippy::too_many_lines)]
fn extract_filtered_text_inner(
    root: &Selection,
    options: &Options,
    filter_named_boilerplate: bool,
    page_title: Option<&str>,
) -> String {
    let mut out = String::new();
    let mut skip_depths: Vec<usize> = Vec::new();

    // Get the root node for traversal
    let Some(root_node) = root.nodes().first() else {
        return String::new();
    };

    // EPIC-06: Pre-build excluded tag set for fast lookup
    let excluded_tags = excluded_tag_names();

    for node in root_node.descendants() {
        if node.is_text() {
            if let Some(parent) = node.parent() {
                if parent.is_element() {
                    // EPIC-06: Zero-allocation tag check using eq_ignore_ascii_case
                    if let Some(tag) = parent.node_name() {
                        if tag.eq_ignore_ascii_case("script")
                            || tag.eq_ignore_ascii_case("style")
                            || tag.eq_ignore_ascii_case("noscript")
                        {
                            continue;
                        }
                    }
                }
            }
        }

        // Count ancestors manually since dom_query's ancestors() API is different
        let mut depth = 0;
        let mut current = node.parent();
        while let Some(parent) = current {
            depth += 1;
            current = parent.parent();
        }
        while let Some(top) = skip_depths.last() {
            if depth <= *top {
                skip_depths.pop();
            } else {
                break;
            }
        }
        if let Some(top) = skip_depths.last() {
            if depth > *top {
                continue;
            }
        }

        let mut excluded = false;
        let mut anc_opt = Some(node);
        while let Some(anc) = anc_opt {
            // Stop ancestor checking at the content root element.
            // This prevents false positives where a wrapper element inside the content area
            // has a boilerplate-looking class (e.g., "share-container" wrapping main content).
            // We only want to check for boilerplate WITHIN the selected content subtree,
            // not the ancestors that were used to find the content.
            if anc.id == root_node.id {
                break;
            }

            if anc.is_element() {
                // EPIC-06: Zero-allocation tag name check using eq_ignore_ascii_case
                if let Some(tag_name) = anc.node_name() {
                    // Check for header tag - must look UP from header to find article/main
                    // NOTE: Cannot use single-pass optimization here because we need to check
                    // if header is INSIDE article/main, but we traverse bottom-up
                    if tag_name.eq_ignore_ascii_case("header") {
                        // Look UP from header to find article/main
                        let mut found_article_or_main = false;
                        let mut cur = anc.parent();
                        while let Some(parent) = cur {
                            if parent.id == root_node.id {
                                break;
                            }
                            if let Some(pname) = parent.node_name() {
                                if pname.eq_ignore_ascii_case("article") || pname.eq_ignore_ascii_case("main") {
                                    found_article_or_main = true;
                                    break;
                                }
                            }
                            cur = parent.parent();
                        }
                        if !found_article_or_main {
                            excluded = true;
                            break;
                        }
                    }

                    // Check for footer tag
                    // Must also look UP from footer to find article/main
                    if tag_name.eq_ignore_ascii_case("footer") {
                        // Always exclude footer if it has boilerplate classes
                        let sel = Selection::from(anc);
                        let has_boilerplate_class = sel
                            .attr("class")
                            .is_some_and(|c| is_boilerplate(&c));

                        if has_boilerplate_class {
                            excluded = true;
                            break;
                        }

                        // For footers without boilerplate classes, look UP to find article/main
                        let mut found_article_or_main = false;
                        let mut cur = anc.parent();
                        while let Some(parent) = cur {
                            if parent.id == root_node.id {
                                break;
                            }
                            if let Some(pname) = parent.node_name() {
                                if pname.eq_ignore_ascii_case("article") || pname.eq_ignore_ascii_case("main") {
                                    found_article_or_main = true;
                                    break;
                                }
                            }
                            cur = parent.parent();
                        }
                        if !found_article_or_main {
                            excluded = true;
                            break;
                        }
                    }

                    // Check for other excluded tags using linear search over small slice
                    // EPIC-06: Linear search over 8 items is faster than HashSet for Tendril
                    if tendril_tag_matches(&tag_name, excluded_tags) {
                        excluded = true;
                        break;
                    }
                }

                let sel = Selection::from(anc);
                if let Some(class) = sel.attr("class") {
                    if is_always_excluded_name(&class) {
                        excluded = true;
                        break;
                    }
                }
                if let Some(id) = sel.attr("id") {
                    if is_always_excluded_name(&id) {
                        excluded = true;
                        break;
                    }
                }

                if filter_named_boilerplate {
                    if let Some(class) = sel.attr("class") {
                        if is_boilerplate(&class) {
                            excluded = true;
                            break;
                        }
                    }
                    if let Some(id) = sel.attr("id") {
                        if is_boilerplate(&id) {
                            excluded = true;
                            break;
                        }
                    }
                }

                if let Some(itemtype) = sel.attr("itemtype") {
                    let itemtype_lower = itemtype.to_ascii_lowercase();
                    if itemtype_lower.contains("breadcrumblist") {
                        excluded = true;
                        break;
                    }
                }
            }

            anc_opt = anc.parent();
        }

        if excluded {
            continue;
        }

        // Handle elements - EPIC-06: Zero-allocation tag name check
        if node.is_element() {
            // EPIC-06: Extract tag name once to avoid duplicate calls
            let tag_name = node.node_name();
            let is_table = tag_name.as_ref().is_some_and(|t| t.eq_ignore_ascii_case("table"));
            let is_div_ul_ol = tag_name.as_ref().is_some_and(|t| {
                t.eq_ignore_ascii_case("div")
                    || t.eq_ignore_ascii_case("ul")
                    || t.eq_ignore_ascii_case("ol")
            });

            // Handle table elements based on include_tables option
            if is_table {
                let table = Selection::from(node);

                // Check link density for tables - skip if mostly links
                if link_density_test_tables(&table, options) {
                    skip_depths.push(depth);
                    continue;
                }

                if options.include_tables {
                    // Extract table content with special formatting
                    if !is_layout_table(&table) {
                        let table_text = extract_table_text(&table);
                        if !table_text.is_empty() {
                            out.push_str("\n\n");
                            out.push_str(&table_text);
                            out.push_str("\n\n");
                        }
                        skip_depths.push(depth);
                        continue;
                    }
                } else {
                    // Skip table and all its descendants when include_tables is false
                    skip_depths.push(depth);
                    continue;
                }
            }

            // Check link density for div and list elements - skip if mostly links (navigation containers)
            // Go equivalent: deleteByLinkDensity for div, ul, ol elements in pruneUnwantedSections
            if is_div_ul_ol {
                let element = Selection::from(node);
                if link_density_test(&element, options) {
                    skip_depths.push(depth);
                    continue;
                }
            }

            // Add line breaks for block-level elements
            // EPIC-06: Zero-allocation tag name checks
            if let Some(tag_name) = node.node_name() {
                // Check for heading elements with boilerplate text content
                let is_heading = tag_name.len() == 2
                    && tag_name.starts_with('h')
                    && tag_name.chars().nth(1).map_or(false, |c| c.is_ascii_digit());

                if is_heading {
                    // Get full text content of the heading to check for boilerplate
                    let heading_sel = Selection::from(node);
                    let heading_text = etree::iter_text(&heading_sel, " ");
                    let heading_text_trimmed = heading_text.trim();

                    // Skip headings with boilerplate patterns (newsletter CTAs, comments, social share)
                    if html_processing::is_share_button_text(heading_text_trimmed) {
                        skip_depths.push(depth);
                        continue;
                    }

                    // Skip headings with title/headline class markers (article title duplicated in body)
                    if let Some(class) = dom::get_attribute(&heading_sel, "class") {
                        let class_lower = class.to_ascii_lowercase();
                        if class_lower.contains("entry-title")
                            || class_lower.contains("post-title")
                            || class_lower.contains("article-title")
                            || class_lower.contains("story-title")
                            || class_lower.contains("pg-headline")
                            || class_lower.contains("headline")
                        {
                            skip_depths.push(depth);
                            continue;
                        }
                    }

                    // Skip headings with itemprop="headline" (schema.org article headline)
                    if let Some(itemprop) = dom::get_attribute(&heading_sel, "itemprop") {
                        if itemprop.to_ascii_lowercase() == "headline" {
                            skip_depths.push(depth);
                            continue;
                        }
                    }

                    // Skip h1 headings that match the page title (article headline duplicated in body)
                    // Only applies to h1 elements to avoid filtering legitimate section headings
                    if tag_name.eq_ignore_ascii_case("h1") {
                        if let Some(title) = page_title {
                            if titles_match(heading_text_trimmed, title) {
                                skip_depths.push(depth);
                                continue;
                            }
                        }
                    }
                }

                // Filter paragraphs that consist entirely of boilerplate text
                // (e.g., standalone "comments", "X comments", social share buttons)
                if tag_name.eq_ignore_ascii_case("p") {
                    let p_sel = Selection::from(node);
                    let p_text = etree::iter_text(&p_sel, " ");
                    let p_text_trimmed = p_text.trim();

                    // Only filter if paragraph is short and matches boilerplate patterns
                    // This prevents filtering legitimate content that mentions these words
                    if p_text_trimmed.len() < 50 && html_processing::is_share_button_text(p_text_trimmed) {
                        skip_depths.push(depth);
                        continue;
                    }
                }

                // Filter divs that consist entirely of boilerplate text (bylines, timestamps, etc.)
                // More restrictive than paragraphs - only filter very short divs
                if tag_name.eq_ignore_ascii_case("div") {
                    let div_sel = Selection::from(node);
                    let div_text = etree::iter_text(&div_sel, " ");
                    let div_text_trimmed = div_text.trim();

                    // Only filter divs with very short text that matches byline/metadata patterns
                    if div_text_trimmed.len() < 80 && html_processing::is_share_button_text(div_text_trimmed) {
                        skip_depths.push(depth);
                        continue;
                    }
                }

                if tag_name.eq_ignore_ascii_case("p")
                    || tag_name.eq_ignore_ascii_case("div")
                    || tag_name.eq_ignore_ascii_case("section")
                    || tag_name.eq_ignore_ascii_case("article")
                    || is_heading
                {
                    out.push_str("\n\n");
                } else if tag_name.eq_ignore_ascii_case("br") || tag_name.eq_ignore_ascii_case("li") {
                    out.push('\n');
                }
            }
        }

        if node.is_text() {
            let text = node.text();
            out.push_str(&text);
            out.push(' ');
        }
    }

    normalize_text_output(&out)
}

fn extract_filtered_html(root: &Selection, options: &Options) -> String {
    extract_filtered_html_inner(root, options, true)
}

fn extract_filtered_html_allow_boilerplate(root: &Selection, options: &Options) -> String {
    extract_filtered_html_inner(root, options, false)
}

fn extract_filtered_html_inner(
    root: &Selection,
    options: &Options,
    filter_named_boilerplate: bool,
) -> String {
    let mut out = String::new();
    let tag = dom::tag_name(root).unwrap_or_default().to_ascii_lowercase();
    let inside_article_or_main = matches!(tag.as_str(), "article" | "main");
    push_filtered_html_children(
        root,
        &mut out,
        inside_article_or_main,
        false,
        options,
        filter_named_boilerplate,
    );
    out.trim().to_string()
}

#[allow(clippy::too_many_lines)]
fn push_filtered_html_children(
    root: &Selection,
    out: &mut String,
    inside_article_or_main: bool,
    inside_layout_table: bool,
    options: &Options,
    filter_named_boilerplate: bool,
) {
    let Some(root_node) = root.nodes().first() else {
        return;
    };

    for child_node in root_node.children() {
        if child_node.is_element() {
            let el = Selection::from(child_node);
            let tag = dom::tag_name(&el).unwrap_or_default().to_ascii_lowercase();

            if tag == "header" && !inside_article_or_main {
                continue;
            }
            if tag == "footer" && !inside_article_or_main {
                continue;
            }
            if matches!(tag.as_str(), "nav" | "aside" | "script" | "style" | "noscript" | "iframe" | "svg" | "ins") {
                continue;
            }

            if let Some(class) = el.attr("class") {
                if is_always_excluded_name(&class) {
                    continue;
                }
            }
            if let Some(id) = el.attr("id") {
                if is_always_excluded_name(&id) {
                    continue;
                }
            }

            if filter_named_boilerplate {
                if let Some(class) = el.attr("class") {
                    if is_boilerplate(&class) {
                        continue;
                    }
                }
                if let Some(id) = el.attr("id") {
                    if is_boilerplate(&id) {
                        continue;
                    }
                }
            }
            if let Some(itemtype) = el.attr("itemtype") {
                let itemtype_lower = itemtype.to_ascii_lowercase();
                if itemtype_lower.contains("breadcrumblist") {
                    continue;
                }
            }

            let next_inside_article_or_main = inside_article_or_main || matches!(tag.as_str(), "article" | "main");

            if inside_layout_table
                && matches!(
                    tag.as_str(),
                    "table"
                        | "thead"
                        | "tbody"
                        | "tfoot"
                        | "tr"
                        | "td"
                        | "th"
                        | "caption"
                        | "colgroup"
                        | "col"
                )
            {
                push_filtered_html_children(
                    &el,
                    out,
                    next_inside_article_or_main,
                    true,
                    options,
                    filter_named_boilerplate,
                );
                continue;
            }

            if tag == "table" && (!options.include_tables || is_layout_table(&el)) {
                push_filtered_html_children(
                    &el,
                    out,
                    next_inside_article_or_main,
                    true,
                    options,
                    filter_named_boilerplate,
                );
                continue;
            }

            if matches!(
                tag.as_str(),
                "p"
                    | "div"
                    | "section"
                    | "article"
                    | "main"
                    | "h1"
                    | "h2"
                    | "h3"
                    | "h4"
                    | "h5"
                    | "h6"
                    | "blockquote"
                    | "strong"
                    | "em"
                    | "b"
                    | "i"
                    | "a"
                    | "ul"
                    | "ol"
                    | "li"
                    | "dl"
                    | "dt"
                    | "dd"
                    | "table"
                    | "thead"
                    | "tbody"
                    | "tfoot"
                    | "tr"
                    | "td"
                    | "th"
                    | "caption"
                    | "colgroup"
                    | "col"
            ) {
                out.push('<');
                out.push_str(&tag);
                if tag == "a" && options.include_links {
                    if let Some(href) = el.attr("href") {
                        out.push_str(" href=\"");
                        out.push_str(&escape_html(&href));
                        out.push('"');
                    }
                }
                if matches!(tag.as_str(), "td" | "th") {
                    if let Some(colspan) = el.attr("colspan") {
                        out.push_str(" colspan=\"");
                        out.push_str(&escape_html(&colspan));
                        out.push('"');
                    }
                    if let Some(rowspan) = el.attr("rowspan") {
                        out.push_str(" rowspan=\"");
                        out.push_str(&escape_html(&rowspan));
                        out.push('"');
                    }
                }
                out.push('>');

                push_filtered_html_children(
                    &el,
                    out,
                    next_inside_article_or_main,
                    inside_layout_table,
                    options,
                    filter_named_boilerplate,
                );

                out.push_str("</");
                out.push_str(&tag);
                out.push('>');
            } else if tag == "br" {
                out.push_str("<br>");
            } else {
                push_filtered_html_children(
                    &el,
                    out,
                    next_inside_article_or_main,
                    inside_layout_table,
                    options,
                    filter_named_boilerplate,
                );
            }
        } else if child_node.is_text() {
            let text = child_node.text();
            out.push_str(&escape_html(&text));
        }
    }
}

fn is_layout_table(table: &Selection) -> bool {
    if let Some(role) = table.attr("role") {
        if role.eq_ignore_ascii_case("presentation") {
            return true;
        }
    }

    // Create temporary document from table HTML to select within it
    let table_html = dom::outer_html(table);
    let doc = Document::from(table_html);

    let tr_sel = doc.select("tr");
    let mut row_count: usize = 0;
    for _ in tr_sel.nodes() {
        row_count = row_count.saturating_add(1);
        if row_count > 1 {
            break;
        }
    }
    if row_count <= 1 {
        return true;
    }

    let cell_sel = doc.select("td, th");
    let mut cell_count: usize = 0;
    for _ in cell_sel.nodes() {
        cell_count = cell_count.saturating_add(1);
        if cell_count > 1 {
            break;
        }
    }
    if cell_count <= 1 {
        return true;
    }

    false
}

fn is_always_excluded_name(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.contains("av-structured-data")
        || name.contains("post-meta-infos")
        || name.contains("comment-container")
        || name.contains("comments-link")
        || name.contains("blog-categories")
        || name.contains("blog-author")
        || name.contains("wp-caption")
        || name.contains("wp-caption-text")
        || name.contains("video__end-slate")
        || name.contains("zn-large-media")
        || name.contains("featured-video-collection")
        || name.contains("el__featured-video")
        || name.contains("messenger-content")
        || name.contains("read-more-link")
        || name.contains("zn-body__read-more")
        || name.contains("js-body-read-more")
        || name.contains("pg-headline")
}

fn parse_usize_attr(value: Option<&str>, default: usize) -> usize {
    let Some(value) = value else {
        return default;
    };
    let Ok(parsed) = value.trim().parse::<usize>() else {
        return default;
    };
    if parsed == 0 {
        default
    } else {
        parsed
    }
}

const MAX_TABLE_CELLS: usize = 20_000;
const MAX_TABLE_TEXT_LEN: usize = 200_000;

fn push_rowspan_cells(
    rowspan: &mut [Option<(usize, String)>],
    row_cells: &mut Vec<String>,
    col: &mut usize,
) {
    while *col < rowspan.len() {
        let Some((remaining, val)) = rowspan[*col].take() else {
            break;
        };
        row_cells.push(val.clone());

        let next_remaining = remaining.saturating_sub(1);
        if next_remaining > 0 {
            rowspan[*col] = Some((next_remaining, val));
        }

        *col = col.saturating_add(1);
    }
}

fn extract_table_text(table: &Selection) -> String {
    let mut out = String::new();
    let mut rowspan: Vec<Option<(usize, String)>> = Vec::new();
    let mut total_cells: usize = 0;

    // Select rows directly from the table selection
    let tr_sel = table.select("tr");
    
    for tr_node in tr_sel.nodes() {
        if total_cells >= MAX_TABLE_CELLS || out.len() >= MAX_TABLE_TEXT_LEN {
            break;
        }

        let tr = Selection::from(*tr_node);
        
        let mut row_cells: Vec<String> = Vec::new();
        let mut col: usize = 0;

        // Select cells directly from the row selection
        let cell_sel = tr.select("td, th");
        for cell_node in cell_sel.nodes() {
            push_rowspan_cells(&mut rowspan, &mut row_cells, &mut col);

            let cell = Selection::from(*cell_node);
            let raw = dom::text_content(&cell);
            let text = clean_text(&raw);

            let colspan_attr = cell.attr("colspan");
            let rowspan_attr = cell.attr("rowspan");
            let colspan = parse_usize_attr(colspan_attr.as_deref(), 1);
            let rowspan_n = parse_usize_attr(rowspan_attr.as_deref(), 1);

            let need_len = col.saturating_add(colspan);
            if rowspan.len() < need_len {
                rowspan.resize_with(need_len, || None);
            }

            for i in 0..colspan {
                total_cells = total_cells.saturating_add(1);
                if total_cells >= MAX_TABLE_CELLS {
                    break;
                }
                row_cells.push(text.clone());
                if rowspan_n > 1 {
                    rowspan[col.saturating_add(i)] = Some((rowspan_n.saturating_sub(1), text.clone()));
                }
            }

            col = col.saturating_add(colspan);
            if total_cells >= MAX_TABLE_CELLS {
                break;
            }
        }

        push_rowspan_cells(&mut rowspan, &mut row_cells, &mut col);

        if row_cells.iter().all(|c| c.trim().is_empty()) {
            continue;
        }

        if !out.is_empty() {
            out.push('\n');
        }
        // Use space separator to match common ground truth format
        out.push_str(&row_cells.join(" "));

        if out.len() >= MAX_TABLE_TEXT_LEN {
            break;
        }
    }

    out
}

fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn normalize_text_output(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut pending_space = false;

    for ch in input.chars() {
        match ch {
            '\r' => {}
            '\n' => {
                if out.ends_with(' ') {
                    out.pop();
                }
                out.push('\n');
                pending_space = false;
            }
            '\t' | ' ' => {
                pending_space = true;
            }
            '.' | ',' | ';' | ':' | '!' | '?' => {
                if out.ends_with(' ') {
                    out.pop();
                }
                out.push(ch);
                pending_space = false;
            }
            _ => {
                if pending_space && !out.ends_with('\n') && !out.is_empty() {
                    out.push(' ');
                }
                out.push(ch);
                pending_space = false;
            }
        }
    }

    let out = LINE_WHITESPACE.replace_all(&out, "");
    let out = MULTIPLE_NEWLINES.replace_all(&out, "\n\n");
    out.trim().to_string()
}

/// Layout/component prefixes used in BEM-style / ITCSS-style CSS naming.
/// These indicate structural/styling concerns, not content type.
const LAYOUT_COMPONENT_PREFIXES: &[&str] = &["l-", "c-"];

/// Check if a token has a layout/component prefix (BEM-style).
fn has_layout_component_prefix(token: &str) -> bool {
    LAYOUT_COMPONENT_PREFIXES
        .iter()
        .any(|prefix| token.starts_with(prefix))
}

/// Check if a token is a false positive due to layout/component prefix.
/// Only exempts if the boilerplate match is *only* due to `sidebar` or `social`.
/// Example: `l-sidebar-fixed` is exempted, but `c-social-share` is NOT (because `share` still matches).
fn is_false_positive_layout_component_token(token: &str) -> bool {
    if !has_layout_component_prefix(token) {
        return false;
    }

    // Special case: sidebar with layout prefix (e.g., l-sidebar-fixed)
    // The sidebar-specific matching in is_boilerplate uses position-aware logic,
    // so we check if this sidebar would be exempted by that logic.
    if token.contains("sidebar") {
        let parts: Vec<&str> = token.split(['-', '_']).collect();
        for (i, part) in parts.iter().enumerate() {
            if *part == "sidebar" {
                // Would the position-aware sidebar logic match this as boilerplate?
                // It matches if: only part, first part, or preceded by position word
                let would_match_as_sidebar = parts.len() == 1
                    || i == 0
                    || (i > 0 && SIDEBAR_POSITION_WORDS.contains(&parts[i - 1]));
                if !would_match_as_sidebar {
                    // Sidebar wouldn't match due to position-aware logic
                    // Check if there are other BOILERPLATE_CLASS matches
                    let without_sidebar = token.replace("sidebar", "");
                    if !BOILERPLATE_CLASS.is_match(&without_sidebar) {
                        return true;
                    }
                }
            }
        }
    }

    // Only exempt if the boilerplate match is *only* due to social.
    let matches = BOILERPLATE_CLASS.is_match(token);
    if !matches {
        return false;
    }

    if token.contains("social") {
        let without_social = token.replace("social", "");
        if !BOILERPLATE_CLASS.is_match(&without_social) {
            return true;
        }
    }

    false
}

/// Check if a token is a false positive for navigation patterns.
/// Similar to boilerplate, but checks against NAVIGATION_CLASS patterns.
fn is_false_positive_navigation_token(token: &str) -> bool {
    if !has_layout_component_prefix(token) {
        return false;
    }

    // Only exempt if the navigation match is *only* due to sidebar.
    let matches = NAVIGATION_CLASS.is_match(token);
    if !matches {
        return false;
    }

    // Check if removing sidebar eliminates the match
    if token.contains("sidebar") {
        let without_sidebar = token.replace("sidebar", "");
        if !NAVIGATION_CLASS.is_match(&without_sidebar) {
            return true;
        }
    }

    false
}

/// Checks if a class or ID name indicates boilerplate content.
/// Handles layout/component prefixed tokens by exempting known false positives.
/// Position words that indicate an actual sidebar (not a theme namespace).
const SIDEBAR_POSITION_WORDS: &[&str] = &["left", "right", "primary", "secondary", "main", "widget"];

/// Suffixes that indicate an actual author box/bio section (not a taxonomy class like "author-john-doe").
const AUTHOR_BOX_SUFFIXES: &[&str] = &[
    "box", "bio", "info", "avatar", "meta", "wrap", "description", "link",
    "details", "card", "profile", "section", "container", "area", "block",
    "ul", "category", "pp", "ppma", "boxes",
];

fn is_boilerplate(name: &str) -> bool {
    // Check each space-separated token for navigation and boilerplate patterns
    for token in name.split_whitespace() {
        // Skip false positive layout/component tokens (e.g., l-sidebar-fixed)
        if is_false_positive_navigation_token(token) {
            continue;
        }

        // Check this token against navigation patterns
        if NAVIGATION_CLASS.is_match(token) {
            return true;
        }

        // Skip false positive layout/component tokens for boilerplate (e.g., c-social-buttons)
        if is_false_positive_layout_component_token(token) {
            continue;
        }

        // Check this token against boilerplate patterns
        if BOILERPLATE_CLASS.is_match(token) {
            return true;
        }

        // Sidebar-specific matching: avoid false positives like "newspaper-x-sidebar"
        // which is a theme namespace, not an actual sidebar element.
        // Only match sidebar when:
        // 1. Exact word: "sidebar"
        // 2. Starts with sidebar: "sidebar-left", "sidebar-container"
        // 3. Preceded by position word: "left-sidebar", "right-sidebar", "main-sidebar"
        let parts: Vec<&str> = token.split(['-', '_']).collect();
        for (i, part) in parts.iter().enumerate() {
            if *part == "sidebar" {
                // Match if it's the only part, the first part, or preceded by position word
                if parts.len() == 1 || i == 0 {
                    return true;
                }
                if i > 0 && SIDEBAR_POSITION_WORDS.contains(&parts[i - 1]) {
                    return true;
                }
                // Otherwise it's likely a namespace prefix (newspaper-x-sidebar) - skip
            }
        }

        // Author-specific matching: avoid false positives like "author-john-doe"
        // which is a WordPress taxonomy class (indicates who wrote the article),
        // not an author box/bio section.
        // Only match author when:
        // 1. Exact word: "author"
        // 2. Followed by box/bio/info suffixes: "author-box", "author-bio", etc.
        // 3. Preceded by known prefixes: "pp-author", "ppma-author", etc.
        for (i, part) in parts.iter().enumerate() {
            if *part == "author" {
                // Match if it's the only part (exact "author")
                if parts.len() == 1 {
                    return true;
                }
                // Check if followed by a known author box suffix
                if i + 1 < parts.len() {
                    let next_part = parts[i + 1];
                    if AUTHOR_BOX_SUFFIXES.contains(&next_part) {
                        return true;
                    }
                }
                // Check if preceded by a known author box prefix (pp, ppma)
                if i > 0 {
                    let prev_part = parts[i - 1];
                    if AUTHOR_BOX_SUFFIXES.contains(&prev_part) {
                        return true;
                    }
                }
                // Otherwise it's likely a taxonomy class (author-john-doe) - skip
            }
        }

        // Widget-specific matching: avoid false positives like "elementor-widget-text-editor"
        // which is an Elementor content container, not a sidebar widget.
        // Only match widget when NOT preceded by "elementor":
        // - Match: "widget", "widget-recent", "sidebar-widget"
        // - Skip: "elementor-widget-text-editor", "elementor-widget-container"
        for (i, part) in parts.iter().enumerate() {
            if *part == "widget" {
                // Skip if preceded by "elementor" (Elementor content widgets)
                if i > 0 && parts[i - 1] == "elementor" {
                    continue;
                }
                // Otherwise it's a regular widget (sidebar-widget, widget-recent, etc.)
                return true;
            }
        }
    }

    // Also check non-alphanumeric-split tokens for advertisement patterns
    // IMPORTANT: Only check the FIRST token to avoid false positives like
    // "body-ad-wrapper" where "ad" appears in the middle of a compound name
    // but doesn't indicate an advertisement. "ad-wrapper" or "ad-container"
    // should match, but "body-ad-wrapper" should not.
    let first_token = name.split(|c: char| !c.is_ascii_alphanumeric()).next();
    if let Some(token) = first_token {
        if !token.is_empty() && ADVERTISEMENT_CLASS.is_match(token) {
            return true;
        }
    }

    false
}

/// Extracts comments section from the document.
fn extract_comments(doc: &Document, options: &Options) -> (Option<String>, Option<String>) {
    let Some(node) = find_comment_section(doc) else {
        return (None, None);
    };

    let text = extract_filtered_text_allow_boilerplate(&node, options);
    if text.is_empty() {
        return (None, None);
    }

    let html = extract_filtered_html_allow_boilerplate(&node, options);
    let comments_html = if html.is_empty() { None } else { Some(html) };

    (Some(text), comments_html)
}

/// Extracts image data from content with hero detection.
///
/// # Arguments
/// * `doc` - The parsed HTML document
/// * `og_image` - The og:image URL from metadata (for hero detection)
fn extract_images(doc: &Document, og_image: Option<&str>) -> Vec<ImageData> {
    let mut images = Vec::new();
    let mut seen_urls = std::collections::HashSet::new();

    // Try to find images within content regions first
    if let Some(content_node) = find_main_content_node_with_options(doc, &Options::default()) {
        extract_images_from_node(&content_node, &mut images, &mut seen_urls);
    }

    // If no images found in content, try body
    if images.is_empty() {
        let body = doc.select("body");
        if body.length() > 0 {
            extract_images_from_node(&body, &mut images, &mut seen_urls);
        }
    }

    // Story 4: Hero image detection
    mark_hero_image(&mut images, og_image);

    images
}

/// Extracts image data from a specific node, including figcaptions.
fn extract_images_from_node(
    node: &Selection,
    images: &mut Vec<ImageData>,
    seen_urls: &mut std::collections::HashSet<String>,
) {
    // Create temporary document from node HTML to select within it
    let node_html = dom::outer_html(node);
    let doc = Document::from(node_html);

    // Story 3: First, process <figure> elements to get images with captions
    let figure_sel = doc.select("figure");
    for figure_node in figure_sel.nodes() {
        let figure = Selection::from(*figure_node);
        extract_image_from_figure(&figure, images, seen_urls);
    }

    // Then process standalone <img> elements (not inside figures)
    let img_sel = doc.select("img");
    for img_node in img_sel.nodes() {
        let img = Selection::from(*img_node);

        // Get src URL (try src first, then data-src for lazy loading)
        let src = img
            .attr("src")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                img.attr("data-src")
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            });

        let Some(src) = src else {
            continue;
        };

        // Skip duplicates (already processed in figures)
        if seen_urls.contains(&src) {
            continue;
        }
        seen_urls.insert(src.clone());

        // Extract alt text
        let alt = img
            .attr("alt")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        // Extract filename from URL
        let filename = extract_filename(&src);

        images.push(ImageData {
            src,
            filename,
            alt,
            caption: None, // No caption for standalone images
            is_hero: false, // Will be set by mark_hero_image
        });
    }
}

/// Story 3: Extracts image data from a <figure> element, including figcaption.
///
/// HTML pattern handled:
/// ```html
/// <figure>
///   <img src="image.jpg" alt="Description">
///   <figcaption>Caption text here</figcaption>
/// </figure>
/// ```
fn extract_image_from_figure(
    figure: &Selection,
    images: &mut Vec<ImageData>,
    seen_urls: &mut std::collections::HashSet<String>,
) {
    // Find img inside the figure
    let img_sel = figure.select("img");
    if img_sel.length() == 0 {
        return;
    }

    // Get the first image in the figure
    let Some(img_node) = img_sel.nodes().first() else {
        return;
    };
    let img = Selection::from(*img_node);

    // Get src URL
    let src = img
        .attr("src")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            img.attr("data-src")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        });

    let Some(src) = src else {
        return;
    };

    // Skip duplicates
    if seen_urls.contains(&src) {
        return;
    }
    seen_urls.insert(src.clone());

    // Extract alt text
    let alt = img
        .attr("alt")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    // Extract filename from URL
    let filename = extract_filename(&src);

    // Story 3: Extract caption from figcaption
    let caption = extract_figcaption(figure);

    images.push(ImageData {
        src,
        filename,
        alt,
        caption,
        is_hero: false, // Will be set by mark_hero_image
    });
}

/// Story 3: Extracts and cleans caption text from a figcaption element.
fn extract_figcaption(figure: &Selection) -> Option<String> {
    let figcaption_sel = figure.select("figcaption");
    if figcaption_sel.length() == 0 {
        return None;
    }

    // Get text content from figcaption
    let caption_text = figcaption_sel.text();
    let cleaned = clean_caption_text(&caption_text);

    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

/// Cleans and normalizes caption text.
fn clean_caption_text(text: &str) -> String {
    // Normalize whitespace: collapse multiple spaces/newlines to single space
    let cleaned: String = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    cleaned.trim().to_string()
}

/// Story 4: Marks the hero image in the image list.
///
/// Hero detection priority:
/// 1. Match filename against og:image URL
/// 2. Fallback: mark first content image as hero
fn mark_hero_image(images: &mut [ImageData], og_image: Option<&str>) {
    if images.is_empty() {
        return;
    }

    // Priority 1: Match against og:image using filename comparison
    if let Some(og_url) = og_image {
        for img in images.iter_mut() {
            if filenames_match(&img.src, og_url) {
                img.is_hero = true;
                return;
            }
        }

        // Also try exact URL match
        for img in images.iter_mut() {
            if img.src == og_url {
                img.is_hero = true;
                return;
            }
        }
    }

    // Priority 2: Fallback - mark first image as hero
    if let Some(first) = images.first_mut() {
        first.is_hero = true;
    }
}

fn find_comment_section(doc: &Document) -> Option<Selection<'_>> {
    for id in ["comments", "comment-section", "disqus_thread", "respond", "discussion"] {
        let sel = format!("#{id}");
        let elements = doc.select(&sel);
        if elements.length() > 0 {
            return Some(elements);
        }
    }

    for class in [
        "comments",
        "comment-list",
        "respond",
        "discussion",
        "disqus",
        "fb-comments",
    ] {
        let sel = format!(".{class}");
        let elements = doc.select(&sel);
        if elements.length() > 0 {
            return Some(elements);
        }
    }

    let body = doc.select("body");
    if body.length() == 0 {
        return None;
    }

    let body_node = body.nodes().first()?;

    let mut best: Option<Selection> = None;
    let mut best_len: usize = 0;

    for node in body_node.descendants() {
        if !node.is_element() {
            continue;
        }

        let el = Selection::from(node);

        let mut matches = false;
        if let Some(id) = el.attr("id") {
            if COMMENT_ID.is_match(&id) {
                matches = true;
            }
        }
        if !matches {
            if let Some(class) = el.attr("class") {
                if COMMENT_CLASS.is_match(&class) {
                    matches = true;
                }
            }
        }

        if !matches {
            continue;
        }

        let raw = dom::text_content(&el);
        let cleaned = clean_text(&raw);
        let len = cleaned.len();
        if len > best_len {
            best_len = len;
            best = Some(el);
        }
    }

    best
}

/// Cleans and normalizes extracted text for metadata fields.
///
/// This function collapses ALL whitespace (including newlines) to single spaces,
/// which is appropriate for single-line metadata like titles and authors.
/// For main content extraction that preserves paragraph structure, use
/// `extract_filtered_text()` and `normalize_text_output()` instead.
fn clean_text(s: &str) -> String {
    let s = s.trim();
    if s.is_empty() {
        return String::new();
    }

    // Normalize whitespace
    let s = WHITESPACE_NORMALIZE.replace_all(s, " ");

    // Normalize multiple newlines
    let s = MULTIPLE_NEWLINES.replace_all(&s, "\n\n");

    s.trim().to_string()
}

/// Check if an h1 heading matches the page title.
/// Handles common title patterns like "Article Title - Site Name" or "Article Title | Site".
fn titles_match(heading: &str, page_title: &str) -> bool {
    // Normalize both for comparison
    let h_norm = normalize_title(heading);
    let t_norm = normalize_title(page_title);

    if h_norm.is_empty() || t_norm.is_empty() {
        return false;
    }

    // Exact match
    if h_norm == t_norm {
        return true;
    }

    // Page title often has suffix like " - Site Name" or " | Site Name"
    // Check if heading matches the prefix of the page title
    let separators = [" - ", " | ", " – ", " — ", ": "];
    for sep in separators {
        if let Some(prefix) = t_norm.split(sep).next() {
            if !prefix.is_empty() && h_norm == normalize_title(prefix) {
                return true;
            }
        }
    }

    // Also check if title starts with heading (heading might be shortened version)
    if t_norm.starts_with(&h_norm) && t_norm.len() > h_norm.len() + 3 {
        // Check that the next char after heading is a separator-like char
        let remaining = &t_norm[h_norm.len()..];
        if remaining.starts_with(" -")
            || remaining.starts_with(" |")
            || remaining.starts_with(" –")
            || remaining.starts_with(" —")
        {
            return true;
        }
    }

    false
}

/// Normalize title for comparison: lowercase, collapse whitespace, remove punctuation edges
fn normalize_title(s: &str) -> String {
    s.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Fix 7: Strip navigation patterns from extraction boundaries.
///
/// Removes common navigation text patterns that appear at the start or end
/// of extracted content, such as "< back | forward >" or "Home | About | Contact".
///
/// Note: Currently disabled - testing showed marginal impact with edge case regressions.
/// Kept for potential future use.
#[allow(dead_code)]
fn strip_navigation_boundaries(text: &str) -> String {
    let mut result = text.to_string();

    // Patterns that indicate navigation at start (case-insensitive check)
    let start_nav_patterns = [
        "< back", "<back", "back |", "| forward", "forward >",
        "home |", "| home", "| about", "| contact", "| links",
        "skip to content", "skip to main", "jump to navigation",
    ];

    // Strip navigation from start
    let lower = result.to_lowercase();
    for pattern in &start_nav_patterns {
        if lower.starts_with(pattern) {
            // Find the end of the navigation line
            if let Some(newline_pos) = result.find('\n') {
                result = result[newline_pos..].trim_start().to_string();
            } else if let Some(dot_pos) = result.find(". ") {
                // Sometimes nav is on same line as content, separated by period
                result = result[dot_pos + 2..].to_string();
            }
            break;
        }
    }

    // Also check for navigation-like first line (multiple pipes/bars)
    if let Some(first_line_end) = result.find('\n') {
        let first_line = &result[..first_line_end];
        let pipe_count = first_line.matches('|').count();
        let gt_count = first_line.matches('>').count();
        let lt_count = first_line.matches('<').count();

        // If first line has 2+ pipes or multiple < >, it's likely navigation
        if pipe_count >= 2 || (gt_count >= 2 && lt_count >= 2) {
            result = result[first_line_end..].trim_start().to_string();
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn extract_returns_content_from_article_tag() {
        let html = r"
            <html>
            <head><title>Test</title></head>
            <body>
                <nav>Navigation</nav>
                <article>
                    <h1>Article Title</h1>
                    <p>This is the main content.</p>
                </article>
                <footer>Footer</footer>
            </body>
            </html>
        ";

        let result = extract_content(html, &Options::default());
        match result {
            Ok(result) => {
                assert!(result.content_text.contains("main content"));
                // The metadata module prefers H1 content ("Article Title") over
                // the generic <title> tag ("Test") when H1 is more descriptive
                assert_eq!(result.metadata.title, Some("Article Title".to_string()));
            }
            Err(err) => panic!("expected Ok(_), got Err({err:?})"),
        }
    }

    #[test]
    fn extract_returns_partial_result_for_empty_content() {
        let html = "<html><body></body></html>";
        let result = extract_content(html, &Options::default()).expect("should return Ok with warnings");
        assert!(result.content_text.is_empty());
        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].contains("Content extraction failed"));
    }

    #[test]
    fn extract_handles_malformed_html_unclosed_tags() {
        // Note: For minimal HTML fragments like this, extraction may not capture
        // all content because fallback logic is triggered due to insufficient
        // word count. This is expected behavior for the library which is designed
        // for full web pages, not tiny HTML fragments.
        let html = "<p>text<div>more";
        let result = extract_content(html, &Options::default());
        match result {
            Ok(result) => {
                // At minimum, we should extract some content without crashing
                assert!(result.content_text.contains("text"));
                // The "more" in the div may or may not be extracted depending
                // on fallback behavior - the important thing is no crash
            }
            Err(err) => panic!("expected Ok(_), got Err({err:?})"),
        }
    }

    #[test]
    fn extract_handles_malformed_html_invalid_nesting() {
        let html = "<p><div></p></div>";
        let result = extract_content(html, &Options::default());
        assert!(result.is_ok());
    }

    #[test]
    fn extract_handles_malformed_html_missing_closing_tags() {
        let html = "<html><body><article>content";
        let result = extract_content(html, &Options::default());
        match result {
            Ok(result) => assert!(result.content_text.contains("content")),
            Err(err) => panic!("expected Ok(_), got Err({err:?})"),
        }
    }

    #[test]
    fn extract_handles_malformed_html_broken_attributes() {
        let html = "<div class=\"test id=broken>";
        let result = extract_content(html, &Options::default());
        assert!(result.is_ok());
    }

    #[test]
    fn extract_returns_partial_result_for_empty_string_input() {
        let result = extract_content("", &Options::default()).expect("should return Ok with warnings");
        assert!(result.content_text.is_empty());
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn extract_returns_partial_result_for_whitespace_only_input() {
        let result = extract_content("   \n\t  ", &Options::default()).expect("should return Ok with warnings");
        assert!(result.content_text.is_empty());
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn extract_returns_partial_result_for_minimal_html() {
        let result = extract_content("<html></html>", &Options::default()).expect("should return Ok with warnings");
        assert!(result.content_text.is_empty());
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn extract_returns_partial_result_for_body_only_html() {
        let result = extract_content("<body></body>", &Options::default()).expect("should return Ok with warnings");
        assert!(result.content_text.is_empty());
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn extract_merges_split_article_body_chunks_conservatively() {
        let html = r#"
            <html><body>
                <article>
                    <div class=\"body body__container article__body\">
                        <p>First paragraph.</p>
                        <p>Second paragraph.</p>
                    </div>
                    <aside class=\"ad\">Buy now</aside>
                    <div class=\"body body__container article__body\">
                        <p>Third paragraph.</p>
                        <p>Fourth paragraph.</p>
                    </div>
                </article>
            </body></html>
        "#;

        let result = extract_content(html, &Options::default()).expect("should extract");
        assert!(result.content_text.contains("First paragraph"));
        assert!(result.content_text.contains("Fourth paragraph"));
        assert!(!result.content_text.contains("Buy now"));
    }

    #[test]
    fn extract_handles_large_html_without_panic() {
        let target_size = 10 * 1024 * 1024 + 1;
        let chunk = "<p>Some repeated content for stress testing.</p>";
        let mut html = String::with_capacity(target_size + 128);
        html.push_str("<html><body><article>");
        while html.len() < target_size {
            html.push_str(chunk);
        }
        html.push_str("</article></body></html>");

        let start = Instant::now();
        let result = extract_content(&html, &Options::default());
        let elapsed = start.elapsed();

        assert!(result.is_ok());
        assert!(elapsed < Duration::from_secs(30), "large HTML parsing took {elapsed:?}");
    }

    #[test]
    fn extract_handles_malformed_html_incomplete_entities() {
        let html = "&amp text &lt;";
        let result = extract_content(html, &Options::default()).expect("should return Ok");
        // Content may be extracted or empty with warnings - both are acceptable
        assert!(result.content_text.contains("text") || result.content_text.is_empty());
    }

    #[test]
    fn clean_text_normalizes_whitespace() {
        assert_eq!(clean_text("  hello   world  "), "hello world");
        assert_eq!(clean_text("\n\n\n\ntest\n\n\n\n"), "test");
    }

    #[test]
    fn is_boilerplate_detects_navigation() {
        assert!(is_boilerplate("main-nav"));
        assert!(is_boilerplate("sidebar-menu"));
        assert!(!is_boilerplate("article-content"));
    }

    // Story 7-1: BEM-aware boilerplate detection tests

    #[test]
    fn test_bem_layout_prefix_not_boilerplate() {
        // Layout prefixed tokens with sidebar/social should NOT be detected as boilerplate
        assert!(!is_boilerplate("l-sidebar-fixed"));
        assert!(!is_boilerplate("l-sidebar l-segment"));
        assert!(!is_boilerplate("l-sidebar-fixed l-article-body-segment"));
    }

    #[test]
    fn test_bem_component_prefix_not_boilerplate() {
        // Component prefixed tokens with social should NOT be detected as boilerplate
        assert!(!is_boilerplate("c-social-buttons"));
        // But c-social-share SHOULD be detected (share still matches after removing social)
        assert!(is_boilerplate("c-social-share"));
    }

    #[test]
    fn test_mixed_bem_and_boilerplate() {
        // If one token is BEM layout and another is actual boilerplate, should detect
        assert!(is_boilerplate("l-sidebar footer"));
        assert!(is_boilerplate("c-widget sidebar"));
    }

    #[test]
    fn test_actual_boilerplate_still_detected() {
        // Non-prefixed boilerplate should still be detected
        assert!(is_boilerplate("sidebar"));
        assert!(is_boilerplate("sidebar-widget"));
        assert!(is_boilerplate("social-share"));
        assert!(is_boilerplate("footer-links"));
        // Prefixed but with other boilerplate patterns should still be detected
        assert!(is_boilerplate("c-newsletter")); // 'newsletter' is in BOILERPLATE_CLASS
        assert!(is_boilerplate("c-related-articles")); // 'related' is in BOILERPLATE_CLASS
        assert!(is_boilerplate("l-footer")); // 'footer' is in NAVIGATION_CLASS
        assert!(is_boilerplate("c-comment-section")); // 'comment' is in BOILERPLATE_CLASS
    }

    #[test]
    fn test_false_positive_helper() {
        // Direct tests for is_false_positive_layout_component_token
        assert!(is_false_positive_layout_component_token("l-sidebar-fixed"));
        assert!(is_false_positive_layout_component_token("c-social-buttons"));
        assert!(!is_false_positive_layout_component_token("c-social-share")); // share still matches
        assert!(!is_false_positive_layout_component_token("sidebar")); // no prefix
        assert!(!is_false_positive_layout_component_token("c-related")); // related matches boilerplate
    }

    #[test]
    fn test_count_words_filters_by_min_length() {
        // All words count with min_length=1
        assert_eq!(count_words("one two three four five", 1), 5);

        // Only words >= 3 chars: "one", "two", "three", "four", "five" (all pass)
        assert_eq!(count_words("one two three four five", 3), 5);

        // Only words >= 4 chars: "three", "four", "five"
        assert_eq!(count_words("one two three four five", 4), 3);

        // Only words >= 5 chars: "three"
        assert_eq!(count_words("one two three four five", 5), 1);

        // Empty string
        assert_eq!(count_words("", 1), 0);

        // Whitespace only
        assert_eq!(count_words("   \n\t  ", 1), 0);

        // Single word
        assert_eq!(count_words("hello", 1), 1);
        assert_eq!(count_words("hello", 10), 0);
    }

    // Story 6-2: Integration tests for final validations

    #[test]
    fn test_content_length_validation_min_extracted_len() {
        let html = r"<html><body><article><p>Short</p></article></body></html>";
        let options = Options {
            min_extracted_len: 1000, // Require at least 1000 chars
            ..Options::default()
        };

        match extract_content(html, &options) {
            Ok(result) => {
                // Should have warning about insufficient content
                assert!(result.warnings.iter().any(|w| w.contains("Insufficient content")));
                assert!(result.warnings.iter().any(|w| w.contains("chars")));
            }
            Err(err) => panic!("expected Ok(_), got Err({err:?})"),
        }
    }

    #[test]
    fn test_content_truncation_max_extracted_len() {
        // Create content with >500 chars
        let long_text = "word ".repeat(200); // 1000 chars
        let html = format!(r"<html><body><article><p>{long_text}</p></article></body></html>");

        let options = Options {
            max_extracted_len: 500, // Truncate at 500 chars
            ..Options::default()
        };

        match extract_content(&html, &options) {
            Ok(result) => {
                // Content should be truncated
                assert!(result.content_text.len() <= 500);

                // Should have warning about truncation
                assert!(result.warnings.iter().any(|w| w.contains("truncated")));
            }
            Err(err) => panic!("expected Ok(_), got Err({err:?})"),
        }
    }

    #[test]
    fn test_word_count_validation_min_output_size() {
        // Create content with few words
        let html = r"<html><body><article><p>One two three</p></article></body></html>";

        let options = Options {
            min_output_size: 100, // Require at least 100 words
            ..Options::default()
        };

        match extract_content(html, &options) {
            Ok(result) => {
                // Should have warning about insufficient content
                assert!(result.warnings.iter().any(|w| w.contains("Insufficient content")));
                assert!(result.warnings.iter().any(|w| w.contains("words")));
            }
            Err(err) => panic!("expected Ok(_), got Err({err:?})"),
        }
    }

    #[test]
    fn test_comments_validation_min_output_comm_size() {
        let html = r#"
            <html><body>
                <article><p>Main content with enough words to pass validation checks here.</p></article>
                <div class="comments"><p>Short comment</p></div>
            </body></html>
        "#;

        let options = Options {
            include_comments: true,
            min_output_comm_size: 50, // Require at least 50 words in comments
            min_output_size: 5,        // Low threshold for main content
            min_extracted_len: 10,     // Low threshold for main content
            ..Options::default()
        };

        match extract_content(html, &options) {
            Ok(result) => {
                // Comments should be removed due to insufficient word count
                assert!(result.comments_text.is_none());
                assert!(result.comments_html.is_none());

                // Should have warning about comments removal
                assert!(result.warnings.iter().any(|w| w.contains("Comments section removed")));
            }
            Err(err) => panic!("expected Ok(_), got Err({err:?})"),
        }
    }

    #[test]
    fn test_warning_generation_insufficient_content() {
        let html = r"<html><body><article><p>Too short</p></article></body></html>";

        let options = Options {
            min_output_size: 100,
            min_extracted_len: 500,
            ..Options::default()
        };

        match extract_content(html, &options) {
            Ok(result) => {
                // Should have specific warning with thresholds
                match result.warnings.iter().find(|w| w.contains("Insufficient content")) {
                    Some(warning) => {
                        assert!(warning.contains("words"));
                        assert!(warning.contains("chars"));
                        assert!(warning.contains("min:"));
                    }
                    None => panic!("expected insufficient content warning"),
                }
            }
            Err(err) => panic!("expected Ok(_), got Err({err:?})"),
        }
    }

    #[test]
    fn test_warning_generation_truncated_content() {
        let long_text = "word ".repeat(300);
        let html = format!(r"<html><body><article><p>{long_text}</p></article></body></html>");

        let options = Options {
            max_extracted_len: 800,
            min_output_size: 5, // Low to avoid insufficient content warning
            ..Options::default()
        };

        match extract_content(&html, &options) {
            Ok(result) => {
                // Should have truncation warning with max length
                match result.warnings.iter().find(|w| w.contains("truncated")) {
                    Some(warning) => {
                        assert!(warning.contains("800"));
                    }
                    None => panic!("expected truncation warning"),
                }
            }
            Err(err) => panic!("expected Ok(_), got Err({err:?})"),
        }
    }

    #[test]
    fn test_warning_generation_removed_comments() {
        let html = r#"
            <html><body>
                <article><p>Main content with sufficient words for validation.</p></article>
                <div class="comments"><p>Brief</p></div>
            </body></html>
        "#;

        let options = Options {
            include_comments: true,
            min_output_comm_size: 20,
            min_output_size: 3,
            min_extracted_len: 10,
            ..Options::default()
        };

        match extract_content(html, &options) {
            Ok(result) => {
                // Should have warning about comments removal
                match result.warnings.iter().find(|w| w.contains("Comments section removed")) {
                    Some(warning) => {
                        assert!(warning.contains("words"));
                        assert!(warning.contains("min:"));
                    }
                    None => panic!("expected comments removal warning"),
                }
            }
            Err(err) => panic!("expected Ok(_), got Err({err:?})"),
        }
    }
}

#[cfg(test)]
#[test]
fn test_bloginner_content_not_boilerplate() {
    assert!(!is_boilerplate("blogInner__content"), "blogInner__content should NOT be boilerplate");
}
