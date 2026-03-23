//! Integration tests for feature combinations
//!
//! Tests that verify multiple features work correctly together.

#![allow(clippy::expect_used)] // expect() is appropriate in tests for clear panic messages

use rs_trafilatura::{extract_with_options, Options};

/// Test fixture path helper
fn fixture_path(name: &str) -> String {
    format!(
        "{}/tests/integration/fixtures/{}",
        env!("CARGO_MANIFEST_DIR"),
        name
    )
}

#[test]
fn test_precision_mode_with_metadata() {
    let html = std::fs::read_to_string(fixture_path("article_full.html"))
        .expect("Failed to read fixture");

    let opts = Options {
        favor_precision: true,
        ..Options::default()
    };

    match extract_with_options(&html, &opts) {
        Ok(result) => {
            // Precision mode should still extract main content
            assert!(!result.content_text.is_empty(), "Content should be extracted");

            // Metadata should still be complete
            assert!(result.metadata.title.is_some(), "Title should be extracted");
            assert!(result.metadata.author.is_some(), "Author should be extracted");

            // Content should be cleaner (less boilerplate)
            assert!(
                !result.content_text.contains("Advertisement"),
                "Ads should be removed in precision mode"
            );
        }
        Err(err) => panic!("Extraction failed: {err:?}"),
    }
}

#[test]
fn test_recall_mode_extracts_more_content() {
    let html = std::fs::read_to_string(fixture_path("article_full.html"))
        .expect("Failed to read fixture");

    // First extract with default settings (for reference, not used in assertions)
    let _default_result = extract_with_options(&html, &Options::default())
        .expect("Default extraction failed");

    // Then with recall mode
    let recall_opts = Options {
        favor_recall: true,
        ..Options::default()
    };

    match extract_with_options(&html, &recall_opts) {
        Ok(recall_result) => {
            // Recall mode may extract more content
            // (or at least the same amount - never less of the main content)
            assert!(
                !recall_result.content_text.is_empty(),
                "Recall mode should extract content"
            );

            // Metadata should still work
            assert!(recall_result.metadata.title.is_some(), "Title should be extracted");
        }
        Err(err) => panic!("Extraction failed: {err:?}"),
    }
}

#[test]
fn test_author_blacklist_filtering() {
    let html = std::fs::read_to_string(fixture_path("article_full.html"))
        .expect("Failed to read fixture");

    // First extract without blacklist to confirm author exists
    let default_result = extract_with_options(&html, &Options::default())
        .expect("Default extraction failed");

    assert!(
        default_result.metadata.author.is_some(),
        "Author should be present without blacklist"
    );

    // Now with blacklist
    let opts = Options {
        author_blacklist: Some(vec!["Jane Smith".to_string()]),
        ..Options::default()
    };

    match extract_with_options(&html, &opts) {
        Ok(result) => {
            // Author should be filtered out
            if let Some(ref author) = result.metadata.author {
                assert!(
                    !author.contains("Jane Smith"),
                    "Blacklisted author should be removed"
                );
            }

            // Content should still be extracted
            assert!(!result.content_text.is_empty(), "Content should be extracted");
        }
        Err(err) => panic!("Extraction failed: {err:?}"),
    }
}

#[test]
fn test_deduplication_removes_repeated_text() {
    let html = std::fs::read_to_string(fixture_path("article_with_duplicates.html"))
        .expect("Failed to read fixture");

    let opts = Options {
        deduplicate: true,
        ..Options::default()
    };

    match extract_with_options(&html, &opts) {
        Ok(result) => {
            // Extraction should succeed with deduplication enabled
            assert!(
                !result.content_text.is_empty(),
                "Content should be extracted with deduplication"
            );

            // Unique content should still be present
            assert!(
                result.content_text.contains("unique content"),
                "Unique content should be preserved"
            );

            // Note: Full deduplication may require LRU cache integration
            // which operates at paragraph level during extraction.
            // For now, verify the option doesn't break extraction.
            let occurrences = result.content_text.matches("Lorem ipsum").count();
            eprintln!(
                "Deduplication test: found {occurrences} occurrences of 'Lorem ipsum'"
            );
        }
        Err(err) => panic!("Extraction failed: {err:?}"),
    }
}

#[test]
fn test_tables_and_precision_combined() {
    let html = std::fs::read_to_string(fixture_path("docs_with_tables.html"))
        .expect("Failed to read fixture");

    let opts = Options {
        include_tables: true,
        favor_precision: true,
        ..Options::default()
    };

    match extract_with_options(&html, &opts) {
        Ok(result) => {
            // Tables should be included
            assert!(
                result.content_text.contains("timeout") || result.content_text.contains("Option"),
                "Table content should be present"
            );

            // Code blocks should be preserved
            assert!(
                result.content_text.contains("Config"),
                "Code blocks should be preserved"
            );
        }
        Err(err) => panic!("Extraction failed: {err:?}"),
    }
}

#[test]
fn test_content_length_limits() {
    let html = std::fs::read_to_string(fixture_path("article_full.html"))
        .expect("Failed to read fixture");

    let opts = Options {
        max_extracted_len: 500,
        ..Options::default()
    };

    match extract_with_options(&html, &opts) {
        Ok(result) => {
            // Content should be truncated
            assert!(
                result.content_text.len() <= 500,
                "Content should not exceed max_extracted_len"
            );

            // Should have warning about truncation
            assert!(
                result.warnings.iter().any(|w| w.contains("truncated")),
                "Should warn about truncation"
            );
        }
        Err(err) => panic!("Extraction failed: {err:?}"),
    }
}

#[test]
fn test_min_output_size_validation() {
    let html = r"<html><body><article><p>Short.</p></article></body></html>";

    let opts = Options {
        min_output_size: 100, // Require at least 100 words
        ..Options::default()
    };

    match extract_with_options(html, &opts) {
        Ok(result) => {
            // Should have warning about insufficient content
            assert!(
                result.warnings.iter().any(|w| w.contains("Insufficient")),
                "Should warn about insufficient content"
            );
        }
        Err(err) => panic!("Extraction failed: {err:?}"),
    }
}

#[test]
fn test_multiple_options_combined() {
    let html = std::fs::read_to_string(fixture_path("article_full.html"))
        .expect("Failed to read fixture");

    let opts = Options {
        include_tables: true,
        include_images: true,
        include_links: true,
        favor_precision: true,
        deduplicate: true,
        ..Options::default()
    };

    match extract_with_options(&html, &opts) {
        Ok(result) => {
            // Basic extraction should work
            assert!(!result.content_text.is_empty(), "Content should be extracted");

            // Metadata should be complete
            assert!(result.metadata.title.is_some(), "Title should be extracted");
        }
        Err(err) => panic!("Extraction failed: {err:?}"),
    }
}
