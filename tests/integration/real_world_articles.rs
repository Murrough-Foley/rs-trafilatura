//! Integration tests for real-world article extraction
//!
//! Tests extraction from realistic HTML samples representing various content types.

#![allow(clippy::expect_used)] // expect() is appropriate in tests for clear panic messages

use rs_trafilatura::{extract, extract_with_options, Options};

/// Test fixture path helper
fn fixture_path(name: &str) -> String {
    format!(
        "{}/tests/integration/fixtures/{}",
        env!("CARGO_MANIFEST_DIR"),
        name
    )
}

#[test]
fn test_extract_full_article_with_metadata() {
    let html = std::fs::read_to_string(fixture_path("article_full.html"))
        .expect("Failed to read fixture");

    match extract(&html) {
        Ok(result) => {
            // Verify content extraction
            assert!(!result.content_text.is_empty(), "Content should not be empty");
            assert!(
                result.content_text.len() > 500,
                "Content should be substantial"
            );

            // Verify main article content is present
            assert!(
                result.content_text.contains("groundbreaking discovery"),
                "Should contain main article content"
            );
            assert!(
                result.content_text.contains("Dr. Maria Johnson"),
                "Should contain key figures from article"
            );

            // Verify metadata extraction
            assert!(result.metadata.title.is_some(), "Title should be extracted");
            assert!(
                result.metadata.author.is_some(),
                "Author should be extracted"
            );
            assert!(result.metadata.date.is_some(), "Date should be extracted");

            // Verify boilerplate removal
            assert!(
                !result.content_text.contains("Subscribe to our newsletter"),
                "Newsletter boilerplate should be removed"
            );
            assert!(
                !result.content_text.contains("Popular Posts"),
                "Sidebar content should be removed"
            );
        }
        Err(err) => panic!("Extraction failed: {err:?}"),
    }
}

#[test]
fn test_extract_blog_with_comments_enabled() {
    let html = std::fs::read_to_string(fixture_path("blog_with_comments.html"))
        .expect("Failed to read fixture");

    let opts = Options {
        include_comments: true,
        min_output_comm_size: 3, // Low threshold to ensure comments are captured
        ..Options::default()
    };

    match extract_with_options(&html, &opts) {
        Ok(result) => {
            // Verify main content extracted
            assert!(!result.content_text.is_empty(), "Content should not be empty");
            assert!(
                result.content_text.contains("Rust"),
                "Should contain main blog content about Rust"
            );

            // Verify metadata
            assert!(result.metadata.title.is_some(), "Title should be extracted");
            if let Some(ref title) = result.metadata.title {
                assert!(
                    title.contains("Rust"),
                    "Title should mention Rust"
                );
            }
        }
        Err(err) => panic!("Extraction failed: {err:?}"),
    }
}

#[test]
fn test_extract_blog_without_comments() {
    let html = std::fs::read_to_string(fixture_path("blog_with_comments.html"))
        .expect("Failed to read fixture");

    let opts = Options {
        include_comments: false,
        ..Options::default()
    };

    match extract_with_options(&html, &opts) {
        Ok(result) => {
            // Verify main content extracted
            assert!(!result.content_text.is_empty(), "Content should not be empty");

            // Comments should not be in result
            assert!(
                result.comments_text.is_none(),
                "Comments should not be extracted when disabled"
            );

            // Comment content should not leak into main content
            assert!(
                !result.content_text.contains("RustFan42"),
                "Comment authors should not be in main content"
            );
            assert!(
                !result.content_text.contains("NewbieCoder"),
                "Comment authors should not be in main content"
            );
        }
        Err(err) => panic!("Extraction failed: {err:?}"),
    }
}

#[test]
fn test_extract_docs_with_tables() {
    let html = std::fs::read_to_string(fixture_path("docs_with_tables.html"))
        .expect("Failed to read fixture");

    let opts = Options {
        include_tables: true,
        ..Options::default()
    };

    match extract_with_options(&html, &opts) {
        Ok(result) => {
            // Verify tables preserved
            assert!(
                result.content_text.contains("timeout"),
                "Table content should be preserved"
            );
            assert!(
                result.content_text.contains("retries"),
                "Table content should be preserved"
            );

            // Verify code blocks preserved
            assert!(
                result.content_text.contains("Config"),
                "Code examples should be preserved"
            );
        }
        Err(err) => panic!("Extraction failed: {err:?}"),
    }
}

#[test]
fn test_extract_article_removes_boilerplate() {
    let html = std::fs::read_to_string(fixture_path("article_with_boilerplate.html"))
        .expect("Failed to read fixture");

    match extract(&html) {
        Ok(result) => {
            // Should keep main content
            assert!(
                result.content_text.contains("actual main content"),
                "Main content should be extracted"
            );

            // Should remove various boilerplate elements
            assert!(
                !result.content_text.contains("Home | About | Contact"),
                "Navigation should be removed"
            );
            assert!(
                !result.content_text.contains("ADVERTISEMENT"),
                "Ads should be removed"
            );
            assert!(
                !result.content_text.contains("Trending Now"),
                "Trending widget should be removed"
            );
            assert!(
                !result.content_text.contains("Newsletter"),
                "Newsletter widget should be removed"
            );
        }
        Err(err) => panic!("Extraction failed: {err:?}"),
    }
}

#[test]
fn test_article_metadata_completeness() {
    let html = std::fs::read_to_string(fixture_path("article_full.html"))
        .expect("Failed to read fixture");

    match extract(&html) {
        Ok(result) => {
            let meta = &result.metadata;

            // Check all expected metadata fields
            assert!(meta.title.is_some(), "Title should be present");
            assert!(meta.author.is_some(), "Author should be present");
            assert!(meta.date.is_some(), "Date should be present");
            assert!(meta.description.is_some(), "Description should be present");

            // Verify specific values
            if let Some(ref title) = meta.title {
                assert!(
                    title.contains("Discovery") || title.contains("Science"),
                    "Title should be about the discovery"
                );
            }

            if let Some(ref author) = meta.author {
                assert!(
                    author.contains("Jane Smith"),
                    "Author should be Jane Smith"
                );
            }
        }
        Err(err) => panic!("Extraction failed: {err:?}"),
    }
}
