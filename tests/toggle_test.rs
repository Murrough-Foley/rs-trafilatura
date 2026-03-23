use rs_trafilatura::{extract_with_options, Options};

/// Padding to ensure content extraction threshold is met
const PADDING: &str = "<p>Additional paragraph with enough content to ensure the extraction algorithm finds sufficient text density to extract this article content properly.</p><p>Second padding paragraph adding more sentences to satisfy the minimum scoring threshold required for content extraction to succeed.</p>";

/// Comment text with enough words (>= 10) to pass the minimum comment size threshold
const COMMENT_TEXT: &str = "<p>First comment here with enough words to pass minimum threshold.</p><p>Second comment continuing the discussion with more text to satisfy the word count requirement.</p>";

/// Test AC#1: include_tables: false excludes table content
#[test]
fn include_tables_false_excludes_table_content() {
    let html = format!(r#"
        <html><body>
            <article>
                <p>Article text before table.</p>
                <table>
                    <tr><th>Header 1</th><th>Header 2</th></tr>
                    <tr><td>Data 1</td><td>Data 2</td></tr>
                </table>
                <p>Article text after table.</p>
                {PADDING}
            </article>
        </body></html>
    "#);

    let options = Options {
        include_tables: false,
        ..Options::default()
    };

    let result = extract_with_options(&html, &options).expect("extraction failed");

    // Should have article text
    assert!(result.content_text.contains("Article text before table"));
    assert!(result.content_text.contains("Article text after table"));

    // Should NOT have table content in pipe-delimited format
    assert!(!result.content_text.contains("Header 1 | Header 2"));
    assert!(!result.content_text.contains("Data 1 | Data 2"));

    // Content HTML should also exclude table if present
    if let Some(html) = result.content_html {
        assert!(!html.contains("<table"));
        assert!(!html.contains("<tr"));
        assert!(!html.contains("<td"));
    }
}

/// Test AC#2: include_tables: true (default) includes table content
#[test]
fn include_tables_true_includes_table_content() {
    let html = format!(r#"
        <html><body>
            <article>
                <p>Article text before table.</p>
                {PADDING}
                <table>
                    <tr><th>Header 1</th><th>Header 2</th></tr>
                    <tr><td>Data 1</td><td>Data 2</td></tr>
                </table>
                <p>Article text after table.</p>
            </article>
        </body></html>
    "#);

    // Test with explicit true
    let options = Options {
        include_tables: true,
        ..Options::default()
    };
    let result = extract_with_options(&html, &options).expect("extraction failed");

    // Should have article text
    assert!(result.content_text.contains("Article text before table"));
    assert!(result.content_text.contains("Article text after table"));

    // Should have table content in pipe-delimited format
    assert!(result.content_text.contains("Header 1 | Header 2"));
    assert!(result.content_text.contains("Data 1 | Data 2"));
}

/// Test that default Options has include_tables: true
#[test]
fn default_options_includes_tables() {
    let html = format!(r#"
        <html><body>
            <article>
                {PADDING}
                <table>
                    <tr><th>H1</th><th>H2</th></tr>
                    <tr><td>A</td><td>B</td></tr>
                </table>
            </article>
        </body></html>
    "#);

    // Use default options
    let result = extract_with_options(&html, &Options::default()).expect("extraction failed");

    // Default should include tables
    assert!(result.content_text.contains("H1 | H2"));
    assert!(result.content_text.contains("A | B"));
}

/// Test AC#3: include_comments: true populates comments fields
#[test]
fn include_comments_true_populates_comments_fields() {
    let html = format!(r#"
        <html><body>
            <article>
                <p>Main article content here with enough text for extraction algorithm threshold.</p>
                <p>Second paragraph adds more substance for better extraction scoring and results.</p>
            </article>
            <div id="comments">
                {COMMENT_TEXT}
            </div>
        </body></html>
    "#);

    let options = Options {
        include_comments: true,
        ..Options::default()
    };

    let result = extract_with_options(&html, &options).expect("extraction failed");

    // Main content should be extracted
    assert!(result.content_text.contains("Main article content"));

    // Comments should be populated (need >= 10 words to pass minimum threshold)
    assert!(result.comments_text.is_some(), "comments_text should be Some");
    let comments = result.comments_text.unwrap();
    assert!(comments.contains("First comment"));
    assert!(comments.contains("Second comment"));
}

/// Test AC#4: include_comments: false (default) returns None for comments
#[test]
fn include_comments_false_returns_none_for_comments() {
    let html = r#"
        <html><body>
            <article>
                <p>Main article content here.</p>
            </article>
            <div id="comments">
                <p>Comment that should not be extracted.</p>
            </div>
        </body></html>
    "#;

    // Test with explicit false
    let options = Options {
        include_comments: false,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    // Main content should be extracted
    assert!(result.content_text.contains("Main article content"));

    // Comments should be None
    assert!(result.comments_text.is_none());
    assert!(result.comments_html.is_none());
}

/// Test that default Options has include_comments: false
#[test]
fn default_options_excludes_comments() {
    let html = r#"
        <html><body>
            <article>
                <p>Main content.</p>
            </article>
            <div id="comments">
                <p>Comment text.</p>
            </div>
        </body></html>
    "#;

    // Use default options
    let result = extract_with_options(html, &Options::default()).expect("extraction failed");

    // Default should NOT include comments
    assert!(result.comments_text.is_none());
    assert!(result.comments_html.is_none());
}

/// Test combined toggles: both disabled
#[test]
fn both_toggles_disabled() {
    let html = r#"
        <html><body>
            <article>
                <p>Article content.</p>
                <table><tr><td>Table data</td></tr></table>
            </article>
            <div id="comments">
                <p>Comment content.</p>
            </div>
        </body></html>
    "#;

    let options = Options {
        include_tables: false,
        include_comments: false,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    // Should have article content
    assert!(result.content_text.contains("Article content"));

    // Should NOT have table content
    assert!(!result.content_text.contains("Table data"));

    // Should NOT have comments
    assert!(result.comments_text.is_none());
}

/// Test combined toggles: both enabled
#[test]
fn both_toggles_enabled() {
    let html = format!(r#"
        <html><body>
            <article>
                <p>Article content here with enough text for extraction.</p>
                {PADDING}
                <table>
                    <tr><th>Table header</th><th>Second column</th></tr>
                    <tr><td>Table data value</td><td>More data</td></tr>
                </table>
            </article>
            <div id="comments">
                {COMMENT_TEXT}
            </div>
        </body></html>
    "#);

    let options = Options {
        include_tables: true,
        include_comments: true,
        ..Options::default()
    };

    let result = extract_with_options(&html, &options).expect("extraction failed");

    // Should have article content
    assert!(result.content_text.contains("Article content"));

    // Should have table content in pipe-delimited format
    assert!(result.content_text.contains("Table header | Second column"));

    // Should have comments (need >= 10 words)
    assert!(result.comments_text.is_some(), "comments_text should be Some");
    assert!(result.comments_text.unwrap().contains("First comment"));
}

/// Test that table toggle doesn't affect non-table content
#[test]
fn table_toggle_doesnt_affect_other_content() {
    let html = format!(r#"
        <html><body>
            <article>
                <p>Paragraph one content here.</p>
                <div>Div content inside article.</div>
                <ul><li>List item content.</li></ul>
                {PADDING}
            </article>
        </body></html>
    "#);

    let with_tables = extract_with_options(&html, &Options {
        include_tables: true,
        ..Options::default()
    }).expect("extraction failed");

    let without_tables = extract_with_options(&html, &Options {
        include_tables: false,
        ..Options::default()
    }).expect("extraction failed");

    // Both should extract the same non-table content
    assert!(with_tables.content_text.contains("Paragraph one content"));
    assert!(with_tables.content_text.contains("List item content"));

    assert!(without_tables.content_text.contains("Paragraph one content"));
    assert!(without_tables.content_text.contains("List item content"));
}

/// Test that comment toggle doesn't affect main content
#[test]
fn comment_toggle_doesnt_affect_main_content() {
    let html = r#"
        <html><body>
            <article>
                <p>Main article content that should always be extracted.</p>
            </article>
            <div id="comments">
                <p>Comment content.</p>
            </div>
        </body></html>
    "#;

    let with_comments = extract_with_options(html, &Options {
        include_comments: true,
        ..Options::default()
    }).expect("extraction failed");

    let without_comments = extract_with_options(html, &Options {
        include_comments: false,
        ..Options::default()
    }).expect("extraction failed");

    // Both should extract the same main content
    assert!(with_comments.content_text.contains("Main article content"));
    assert!(without_comments.content_text.contains("Main article content"));

    // Content text should be identical
    assert_eq!(with_comments.content_text, without_comments.content_text);
}
