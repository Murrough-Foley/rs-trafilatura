use chrono::{TimeZone, Utc};

use rs_trafilatura::extract;

#[test]
fn author_from_meta_is_extracted_and_cleaned() {
    let html = r#"
        <html>
          <head>
            <meta name="author" content="By Alice" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.author.as_deref(), Some("Alice")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn author_from_article_author_meta_is_extracted() {
    let html = r#"
        <html>
          <head>
            <meta property="article:author" content="Dana" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.author.as_deref(), Some("Dana")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn author_from_class_selector_is_extracted_and_cleaned() {
    let html = r#"
        <html>
          <body>
            <div class="byline">Written by Bob</div>
            <article><p>Body</p></article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.author.as_deref(), Some("Bob")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn author_from_jsonld_is_extracted() {
    let html = r#"
        <html>
          <body>
            <script type="application/ld+json">
              {"@type":"NewsArticle","author":{"name":"Carol"}}
            </script>
            <article><p>Body</p></article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert_eq!(result.metadata.author.as_deref(), Some("Carol")),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn date_from_meta_published_time_is_parsed() {
    let html = r#"
        <html>
          <head>
            <meta property="article:published_time" content="2024-01-15T12:34:56Z" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            let expected = Utc.with_ymd_and_hms(2024, 1, 15, 12, 34, 56).single();
            assert_eq!(result.metadata.date, expected);
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn date_from_meta_modified_time_is_parsed() {
    let html = r#"
        <html>
          <head>
            <meta property="article:modified_time" content="2024-01-16T01:02:03Z" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            let expected = Utc.with_ymd_and_hms(2024, 1, 16, 1, 2, 3).single();
            assert_eq!(result.metadata.date, expected);
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn date_from_time_datetime_is_parsed() {
    let html = r#"
        <html>
          <body>
            <time datetime="2024-01-15">January 15, 2024</time>
            <article><p>Body</p></article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            let expected = Utc.with_ymd_and_hms(2024, 1, 15, 0, 0, 0).single();
            assert_eq!(result.metadata.date, expected);
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn date_from_human_readable_text_is_parsed() {
    let html = r#"
        <html>
          <body>
            <time>January 15, 2024</time>
            <article><p>Body</p></article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            let expected = Utc.with_ymd_and_hms(2024, 1, 15, 0, 0, 0).single();
            assert_eq!(result.metadata.date, expected);
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn date_parse_failure_is_graceful() {
    let html = r#"
        <html>
          <head>
            <meta property="article:published_time" content="not-a-date" />
          </head>
          <body><article><p>Body</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert!(result.metadata.date.is_none()),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn author_is_none_when_no_sources_present() {
    let html = r#"
        <html>
          <head></head>
          <body><article><p>Body content here</p></article></body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert!(result.metadata.author.is_none()),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn date_parses_short_month_format() {
    // "15 Jan 2024" format (%d %b %Y)
    let html = r#"
        <html>
          <body>
            <time>15 Jan 2024</time>
            <article><p>Body</p></article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            let expected = Utc.with_ymd_and_hms(2024, 1, 15, 0, 0, 0).single();
            assert_eq!(result.metadata.date, expected);
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn date_parses_us_date_format() {
    // "01/15/2024" format (%m/%d/%Y)
    let html = r#"
        <html>
          <body>
            <time>01/15/2024</time>
            <article><p>Body</p></article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            let expected = Utc.with_ymd_and_hms(2024, 1, 15, 0, 0, 0).single();
            assert_eq!(result.metadata.date, expected);
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn date_is_none_when_time_element_is_empty() {
    let html = r#"
        <html>
          <body>
            <time>   </time>
            <article><p>Body</p></article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => assert!(result.metadata.date.is_none()),
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}
