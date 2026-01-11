use rs_trafilatura::extract;

#[test]
fn subtitle_extracted_from_h1_following_paragraph() {
    let html = r#"
        <html>
          <body>
            <article>
              <h1>Main Title</h1>
              <p>This is the subtitle that should be extracted.</p>
              <p>This is the main content paragraph.</p>
            </article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            let content = result.content_text;
            assert!(content.contains("This is the subtitle that should be extracted"),
                "Content should contain subtitle: {content:?}");
            assert!(content.contains("This is the main content paragraph"),
                "Content should contain main content");
            // Subtitle should appear before main content
            let subtitle_pos = content.find("This is the subtitle");
            let main_content_pos = content.find("This is the main content");
            assert!(subtitle_pos < main_content_pos,
                "Subtitle should appear before main content");
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn subtitle_extracted_from_nytimes_style_deck() {
    // Test NY Times style with css-178vgup class
    let html = r#"
        <html>
          <body>
            <article>
              <h1>How much protein do you need?</h1>
              <div class="css-178vgup">Is there a 'least bad' alcohol? We tackled these questions and more.</div>
              <p>Main content starts here.</p>
            </article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            let content = result.content_text;
            assert!(content.contains("How much protein do you need?"),
                "Content should contain subtitle: {content:?}");
            assert!(content.contains("least bad") || content.contains("tackled these questions"),
                "Content should contain deck/subtitle text");
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn subtitle_with_common_selectors() {
    // Test various subtitle selector patterns
    let html = r#"
        <html>
          <body>
            <article>
              <h1>Article Title</h1>
              <p class="subtitle">This is a subtitle</p>
              <p class="deck">This is a deck</p>
              <p class="excerpt">This is an excerpt</p>
              <p>Main content.</p>
            </article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            let content = result.content_text;
            // Should extract at least one of the subtitle/deck/excerpt elements
            assert!(content.contains("subtitle") || content.contains("deck") || content.contains("excerpt"),
                "Content should contain subtitle-like text: {content:?}");
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn no_subtitle_when_not_present() {
    // Test that extraction still works when no subtitle is present
    let html = r#"
        <html>
          <body>
            <article>
              <h1>Simple Title</h1>
              <p>Just regular content without a subtitle.</p>
            </article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            let content = result.content_text;
            assert!(content.contains("Just regular content without a subtitle"),
                "Content should contain main text even without subtitle: {content:?}");
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn subtitle_prepended_to_content() {
    // Ensure subtitle appears at the beginning of content
    let html = r#"
        <html>
          <body>
            <article>
              <h1>Title</h1>
              <p>Subtitle text</p>
              <div>Some middle content</div>
              <p>End content</p>
            </article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            let content = result.content_text;
            // Subtitle should be first or near first
            let first_100_chars = &content[..content.len().min(100)];
            assert!(first_100_chars.contains("Subtitle"),
                "Subtitle should be near the beginning: {first_100_chars:?}");
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}
