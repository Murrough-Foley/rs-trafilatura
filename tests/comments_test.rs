use rs_trafilatura::{extract, extract_with_options, Options};

#[test]
fn extract_excludes_comments_by_default() {
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
            assert!(result.content_text.contains("ARTICLE_MARKER"));
            assert!(!result.content_text.contains("COMMENT_MARKER"));

            let content_html = result
                .content_html
                .as_deref()
                .expect("expected Some(content_html)");
            assert!(content_html.contains("ARTICLE_MARKER"));
            assert!(!content_html.contains("COMMENT_MARKER"));

            assert!(result.comments_text.is_none());
            assert!(result.comments_html.is_none());
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_includes_comments_when_option_enabled() {
    let html = r#"
        <html><body>
            <article><p>ARTICLE_MARKER</p></article>
            <section class="comments">
                <ul>
                    <li>COMMENT_1</li>
                    <li>COMMENT_2<ul><li>REPLY</li></ul></li>
                </ul>
            </section>
        </body></html>
    "#;

    let options = Options {
        include_comments: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("ARTICLE_MARKER"));
            assert!(!result.content_text.contains("COMMENT_1"));

            let comments_text = result
                .comments_text
                .as_deref()
                .expect("expected Some(comments_text)");
            assert!(comments_text.contains("COMMENT_1"));
            assert!(comments_text.contains("REPLY"));

            let comments_html = result
                .comments_html
                .as_deref()
                .expect("expected Some(comments_html)");
            assert!(comments_html.contains("<ul>"));
            assert!(comments_html.contains("COMMENT_2"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_detects_disqus_container_as_comments() {
    let html = r#"
        <html><body>
            <article><p>ARTICLE_MARKER</p></article>
            <div id="disqus_thread"><p>DISQUS_MARKER</p></div>
        </body></html>
    "#;

    let options = Options {
        include_comments: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options);
    match result {
        Ok(result) => {
            let comments_text = result
                .comments_text
                .as_deref()
                .expect("expected Some(comments_text)");
            assert!(comments_text.contains("DISQUS_MARKER"));

            assert!(!result.content_text.contains("DISQUS_MARKER"));
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
    let html = r#"
        <html><body>
            <article><p>ARTICLE_MARKER</p></article>
            <div class="fb-comments"><p>FB_COMMENT_MARKER</p></div>
        </body></html>
    "#;

    let options = Options {
        include_comments: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options);
    match result {
        Ok(result) => {
            let comments_text = result
                .comments_text
                .as_deref()
                .expect("expected Some(comments_text)");
            assert!(comments_text.contains("FB_COMMENT_MARKER"));
            assert!(!result.content_text.contains("FB_COMMENT_MARKER"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_detects_respond_id_as_comment_section() {
    let html = r#"
        <html><body>
            <article><p>ARTICLE_MARKER</p></article>
            <div id="respond"><p>RESPOND_MARKER</p></div>
        </body></html>
    "#;

    let options = Options {
        include_comments: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options);
    match result {
        Ok(result) => {
            let comments_text = result
                .comments_text
                .as_deref()
                .expect("expected Some(comments_text)");
            assert!(comments_text.contains("RESPOND_MARKER"));
            assert!(!result.content_text.contains("RESPOND_MARKER"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn extract_detects_comment_list_class_via_regex_fallback() {
    let html = r#"
        <html><body>
            <article><p>ARTICLE_MARKER</p></article>
            <div class="post-comment-list"><p>COMMENT_LIST_MARKER</p></div>
        </body></html>
    "#;

    let options = Options {
        include_comments: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options);
    match result {
        Ok(result) => {
            let comments_text = result
                .comments_text
                .as_deref()
                .expect("expected Some(comments_text)");
            assert!(comments_text.contains("COMMENT_LIST_MARKER"));
            assert!(!result.content_text.contains("COMMENT_LIST_MARKER"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}
