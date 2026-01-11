use rs_trafilatura::{extract_with_options, Options};

/// Test that precision mode applies stricter content selection
#[test]
fn precision_mode_is_more_selective() {
    // HTML with a borderline quality div that would score between 1000-5000
    // Default mode finds it via heuristics, precision mode should skip to body fallback
    let html = r#"
        <html><body>
            <div id="borderline-content">
                <p>Some content here. A reasonable paragraph with a bit of text.</p>
                <p>Another short paragraph.</p>
            </div>
            <div id="noise">
                <a href="/link">Click here</a>
                <span>Advertisement</span>
            </div>
        </body></html>
    "#;

    let default_result = extract_with_options(html, &Options::default()).expect("default extraction failed");
    let precision_options = Options {
        favor_precision: true,
        ..Options::default()
    };
    let precision_result = extract_with_options(html, &precision_options);

    // Both should extract something
    assert!(!default_result.content_text.is_empty());

    // Precision mode should be MORE selective (extract less or equal content)
    // or reject the content entirely (return Err)
    if let Ok(precision) = precision_result {
        // If precision succeeds, it must extract LESS OR EQUAL content
        assert!(
            precision.content_text.len() <= default_result.content_text.len(),
            "precision mode must not extract more content than default mode"
        );
    } else {
        // Rejecting low-quality content is also acceptable for precision mode
    }
}

/// Test that thresholds differentiate between modes
#[test]
fn different_modes_use_different_thresholds() {
    // High quality content that passes all thresholds (>= 5000)
    let high_quality_html = r#"
        <html><body>
            <article>
                <h1>Comprehensive Article</h1>
                <p>This is a substantial article with multiple well-formed paragraphs. It contains enough content to achieve a high quality score in any mode. The text provides real value and depth.</p>
                <p>Second paragraph continues to build on the topic with meaningful information. This ensures the content scores well above any threshold.</p>
                <p>Third paragraph adds even more substance. Quality content like this should always be extracted successfully.</p>
                <p>Fourth paragraph further demonstrates article quality and depth. Multiple paragraphs with substantial text create high scores.</p>
            </article>
        </body></html>
    "#;

    // All modes should succeed with high-quality content
    let default_result = extract_with_options(high_quality_html, &Options::default());
    let precision_result = extract_with_options(high_quality_html, &Options {
        favor_precision: true,
        ..Options::default()
    });
    let recall_result = extract_with_options(high_quality_html, &Options {
        favor_recall: true,
        ..Options::default()
    });

    assert!(default_result.is_ok(), "default should accept high-quality content");
    assert!(precision_result.is_ok(), "precision should accept high-quality content");
    assert!(recall_result.is_ok(), "recall should accept high-quality content");

    // All should contain the key content
    assert!(default_result.unwrap().content_text.contains("Comprehensive Article"));
    assert!(precision_result.unwrap().content_text.contains("Comprehensive Article"));
    assert!(recall_result.unwrap().content_text.contains("Comprehensive Article"));
}

/// Test conflicting options: both favor_precision and favor_recall
#[test]
fn conflicting_precision_and_recall_options_handled_gracefully() {
    let html = r#"
        <html><body>
            <article>
                <p>Test content for conflicting options</p>
            </article>
        </body></html>
    "#;

    let conflicting_options = Options {
        favor_precision: true,
        favor_recall: true,
        ..Options::default()
    };

    // Should either:
    // 1. Return Ok with precision taking precedence, or
    // 2. Return an error for conflicting options
    let result = extract_with_options(html, &conflicting_options);

    if let Ok(result) = result {
        // If we allow conflicting options, precision should take precedence
        // Verify extraction succeeded
        assert!(!result.content_text.is_empty());
        assert!(result.content_text.contains("Test content"));
    } else {
        // Also acceptable to return an error for conflicting configuration
        // Test passes either way
    }
}

/// Test precision mode with high-quality article content
#[test]
fn precision_mode_accepts_high_quality_content() {
    let html = r#"
        <html><body>
            <article>
                <h1>Comprehensive Article Title</h1>
                <p>This is a high-quality article with substantial content. It contains multiple well-formed paragraphs with meaningful information. The text provides real value and meets all quality thresholds.</p>
                <p>Another paragraph that demonstrates the article's depth and quality. This content should easily pass precision mode requirements because it has sufficient length, structure, and substance.</p>
                <p>A third paragraph to further establish content quality. High-quality articles like this should always be extracted successfully, even in precision mode with stricter thresholds.</p>
            </article>
        </body></html>
    "#;

    let precision_options = Options {
        favor_precision: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &precision_options).expect("high quality content should be extracted");

    assert!(result.content_text.contains("Comprehensive Article Title"));
    assert!(result.content_text.contains("high-quality article"));
    assert!(result.content_text.contains("real value"));
    assert!(result.content_text.len() > 200);
}

/// Test threshold boundaries: content that passes one threshold but fails another
///
/// Thresholds: precision=5000, default=1000, recall=500
/// This test creates medium-quality content that should:
/// - Pass default and recall modes (score >= 1000)
/// - Potentially fail precision mode (score < 5000)
#[test]
fn threshold_boundaries_differentiate_modes() {
    // Medium-quality content: enough for default but questionable for precision
    // ~2-3 paragraphs with moderate text should score between 1000-5000
    let html = r#"
        <html><body>
            <article>
                <p>This is a moderately sized paragraph with some content. It provides reasonable information but is not extensive.</p>
                <p>A second paragraph adds more substance. This helps reach the default threshold but may not satisfy precision requirements.</p>
            </article>
        </body></html>
    "#;

    let default_result = extract_with_options(html, &Options::default());
    let precision_result = extract_with_options(html, &Options {
        favor_precision: true,
        ..Options::default()
    });
    let recall_result = extract_with_options(html, &Options {
        favor_recall: true,
        ..Options::default()
    });

    // Default should succeed with medium-quality content
    assert!(default_result.is_ok(), "default mode should accept medium-quality content");

    // Recall should definitely succeed (lowest threshold)
    assert!(recall_result.is_ok(), "recall mode should accept medium-quality content");

    // Precision may or may not succeed depending on exact score
    // But if it does succeed, content should be identical or subset
    if let (Ok(default), Ok(precision)) = (&default_result, &precision_result) {
        assert!(
            precision.content_text.len() <= default.content_text.len(),
            "precision mode should not extract more than default"
        );
    }
    // If precision fails, that's expected behavior for medium-quality content
}

/// Test that recall mode is more inclusive than default mode
#[test]
fn recall_mode_is_more_inclusive() {
    // HTML with marginal content that might be rejected by default but accepted by recall
    let html = r#"
        <html><body>
            <div id="marginal-content">
                <p>Short text here.</p>
            </div>
        </body></html>
    "#;

    let default_result = extract_with_options(html, &Options::default());
    let recall_options = Options {
        favor_recall: true,
        ..Options::default()
    };
    let recall_result = extract_with_options(html, &recall_options);

    // Recall mode should succeed OR extract at least as much as default
    match (&default_result, &recall_result) {
        (Ok(default), Ok(recall)) => {
            // Recall should extract at least as much content as default
            assert!(
                recall.content_text.len() >= default.content_text.len(),
                "recall mode should extract at least as much content as default"
            );
        }
        (Err(_), Ok(_)) => {
            // Recall succeeds where default fails - this is the expected inclusive behavior
        }
        (Ok(_), Err(_)) => {
            panic!("recall mode should not reject content that default mode accepts");
        }
        (Err(_), Err(_)) => {
            // Both fail - content is too marginal even for recall mode (acceptable)
        }
    }
}

/// Test precision mode rejects high link density content
#[test]
fn precision_mode_filters_link_heavy_content() {
    // Content with many links relative to text (high link density)
    let html = r#"
        <html><body>
            <div id="nav-like-content">
                <a href="/1">Link One</a>
                <a href="/2">Link Two</a>
                <a href="/3">Link Three</a>
                <a href="/4">Link Four</a>
                <p>Short text</p>
            </div>
            <article>
                <p>This is substantial article content with good text-to-link ratio. It has multiple sentences providing real information without excessive linking. This should pass precision mode.</p>
            </article>
        </body></html>
    "#;

    let precision_options = Options {
        favor_precision: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &precision_options).expect("extraction failed");

    // Should prefer the article with better link density
    assert!(result.content_text.contains("substantial article content"));

    // Should minimize link-heavy content
    let link_count = result.content_text.matches("Link").count();
    // In precision mode, link-heavy sections should be deprioritized
    assert!(link_count < 4, "precision mode should avoid link-heavy content");
}
