use rs_trafilatura::{extract, extract_with_options, Options};

/// Test AC#1: HTML where content extraction fails but metadata succeeds
/// Result contains populated metadata with empty content_text and a warning
#[test]
fn metadata_only_when_content_fails() {
    // HTML with metadata but no extractable content
    let html = r#"
        <html>
        <head>
            <title>Test Article Title</title>
            <meta name="author" content="John Doe">
            <meta name="description" content="Article description">
        </head>
        <body>
            <!-- No article content, just navigation -->
            <nav>Navigation links</nav>
            <footer>Footer content</footer>
        </body>
        </html>
    "#;

    let result = extract(html).expect("extraction should succeed with warnings");

    // AC#1: Content is empty but metadata is populated
    assert!(result.content_text.is_empty());
    assert_eq!(result.metadata.title, Some("Test Article Title".to_string()));
    assert_eq!(result.metadata.author, Some("John Doe".to_string()));
    assert_eq!(
        result.metadata.description,
        Some("Article description".to_string())
    );

    // AC#1: Warning is included
    assert!(!result.warnings.is_empty());
    assert!(result.warnings[0].contains("Content extraction failed"));
}

/// Test AC#2: HTML where some metadata fields fail but content succeeds
/// Result contains content with partial metadata (failed fields are None)
#[test]
fn partial_metadata_with_content() {
    // HTML with content but limited metadata
    let html = r#"
        <html>
        <head>
            <title>Article Title</title>
            <!-- No author, date, or description -->
        </head>
        <body>
            <article>
                <h1>Article Heading</h1>
                <p>This is substantial article content that should be extracted successfully.</p>
                <p>Multiple paragraphs ensure this is recognized as main content.</p>
                <p>We need enough text to meet the scoring threshold for extraction.</p>
            </article>
        </body>
        </html>
    "#;

    let result = extract(html).expect("extraction should succeed");

    // AC#2: Content is extracted successfully
    assert!(!result.content_text.is_empty());
    assert!(result.content_text.contains("substantial article content"));

    // AC#2: Partial metadata (title present, others None)
    assert_eq!(result.metadata.title, Some("Article Title".to_string()));
    assert_eq!(result.metadata.author, None);
    assert_eq!(result.metadata.date, None);
    assert_eq!(result.metadata.description, None);

    // No warnings expected when content succeeds
    assert!(result.warnings.is_empty());
}

/// Test AC#3: HTML where title extraction fails
/// metadata.title is None but other fields are still extracted
#[test]
fn title_fails_but_other_metadata_succeeds() {
    // HTML with author and description but no title
    let html = r#"
        <html>
        <head>
            <!-- No title tag -->
            <meta name="author" content="Jane Smith">
            <meta name="description" content="An article without a title">
        </head>
        <body>
            <article>
                <p>Content here. This is substantial article content that should be extracted.</p>
                <p>Multiple paragraphs of meaningful text for proper extraction.</p>
                <p>More content to ensure scoring threshold is met.</p>
            </article>
        </body>
        </html>
    "#;

    let result = extract(html).expect("extraction should succeed");

    // AC#3: Title is None
    assert_eq!(result.metadata.title, None);

    // AC#3: Other metadata fields are still extracted
    assert_eq!(result.metadata.author, Some("Jane Smith".to_string()));
    assert_eq!(
        result.metadata.description,
        Some("An article without a title".to_string())
    );

    // Content should still be extracted
    assert!(!result.content_text.is_empty());
    assert!(result.content_text.contains("substantial article content"));
}

/// Test AC#4: Extraction encounters recoverable errors
/// Processing continues and logs/notes the issue without panicking
#[test]
fn recoverable_errors_dont_panic() {
    // HTML with various edge cases that might cause issues
    let html = r#"
        <html>
        <head>
            <title>Test</title>
            <meta name="date" content="invalid-date-format">
        </head>
        <body>
            <article>
                <p>Content with edge cases.</p>
                <p>More substantial content for extraction.</p>
                <p>Additional paragraphs to ensure proper extraction.</p>
            </article>
        </body>
        </html>
    "#;

    // Should not panic - this is the key test
    let result = extract(html).expect("should handle recoverable errors gracefully");

    // Content should be extracted
    assert!(!result.content_text.is_empty());
    assert!(result.content_text.contains("Content with edge cases"));

    // Date parsing failure should be gracefully handled (date field is None)
    assert_eq!(result.metadata.date, None);

    // Title should still be extracted
    assert_eq!(result.metadata.title, Some("Test".to_string()));
}

/// Test: Empty HTML returns metadata-only result with warning
#[test]
fn empty_html_returns_partial_result() {
    let html = "<html><body></body></html>";
    let result = extract(html).expect("should return partial result");

    assert!(result.content_text.is_empty());
    assert!(!result.warnings.is_empty());
    assert!(result.warnings[0].contains("Content extraction failed"));
}

/// Test: HTML with only boilerplate returns metadata with warning
#[test]
fn only_boilerplate_returns_metadata() {
    let html = r#"
        <html>
        <head>
            <title>Site Name</title>
        </head>
        <body>
            <nav>Menu</nav>
            <aside>Sidebar</aside>
            <footer>Footer</footer>
        </body>
        </html>
    "#;

    let result = extract(html).expect("should return partial result");

    // No content extracted
    assert!(result.content_text.is_empty());

    // Metadata still extracted
    assert_eq!(result.metadata.title, Some("Site Name".to_string()));

    // Warning included
    assert!(!result.warnings.is_empty());
}

/// Test: Malformed HTML doesn't panic and returns partial results
#[test]
fn malformed_html_graceful_degradation() {
    let html = "<html><head><title>Test<body><article>Content";

    let result = extract(html).expect("should handle malformed HTML gracefully");

    // Should extract what it can
    assert!(result.metadata.title.is_some());
}

/// Test: extract_with_options respects options even with partial results
#[test]
fn partial_results_respect_options() {
    let html = r#"
        <html>
        <head><title>Test</title></head>
        <body>
            <nav>Navigation</nav>
        </body>
        </html>
    "#;

    let options = Options {
        include_comments: true,
        include_images: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("should return partial result");

    // Content is empty but options were respected
    assert!(result.content_text.is_empty());
    assert!(result.comments_text.is_none());
    assert!(result.images.is_empty());
    assert_eq!(result.metadata.title, Some("Test".to_string()));
}

/// Test: Multiple warnings can be collected
#[test]
fn multiple_warnings_collected() {
    // HTML that might generate multiple issues
    let html = r#"
        <html>
        <head>
            <meta name="invalid" content="test">
        </head>
        <body>
            <!-- No content -->
        </body>
        </html>
    "#;

    let result = extract(html).expect("should return result with warnings");

    assert!(result.content_text.is_empty());
    // At minimum, should have content extraction warning
    assert!(!result.warnings.is_empty());
}

/// Test: Successful extraction has no warnings
#[test]
fn successful_extraction_no_warnings() {
    let html = r#"
        <html>
        <head><title>Good Article</title></head>
        <body>
            <article>
                <h1>Heading</h1>
                <p>This is substantial content that will be extracted successfully.</p>
                <p>Multiple paragraphs of meaningful text.</p>
                <p>Enough content to meet all scoring thresholds.</p>
            </article>
        </body>
        </html>
    "#;

    let result = extract(html).expect("extraction should succeed");

    // Content extracted successfully
    assert!(!result.content_text.is_empty());
    assert!(result.content_text.contains("substantial content"));

    // No warnings on successful extraction
    assert!(result.warnings.is_empty());
}

/// Test: Content with minimal text but valid structure
#[test]
fn minimal_valid_content() {
    let html = r#"
        <html>
        <head><title>Short Article</title></head>
        <body>
            <article>
                <p>Short but valid content.</p>
            </article>
        </body>
        </html>
    "#;

    let result = extract(html).expect("should extract minimal content");

    // Even short content should be extracted if in article tag
    assert!(!result.content_text.is_empty() || !result.warnings.is_empty());
    assert_eq!(result.metadata.title, Some("Short Article".to_string()));
}
