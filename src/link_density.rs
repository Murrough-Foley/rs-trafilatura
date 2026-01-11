//! Link Density Testing
//!
//! This module ports link density testing from go-trafilatura's html-processing.go.
//! It checks whether sections should be removed because they're rich in links (probably boilerplate).

use dom_query::Selection;
use crate::dom;
use crate::Options;

/// Result of link density test
pub struct LinkDensityResult {
    /// Links that are non-empty (have text content)
    pub non_empty_links: Vec<dom_query::NodeRef<'static>>,
    /// Whether the element should be removed due to high link density
    pub should_remove: bool,
}

/// Collect heuristics on link text.
///
/// Go equivalent: `collectLinkInfo` (html-processing.go:342-360)
///
/// Returns (total_link_length, num_short_links, non_empty_links)
fn collect_link_info(links: &Selection) -> (usize, usize, usize) {
    let mut link_length = 0;
    let mut n_short_links = 0;
    let mut n_non_empty_links = 0;

    for link in links.iter() {
        let text = link.text().to_string();
        let text = text.trim();
        let text_length = text.chars().count();

        if text_length == 0 {
            continue;
        }

        link_length += text_length;
        if text_length < 10 {
            n_short_links += 1;
        }
        n_non_empty_links += 1;
    }

    (link_length, n_short_links, n_non_empty_links)
}

/// Check whether sections will be removed because they're rich in links (probably boilerplate).
///
/// Go equivalent: `linkDensityTest` (html-processing.go:246-306)
///
/// Returns true if the element should be removed due to high link density.
#[must_use]
pub fn link_density_test(element: &Selection, options: &Options) -> bool {
    // Fetch links in node
    let links = element.select("a");
    let n_links = links.length();

    if n_links == 0 {
        return false;
    }

    // Get element text
    // EPIC-05: Remove unnecessary .to_string() - use StrTendril directly
    // Note: trim() returns &str, so we must bind the StrTendril first
    let text_tendril = dom::text_content(element);
    let text = text_tendril.trim();
    let text_length = text.chars().count();

    // Shortcut for single link
    if n_links == 1 {
        let threshold: usize = if options.favor_precision { 10 } else { 100 };

        if let Some(link_node) = links.nodes().first() {
            let link = Selection::from(*link_node);
            let link_tendril = dom::text_content(&link);
            let link_text = link_tendril.trim();
            let link_text_length = link_text.chars().count();

            if link_text_length > threshold
                && (link_text_length as f64) > (text_length as f64) * 0.9
            {
                return true;
            }
        }
    }

    // Get tag name for limit calculation
    let tag_name = dom::tag_name(element).unwrap_or_default().to_ascii_lowercase();

    // Check if element has a next sibling
    let has_next_sibling = element
        .nodes()
        .first()
        .and_then(dom_query::NodeRef::next_element_sibling)
        .is_some();

    // Prepare limit based on tag and sibling presence
    let limit_length: usize = if tag_name == "p" {
        if has_next_sibling { 30 } else { 60 }
    } else if has_next_sibling { 100 } else { 300 };

    // Check if text of this node is within limit
    if text_length < limit_length {
        // Collect link info
        let (link_length, n_short_links, n_non_empty_links) = collect_link_info(&links);

        if n_non_empty_links == 0 {
            return true;
        }

        // Check if links data surpass threshold
        // Link text > 80% of total text
        if (link_length as f64) > (text_length as f64) * 0.8 {
            return true;
        }

        // More than 80% of links are short (< 10 chars) - typical of nav menus
        if n_non_empty_links > 1
            && (n_short_links as f64) / (n_non_empty_links as f64) > 0.8
        {
            return true;
        }
    }

    false
}

/// Check link density and return both density result and whether non-empty links exist.
///
/// This is used by `delete_by_link_density` which needs to know about non-empty links
/// for its backtracking logic.
///
/// Returns (has_non_empty_links, is_high_density)
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn link_density_test_with_info(element: &Selection, options: &Options) -> (bool, bool) {
    // Fetch links in node
    let links = element.select("a");
    let n_links = links.length();

    if n_links == 0 {
        return (false, false);
    }

    // Get element text
    // EPIC-05: Remove unnecessary .to_string() - use StrTendril directly
    // Note: trim() returns &str, so we must bind the StrTendril first
    let text_tendril = dom::text_content(element);
    let text = text_tendril.trim();
    let text_length = text.chars().count();

    // Shortcut for single link
    if n_links == 1 {
        let threshold: usize = if options.favor_precision { 10 } else { 100 };

        if let Some(link_node) = links.nodes().first() {
            let link = Selection::from(*link_node);
            let link_tendril = dom::text_content(&link);
            let link_text = link_tendril.trim();
            let link_text_length = link_text.chars().count();

            if link_text_length > threshold
                && (link_text_length as f64) > (text_length as f64) * 0.9
            {
                return (true, true);
            }
        }
    }

    // Get tag name for limit calculation
    let tag_name = dom::tag_name(element).unwrap_or_default().to_ascii_lowercase();

    // Check if element has a next sibling
    let has_next_sibling = element
        .nodes()
        .first()
        .and_then(dom_query::NodeRef::next_element_sibling)
        .is_some();

    // Prepare limit based on tag and sibling presence
    let limit_length: usize = if tag_name == "p" {
        if has_next_sibling { 30 } else { 60 }
    } else if has_next_sibling { 100 } else { 300 };

    // Check if text of this node is within limit
    if text_length < limit_length {
        // Collect link info
        let (link_length, n_short_links, n_non_empty_links) = collect_link_info(&links);

        if n_non_empty_links == 0 {
            return (false, true);
        }

        // Check if links data surpass threshold
        // Link text > 80% of total text
        if (link_length as f64) > (text_length as f64) * 0.8 {
            return (true, true);
        }

        // More than 80% of links are short (< 10 chars) - typical of nav menus
        if n_non_empty_links > 1
            && (n_short_links as f64) / (n_non_empty_links as f64) > 0.8
        {
            return (true, true);
        }

        // Has non-empty links but not high density
        return (true, false);
    }

    // Has links but didn't meet threshold criteria
    let (_, _, n_non_empty_links) = collect_link_info(&links);
    (n_non_empty_links > 0, false)
}

/// Check whether a table will be removed because it's rich in links (probably boilerplate).
///
/// Go equivalent: `linkDensityTestTables` (html-processing.go:312-340)
#[must_use]
pub fn link_density_test_tables(table: &Selection, _options: &Options) -> bool {
    // Fetch links in table
    let links = table.select("a");
    if links.length() == 0 {
        return false;
    }

    // Check text length
    // EPIC-05: Remove unnecessary .to_string() - use StrTendril directly
    // Note: trim() returns &str, so we must bind the StrTendril first
    let text_tendril = dom::text_content(table);
    let text = text_tendril.trim();
    let text_length = text.chars().count();

    if text_length < 200 {
        return false;
    }

    // Check link info
    let (link_length, _, n_non_empty_links) = collect_link_info(&links);

    if n_non_empty_links == 0 {
        return true;
    }

    // Different thresholds based on table size
    if text_length < 1000 {
        (link_length as f64) > (text_length as f64) * 0.8
    } else {
        (link_length as f64) > (text_length as f64) * 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dom_query::Document;

    #[test]
    fn test_link_density_nav_menu() {
        let html = r#"
        <div>
            <a href="/home">Home</a>
            <a href="/about">About</a>
            <a href="/contact">Contact</a>
            <a href="/blog">Blog</a>
        </div>
        "#;
        let doc = Document::from(html);
        let div = doc.select("div");
        let options = Options::default();

        // Nav menu with mostly links should be flagged
        assert!(link_density_test(&div, &options));
    }

    #[test]
    fn test_link_density_article_paragraph() {
        let html = r#"
        <p>
            This is a long paragraph with substantial text content that discusses 
            various topics. It contains a <a href="/link">single link</a> but the 
            majority of the content is regular text, not links. This should not be 
            flagged as link-heavy boilerplate content.
        </p>
        "#;
        let doc = Document::from(html);
        let p = doc.select("p");
        let options = Options::default();

        // Article paragraph with one link should NOT be flagged
        assert!(!link_density_test(&p, &options));
    }

    #[test]
    fn test_link_density_no_links() {
        let html = r#"<p>This paragraph has no links at all.</p>"#;
        let doc = Document::from(html);
        let p = doc.select("p");
        let options = Options::default();

        // No links = not flagged
        assert!(!link_density_test(&p, &options));
    }

    #[test]
    fn test_link_density_table_nav() {
        // Table needs >200 chars of text to trigger the check
        // This simulates a navigation table with many links
        let html = r#"
        <table>
            <tr><td><a href="/1">Navigation Link Category One Section</a></td><td><a href="/2">Navigation Link Category Two Section</a></td></tr>
            <tr><td><a href="/3">Navigation Link Category Three Section</a></td><td><a href="/4">Navigation Link Category Four Section</a></td></tr>
            <tr><td><a href="/5">Navigation Link Category Five Section</a></td><td><a href="/6">Navigation Link Category Six Section</a></td></tr>
            <tr><td><a href="/7">Navigation Link Category Seven Section</a></td><td><a href="/8">Navigation Link Category Eight Section</a></td></tr>
            <tr><td><a href="/9">Navigation Link Category Nine Section</a></td><td><a href="/10">Navigation Link Category Ten Section</a></td></tr>
            <tr><td><a href="/11">Navigation Link Category Eleven Section</a></td><td><a href="/12">Navigation Link Category Twelve Section</a></td></tr>
        </table>
        "#;
        let doc = Document::from(html);
        let table = doc.select("table");
        let text = crate::dom::text_content(&table);
        let text_len = text.trim().chars().count();
        let options = Options::default();

        // Verify we have enough text (>200 chars)
        assert!(text_len > 200, "Table text length {text_len} should be > 200");
        
        // Table with mostly links should be flagged (text > 200 chars, link ratio > 80%)
        assert!(link_density_test_tables(&table, &options));
    }

    #[test]
    fn test_link_density_data_table() {
        let html = r#"
        <table>
            <tr><th>Name</th><th>Score</th><th>Date</th></tr>
            <tr><td>John Smith</td><td>95</td><td>2024-01-15</td></tr>
            <tr><td>Jane Doe</td><td>87</td><td>2024-01-16</td></tr>
            <tr><td>Bob Wilson</td><td>92</td><td>2024-01-17</td></tr>
            <tr><td>Alice Brown</td><td>88</td><td>2024-01-18</td></tr>
            <tr><td>Charlie Davis</td><td>91</td><td>2024-01-19</td></tr>
        </table>
        "#;
        let doc = Document::from(html);
        let table = doc.select("table");
        let options = Options::default();

        // Data table without links should NOT be flagged
        assert!(!link_density_test_tables(&table, &options));
    }
}
