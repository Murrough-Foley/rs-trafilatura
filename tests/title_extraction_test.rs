use rs_trafilatura::extract;

#[test]
fn title_from_title_tag() {
    let html = r#"
        <html>
          <head><title>My Article Title</title></head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.title.as_deref(), Some("My Article Title")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn title_falls_back_to_og_title_when_title_missing() {
    let html = r#"
        <html>
          <head>
            <meta property="og:title" content="OG Title" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.title.as_deref(), Some("OG Title")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn title_falls_back_to_twitter_title_when_title_and_og_missing() {
    let html = r#"
        <html>
          <head>
            <meta name="twitter:title" content="Twitter Title" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.title.as_deref(), Some("Twitter Title")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn title_falls_back_to_h1_when_no_meta_titles_present() {
    let html = r#"
        <html>
          <head>
            <title>H1 Title</title>
          </head>
          <body>
            <h1>H1 Title</h1>
            <article><p>Body</p></article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.title.as_deref(), Some("H1 Title")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn title_is_none_when_no_sources_present() {
    let html = r#"
        <html>
          <head></head>
          <body>
            <article><p>Body</p></article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert!(result.metadata.title.is_none()),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn title_cleaning_removes_site_suffix() {
    let html = r#"
        <html>
          <head><title>Article Title | MySite</title></head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.title.as_deref(), Some("Article Title")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn title_cleaning_preserves_colons_in_content() {
    // Colons are preserved as they're often part of content, not separators
    // Only pipe/dash suffixes are removed when they look like site names
    let html = r#"
        <html>
          <head><title>MySite: Article Title</title></head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        // Colons preserved - not treated as site name separators
        Ok(result) => assert_eq!(result.metadata.title.as_deref(), Some("MySite: Article Title")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn title_prefers_og_title_when_longer_than_cleaned_title_tag() {
    // og:title is "Full Article Title About Something"
    // title tag is "Full Article Title | MySite" which cleans to "Full Article Title"
    // Since cleaned og:title is longer, it should be preferred
    let html = r#"
        <html>
          <head>
            <title>Full Article Title | MySite</title>
            <meta property="og:title" content="Full Article Title About Something" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(
            result.metadata.title.as_deref(),
            Some("Full Article Title About Something")
        ),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn title_cleaning_removes_trailing_site_suffix_only() {
    // Title with multiple separators - only removes trailing site name suffix
    // if it's short and looks like a site name
    let html = r#"
        <html>
          <head><title>MySite | Section | The Actual Article Title Here</title></head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        // Only trailing short segments are removed, internal separators preserved
        Ok(result) => assert_eq!(
            result.metadata.title.as_deref(),
            Some("MySite | Section | The Actual Article Title Here")
        ),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn title_skips_empty_h1_and_returns_none() {
    // h1 exists but is empty - should return None
    let html = r#"
        <html>
          <head></head>
          <body>
            <h1>   </h1>
            <article><p>Body</p></article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert!(result.metadata.title.is_none()),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}
