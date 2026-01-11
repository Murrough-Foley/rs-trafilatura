//! Metadata Selectors
//!
//! Selector rules for extracting article metadata (author, title, tags, categories).
//!
//! Port of:
//! - `internal/selector/meta-author.go` - Author extraction
//! - `internal/selector/meta-author-discard.go` - Author noise removal
//! - `internal/selector/meta-title.go` - Title extraction
//! - `internal/selector/meta-tags.go` - Tag extraction
//! - `internal/selector/meta-categories.go` - Category extraction

use dom_query::Selection;

use crate::selector::utils::{attr, class, contains, id, lower, tag};
use crate::selector::Rule;

// ============================================================
// AUTHOR SELECTORS
// ============================================================

/// Author selector rules (in priority order)
pub static META_AUTHOR: &[Rule] = &[meta_author_rule_1, meta_author_rule_2, meta_author_rule_3];

/// Rule 1: Specific author markers
///
/// Tags: a, address, div, link, p, span, strong, author
/// Patterns: rel="author", id="author", class="author", itemprop="author name", etc.
///
/// Go: metaAuthorRule1 (lines 37-70)
#[must_use]
pub fn meta_author_rule_1(sel: &Selection) -> bool {
    let tag_val = tag(sel);
    let id_val = id(sel);
    let class_val = class(sel);
    let rel = attr(sel, "rel");
    let item_prop = attr(sel, "itemprop");
    let data_testid = attr(sel, "data-testid");

    // <author> tag always matches
    if tag_val == "author" {
        return true;
    }

    // Tag filter
    if !matches!(
        tag_val.as_str(),
        "a" | "address" | "div" | "link" | "p" | "span" | "strong"
    ) {
        return false;
    }

    // Pattern matching
    rel == "author"
        || id_val == "author"
        || class_val == "author"
        || item_prop == "author name"
        || rel == "me"
        || contains(&class_val, "author-name")
        || contains(&class_val, "AuthorName")
        || contains(&class_val, "authorName")
        || contains(&class_val, "author name")
        || data_testid == "AuthorCard"
        || data_testid == "AuthorURL"
}

/// Rule 2: Generic author markers
///
/// Tags: a, div, h3, h4, p, span
/// Patterns: *author*, byline, channel-name, submitted-by, posted-by, etc.
///
/// Go: metaAuthorRule2 (lines 72-109)
#[must_use]
pub fn meta_author_rule_2(sel: &Selection) -> bool {
    let tag_val = tag(sel);
    let id_val = id(sel);
    let class_val = class(sel);
    let item_prop = attr(sel, "itemprop");

    // Tag filter
    if !matches!(
        tag_val.as_str(),
        "a" | "div" | "h3" | "h4" | "p" | "span"
    ) {
        return false;
    }

    // Pattern matching
    contains(&class_val, "author")
        || contains(&id_val, "author")
        || contains(&item_prop, "author")
        || class_val == "byline"
        || contains(&class_val, "channel-name")
        // Chinese patterns
        || contains(&id_val, "zuozhe")
        || contains(&class_val, "zuozhe")
        || contains(&id_val, "bianji")
        || contains(&class_val, "bianji")
        || contains(&id_val, "xiaobian")
        || contains(&class_val, "xiaobian")
        // Other patterns
        || contains(&class_val, "submitted-by")
        || contains(&class_val, "posted-by")
        || class_val == "username"
        || class_val == "byl"
        || class_val == "BBL"
        || contains(&class_val, "journalist-name")
}

/// Rule 3: Last resort author markers (any element)
///
/// No tag filter - matches any element
/// Patterns: author (case-insensitive), screenname, byline, writer
///
/// Go: metaAuthorRule3 (lines 111-132)
#[must_use]
pub fn meta_author_rule_3(sel: &Selection) -> bool {
    let id_val = id(sel);
    let class_val = class(sel);
    let data_component = attr(sel, "data-component");
    let item_prop = attr(sel, "itemprop");

    // No tag filter

    // Pattern matching
    contains(&lower(&id_val), "author")
        || contains(&lower(&class_val), "author")
        || contains(&class_val, "screenname")
        || contains(&lower(&data_component), "byline")
        || contains(&item_prop, "author")
        || contains(&class_val, "writer")
        || contains(&lower(&class_val), "byline")
}

// ============================================================
// AUTHOR DISCARD SELECTORS
// ============================================================

/// Author discard rules (noise to remove from author elements)
pub static META_AUTHOR_DISCARD: &[Rule] = &[
    meta_author_discard_rule_1,
    meta_author_discard_rule_2,
    meta_author_discard_rule_3,
];

/// Rule 1: Author noise by tag
///
/// Tags: a, span, div
/// Patterns: link, author-link, mailto, twitter, avatar, pic, photo, image, logo
///
/// Go: metaAuthorDiscardRule1 (lines 30-55)
#[must_use]
pub fn meta_author_discard_rule_1(sel: &Selection) -> bool {
    let tag_val = tag(sel);
    let class_val = class(sel);
    let href = attr(sel, "href");

    // Tag filter
    if !matches!(tag_val.as_str(), "a" | "span" | "div") {
        return false;
    }

    // Pattern matching
    contains(&class_val, "link")
        || contains(&class_val, "author-link")
        || contains(&href, "mailto")
        || contains(&href, "twitter")
        || contains(&class_val, "avatar")
        || contains(&class_val, "pic")
        || contains(&class_val, "photo")
        || contains(&class_val, "image")
        || contains(&class_val, "logo")
}

/// Rule 2: Author social links
///
/// Tags: a
/// Patterns: href contains social media domains
///
/// Go: metaAuthorDiscardRule2 (lines 57-73)
#[must_use]
pub fn meta_author_discard_rule_2(sel: &Selection) -> bool {
    if tag(sel) != "a" {
        return false;
    }

    let href = attr(sel, "href");

    // Social media patterns
    contains(&href, "facebook")
        || contains(&href, "twitter")
        || contains(&href, "linkedin")
        || contains(&href, "instagram")
        || contains(&href, "youtube")
}

/// Rule 3: Author image elements
///
/// Tags: img, figure
///
/// Go: metaAuthorDiscardRule3 (lines 75-93)
#[must_use]
pub fn meta_author_discard_rule_3(sel: &Selection) -> bool {
    let tag_val = tag(sel);
    tag_val == "img" || tag_val == "figure"
}

// ============================================================
// TITLE SELECTORS
// ============================================================

/// Title selector rules
pub static META_TITLE: &[Rule] = &[meta_title_rule_1, meta_title_rule_2, meta_title_rule_3];

/// Rule 1: Specific title markers
///
/// Tags: h1, h2
/// Patterns: itemprop="headline", class*="title", id*="title"
///
/// Go: metaTitleRule1 (lines 30-52)
#[must_use]
pub fn meta_title_rule_1(sel: &Selection) -> bool {
    let tag_val = tag(sel);
    let id_val = id(sel);
    let class_val = class(sel);
    let item_prop = attr(sel, "itemprop");

    // Tag filter
    if !matches!(tag_val.as_str(), "h1" | "h2") {
        return false;
    }

    // Pattern matching
    item_prop == "headline"
        || item_prop == "name"
        || contains(&class_val, "title")
        || contains(&id_val, "title")
        || contains(&class_val, "headline")
        || contains(&id_val, "headline")
}

/// Rule 2: Generic h1 elements
///
/// Tags: h1 only
///
/// Go: metaTitleRule2 (lines 54-74)
#[must_use]
pub fn meta_title_rule_2(sel: &Selection) -> bool {
    tag(sel) == "h1"
}

/// Rule 3: Entry title markers
///
/// Any tag
/// Patterns: entry-title, post-title, article-title
///
/// Go: metaTitleRule3 (lines 76-93)
#[must_use]
pub fn meta_title_rule_3(sel: &Selection) -> bool {
    let class_val = class(sel);

    // No tag filter
    contains(&class_val, "entry-title")
        || contains(&class_val, "post-title")
        || contains(&class_val, "article-title")
}

// ============================================================
// TAG SELECTORS
// ============================================================

/// Tag selector rules (article tags/keywords)
pub static META_TAGS: &[Rule] = &[
    meta_tags_rule_1,
    meta_tags_rule_2,
    meta_tags_rule_3,
    meta_tags_rule_4,
];

/// Rule 1: Specific tag containers
///
/// Tags: div, p, ul, ol, span
/// Patterns: tags, tag-list, keywords
///
/// Go: metaTagsRule1 (lines 30-55)
#[must_use]
pub fn meta_tags_rule_1(sel: &Selection) -> bool {
    let tag_val = tag(sel);
    let id_val = id(sel);
    let class_val = class(sel);
    let id_class = format!("{id_val}{class_val}");

    // Tag filter
    if !matches!(tag_val.as_str(), "div" | "p" | "ul" | "ol" | "span") {
        return false;
    }

    // Pattern matching
    contains(&id_class, "tags") || contains(&class_val, "tag-list") || contains(&id_class, "keywords")
}

/// Rule 2: Link-based tags
///
/// Tags: a
/// Patterns: rel="tag", href contains "/tag/"
///
/// Go: metaTagsRule2 (lines 57-80)
#[must_use]
pub fn meta_tags_rule_2(sel: &Selection) -> bool {
    if tag(sel) != "a" {
        return false;
    }

    let rel = attr(sel, "rel");
    let href = attr(sel, "href");

    rel == "tag" || contains(&href, "/tag/")
}

/// Rule 3: Meta tag elements
///
/// Tags: meta
/// Patterns: name="keywords"
///
/// Go: metaTagsRule3 (lines 82-100)
#[must_use]
pub fn meta_tags_rule_3(sel: &Selection) -> bool {
    if tag(sel) != "meta" {
        return false;
    }

    let name = attr(sel, "name");
    name == "keywords" || name == "news_keywords"
}

/// Rule 4: Generic tag markers
///
/// Any tag
/// Patterns: itemprop="keywords"
///
/// Go: metaTagsRule4 (lines 102-125)
#[must_use]
pub fn meta_tags_rule_4(sel: &Selection) -> bool {
    let item_prop = attr(sel, "itemprop");
    item_prop == "keywords"
}

// ============================================================
// CATEGORY SELECTORS
// ============================================================

/// Category selector rules
pub static META_CATEGORIES: &[Rule] = &[
    meta_categories_rule_1,
    meta_categories_rule_2,
    meta_categories_rule_3,
    meta_categories_rule_4,
    meta_categories_rule_5,
    meta_categories_rule_6,
];

/// Rule 1: Specific category containers
///
/// Tags: div, p, ul, ol, span
/// Patterns: category, categories, section, rubric
///
/// Go: metaCategoriesRule1 (lines 30-60)
#[must_use]
pub fn meta_categories_rule_1(sel: &Selection) -> bool {
    let tag_val = tag(sel);
    let id_val = id(sel);
    let class_val = class(sel);
    let id_class = format!("{id_val}{class_val}");

    // Tag filter
    if !matches!(tag_val.as_str(), "div" | "p" | "ul" | "ol" | "span") {
        return false;
    }

    // Pattern matching
    contains(&id_class, "category")
        || contains(&id_class, "categories")
        || contains(&id_class, "section")
        || contains(&id_class, "rubric")
}

/// Rule 2: Link-based categories
///
/// Tags: a
/// Patterns: href contains "/category/"
///
/// Go: metaCategoriesRule2 (lines 62-85)
#[must_use]
pub fn meta_categories_rule_2(sel: &Selection) -> bool {
    if tag(sel) != "a" {
        return false;
    }

    let href = attr(sel, "href");
    contains(&href, "/category/") || contains(&href, "/section/")
}

/// Rule 3: Meta category elements
///
/// Tags: meta
/// Patterns: name="category", property="article:section"
///
/// Go: metaCategoriesRule3 (lines 87-110)
#[must_use]
pub fn meta_categories_rule_3(sel: &Selection) -> bool {
    if tag(sel) != "meta" {
        return false;
    }

    let name = attr(sel, "name");
    let property = attr(sel, "property");

    name == "category" || property == "article:section"
}

/// Rule 4: Breadcrumb categories
///
/// Tags: nav, ol, ul
/// Patterns: breadcrumb
///
/// Go: metaCategoriesRule4 (lines 112-135)
#[must_use]
pub fn meta_categories_rule_4(sel: &Selection) -> bool {
    let tag_val = tag(sel);
    let id_val = id(sel);
    let class_val = class(sel);
    let id_class = format!("{id_val}{class_val}");

    // Tag filter
    if !matches!(tag_val.as_str(), "nav" | "ol" | "ul") {
        return false;
    }

    contains(&id_class, "breadcrumb")
}

/// Rule 5: Schema.org article section
///
/// Any tag
/// Patterns: itemprop="articleSection"
///
/// Go: metaCategoriesRule5 (lines 137-160)
#[must_use]
pub fn meta_categories_rule_5(sel: &Selection) -> bool {
    let item_prop = attr(sel, "itemprop");
    item_prop == "articleSection"
}

/// Rule 6: Generic section markers
///
/// Any tag
/// Patterns: data-section, section-name
///
/// Go: metaCategoriesRule6 (lines 162-192)
#[must_use]
pub fn meta_categories_rule_6(sel: &Selection) -> bool {
    let class_val = class(sel);
    let data_section = attr(sel, "data-section");

    !data_section.is_empty() || contains(&class_val, "section-name")
}

// ============================================================
// SITENAME SELECTORS
// ============================================================

/// Sitename selector rules
pub static META_SITENAME: &[Rule] = &[meta_sitename_rule_1, meta_sitename_rule_2];

/// Rule 1: Specific sitename markers
///
/// Tags: div, p, span, a
/// Patterns: site-name, site-title, brand
#[must_use]
pub fn meta_sitename_rule_1(sel: &Selection) -> bool {
    let tag_val = tag(sel);
    let id_val = id(sel);
    let class_val = class(sel);
    let id_class = format!("{id_val}{class_val}");

    // Tag filter
    if !matches!(tag_val.as_str(), "div" | "p" | "span" | "a") {
        return false;
    }

    // Pattern matching
    contains(&id_class, "site-name")
        || contains(&id_class, "sitename")
        || contains(&id_class, "site-title")
        || contains(&id_class, "brand")
        || id_val == "logo"
        || class_val == "logo"
}

/// Rule 2: Header/nav sitename
///
/// Tags: header, nav
/// Patterns: any header/nav with site info
#[must_use]
pub fn meta_sitename_rule_2(sel: &Selection) -> bool {
    let tag_val = tag(sel);

    // Look for header or nav tags that might contain site name
    matches!(tag_val.as_str(), "header" | "nav")
}

// ============================================================
// HELPER FUNCTIONS
// ============================================================

/// Find author elements
#[must_use]
pub fn find_authors<'a>(root: &Selection<'a>) -> Vec<Selection<'a>> {
    use crate::selector::query_all;
    let mut all = Vec::new();
    for rule in META_AUTHOR {
        all.extend(query_all(root, *rule));
    }
    all
}

/// Find title elements
#[must_use]
pub fn find_titles<'a>(root: &Selection<'a>) -> Vec<Selection<'a>> {
    use crate::selector::query_all;
    let mut all = Vec::new();
    for rule in META_TITLE {
        all.extend(query_all(root, *rule));
    }
    all
}

/// Find tag elements
#[must_use]
pub fn find_tags<'a>(root: &Selection<'a>) -> Vec<Selection<'a>> {
    use crate::selector::query_all;
    let mut all = Vec::new();
    for rule in META_TAGS {
        all.extend(query_all(root, *rule));
    }
    all
}

/// Find category elements
#[must_use]
pub fn find_categories<'a>(root: &Selection<'a>) -> Vec<Selection<'a>> {
    use crate::selector::query_all;
    let mut all = Vec::new();
    for rule in META_CATEGORIES {
        all.extend(query_all(root, *rule));
    }
    all
}

/// Check if element is author noise
#[must_use]
pub fn is_author_discard(sel: &Selection) -> bool {
    META_AUTHOR_DISCARD.iter().any(|rule| rule(sel))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom;

    // ============================================================
    // AUTHOR TESTS
    // ============================================================

    #[test]
    fn test_author_rule_1_rel() {
        let doc = dom::parse(r#"<a rel="author" href="/author">Author</a>"#);
        assert!(meta_author_rule_1(&doc.select("a")));
    }

    #[test]
    fn test_author_rule_1_itemprop() {
        let doc = dom::parse(r#"<span itemprop="author name">John Doe</span>"#);
        assert!(meta_author_rule_1(&doc.select("span")));
    }

    #[test]
    fn test_author_rule_1_author_tag() {
        let doc = dom::parse("<author>John Doe</author>");
        assert!(meta_author_rule_1(&doc.select("author")));
    }

    #[test]
    fn test_author_rule_1_id_author() {
        let doc = dom::parse(r#"<div id="author">John</div>"#);
        assert!(meta_author_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_author_rule_1_class_author() {
        let doc = dom::parse(r#"<p class="author">Jane</p>"#);
        assert!(meta_author_rule_1(&doc.select("p")));
    }

    #[test]
    fn test_author_rule_1_author_name() {
        let doc = dom::parse(r#"<span class="author-name">Bob</span>"#);
        assert!(meta_author_rule_1(&doc.select("span")));
    }

    #[test]
    fn test_author_rule_1_testid() {
        let doc = dom::parse(r#"<div data-testid="AuthorCard">Alice</div>"#);
        assert!(meta_author_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_author_rule_1_wrong_tag() {
        let doc = dom::parse(r#"<article rel="author">Author</article>"#);
        assert!(!meta_author_rule_1(&doc.select("article")));
    }

    #[test]
    fn test_author_rule_2_byline() {
        let doc = dom::parse(r#"<div class="byline">By John</div>"#);
        assert!(meta_author_rule_2(&doc.select("div")));
    }

    #[test]
    fn test_author_rule_2_chinese() {
        let doc = dom::parse(r#"<span class="zuozhe">作者</span>"#);
        assert!(meta_author_rule_2(&doc.select("span")));
    }

    #[test]
    fn test_author_rule_2_journalist() {
        let doc = dom::parse(r#"<div class="journalist-name">Reporter</div>"#);
        assert!(meta_author_rule_2(&doc.select("div")));
    }

    #[test]
    fn test_author_rule_2_username() {
        let doc = dom::parse(r#"<span class="username">user123</span>"#);
        assert!(meta_author_rule_2(&doc.select("span")));
    }

    #[test]
    fn test_author_rule_2_wrong_tag() {
        let doc = dom::parse(r#"<article class="byline">By John</article>"#);
        assert!(!meta_author_rule_2(&doc.select("article")));
    }

    #[test]
    fn test_author_rule_3_case_insensitive() {
        let doc = dom::parse(r#"<div class="Author-Box">John</div>"#);
        assert!(meta_author_rule_3(&doc.select("div")));
    }

    #[test]
    fn test_author_rule_3_screenname() {
        let doc = dom::parse(r#"<span class="screenname">@user</span>"#);
        assert!(meta_author_rule_3(&doc.select("span")));
    }

    #[test]
    fn test_author_rule_3_writer() {
        let doc = dom::parse(r#"<div class="writer">Writer Name</div>"#);
        assert!(meta_author_rule_3(&doc.select("div")));
    }

    // ============================================================
    // AUTHOR DISCARD TESTS
    // ============================================================

    #[test]
    fn test_author_discard_1_avatar() {
        let doc = dom::parse(r#"<span class="author-avatar">avatar</span>"#);
        assert!(meta_author_discard_rule_1(&doc.select("span")));
    }

    #[test]
    fn test_author_discard_1_mailto() {
        let doc = dom::parse(r#"<a href="mailto:author@example.com">Email</a>"#);
        assert!(meta_author_discard_rule_1(&doc.select("a")));
    }

    #[test]
    fn test_author_discard_1_photo() {
        let doc = dom::parse(r#"<div class="author-photo">photo</div>"#);
        assert!(meta_author_discard_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_author_discard_1_wrong_tag() {
        let doc = dom::parse(r#"<img class="avatar" src="photo.jpg">"#);
        assert!(!meta_author_discard_rule_1(&doc.select("img")));
    }

    #[test]
    fn test_author_discard_2_twitter() {
        let doc = dom::parse(r#"<a href="https://twitter.com/user">@user</a>"#);
        assert!(meta_author_discard_rule_2(&doc.select("a")));
    }

    #[test]
    fn test_author_discard_2_facebook() {
        let doc = dom::parse(r#"<a href="https://facebook.com/user">User</a>"#);
        assert!(meta_author_discard_rule_2(&doc.select("a")));
    }

    #[test]
    fn test_author_discard_2_linkedin() {
        let doc = dom::parse(r#"<a href="https://linkedin.com/in/user">Profile</a>"#);
        assert!(meta_author_discard_rule_2(&doc.select("a")));
    }

    #[test]
    fn test_author_discard_2_wrong_tag() {
        let doc = dom::parse(r#"<div href="https://twitter.com/user">@user</div>"#);
        assert!(!meta_author_discard_rule_2(&doc.select("div")));
    }

    #[test]
    fn test_author_discard_3_img() {
        let doc = dom::parse(r#"<img src="photo.jpg">"#);
        assert!(meta_author_discard_rule_3(&doc.select("img")));
    }

    #[test]
    fn test_author_discard_3_figure() {
        let doc = dom::parse("<figure><img src=\"photo.jpg\"></figure>");
        assert!(meta_author_discard_rule_3(&doc.select("figure")));
    }

    // ============================================================
    // TITLE TESTS
    // ============================================================

    #[test]
    fn test_title_rule_1_headline() {
        let doc = dom::parse(r#"<h1 itemprop="headline">Title</h1>"#);
        assert!(meta_title_rule_1(&doc.select("h1")));
    }

    #[test]
    fn test_title_rule_1_class_title() {
        let doc = dom::parse(r#"<h2 class="post-title">Title</h2>"#);
        assert!(meta_title_rule_1(&doc.select("h2")));
    }

    #[test]
    fn test_title_rule_1_id_headline() {
        let doc = dom::parse(r#"<h1 id="article-headline">Title</h1>"#);
        assert!(meta_title_rule_1(&doc.select("h1")));
    }

    #[test]
    fn test_title_rule_1_wrong_tag() {
        let doc = dom::parse(r#"<div itemprop="headline">Title</div>"#);
        assert!(!meta_title_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_title_rule_2_h1() {
        let doc = dom::parse("<h1>Any H1</h1>");
        assert!(meta_title_rule_2(&doc.select("h1")));
    }

    #[test]
    fn test_title_rule_2_not_h1() {
        let doc = dom::parse("<h2>H2 Title</h2>");
        assert!(!meta_title_rule_2(&doc.select("h2")));
    }

    #[test]
    fn test_title_rule_3_entry() {
        let doc = dom::parse(r#"<div class="entry-title">Title</div>"#);
        assert!(meta_title_rule_3(&doc.select("div")));
    }

    #[test]
    fn test_title_rule_3_post_title() {
        let doc = dom::parse(r#"<span class="post-title">Title</span>"#);
        assert!(meta_title_rule_3(&doc.select("span")));
    }

    #[test]
    fn test_title_rule_3_article_title() {
        let doc = dom::parse(r#"<p class="article-title">Title</p>"#);
        assert!(meta_title_rule_3(&doc.select("p")));
    }

    // ============================================================
    // TAG TESTS
    // ============================================================

    #[test]
    fn test_tags_rule_1_container() {
        let doc = dom::parse(r#"<div class="tag-list">tags</div>"#);
        assert!(meta_tags_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_tags_rule_1_id_tags() {
        let doc = dom::parse(r#"<ul id="post-tags">tags</ul>"#);
        assert!(meta_tags_rule_1(&doc.select("ul")));
    }

    #[test]
    fn test_tags_rule_1_keywords() {
        let doc = dom::parse(r#"<div class="keywords">keywords</div>"#);
        assert!(meta_tags_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_tags_rule_1_wrong_tag() {
        let doc = dom::parse(r#"<article class="tags">tags</article>"#);
        assert!(!meta_tags_rule_1(&doc.select("article")));
    }

    #[test]
    fn test_tags_rule_2_rel() {
        let doc = dom::parse(r#"<a rel="tag" href="/tag/news">News</a>"#);
        assert!(meta_tags_rule_2(&doc.select("a")));
    }

    #[test]
    fn test_tags_rule_2_href() {
        let doc = dom::parse(r#"<a href="/tag/tech">Tech</a>"#);
        assert!(meta_tags_rule_2(&doc.select("a")));
    }

    #[test]
    fn test_tags_rule_2_wrong_tag() {
        let doc = dom::parse(r#"<div rel="tag">Tag</div>"#);
        assert!(!meta_tags_rule_2(&doc.select("div")));
    }

    #[test]
    fn test_tags_rule_3_meta_keywords() {
        let doc = dom::parse(r#"<meta name="keywords" content="news,tech">"#);
        assert!(meta_tags_rule_3(&doc.select("meta")));
    }

    #[test]
    fn test_tags_rule_3_news_keywords() {
        let doc = dom::parse(r#"<meta name="news_keywords" content="breaking">"#);
        assert!(meta_tags_rule_3(&doc.select("meta")));
    }

    #[test]
    fn test_tags_rule_3_wrong_tag() {
        let doc = dom::parse(r#"<div name="keywords">keywords</div>"#);
        assert!(!meta_tags_rule_3(&doc.select("div")));
    }

    #[test]
    fn test_tags_rule_4_itemprop() {
        let doc = dom::parse(r#"<span itemprop="keywords">tags</span>"#);
        assert!(meta_tags_rule_4(&doc.select("span")));
    }

    // ============================================================
    // CATEGORY TESTS
    // ============================================================

    #[test]
    fn test_categories_rule_1_container() {
        let doc = dom::parse(r#"<div class="categories">categories</div>"#);
        assert!(meta_categories_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_categories_rule_1_section() {
        let doc = dom::parse(r#"<span id="article-section">Tech</span>"#);
        assert!(meta_categories_rule_1(&doc.select("span")));
    }

    #[test]
    fn test_categories_rule_1_rubric() {
        let doc = dom::parse(r#"<div class="rubric">Opinion</div>"#);
        assert!(meta_categories_rule_1(&doc.select("div")));
    }

    #[test]
    fn test_categories_rule_1_wrong_tag() {
        let doc = dom::parse(r#"<article class="category">Tech</article>"#);
        assert!(!meta_categories_rule_1(&doc.select("article")));
    }

    #[test]
    fn test_categories_rule_2_link() {
        let doc = dom::parse(r#"<a href="/category/tech">Tech</a>"#);
        assert!(meta_categories_rule_2(&doc.select("a")));
    }

    #[test]
    fn test_categories_rule_2_section_link() {
        let doc = dom::parse(r#"<a href="/section/opinion">Opinion</a>"#);
        assert!(meta_categories_rule_2(&doc.select("a")));
    }

    #[test]
    fn test_categories_rule_2_wrong_tag() {
        let doc = dom::parse(r#"<div href="/category/tech">Tech</div>"#);
        assert!(!meta_categories_rule_2(&doc.select("div")));
    }

    #[test]
    fn test_categories_rule_3_meta_category() {
        let doc = dom::parse(r#"<meta name="category" content="Tech">"#);
        assert!(meta_categories_rule_3(&doc.select("meta")));
    }

    #[test]
    fn test_categories_rule_3_article_section() {
        let doc = dom::parse(r#"<meta property="article:section" content="Opinion">"#);
        assert!(meta_categories_rule_3(&doc.select("meta")));
    }

    #[test]
    fn test_categories_rule_3_wrong_tag() {
        let doc = dom::parse(r#"<div name="category">Tech</div>"#);
        assert!(!meta_categories_rule_3(&doc.select("div")));
    }

    #[test]
    fn test_categories_rule_4_breadcrumb() {
        let doc = dom::parse(r#"<nav class="breadcrumb">Home > Tech</nav>"#);
        assert!(meta_categories_rule_4(&doc.select("nav")));
    }

    #[test]
    fn test_categories_rule_4_breadcrumb_ul() {
        let doc = dom::parse(r#"<ul id="breadcrumb-list">items</ul>"#);
        assert!(meta_categories_rule_4(&doc.select("ul")));
    }

    #[test]
    fn test_categories_rule_4_wrong_tag() {
        let doc = dom::parse(r#"<div class="breadcrumb">Home > Tech</div>"#);
        assert!(!meta_categories_rule_4(&doc.select("div")));
    }

    #[test]
    fn test_categories_rule_5_schema() {
        let doc = dom::parse(r#"<span itemprop="articleSection">Tech</span>"#);
        assert!(meta_categories_rule_5(&doc.select("span")));
    }

    #[test]
    fn test_categories_rule_6_data_section() {
        let doc = dom::parse(r#"<div data-section="technology">Tech</div>"#);
        assert!(meta_categories_rule_6(&doc.select("div")));
    }

    #[test]
    fn test_categories_rule_6_section_name() {
        let doc = dom::parse(r#"<span class="section-name">Opinion</span>"#);
        assert!(meta_categories_rule_6(&doc.select("span")));
    }

    // ============================================================
    // HELPER FUNCTION TESTS
    // ============================================================

    #[test]
    fn test_find_authors() {
        let doc = dom::parse(
            r#"
            <div>
                <a rel="author">Author 1</a>
                <span class="byline">Author 2</span>
            </div>
        "#,
        );
        let root = doc.select("div");
        let authors = find_authors(&root);
        // Note: <span class="byline"> matches both rule 2 and rule 3
        assert_eq!(authors.len(), 3);
    }

    #[test]
    fn test_find_titles() {
        let doc = dom::parse(
            r#"
            <div>
                <h1>Main Title</h1>
                <h2 class="post-title">Subtitle</h2>
            </div>
        "#,
        );
        let root = doc.select("div");
        let titles = find_titles(&root);
        // Note: <h2 class="post-title"> matches both rule 1 and rule 3
        assert_eq!(titles.len(), 3);
    }

    #[test]
    fn test_find_tags() {
        let doc = dom::parse(
            r#"
            <div>
                <a rel="tag">Tag1</a>
                <a href="/tag/tech">Tag2</a>
            </div>
        "#,
        );
        let root = doc.select("div");
        let tags = find_tags(&root);
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn test_find_categories() {
        let doc = dom::parse(
            r#"
            <div>
                <a href="/category/tech">Tech</a>
                <span itemprop="articleSection">Opinion</span>
            </div>
        "#,
        );
        let root = doc.select("div");
        let cats = find_categories(&root);
        assert_eq!(cats.len(), 2);
    }

    #[test]
    fn test_is_author_discard() {
        let doc = dom::parse(r#"<img src="author.jpg">"#);
        assert!(is_author_discard(&doc.select("img")));
    }
}
