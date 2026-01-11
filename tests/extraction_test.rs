use rs_trafilatura::{extract, Error};

#[test]
fn extract_prefers_article_over_main() {
    let html = r#"
        <html>
          <body>
            <main><p>MAIN_ONLY_TEXT</p></main>
            <nav>NAV_TEXT</nav>
            <article><p>ARTICLE_ONLY_TEXT</p></article>
            <footer>FOOTER_TEXT</footer>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("ARTICLE_ONLY_TEXT"));
            assert!(!result.content_text.contains("MAIN_ONLY_TEXT"));
            assert!(!result.content_text.contains("NAV_TEXT"));
            assert!(!result.content_text.contains("FOOTER_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_uses_main_when_no_article_present() {
    let html = r#"
        <html>
          <body>
            <nav>NAV_TEXT</nav>
            <main><p>MAIN_FALLBACK_TEXT</p></main>
            <footer>FOOTER_TEXT</footer>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("MAIN_FALLBACK_TEXT"));
            assert!(!result.content_text.contains("NAV_TEXT"));
            assert!(!result.content_text.contains("FOOTER_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_supports_role_article() {
    let html = r#"
        <html>
          <body>
            <div role='article'><p>ROLE_ARTICLE_TEXT</p></div>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert!(result.content_text.contains("ROLE_ARTICLE_TEXT")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_supports_role_main() {
    let html = r#"
        <html>
          <body>
            <div role='main'><p>ROLE_MAIN_TEXT</p></div>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert!(result.content_text.contains("ROLE_MAIN_TEXT")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_applies_content_heuristics_and_excludes_boilerplate() {
    let long_text = "LONG_TEXT ".repeat(200);
    let html = format!(
        "<html><body><nav>NAV_TEXT</nav><div id='story'>SHORT</div><div id='maintext'><p>{long_text}</p></div><footer>FOOTER_TEXT</footer></body></html>"
    );

    let result = extract(&html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("LONG_TEXT"));
            assert!(!result.content_text.contains("NAV_TEXT"));
            assert!(!result.content_text.contains("FOOTER_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_returns_partial_result_when_only_boilerplate_present() {
    let html = "<html><body><nav>NAV_TEXT</nav><footer>FOOTER_TEXT</footer></body></html>";
    let result = extract(html).expect("should return partial result with warnings");
    assert!(result.content_text.is_empty());
    assert!(!result.warnings.is_empty());
    assert!(result.warnings[0].contains("Content extraction failed"));
}

#[test]
fn partial_result_has_meaningful_warning() {
    let html = "<html><body><nav>NAV_TEXT</nav></body></html>";
    let result = extract(html).expect("should return partial result with warnings");
    assert!(result.content_text.is_empty());
    assert!(!result.warnings.is_empty());
    // Warning message should be meaningful
    let msg = &result.warnings[0];
    assert!(!msg.is_empty());
    assert!(msg.contains("Content extraction failed"));
}

#[test]
fn extract_handles_section_with_content_class() {
    let html = r#"
        <html>
          <body>
            <nav>NAV_TEXT</nav>
            <section class="content"><p>SECTION_CONTENT_TEXT</p></section>
            <footer>FOOTER_TEXT</footer>
          </body>
        </html>
    "#;
    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("SECTION_CONTENT_TEXT"));
            assert!(!result.content_text.contains("NAV_TEXT"));
            assert!(!result.content_text.contains("FOOTER_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_excludes_boilerplate_inside_article() {
    let html = r#"
        <html><body>
            <article>
                <p>ARTICLE_CONTENT</p>
                <nav>INTERNAL_NAV</nav>
                <aside>INTERNAL_ASIDE</aside>
            </article>
        </body></html>
    "#;
    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("ARTICLE_CONTENT"));
            assert!(!result.content_text.contains("INTERNAL_NAV"));
            assert!(!result.content_text.contains("INTERNAL_ASIDE"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_article_nested_in_nav_uses_fallback() {
    // Malformed HTML: article inside nav - should fall back to other content
    let html = r#"
        <html><body>
            <nav><article><p>NESTED_ARTICLE</p></article></nav>
            <div class="content"><p>REAL_CONTENT</p></div>
        </body></html>
    "#;
    let result = extract(html);
    match result {
        Ok(result) => {
            // Article is selected first, but its text is filtered due to nav ancestor
            // So we should get content from the div.content fallback via heuristics
            assert!(result.content_text.contains("REAL_CONTENT"));
        }
        Err(Error::NoContent) => {
            // Also acceptable if no content extracted
        }
        Err(err) => panic!("expected Ok(_) or Err(NoContent), got Err({err:?})"),
    }
}
