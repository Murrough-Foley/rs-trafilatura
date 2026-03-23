use rs_trafilatura::{extract, extract_with_options, Options};

#[test]
fn sitename_from_og_site_name() {
    let html = r#"
        <html>
          <head>
            <meta property="og:site_name" content="Example Site" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.sitename.as_deref(), Some("Example Site")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn url_from_canonical_link() {
    let html = r#"
        <html>
          <head>
            <link rel="canonical" href="https://example.com/canonical" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.url.as_deref(), Some("https://example.com/canonical")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn url_falls_back_to_og_url_when_no_canonical() {
    let html = r#"
        <html>
          <head>
            <meta property="og:url" content="https://example.com/og" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.url.as_deref(), Some("https://example.com/og")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn hostname_extracted_from_options_url() {
    let html = "<html><body><article><p>Body</p></article></body></html>";
    let options = Options {
        url: Some("https://example.com/some/path?x=1".to_string()),
        ..Options::default()
    };

    let result = extract_with_options(html, &options);
    match result {
        Ok(result) => assert_eq!(result.metadata.hostname.as_deref(), Some("example.com")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn sitename_can_fall_back_to_title_suffix() {
    let html = r#"
        <html>
          <head>
            <title>Article Title | MySite</title>
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.sitename.as_deref(), Some("MySite")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn sitename_is_none_when_no_sources() {
    let html = r#"
        <html>
          <head><title>Article Title</title></head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert!(result.metadata.sitename.is_none()),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn url_is_none_when_invalid_url_found() {
    let html = r#"
        <html>
          <head>
            <link rel="canonical" href="not-a-url" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert!(result.metadata.url.is_none()),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn hostname_falls_back_to_extracted_url_when_no_options_url() {
    // When options.url is not provided, hostname should be extracted from canonical URL
    let html = r#"
        <html>
          <head>
            <link rel="canonical" href="https://fallback-example.com/article" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert_eq!(result.metadata.url.as_deref(), Some("https://fallback-example.com/article"));
            assert_eq!(result.metadata.hostname.as_deref(), Some("fallback-example.com"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn sitename_falls_back_to_application_name() {
    let html = r#"
        <html>
          <head>
            <meta name="application-name" content="MyApp" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.sitename.as_deref(), Some("MyApp")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn url_is_none_when_no_sources_present() {
    let html = r#"
        <html>
          <head></head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert!(result.metadata.url.is_none()),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}
