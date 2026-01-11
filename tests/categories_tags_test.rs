use rs_trafilatura::extract;

#[test]
fn tags_collect_all_article_tag_meta_values() {
    let html = r#"
        <html>
          <head>
            <meta property="article:tag" content="Rust" />
            <meta property="article:tag" content="Web" />
            <meta property="article:tag" content="Rust" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.metadata.tags.contains(&"Rust".to_string()));
            assert!(result.metadata.tags.contains(&"Web".to_string()));
            assert_eq!(result.metadata.tags.len(), 2);
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn tags_parse_keywords_meta_comma_separated() {
    let html = r#"
        <html>
          <head>
            <meta name="keywords" content=" rust,  scraping , ,web " />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.metadata.tags.contains(&"rust".to_string()));
            assert!(result.metadata.tags.contains(&"scraping".to_string()));
            assert!(result.metadata.tags.contains(&"web".to_string()));
            assert_eq!(result.metadata.tags.len(), 3);
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn categories_extract_article_section() {
    let html = r#"
        <html>
          <head>
            <meta property="article:section" content="Technology" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert_eq!(result.metadata.categories, vec!["Technology".to_string()]);
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn page_type_extracts_og_type() {
    let html = r#"
        <html>
          <head>
            <meta property="og:type" content="article" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.page_type.as_deref(), Some("article")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn categories_and_tags_are_empty_when_no_sources() {
    let html = r#"
        <html>
          <head></head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.metadata.tags.is_empty());
            assert!(result.metadata.categories.is_empty());
            assert!(result.metadata.page_type.is_none());
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn tags_combine_article_tag_and_keywords_sources() {
    // When both article:tag and keywords exist, tags should be combined
    let html = r#"
        <html>
          <head>
            <meta property="article:tag" content="Rust" />
            <meta name="keywords" content="programming, web" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.metadata.tags.contains(&"Rust".to_string()));
            assert!(result.metadata.tags.contains(&"programming".to_string()));
            assert!(result.metadata.tags.contains(&"web".to_string()));
            assert_eq!(result.metadata.tags.len(), 3);
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn tags_deduplicate_across_article_tag_and_keywords() {
    // Duplicate tags across sources should be deduplicated
    let html = r#"
        <html>
          <head>
            <meta property="article:tag" content="Rust" />
            <meta property="article:tag" content="Web" />
            <meta name="keywords" content="rust, web, programming" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            // "Rust" from article:tag and "rust" from keywords are different (case-sensitive)
            // "Web" from article:tag and "web" from keywords are different (case-sensitive)
            // Should have: Rust, Web, rust, web, programming = 5 tags
            assert!(result.metadata.tags.contains(&"Rust".to_string()));
            assert!(result.metadata.tags.contains(&"Web".to_string()));
            assert!(result.metadata.tags.contains(&"programming".to_string()));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}
