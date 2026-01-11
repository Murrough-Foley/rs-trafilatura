use rs_trafilatura::extract;

#[test]
fn extract_formats_simple_tables_in_content_text_and_preserves_in_content_html() {
    let html = r#"
        <article>
            <p>Intro</p>
            <table>
                <tr><th>H1</th><th>H2</th></tr>
                <tr><td>A</td><td>B</td></tr>
            </table>
        </article>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("H1 | H2"));
            assert!(result.content_text.contains("A | B"));

            let content_html = result
                .content_html
                .as_deref()
                .expect("expected Some(content_html)");
            assert!(content_html.contains("<table>"));
            assert!(content_html.contains("<tr>"));
            assert!(content_html.contains("<th>H1</th>"));
            assert!(content_html.contains("<td>A</td>"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_treats_layout_tables_as_regular_content() {
    let html = r#"
        <article>
            <table role="presentation">
                <tr><td><p>LAYOUT_MARKER</p></td></tr>
            </table>
        </article>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("LAYOUT_MARKER"));
            assert!(!result.content_text.contains('|'));

            let content_html = result
                .content_html
                .as_deref()
                .expect("expected Some(content_html)");
            assert!(!content_html.contains("<table"));
            assert!(content_html.contains("<p>LAYOUT_MARKER</p>"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_handles_colspan_and_rowspan_in_table_text() {
    let html = r#"
        <article>
            <table>
                <tr><th>H1</th><th>H2</th></tr>
                <tr><td colspan="2">X</td></tr>
                <tr><td rowspan="2">A</td><td>B1</td></tr>
                <tr><td>B2</td></tr>
            </table>
        </article>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("X | X"));
            assert!(result.content_text.contains("A | B1"));
            assert!(result.content_text.contains("A | B2"));

            let content_html = result
                .content_html
                .as_deref()
                .expect("expected Some(content_html)");
            assert!(content_html.contains(r#"<td colspan="2">X</td>"#));
            assert!(content_html.contains(r#"<td rowspan="2">A</td>"#));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_handles_large_tables_without_panic() {
    let mut table = String::from("<article><table>");
    table.push_str("<tr><th>H1</th><th>H2</th><th>H3</th></tr>");
    for r in 0..200 {
        table.push_str("<tr>");
        for c in 0..20 {
            table.push_str(&format!("<td>R{r}C{c}</td>"));
        }
        table.push_str("</tr>");
    }
    table.push_str("</table></article>");

    let result = extract(&table);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("H1 | H2 | H3"));
            assert!(result.content_text.contains("R0C0"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_treats_single_row_table_as_layout() {
    let html = r#"
        <article>
            <table>
                <tr><td>SINGLE_ROW_CONTENT</td><td>More</td></tr>
            </table>
            <p>BODY_TEXT</p>
        </article>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            // Single row table is treated as layout - content extracted but no pipe separators
            assert!(result.content_text.contains("SINGLE_ROW_CONTENT"));
            assert!(result.content_text.contains("BODY_TEXT"));
            // Layout tables don't get pipe-formatted
            assert!(!result.content_text.contains("SINGLE_ROW_CONTENT | More"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}
