use rs_trafilatura::{extract, Error};
use std::time::{Duration, Instant};

#[test]
fn extract_does_not_panic_on_malformed_html_unclosed_tags() {
    let html = "<p>text<div>more";
    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("text"));
            assert!(result.content_text.contains("more"));
        }
        Err(Error::NoContent) => {}
        Err(err) => panic!("expected Ok(_) or Err(NoContent), got Err({err:?})"),
    }
}

#[test]
fn extract_does_not_panic_on_malformed_html_invalid_nesting() {
    let html = "<p><div></p></div>";
    let result = extract(html);
    assert!(matches!(result, Ok(_) | Err(Error::NoContent)));
}

#[test]
fn extract_does_not_panic_on_malformed_html_missing_closing_tags() {
    let html = "<html><body><article>content";
    let result = extract(html);
    match result {
        Ok(result) => assert!(result.content_text.contains("content")),
        Err(Error::NoContent) => {}
        Err(err) => panic!("expected Ok(_) or Err(NoContent), got Err({err:?})"),
    }
}

#[test]
fn extract_does_not_panic_on_malformed_html_broken_attributes() {
    let html = "<div class=\"test id=broken>";
    let result = extract(html);
    assert!(matches!(result, Ok(_) | Err(Error::NoContent)));
}

#[test]
fn extract_does_not_panic_on_malformed_html_incomplete_entities() {
    let html = "&amp text &lt;";
    let result = extract(html);
    match result {
        Ok(result) => assert!(result.content_text.contains("text")),
        Err(Error::NoContent) => {}
        Err(err) => panic!("expected Ok(_) or Err(NoContent), got Err({err:?})"),
    }
}

#[test]
fn extract_returns_partial_result_for_empty_string() {
    let result = extract("").expect("should return partial result with warnings");
    assert!(result.content_text.is_empty());
    assert!(!result.warnings.is_empty());
}

#[test]
fn extract_returns_partial_result_for_whitespace_only_input() {
    let result = extract("   \n\t  ").expect("should return partial result with warnings");
    assert!(result.content_text.is_empty());
    assert!(!result.warnings.is_empty());
}

#[test]
fn extract_returns_partial_result_for_minimal_html() {
    let result = extract("<html></html>").expect("should return partial result with warnings");
    assert!(result.content_text.is_empty());
    assert!(!result.warnings.is_empty());
}

#[test]
fn extract_returns_partial_result_for_body_only_html() {
    let result = extract("<body></body>").expect("should return partial result with warnings");
    assert!(result.content_text.is_empty());
    assert!(!result.warnings.is_empty());
}

#[test]
fn extract_handles_large_html_without_panic() {
    let target_size = 10 * 1024 * 1024 + 1;
    let chunk = "<p>Some repeated content for stress testing.</p>";
    let mut html = String::with_capacity(target_size + 128);
    html.push_str("<html><body><article>");
    while html.len() < target_size {
        html.push_str(chunk);
    }
    html.push_str("</article></body></html>");

    let start = Instant::now();
    let result = extract(&html);
    let elapsed = start.elapsed();

    assert!(matches!(result, Ok(_) | Err(Error::NoContent)));
    assert!(elapsed < Duration::from_secs(30), "large HTML parsing took {elapsed:?}");
}

#[test]
fn extract_skips_script_tags() {
    let html = r#"<html><body>
        <script>alert('xss')</script>
        <article><p>Safe content here</p></article>
    </body></html>"#;
    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(!result.content_text.contains("alert"));
            assert!(!result.content_text.contains("xss"));
            assert!(result.content_text.contains("Safe content"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_handles_null_bytes_gracefully() {
    let html = "text\x00more";
    let result = extract(html);
    assert!(matches!(result, Ok(_) | Err(Error::NoContent)));
}
