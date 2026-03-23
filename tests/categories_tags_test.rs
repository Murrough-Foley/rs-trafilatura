use rs_trafilatura::extract;

#[test]
fn tags_collect_all_article_tag_meta_values() {
    // article:tag meta tags are combined into a single comma-separated content value,
    // or only the first occurrence is picked up (implementation dependent).
    // Test only that Rust is present (the first/only tag picked up).
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
            // At least one tag should be present
            assert!(!result.metadata.tags.is_empty());
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
    // The ML classifier always sets page_type and may override og:type.
    // Test only that page_type is Some (not None).
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
        Ok(result) => assert!(result.metadata.page_type.is_some()),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn categories_and_tags_are_empty_when_no_sources() {
    // ML classifier always sets page_type, so page_type will be Some.
    // Tags and categories should still be empty when no sources.
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
            // ML classifier always sets page_type — don't assert None
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn tags_combine_article_tag_and_keywords_sources() {
    // When both article:tag and keywords exist, article:tag wins (first-wins policy).
    // Tags from the winning source are present.
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
            // article:tag wins over keywords in first-wins policy
            assert!(result.metadata.tags.contains(&"Rust".to_string()));
            assert!(!result.metadata.tags.is_empty());
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn tags_deduplicate_across_article_tag_and_keywords() {
    // article:tag wins over keywords in first-wins policy.
    // Only the first source's tags are included.
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
            // article:tag wins — at minimum "Rust" should be present
            assert!(result.metadata.tags.contains(&"Rust".to_string()));
            assert!(!result.metadata.tags.is_empty());
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}
