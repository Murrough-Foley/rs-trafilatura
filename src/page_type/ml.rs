//! ML-based page type classification.
//!
//! Feature extraction from HTML documents (requires DOM access) and delegation
//! to the web-page-classifier crate for model evaluation.

use dom_query::Selection;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

#[allow(clippy::expect_used)]
static PRODUCT_COUNT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\d+\s*(results|items|products|pieces)").expect("valid regex")
});

use crate::dom::Document;
use crate::result::Metadata;

use super::PageType;

/// Number of numeric features extracted from the document.
pub const N_NUMERIC: usize = web_page_classifier::N_NUMERIC_FEATURES; // 81

/// Classify page type using the ML model.
///
/// Takes pre-extracted numeric features and title/description text.
/// Delegates to the web-page-classifier crate for model evaluation.
#[must_use]
pub fn classify_ml(numeric_features: &[f64; N_NUMERIC], title_meta: &str) -> (PageType, f64) {
    let (crate_type, confidence) = web_page_classifier::classify_ml(numeric_features, title_meta);

    // Map from crate's PageType to our internal PageType
    let page_type = match crate_type {
        web_page_classifier::PageType::Article => PageType::Article,
        web_page_classifier::PageType::Collection => PageType::Category,
        web_page_classifier::PageType::Documentation => PageType::Documentation,
        web_page_classifier::PageType::Forum => PageType::Forum,
        web_page_classifier::PageType::Listing => PageType::Listing,
        web_page_classifier::PageType::Product => PageType::Product,
        web_page_classifier::PageType::Service => PageType::Service,
    };

    (page_type, confidence)
}

/// Extract numeric features from the HTML document for ML classification.
///
/// Returns 81 features:
/// - f[0..14]: URL pattern signals
/// - f[14..63]: HTML structural signals (paragraphs, headings, JSON-LD, etc.)
/// - f[63..73]: Enhanced structural features (repeated siblings, prices, etc.)
/// - f[73..81]: DOM vocabulary features (commercial/content/tech/forum density)
#[must_use]
pub fn extract_ml_features(doc: &Document, metadata: &Metadata, url: &str) -> [f64; N_NUMERIC] {
    let mut f = [0.0f64; N_NUMERIC];

    let url_lower = url.to_ascii_lowercase();
    let (domain, path) = super::extract_domain_path(&url_lower);

    // === f[0..14]: URL pattern features ===
    f[0] = if super::contains_any(domain, super::FORUM_DOMAINS) { 1.0 } else { 0.0 };
    f[1] = if super::contains_any(path, super::FORUM_PATHS) { 1.0 } else { 0.0 };
    f[2] = if super::contains_any(&url_lower, super::FORUM_URL_PATTERNS) { 1.0 } else { 0.0 };
    f[3] = if super::contains_any(domain, super::DOCS_DOMAINS) { 1.0 } else { 0.0 };
    f[4] = if super::contains_any(path, super::DOCS_PATHS) { 1.0 } else { 0.0 };
    f[5] = if super::contains_any(path, super::PRODUCT_PATHS) { 1.0 } else { 0.0 };
    f[6] = if super::contains_any(path, super::CATEGORY_PATHS) { 1.0 } else { 0.0 };
    f[7] = if super::contains_any(path, super::SERVICE_PATHS) { 1.0 } else { 0.0 };
    f[8] = if super::contains_any(&url_lower, super::SERVICE_SLUG_PATTERNS) { 1.0 } else { 0.0 };
    f[9] = if super::contains_any(path, super::ARTICLE_PATHS) { 1.0 } else { 0.0 };
    f[10] = if super::contains_any(&url_lower, super::BLOG_SLUG_PATTERNS) { 1.0 } else { 0.0 };
    let path_trimmed = path.trim_end_matches('/');
    f[11] = if super::LISTING_PATH_ENDINGS.iter().any(|p| path_trimmed.ends_with(p)) { 1.0 } else { 0.0 };
    f[12] = if super::contains_any(path, super::LISTING_PATH_CONTAINS) { 1.0 } else { 0.0 };
    f[13] = if domain.contains("shop.") || domain.contains("store.") { 1.0 } else { 0.0 };

    // === f[14..63]: HTML structural features ===

    // Paragraph stats
    let mut p_count = 0u32;
    let mut p_total_len = 0u32;
    for node in doc.select("p").nodes() {
        let sel = Selection::from(*node);
        let text = sel.text();
        let trimmed = text.trim();
        if trimmed.len() > 20 {
            p_count += 1;
            p_total_len += trimmed.len() as u32;
        }
    }
    f[14] = p_count as f64;
    f[15] = if p_count > 0 { p_total_len as f64 / p_count as f64 } else { 0.0 };
    f[16] = doc.select("h1, h2, h3, h4, h5, h6").length() as f64;
    let h2_count = doc.select("h2").length();
    let body_text_full = doc.select("body").text().to_string();
    let body_text_len = body_text_full.len();
    f[17] = if h2_count > 0 { body_text_len as f64 / h2_count as f64 } else { 0.0 };
    f[18] = if doc.select("article").length() > 0 { 1.0 } else { 0.0 };
    f[19] = if doc.select("time").length() > 0 { 1.0 } else { 0.0 };
    f[20] = if doc.select("main").length() > 0 { 1.0 } else { 0.0 };
    f[21] = if doc.select("aside").length() > 0 { 1.0 } else { 0.0 };
    f[22] = if doc.select(r#"meta[name="author"], meta[property="article:author"], [class*="author"]"#).length() > 0 { 1.0 } else { 0.0 };

    // JSON-LD signals
    for node in doc.select(r#"script[type="application/ld+json"]"#).nodes() {
        let sel = Selection::from(*node);
        let text = sel.text();
        if text.contains(r#""Article""#) || text.contains(r#""NewsArticle""#) || text.contains(r#""BlogPosting""#) { f[23] = 1.0; }
        if text.contains(r#""Product""#) { f[24] = 1.0; }
        if text.contains(r#""FAQPage""#) { f[25] = 1.0; }
        if text.contains(r#""CollectionPage""#) || text.contains(r#""OfferCatalog""#) { f[26] = 1.0; }
        if text.contains(r#""ItemList""#) { f[27] = 1.0; }
        if text.contains(r#""LocalBusiness""#) { f[28] = 1.0; }
        if text.contains(r#""Service""#) { f[29] = 1.0; }
        if text.contains(r#""AggregateOffer""#) { f[30] = 1.0; }
    }

    let og_type = metadata.page_type.as_deref().unwrap_or("").to_ascii_lowercase();
    f[31] = if og_type.contains("product") { 1.0 } else { 0.0 };
    f[32] = if og_type == "article" { 1.0 } else { 0.0 };
    f[33] = if og_type == "website" { 1.0 } else { 0.0 };
    f[34] = if doc.select("[class*='product-grid'], [class*='product-list'], [class*='product-card']").length() > 0 { 1.0 } else { 0.0 };
    f[35] = if doc.select("[class*='add-to-cart'], [class*='addtocart'], [class*='buy-now']").length() > 0 { 1.0 } else { 0.0 };
    f[36] = doc.select("[class*='product-card'], [class*='product-tile'], [class*='product-item']").length() as f64;
    f[37] = if doc.select("link[rel='next'], [class*='pagination'], [class*='pager']").length() > 0 { 1.0 } else { 0.0 };
    f[38] = doc.select("code, pre").length() as f64;
    f[39] = if doc.select("[class*='docs-sidebar'], [class*='doc-sidebar'], [class*='docs-nav'], [class*='table-of-contents']").length() > 0 { 1.0 } else { 0.0 };

    let link_count = doc.select("a").length();
    let p_text = doc.select("p").text();
    let p_words = p_text.split_whitespace().count();
    f[40] = if p_words > 0 { link_count as f64 / p_words as f64 } else { 0.0 };
    f[41] = p_words as f64;
    f[42] = doc.select("[class*='grid'], [class*='col-'], [class*='column'], [class*='card']").length() as f64;
    f[43] = doc.select("svg").length() as f64;

    let mut cta_count = 0u32;
    for node in doc.select("button, a").nodes() {
        let sel = Selection::from(*node);
        let text = sel.text().to_ascii_lowercase();
        if text.contains("get started") || text.contains("free trial") || text.contains("contact us")
            || text.contains("sign up") || text.contains("try free") || text.contains("get pricing")
            || text.contains("book a") || text.contains("schedule")
        {
            cta_count += 1;
        }
    }
    f[44] = cta_count as f64;
    f[45] = if doc.select("[class*='hero']").length() > 0 { 1.0 } else { 0.0 };
    f[46] = if doc.select("[class*='testimonial']").length() > 0 { 1.0 } else { 0.0 };
    f[47] = if doc.select("[class*='pricing']").length() > 0 { 1.0 } else { 0.0 };
    f[48] = if doc.select("[class*='feature']").length() > 0 { 1.0 } else { 0.0 };
    f[49] = if doc.select("[class*='breadcrumb']").length() > 0 { 1.0 } else { 0.0 };
    f[50] = doc.select("form").length() as f64;
    f[51] = doc.select("img").length() as f64;
    f[52] = doc.select("ul, ol").length() as f64;
    f[53] = doc.select("table").length() as f64;
    f[54] = doc.select("nav").length() as f64;
    f[55] = doc.select("section").length() as f64;
    f[56] = doc.select("button").length() as f64;
    f[57] = doc.select("input").length() as f64;
    f[58] = body_text_len as f64;

    let mut link_hrefs = HashSet::new();
    for node in doc.select("a[href]").nodes() {
        let sel = Selection::from(*node);
        if let Some(href) = sel.attr("href") {
            link_hrefs.insert(href.to_string());
        }
    }
    f[59] = link_hrefs.len() as f64;
    f[60] = doc.select("[class*='comment']").length() as f64;
    f[61] = doc.select("[class*='post']").length() as f64;
    f[62] = doc.select("[class*='message']").length() as f64;

    // === f[63..73]: Enhanced structural features ===
    // Skip expensive features on very large documents (>500KB text)
    if body_text_len > 500_000 {
        return f;
    }

    // Repeated sibling structure (listing fingerprint)
    let mut max_repeated_class = 0u32;
    let mut parents_with_repeats = 0u32;
    for node in doc.select("body > *, body > * > *, body > * > * > *").nodes() {
        let sel = Selection::from(*node);
        let children = sel.children();
        if children.length() < 3 {
            continue;
        }
        let mut class_counts: HashMap<String, u32> = HashMap::new();
        for child_node in children.nodes() {
            let child = Selection::from(*child_node);
            if let Some(cls) = child.attr("class") {
                *class_counts.entry(cls.to_string()).or_insert(0) += 1;
            }
        }
        if let Some(&max_count) = class_counts.values().max() {
            if max_count >= 3 {
                parents_with_repeats += 1;
                max_repeated_class = max_repeated_class.max(max_count);
            }
        }
    }
    f[63] = max_repeated_class as f64;
    f[64] = parents_with_repeats as f64;

    // Price pattern count (reuse body_text_full from above)
    let body_text_str = &body_text_full;
    let price_count = body_text_str.matches('$').count()
        + body_text_str.matches('€').count()
        + body_text_str.matches('£').count();
    f[65] = price_count as f64;

    // Image-to-text ratio (reuse img count from f[51])
    let img_count = f[51] as usize;
    f[66] = if body_text_len > 0 { img_count as f64 / (body_text_len as f64 / 1000.0) } else { 0.0 };

    // Heading hierarchy breadth ratio
    let mut heading_level_counts = [0u32; 6];
    for node in doc.select("h1, h2, h3, h4, h5, h6").nodes() {
        let sel = Selection::from(*node);
        if let Some(name) = sel.nodes().first().and_then(|n| n.node_name()) {
            if let Some(level) = name.chars().nth(1).and_then(|c| c.to_digit(10)) {
                if level >= 1 && level <= 6 {
                    heading_level_counts[(level - 1) as usize] += 1;
                }
            }
        }
    }
    let max_same_level = heading_level_counts.iter().max().copied().unwrap_or(0);
    let n_levels_used = heading_level_counts.iter().filter(|&&c| c > 0).count();
    f[67] = if n_levels_used > 0 { max_same_level as f64 / n_levels_used as f64 } else { 0.0 };

    // BreadcrumbList schema (check lowercased body text, computed once below)
    let body_lower = body_text_str.to_ascii_lowercase();
    f[68] = if body_lower.contains("breadcrumblist") { 1.0 } else { 0.0 };

    // Repeated link texts
    let mut link_text_counts: HashMap<String, u32> = HashMap::new();
    for node in doc.select("a").nodes() {
        let sel = Selection::from(*node);
        let text = sel.text().trim().to_ascii_lowercase();
        if text.len() > 3 {
            *link_text_counts.entry(text).or_insert(0) += 1;
        }
    }
    let repeated_link_texts = link_text_counts.values().filter(|&&c| c >= 3).count();
    f[69] = repeated_link_texts as f64;

    // f[70]: Section link density variance
    // Track link count and text length per section/article/div boundary.
    // Listings have uniform link density; articles vary.
    {
        let mut section_ratios: Vec<f64> = Vec::new();
        let mut current_links = 0u32;
        let mut current_text_len = 0u32;

        for node in doc.select("section, article, div").nodes() {
            // Flush previous section
            if current_text_len > 50 {
                section_ratios.push(current_links as f64 / current_text_len as f64 * 1000.0);
            }
            current_links = 0;
            current_text_len = 0;

            let sel = Selection::from(*node);
            current_links = sel.select("a").length() as u32;
            let text = sel.text();
            current_text_len = text.trim().len() as u32;
        }
        if current_text_len > 50 {
            section_ratios.push(current_links as f64 / current_text_len as f64 * 1000.0);
        }

        if section_ratios.len() >= 3 {
            let mean = section_ratios.iter().sum::<f64>() / section_ratios.len() as f64;
            let var = section_ratios.iter().map(|&r| (r - mean).powi(2)).sum::<f64>()
                / section_ratios.len() as f64;
            f[70] = var;
        }
    }

    // Meta robots noindex
    f[71] = if doc.select(r#"meta[name="robots"][content*="noindex"]"#).length() > 0 { 1.0 } else { 0.0 };

    // URL path depth
    let path_segments = path.trim_matches('/').split('/').filter(|s| !s.is_empty()).count();
    f[72] = path_segments as f64;

    // === f[73..81]: DOM vocabulary features ===

    // f[73-74]: DOM subtree structure hashing (tag + semantic class keywords)
    // Different from f[63-64] which uses raw class strings.
    // This builds structural signatures: "div|item", "li|product", etc.
    {
        let mut dom_max_sig = 0u32;
        let mut dom_parents_with_repeats = 0u32;

        // Iterate shallow DOM (depth ≤ 4 approximated by 3-level selector)
        for node in doc.select("body > *, body > * > *, body > * > * > *").nodes() {
            let sel = Selection::from(*node);
            let children = sel.children();
            if children.length() < 3 {
                continue;
            }
            let mut sig_counts: HashMap<String, u32> = HashMap::new();
            for child_node in children.nodes() {
                let child = Selection::from(*child_node);
                let tag = child.nodes().first()
                    .and_then(|n| n.node_name())
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if tag.is_empty() {
                    continue;
                }
                // Build structural signature: tag + semantic keyword from class
                let cls = child.attr("class").unwrap_or_default().to_ascii_lowercase();
                let keyword = ["item", "card", "product", "post", "entry", "result", "row", "cell"]
                    .iter()
                    .find(|&&kw| cls.contains(kw))
                    .unwrap_or(&"");
                let sig = if keyword.is_empty() {
                    tag.to_string()
                } else {
                    format!("{tag}|{keyword}")
                };
                *sig_counts.entry(sig).or_insert(0) += 1;
            }
            if let Some(&top) = sig_counts.values().max() {
                if top >= 3 {
                    dom_parents_with_repeats += 1;
                    dom_max_sig = dom_max_sig.max(top);
                }
            }
        }
        f[73] = dom_max_sig as f64;
        f[74] = dom_parents_with_repeats as f64;
    }

    // Reuse body_lower from breadcrumb check above
    let body_words: Vec<&str> = body_lower.split_whitespace().collect();
    let total_words = body_words.len();

    if total_words > 0 {
        let mut word_counts: HashMap<&str, u32> = HashMap::new();
        for &word in &body_words {
            *word_counts.entry(word).or_insert(0) += 1;
        }

        // f[75]: Commercial vocabulary density
        let commercial = ["price", "buy", "cart", "shop", "order", "shipping",
            "delivery", "stock", "sale", "discount", "offer", "deal",
            "checkout", "payment", "warranty", "returns", "refund"];
        let commercial_sum: u32 = commercial.iter().map(|w| word_counts.get(w).copied().unwrap_or(0)).sum();
        f[75] = commercial_sum as f64 / total_words as f64;

        // f[76]: Content vocabulary density
        let content = ["posted", "author", "published", "updated", "comments",
            "share", "tweet", "read", "article", "blog", "opinion",
            "editor", "journalist", "source", "according"];
        let content_sum: u32 = content.iter().map(|w| word_counts.get(w).copied().unwrap_or(0)).sum();
        f[76] = content_sum as f64 / total_words as f64;

        // f[77]: Tech vocabulary density
        let tech = ["api", "function", "parameter", "returns", "example",
            "syntax", "reference", "deprecated", "version", "module",
            "class", "method", "interface", "configuration", "install"];
        let tech_sum: u32 = tech.iter().map(|w| word_counts.get(w).copied().unwrap_or(0)).sum();
        f[77] = tech_sum as f64 / total_words as f64;

        // f[78]: Forum vocabulary density
        let forum = ["reply", "thread", "post", "member", "joined", "reputation",
            "moderator", "admin", "quote", "likes", "views", "topic",
            "answered", "solution", "vote", "upvote"];
        let forum_sum: u32 = forum.iter().map(|w| word_counts.get(w).copied().unwrap_or(0)).sum();
        f[78] = forum_sum as f64 / total_words as f64;
    }

    // f[79]: Max frequency of any single repeated link text
    // f[80]: Count of link texts appearing ≥3 times
    // These use link texts collected from <a> elements (same as f[69] collection
    // but f[79] is max() while f[69]/f[80] are count(≥3))
    let max_link_repeat = link_text_counts.values().max().copied().unwrap_or(0);
    f[79] = max_link_repeat as f64;
    f[80] = link_text_counts.values().filter(|&&c| c >= 3).count() as f64;

    // === f[81..89]: Collection-specific features ===

    // f[81]: og:type = product.group
    f[81] = if doc.select(r#"meta[property="og:type"][content*="product.group"]"#).length() > 0 { 1.0 } else { 0.0 };

    // f[82]: Has filter sidebar
    f[82] = if doc.select("[class*='filter'][class*='sidebar'], [class*='filter'][class*='panel'], [class*='filter'][class*='bar'], [class*='filter'][class*='menu']").length() > 0 { 1.0 } else { 0.0 };

    // f[83]: Has sort control
    f[83] = if doc.select("[class*='sort'][class*='select'], [class*='sort'][class*='dropdown'], [class*='sort'][class*='control'], [class*='sort'][class*='option']").length() > 0 { 1.0 } else { 0.0 };

    // f[84]: Has product count text ("showing X results", "X items")
    f[84] = if PRODUCT_COUNT_RE.is_match(&body_lower) { 1.0 } else { 0.0 };

    // f[85]: Product cards with price (cards that have both product-class and price-class)
    let mut cards_with_price = 0u32;
    let card_selector = "[class*='product-card'], [class*='product-tile'], [class*='product-item'], [class*='product-grid-item'], [class*='grid-item'], [class*='collection-item']";
    let total_cards = doc.select(card_selector).length();
    for node in doc.select(card_selector).nodes() {
        let sel = Selection::from(*node);
        if sel.select("[class*='price'], [class*='cost'], [class*='amount']").length() > 0 {
            cards_with_price += 1;
        }
    }
    f[85] = cards_with_price as f64;

    // f[86]: Has CollectionPage schema
    f[86] = if body_lower.contains("collectionpage") || body_lower.contains("productcollection") { 1.0 } else { 0.0 };

    // f[87]: Total card count
    f[87] = total_cards as f64;

    // f[88]: Price-to-card ratio
    f[88] = if total_cards > 0 { cards_with_price as f64 / total_cards as f64 } else { 0.0 };

    f
}
