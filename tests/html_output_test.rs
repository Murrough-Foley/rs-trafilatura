use rs_trafilatura::extract;

#[test]
fn extract_returns_content_html_with_block_structure() {
    let html = "<article><h2>Heading</h2><p>Para</p></article>";
    let result = extract(html);
    match result {
        Ok(result) => {
            let content_html = result
                .content_html
                .as_deref()
                .expect("expected Some(content_html)");
            assert!(content_html.contains("<h2>Heading</h2>"));
            assert!(content_html.contains("<p>Para</p>"));

            assert!(result.content_text.contains("Heading"));
            assert!(result.content_text.contains("Para"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_preserves_inline_formatting_in_content_html() {
    let html = r#"<article><p>Text <strong>bold</strong> <em>italic</em> <a href="https://example.com">link</a></p></article>"#;
    let result = extract(html);
    match result {
        Ok(result) => {
            let content_html = result
                .content_html
                .as_deref()
                .expect("expected Some(content_html)");
            assert!(content_html.contains("<strong>bold</strong>"));
            assert!(content_html.contains("<em>italic</em>"));
            // By default (include_links: false), href is not preserved but link text is
            assert!(content_html.contains("<a>link</a>"));
            assert!(!content_html.contains("href="));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_preserves_list_structure_in_content_html() {
    let html = "<article><ul><li>Item 1</li><li>Item 2<ul><li>Nested</li></ul></li></ul></article>";
    let result = extract(html);
    match result {
        Ok(result) => {
            let content_html = result
                .content_html
                .as_deref()
                .expect("expected Some(content_html)");
            assert!(content_html.contains("<ul>"));
            assert!(content_html.contains("<li>Item 1</li>"));
            assert!(content_html.contains("<li>Nested</li>"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_content_html_is_well_formed_and_escapes_special_chars() {
    let html = "<article><p>5 < 6 & 7 > 3</p></article>";
    let result = extract(html);
    match result {
        Ok(result) => {
            let content_html = result
                .content_html
                .as_deref()
                .expect("expected Some(content_html)");

            assert!(content_html.contains("5 &lt; 6 &amp; 7 &gt; 3"));
            // Verify HTML is parseable by dom_query
            let _parsed = dom_query::Document::from(content_html);
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_preserves_ordered_list_structure() {
    let html = "<article><ol><li>First</li><li>Second</li><li>Third</li></ol></article>";
    let result = extract(html);
    match result {
        Ok(result) => {
            let content_html = result
                .content_html
                .as_deref()
                .expect("expected Some(content_html)");
            assert!(content_html.contains("<ol>"));
            assert!(content_html.contains("<li>First</li>"));
            assert!(content_html.contains("<li>Second</li>"));
            assert!(content_html.contains("</ol>"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_preserves_definition_list_structure() {
    let html = "<article><dl><dt>Term</dt><dd>Definition</dd></dl></article>";
    let result = extract(html);
    match result {
        Ok(result) => {
            let content_html = result
                .content_html
                .as_deref()
                .expect("expected Some(content_html)");
            assert!(content_html.contains("<dl>"));
            assert!(content_html.contains("<dt>Term</dt>"));
            assert!(content_html.contains("<dd>Definition</dd>"));
            assert!(content_html.contains("</dl>"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_preserves_blockquote() {
    let html = "<article><blockquote>Quoted text here</blockquote></article>";
    let result = extract(html);
    match result {
        Ok(result) => {
            let content_html = result
                .content_html
                .as_deref()
                .expect("expected Some(content_html)");
            assert!(content_html.contains("<blockquote>Quoted text here</blockquote>"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_preserves_b_and_i_tags() {
    let html = "<article><p>Text <b>bold</b> and <i>italic</i></p></article>";
    let result = extract(html);
    match result {
        Ok(result) => {
            let content_html = result
                .content_html
                .as_deref()
                .expect("expected Some(content_html)");
            assert!(content_html.contains("<b>bold</b>"));
            assert!(content_html.contains("<i>italic</i>"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}
