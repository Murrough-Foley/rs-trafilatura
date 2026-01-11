//! HTML Processing and Pruning
//!
//! Functions for analyzing and cleaning the HTML tree before extraction.
//! Port of `html-processing.go`.

use crate::dom::{self, Document, Selection};
use crate::etree;
use crate::extractor::tags::{
    TAGS_TO_CLEAN, TAGS_TO_STRIP, EMPTY_TAGS_TO_REMOVE_SET,
    TABLE_TAGS_TO_STRIP,
};
use crate::link_density::link_density_test_with_info;
use crate::lru::LruCache;
use crate::options::Options;
use crate::selector::{self, Rule};
use std::collections::HashSet;
use std::sync::LazyLock;

/// Maximum document size (in characters) for running prune_html.
/// Documents larger than this skip empty element pruning for performance.
/// The main extraction already filters empty elements during traversal.
const MAX_PRUNE_DOCUMENT_SIZE: usize = 1_000_000;

/// Elements allowed to have width/height attributes
///
/// Go source: `elementWithSizeAttr` in settings.go line 79
pub static ELEMENT_WITH_SIZE_ATTR: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    ["table", "th", "td", "hr", "pre"].into_iter().collect()
});

/// Allowed attributes for post-cleaning
///
/// Go source: `allowedAttributes` in settings.go lines 82-116
/// List of allowed attributes taken from go-domdistiller
pub static ALLOWED_ATTRIBUTES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "abbr", "accept-charset", "accept", "accesskey", "action", "align", "alink",
        "allow", "allowfullscreen", "allowpaymentrequest", "alt", "archive", "as",
        "async", "autocapitalize", "autocomplete", "autocorrect", "autofocus",
        "autoplay", "autopictureinpicture", "axis", "background", "behavior",
        "bgcolor", "border", "bordercolor", "capture", "cellpadding", "cellspacing",
        "char", "challenge", "charoff", "charset", "checked", "cite", "class",
        "classid", "clear", "code", "codebase", "codetype", "color", "cols",
        "colspan", "compact", "content", "contenteditable", "controls",
        "controlslist", "conversiondestination", "coords", "crossorigin",
        "csp", "data", "datetime", "declare", "decoding", "default", "defer",
        "dir", "direction", "dirname", "disabled", "disablepictureinpicture",
        "disableremoteplayback", "disallowdocumentaccess", "download", "draggable",
        "elementtiming", "enctype", "end", "enterkeyhint", "event", "exportparts",
        "face", "for", "form", "formaction", "formenctype", "formmethod",
        "formnovalidate", "formtarget", "frame", "frameborder", "headers",
        "height", "hidden", "high", "href", "hreflang", "hreftranslate", "hspace",
        "http-equiv", "id", "imagesizes", "imagesrcset", "importance",
        "impressiondata", "impressionexpiry", "incremental", "inert", "inputmode",
        "integrity", "is", "ismap", "keytype", "kind", "invisible", "label", "lang",
        "language", "latencyhint", "leftmargin", "link", "list", "loading", "longdesc",
        "loop", "low", "lowsrc", "manifest", "marginheight", "marginwidth", "max",
        "maxlength", "mayscript", "media", "method", "min", "minlength", "multiple",
        "muted", "name", "nohref", "nomodule", "nonce", "noresize", "noshade",
        "novalidate", "nowrap", "object", "open", "optimum", "part", "pattern",
        "placeholder", "playsinline", "ping", "policy", "poster", "preload", "pseudo",
        "readonly", "referrerpolicy", "rel", "reportingorigin", "required", "resources",
        "rev", "reversed", "role", "rows", "rowspan", "rules", "sandbox", "scheme",
        "scope", "scrollamount", "scrolldelay", "scrolling", "select", "selected",
        "shadowroot", "shadowrootdelegatesfocus", "shape", "size", "sizes", "slot",
        "span", "spellcheck", "src", "srcset", "srcdoc", "srclang", "standby", "start",
        "step", "style", "summary", "tabindex", "target", "text", "title", "topmargin",
        "translate", "truespeed", "trusttoken", "type", "usemap", "valign", "value",
        "valuetype", "version", "vlink", "vspace", "virtualkeyboardpolicy",
        "webkitdirectory", "width", "wrap",
    ]
    .into_iter()
    .collect()
});

// === Tag Sets for Text Processing ===
// These tag sets are ported from go-trafilatura/tag-converter.go

/// Tags that represent line breaks
///
/// Go source: `listXmlLbTags` in tag-converter.go line 7
static XML_LB_TAGS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    ["br", "hr", "lb"].into_iter().collect()
});

/// Tags for graphic elements
///
/// Go source: `listXmlGraphicTags` in tag-converter.go line 10
static XML_GRAPHIC_TAGS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    ["img"].into_iter().collect()
});

/// Tags that may contain code/quotes
///
/// Go source: `listXmlQuoteTags` in tag-converter.go line 5
static XML_QUOTE_TAGS: [&str; 3] = ["blockquote", "pre", "q"];

// === Deduplication Config Constants ===
// Default values from go-trafilatura/core-options.go

/// Minimum text length to check for duplicates (default: 100 chars)
const MIN_DUPLICATE_CHECK_SIZE: usize = 100;

/// Maximum times text can appear before being considered duplicate (default: 2)
const MAX_DUPLICATE_COUNT: i32 = 2;

// === Document Cleaning Functions ===

/// Clean the document by discarding unwanted elements
///
/// Go equivalent: `docCleaning(doc, opts)` (lines 35-97)
///
/// Performance: Uses combined CSS selectors for O(1) queries instead of O(n) per-tag queries.
pub fn doc_cleaning(doc: &Document, opts: &Options) {
    let exclude_tables = !opts.include_tables;

    // Handle figure elements containing tables or blockquotes before cleaning.
    // EPIC-05: Combined selectors - 2 tree scans → 1 tree scan
    // Before: Two separate loops over doc.select("figure")
    // After: Single loop, checking both conditions
    for figure_node in doc.select("figure").nodes() {
        let figure = Selection::from(*figure_node);

        // Handle figure containing tables (if tables are included)
        let has_table = !exclude_tables && figure.select("table").length() > 0;

        // Handle figure containing blockquotes.
        // These are quote boxes (like testimonials/reviews) that contain text content,
        // not images, so they should be preserved by renaming to div.
        let has_blockquote = figure.select("blockquote").length() > 0;

        if has_table || has_blockquote {
            dom::rename(&figure, "div");
        }
    }

    // Handle noscript elements specially: if they contain substantial content
    // (due to malformed HTML where content got parsed inside noscript),
    // strip the tag but keep children instead of removing everything.
    // This handles cases like legacy pages with `<noscript><div>...</noscript>` without proper closing.
    // BUT skip noscript elements that contain consent/GDPR banners.
    const NOSCRIPT_CONTENT_THRESHOLD: usize = 500;
    for noscript_node in doc.select("noscript").nodes() {
        let noscript = Selection::from(*noscript_node);
        let text = noscript.text();
        let text_lower = text.to_lowercase();
        let text_len = text.trim().len();

        // Skip if it looks like a consent/GDPR banner
        let is_consent = text_lower.contains("cookie")
            || text_lower.contains("consent")
            || text_lower.contains("gdpr")
            || text_lower.contains("privacy")
            || text_lower.contains("third party partners")
            || text_lower.contains("personalize content")
            || text_lower.contains("enable javascript");

        if text_len > NOSCRIPT_CONTENT_THRESHOLD && !is_consent {
            // Has substantial non-consent content - strip tag but keep children
            etree::strip(&noscript);
        }
        // else: will be removed by normal cleaning
    }

    // Build combined CSS selector for cleaning (performance optimization)
    // Instead of 50+ separate select() calls, use one comma-separated selector
    let clean_selector = build_clean_selector(opts);
    let strip_selector = build_strip_selector(opts);

    // Strip tags first (remove tag but keep children) - single combined selector
    if !strip_selector.is_empty() {
        dom::strip_tags(&doc.select("body"), &strip_selector.iter().map(String::as_str).collect::<Vec<_>>());
    }

    // Remove cleaning tags - single combined selector
    if !clean_selector.is_empty() {
        let combined = clean_selector.join(", ");
        doc.select(&combined).remove();
    }

    // Remove HTML comments
    dom::remove_comments(doc);

    // Prune empty elements (skip for performance if document is very large)
    // EPIC-04: Use StrTendril.len() directly - no allocation needed
    let body_len = doc.select("body").text().len();
    if body_len < MAX_PRUNE_DOCUMENT_SIZE {
        prune_html(doc, opts);
    }
}

/// Build combined selector for tags to clean (remove with children).
fn build_clean_selector(opts: &Options) -> Vec<String> {
    let mut selectors: Vec<String> = TAGS_TO_CLEAN.iter().map(|s| (*s).to_string()).collect();

    if !opts.include_tables {
        selectors.extend(["table", "td", "th", "tr"].iter().map(|s| (*s).to_string()));
    }

    if opts.include_images {
        selectors.retain(|t| !matches!(t.as_str(), "figure" | "picture" | "source"));
    }

    // Add modal/gdpr/consent selectors - these are cookie consent banners that
    // should be removed before content extraction.
    //
    // NOTE: Be careful with [class*="modal"] - it's too aggressive and matches
    // things like "js-modal-gallery" which is a JavaScript gallery feature, not
    // a modal popup. Use more specific patterns that target actual modal dialogs.
    selectors.extend([
        // Modal dialogs (more specific patterns to avoid matching galleries)
        ".modal-dialog".to_string(),
        ".modal-content".to_string(),
        ".modal-backdrop".to_string(),
        ".modal-overlay".to_string(),
        "[class~=\"modal\"]".to_string(),  // Exact word match (space-separated)
        "[role=\"dialog\"]".to_string(),
        // GDPR/consent patterns
        "[id*=\"gdpr\"]".to_string(),
        "[class*=\"gdpr\"]".to_string(),
        "[id*=\"consent\"]".to_string(),
        "[class*=\"consent\"]".to_string(),
        "[class*=\"cookie-banner\"]".to_string(),
        "[id*=\"cookie-banner\"]".to_string(),
        "[class*=\"cookiebanner\"]".to_string(),
        "[id*=\"cookiebanner\"]".to_string(),
    ]);

    selectors
}

/// Build list of tags to strip (remove tag but keep children).
fn build_strip_selector(opts: &Options) -> Vec<String> {
    let mut tags: Vec<String> = TAGS_TO_STRIP.iter().map(|s| (*s).to_string()).collect();
    // Also include table structure tags (tbody, tfoot, thead)
    tags.extend(TABLE_TAGS_TO_STRIP.iter().map(|s| (*s).to_string()));

    if opts.include_images {
        tags.retain(|t| t != "img");
    }

    tags
}

/// Delete selected empty elements to save space and processing time
///
/// Go equivalent: `pruneHTML(doc, opts)` (lines 123-138)
pub fn prune_html(doc: &Document, opts: &Options) {
    let keep_tail = !opts.favor_precision;

    // Process in reverse document order (children before parents)
    let all_elements = doc.select("*").nodes().to_vec();

    for node in all_elements.into_iter().rev() {
        let sel = Selection::from(node);
        let tag_name = dom::tag_name(&sel).unwrap_or_default();

        if !EMPTY_TAGS_TO_REMOVE_SET.contains(tag_name.as_str()) {
            continue;
        }

        let children = dom::children(&sel);
        let text = etree::text(&sel);
        let tail = etree::tail(&sel);

        // Remove if empty (no children and no text content)
        if children.is_empty() && text.trim().is_empty() && tail.trim().is_empty() {
            etree::remove(&sel, keep_tail);
        }
    }
}

/// Clean the extracted content (post-processing)
///
/// Go equivalent: `postCleaning(doc)` (lines 398-448)
pub fn post_cleaning(doc: &Document) {
    // Remove empty nodes (process in reverse order)
    let all_elements = doc.select("*").nodes().to_vec();

    for node in all_elements.into_iter().rev() {
        let sel = Selection::from(node);
        let children = dom::children(&sel);
        let is_void = dom::is_void_element(&sel);
        let text = etree::text(&sel);
        let is_empty = !text_chars_test(&text);

        if children.is_empty() && is_empty && !is_void {
            etree::strip(&sel);
        }
    }

    // Remove useless attributes
    for node in doc.select("*").nodes() {
        let sel = Selection::from(*node);
        let tag_name = dom::tag_name(&sel).unwrap_or_default();
        let allows_size = ELEMENT_WITH_SIZE_ATTR.contains(tag_name.as_str());

        // Get current attributes
        let attrs = dom::get_all_attributes(&sel);

        // Remove presentational and unsafe attributes
        for (key, _) in attrs {
            let should_remove = match key.as_str() {
                // Always remove presentational attributes
                "id" | "class" | "align" | "background" | "bgcolor" | "border"
                | "cellpadding" | "cellspacing" | "frame" | "hspace" | "rules"
                | "style" | "valign" | "vspace" => true,

                // Size attributes only allowed on specific elements
                "width" | "height" => !allows_size,

                // Keep only allowed attributes
                _ => !ALLOWED_ATTRIBUTES.contains(key.as_str()),
            };

            if should_remove {
                dom::remove_attribute(&sel, &key);
            }
        }
    }
}

// === Link Density Functions ===
// Note: link_density_test and link_density_test_tables are in src/link_density.rs
// We use link_density_test_with_info for delete_by_link_density's backtracking logic.

/// Delete elements with high link density
///
/// Determines the link density of elements with respect to their length,
/// and removes the elements identified as boilerplate.
///
/// Go equivalent: `deleteByLinkDensity(subTree, opts, backtracking, tagNames...)`
pub fn delete_by_link_density(
    sub_tree: &Selection,
    opts: &Options,
    backtracking: bool,
    tag_names: &[&str],
) {
    let mut nodes_to_delete = Vec::new();

    let threshold = if opts.favor_precision { 200 } else { 100 };
    let n_child_limit = if opts.favor_precision { 1 } else { 3 };

    // Collect nodes to delete
    for elem_node in etree::iter(sub_tree, tag_names).nodes() {
        let elem = Selection::from(*elem_node);
        let (has_non_empty_links, is_high_density) = link_density_test_with_info(&elem, opts);

        if is_high_density {
            nodes_to_delete.push(*elem_node);
        } else if backtracking && has_non_empty_links {
            let text = dom::text_content(&elem).trim().to_string();
            let text_length = text.chars().count();
            let n_children = dom::children(&elem).length();

            if text_length > 0 && text_length < threshold && n_children >= n_child_limit {
                nodes_to_delete.push(*elem_node);
            }
        }
    }

    // Remove nodes in reverse order
    for node in nodes_to_delete.iter().rev() {
        let sel = Selection::from(*node);
        etree::remove(&sel, false);
    }
}

/// Check if text contains enough characters
///
/// Go equivalent: `textCharsTest(text)`
#[must_use]
pub fn text_chars_test(text: &str) -> bool {
    text.chars().any(char::is_alphanumeric)
}

/// Text filter - check if element should be filtered out
///
/// Go equivalent: `textFilter(n)` in utils-extractor.go lines 112-127
fn text_filter(element: &Selection) -> bool {
    // Get all descendant text to properly catch boilerplate in child elements
    // This fixes cases like <h2><a>Subscribe to Newsletter</a></h2>
    // where the text is inside the anchor child
    let all_text = etree::iter_text(element, " ");
    let all_text = all_text.trim();

    // Also check tail (text after this element)
    let tail = etree::tail(element);

    // Check if text has no alphanumeric characters
    if all_text.is_empty() {
        if !text_chars_test(&tail) {
            return true;
        }
    } else if !text_chars_test(all_text) {
        return true;
    }

    // Determine which text to test for boilerplate patterns
    let test_text = if all_text.is_empty() { &tail } else { all_text };

    // Check for share button / social media patterns on each line
    for line in test_text.lines() {
        if is_share_button_text(line) {
            return true;
        }
    }

    false
}

/// Check if text matches share button / social media patterns
///
/// Go equivalent: `re2go.IsTextFilter` in internal/re2go/utils-extractor.re lines 25-27
/// Pattern: social media names, share buttons, "More on this", "Mehr zum Thema", etc.
pub fn is_share_button_text(text: &str) -> bool {
    let trimmed = text.trim();

    // Skip if text has alphanumeric prefix (not a standalone share button)
    // Go pattern: (![^a-zA-Z0-9_])? means optional non-alphanumeric prefix
    let test_str = trimmed
        .trim_start_matches(|c: char| !c.is_alphanumeric() && c != '_');

    // Check for exact matches of social media / share button text
    let patterns = [
        "Drucken",
        "E-Mail",
        "Email",
        "EMail",
        "Facebook",
        "Flipboard",
        "Google",
        "Instagram",
        "Linkedin",
        "LinkedIn",
        "Mail",
        "PDF",
        "Pinterest",
        "Pocket",
        "Print",
        "QQ",
        "Reddit",
        "Twitter",
        "WeChat",
        "WeiBo",
        "Weibo",
        "Whatsapp",
        "WhatsApp",
        "Xing",
        "XING",
    ];

    for pattern in patterns {
        if let Some(rest) = test_str.strip_prefix(pattern) {
            // Check that pattern is followed by non-alphanumeric or end of string
            if rest.is_empty() || !rest.chars().next().unwrap_or(' ').is_alphanumeric() {
                return true;
            }
        }
    }

    // Check for "More on this" and similar patterns (case insensitive)
    let lower = test_str.to_lowercase();
    if lower.starts_with("more on this") || lower.starts_with("mehr zum thema") {
        return true;
    }

    // Check for standalone "Comments" (common boilerplate heading)
    if lower == "comments" || lower == "comment" || lower == "kommentare" {
        return true;
    }

    // Check for newsletter/subscribe CTAs (common boilerplate)
    if (lower.contains("subscribe") && lower.contains("newsletter"))
        || lower.starts_with("click here to subscribe")
        || lower.starts_with("sign up for")
        || lower.starts_with("join our newsletter")
        || lower.starts_with("breaking news emails")  // NBC News newsletter
        || lower.starts_with("get breaking news")     // Newsletter signup text
        || lower == "subscribe"                        // Standalone subscribe button
    {
        return true;
    }

    // Check for image interaction UI elements
    if lower == "enlarge image"
        || lower == "view image"
        || lower == "click to enlarge"
        || lower == "zoom"
        || lower == "view gallery"
        || lower == "view photos"
    {
        return true;
    }

    // Check for photo/image credit lines (short text only)
    // Pattern: "Photo by X", "Image: X", "Credit: X", or "X | Getty Images"
    if trimmed.len() < 120 {
        // Direct prefix patterns
        if lower.starts_with("photo:")
            || lower.starts_with("photo by")
            || lower.starts_with("image:")
            || lower.starts_with("image by")
            || lower.starts_with("credit:")
            || lower.starts_with("source:")
        {
            return true;
        }

        // Photo agency names in short text (e.g., "Natalie Naccache | Bloomberg | Getty Images")
        let photo_agencies = [
            "getty images",
            "getty",
            "afp",
            "ap photo",
            "associated press",
            "shutterstock",
            "alamy",
            "rex features",
            "splash news",
            "wireimage",
            "filmmagic",
        ];
        for agency in photo_agencies {
            if lower.contains(agency) {
                return true;
            }
        }
    }

    // Check for byline and publication metadata (short text only)
    if trimmed.len() < 80 {
        // News agency attribution at start of text
        // e.g., "Reuters", "PTI", "AFP", "Staff Reports", "By Reuters"
        let news_agencies = [
            "reuters,",       // "Reuters, Washington"
            "pti,",           // "PTI, New Delhi"
            "ians,",          // "IANS, Mumbai"
            "ani,",           // "ANI, Delhi"
            "xinhua,",        // "Xinhua, Beijing"
            "staff reports",  // Common byline
            "staff report",
            "staff writer",
            "special to",     // "Special to The Times"
        ];
        for agency in news_agencies {
            if lower.starts_with(agency) {
                return true;
            }
        }

        // "By [Author]" byline pattern - very short text only
        // Avoid matching "By the end of..." type content
        if lower.starts_with("by ") && trimmed.len() < 50 {
            let after_by = &trimmed[3..];
            // Check that next char is uppercase (author name) and no sentence structure
            if let Some(first_char) = after_by.chars().next() {
                if first_char.is_uppercase() && !after_by.contains(". ") && !after_by.contains(", the ") {
                    return true;
                }
            }
        }

        // Timestamp/publication metadata prefixes
        if lower.starts_with("updated:")
            || lower.starts_with("published:")
            || lower.starts_with("last updated")
            || lower.starts_with("posted:")
            || lower.starts_with("date:")
        {
            return true;
        }
    }

    false
}

/// Check if element is an image element with valid source
///
/// Go equivalent: `isImageElement(node)` in utils-common.go lines 54-64
fn is_image_element(sel: &Selection) -> bool {
    // Check src attribute
    if let Some(src) = dom::get_attribute(sel, "src") {
        if is_image_file(&src) {
            return true;
        }
    }

    // Check data-src* attributes (lazy loading patterns)
    let attrs = dom::get_all_attributes(sel);
    for (key, value) in attrs {
        if key.starts_with("data-src") && is_image_file(&value) {
            return true;
        }
    }

    false
}

/// Check if a URL points to an image file by extension
///
/// Go equivalent: `isImageFile(imageSrc)` in utils-common.go lines 66-74
fn is_image_file(src: &str) -> bool {
    if src.is_empty() {
        return false;
    }

    // Extract extension from URL (handle query strings)
    let path = src.split('?').next().unwrap_or(src);
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();

    // Check common image extensions
    matches!(
        ext.as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "bmp" | "ico" | "tiff" | "tif" | "avif"
    )
}

/// Check for duplicate content using cache
///
/// Go equivalent: `duplicateTest(element, cache, opts)` in utils-extractor.go lines 136-149
///
/// Returns `true` if this text has been seen more than `MAX_DUPLICATE_COUNT` times.
fn duplicate_test(element: &Selection, cache: &mut LruCache) -> bool {
    let test_string = etree::iter_text(element, " ").trim().to_string();

    // Skip short text
    if test_string.chars().count() <= MIN_DUPLICATE_CHECK_SIZE {
        return false;
    }

    // Check count and increment
    let count = cache.get(&test_string).unwrap_or(0);
    let is_duplicate = count > MAX_DUPLICATE_COUNT;

    // Always increment the count
    cache.put(&test_string, count + 1);

    is_duplicate
}

/// Clear all attributes from an element
///
/// Go equivalent: `elem.Attr = nil` in html-processing.go
fn clear_all_attributes(sel: &Selection) {
    let attrs = dom::get_all_attributes(sel);
    for (key, _) in attrs {
        dom::remove_attribute(sel, &key);
    }
}

/// Convert relative URL to absolute using base URL
///
/// Go equivalent: `createAbsoluteURL(url, base)` in url.go lines 48-80
fn create_absolute_url(href: &str, base_url: Option<&str>) -> String {
    // Empty href or no base - return as-is
    if href.is_empty() {
        return href.to_string();
    }

    // Hash URLs - return as-is
    if href.starts_with('#') {
        return href.to_string();
    }

    // Data URIs - return as-is
    if href.starts_with("data:") {
        return href.to_string();
    }

    // Already absolute - return as-is
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }

    // Protocol-relative URLs
    if href.starts_with("//") {
        return format!("https:{href}");
    }

    // No base URL - return as-is
    let Some(base) = base_url else {
        return href.to_string();
    };

    // Use url crate to resolve
    match url::Url::parse(base) {
        Ok(base_url) => match base_url.join(href) {
            Ok(absolute) => absolute.to_string(),
            Err(_) => href.to_string(),
        },
        Err(_) => href.to_string(),
    }
}

/// Process node - converts, formats, and probes potential text elements (light format)
///
/// Go equivalent: `processNode(element, cache, opts)` (lines 362-396)
///
/// Note: This implementation differs from Go in one way - we reject whitespace-only
/// nodes (after trimming both text and tail are empty), while Go returns the element.
/// This is arguably better behavior for content extraction.
#[must_use]
pub fn process_node(element: &Selection, cache: Option<&mut LruCache>, opts: &Options) -> bool {
    let mut text = etree::text(element);
    let mut tail = etree::tail(element);
    let tag_name = dom::tag_name(element).unwrap_or_default();
    let children = dom::children(element);

    if tag_name == "done" || (children.is_empty() && text.is_empty() && tail.is_empty()) {
        return false;
    }

    // Trim
    text = text.trim().to_string();
    tail = tail.trim().to_string();
    etree::set_text(element, &text);
    etree::set_tail(element, &tail);

    // Adapt content string - move tail to text for non-linebreak tags
    if !XML_LB_TAGS.contains(tag_name.as_str()) && text.is_empty() && !tail.is_empty() {
        std::mem::swap(&mut text, &mut tail);
        etree::set_text(element, &text);
        etree::set_tail(element, &tail);
    }

    // Content checks
    if !text.is_empty() || !tail.is_empty() {
        if text_filter(element) {
            return false;
        }

        // Deduplication check
        if opts.deduplicate {
            if let Some(cache) = cache {
                if duplicate_test(element, cache) {
                    return false;
                }
            }
        }
    } else {
        // After trimming, if both text and tail are empty, reject the node
        // Note: Go returns element here, but rejecting empty nodes is better behavior
        return false;
    }

    true
}

/// Handle text node - converts, formats and probes potential text elements
///
/// Go equivalent: `handleTextNode(node, cache, fixComments, preserveSpaces, opts)` (lines 190-242)
///
/// # Arguments
/// * `node` - Element to process
/// * `cache` - Optional LRU cache for deduplication
/// * `fix_comments` - Whether to fix comment formatting (converts br/hr to p)
/// * `preserve_spaces` - Whether to preserve whitespace
/// * `opts` - Extraction options
///
/// # Returns
/// * `true` if node should be kept
/// * `false` if node should be removed
#[must_use]
pub fn handle_text_node(
    node: &Selection,
    cache: Option<&mut LruCache>,
    fix_comments: bool,
    preserve_spaces: bool,
    opts: &Options,
) -> bool {
    let tag_name = dom::tag_name(node).unwrap_or_default();

    // Image element bypass
    if XML_GRAPHIC_TAGS.contains(tag_name.as_str()) && is_image_element(node) {
        return true;
    }

    // Make sure text is not empty
    let mut text = etree::text(node);
    let mut tail = etree::tail(node);
    let children = dom::children(node);

    if tag_name == "done" || (children.is_empty() && text.is_empty() && tail.is_empty()) {
        return false;
    }

    // Line break bypass
    if !fix_comments && XML_LB_TAGS.contains(tag_name.as_str()) {
        if !preserve_spaces {
            etree::set_tail(node, tail.trim());
        }
        return true;
    }

    // If text is empty, try tail
    if text.is_empty() && children.is_empty() {
        std::mem::swap(&mut text, &mut tail);
        etree::set_text(node, &text);
        etree::set_tail(node, &tail);

        // Handle differently for br/hr - convert to paragraph
        if fix_comments && XML_LB_TAGS.contains(tag_name.as_str()) {
            dom::rename(node, "p");
        }
    }

    // Trim values
    if !preserve_spaces {
        text = text.trim().to_string();
        tail = tail.trim().to_string();
        etree::set_text(node, &text);
        etree::set_tail(node, &tail);
    }

    // Filter out empty text
    if text.is_empty() && text_filter(node) {
        return false;
    }

    // Deduplication check
    if opts.deduplicate {
        if let Some(cache) = cache {
            if duplicate_test(node, cache) {
                return false;
            }
        }
    }

    true
}

/// Prune unwanted nodes from the HTML tree
///
/// Go equivalent: `pruneUnwantedNodes(tree, queries, withBackup...)`
/// 
/// Note: This function modifies the tree in place and returns a clone.
/// In Go, the tree is cloned internally. We do the same here.
pub fn prune_unwanted_nodes(
    tree: &Selection,
    queries: &[Rule],
    with_backup: bool,
) {
    let old_len = if with_backup {
        dom::text_content(tree).chars().count()
    } else {
        0
    };

    // Collect all elements to remove first
    let mut all_elements_to_remove = Vec::new();
    
    for query in queries {
        let sub_elements = selector::query_all(tree, *query);
        all_elements_to_remove.extend(sub_elements);
    }

    // Process in reverse order to avoid issues with tree modification
    for sub_element in all_elements_to_remove.iter().rev() {
        // Preserve tail text from deletion
        let tail = etree::tail(sub_element);
        if !tail.is_empty() {
            if let Some(previous) = dom::previous_element_sibling(sub_element) {
                let previous_tail = etree::tail(&previous);
                if previous_tail.is_empty() {
                    etree::set_tail(&previous, &tail);
                } else {
                    etree::set_tail(&previous, &format!("{previous_tail} {tail}"));
                }
            } else {
                // Use parent if no previous sibling
                let parent = dom::parent(sub_element);
                if parent.length() > 0 {
                    let parent_tail = etree::tail(&parent);
                    if parent_tail.is_empty() {
                        etree::set_tail(&parent, &tail);
                    } else {
                        etree::set_tail(&parent, &format!("{parent_tail} {tail}"));
                    }
                }
            }
        }

        etree::remove(sub_element, false);
    }

    // Check if we removed too much content - would need backup/restore logic
    // For now, we just log a warning if too much was removed
    if with_backup {
        let new_len = dom::text_content(tree).chars().count();
        if new_len <= old_len / 7 {
            // In Go, this would restore from backup
            // For now, we just note this happened
            // TODO: Implement proper backup/restore mechanism
        }
    }
}

// === Tag Conversion Functions ===

/// Simplify HTML markup by converting tags
///
/// This function processes the HTML tree to:
/// 1. Handle links - either strip non-essential links or convert relative URLs to absolute
/// 2. Detect code blocks - convert pre/code/blockquote with hljs markers to code tags
///
/// Go equivalent: `convertTags(tree, opts)` (lines 485-557)
pub fn convert_tags(tree: &Selection, opts: &Options) {
    if opts.include_links {
        // Convert relative URLs to absolute
        for node in tree.select("a").nodes() {
            let sel = Selection::from(*node);

            // Extract link attributes before clearing
            let href = dom::get_attribute(&sel, "href")
                .map(|h| h.trim().to_string())
                .unwrap_or_default();
            let target = dom::get_attribute(&sel, "target")
                .map(|t| t.trim().to_string())
                .unwrap_or_default();

            // Clear all attributes
            clear_all_attributes(&sel);

            // Convert relative URL to absolute and set back
            if !href.is_empty() {
                let absolute_href = create_absolute_url(&href, opts.url.as_deref());
                dom::set_attribute(&sel, "href", &absolute_href);
            }

            if !target.is_empty() {
                dom::set_attribute(&sel, "target", &target);
            }
        }
    } else {
        // Delete links for faster processing (if not including links)
        // Prepare selector for important links to preserve
        // Links inside content containers should be kept
        let css_selector = if opts.include_tables {
            "div a, ul a, ol a, dl a, p a, table a"
        } else {
            "div a, ul a, ol a, dl a, p a"
        };

        // Temporarily rename important links to protect them
        for node in tree.select(css_selector).nodes() {
            let sel = Selection::from(*node);
            dom::rename(&sel, "protected-a");
        }

        // Strip all remaining (unimportant) links - removes tag but keeps text
        dom::strip_tags(tree, &["a"]);

        // Revert protected links back to 'a' tags
        for node in tree.select("protected-a").nodes() {
            let sel = Selection::from(*node);
            dom::rename(&sel, "a");
        }
    }

    // Process quote/code elements - detect code blocks
    // EPIC-05: Combined selectors - 3 tree scans → 1 tree scan
    // Before: Loop over XML_QUOTE_TAGS = ["blockquote", "pre", "q"]
    // After: Single combined selector
    let quote_selector = XML_QUOTE_TAGS.join(", ");
    for node in tree.select(&quote_selector).nodes() {
        let sel = Selection::from(*node);
        let tag_name = dom::tag_name(&sel).unwrap_or_default();
        let mut code_flag = false;

        // Pre with a single span child is more likely to be code
        if tag_name == "pre" {
            let children = dom::children(&sel);
            if children.length() == 1 {
                let first_child = children.first();
                if dom::tag_name(&first_child) == Some("span".to_string()) {
                    code_flag = true;
                }
            }
        }

        // Find hljs (highlight.js) elements to detect code
        // Classes like "hljs-keyword", "hljs-string", etc.
        let hljs_selector = r#"span[class*=" hljs"], span[class^="hljs"]"#;
        let hljs_elems = sel.select(hljs_selector);
        if hljs_elems.length() > 0 {
            code_flag = true;
            // Clear attributes from hljs spans (remove class attribute noise)
            for hljs_node in hljs_elems.nodes() {
                clear_all_attributes(&Selection::from(*hljs_node));
            }
        }

        // Convert to code tag if code was detected
        if code_flag {
            dom::rename(&sel, "code");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom;
    use crate::options::Options;

    // Note: link_density_test* tests moved to src/link_density.rs

    #[test]
    fn test_text_chars_test() {
        assert!(text_chars_test("Hello world"));
        assert!(text_chars_test("123"));
        assert!(!text_chars_test("   "));
        assert!(!text_chars_test(""));
        assert!(!text_chars_test("..."));
    }

    #[test]
    fn test_process_node() {
        let doc = dom::parse("<p>  Text content  </p>");
        let p = doc.select("p");
        let opts = Options::default();

        assert!(process_node(&p, None, &opts));
        assert_eq!(etree::text(&p).trim(), "Text content");
    }

    #[test]
    fn test_process_node_empty() {
        let doc = dom::parse("<p>   </p>");
        let p = doc.select("p");
        let opts = Options::default();

        assert!(!process_node(&p, None, &opts));
    }

    #[test]
    fn test_delete_by_link_density() {
        let html = r#"<div>
            <p>Good paragraph with real content here</p>
            <div><a href='#'>Link 1</a><a href='#'>Link 2</a></div>
        </div>"#;
        let doc = dom::parse(html);
        let root = doc.select("div").first();
        let opts = Options::default();

        delete_by_link_density(&root, &opts, false, &["div"]);

        // The inner div with links should be removed
        assert_eq!(root.select("div").length(), 0);
        // But paragraph preserved
        assert_eq!(root.select("p").length(), 1);
    }

    // === Document Cleaning Tests ===

    #[test]
    fn test_doc_cleaning_removes_script() {
        let doc = dom::parse("<div><script>alert(1)</script><p>Content</p></div>");
        let opts = Options::default();

        doc_cleaning(&doc, &opts);

        assert_eq!(doc.select("script").length(), 0);
        assert_eq!(doc.select("p").length(), 1);
    }

    #[test]
    fn test_doc_cleaning_preserves_tables() {
        let doc = dom::parse("<div><table><tr><td>Data</td></tr></table></div>");
        let mut opts = Options::default();
        opts.include_tables = true;

        doc_cleaning(&doc, &opts);

        assert!(doc.select("table").length() > 0);
    }

    #[test]
    fn test_doc_cleaning_removes_tables() {
        let doc = dom::parse("<div><table><tr><td>Data</td></tr></table><p>Text</p></div>");
        let mut opts = Options::default();
        opts.include_tables = false;

        doc_cleaning(&doc, &opts);

        assert_eq!(doc.select("table").length(), 0);
        assert_eq!(doc.select("p").length(), 1);
    }

    #[test]
    fn test_prune_html_removes_empty() {
        let doc = dom::parse("<div><p></p><p>Content</p></div>");
        let opts = Options::default();

        prune_html(&doc, &opts);

        // Empty p should be removed
        assert_eq!(doc.select("p").length(), 1);
        assert_eq!(doc.select("p").text().to_string(), "Content");
    }

    #[test]
    fn test_post_cleaning_removes_class() {
        let doc = dom::parse(r##"<div><p class="article" id="main">Text</p></div>"##);

        post_cleaning(&doc);

        let p = doc.select("p");
        assert!(dom::get_attribute(&p, "class").is_none());
        assert!(dom::get_attribute(&p, "id").is_none());
    }

    #[test]
    fn test_post_cleaning_keeps_href() {
        let doc = dom::parse(r##"<div><a href="http://example.com" class="link">Link</a></div>"##);

        post_cleaning(&doc);

        let a = doc.select("a");
        assert!(dom::get_attribute(&a, "href").is_some());
        assert!(dom::get_attribute(&a, "class").is_none());
    }

    #[test]
    fn test_post_cleaning_removes_empty_nodes() {
        let doc = dom::parse("<div><p></p><span></span><p>Content</p></div>");

        post_cleaning(&doc);

        // Empty p and span should be stripped
        assert_eq!(doc.select("p").length(), 1);
        assert_eq!(doc.select("span").length(), 0);
        // Content paragraph preserved
        assert!(doc.select("p").text().to_string().contains("Content"));
    }

    #[test]
    fn test_post_cleaning_keeps_void_elements() {
        let doc = dom::parse(r##"<div><br><img src="test.jpg"><p>Content</p></div>"##);

        post_cleaning(&doc);

        // Void elements should be preserved even though they're "empty"
        assert_eq!(doc.select("br").length(), 1);
        assert_eq!(doc.select("img").length(), 1);
    }

    // === Story 3.3 Tests: Text Node Processing ===

    #[test]
    fn test_handle_text_node_empty() {
        let doc = dom::parse("<p></p>");
        let p = doc.select("p");
        let opts = Options::default();

        assert!(!handle_text_node(&p, None, false, false, &opts));
    }

    #[test]
    fn test_handle_text_node_with_content() {
        let doc = dom::parse("<p>Content</p>");
        let p = doc.select("p");
        let opts = Options::default();

        assert!(handle_text_node(&p, None, false, false, &opts));
    }

    #[test]
    fn test_handle_text_node_image_bypass() {
        let doc = dom::parse(r#"<img src="test.jpg">"#);
        let img = doc.select("img");
        let opts = Options::default();

        assert!(handle_text_node(&img, None, false, false, &opts));
    }

    #[test]
    fn test_handle_text_node_image_without_src() {
        let doc = dom::parse("<img>");
        let img = doc.select("img");
        let opts = Options::default();

        // Image without src/srcset should not bypass
        assert!(!handle_text_node(&img, None, false, false, &opts));
    }

    #[test]
    fn test_handle_text_node_linebreak_bypass() {
        let doc = dom::parse("<div><br> tail text</div>");
        let br = doc.select("br");
        let opts = Options::default();

        // br should bypass when fix_comments=false
        assert!(handle_text_node(&br, None, false, false, &opts));
    }

    #[test]
    fn test_handle_text_node_fix_comments_converts_br_to_p() {
        let doc = dom::parse("<div><br>content</div>");
        let br = doc.select("br");
        let opts = Options::default();

        // With fix_comments=true, br with tail should become p
        let _ = handle_text_node(&br, None, true, false, &opts);

        // Check if renamed to p
        assert_eq!(doc.select("p").length(), 1);
        assert_eq!(doc.select("br").length(), 0);
    }

    #[test]
    fn test_handle_text_node_moves_tail_to_text() {
        let doc = dom::parse("<div><span></span>tail content</div>");
        let span = doc.select("span");
        let opts = Options::default();

        let _ = handle_text_node(&span, None, false, false, &opts);

        // Tail should be moved to text
        assert_eq!(etree::text(&span).trim(), "tail content");
        assert!(etree::tail(&span).is_empty());
    }

    #[test]
    fn test_handle_text_node_preserves_spaces() {
        let doc = dom::parse("<p>  spaced content  </p>");
        let p = doc.select("p");
        let opts = Options::default();

        let _ = handle_text_node(&p, None, false, true, &opts);

        // Spaces should be preserved
        assert_eq!(etree::text(&p), "  spaced content  ");
    }

    #[test]
    fn test_process_node_with_cache_dedup() {
        // Text must be > 100 characters to trigger deduplication check
        let long_text = "This is some very long text that needs to exceed the minimum duplicate check size of one hundred characters in order to properly test the deduplication feature.";
        let html = format!("<p>{long_text}</p>");
        let doc = dom::parse(&html);
        let p = doc.select("p");
        let mut opts = Options::default();
        opts.deduplicate = true;

        let mut cache = LruCache::new(100);

        // First occurrence - should pass (count goes to 1)
        assert!(process_node(&p, Some(&mut cache), &opts));

        // Second occurrence - should pass (count goes to 2, check is 1 > 2 = false)
        assert!(process_node(&p, Some(&mut cache), &opts));

        // Third occurrence - should pass (count goes to 3, check is 2 > 2 = false)
        assert!(process_node(&p, Some(&mut cache), &opts));

        // Fourth occurrence - should be marked as duplicate (check is 3 > 2 = true)
        assert!(!process_node(&p, Some(&mut cache), &opts));
    }

    #[test]
    fn test_is_image_element_with_src() {
        let doc = dom::parse(r#"<img src="test.jpg">"#);
        let img = doc.select("img");
        assert!(is_image_element(&img));
    }

    #[test]
    fn test_is_image_element_with_data_src() {
        let doc = dom::parse(r#"<img data-src="lazy.png">"#);
        let img = doc.select("img");
        assert!(is_image_element(&img));
    }

    #[test]
    fn test_is_image_element_with_data_srcset() {
        let doc = dom::parse(r#"<img data-srcset="lazy.webp">"#);
        let img = doc.select("img");
        assert!(is_image_element(&img));
    }

    #[test]
    fn test_is_image_element_without_src() {
        let doc = dom::parse("<img alt='no source'>");
        let img = doc.select("img");
        assert!(!is_image_element(&img));
    }

    #[test]
    fn test_is_image_element_non_image_src() {
        // src exists but doesn't point to an image file
        let doc = dom::parse(r#"<img src="data.json">"#);
        let img = doc.select("img");
        assert!(!is_image_element(&img));
    }

    #[test]
    fn test_is_image_file() {
        assert!(is_image_file("photo.jpg"));
        assert!(is_image_file("image.PNG"));
        assert!(is_image_file("icon.gif"));
        assert!(is_image_file("banner.webp"));
        assert!(is_image_file("https://example.com/path/to/image.jpeg?v=123"));
        assert!(!is_image_file("document.pdf"));
        assert!(!is_image_file("script.js"));
        assert!(!is_image_file(""));
    }

    #[test]
    fn test_create_absolute_url_already_absolute() {
        assert_eq!(
            create_absolute_url("https://example.com/page", Some("https://base.com")),
            "https://example.com/page"
        );
    }

    #[test]
    fn test_create_absolute_url_relative_path() {
        assert_eq!(
            create_absolute_url("/page", Some("https://example.com/article/")),
            "https://example.com/page"
        );
    }

    #[test]
    fn test_create_absolute_url_relative_file() {
        assert_eq!(
            create_absolute_url("other.html", Some("https://example.com/article/")),
            "https://example.com/article/other.html"
        );
    }

    #[test]
    fn test_create_absolute_url_protocol_relative() {
        assert_eq!(
            create_absolute_url("//cdn.example.com/file.js", Some("https://example.com")),
            "https://cdn.example.com/file.js"
        );
    }

    #[test]
    fn test_create_absolute_url_hash() {
        assert_eq!(
            create_absolute_url("#section", Some("https://example.com")),
            "#section"
        );
    }

    #[test]
    fn test_create_absolute_url_data_uri() {
        let data_uri = "data:image/png;base64,ABC123";
        assert_eq!(
            create_absolute_url(data_uri, Some("https://example.com")),
            data_uri
        );
    }

    #[test]
    fn test_create_absolute_url_no_base() {
        assert_eq!(create_absolute_url("relative", None), "relative");
    }

    #[test]
    fn test_convert_tags_strips_standalone_links() {
        // Standalone links that are direct children of body (not in div/p/ul/ol/dl) get stripped
        let html = r##"<body><p>Text <a href="#">Link</a></p><span><a href="#">Standalone</a></span></body>"##;
        let doc = dom::parse(html);
        let body = doc.select("body");
        let mut opts = Options::default();
        opts.include_links = false;

        convert_tags(&body, &opts);

        // Link in paragraph should be preserved (important link)
        // Link in span (not a content container) should be stripped
        let links = body.select("a");
        assert_eq!(links.length(), 1);
    }

    #[test]
    fn test_convert_tags_preserves_content_links() {
        let html = r##"<div><p>Text <a href="#">P Link</a></p><ul><li><a href="#">List Link</a></li></ul></div>"##;
        let doc = dom::parse(html);
        let div = doc.select("div");
        let mut opts = Options::default();
        opts.include_links = false;

        convert_tags(&div, &opts);

        // Links in p and ul should be preserved
        assert_eq!(div.select("a").length(), 2);
    }

    #[test]
    fn test_convert_tags_makes_urls_absolute() {
        let html = r#"<div><a href="/page">Link</a></div>"#;
        let doc = dom::parse(html);
        let div = doc.select("div");
        let mut opts = Options::default();
        opts.include_links = true;
        opts.url = Some("https://example.com/article/".to_string());

        convert_tags(&div, &opts);

        let href = dom::get_attribute(&div.select("a"), "href");
        assert_eq!(href, Some("https://example.com/page".to_string()));
    }

    #[test]
    fn test_convert_tags_code_detection_hljs() {
        let html = r#"<pre><span class="hljs-keyword">let</span> x = 1;</pre>"#;
        let doc = dom::parse(html);
        let root = doc.select("body");
        let opts = Options::default();

        convert_tags(&root, &opts);

        // pre with hljs should become code
        assert_eq!(doc.select("code").length(), 1);
        assert_eq!(doc.select("pre").length(), 0);
    }

    #[test]
    fn test_convert_tags_code_detection_single_span() {
        let html = r#"<pre><span>code content</span></pre>"#;
        let doc = dom::parse(html);
        let root = doc.select("body");
        let opts = Options::default();

        convert_tags(&root, &opts);

        // pre with single span child should become code
        assert_eq!(doc.select("code").length(), 1);
        assert_eq!(doc.select("pre").length(), 0);
    }

    #[test]
    fn test_convert_tags_no_code_detection_for_normal_pre() {
        let html = r#"<pre>plain text content</pre>"#;
        let doc = dom::parse(html);
        let root = doc.select("body");
        let opts = Options::default();

        convert_tags(&root, &opts);

        // Plain pre without code indicators should stay as pre
        assert_eq!(doc.select("pre").length(), 1);
        assert_eq!(doc.select("code").length(), 0);
    }

    #[test]
    fn test_duplicate_test_short_text() {
        let doc = dom::parse("<p>Short</p>");
        let p = doc.select("p");
        let mut cache = LruCache::new(100);

        // Short text should never be considered duplicate
        assert!(!duplicate_test(&p, &mut cache));
        assert!(!duplicate_test(&p, &mut cache));
        assert!(!duplicate_test(&p, &mut cache));
    }

    #[test]
    fn test_duplicate_test_long_text_threshold() {
        let doc = dom::parse("<p>This is a much longer text that exceeds the minimum duplicate check size threshold of one hundred characters for proper testing.</p>");
        let p = doc.select("p");
        let mut cache = LruCache::new(100);

        // First 3 occurrences should pass (count <= MAX_DUPLICATE_COUNT)
        assert!(!duplicate_test(&p, &mut cache)); // count becomes 1
        assert!(!duplicate_test(&p, &mut cache)); // count becomes 2
        assert!(!duplicate_test(&p, &mut cache)); // count becomes 3, but check is count > 2, so still false

        // Fourth occurrence should be duplicate (count=3 > MAX_DUPLICATE_COUNT=2)
        assert!(duplicate_test(&p, &mut cache)); // count becomes 4, check is 4 > 2 = true
    }

    #[test]
    fn test_clear_all_attributes() {
        let doc = dom::parse(r#"<p class="article" id="main" data-test="value">Text</p>"#);
        let p = doc.select("p");

        clear_all_attributes(&p);

        assert!(dom::get_attribute(&p, "class").is_none());
        assert!(dom::get_attribute(&p, "id").is_none());
        assert!(dom::get_attribute(&p, "data-test").is_none());
    }
}
