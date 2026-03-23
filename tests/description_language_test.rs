use rs_trafilatura::extract;

#[test]
fn description_from_meta_description() {
    let html = r#"
        <html>
          <head>
            <meta name="description" content="Meta description" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.description.as_deref(), Some("Meta description")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn description_falls_back_to_og_description() {
    let html = r#"
        <html>
          <head>
            <meta property="og:description" content="OG description" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.description.as_deref(), Some("OG description")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn description_falls_back_to_twitter_description() {
    let html = r#"
        <html>
          <head>
            <meta name="twitter:description" content="Twitter description" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.description.as_deref(), Some("Twitter description")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn language_from_html_lang_is_normalized() {
    let html = r#"
        <html lang="en-US">
          <head></head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.language.as_deref(), Some("en")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn language_from_content_language_meta() {
    let html = r#"
        <html>
          <head>
            <meta http-equiv="content-language" content="de" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.language.as_deref(), Some("de")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn language_from_og_locale_is_normalized() {
    let html = r#"
        <html>
          <head>
            <meta property="og:locale" content="pt_BR" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.language.as_deref(), Some("pt")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn language_is_none_when_no_indicators() {
    let html = r#"
        <html>
          <head></head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert!(result.metadata.language.is_none()),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn description_is_none_when_no_sources() {
    let html = r#"
        <html>
          <head></head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert!(result.metadata.description.is_none()),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn language_from_simple_code_without_region() {
    // Simple language code "fr" without region suffix
    let html = r#"
        <html lang="fr">
          <head></head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.language.as_deref(), Some("fr")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn language_from_meta_name_language() {
    // Fallback to <meta name="language">
    let html = r#"
        <html>
          <head>
            <meta name="language" content="es" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.language.as_deref(), Some("es")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}
