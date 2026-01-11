use rs_trafilatura::{extract, extract_with_options, Options};

#[test]
fn options_default_values_are_sensible() {
    let options = Options::default();
    assert!(!options.include_comments);
    assert!(options.include_tables);
    assert!(!options.include_images);
    assert!(!options.include_links);
    assert!(!options.favor_precision);
    assert!(!options.favor_recall);
    assert!(options.target_language.is_none());
    assert!(options.url.is_none());
}

#[test]
fn options_struct_update_syntax_overrides_selected_fields_only() {
    let options = Options {
        include_comments: true,
        url: Some("https://example.com/article".to_string()),
        ..Options::default()
    };

    assert!(options.include_comments);
    assert!(options.include_tables);
    assert_eq!(options.url.as_deref(), Some("https://example.com/article"));
}

#[test]
fn extract_with_options_respects_non_default_options_and_extract_remains_unchanged() {
    let html = r#"
        <html><body>
            <article><p>ARTICLE_MARKER</p></article>
            <div id="comments"><p>COMMENT_MARKER</p></div>
        </body></html>
    "#;

    let with_comments = Options {
        include_comments: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &with_comments).expect("expected Ok(_)");
    assert!(result.content_text.contains("ARTICLE_MARKER"));
    assert!(result.comments_text.as_deref().unwrap_or("").contains("COMMENT_MARKER"));

    let default_result = extract(html).expect("expected Ok(_)");
    assert!(default_result.content_text.contains("ARTICLE_MARKER"));
    assert!(!default_result.content_text.contains("COMMENT_MARKER"));
    assert!(default_result.comments_text.is_none());
    assert!(default_result.comments_html.is_none());
}

#[test]
fn extract_with_options_can_use_options_url_for_hostname_extraction() {
    let html = r#"<html><body><article><p>ARTICLE_MARKER</p></article></body></html>"#;

    let options = Options {
        url: Some("https://example.com/some/path".to_string()),
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("expected Ok(_)");
    assert_eq!(result.metadata.hostname.as_deref(), Some("example.com"));
}

#[test]
fn extract_and_extract_with_default_options_match() {
    let html = r#"<html><body><article><p>ARTICLE_MARKER</p></article></body></html>"#;

    let a = extract(html).expect("expected Ok(_)");
    let b = extract_with_options(html, &Options::default()).expect("expected Ok(_)");

    assert_eq!(a.content_text, b.content_text);
    assert_eq!(a.content_html, b.content_html);
    assert_eq!(a.comments_text, b.comments_text);
    assert_eq!(a.comments_html, b.comments_html);
    assert_eq!(a.metadata.hostname, b.metadata.hostname);
    assert_eq!(a.metadata.url, b.metadata.url);
}

#[test]
fn extract_with_options_respects_include_tables_false() {
    let html = r#"
        <html><body>
            <article>
                <p>ARTICLE_MARKER</p>
                <table><tr><th>H1</th><th>H2</th></tr><tr><td>A</td><td>B</td></tr></table>
            </article>
        </body></html>
    "#;

    let options = Options {
        include_tables: false,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("expected Ok(_)");
    assert!(result.content_text.contains("ARTICLE_MARKER"));
    // Table content should still appear but not be pipe-formatted
    assert!(!result.content_text.contains("H1 | H2"));

    // With default (include_tables: true), tables are formatted
    let default_result = extract(html).expect("expected Ok(_)");
    assert!(default_result.content_text.contains("H1 | H2"));
}

#[test]
fn options_implements_debug_and_clone() {
    let options = Options::default();

    // Test Debug
    let debug_str = format!("{options:?}");
    assert!(debug_str.contains("Options"));
    assert!(debug_str.contains("include_comments"));

    // Test Clone
    let cloned = options.clone();
    assert_eq!(cloned.include_comments, options.include_comments);
    assert_eq!(cloned.include_tables, options.include_tables);
}
