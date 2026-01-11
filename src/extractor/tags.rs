//! Tag constants and catalogs from go-trafilatura.
//!
//! This module ports all tag lists from tag-converter.go and catalogs from settings.go.
//! It provides both arrays (for iteration) and `HashSets` (for O(1) lookup).

use std::collections::HashSet;
use std::sync::LazyLock;

// === Tag Lists (arrays for iteration) ===

/// List tags: ul, ol, dl
pub static XML_LIST_TAGS: [&str; 3] = ["ul", "ol", "dl"];

/// Quote tags: blockquote, pre, q
pub static XML_QUOTE_TAGS: [&str; 3] = ["blockquote", "pre", "q"];

/// Head tags: h1-h6, summary
pub static XML_HEAD_TAGS: [&str; 7] = ["h1", "h2", "h3", "h4", "h5", "h6", "summary"];

/// Line break tags: br, hr, lb
pub static XML_LB_TAGS: [&str; 3] = ["br", "hr", "lb"];

/// Highlight/formatting tags: em, i, b, strong, u, kbd, samp, tt, var, sub, sup, mark
pub static XML_HI_TAGS: [&str; 12] = [
    "em", "i", "b", "strong", "u", "kbd", "samp", "tt", "var", "sub", "sup", "mark",
];

/// Reference tags: a
pub static XML_REF_TAGS: [&str; 1] = ["a"];

/// Graphic tags: img
pub static XML_GRAPHIC_TAGS: [&str; 1] = ["img"];

/// Item tags: dd, dt, li
pub static XML_ITEM_TAGS: [&str; 3] = ["dd", "dt", "li"];

/// Cell tags: th, td
pub static XML_CELL_TAGS: [&str; 2] = ["th", "td"];

// === Tag Sets (HashSets for O(1) lookup) ===

/// `XML_LIST_TAGS` as a `HashSet`
pub static XML_LIST_TAG_SET: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    XML_LIST_TAGS.into_iter().collect()
});

/// `XML_QUOTE_TAGS` as a `HashSet`
pub static XML_QUOTE_TAG_SET: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    XML_QUOTE_TAGS.into_iter().collect()
});

/// `XML_HEAD_TAGS` as a `HashSet`
pub static XML_HEAD_TAG_SET: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    XML_HEAD_TAGS.into_iter().collect()
});

/// `XML_LB_TAGS` as a `HashSet`
pub static XML_LB_TAG_SET: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    XML_LB_TAGS.into_iter().collect()
});

/// `XML_HI_TAGS` as a `HashSet`
pub static XML_HI_TAG_SET: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    XML_HI_TAGS.into_iter().collect()
});

/// `XML_REF_TAGS` as a `HashSet`
pub static XML_REF_TAG_SET: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    XML_REF_TAGS.into_iter().collect()
});

/// `XML_GRAPHIC_TAGS` as a `HashSet`
pub static XML_GRAPHIC_TAG_SET: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    XML_GRAPHIC_TAGS.into_iter().collect()
});

/// `XML_ITEM_TAGS` as a `HashSet`
pub static XML_ITEM_TAG_SET: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    XML_ITEM_TAGS.into_iter().collect()
});

/// `XML_CELL_TAGS` as a `HashSet`
pub static XML_CELL_TAG_SET: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    XML_CELL_TAGS.into_iter().collect()
});

// === Cleaning Tag Lists ===

/// Tags to completely remove (including children) during document cleaning.
/// From go-trafilatura settings.go `tagsToClean`.
pub static TAGS_TO_CLEAN: [&str; 50] = [
    // important
    "aside", "embed", "footer", "form", "head", "iframe", "menu", "object", "script",
    // other content
    "applet", "audio", "canvas", "figure", "map", "picture", "svg", "video",
    // secondary
    "area", "blink", "button", "datalist", "dialog", "frame", "frameset", "fieldset",
    "link", "input", "ins", "label", "legend", "marquee", "math", "menuitem", "nav",
    "noscript", "optgroup", "option", "output", "param", "progress", "rp", "rt", "rtc",
    "select", "source", "style", "track", "textarea", "time", "use",
];

/// `TAGS_TO_CLEAN` as a `HashSet`
pub static TAGS_TO_CLEAN_SET: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    TAGS_TO_CLEAN.into_iter().collect()
});

/// Tags to strip (remove tag but keep children) during document cleaning.
/// From go-trafilatura settings.go `tagsToStrip`.
pub static TAGS_TO_STRIP: [&str; 18] = [
    "abbr", "acronym", "address", "bdi", "bdo", "big", "cite", "data", "dfn", "font",
    "hgroup", "img", "ins", "mark", "meta", "ruby", "small", "template",
];

/// `TAGS_TO_STRIP` as a `HashSet`
pub static TAGS_TO_STRIP_SET: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    TAGS_TO_STRIP.into_iter().collect()
});

/// Tags to remove if they are empty (no text content).
/// From go-trafilatura settings.go `emptyTagsToRemove`.
pub static EMPTY_TAGS_TO_REMOVE: [&str; 22] = [
    "article", "b", "blockquote", "dd", "div", "dt", "em",
    "h1", "h2", "h3", "h4", "h5", "h6", "i", "li", "main",
    "p", "pre", "q", "section", "span", "strong",
];

/// `EMPTY_TAGS_TO_REMOVE` as a `HashSet`
pub static EMPTY_TAGS_TO_REMOVE_SET: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    EMPTY_TAGS_TO_REMOVE.into_iter().collect()
});

/// Table structure tags to strip (keep children).
/// From go-trafilatura settings.go `tagsToStrip` (table-related subset).
pub static TABLE_TAGS_TO_STRIP: [&str; 3] = ["tbody", "tfoot", "thead"];

/// `TABLE_TAGS_TO_STRIP` as a `HashSet`
pub static TABLE_TAGS_TO_STRIP_SET: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    TABLE_TAGS_TO_STRIP.into_iter().collect()
});

// === Helper Functions for Cleaning ===

/// Check if tag should be completely removed during cleaning
#[inline]
#[must_use]
pub fn should_clean_tag(tag: &str) -> bool {
    TAGS_TO_CLEAN_SET.contains(tag)
}

/// Check if tag should be stripped (keep children) during cleaning
#[inline]
#[must_use]
pub fn should_strip_tag(tag: &str) -> bool {
    TAGS_TO_STRIP_SET.contains(tag)
}

/// Check if tag should be removed when empty
#[inline]
#[must_use]
pub fn should_remove_if_empty(tag: &str) -> bool {
    EMPTY_TAGS_TO_REMOVE_SET.contains(tag)
}

// === Catalogs ===

/// `TAG_CATALOG` from settings.go - default set of content tags
pub static TAG_CATALOG: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "blockquote", "code",
        "del", "s", "strike",
        "h1", "h2", "h3", "h4", "h5", "h6",
        "em", "i", "b", "strong", "u", "kbd", "samp", "tt", "var", "sub", "sup",
        "br", "hr",
        "ul", "ol", "dl",
        "p", "pre", "q",
        "details", "summary",
    ].into_iter().collect()
});

/// `FORMAT_TAG_CATALOG` from settings.go - formatting tags
pub static FORMAT_TAG_CATALOG: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "em", "i", "b", "strong", "u", "kbd",
        "samp", "tt", "var", "sub", "sup",
    ].into_iter().collect()
});

/// `VALID_TAG_CATALOG` from settings.go - all valid HTML tags
pub static VALID_TAG_CATALOG: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "a", "abbr", "address", "area", "b", "base", "bdo", "blockquote", "body", "br", "button",
        "caption", "cite", "code", "col", "colgroup", "dd", "del", "dfn", "div", "dl", "dt", "em",
        "fieldset", "form", "h1", "h2", "h3", "h4", "h5", "h6", "head", "hr", "html", "i", "iframe",
        "img", "input", "ins", "kbd", "label", "legend", "li", "link", "map", "menu", "meta",
        "noscript", "object", "ol", "optgroup", "option", "p", "param", "pre", "q", "s", "samp",
        "script", "select", "small", "span", "strong", "style", "sub", "sup", "table", "tbody",
        "td", "textarea", "tfoot", "th", "thead", "title", "tr", "u", "ul", "var", "article",
        "aside", "audio", "canvas", "command", "datalist", "details", "embed", "figcaption",
        "figure", "footer", "header", "mark", "meter", "nav", "output", "progress", "rp", "rt",
        "ruby", "section", "source", "summary", "time", "track", "video", "wbr",
    ].into_iter().collect()
});

// === Helper Functions ===

/// Check if tag is a list tag (ul, ol, dl)
#[inline]
#[must_use]
pub fn is_xml_list_tag(tag: &str) -> bool {
    XML_LIST_TAG_SET.contains(tag)
}

/// Check if tag is a quote tag (blockquote, pre, q)
#[inline]
#[must_use]
pub fn is_xml_quote_tag(tag: &str) -> bool {
    XML_QUOTE_TAG_SET.contains(tag)
}

/// Check if tag is a head tag (h1-h6, summary)
#[inline]
#[must_use]
pub fn is_xml_head_tag(tag: &str) -> bool {
    XML_HEAD_TAG_SET.contains(tag)
}

/// Check if tag is a line break tag (br, hr, lb)
#[inline]
#[must_use]
pub fn is_xml_lb_tag(tag: &str) -> bool {
    XML_LB_TAG_SET.contains(tag)
}

/// Check if tag is a highlight/formatting tag (em, i, b, strong, etc.)
#[inline]
#[must_use]
pub fn is_xml_hi_tag(tag: &str) -> bool {
    XML_HI_TAG_SET.contains(tag)
}

/// Check if tag is a reference tag (a)
#[inline]
#[must_use]
pub fn is_xml_ref_tag(tag: &str) -> bool {
    XML_REF_TAG_SET.contains(tag)
}

/// Check if tag is a graphic tag (img)
#[inline]
#[must_use]
pub fn is_xml_graphic_tag(tag: &str) -> bool {
    XML_GRAPHIC_TAG_SET.contains(tag)
}

/// Check if tag is an item tag (dd, dt, li)
#[inline]
#[must_use]
pub fn is_xml_item_tag(tag: &str) -> bool {
    XML_ITEM_TAG_SET.contains(tag)
}

/// Check if tag is a cell tag (th, td)
#[inline]
#[must_use]
pub fn is_xml_cell_tag(tag: &str) -> bool {
    XML_CELL_TAG_SET.contains(tag)
}

/// Check if tag is in `TAG_CATALOG`
#[inline]
#[must_use]
pub fn is_in_tag_catalog(tag: &str) -> bool {
    TAG_CATALOG.contains(tag)
}

/// Check if tag is a format tag
#[inline]
#[must_use]
pub fn is_format_tag(tag: &str) -> bool {
    FORMAT_TAG_CATALOG.contains(tag)
}

/// Check if tag is a valid HTML tag
#[inline]
#[must_use]
pub fn is_valid_tag(tag: &str) -> bool {
    VALID_TAG_CATALOG.contains(tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_list_tags() {
        assert!(is_xml_list_tag("ul"));
        assert!(is_xml_list_tag("ol"));
        assert!(is_xml_list_tag("dl"));
        assert!(!is_xml_list_tag("li"));
    }

    #[test]
    fn test_xml_quote_tags() {
        assert!(is_xml_quote_tag("blockquote"));
        assert!(is_xml_quote_tag("pre"));
        assert!(is_xml_quote_tag("q"));
        assert!(!is_xml_quote_tag("p"));
    }

    #[test]
    fn test_xml_head_tags() {
        assert!(is_xml_head_tag("h1"));
        assert!(is_xml_head_tag("h6"));
        assert!(is_xml_head_tag("summary"));
        assert!(!is_xml_head_tag("p"));
    }

    #[test]
    fn test_xml_lb_tags() {
        assert!(is_xml_lb_tag("br"));
        assert!(is_xml_lb_tag("hr"));
        assert!(is_xml_lb_tag("lb"));
        assert!(!is_xml_lb_tag("p"));
    }

    #[test]
    fn test_xml_hi_tags() {
        assert!(is_xml_hi_tag("em"));
        assert!(is_xml_hi_tag("strong"));
        assert!(is_xml_hi_tag("mark"));
        assert!(!is_xml_hi_tag("p"));
    }

    #[test]
    fn test_xml_ref_tags() {
        assert!(is_xml_ref_tag("a"));
        assert!(!is_xml_ref_tag("link"));
    }

    #[test]
    fn test_xml_graphic_tags() {
        assert!(is_xml_graphic_tag("img"));
        assert!(!is_xml_graphic_tag("picture"));
    }

    #[test]
    fn test_xml_item_tags() {
        assert!(is_xml_item_tag("li"));
        assert!(is_xml_item_tag("dd"));
        assert!(is_xml_item_tag("dt"));
        assert!(!is_xml_item_tag("ul"));
    }

    #[test]
    fn test_xml_cell_tags() {
        assert!(is_xml_cell_tag("td"));
        assert!(is_xml_cell_tag("th"));
        assert!(!is_xml_cell_tag("tr"));
    }

    #[test]
    fn test_tag_catalog() {
        assert!(is_in_tag_catalog("p"));
        assert!(is_in_tag_catalog("blockquote"));
        assert!(is_in_tag_catalog("h1"));
        assert!(!is_in_tag_catalog("div"));  // div not in default catalog
        assert!(!is_in_tag_catalog("table")); // table added via options
    }

    #[test]
    fn test_format_tag_catalog() {
        assert!(is_format_tag("em"));
        assert!(is_format_tag("strong"));
        assert!(is_format_tag("kbd"));
        assert!(!is_format_tag("p"));
        assert!(!is_format_tag("blockquote"));
    }

    #[test]
    fn test_valid_tag_catalog() {
        assert!(is_valid_tag("div"));
        assert!(is_valid_tag("table"));
        assert!(is_valid_tag("article"));
        assert!(!is_valid_tag("custom-element"));
    }

    #[test]
    fn test_all_tags_in_sets() {
        // Verify all tags in arrays are in corresponding sets
        for tag in &XML_LIST_TAGS {
            assert!(XML_LIST_TAG_SET.contains(tag));
        }
        for tag in &XML_QUOTE_TAGS {
            assert!(XML_QUOTE_TAG_SET.contains(tag));
        }
        for tag in &XML_HEAD_TAGS {
            assert!(XML_HEAD_TAG_SET.contains(tag));
        }
        for tag in &XML_LB_TAGS {
            assert!(XML_LB_TAG_SET.contains(tag));
        }
        for tag in &XML_HI_TAGS {
            assert!(XML_HI_TAG_SET.contains(tag));
        }
        for tag in &XML_REF_TAGS {
            assert!(XML_REF_TAG_SET.contains(tag));
        }
        for tag in &XML_GRAPHIC_TAGS {
            assert!(XML_GRAPHIC_TAG_SET.contains(tag));
        }
        for tag in &XML_ITEM_TAGS {
            assert!(XML_ITEM_TAG_SET.contains(tag));
        }
        for tag in &XML_CELL_TAGS {
            assert!(XML_CELL_TAG_SET.contains(tag));
        }
    }

    // === Tests for Cleaning Tags ===

    #[test]
    fn test_tags_to_clean() {
        // Important tags
        assert!(should_clean_tag("script"));
        assert!(should_clean_tag("style"));
        assert!(should_clean_tag("nav"));
        assert!(should_clean_tag("footer"));
        assert!(should_clean_tag("aside"));
        assert!(should_clean_tag("form"));
        // Media tags
        assert!(should_clean_tag("video"));
        assert!(should_clean_tag("audio"));
        assert!(should_clean_tag("svg"));
        // Not in clean list
        assert!(!should_clean_tag("div"));
        assert!(!should_clean_tag("p"));
        assert!(!should_clean_tag("article"));
    }

    #[test]
    fn test_tags_to_strip() {
        assert!(should_strip_tag("abbr"));
        assert!(should_strip_tag("font"));
        assert!(should_strip_tag("img"));
        assert!(should_strip_tag("meta"));
        assert!(should_strip_tag("template"));
        // Not in strip list
        assert!(!should_strip_tag("div"));
        assert!(!should_strip_tag("p"));
        assert!(!should_strip_tag("script"));
    }

    #[test]
    fn test_empty_tags_to_remove() {
        assert!(should_remove_if_empty("div"));
        assert!(should_remove_if_empty("p"));
        assert!(should_remove_if_empty("span"));
        assert!(should_remove_if_empty("article"));
        assert!(should_remove_if_empty("h1"));
        assert!(should_remove_if_empty("h6"));
        // Not in empty removal list
        assert!(!should_remove_if_empty("table"));
        assert!(!should_remove_if_empty("a"));
        assert!(!should_remove_if_empty("script"));
    }

    #[test]
    fn test_cleaning_tags_in_sets() {
        // Verify all cleaning tags in arrays are in corresponding sets
        for tag in &TAGS_TO_CLEAN {
            assert!(TAGS_TO_CLEAN_SET.contains(tag), "Missing in set: {tag}");
        }
        for tag in &TAGS_TO_STRIP {
            assert!(TAGS_TO_STRIP_SET.contains(tag), "Missing in set: {tag}");
        }
        for tag in &EMPTY_TAGS_TO_REMOVE {
            assert!(EMPTY_TAGS_TO_REMOVE_SET.contains(tag), "Missing in set: {tag}");
        }
        for tag in &TABLE_TAGS_TO_STRIP {
            assert!(TABLE_TAGS_TO_STRIP_SET.contains(tag), "Missing in set: {tag}");
        }
    }

    #[test]
    fn test_cleaning_tag_counts() {
        // Verify tag counts match go-trafilatura settings.go
        assert_eq!(TAGS_TO_CLEAN.len(), 50, "tagsToClean should have 50 tags");
        assert_eq!(TAGS_TO_STRIP.len(), 18, "tagsToStrip should have 18 tags");
        assert_eq!(EMPTY_TAGS_TO_REMOVE.len(), 22, "emptyTagsToRemove should have 22 tags");
    }
}
