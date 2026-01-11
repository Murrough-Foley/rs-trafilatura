//! Extraction state tracking.
//!
//! This module provides `ExtractionState` to track processed nodes and potential tags.
//! Replaces Go's `element.Data = "done"` pattern with HashSet-based tracking.

use std::collections::HashSet;
use dom_query::NodeId;

/// Tracks extraction state including processed nodes and potential tags.
///
/// # Purpose
///
/// - **Processed Nodes**: Tracks which nodes have been processed to avoid duplication
///   (replaces Go's `element.Data = "done"` pattern)
/// - **Potential Tags**: Set of tag names that are considered potential content tags
///   for this extraction (configured based on Options)
pub struct ExtractionState {
    /// Set of node IDs that have been processed (equivalent to Go's element.Data = "done")
    processed_nodes: HashSet<NodeId>,

    /// Set of tag names that are potential content tags for this extraction
    potential_tags: HashSet<String>,
}

impl ExtractionState {
    /// Create new extraction state with default potential tags from `TAG_CATALOG`
    ///
    /// The default potential tags include: p, blockquote, code, h1-h6, formatting tags, etc.
    /// Additional tags (table, img, a) can be added via `configure_from_options()`.
    #[must_use]
    pub fn new() -> Self {
        use super::tags::TAG_CATALOG;

        Self {
            processed_nodes: HashSet::new(),
            potential_tags: TAG_CATALOG.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    /// Mark a node as processed
    ///
    /// Equivalent to setting `element.Data = "done"` in go-trafilatura.
    pub fn mark_done(&mut self, node_id: NodeId) {
        self.processed_nodes.insert(node_id);
    }

    /// Check if a node has been processed
    ///
    /// Equivalent to checking `element.Data == "done"` in go-trafilatura.
    #[must_use]
    pub fn is_done(&self, node_id: NodeId) -> bool {
        self.processed_nodes.contains(&node_id)
    }

    /// Check if a tag is a potential content tag
    #[must_use]
    pub fn is_potential_tag(&self, tag: &str) -> bool {
        self.potential_tags.contains(tag)
    }

    /// Add a tag to potential tags
    pub fn add_potential_tag(&mut self, tag: &str) {
        self.potential_tags.insert(tag.to_string());
    }

    /// Remove a tag from potential tags
    pub fn remove_potential_tag(&mut self, tag: &str) {
        self.potential_tags.remove(tag);
    }

    /// Configure potential tags based on extraction options
    ///
    /// Matches the logic from go-trafilatura's main-extractor.go lines 666-686
    /// where potentialTags is configured based on options.
    pub fn configure_from_options(&mut self, opts: &crate::Options) {
        // Add table tags if tables included
        if opts.include_tables {
            self.add_potential_tag("table");
            self.add_potential_tag("tr");
            self.add_potential_tag("th");
            self.add_potential_tag("td");
        }

        // Add image tag if images included
        if opts.include_images {
            self.add_potential_tag("img");
        }

        // Add link tag if links included
        if opts.include_links {
            self.add_potential_tag("a");
        }
    }

    /// Get reference to potential tags set
    #[must_use]
    pub fn potential_tags(&self) -> &HashSet<String> {
        &self.potential_tags
    }

    /// Clone potential tags (for modifications without affecting original)
    #[must_use]
    pub fn clone_potential_tags(&self) -> HashSet<String> {
        self.potential_tags.clone()
    }
}

impl Default for ExtractionState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Options;

    #[test]
    fn test_extraction_state_done_tracking() {
        use dom_query::Document;

        let mut state = ExtractionState::new();

        // Create a real DOM node to get a valid NodeId
        let doc = Document::from("<p>test</p>");
        let node = doc.select("p");
        let node_id = node.nodes().first().unwrap().id;

        assert!(!state.is_done(node_id));
        state.mark_done(node_id);
        assert!(state.is_done(node_id));
    }

    #[test]
    fn test_extraction_state_potential_tags_default() {
        let state = ExtractionState::new();

        assert!(state.is_potential_tag("p"));
        assert!(state.is_potential_tag("h1"));
        assert!(state.is_potential_tag("blockquote"));
        assert!(!state.is_potential_tag("table")); // not in default
        assert!(!state.is_potential_tag("img"));   // not in default
        assert!(!state.is_potential_tag("a"));     // not in default (added with include_links)
    }

    #[test]
    fn test_extraction_state_configure_with_tables() {
        let mut state = ExtractionState::new();
        let opts = Options {
            include_tables: true,
            ..Options::default()
        };

        state.configure_from_options(&opts);

        assert!(state.is_potential_tag("table"));
        assert!(state.is_potential_tag("tr"));
        assert!(state.is_potential_tag("th"));
        assert!(state.is_potential_tag("td"));
    }

    #[test]
    fn test_extraction_state_configure_with_images() {
        let mut state = ExtractionState::new();
        let opts = Options {
            include_images: true,
            ..Options::default()
        };

        state.configure_from_options(&opts);

        assert!(state.is_potential_tag("img"));
    }

    #[test]
    fn test_extraction_state_configure_with_links() {
        let mut state = ExtractionState::new();
        let opts = Options {
            include_links: true,
            ..Options::default()
        };

        state.configure_from_options(&opts);

        assert!(state.is_potential_tag("a"));
    }

    #[test]
    fn test_extraction_state_configure_all_options() {
        let mut state = ExtractionState::new();
        let opts = Options {
            include_tables: true,
            include_images: true,
            include_links: true,
            ..Options::default()
        };

        state.configure_from_options(&opts);

        assert!(state.is_potential_tag("table"));
        assert!(state.is_potential_tag("img"));
        assert!(state.is_potential_tag("a"));
    }

    #[test]
    fn test_add_remove_potential_tag() {
        let mut state = ExtractionState::new();

        assert!(!state.is_potential_tag("custom-tag"));
        state.add_potential_tag("custom-tag");
        assert!(state.is_potential_tag("custom-tag"));
        state.remove_potential_tag("custom-tag");
        assert!(!state.is_potential_tag("custom-tag"));
    }

    #[test]
    fn test_clone_potential_tags() {
        let mut state = ExtractionState::new();
        state.add_potential_tag("custom");

        let cloned = state.clone_potential_tags();
        assert!(cloned.contains("custom"));
        assert!(cloned.contains("p"));
    }

    #[test]
    fn test_multiple_nodes_done_tracking() {
        use dom_query::Document;

        let mut state = ExtractionState::new();

        // Create a real DOM with multiple nodes
        let doc = Document::from("<div><p>one</p><p>two</p><p>three</p></div>");
        let nodes: Vec<_> = doc.select("p").nodes().iter().map(|n| n.id).collect();
        let node1 = nodes[0];
        let node2 = nodes[1];
        let node3 = nodes[2];

        state.mark_done(node1);
        state.mark_done(node3);

        assert!(state.is_done(node1));
        assert!(!state.is_done(node2));
        assert!(state.is_done(node3));
    }
}
