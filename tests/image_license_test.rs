use rs_trafilatura::extract;

#[test]
fn image_from_og_image() {
    let html = r#"
        <html>
          <head>
            <meta property="og:image" content="https://example.com/og.png" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.image.as_deref(), Some("https://example.com/og.png")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn license_from_dc_rights_meta_creative_commons_url_is_normalized() {
    let html = r#"
        <html>
          <head>
            <meta name="dc.rights" content="https://creativecommons.org/licenses/by/4.0/" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.license.as_deref(), Some("CC BY 4.0")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn image_falls_back_to_twitter_image_name() {
    let html = r#"
        <html>
          <head>
            <meta name="twitter:image" content="https://example.com/tw.png" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.image.as_deref(), Some("https://example.com/tw.png")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn image_falls_back_to_twitter_image_property() {
    let html = r#"
        <html>
          <head>
            <meta property="twitter:image" content="https://example.com/twprop.png" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.image.as_deref(), Some("https://example.com/twprop.png")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn license_from_link_rel_license() {
    let html = r#"
        <html>
          <head>
            <link rel="license" href="https://creativecommons.org/licenses/by/4.0/" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(
            result.metadata.license.as_deref(),
            Some("https://creativecommons.org/licenses/by/4.0/")
        ),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn license_from_anchor_rel_license() {
    let html = r#"
        <html>
          <body>
            <a rel="license" href="https://example.com/license">License</a>
            <article><p>Body</p></article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.license.as_deref(), Some("https://example.com/license")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn license_from_dc_rights_meta() {
    let html = r#"
        <html>
          <head>
            <meta name="dc.rights" content="CC BY 4.0" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.license.as_deref(), Some("CC BY 4.0")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn no_image_and_license_are_none() {
    let html = r#"<html><head></head><body><article><p>Body</p></article></body></html>"#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.metadata.image.is_none());
            assert!(result.metadata.license.is_none());
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn license_from_dcterms_license_meta() {
    let html = r#"
        <html>
          <head>
            <meta name="dcterms.license" content="MIT License" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.license.as_deref(), Some("MIT License")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn license_normalizes_cc0_public_domain() {
    let html = r#"
        <html>
          <head>
            <meta name="dc.rights" content="https://creativecommons.org/publicdomain/zero/1.0/" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.license.as_deref(), Some("CC0 1.0")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}
