//! Page type classification for web content extraction.
//!
//! Classifies web pages into content types using a two-stage approach:
//!
//! 1. **URL-based heuristics** — fast pattern matching on URL path and domain
//! 2. **HTML-based signals** — structured data (JSON-LD, OpenGraph) for ambiguous URLs
//!
//! The detected page type can be used to select extraction strategies optimized
//! for each content type.
//!
//! # Example
//!
//! ```rust
//! use rs_trafilatura::page_type::{PageType, classify_url};
//!
//! let page_type = classify_url("https://shop.example.com/products/widget-123");
//! assert_eq!(page_type, PageType::Product);
//!
//! let page_type = classify_url("https://community.example.com/t/help-me/12345");
//! assert_eq!(page_type, PageType::Forum);
//! ```

/// The type of content on a web page.
///
/// Each variant represents a distinct content structure that may benefit
/// from different extraction strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageType {
    /// Blog posts, news articles, editorials, guides, reviews.
    /// The most common page type and the default fallback.
    Article,

    /// Discussion threads, Q&A pages, community posts.
    /// Characterized by multiple user-contributed posts in sequence.
    Forum,

    /// Individual product pages with descriptions, specs, pricing.
    /// Typically has structured data (JSON-LD Product, og:type product).
    Product,

    /// Product listings, collections, category browse pages.
    /// Contains grids/lists of products with minimal per-item detail.
    Category,

    /// Content index pages: news feeds, course catalogs, review lists,
    /// testimonial pages, award listings. Lists of content items rather
    /// than products. Uses article-like extraction since content is text-based.
    Listing,

    /// Technical documentation, API references, tutorials, wikis, man pages.
    /// Usually has code blocks, structured navigation, versioned content.
    Documentation,

    /// SaaS feature pages, service descriptions, solution pages.
    /// Mix of marketing copy, feature lists, CTAs, and testimonials.
    Service,
}

impl PageType {
    /// Returns the string name of this page type.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Article => "article",
            Self::Forum => "forum",
            Self::Product => "product",
            Self::Category => "collection",
            Self::Listing => "listing",
            Self::Documentation => "documentation",
            Self::Service => "service",
        }
    }

    /// Returns the extraction profile for this page type.
    ///
    /// The profile configures how the extraction pipeline behaves for
    /// this type of content — which elements to preserve, how aggressively
    /// to filter boilerplate, fallback thresholds, etc.
    #[must_use]
    pub(crate) fn extraction_profile(&self) -> ExtractionProfile {
        match self {
            Self::Article => ExtractionProfile::ARTICLE,
            Self::Forum => ExtractionProfile::FORUM,
            Self::Product => ExtractionProfile::PRODUCT,
            Self::Category => ExtractionProfile::CATEGORY,
            Self::Listing => ExtractionProfile::LISTING,
            Self::Documentation => ExtractionProfile::DOCUMENTATION,
            Self::Service => ExtractionProfile::SERVICE,
        }
    }
}

/// Extraction configuration for a specific page type.
///
/// Controls how the extraction pipeline behaves — which elements to keep,
/// how strictly to filter boilerplate, and fallback thresholds. Each page
/// type has a static profile accessed via [`PageType::extraction_profile`].
#[derive(Debug, Clone)]
pub(crate) struct ExtractionProfile {
    /// Whether elements with comment-like classes are considered content.
    ///
    /// `true` for forums (comments ARE the content), `false` for articles
    /// (comments are supplementary).
    pub comments_are_content: bool,

    /// Whether to apply lenient boilerplate filtering.
    ///
    /// `true` skips some boilerplate class checks that would otherwise
    /// remove valid content (e.g., "related" sections on product pages).
    pub lenient_boilerplate: bool,

    /// Additional CSS selectors to try when finding the main content node.
    ///
    /// Checked before the default article/main/section selectors.
    pub content_selectors: &'static [&'static str],

    /// Tags that should be preserved during doc_cleaning (not removed).
    ///
    /// Overrides TAGS_TO_CLEAN for this page type. E.g., documentation
    /// pages may preserve `<nav>` for sidebar navigation.
    pub preserve_tags: &'static [&'static str],

    /// Minimum paragraph density for fallback usability (0.0 - 1.0).
    ///
    /// Lower values accept content with fewer `<p>` tags (e.g., documentation
    /// with lots of code blocks, or products with spec tables).
    pub min_paragraph_density: f64,

    /// CSS selectors for boilerplate elements specific to this page type.
    ///
    /// These elements are removed during doc_cleaning in addition to the
    /// standard boilerplate removal. E.g., forum user info panels, vote
    /// controls, action bars.
    pub boilerplate_selectors: &'static [&'static str],

    /// Whether to aggregate content across multiple sections.
    ///
    /// When `true`, the extractor collects text from all content-bearing
    /// `<section>` elements within the page wrapper, rather than picking
    /// a single best content node. This handles service/marketing pages
    /// where content is distributed across many independent sections
    /// (hero, features, testimonials, pricing, FAQ, etc.).
    pub aggregate_sections: bool,

    /// Whether to try repeated-element extraction.
    ///
    /// When `true` and single-node extraction is under-extracted, look for
    /// repeated sibling elements (article cards, list items) within the
    /// content area and concatenate them. This handles listing/index pages
    /// (news feeds, course catalogs, review lists) where content is in
    /// 10-50 repeated card structures.
    pub collect_repeated_items: bool,
}

impl ExtractionProfile {
    /// Article extraction: standard behavior, strict boilerplate filtering.
    /// aggregate_sections enabled as fallback for long listicle articles
    /// where content is distributed across multiple sections.
    const ARTICLE: Self = Self {
        comments_are_content: false,
        lenient_boilerplate: false,
        content_selectors: &[],
        preserve_tags: &[],
        min_paragraph_density: 0.4,
        boilerplate_selectors: &[],
        aggregate_sections: true,
        collect_repeated_items: false,
    };

    /// Forum extraction: comments are content, relax boilerplate filtering.
    const FORUM: Self = Self {
        comments_are_content: true,
        lenient_boilerplate: true,
        content_selectors: &[
            // Discourse (crawler view)
            "div[itemscope][itemtype='http://schema.org/DiscussionForumPosting']",
            // StackExchange
            "#mainbar",
            // XenForo 2
            "div.block--messages",
            // XenForo 1
            "ol.messageList",
            // IPS/Invision Community
            "div.cTopic",
            // Hacker News
            "table.comment-tree",
            // phpBB
            "#page-body",
            // Lemmy
            "#postContent",
            // Slashdot
            "ul#commentlisting",
            // Reddit (old)
            "div.commentarea",
            // TheStudentRoom / custom forums
            ".thread-content",
            ".topic-body",
            ".post-container",
            // Cruise Critic / IPS variant
            "[data-controller='topic']",
            // vBulletin
            "#posts",
            // Generic
            "[role='main']",
        ],
        // XenForo 1 wraps entire thread in a <form> for inline moderation.
        // Without preserving form, the thread content gets removed by doc_cleaning.
        preserve_tags: &["form"],
        min_paragraph_density: 0.2,
        boilerplate_selectors: FORUM_BOILERPLATE_SELECTORS,
        aggregate_sections: false,
        collect_repeated_items: false,
    };

    /// Product extraction: product-specific selectors, keep spec tables, relax paragraph density.
    const PRODUCT: Self = Self {
        comments_are_content: false,
        lenient_boilerplate: true,
        content_selectors: &[
            // Structured product containers
            "[itemtype*='schema.org/Product']",
            "[itemtype*='schema.org/SoftwareApplication']",
            // Common product page selectors
            ".product-page",
            ".product-detail",
            ".product-description",
            ".product-content",
            ".product-info",
            ".pdp-main",
            ".pdp-content",
            "#product-description",
            "#productDescription",
            "#descriptionAndDetails",
            ".item-description",
            "#item-description",
            // itemprop-based (schema.org microdata)
            "[itemprop='description']",
            // Game stores (Steam, Epic, GOG)
            ".game_description_snippet",
            ".game_area_description",
            "#game_area_description",
            // eBay
            "#desc_ifr",
            "#viTabs_0_is",
            ".x-item-description",
            // REI / outdoor brands
            "[class*='buy-box-product-description']",
            // Shopify themes
            ".product__description",
            ".product-single__description",
            // Prose/content blocks (iFixit, documentation-style product pages)
            ".prose",
            ".rich-text",
            ".rte",
            // Generic
            "[role='main']",
            "main",
        ],
        preserve_tags: &[],
        min_paragraph_density: 0.2,
        boilerplate_selectors: PRODUCT_BOILERPLATE_SELECTORS,
        aggregate_sections: true,
        collect_repeated_items: false,
    };

    /// Category/collection extraction: standard filtering.
    const CATEGORY: Self = Self {
        comments_are_content: false,
        lenient_boilerplate: false,
        content_selectors: &[],
        preserve_tags: &[],
        min_paragraph_density: 0.3,
        boilerplate_selectors: &[],
        aggregate_sections: false,
        collect_repeated_items: false,
    };

    /// Listing extraction: content indexes with repeated item structures.
    const LISTING: Self = Self {
        comments_are_content: false,
        lenient_boilerplate: false,
        content_selectors: &[],
        preserve_tags: &[],
        min_paragraph_density: 0.3,
        boilerplate_selectors: &[],
        aggregate_sections: false,
        collect_repeated_items: true,
    };

    /// Documentation extraction: preserve code blocks and nav, relax density.
    const DOCUMENTATION: Self = Self {
        comments_are_content: false,
        lenient_boilerplate: false,
        content_selectors: &[
            // Sphinx (Python ecosystem: Django, Flask, Celery, Requests, etc.)
            "div.body",
            // Django custom docs
            "main#main-content > article",
            // PostgreSQL
            "#docContent",
            // Git-scm (Pro Git book)
            "#main",
            // Go docs
            "article.Doc",
            // Kubernetes / Docsy theme
            ".td-content",
            // MDN Web Docs
            "article.main-page-content",
            // Arch Wiki / MediaWiki
            "#mw-content-text",
            ".mw-parser-output",
            // Tailwind CSS docs
            "#content-wrapper",
            // MkDocs / ReadTheDocs
            "[role='main']",
            // Docusaurus
            "article[role='main']",
            ".markdown",
            // Generic doc patterns
            ".docs-content",
            ".guide-body",
            ".wiki-content",
            ".api-reference",
            ".markdown-body",
        ],
        preserve_tags: &[],
        min_paragraph_density: 0.2,
        boilerplate_selectors: DOC_BOILERPLATE_SELECTORS,
        aggregate_sections: false,
        collect_repeated_items: false,
    };

    /// Service extraction: aggregate sections as fallback for under-extraction.
    /// Otherwise identical to Article to avoid regressions on well-extracted pages.
    const SERVICE: Self = Self {
        comments_are_content: false,
        lenient_boilerplate: false,
        content_selectors: &[],
        preserve_tags: &[],
        min_paragraph_density: 0.4,
        boilerplate_selectors: &[],
        aggregate_sections: true,
        collect_repeated_items: false,
    };
}

impl std::fmt::Display for PageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for PageType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "article" => Ok(Self::Article),
            "forum" => Ok(Self::Forum),
            "product" => Ok(Self::Product),
            "category" | "collection" => Ok(Self::Category),
            "listing" => Ok(Self::Listing),
            "documentation" | "docs" => Ok(Self::Documentation),
            "service" => Ok(Self::Service),
            _ => Err(format!("unknown page type: {s}")),
        }
    }
}

// ---------------------------------------------------------------------------
// Stage 1: URL-based classification
// ---------------------------------------------------------------------------

/// Forum indicators in the domain (e.g. "community.example.com").
const FORUM_DOMAINS: &[&str] = &[
    "forum.",
    "forums.",
    "community.",
    "discuss.",
    "discussion.",
    "users.", // users.rust-lang.org
    "bbs.",   // bbs.archlinux.org
    // Well-known forum sites
    "reddit.com",
    "stackoverflow.com",
    "stackexchange.com",
    "gamefaqs.",
    "discourse.",
    "news.ycombinator.com",
    "quora.com",
    "lemmy.",
    "tapatalk.com",
    "webhostingtalk.com",
    "netmums.com",
    "mumsnet.com",
    "nairaland.com",
    "lobste.rs",
];

/// Forum indicators in the URL path.
const FORUM_PATHS: &[&str] = &[
    "/forum",
    "/forums/",
    "/thread/",
    "/threads/",
    "/topic/",
    "/topics/",
    "/discussion/",
    "/discussions/",
    "/community/",
    "/t/", // Discourse
    "/questions/",
    "/question/",
    "/comments/",
    "/talk/", // Mumsnet
];

/// Forum indicators matched against the full URL.
const FORUM_URL_PATTERNS: &[&str] = &[
    "/viewtopic.php", // phpBB
    "/showthread.php", // vBulletin
    "/item?id=",       // Hacker News
];

/// Documentation indicators in the domain.
const DOCS_DOMAINS: &[&str] = &[
    "docs.",
    "doc.",
    "wiki.",
    "devdocs.",
    "man7.org",
    "readthedocs.io",
    "readthedocs.org",
    "developer.hashicorp.com",
    "developer.mozilla.org",
];

/// Documentation indicators in the URL path.
const DOCS_PATHS: &[&str] = &[
    "/docs/",
    "/doc/",
    "/documentation/",
    "/reference/",
    "/api/",
    "/guide/",
    "/tutorial/",
    "/tutorials/",
    "/manual/",
    "/handbook/",
    "/wiki/",
    "/man-pages/",
    "/man/",
    "/concepts/",
    "/userguide/",
    "/quickstart",
    "/getting-started",
    "/book/",
    "/glossary/",
    "/tech_notes/",
];

/// Product page indicators in the URL path.
const PRODUCT_PATHS: &[&str] = &[
    "/products/",
    "/product/",
    "/shop/",  // /shop/item-slug
    "/dp/",    // Amazon
    "/ip/",    // Walmart
];

/// Product page indicators in the domain.
const PRODUCT_DOMAINS: &[&str] = &[
    "shop.",   // shop.example.com
    "store.",  // store.example.com
];

/// Category/collection page indicators in the URL path.
const CATEGORY_PATHS: &[&str] = &[
    "/collections/",
    "/collection/",
    "/categories/",
    "/category/",
    "/browse/",
    "/cat/",          // IKEA-style
    "/subcategory/",
];

/// Service page indicators in the URL path.
const SERVICE_PATHS: &[&str] = &[
    "/services/",
    "/service/",
    "/services.html",
    "/solutions/",
    "/solution/",
    "/offerings/",
    "/what-we-do",
];

/// Service slug patterns matched against the full URL.
/// These are specific enough to avoid false positives on article pages.
const SERVICE_SLUG_PATTERNS: &[&str] = &[
    "-consulting-services",
    "-development-services",
    "-management-services",
    "-support-services",
    "-outsourcing-services",
    "-integration-services",
    "-development-company",
    "-consulting-company",
    "-ai-consulting",
    "-ai-development",
    "-ai-solutions",
];

/// Listing/index page indicators.
/// These match only when the path ENDS with the pattern (no further segments),
/// distinguishing e.g. `/news` (listing) from `/news/some-article` (article).
const LISTING_PATH_ENDINGS: &[&str] = &[
    "/news",
    "/testimonials",
    "/coupons",
    "/issues",
    "/reviews",
    "/rankings",
    "-courses",
];

/// Listing page indicators that match anywhere in the path.
const LISTING_PATH_CONTAINS: &[&str] = &[
    "/awards/",
    "/trending/",
    "/list/",
];

/// Article/blog indicators in the URL path.
const ARTICLE_PATHS: &[&str] = &[
    "/blog/",
    "/blog",
    "/news/",  // /news/ with trailing content = article section
    "/article/",
    "/articles/",
    "/post/",
    "/posts/",
    "/insight/",
    "/insights/",
    "/resource/",
    "/resources/",
    "/stories/",
    "/magazine/",
    "/journal/",
    "/press/",
    "/editorial/",
    "/opinion/",
    "/review/",
    "/column/",
];

/// Blog-like slug patterns matched against the full URL.
const BLOG_SLUG_PATTERNS: &[&str] = &[
    "-ways-to-",
    "-tips-",
    "-reasons-",
    "-steps-to-",
    "-things-to-",
    "-best-",
    "-top-",
    "-essential-",
    "beginners-guide",
    "complete-guide",
    "ultimate-guide",
    "how-to-",
    "what-is-",
    "why-",
    "when-to-",
    "-vs-",
    "-versus-",
    "-comparison",
    "-checklist",
    "-trends-",
    "-strategies-",
    "-challenges-",
    "-benefits-",
    "-advantages-",
];

/// Classify a URL into a page type using heuristic pattern matching.
///
/// This is stage 1 of classification. When the URL is ambiguous (returns
/// `PageType::Article` as default), stage 2 HTML signals can refine the result.
///
/// # Arguments
///
/// * `url` - Full URL or path to classify
///
/// # Returns
///
/// The detected `PageType`. Returns `PageType::Article` when no patterns match,
/// since articles are the most common page type on the web.
#[must_use]
pub fn classify_url(url: &str) -> PageType {
    if url.is_empty() {
        return PageType::Article;
    }

    let url_lower = url.to_ascii_lowercase();

    // Extract domain and path
    let (domain, path) = extract_domain_path(&url_lower);

    // 1. Forum — distinctive domains and path patterns
    if contains_any(domain, FORUM_DOMAINS)
        || contains_any(path, FORUM_PATHS)
        || contains_any(&url_lower, FORUM_URL_PATTERNS)
    {
        return PageType::Forum;
    }

    // 2. Documentation — before article (e.g. /docs/guide/ is docs, not article)
    if contains_any(domain, DOCS_DOMAINS) || contains_any(path, DOCS_PATHS) {
        return PageType::Documentation;
    }

    // 3. Product — before category (/products/slug is a product, not a listing)
    if contains_any(path, PRODUCT_PATHS) || contains_any(domain, PRODUCT_DOMAINS) {
        return PageType::Product;
    }

    // 4. Category / collection
    if contains_any(path, CATEGORY_PATHS) {
        return PageType::Category;
    }

    // 5. Service page
    if contains_any(path, SERVICE_PATHS) || contains_any(&url_lower, SERVICE_SLUG_PATTERNS) {
        return PageType::Service;
    }

    // 6. Listing / content index — path ends with pattern (no further segments)
    {
        let path_trimmed = path.trim_end_matches('/');
        if LISTING_PATH_ENDINGS.iter().any(|p| path_trimmed.ends_with(p))
            || contains_any(path, LISTING_PATH_CONTAINS)
        {
            return PageType::Listing;
        }
    }

    // 7. Article / blog
    if contains_any(path, ARTICLE_PATHS) || contains_any(&url_lower, BLOG_SLUG_PATTERNS) {
        return PageType::Article;
    }

    // 7. Default — article is the most common type
    PageType::Article
}

// ---------------------------------------------------------------------------
// Stage 2: HTML-based classification (for ambiguous URLs)
// ---------------------------------------------------------------------------

/// HTML-level signals extracted from the document head.
///
/// These are cheap to extract (only scan `<head>`) and provide strong
/// signals for product/category pages that have ambiguous URLs.
#[derive(Debug, Default)]
pub(crate) struct HtmlSignals {
    /// Value of `<meta property="og:type" content="...">`.
    pub og_type: Option<String>,

    /// `@type` values found in `<script type="application/ld+json">` blocks.
    pub ld_types: Vec<String>,

    /// Whether JSON-LD Product has AggregateOffer (price range = multiple products).
    /// This indicates a category page, not a single product page.
    pub has_aggregate_offer: bool,

    /// Whether add-to-cart / buy-now patterns were found in the HTML.
    pub has_add_to_cart: bool,

    /// Whether product grid/list CSS classes were found.
    pub has_product_grid: bool,

    /// Number of elements with product-related CSS classes (product-card, etc.).
    /// A high count (5+) suggests a category/listing page rather than a single product.
    pub product_element_count: usize,

    /// Whether pagination elements were found (next/prev links, page numbers).
    /// Combined with product elements, this strongly indicates a category page.
    pub has_pagination: bool,

    /// Number of `<code>` and `<pre>` elements in the document.
    /// High counts strongly indicate documentation or technical content.
    pub code_block_count: usize,

    /// Whether the page has documentation-style navigation (sidebar, TOC).
    pub has_docs_nav: bool,

    /// Ratio of `<a>` link count to paragraph word count.
    /// Listing/index pages have high ratios (many links, few prose words).
    /// Typical articles: median ~0.1. Listings: median ~0.4.
    pub link_ratio: f64,

    /// Total word count inside `<p>` tags.
    pub paragraph_word_count: usize,
}

/// Refine a page type classification using HTML signals.
///
/// This should be called when `classify_url` returns `PageType::Article`
/// (the default/fallback), to check whether the page is actually a
/// product or category page based on structured data in the HTML.
///
/// # Arguments
///
/// * `url_type` - The page type from `classify_url`
/// * `signals` - HTML signals extracted from the document
///
/// # Returns
///
/// The refined `PageType`. Only overrides `Article` — if the URL gave
/// a high-confidence classification, that result is preserved.
/// Minimum number of product-class elements to consider a page a listing/category.
/// Must be combined with at least one other signal (product grid, add-to-cart, or og:type)
/// to avoid false positives on article pages that review multiple products.
const MIN_PRODUCT_ELEMENTS_FOR_CATEGORY: usize = 5;

#[must_use]
pub(crate) fn refine_with_html_signals(url_type: PageType, signals: &HtmlSignals) -> PageType {
    // Only refine when URL classification was ambiguous (defaulted to Article)
    if url_type != PageType::Article {
        return url_type;
    }

    // 1. Check for explicit Category signals in structured data (highest confidence)
    if has_category_signal(signals) {
        return PageType::Category;
    }

    // 2. og:type = "product.group" → Category (not Product)
    if let Some(og) = &signals.og_type {
        let og_lower = og.to_ascii_lowercase();
        if og_lower == "product.group" || og_lower == "product:group" {
            return PageType::Category;
        }
    }

    // 3. Many product elements + supporting signal → Category
    //    Pagination is strongest (review articles never paginate product cards).
    //    Product grid + add-to-cart is secondary (some FPs on review sites, but
    //    catches infinite-scroll category pages that lack pagination markup).
    if signals.product_element_count >= MIN_PRODUCT_ELEMENTS_FOR_CATEGORY
        && (signals.has_pagination || (signals.has_product_grid && signals.has_add_to_cart))
    {
        return PageType::Category;
    }

    // 4. Check for single Product signals
    if has_product_signal(signals) {
        // Product grid + product signal but NOT a single-product LD → Category
        if signals.has_product_grid && !has_single_product_ld(signals) {
            return PageType::Category;
        }
        return PageType::Product;
    }

    // 5. Product grid + add-to-cart without structured data → Category
    if signals.has_product_grid && signals.has_add_to_cart {
        return PageType::Category;
    }

    // 6. Documentation signals — code blocks + doc-style navigation
    //    Many code blocks alone isn't enough (technical blog posts have them too).
    //    Combined with doc-style nav (sidebar, TOC, wiki), it's very reliable.
    if signals.has_docs_nav && signals.code_block_count >= 3 {
        return PageType::Documentation;
    }

    // 7. Very high code block count without nav → still documentation
    //    The highest article page has ~326 code blocks; 500+ is safely docs-only.
    if signals.code_block_count >= 500 {
        return PageType::Documentation;
    }

    // 8. Listing detection via link density ratio.
    //    Only triggers for extreme cases: many links with virtually no prose.
    //    Articles that use <div>/<li> instead of <p> for content can have
    //    misleadingly high ratios, so we require very few paragraph words.
    if signals.link_ratio >= 3.0 && signals.paragraph_word_count < 30 {
        return PageType::Listing;
    }

    PageType::Article
}

/// Check if signals indicate a category/listing page via structured data.
fn has_category_signal(signals: &HtmlSignals) -> bool {
    let has_collection_ld = signals.ld_types.iter().any(|t| {
        t == "CollectionPage" || t == "OfferCatalog" || t == "ProductCollection"
    });

    // CollectionPage/OfferCatalog are specific to e-commerce → high confidence
    if has_collection_ld {
        return true;
    }

    // Product with AggregateOffer = price range = category of products
    let has_product_ld = signals.ld_types.iter().any(|t| t == "Product" || t == "ProductGroup");
    if has_product_ld && signals.has_aggregate_offer {
        return true;
    }

    // ItemList is used by both category pages AND listicle articles (SEO),
    // so only treat it as a category signal when combined with product elements.
    let has_item_list = signals.ld_types.iter().any(|t| t == "ItemList");
    if has_item_list && (signals.has_product_grid || signals.product_element_count >= MIN_PRODUCT_ELEMENTS_FOR_CATEGORY) {
        return true;
    }

    false
}

/// Check if signals indicate a single product page.
fn has_product_signal(signals: &HtmlSignals) -> bool {
    // AggregateOffer means price range = multiple products = category
    if signals.has_aggregate_offer {
        return false;
    }

    // og:type containing "product" (but not "product.group" — handled above)
    if let Some(og) = &signals.og_type {
        let og_lower = og.to_ascii_lowercase();
        if og_lower.contains("product") && og_lower != "product.group" && og_lower != "product:group" {
            return true;
        }
    }

    // JSON-LD @type = "Product" or "ProductGroup"
    signals
        .ld_types
        .iter()
        .any(|t| t == "Product" || t == "ProductGroup")
}

/// Check if JSON-LD contains a single Product (not just BreadcrumbList etc.)
fn has_single_product_ld(signals: &HtmlSignals) -> bool {
    // AggregateOffer means price range = multiple products = category, not single product
    if signals.has_aggregate_offer {
        return false;
    }
    signals
        .ld_types
        .iter()
        .any(|t| t == "Product" || t == "ProductGroup")
}

// ---------------------------------------------------------------------------
// Stage 2: HTML signal extraction
// ---------------------------------------------------------------------------

use crate::dom::{Document, Selection};
use crate::result::Metadata;

/// Forum-specific boilerplate CSS selectors.
///
/// These elements are removed during doc_cleaning for forum pages. They contain
/// user profile panels, vote controls, action bars, AI summaries, and other
/// non-content elements specific to forum platforms.
const FORUM_BOILERPLATE_SELECTORS: &[&str] = &[
    // === XenForo 2 (physicsforums.com, defence.pk, resetera.com) ===
    ".message-cell--user",       // User info panel (avatar, stats, badges)
    ".message-actionBar",        // Reply/report/share action bar
    ".message-attribution",      // Post number, date permalink
    ".message-footer",           // Post footer
    ".message-lastEdit",         // "Last edited" notice
    ".message-userExtras",       // User join date, message count, etc.
    "#ai-summary-block",         // XenForo AI summary plugin
    ".xfa-gptts-block",          // XenForo GPT summary block
    "[class*='ai-summary']",     // Any AI summary element
    ".p-body-sidebar",           // Thread sidebar
    ".p-body-sidebarCol",        // Sidebar column
    ".js-quickReply",            // Quick reply form
    ".block-outer",              // Thread status/page nav wrapper
    // === XenForo 1 (spigotmc.org) ===
    ".messageUserInfo",          // User info block
    ".messageUserBlock",         // User block (avatar, name, stats)
    ".messageDetails",           // Post number, date
    ".dark_postrating",          // Post ratings
    ".extraUserInfo",            // Extended user info
    // === Discourse (openai, docker, rust-lang, obsidian, etc.) ===
    ".crawler-post-meta",        // Post metadata (author, date)
    "[itemprop='interactionStatistic']", // Like counts
    ".post-likes",               // Like display
    "#related-topics",           // Related topics section
    ".more-topics__list",        // More topics list
    // === StackExchange ===
    ".votecell",                 // Vote up/down buttons
    ".post-layout--left",        // Left column (votes)
    ".user-info",                // User card (avatar, rep, badges)
    ".user-gravatar32",          // User avatar
    "#hot-network-questions",    // Hot network questions sidebar
    ".js-post-menu",             // Share/edit/follow/flag menu
    "#post-form",                // Answer form
    ".related",                  // Related questions sidebar
    "#sidebar",                  // Sidebar
    ".comments",                 // SO comment sections under answers
    ".post-signature",           // SO user signature cards
    // === IPS/Invision Community (prestashop, squarespace, linustechtips) ===
    ".ipsComment_author",        // Author panel (avatar, name, stats)
    ".cAuthorPane",              // Author pane
    ".ipsComment_tools",         // Post action tools
    ".ipsComment_meta",          // Post metadata
    ".ipsComment_badges",        // User badges
    ".ipsSideMenu",              // Side menu
    ".ipsWidget",                // Sidebar widgets (Top Posters, etc.)
    "[data-role='replyArea']",   // Reply prompt
    // === Hacker News ===
    ".pagetop",                  // Top navigation bar
    ".yclinks",                  // Footer links
    ".morelink",                 // "More" pagination
    "td.subtext",                // Post score/metadata line
    ".comhead",                  // Comment metadata (user, date, nav links)
    ".votelinks",                // Vote arrows
    "td.ind",                    // Indent spacer images
    ".fatitem .title",           // Post title (already in metadata)
    // === Discourse ===
    "aside.onebox",              // Link preview embeds (duplicated text)
    // === XenForo quote dedup ===
    ".bbCodeBlock--quote",       // Quoted text blocks (causes duplication)
    ".bbCodeBlock--expandable",  // Expandable quote blocks
    // === phpBB ===
    ".postprofile",              // User profile in posts
    "dl.postprofile",            // Alternative user profile selector
    // === Reddit (old) ===
    ".tagline",                  // Comment metadata (user, date, points)
    ".child .midcol",            // Vote arrows in comments
    // === Slashdot ===
    ".commentTop",               // Comment metadata header
    // === Generic forum patterns ===
    ".post-actions",             // Action buttons
    ".post-toolbar",             // Post toolbar
    ".reply-button",             // Reply button
    ".share-button",             // Share button
    ".user-signature",           // User signatures
    ".signature",                // User signatures (alt)
];

/// Boilerplate selectors for product pages.
///
/// Conservative approach: only remove elements that are clearly non-content.
/// Avoid `[class*='...']` patterns that could match content containers
/// (e.g., `[class*='gallery']` would wrongly remove `product-gallery-and-description`).
const PRODUCT_BOILERPLATE_SELECTORS: &[&str] = &[
    // Breadcrumbs (always navigation, never content)
    "nav[aria-label='breadcrumb']",
    "nav[aria-label='Breadcrumb']",
    ".breadcrumb",
    ".breadcrumbs",
    // Related/recommended product sections (separate from main product)
    ".related-products",
    ".recommended-products",
    ".recently-viewed",
    ".also-bought",
    ".cross-sells",
    ".upsells",
    "#recently-viewed",
    // Newsletter/popup overlays
    ".newsletter-popup",
    ".newsletter-signup",
    ".popup-overlay",
    // Customer reviews section (not part of product description)
    "#reviews",
    "#customer-reviews",
    ".reviews-section",
    ".customer-reviews",
    // Reviews by class/id substring
    "[class*='reviews']",
    "[class*='review-']",
    "[class*='-review']",
    "[id*='reviews']",
    // Ratings
    "[class*='rating']",
    "[class*='ratings']",
    // Q&A / FAQ sections
    "[class*='questions']",
    "[class*='faq']",
    "[id*='questions']",
    "[id*='faq']",
    // Email signup / newsletter
    "[class*='newsletter']",
    "[class*='email-signup']",
    "[class*='signup']",
    // Recently viewed / recommended
    "[class*='recently-viewed']",
    "[class*='recommend']",
    "[class*='related-']",
    // Amazon-specific boilerplate
    "[class*='sponsored']",
    "[class*='a-carousel']",
    "[class*='similarities']",
    // eBay-specific boilerplate
    "[class*='merch-module']",
    "[class*='vi-ilComp']",
    // Generic "similar/related items" headings and sections
    "[class*='similar-']",
    "[class*='also-viewed']",
    "[class*='also-bought']",
    "[class*='people-also']",
    "[class*='you-may-also']",
];

/// Boilerplate selectors for documentation pages.
const DOC_BOILERPLATE_SELECTORS: &[&str] = &[
    // Sphinx sidebar and navigation
    "div.sphinxsidebar",         // Sphinx left sidebar (ToC, search)
    "div.related",               // Sphinx breadcrumb/navigation bars
    "a.headerlink",              // Pilcrow (¶) paragraph anchor links
    // SQLAlchemy custom theme
    "#docs-sidebar",             // Left sidebar nav tree
    "#docs-sidebar-popout",      // Top sidebar with site title
    "#docs-bottom-navigation",   // Previous/Next + copyright
    // Django docs
    "[role='complementary']",    // Django sidebar (ToC, donation)
    "nav.browse-horizontal",     // Django previous/next navigation
    // ReadTheDocs
    ".rst-other-versions",       // Version selector
    "nav.wy-nav-side",           // RTD left sidebar
    // Rustdoc
    ".sidebar",                  // Rustdoc left sidebar (module list, search)
    ".sidebar-elems",            // Rustdoc sidebar elements (methods, traits)
    ".sidebar-crate",            // Rustdoc crate name in sidebar
    "a.src",                     // Rustdoc [src] source links
    // MDN Web Docs
    ".left-sidebar",             // MDN left sidebar (API reference tree)
    ".reference-toc",            // MDN right sidebar (on-this-page ToC)
    ".document-toc",             // MDN table of contents
    ".bc-table",                 // MDN browser compatibility table
    // PostgreSQL
    "div.navheader",             // PostgreSQL top navigation
    "div.navfooter",             // PostgreSQL bottom navigation
    // Generic doc patterns
    "nav.toc",                   // Table of contents nav
    ".nav-sidebar",              // Sidebar navigation
    ".docs-sidebar",             // Documentation sidebar
    ".page-nav",                 // Page navigation (prev/next)
    ".breadcrumb",               // Breadcrumbs
];

/// Boilerplate selectors for service/marketing pages.
///
/// Remove elements that are decorative or navigational on marketing pages,
/// while preserving the actual service descriptions, features, and benefits.
const SERVICE_BOILERPLATE_SELECTORS: &[&str] = &[
    // Cookie/consent banners
    "[class*='cookie']",
    "[class*='consent']",
    "[id*='cookie']",
    // Modal/popup overlays
    "[class*='modal']",
    "[class*='popup']",
    "[class*='overlay']",
    // Newsletter/signup forms (not the main CTA)
    "[class*='newsletter']",
    "[class*='subscribe']",
    // Chat widgets
    "[class*='chat-widget']",
    "[class*='intercom']",
    "[class*='drift']",
    "[class*='zendesk']",
];

/// CSS class/id patterns that indicate a product grid or listing.
const PRODUCT_GRID_PATTERNS: &[&str] = &[
    "product-grid",
    "product-list",
    "product-listing",
    "products-grid",
    "product-card",
    "product-tile",
    "collection-products",
    "search-results-products",
];

/// Patterns in class/id/text that indicate an add-to-cart action.
const ADD_TO_CART_PATTERNS: &[&str] = &[
    "add-to-cart",
    "add_to_cart",
    "addtocart",
    "add-to-bag",
    "buy-now",
    "buynow",
];

/// Extract HTML-level signals from a parsed document.
///
/// Uses the already-extracted metadata for og:type, and scans JSON-LD
/// scripts for Product/@type values. Also checks for product grid
/// and add-to-cart patterns in the document body.
///
/// This is intentionally lightweight — it avoids re-parsing the full
/// metadata and only looks for signals relevant to page type refinement.
#[must_use]
pub(crate) fn extract_html_signals(doc: &crate::dom::Document, metadata: &crate::result::Metadata) -> HtmlSignals {
    let mut signals = HtmlSignals::default();

    // 1. og:type — already extracted by metadata module
    signals.og_type = metadata.page_type.clone();

    // 2. JSON-LD @type values — scan for Product/ProductGroup
    signals.ld_types = extract_ld_types(doc);

    // 2b. AggregateOffer detection — Product with price range = category page
    signals.has_aggregate_offer = has_aggregate_offer_in_ld(doc);

    // 3. Product grid patterns — check class/id attributes in body
    signals.has_product_grid = has_pattern_in_classes(doc, PRODUCT_GRID_PATTERNS);

    // 4. Add-to-cart patterns — check class/id attributes and button text
    signals.has_add_to_cart = has_pattern_in_classes(doc, ADD_TO_CART_PATTERNS)
        || has_cart_button_text(doc);

    // 5. Count product-class elements (product-card, product-tile, product-item, etc.)
    signals.product_element_count = count_product_elements(doc);

    // 6. Pagination — rel="next", pagination CSS classes, page number links
    signals.has_pagination = has_pagination(doc);

    // 7. Documentation signals — code blocks and doc-style navigation
    signals.code_block_count = doc.select("code, pre").length();
    signals.has_docs_nav = has_docs_navigation(doc, signals.code_block_count);

    // 8. Link density signals for listing detection
    let link_count = doc.select("a").length();
    let p_text = doc.select("p").text();
    let p_word_count = p_text.split_whitespace().count();
    signals.paragraph_word_count = p_word_count;
    signals.link_ratio = if p_word_count > 0 {
        link_count as f64 / p_word_count as f64
    } else if link_count > 0 {
        // Many links with zero paragraph words → very high ratio
        link_count as f64
    } else {
        0.0
    };

    signals
}

/// Extract @type values from all JSON-LD scripts in the document.
///
/// Returns the original-case @type values (e.g. "Product", "Article").
/// Only returns types relevant to page classification.
fn extract_ld_types(doc: &Document) -> Vec<String> {
    let mut types = Vec::new();

    for script in doc.select(r#"script[type="application/ld+json"]"#).nodes() {
        let sel = Selection::from(*script);
        let text = sel.text();
        let text = text.trim();
        if text.is_empty() {
            continue;
        }

        let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
            continue;
        };

        collect_types(&value, &mut types);
    }

    types
}

/// Check if any JSON-LD Product block has an AggregateOffer (price range = multiple products).
fn has_aggregate_offer_in_ld(doc: &Document) -> bool {
    for script in doc.select(r#"script[type="application/ld+json"]"#).nodes() {
        let sel = Selection::from(*script);
        let text = sel.text();
        let text = text.trim();
        if text.is_empty() {
            continue;
        }

        let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
            continue;
        };

        if check_aggregate_offer(&value) {
            return true;
        }
    }
    false
}

/// Recursively check if a JSON-LD value contains a Product with AggregateOffer.
fn check_aggregate_offer(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            // Check if this is a Product with AggregateOffer in offers
            let is_product = map.get("@type").map_or(false, |t| match t {
                serde_json::Value::String(s) => s == "Product" || s == "ProductGroup",
                serde_json::Value::Array(arr) => arr.iter().any(|v| {
                    v.as_str().map_or(false, |s| s == "Product" || s == "ProductGroup")
                }),
                _ => false,
            });

            if is_product {
                if let Some(offers) = map.get("offers") {
                    // Check if offers is an AggregateOffer directly
                    if let Some(offer_type) = offers.get("@type").and_then(|t| t.as_str()) {
                        if offer_type == "AggregateOffer" {
                            return true;
                        }
                    }
                    // Check if offers is an array containing AggregateOffer
                    if let serde_json::Value::Array(arr) = offers {
                        for item in arr {
                            if let Some(t) = item.get("@type").and_then(|t| t.as_str()) {
                                if t == "AggregateOffer" {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }

            // Recurse into nested objects
            for val in map.values() {
                if check_aggregate_offer(val) {
                    return true;
                }
            }
            false
        }
        serde_json::Value::Array(arr) => arr.iter().any(check_aggregate_offer),
        _ => false,
    }
}

/// Recursively collect @type values from a JSON-LD value.
fn collect_types(value: &serde_json::Value, types: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(type_val) = map.get("@type") {
                match type_val {
                    serde_json::Value::String(s) => {
                        types.push(s.clone());
                    }
                    serde_json::Value::Array(arr) => {
                        for item in arr {
                            if let serde_json::Value::String(s) = item {
                                types.push(s.clone());
                            }
                        }
                    }
                    _ => {}
                }
            }
            // Recurse into nested objects (handles @graph, nested schemas)
            for (_, val) in map {
                collect_types(val, types);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                collect_types(item, types);
            }
        }
        _ => {}
    }
}

/// Check if any element's class or id contains one of the patterns.
fn has_pattern_in_classes(doc: &Document, patterns: &[&str]) -> bool {
    // Check all elements with class attributes
    for node in doc.select("[class]").nodes() {
        let sel = Selection::from(*node);
        let class = sel.attr("class").unwrap_or_default().to_lowercase();
        if patterns.iter().any(|p| class.contains(p)) {
            return true;
        }
    }
    // Check id attributes too
    for node in doc.select("[id]").nodes() {
        let sel = Selection::from(*node);
        let id = sel.attr("id").unwrap_or_default().to_lowercase();
        if patterns.iter().any(|p| id.contains(p)) {
            return true;
        }
    }
    false
}

/// Check if any button/a element contains add-to-cart text.
fn has_cart_button_text(doc: &Document) -> bool {
    for node in doc.select("button, a.btn, a.button, input[type='submit']").nodes() {
        let sel = Selection::from(*node);
        let text = sel.text().to_lowercase();
        if text.contains("add to cart")
            || text.contains("add to bag")
            || text.contains("buy now")
        {
            return true;
        }
    }
    false
}

/// CSS class/id patterns that strongly indicate documentation navigation.
const DOCS_NAV_PATTERNS: &[&str] = &[
    "docs-sidebar",
    "doc-sidebar",
    "docs-nav",
    "doc-nav",
    "docsidebar",
    "docnav",
    "sidebar-docs",
    "api-reference",
    "apireference",
    "table-of-contents",
    "tableofcontents",
    "wiki-body",
    "wikibody",
    "mw-content", // MediaWiki
];

/// Broader class patterns — reliable for docs when combined with code blocks.
const DOCS_NAV_BROAD_PATTERNS: &[&str] = &[
    "sidebar",
];

/// Check if the document has documentation-style navigation.
///
/// Uses specific patterns first (high confidence), then broader patterns
/// only when code block count is high (reduces false positives on blogs with sidebars).
fn has_docs_navigation(doc: &Document, code_block_count: usize) -> bool {
    // Specific doc nav patterns — always trust these
    if has_pattern_in_classes(doc, DOCS_NAV_PATTERNS) {
        return true;
    }

    // Broader patterns — only trust with many code blocks (10+)
    // (blog sidebars exist but rarely alongside 10+ code blocks)
    if code_block_count >= 10 && has_pattern_in_classes(doc, DOCS_NAV_BROAD_PATTERNS) {
        return true;
    }

    false
}

/// CSS class/id patterns that indicate pagination.
const PAGINATION_CLASS_PATTERNS: &[&str] = &[
    "pagination",
    "pager",
    "paginator",
    "page-numbers",
    "page-nav",
];

/// Check if the document has pagination elements.
///
/// Looks for:
/// - `<link rel="next">` or `<link rel="prev">` in head
/// - Elements with pagination-related CSS classes
/// - `<nav>` elements with aria-label containing "pagination"
fn has_pagination(doc: &Document) -> bool {
    // rel="next" / rel="prev" in <link> tags (most reliable)
    if doc.select(r#"link[rel="next"], link[rel="prev"]"#).length() > 0 {
        return true;
    }

    // Pagination CSS classes
    if has_pattern_in_classes(doc, PAGINATION_CLASS_PATTERNS) {
        return true;
    }

    // aria-label="pagination" on nav elements
    for node in doc.select("nav[aria-label]").nodes() {
        let sel = Selection::from(*node);
        let label = sel.attr("aria-label").unwrap_or_default().to_lowercase();
        if label.contains("pagination") || label.contains("paging") {
            return true;
        }
    }

    false
}

/// Patterns in CSS classes that indicate individual product items in a listing.
const PRODUCT_ITEM_CLASS_PATTERNS: &[&str] = &[
    "product-card",
    "product-tile",
    "product-item",
    "product-block",
    "productcard",
    "producttile",
    "productitem",
    "product_card",
    "product_tile",
    "product_item",
];

/// Count elements with product-item CSS classes.
///
/// A high count (5+) strongly indicates a category/listing page.
fn count_product_elements(doc: &Document) -> usize {
    let mut count = 0;
    for node in doc.select("[class]").nodes() {
        let sel = Selection::from(*node);
        let class = sel.attr("class").unwrap_or_default().to_lowercase();
        if PRODUCT_ITEM_CLASS_PATTERNS.iter().any(|p| class.contains(p)) {
            count += 1;
        }
    }
    count
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract domain and path from a lowercased URL.
fn extract_domain_path<'a>(url: &'a str) -> (&'a str, &'a str) {
    // Strip protocol
    let without_proto = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

    // Split domain from path
    match without_proto.find('/') {
        Some(idx) => (&without_proto[..idx], &without_proto[idx..]),
        None => (without_proto, "/"),
    }
}

/// Check if `haystack` contains any of the `needles`.
#[inline]
fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- URL classification tests ---

    #[test]
    fn test_forum_by_domain() {
        assert_eq!(
            classify_url("https://community.openai.com/t/some-topic/12345"),
            PageType::Forum
        );
        assert_eq!(
            classify_url("https://forums.docker.com/t/confusing-resolver/144854"),
            PageType::Forum
        );
        assert_eq!(
            classify_url("https://reddit.com/r/rust/comments/abc123"),
            PageType::Forum
        );
        assert_eq!(
            classify_url("https://stackoverflow.com/questions/12345/how-to"),
            PageType::Forum
        );
        assert_eq!(
            classify_url("https://news.ycombinator.com/item?id=38778098"),
            PageType::Forum
        );
        assert_eq!(
            classify_url("https://bbs.archlinux.org/viewtopic.php?id=309936"),
            PageType::Forum
        );
    }

    #[test]
    fn test_forum_by_path() {
        assert_eq!(
            classify_url("https://example.com/forum/general/topic-123"),
            PageType::Forum
        );
        assert_eq!(
            classify_url("https://example.com/discussion/my-topic"),
            PageType::Forum
        );
    }

    #[test]
    fn test_documentation_by_domain() {
        assert_eq!(
            classify_url("https://docs.djangoproject.com/en/5.1/ref/models/querysets/"),
            PageType::Documentation
        );
        assert_eq!(
            classify_url("https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html"),
            PageType::Documentation
        );
        assert_eq!(
            classify_url("https://wiki.archlinux.org/title/OpenSSH"),
            PageType::Documentation
        );
        assert_eq!(
            classify_url("https://developer.mozilla.org/en-US/docs/Web/API/Fetch_API"),
            PageType::Documentation
        );
    }

    #[test]
    fn test_documentation_by_path() {
        assert_eq!(
            classify_url("https://example.com/docs/getting-started"),
            PageType::Documentation
        );
        assert_eq!(
            classify_url("https://example.com/api/v2/reference"),
            PageType::Documentation
        );
        assert_eq!(
            classify_url("https://example.com/tutorial/basics"),
            PageType::Documentation
        );
    }

    #[test]
    fn test_product_by_path() {
        assert_eq!(
            classify_url("https://www.allbirds.com/products/mens-wool-runners"),
            PageType::Product
        );
        assert_eq!(
            classify_url("https://www.amazon.com/dp/B07XYZ"),
            PageType::Product
        );
        assert_eq!(
            classify_url("https://example.com/shop/vitality-elixir/"),
            PageType::Product
        );
    }

    #[test]
    fn test_product_by_domain() {
        assert_eq!(
            classify_url("https://shop.bombas.com/something"),
            PageType::Product
        );
    }

    #[test]
    fn test_category_by_path() {
        assert_eq!(
            classify_url("https://www.allbirds.com/collections/mens-sneakers"),
            PageType::Category
        );
        assert_eq!(
            classify_url("https://www.ikea.com/us/en/cat/bookcases-st002/"),
            PageType::Category
        );
        assert_eq!(
            classify_url("https://example.com/browse/electronics"),
            PageType::Category
        );
    }

    #[test]
    fn test_service_by_path() {
        assert_eq!(
            classify_url("https://example.com/services/consulting"),
            PageType::Service
        );
        assert_eq!(
            classify_url("https://example.com/solutions/enterprise"),
            PageType::Service
        );
    }

    #[test]
    fn test_article_by_path() {
        assert_eq!(
            classify_url("https://example.com/blog/my-post"),
            PageType::Article
        );
        assert_eq!(
            classify_url("https://example.com/news/breaking"),
            PageType::Article
        );
        assert_eq!(
            classify_url("https://example.com/insights/trends-2025"),
            PageType::Article
        );
    }

    #[test]
    fn test_article_by_slug() {
        assert_eq!(
            classify_url("https://example.com/how-to-build-a-widget"),
            PageType::Article
        );
        assert_eq!(
            classify_url("https://example.com/what-is-cloud-computing"),
            PageType::Article
        );
        assert_eq!(
            classify_url("https://example.com/react-vs-vue-comparison"),
            PageType::Article
        );
    }

    #[test]
    fn test_fallback_to_article() {
        assert_eq!(
            classify_url("https://www.salesforce.com/crm/"),
            PageType::Article
        );
        assert_eq!(
            classify_url("https://stripe.com/payments"),
            PageType::Article
        );
        assert_eq!(classify_url(""), PageType::Article);
    }

    // --- HTML signal refinement tests ---

    #[test]
    fn test_refine_product_by_og_type() {
        let signals = HtmlSignals {
            og_type: Some("product".to_string()),
            ..Default::default()
        };
        assert_eq!(
            refine_with_html_signals(PageType::Article, &signals),
            PageType::Product
        );
    }

    #[test]
    fn test_refine_product_by_ld_json() {
        let signals = HtmlSignals {
            ld_types: vec!["Product".to_string(), "Brand".to_string()],
            ..Default::default()
        };
        assert_eq!(
            refine_with_html_signals(PageType::Article, &signals),
            PageType::Product
        );
    }

    #[test]
    fn test_refine_category_by_grid_plus_cart() {
        let signals = HtmlSignals {
            has_product_grid: true,
            has_add_to_cart: true,
            ..Default::default()
        };
        assert_eq!(
            refine_with_html_signals(PageType::Article, &signals),
            PageType::Category
        );
    }

    #[test]
    fn test_refine_preserves_url_classification() {
        let signals = HtmlSignals {
            og_type: Some("product".to_string()),
            ..Default::default()
        };
        // URL said Forum — don't override even if HTML says product
        assert_eq!(
            refine_with_html_signals(PageType::Forum, &signals),
            PageType::Forum
        );
    }

    #[test]
    fn test_refine_no_signals_stays_article() {
        let signals = HtmlSignals::default();
        assert_eq!(
            refine_with_html_signals(PageType::Article, &signals),
            PageType::Article
        );
    }

    #[test]
    fn test_refine_category_with_grid_and_product_ld() {
        // Product grid + Product in LD-JSON but no single product LD → still product
        // (CollectionPage with Product items could be either, but Product LD wins)
        let signals = HtmlSignals {
            has_product_grid: true,
            ld_types: vec!["Product".to_string()],
            has_add_to_cart: true,
            ..Default::default()
        };
        // Product signal found, and has grid but also has single product LD → Product
        assert_eq!(
            refine_with_html_signals(PageType::Article, &signals),
            PageType::Product
        );
    }

    // --- New category signal tests ---

    #[test]
    fn test_refine_category_by_collection_page_ld() {
        let signals = HtmlSignals {
            ld_types: vec!["CollectionPage".to_string(), "BreadcrumbList".to_string()],
            ..Default::default()
        };
        assert_eq!(
            refine_with_html_signals(PageType::Article, &signals),
            PageType::Category
        );
    }

    #[test]
    fn test_refine_category_by_item_list_with_product_grid() {
        // ItemList alone is not enough (listicle articles use it for SEO).
        // Needs product grid or product elements as supporting evidence.
        let signals = HtmlSignals {
            ld_types: vec!["ItemList".to_string()],
            has_product_grid: true,
            ..Default::default()
        };
        assert_eq!(
            refine_with_html_signals(PageType::Article, &signals),
            PageType::Category
        );
    }

    #[test]
    fn test_refine_article_stays_with_item_list_alone() {
        // ItemList without product signals stays Article (likely a listicle)
        let signals = HtmlSignals {
            ld_types: vec!["ItemList".to_string()],
            ..Default::default()
        };
        assert_eq!(
            refine_with_html_signals(PageType::Article, &signals),
            PageType::Article
        );
    }

    #[test]
    fn test_refine_category_by_og_product_group() {
        let signals = HtmlSignals {
            og_type: Some("product.group".to_string()),
            ..Default::default()
        };
        assert_eq!(
            refine_with_html_signals(PageType::Article, &signals),
            PageType::Category
        );
    }

    #[test]
    fn test_refine_category_by_product_elements_with_pagination() {
        let signals = HtmlSignals {
            product_element_count: 12,
            has_pagination: true,
            ..Default::default()
        };
        assert_eq!(
            refine_with_html_signals(PageType::Article, &signals),
            PageType::Category
        );
    }

    #[test]
    fn test_refine_category_by_product_elements_with_grid_and_cart() {
        // Infinite-scroll category pages lack pagination markup
        // but have product grid + add-to-cart
        let signals = HtmlSignals {
            product_element_count: 12,
            has_product_grid: true,
            has_add_to_cart: true,
            ..Default::default()
        };
        assert_eq!(
            refine_with_html_signals(PageType::Article, &signals),
            PageType::Category
        );
    }

    #[test]
    fn test_refine_article_stays_with_product_elements_only() {
        // Many product elements WITHOUT supporting signals stays Article
        // (could be a review/comparison article)
        let signals = HtmlSignals {
            product_element_count: 12,
            ..Default::default()
        };
        assert_eq!(
            refine_with_html_signals(PageType::Article, &signals),
            PageType::Article
        );
    }

    #[test]
    fn test_refine_product_few_product_elements() {
        // Only 1 product element + Product LD → still Product (not Category)
        let signals = HtmlSignals {
            ld_types: vec!["Product".to_string()],
            product_element_count: 1,
            ..Default::default()
        };
        assert_eq!(
            refine_with_html_signals(PageType::Article, &signals),
            PageType::Product
        );
    }

    #[test]
    fn test_extract_signals_collection_page_ld() {
        let html = r#"<html><head>
            <script type="application/ld+json">
            {"@type": "CollectionPage", "name": "Men's Shoes", "hasPart": [
                {"@type": "Product", "name": "Shoe A"},
                {"@type": "Product", "name": "Shoe B"}
            ]}
            </script>
            </head><body></body></html>"#;
        let doc = Document::from(html);
        let metadata = Metadata::default();
        let signals = extract_html_signals(&doc, &metadata);
        assert!(signals.ld_types.contains(&"CollectionPage".to_string()));
        assert_eq!(
            refine_with_html_signals(PageType::Article, &signals),
            PageType::Category
        );
    }

    #[test]
    fn test_extract_signals_product_element_count() {
        let html = r#"<html><body>
            <div class="product-grid">
            <div class="product-card">A</div>
            <div class="product-card">B</div>
            <div class="product-card">C</div>
            <div class="product-card">D</div>
            <div class="product-card">E</div>
            <div class="product-card">F</div>
            </div>
            <nav class="pagination"><a href="?page=2">2</a></nav>
            </body></html>"#;
        let doc = Document::from(html);
        let metadata = Metadata::default();
        let signals = extract_html_signals(&doc, &metadata);
        assert_eq!(signals.product_element_count, 6);
        assert!(signals.has_product_grid);
        assert!(signals.has_pagination);
        assert_eq!(
            refine_with_html_signals(PageType::Article, &signals),
            PageType::Category
        );
    }

    #[test]
    fn test_extract_signals_pagination_rel_next() {
        let html = r#"<html><head>
            <link rel="next" href="?page=2">
            </head><body></body></html>"#;
        let doc = Document::from(html);
        let metadata = Metadata::default();
        let signals = extract_html_signals(&doc, &metadata);
        assert!(signals.has_pagination);
    }

    #[test]
    fn test_extract_signals_pagination_aria_label() {
        let html = r#"<html><body>
            <nav aria-label="Pagination">
            <a href="?page=1">1</a>
            <a href="?page=2">2</a>
            </nav>
            </body></html>"#;
        let doc = Document::from(html);
        let metadata = Metadata::default();
        let signals = extract_html_signals(&doc, &metadata);
        assert!(signals.has_pagination);
    }

    // --- Documentation signal tests ---

    #[test]
    fn test_refine_documentation_by_code_and_nav() {
        let signals = HtmlSignals {
            code_block_count: 20,
            has_docs_nav: true,
            ..Default::default()
        };
        assert_eq!(
            refine_with_html_signals(PageType::Article, &signals),
            PageType::Documentation
        );
    }

    #[test]
    fn test_refine_article_stays_with_code_only() {
        // Code blocks without doc nav → stays Article (could be a tech blog post)
        let signals = HtmlSignals {
            code_block_count: 20,
            has_docs_nav: false,
            ..Default::default()
        };
        assert_eq!(
            refine_with_html_signals(PageType::Article, &signals),
            PageType::Article
        );
    }

    #[test]
    fn test_refine_article_stays_with_nav_but_few_code() {
        // Doc nav but very few code blocks → stays Article
        let signals = HtmlSignals {
            code_block_count: 1,
            has_docs_nav: true,
            ..Default::default()
        };
        assert_eq!(
            refine_with_html_signals(PageType::Article, &signals),
            PageType::Article
        );
    }

    #[test]
    fn test_extract_signals_docs_page() {
        let html = r#"<html><body>
            <nav class="docs-sidebar"><a href="/intro">Intro</a></nav>
            <div id="doc-content">
            <pre><code>fn main() {}</code></pre>
            <pre><code>let x = 42;</code></pre>
            <code>inline</code>
            </div>
            </body></html>"#;
        let doc = Document::from(html);
        let metadata = Metadata::default();
        let signals = extract_html_signals(&doc, &metadata);
        assert!(signals.has_docs_nav);
        assert!(signals.code_block_count >= 3);
        assert_eq!(
            refine_with_html_signals(PageType::Article, &signals),
            PageType::Documentation
        );
    }

    // --- Helper tests ---

    #[test]
    fn test_extract_domain_path() {
        assert_eq!(
            extract_domain_path("https://example.com/path/to/page"),
            ("example.com", "/path/to/page")
        );
        assert_eq!(
            extract_domain_path("http://docs.example.com/"),
            ("docs.example.com", "/")
        );
        assert_eq!(
            extract_domain_path("example.com"),
            ("example.com", "/")
        );
        assert_eq!(
            extract_domain_path("/just/a/path"),
            ("", "/just/a/path")
        );
    }

    #[test]
    fn test_page_type_display() {
        assert_eq!(PageType::Article.to_string(), "article");
        assert_eq!(PageType::Forum.to_string(), "forum");
        assert_eq!(PageType::Product.to_string(), "product");
        assert_eq!(PageType::Category.to_string(), "collection");
        assert_eq!(PageType::Documentation.to_string(), "documentation");
        assert_eq!(PageType::Service.to_string(), "service");
    }

    // --- HTML signal extraction tests ---

    #[test]
    fn test_extract_signals_product_json_ld() {
        let html = r#"<html><head>
            <script type="application/ld+json">
            {"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "price": "29.99"}}
            </script>
            </head><body><p>A great widget.</p></body></html>"#;
        let doc = Document::from(html);
        let metadata = Metadata::default();
        let signals = extract_html_signals(&doc, &metadata);
        assert!(signals.ld_types.contains(&"Product".to_string()));
        assert!(signals.ld_types.contains(&"Offer".to_string()));
    }

    #[test]
    fn test_extract_signals_og_type_from_metadata() {
        let doc = Document::from("<html><body></body></html>");
        let metadata = Metadata {
            page_type: Some("product".to_string()),
            ..Metadata::default()
        };
        let signals = extract_html_signals(&doc, &metadata);
        assert_eq!(signals.og_type, Some("product".to_string()));
    }

    #[test]
    fn test_extract_signals_product_grid() {
        let html = r#"<html><body>
            <div class="product-grid">
                <div class="product-card">Item 1</div>
                <div class="product-card">Item 2</div>
            </div>
            </body></html>"#;
        let doc = Document::from(html);
        let metadata = Metadata::default();
        let signals = extract_html_signals(&doc, &metadata);
        assert!(signals.has_product_grid);
    }

    #[test]
    fn test_extract_signals_add_to_cart_button() {
        let html = r#"<html><body>
            <button class="add-to-cart">Add to Cart</button>
            </body></html>"#;
        let doc = Document::from(html);
        let metadata = Metadata::default();
        let signals = extract_html_signals(&doc, &metadata);
        assert!(signals.has_add_to_cart);
    }

    #[test]
    fn test_extract_signals_no_signals_on_article() {
        let html = r#"<html><head>
            <script type="application/ld+json">
            {"@type": "Article", "headline": "News Story"}
            </script>
            </head><body><article><p>Content here.</p></article></body></html>"#;
        let doc = Document::from(html);
        let metadata = Metadata::default();
        let signals = extract_html_signals(&doc, &metadata);
        assert!(signals.og_type.is_none());
        assert!(!signals.has_product_grid);
        assert!(!signals.has_add_to_cart);
        // ld_types has "Article" but that won't trigger Product refinement
        assert_eq!(
            refine_with_html_signals(PageType::Article, &signals),
            PageType::Article
        );
    }

    #[test]
    fn test_extract_signals_graph_array() {
        let html = r#"<html><head>
            <script type="application/ld+json">
            {"@graph": [
                {"@type": "WebSite", "name": "Example"},
                {"@type": "Product", "name": "Widget"}
            ]}
            </script>
            </head><body></body></html>"#;
        let doc = Document::from(html);
        let metadata = Metadata::default();
        let signals = extract_html_signals(&doc, &metadata);
        assert!(signals.ld_types.contains(&"Product".to_string()));
        assert!(signals.ld_types.contains(&"WebSite".to_string()));
    }

    #[test]
    fn test_full_pipeline_ambiguous_url_with_product_signals() {
        // URL says nothing useful (falls back to Article)
        let url_type = classify_url("https://example.com/staple-tee-5001");
        assert_eq!(url_type, PageType::Article);

        // But HTML has Product JSON-LD
        let signals = HtmlSignals {
            ld_types: vec!["Product".to_string()],
            og_type: Some("product".to_string()),
            ..Default::default()
        };
        let refined = refine_with_html_signals(url_type, &signals);
        assert_eq!(refined, PageType::Product);
    }
}


pub(crate) mod ml;
