use rs_trafilatura::extract;

#[test]
fn extract_preserves_paragraph_separation() {
    let html = "<article><p>First paragraph.</p><p>Second paragraph.</p></article>";
    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("First paragraph."));
            assert!(result.content_text.contains("Second paragraph."));
            assert!(result.content_text.contains("\n\n"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_handles_br_as_single_newline() {
    let html = "<article><p>Line 1<br>Line 2</p></article>";
    let result = extract(html);
    match result {
        Ok(result) => assert!(result.content_text.contains("Line 1\nLine 2")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_strips_inline_elements() {
    let html = "<article><p>This is <strong>bold</strong> and <em>italic</em>.</p></article>";
    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.content_text.trim(), "This is bold and italic."),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_preserves_link_text_without_url() {
    let html = r#"<article><p>Visit <a href="https://example.com">our site</a>.</p></article>"#;
    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("our site"));
            assert!(!result.content_text.contains("https://"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_preserves_headings_with_separation() {
    let html = "<article><h2>Heading</h2><p>Para</p></article>";
    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("Heading"));
            assert!(result.content_text.contains("Para"));
            assert!(result.content_text.contains("Heading\n\nPara"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_normalizes_inline_whitespace_but_preserves_newlines() {
    let html = "<article><p> Hello\t\tworld </p><p> Second\nline </p></article>";
    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("Hello world"));
            assert!(result.content_text.contains("\n\n"));
            assert!(!result.content_text.contains("  "));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_result_fields_are_public_and_options_are_defaultable() {
    let html = "<article><p>Text</p></article>";
    let result = extract(html);
    match result {
        Ok(result) => {
            let _text: &str = &result.content_text;
            let _html_defaulted: String = result.content_html.clone().unwrap_or_default();
            let title = result.metadata.title.clone().unwrap_or_default();
            let author = result.metadata.author.as_deref().unwrap_or("Unknown");
            let _ = (title, author, _html_defaulted);
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn no_content_returns_partial_result_with_warning() {
    let html = "<html><body><nav>Nav</nav></body></html>";
    let result = extract(html).expect("should return partial result");
    // Graceful degradation: returns Ok with empty content and warnings
    assert!(result.content_text.is_empty());
    assert!(!result.warnings.is_empty());
    assert!(result.warnings[0].contains("Content extraction failed"));
}

#[test]
fn extract_handles_nested_inline_elements() {
    let html = "<article><p>This is <strong><em>bold and italic</em></strong> text.</p></article>";
    let result = extract(html);
    match result {
        Ok(result) => {
            assert_eq!(result.content_text.trim(), "This is bold and italic text.");
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_handles_list_items() {
    let html = "<article><ul><li>Item 1</li><li>Item 2</li><li>Item 3</li></ul></article>";
    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("Item 1"));
            assert!(result.content_text.contains("Item 2"));
            assert!(result.content_text.contains("Item 3"));
            // Items should be on separate lines
            assert!(result.content_text.contains("Item 1\n"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}
