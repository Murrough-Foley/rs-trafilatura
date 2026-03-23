use rs_trafilatura::{extract, extract_with_options, Options};

/// Sufficient comment text to pass the minimum comment word count threshold (>= 10 words)
const COMMENTS_CONTENT: &str = "<p>First comment with sufficient words to pass the minimum threshold requirement for comment extraction.</p><p>Second comment adding more content to ensure the word count is adequate for comment detection.</p>";

/// Article padding for proper content extraction
const ARTICLE_CONTENT: &str = "<p>Main article content here with sufficient text to pass extraction threshold.</p><p>Second article paragraph adds more substance for proper content scoring and extraction.</p>";

#[test]
fn extract_excludes_comments_by_default() {
    // By default (include_comments: false), comments_text and comments_html are None
    let html = r#"
        <html><body>
            <article><p>ARTICLE_MARKER</p></article>
            <div id="comments">
                <p>COMMENT_MARKER</p>
            </div>
        </body></html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            // Article content is extracted
            assert!(result.content_text.contains("ARTICLE_MARKER"));
            // With include_comments: false, comments fields are always None
            assert!(result.comments_text.is_none());
            assert!(result.comments_html.is_none());
            // Note: COMMENT_MARKER may or may not appear in content_text depending on
            // whether the extractor detects and excludes the comment div
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_includes_comments_when_option_enabled() {
    // Comments need >= 10 words and a recognized comment container
    let html = format!(r#"
        <html><body>
            <article>{ARTICLE_CONTENT}</article>
            <section class="comments">
                {COMMENTS_CONTENT}
            </section>
        </body></html>
    "#);

    let options = Options {
        include_comments: true,
        ..Options::default()
    };

    let result = extract_with_options(&html, &options);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("Main article content"));
            // Comments should be populated (class="comments" is recognized, >= 10 words)
            assert!(result.comments_text.is_some(), "comments_text should be Some - found: {:?}", result.comments_text);
            let comments_text = result.comments_text.unwrap();
            assert!(comments_text.contains("First comment"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_detects_disqus_container_as_comments() {
    let html = format!(r#"
        <html><body>
            <article>{ARTICLE_CONTENT}</article>
            <div id="disqus_thread">{COMMENTS_CONTENT}</div>
        </body></html>
    "#);

    let options = Options {
        include_comments: true,
        ..Options::default()
    };

    let result = extract_with_options(&html, &options);
    match result {
        Ok(result) => {
            // Comments should be detected from disqus_thread
            assert!(result.comments_text.is_some(), "disqus comments should be detected");
            let comments_text = result.comments_text.unwrap();
            assert!(comments_text.contains("First comment"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_returns_none_when_no_comments_found() {
    let html = r#"<html><body><article><p>ARTICLE_MARKER</p></article></body></html>"#;

    let options = Options {
        include_comments: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options);
    match result {
        Ok(result) => {
            assert!(result.comments_text.is_none());
            assert!(result.comments_html.is_none());
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_detects_fb_comments_container() {
    let html = format!(r#"
        <html><body>
            <article>{ARTICLE_CONTENT}</article>
            <div class="fb-comments">{COMMENTS_CONTENT}</div>
        </body></html>
    "#);

    let options = Options {
        include_comments: true,
        ..Options::default()
    };

    let result = extract_with_options(&html, &options);
    match result {
        Ok(result) => {
            // fb-comments class should be detected as comment section
            assert!(result.comments_text.is_some(), "fb-comments should be detected");
            let comments_text = result.comments_text.unwrap();
            assert!(comments_text.contains("First comment"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_detects_respond_id_as_comment_section() {
    let html = format!(r#"
        <html><body>
            <article>{ARTICLE_CONTENT}</article>
            <div id="respond">{COMMENTS_CONTENT}</div>
        </body></html>
    "#);

    let options = Options {
        include_comments: true,
        ..Options::default()
    };

    let result = extract_with_options(&html, &options);
    match result {
        Ok(result) => {
            // id="respond" should be detected as comment section
            assert!(result.comments_text.is_some(), "respond section should be detected");
            let comments_text = result.comments_text.unwrap();
            assert!(comments_text.contains("First comment"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_detects_comment_list_class_via_regex_fallback() {
    let html = format!(r#"
        <html><body>
            <article>{ARTICLE_CONTENT}</article>
            <div class="post-comment-list">{COMMENTS_CONTENT}</div>
        </body></html>
    "#);

    let options = Options {
        include_comments: true,
        ..Options::default()
    };

    let result = extract_with_options(&html, &options);
    match result {
        Ok(result) => {
            // post-comment-list class should match COMMENT_CLASS regex
            assert!(result.comments_text.is_some(), "comment-list should be detected via regex");
            let comments_text = result.comments_text.unwrap();
            assert!(comments_text.contains("First comment"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}
