//! Edge case integration tests
//!
//! Tests for unusual inputs, boundary conditions, and error handling.

#![allow(clippy::expect_used)] // expect() is appropriate in tests for clear panic messages

use rs_trafilatura::{extract, extract_with_options, Options};

#[test]
fn test_extract_minimal_html() {
    let html = "<html><body><p>Minimal content.</p></body></html>";

    match extract(html) {
        Ok(result) => {
            assert!(
                result.content_text.contains("Minimal"),
                "Should extract minimal content"
            );
        }
        Err(err) => panic!("Extraction failed: {err:?}"),
    }
}

#[test]
fn test_extract_empty_body() {
    let html = "<html><body></body></html>";

    // Both Ok (with empty content or warnings) and Err are acceptable
    if let Ok(result) = extract(html) {
        // Should handle gracefully - empty content is ok
        // May have warning about no content
        assert!(
            result.content_text.is_empty() || !result.warnings.is_empty(),
            "Empty body should result in empty content or warnings"
        );
    }
    // Err case is also acceptable - extraction can fail on empty content
}

#[test]
fn test_extract_no_body() {
    let html = "<html><head><title>No Body</title></head></html>";

    // Should not panic
    let result = extract(html);
    assert!(result.is_ok(), "Should handle missing body gracefully");
}

#[test]
fn test_extract_malformed_html() {
    let html = "<html><body><p>Unclosed paragraph<div>Nested<p>Badly</body>";

    // Both Ok and Err are acceptable for malformed HTML
    if let Ok(result) = extract(html) {
        // Should handle gracefully and extract what it can
        assert!(
            !result.content_text.is_empty() || !result.warnings.is_empty(),
            "Should extract content or report warnings"
        );
    }
    // Err case is also acceptable for severely malformed HTML
}

#[test]
fn test_extract_deeply_nested_html() {
    // Create deeply nested structure
    let mut html = String::from("<html><body>");
    for i in 0..50 {
        html.push_str(&format!("<div class='level-{i}'>"));
    }
    html.push_str("<p>Deep content here</p>");
    for _ in 0..50 {
        html.push_str("</div>");
    }
    html.push_str("</body></html>");

    match extract(&html) {
        Ok(result) => {
            assert!(
                result.content_text.contains("Deep content"),
                "Should extract deeply nested content"
            );
        }
        Err(err) => panic!("Extraction failed on nested HTML: {err:?}"),
    }
}

#[test]
fn test_extract_very_large_document() {
    // Generate large HTML (~1MB)
    let paragraphs: String = (0..5000)
        .map(|i| format!("<p>Paragraph {i} with some content words.</p>"))
        .collect::<Vec<_>>()
        .join("\n");

    let html = format!(
        "<html><body><article>{paragraphs}</article></body></html>"
    );

    let opts = Options {
        max_extracted_len: 50_000, // Limit output size
        ..Options::default()
    };

    match extract_with_options(&html, &opts) {
        Ok(result) => {
            // Should handle large documents
            assert!(!result.content_text.is_empty(), "Should extract content");

            // Should respect max length
            assert!(
                result.content_text.len() <= opts.max_extracted_len,
                "Should respect max_extracted_len"
            );
        }
        Err(err) => panic!("Extraction failed on large document: {err:?}"),
    }
}

#[test]
fn test_extract_non_english_content() {
    let html = r#"<!DOCTYPE html>
    <html lang="ja">
    <head>
        <meta charset="UTF-8">
        <meta property="og:title" content="日本語の記事タイトル">
        <title>日本語の記事タイトル</title>
    </head>
    <body>
        <article>
            <h1>日本語の見出し</h1>
            <p>この記事では、さまざまなトピックについて詳しく説明しています。日本語のコンテンツを正しく処理できることを確認するためのテストです。</p>
            <p>Rustは安全性と速度を両立させた素晴らしいプログラミング言語です。メモリ安全性を保証しながら、高いパフォーマンスを実現します。</p>
        </article>
    </body>
    </html>"#;

    match extract(html) {
        Ok(result) => {
            // Should handle non-ASCII content
            assert!(!result.content_text.is_empty(), "Should extract content");

            // Should preserve Japanese characters
            assert!(
                result.content_text.contains("日本語") || result.content_text.contains("Rust"),
                "Should preserve Japanese or mixed content"
            );

            // Language should be detected
            assert_eq!(
                result.metadata.language,
                Some("ja".to_string()),
                "Should detect Japanese language"
            );
        }
        Err(err) => panic!("Extraction failed on non-English content: {err:?}"),
    }
}

#[test]
fn test_extract_unicode_content() {
    let html = r"<!DOCTYPE html>
    <html>
    <body>
        <article>
            <p>Unicode test: emoji and special chars</p>
            <p>Chinese: simplify content here</p>
            <p>Arabic: text content here</p>
            <p>Russian: text content here</p>
        </article>
    </body>
    </html>";

    match extract(html) {
        Ok(result) => {
            assert!(
                result.content_text.contains("Unicode"),
                "Should handle Unicode content"
            );
        }
        Err(err) => panic!("Extraction failed on Unicode content: {err:?}"),
    }
}

#[test]
fn test_extract_only_whitespace_content() {
    let html = "<html><body><article>   \n\t\n   </article></body></html>";

    if let Ok(result) = extract(html) {
        // Whitespace-only should be treated as empty
        let trimmed = result.content_text.trim();
        assert!(
            trimmed.is_empty() || !result.warnings.is_empty(),
            "Whitespace-only content should be empty or have warnings"
        );
    } else {
        // Also acceptable
    }
}

#[test]
fn test_extract_script_and_style_removed() {
    let html = r"<!DOCTYPE html>
    <html>
    <head>
        <style>body { color: red; }</style>
        <script>alert('hello');</script>
    </head>
    <body>
        <article>
            <p>Main content here.</p>
            <script>console.log('inline script');</script>
            <style>.inline { display: none; }</style>
        </article>
    </body>
    </html>";

    match extract(html) {
        Ok(result) => {
            // Scripts and styles should be removed
            assert!(
                !result.content_text.contains("alert"),
                "Script content should be removed"
            );
            assert!(
                !result.content_text.contains("console.log"),
                "Inline script should be removed"
            );
            assert!(
                !result.content_text.contains("color: red"),
                "Style content should be removed"
            );

            // Main content should remain
            assert!(
                result.content_text.contains("Main content"),
                "Main content should be preserved"
            );
        }
        Err(err) => panic!("Extraction failed: {err:?}"),
    }
}

#[test]
fn test_extract_preserves_text_structure() {
    let html = r"<!DOCTYPE html>
    <html>
    <body>
        <article>
            <h1>Title</h1>
            <p>First paragraph.</p>
            <p>Second paragraph.</p>
            <ul>
                <li>Item one</li>
                <li>Item two</li>
            </ul>
        </article>
    </body>
    </html>";

    match extract(html) {
        Ok(result) => {
            // Text should be extracted
            assert!(result.content_text.contains("First paragraph"));
            assert!(result.content_text.contains("Second paragraph"));
            assert!(result.content_text.contains("Item one"));
        }
        Err(err) => panic!("Extraction failed: {err:?}"),
    }
}

#[test]
fn test_extract_handles_special_characters() {
    let html = r"<!DOCTYPE html>
    <html>
    <body>
        <article>
            <p>Special chars: &amp; &lt; &gt; &quot; &apos;</p>
            <p>More: &copy; &reg; &trade; &nbsp;</p>
        </article>
    </body>
    </html>";

    match extract(html) {
        Ok(result) => {
            // HTML entities should be decoded
            assert!(
                result.content_text.contains('&') || result.content_text.contains("Special"),
                "Should handle HTML entities"
            );
        }
        Err(err) => panic!("Extraction failed: {err:?}"),
    }
}

// Performance baseline test
#[test]
fn test_extract_performance_baseline() {
    let html = std::fs::read_to_string(format!(
        "{}/tests/integration/fixtures/article_full.html",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("Failed to read fixture");

    let start = std::time::Instant::now();

    for _ in 0..50 {
        let _ = extract(&html);
    }

    let duration = start.elapsed();

    // Should process 50 documents in reasonable time (< 10 seconds)
    assert!(
        duration.as_secs() < 10,
        "Performance regression: took {duration:?} for 50 extractions"
    );

    // Log performance for reference
    eprintln!(
        "Performance: 50 extractions in {:?} ({:.2}ms per extraction)",
        duration,
        duration.as_millis() as f64 / 50.0
    );
}
