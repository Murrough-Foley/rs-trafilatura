use rs_trafilatura::{extract, extract_with_options, Options};

/// Test that recall mode uses lower threshold and accepts more content
#[test]
fn recall_mode_accepts_sparse_content() {
    // Sparse content that might not pass default threshold
    let html = r#"
        <html><body>
            <div id="content">
                <p>Brief article with minimal text.</p>
            </div>
        </body></html>
    "#;

    let default_result = extract_with_options(html, &Options::default());
    let recall_options = Options {
        favor_recall: true,
        ..Options::default()
    };
    let recall_result = extract_with_options(html, &recall_options);

    // Recall mode should be more lenient
    match (&default_result, &recall_result) {
        (Err(_), Ok(recall)) => {
            // Recall succeeds where default fails - ideal recall behavior
            assert!(recall.content_text.contains("Brief article"));
        }
        (Ok(default), Ok(recall)) => {
            // Both succeed - recall should extract at least as much
            assert!(
                recall.content_text.len() >= default.content_text.len(),
                "recall should extract at least as much as default"
            );
        }
        (Ok(_), Err(_)) => {
            panic!("recall mode should not fail when default succeeds");
        }
        (Err(_), Err(_)) => {
            // Both fail - content is truly insufficient (acceptable if extremely sparse)
            // But for this test case, recall should succeed
            panic!("recall mode should accept sparse content");
        }
    }
}

/// Test recall mode with borderline content
#[test]
fn recall_mode_includes_borderline_content() {
    // Content with borderline quality - short paragraphs, some structure
    let html = r#"
        <html><body>
            <article>
                <h2>Quick Update</h2>
                <p>Short paragraph one.</p>
                <p>Short paragraph two.</p>
            </article>
        </body></html>
    "#;

    let recall_options = Options {
        favor_recall: true,
        ..Options::default()
    };
    let result = extract_with_options(html, &recall_options).expect("recall should accept borderline content");

    assert!(result.content_text.contains("Quick Update"));
    assert!(result.content_text.contains("Short paragraph one"));
    assert!(result.content_text.contains("Short paragraph two"));
}

/// Test recall mode is more inclusive than default mode
#[test]
fn recall_mode_extracts_more_or_equal_content() {
    // HTML with various content regions of different quality
    let html = r#"
        <html><body>
            <div id="main">
                <p>Main content paragraph with reasonable length and substance.</p>
                <p>Another paragraph in the main section.</p>
            </div>
            <div id="secondary">
                <p>Secondary content here.</p>
            </div>
        </body></html>
    "#;

    let default_result = extract_with_options(html, &Options::default()).expect("default extraction failed");
    let recall_options = Options {
        favor_recall: true,
        ..Options::default()
    };
    let recall_result = extract_with_options(html, &recall_options).expect("recall extraction failed");

    // Recall should extract at least as much content as default
    assert!(
        recall_result.content_text.len() >= default_result.content_text.len(),
        "recall mode should extract at least as much content as default mode"
    );

    // Verify main content is included
    assert!(
        recall_result.content_text.contains("Main content paragraph"),
        "recall should include main content"
    );
}

/// Test recall mode with very minimal content
#[test]
fn recall_mode_handles_minimal_content() {
    let html = r#"
        <html><body>
            <p>Tiny.</p>
        </body></html>
    "#;

    let recall_options = Options {
        favor_recall: true,
        ..Options::default()
    };
    let result = extract_with_options(html, &recall_options);

    // Recall mode should attempt extraction even with minimal content
    // May succeed or fail depending on scoring, but should not panic
    if let Ok(r) = result {
        // If it succeeds, should have extracted something
        assert!(!r.content_text.is_empty());
    } else {
        // If it fails, content is truly insufficient (acceptable)
    }
}

/// Test recall mode with scattered content across multiple elements
#[test]
fn recall_mode_extracts_from_multiple_small_paragraphs() {
    // Content split across many small paragraphs
    let html = r#"
        <html><body>
            <article>
                <p>First piece.</p>
                <p>Second piece.</p>
                <p>Third piece.</p>
                <p>Fourth piece.</p>
                <p>Fifth piece.</p>
            </article>
        </body></html>
    "#;

    let recall_options = Options {
        favor_recall: true,
        ..Options::default()
    };
    let result = extract_with_options(html, &recall_options).expect("recall should handle scattered content");

    // Should extract all the pieces
    assert!(result.content_text.contains("First piece"));
    assert!(result.content_text.contains("Second piece"));
    assert!(result.content_text.contains("Third piece"));
    assert!(result.content_text.contains("Fourth piece"));
    assert!(result.content_text.contains("Fifth piece"));
}

/// Test that recall mode threshold (500) is lower than default (1000)
#[test]
fn recall_mode_uses_lower_threshold_than_default() {
    // Medium-low quality content that should:
    // - Pass recall threshold (500)
    // - Potentially fail default threshold (1000)
    let html = r#"
        <html><body>
            <div>
                <p>Short content with minimal substance.</p>
            </div>
        </body></html>
    "#;

    let default_result = extract_with_options(html, &Options::default());
    let recall_options = Options {
        favor_recall: true,
        ..Options::default()
    };
    let recall_result = extract_with_options(html, &recall_options);

    // Recall mode should never reject content that default accepts
    if default_result.is_ok() {
        assert!(
            recall_result.is_ok(),
            "recall mode (threshold 500) must not reject content that default (threshold 1000) accepts"
        );
    }

    // If recall succeeds, verify content is extracted
    if let Ok(recall) = &recall_result {
        assert!(
            recall.content_text.contains("Short content"),
            "recall should extract the available content"
        );
    }
}

/// Test recall mode with high-quality content (should always work)
#[test]
fn recall_mode_accepts_high_quality_content() {
    let html = r#"
        <html><body>
            <article>
                <h1>Comprehensive Article</h1>
                <p>This is substantial content with multiple paragraphs. High quality articles should be extracted successfully by all modes including recall.</p>
                <p>Second paragraph adds more depth and content. This ensures the article passes all thresholds easily.</p>
                <p>Third paragraph continues the article. Quality content like this is never rejected.</p>
            </article>
        </body></html>
    "#;

    let recall_options = Options {
        favor_recall: true,
        ..Options::default()
    };
    let result = extract_with_options(html, &recall_options).expect("recall should accept high-quality content");

    assert!(result.content_text.contains("Comprehensive Article"));
    assert!(result.content_text.contains("substantial content"));
    assert!(result.content_text.len() > 200);
}

/// Test recall vs precision: recall should be more inclusive
#[test]
fn recall_is_more_inclusive_than_precision() {
    // Medium quality content
    let html = r#"
        <html><body>
            <div>
                <p>This is a medium-quality paragraph. It has some substance but isn't extensive.</p>
                <p>Another paragraph that adds a bit more content.</p>
            </div>
        </body></html>
    "#;

    let precision_result = extract_with_options(html, &Options {
        favor_precision: true,
        ..Options::default()
    });

    let recall_result = extract_with_options(html, &Options {
        favor_recall: true,
        ..Options::default()
    });

    // If precision rejects it, recall should accept it
    if precision_result.is_err() {
        assert!(
            recall_result.is_ok(),
            "recall should accept content that precision rejects"
        );
    }

    // If both accept, recall should extract at least as much
    if let (Ok(precision), Ok(recall)) = (&precision_result, &recall_result) {
        assert!(
            recall.content_text.len() >= precision.content_text.len(),
            "recall should extract at least as much as precision"
        );
    }
}

/// Test that content from multiple separate regions is combined (AC #2)
///
/// This tests the emergent multi-region combining behavior where:
/// 1. Heuristic scoring selects a parent node containing multiple content regions
/// 2. Boilerplate filtering removes navigation/sidebar noise
/// 3. Content from all valid regions is preserved
///
/// Note: This is achieved through scoring + filtering, not explicit multi-region
/// merging. Future enhancement could add explicit candidate merging for edge cases.
#[test]
fn recall_mode_combines_content_from_multiple_regions() {
    // HTML with content split across SEPARATE divs (not within single article/main)
    let html = r#"
        <html><body>
            <div id="intro" class="content-block">
                <p>Introduction paragraph with substantial content here. This is the first content region.</p>
            </div>
            <nav id="navigation">
                <a href="/link1">Nav 1</a>
                <a href="/link2">Nav 2</a>
            </nav>
            <div id="main-content" class="content-block">
                <p>Main body paragraph with more substantial content. This is the second content region.</p>
            </div>
            <aside id="sidebar">
                <p>Sidebar noise that should be excluded</p>
            </aside>
            <div id="conclusion" class="content-block">
                <p>Conclusion paragraph wrapping up the article. This is the third content region.</p>
            </div>
        </body></html>
    "#;

    // Test with recall mode
    let recall_options = Options {
        favor_recall: true,
        ..Options::default()
    };
    let recall_result = extract_with_options(html, &recall_options)
        .expect("recall mode should extract multi-region content");

    // All three content regions should be included
    assert!(
        recall_result.content_text.contains("Introduction paragraph"),
        "should include first content region"
    );
    assert!(
        recall_result.content_text.contains("Main body paragraph"),
        "should include second content region"
    );
    assert!(
        recall_result.content_text.contains("Conclusion paragraph"),
        "should include third content region"
    );

    // Navigation and sidebar should be excluded
    assert!(
        !recall_result.content_text.contains("Nav 1"),
        "should exclude navigation"
    );
    assert!(
        !recall_result.content_text.contains("Sidebar noise"),
        "should exclude sidebar"
    );

    // Verify default mode also works (behavior should be consistent)
    let default_result = extract(html).expect("default mode should also extract multi-region content");
    assert!(default_result.content_text.contains("Introduction paragraph"));
    assert!(default_result.content_text.contains("Main body paragraph"));
    assert!(default_result.content_text.contains("Conclusion paragraph"));
}
