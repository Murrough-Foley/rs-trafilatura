//! Configuration options for content extraction.
//!
//! The `Options` struct controls extraction behavior, allowing users to
//! tune the precision/recall tradeoff and enable/disable specific features.

/// Configuration options for content extraction.
///
/// All fields are public for easy configuration. Use `Default::default()`
/// for standard settings.
///
/// # Example
///
/// ```rust
/// use rs_trafilatura::Options;
///
/// // Use defaults
/// let options = Options::default();
///
/// // Customize specific fields
/// let options = Options {
///     include_comments: true,
///     favor_precision: true,
///     ..Options::default()
/// };
/// ```
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct Options {
    /// Include comments section in extraction.
    ///
    /// Default: `false`
    pub include_comments: bool,

    /// Include table content in extraction.
    ///
    /// Default: `true`
    pub include_tables: bool,

    /// Include image references in extraction.
    ///
    /// Default: `false`
    pub include_images: bool,

    /// Preserve link URLs in extracted content.
    ///
    /// Default: `false`
    pub include_links: bool,

    /// Tune extraction for higher precision (fewer false positives).
    ///
    /// When enabled, uses stricter content scoring thresholds (`min_score`: 5000)
    /// to exclude borderline content. This reduces false positives at the cost
    /// of potentially missing marginal content.
    ///
    /// If both `favor_precision` and `favor_recall` are true, precision takes
    /// precedence (the stricter threshold is used).
    ///
    /// Default: `false`
    pub favor_precision: bool,

    /// Tune extraction for higher recall (fewer missed content).
    ///
    /// When enabled, uses more lenient content scoring thresholds (`min_score`: 500)
    /// to include borderline content. This reduces false negatives at the cost
    /// of potentially including more noise.
    ///
    /// If both `favor_precision` and `favor_recall` are true, precision takes
    /// precedence (the stricter threshold is used).
    ///
    /// Default: `false`
    pub favor_recall: bool,

    /// Filter content by expected language (ISO 639-1 code).
    ///
    /// Default: `None`
    pub target_language: Option<String>,

    /// Source URL of the document for hostname extraction.
    ///
    /// When provided, the hostname is extracted from this URL and stored
    /// in `metadata.hostname`. This is useful when the HTML doesn't contain
    /// canonical URL information.
    ///
    /// Default: `None`
    pub url: Option<String>,

    /// Author names to filter out during extraction.
    ///
    /// Names containing any of these strings (case-insensitive) will be removed.
    /// Useful for filtering out site-wide bylines or bot names.
    ///
    /// Default: `None`
    pub author_blacklist: Option<Vec<String>>,

    /// Remove duplicate text segments and sections.
    ///
    /// When enabled, uses an LRU cache to track seen text and skip
    /// duplicate content during extraction.
    ///
    /// Default: `false`
    pub deduplicate: bool,

    /// Minimum size of extracted content (character count).
    ///
    /// Used to determine if fallback extraction (wild text recovery)
    /// should be attempted when initial extraction yields insufficient content.
    ///
    /// Default: `200`
    pub min_extracted_size: usize,

    // === Additional threshold fields (Story 6-1) ===

    /// Minimum text length for extracted content (characters).
    ///
    /// Content shorter than this is considered insufficient.
    ///
    /// Default: `200`
    pub min_extracted_len: usize,

    /// Maximum text length for extracted content (characters).
    ///
    /// Prevents extracting excessively long documents.
    ///
    /// Default: `1000000` (1M chars)
    pub max_extracted_len: usize,

    /// Minimum number of words in extracted output.
    ///
    /// Default: `50`
    pub min_output_size: usize,

    /// Minimum number of words for comments section.
    ///
    /// Default: `10`
    pub min_output_comm_size: usize,

    /// Minimum score threshold for content selection.
    ///
    /// Higher values = stricter filtering. Overridden by `favor_precision/favor_recall`.
    ///
    /// Default: `1000`
    pub min_score: usize,

    /// Maximum ratio of duplicate text allowed.
    ///
    /// Ratio of duplicate segments to total text (0.0 - 1.0).
    ///
    /// Default: `0.5`
    pub max_duplicate_ratio: f64,

    /// Maximum proportion of link text in a segment.
    ///
    /// Segments with higher link density may be discarded as navigation.
    ///
    /// Default: `0.8`
    pub max_link_density: f64,

    /// Minimum number of consecutive paragraph tags needed.
    ///
    /// Used in fallback extraction to identify content clusters.
    ///
    /// Default: `3`
    pub min_paragraph_cluster: usize,

    /// Include formatting markup in output.
    ///
    /// When true, preserves basic formatting tags (bold, italic, etc.).
    ///
    /// Default: `false`
    pub include_formatting: bool,

    /// Only extract date, no content.
    ///
    /// Performance optimization when only metadata.date is needed.
    ///
    /// Default: `false`
    pub only_with_metadata: bool,

    /// Maximum tree depth for content extraction.
    ///
    /// Prevents processing overly nested DOM structures.
    ///
    /// Default: `100`
    pub max_tree_depth: usize,

    /// Minimum word length to count as valid.
    ///
    /// Words shorter than this are excluded from word count metrics.
    ///
    /// Default: `2`
    pub min_word_length: usize,

    /// Use readability fallback when main extraction fails.
    ///
    /// Requires `readability` feature flag.
    ///
    /// Default: `true`
    pub use_readability_fallback: bool,

    /// Maximum size of deduplication cache (number of entries).
    ///
    /// Only used when `deduplicate = true`.
    ///
    /// Default: `1000`
    pub dedup_cache_size: usize,

    /// Include title element in output.
    ///
    /// When false, title is only in metadata, not content.
    ///
    /// Default: `false`
    pub include_title_in_content: bool,

    // === EPIC-02: Markdown Output ===
    /// Output extracted content as Markdown.
    ///
    /// When enabled, `ExtractResult.content_markdown` is populated with
    /// GitHub Flavored Markdown preserving document structure (headings,
    /// lists, tables, bold/italic, links, code blocks).
    ///
    /// Default: `false`
    pub output_markdown: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            include_comments: false,
            include_tables: true,
            include_images: false,
            include_links: false,
            favor_precision: false,
            favor_recall: false,
            target_language: None,
            url: None,
            author_blacklist: None,
            deduplicate: false,
            min_extracted_size: 200,
            // Story 6-1: Additional threshold defaults (from go-trafilatura settings.go)
            min_extracted_len: 200,
            max_extracted_len: 1_000_000,
            min_output_size: 50,
            min_output_comm_size: 10,
            min_score: 1000,
            max_duplicate_ratio: 0.5,
            max_link_density: 0.8,
            min_paragraph_cluster: 3,
            include_formatting: false,
            only_with_metadata: false,
            max_tree_depth: 100,
            min_word_length: 2,
            use_readability_fallback: true,
            dedup_cache_size: 1000,
            include_title_in_content: false,
            // EPIC-02: Markdown output
            output_markdown: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options_thresholds() {
        let opts = Options::default();

        // Original fields
        assert!(!opts.include_comments);
        assert!(opts.include_tables);
        assert!(!opts.include_images);
        assert!(!opts.include_links);
        assert!(!opts.favor_precision);
        assert!(!opts.favor_recall);
        assert!(opts.target_language.is_none());
        assert!(opts.url.is_none());
        assert!(opts.author_blacklist.is_none());
        assert!(!opts.deduplicate);
        assert_eq!(opts.min_extracted_size, 200);

        // Story 6-1: New threshold fields
        assert_eq!(opts.min_extracted_len, 200);
        assert_eq!(opts.max_extracted_len, 1_000_000);
        assert_eq!(opts.min_output_size, 50);
        assert_eq!(opts.min_output_comm_size, 10);
        assert_eq!(opts.min_score, 1000);
        assert!((opts.max_duplicate_ratio - 0.5).abs() < f64::EPSILON);
        assert!((opts.max_link_density - 0.8).abs() < f64::EPSILON);
        assert_eq!(opts.min_paragraph_cluster, 3);
        assert!(!opts.include_formatting);
        assert!(!opts.only_with_metadata);
        assert_eq!(opts.max_tree_depth, 100);
        assert_eq!(opts.min_word_length, 2);
        assert!(opts.use_readability_fallback);
        assert_eq!(opts.dedup_cache_size, 1000);
        assert!(!opts.include_title_in_content);
        // EPIC-02: Markdown output
        assert!(!opts.output_markdown);
    }

    #[test]
    fn test_favor_precision_overrides_min_score() {
        let opts = Options {
            favor_precision: true,
            min_score: 1000, // Should be overridden to 5000 in extraction code
            ..Options::default()
        };

        // In extraction code, effective min_score calculation:
        let effective_min_score = if opts.favor_precision {
            5000
        } else if opts.favor_recall {
            500
        } else {
            opts.min_score
        };

        assert_eq!(effective_min_score, 5000);
        assert_eq!(opts.min_score, 1000); // Field value unchanged
    }

    #[test]
    fn test_favor_recall_overrides_min_score() {
        let opts = Options {
            favor_recall: true,
            min_score: 1000,
            ..Options::default()
        };

        let effective_min_score = if opts.favor_precision {
            5000
        } else if opts.favor_recall {
            500
        } else {
            opts.min_score
        };

        assert_eq!(effective_min_score, 500);
        assert_eq!(opts.min_score, 1000);
    }

    #[test]
    fn test_favor_precision_takes_precedence_over_recall() {
        let opts = Options {
            favor_precision: true,
            favor_recall: true, // Both set, precision takes precedence
            ..Options::default()
        };

        let effective_min_score = if opts.favor_precision {
            5000
        } else if opts.favor_recall {
            500
        } else {
            opts.min_score
        };

        assert_eq!(effective_min_score, 5000);
    }

    #[test]
    fn test_custom_thresholds() {
        let opts = Options {
            min_output_size: 100,
            max_link_density: 0.5,
            min_paragraph_cluster: 5,
            max_tree_depth: 50,
            dedup_cache_size: 500,
            ..Options::default()
        };

        assert_eq!(opts.min_output_size, 100);
        assert!((opts.max_link_density - 0.5).abs() < f64::EPSILON);
        assert_eq!(opts.min_paragraph_cluster, 5);
        assert_eq!(opts.max_tree_depth, 50);
        assert_eq!(opts.dedup_cache_size, 500);
    }

    #[test]
    fn test_boolean_options_can_be_toggled() {
        let opts = Options {
            include_formatting: true,
            only_with_metadata: true,
            use_readability_fallback: false,
            include_title_in_content: true,
            ..Options::default()
        };

        assert!(opts.include_formatting);
        assert!(opts.only_with_metadata);
        assert!(!opts.use_readability_fallback);
        assert!(opts.include_title_in_content);
    }
}
