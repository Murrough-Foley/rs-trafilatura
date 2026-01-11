use rs_trafilatura::{extract_with_options, ImageData, Options};

/// Helper to check if images contain a URL
fn images_contain_src(images: &[ImageData], src: &str) -> bool {
    images.iter().any(|img| img.src == src)
}

// ============================================================================
// IMAGE TOGGLE TESTS
// ============================================================================

/// Test AC#1: include_images: true collects image URLs
#[test]
fn include_images_true_collects_image_urls() {
    let html = r#"
        <html><body>
            <article>
                <p>Article with images.</p>
                <img src="https://example.com/image1.jpg" alt="Image 1">
                <p>More text.</p>
                <img src="https://example.com/image2.png" alt="Image 2">
            </article>
        </body></html>
    "#;

    let options = Options {
        include_images: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    // Should have article text
    assert!(result.content_text.contains("Article with images"));

    // Should have collected image URLs
    assert_eq!(result.images.len(), 2);
    assert!(images_contain_src(&result.images, "https://example.com/image1.jpg"));
    assert!(images_contain_src(&result.images, "https://example.com/image2.png"));
}

/// Test AC#2: include_images: false (default) returns empty images vec
#[test]
fn include_images_false_returns_empty_images() {
    let html = r#"
        <html><body>
            <article>
                <p>Article with images.</p>
                <img src="https://example.com/image1.jpg" alt="Image 1">
                <img src="https://example.com/image2.png" alt="Image 2">
            </article>
        </body></html>
    "#;

    // Test with explicit false
    let options = Options {
        include_images: false,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    // Should have article text
    assert!(result.content_text.contains("Article with images"));

    // Should NOT have collected images
    assert!(result.images.is_empty());
}

/// Test that default Options has include_images: false
#[test]
fn default_options_excludes_images() {
    let html = r#"
        <html><body>
            <article>
                <p>Content here.</p>
                <img src="https://example.com/image.jpg">
            </article>
        </body></html>
    "#;

    let result = extract_with_options(html, &Options::default()).expect("extraction failed");

    // Default should NOT include images
    assert!(result.images.is_empty());
}

/// Test image extraction with lazy-loaded images (data-src)
#[test]
fn include_images_extracts_lazy_loaded_images() {
    let html = r#"
        <html><body>
            <article>
                <p>Content with lazy images.</p>
                <img data-src="https://example.com/lazy.jpg" alt="Lazy">
                <img src="https://example.com/normal.jpg" alt="Normal">
            </article>
        </body></html>
    "#;

    let options = Options {
        include_images: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    // Should collect both normal and data-src images
    assert_eq!(result.images.len(), 2);
    assert!(images_contain_src(&result.images, "https://example.com/lazy.jpg"));
    assert!(images_contain_src(&result.images, "https://example.com/normal.jpg"));
}

/// Test that duplicate images are not added twice
#[test]
fn include_images_deduplicates_urls() {
    let html = r#"
        <html><body>
            <article>
                <p>Article with duplicate images.</p>
                <img src="https://example.com/same.jpg" alt="Same 1">
                <img src="https://example.com/same.jpg" alt="Same 2">
                <img src="https://example.com/different.jpg" alt="Different">
            </article>
        </body></html>
    "#;

    let options = Options {
        include_images: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    // Should only have 2 unique images
    assert_eq!(result.images.len(), 2);
    assert!(images_contain_src(&result.images, "https://example.com/same.jpg"));
    assert!(images_contain_src(&result.images, "https://example.com/different.jpg"));
}

// ============================================================================
// LINK TOGGLE TESTS
// ============================================================================

/// Test AC#3: include_links: true preserves href in content_html
#[test]
fn include_links_true_preserves_href_in_html() {
    let html = r#"
        <html><body>
            <article>
                <p>Read more at <a href="https://example.com/article">this link</a>.</p>
            </article>
        </body></html>
    "#;

    let options = Options {
        include_links: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    // Should have link text in content_text
    assert!(result.content_text.contains("this link"));

    // Should have href preserved in content_html
    if let Some(html) = result.content_html {
        assert!(html.contains("href=\"https://example.com/article\""));
        assert!(html.contains("<a href"));
    } else {
        panic!("content_html should be Some");
    }
}

/// Test AC#4: include_links: false (default) strips href from content_html
#[test]
fn include_links_false_strips_href_from_html() {
    let html = r#"
        <html><body>
            <article>
                <p>Read more at <a href="https://example.com/article">this link</a>.</p>
            </article>
        </body></html>
    "#;

    // Test with explicit false
    let options = Options {
        include_links: false,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    // Should have link text in content_text
    assert!(result.content_text.contains("this link"));

    // Should NOT have href in content_html (link tag present but no href attribute)
    if let Some(html) = result.content_html {
        assert!(!html.contains("href="));
        // The <a> tag should still be there, just without href
        assert!(html.contains("<a>"));
    }
}

/// Test that default Options has include_links: false
#[test]
fn default_options_excludes_link_urls() {
    let html = r#"
        <html><body>
            <article>
                <p>Check <a href="https://example.com">this</a> out.</p>
            </article>
        </body></html>
    "#;

    let result = extract_with_options(html, &Options::default()).expect("extraction failed");

    // Should have link text
    assert!(result.content_text.contains("this"));

    // Default should NOT preserve href
    if let Some(html) = result.content_html {
        assert!(!html.contains("href="));
    }
}

/// Test link preservation with multiple links
#[test]
fn include_links_preserves_multiple_hrefs() {
    let html = r#"
        <html><body>
            <article>
                <p>Visit <a href="https://example.com/page1">page 1</a> and <a href="https://example.com/page2">page 2</a>.</p>
            </article>
        </body></html>
    "#;

    let options = Options {
        include_links: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    if let Some(html) = result.content_html {
        assert!(html.contains("href=\"https://example.com/page1\""));
        assert!(html.contains("href=\"https://example.com/page2\""));
    } else {
        panic!("content_html should be Some");
    }
}

// ============================================================================
// COMBINED TOGGLE TESTS
// ============================================================================

/// Test both toggles enabled
#[test]
fn both_image_and_link_toggles_enabled() {
    let html = r#"
        <html><body>
            <article>
                <p>Content with <a href="https://example.com">link</a> and image.</p>
                <img src="https://example.com/pic.jpg">
            </article>
        </body></html>
    "#;

    let options = Options {
        include_images: true,
        include_links: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    // Should have content
    assert!(result.content_text.contains("Content with"));

    // Should have image
    assert_eq!(result.images.len(), 1);
    assert!(images_contain_src(&result.images, "https://example.com/pic.jpg"));

    // Should have href
    if let Some(html) = result.content_html {
        assert!(html.contains("href=\"https://example.com\""));
    }
}

/// Test both toggles disabled (default)
#[test]
fn both_image_and_link_toggles_disabled() {
    let html = r#"
        <html><body>
            <article>
                <p>Content with <a href="https://example.com">link</a> and image.</p>
                <img src="https://example.com/pic.jpg">
            </article>
        </body></html>
    "#;

    let options = Options {
        include_images: false,
        include_links: false,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    // Should have content
    assert!(result.content_text.contains("Content with"));
    assert!(result.content_text.contains("link"));

    // Should NOT have images
    assert!(result.images.is_empty());

    // Should NOT have href
    if let Some(html) = result.content_html {
        assert!(!html.contains("href="));
    }
}

/// Test that image toggle doesn't affect other content
#[test]
fn image_toggle_doesnt_affect_text_content() {
    let html = r#"
        <html><body>
            <article>
                <h1>Title</h1>
                <p>Paragraph text.</p>
                <img src="https://example.com/image.jpg">
                <p>More text.</p>
            </article>
        </body></html>
    "#;

    let with_images = extract_with_options(html, &Options {
        include_images: true,
        ..Options::default()
    }).expect("extraction failed");

    let without_images = extract_with_options(html, &Options {
        include_images: false,
        ..Options::default()
    }).expect("extraction failed");

    // Text content should be identical
    assert_eq!(with_images.content_text, without_images.content_text);

    // Only difference should be images collection
    assert!(!with_images.images.is_empty());
    assert!(without_images.images.is_empty());
}

/// Test that link toggle doesn't affect text content
#[test]
fn link_toggle_doesnt_affect_text_content() {
    let html = r#"
        <html><body>
            <article>
                <p>Text with <a href="https://example.com">a link</a> here.</p>
            </article>
        </body></html>
    "#;

    let with_links = extract_with_options(html, &Options {
        include_links: true,
        ..Options::default()
    }).expect("extraction failed");

    let without_links = extract_with_options(html, &Options {
        include_links: false,
        ..Options::default()
    }).expect("extraction failed");

    // Text content should be identical (link text preserved in both)
    assert_eq!(with_links.content_text, without_links.content_text);
    assert!(with_links.content_text.contains("a link"));
}

// ============================================================================
// STORY 3: FIGCAPTION EXTRACTION TESTS
// ============================================================================

/// Test that figcaption is extracted from figure elements
#[test]
fn figcaption_extracted_from_figure() {
    let html = r#"
        <html><body>
            <article>
                <p>Article content.</p>
                <figure>
                    <img src="https://example.com/photo.jpg" alt="A photo">
                    <figcaption>This is the caption for the photo.</figcaption>
                </figure>
            </article>
        </body></html>
    "#;

    let options = Options {
        include_images: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    assert_eq!(result.images.len(), 1);
    let img = &result.images[0];
    assert_eq!(img.src, "https://example.com/photo.jpg");
    assert_eq!(img.caption, Some("This is the caption for the photo.".to_string()));
}

/// Test that figcaption whitespace is normalized
#[test]
fn figcaption_whitespace_normalized() {
    let html = r#"
        <html><body>
            <article>
                <figure>
                    <img src="https://example.com/image.jpg">
                    <figcaption>
                        Caption with
                        multiple   spaces   and
                        newlines.
                    </figcaption>
                </figure>
            </article>
        </body></html>
    "#;

    let options = Options {
        include_images: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    assert_eq!(result.images.len(), 1);
    let img = &result.images[0];
    assert_eq!(img.caption, Some("Caption with multiple spaces and newlines.".to_string()));
}

/// Test that images without figcaption have None caption
#[test]
fn standalone_image_has_no_caption() {
    let html = r#"
        <html><body>
            <article>
                <p>Content here.</p>
                <img src="https://example.com/standalone.jpg" alt="Standalone">
            </article>
        </body></html>
    "#;

    let options = Options {
        include_images: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    assert_eq!(result.images.len(), 1);
    let img = &result.images[0];
    assert_eq!(img.caption, None);
}

/// Test that empty figcaption results in None
#[test]
fn empty_figcaption_is_none() {
    let html = r#"
        <html><body>
            <article>
                <figure>
                    <img src="https://example.com/image.jpg">
                    <figcaption>   </figcaption>
                </figure>
            </article>
        </body></html>
    "#;

    let options = Options {
        include_images: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    assert_eq!(result.images.len(), 1);
    let img = &result.images[0];
    assert_eq!(img.caption, None);
}

/// Test figure without figcaption
#[test]
fn figure_without_figcaption() {
    let html = r#"
        <html><body>
            <article>
                <figure>
                    <img src="https://example.com/image.jpg" alt="Alt text">
                </figure>
            </article>
        </body></html>
    "#;

    let options = Options {
        include_images: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    assert_eq!(result.images.len(), 1);
    let img = &result.images[0];
    assert_eq!(img.alt, Some("Alt text".to_string()));
    assert_eq!(img.caption, None);
}

// ============================================================================
// STORY 4: HERO IMAGE DETECTION TESTS
// ============================================================================

/// Test that first image is marked as hero when no og:image
#[test]
fn first_image_is_hero_without_og_image() {
    let html = r#"
        <html><body>
            <article>
                <img src="https://example.com/first.jpg">
                <img src="https://example.com/second.jpg">
                <img src="https://example.com/third.jpg">
            </article>
        </body></html>
    "#;

    let options = Options {
        include_images: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    assert_eq!(result.images.len(), 3);
    assert!(result.images[0].is_hero, "First image should be hero");
    assert!(!result.images[1].is_hero, "Second image should not be hero");
    assert!(!result.images[2].is_hero, "Third image should not be hero");
}

/// Test that og:image match is detected as hero (exact URL match)
#[test]
fn og_image_exact_match_is_hero() {
    let html = r#"
        <html>
        <head>
            <meta property="og:image" content="https://example.com/hero.jpg">
        </head>
        <body>
            <article>
                <img src="https://example.com/first.jpg">
                <img src="https://example.com/hero.jpg">
                <img src="https://example.com/third.jpg">
            </article>
        </body></html>
    "#;

    let options = Options {
        include_images: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    assert_eq!(result.images.len(), 3);
    assert!(!result.images[0].is_hero, "First image should not be hero");
    assert!(result.images[1].is_hero, "Second image (og:image match) should be hero");
    assert!(!result.images[2].is_hero, "Third image should not be hero");
}

/// Test that og:image match by filename works for CDN variations
#[test]
fn og_image_filename_match_is_hero() {
    let html = r#"
        <html>
        <head>
            <meta property="og:image" content="https://cdn.example.com/uploads/hero-image.jpg">
        </head>
        <body>
            <article>
                <img src="https://example.com/first.jpg">
                <img src="https://images.example.com/content/hero-image.jpg">
                <img src="https://example.com/third.jpg">
            </article>
        </body></html>
    "#;

    let options = Options {
        include_images: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    assert_eq!(result.images.len(), 3);
    assert!(!result.images[0].is_hero, "First image should not be hero");
    assert!(result.images[1].is_hero, "Second image (filename match) should be hero");
    assert!(!result.images[2].is_hero, "Third image should not be hero");
}

/// Test that filename extraction works correctly for images
#[test]
fn image_filename_extracted_correctly() {
    let html = r#"
        <html><body>
            <article>
                <img src="https://example.com/path/to/my-photo.jpg?v=123">
            </article>
        </body></html>
    "#;

    let options = Options {
        include_images: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    assert_eq!(result.images.len(), 1);
    assert_eq!(result.images[0].filename, "my-photo.jpg");
}

/// Test combined figcaption and hero detection
#[test]
fn figcaption_and_hero_combined() {
    let html = r#"
        <html>
        <head>
            <meta property="og:image" content="https://cdn.example.com/featured.jpg">
        </head>
        <body>
            <article>
                <figure>
                    <img src="https://example.com/featured.jpg" alt="Featured">
                    <figcaption>The featured image</figcaption>
                </figure>
                <figure>
                    <img src="https://example.com/secondary.jpg" alt="Secondary">
                    <figcaption>A secondary image</figcaption>
                </figure>
            </article>
        </body></html>
    "#;

    let options = Options {
        include_images: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    assert_eq!(result.images.len(), 2);

    // First image should be hero (filename matches og:image)
    assert!(result.images[0].is_hero);
    assert_eq!(result.images[0].caption, Some("The featured image".to_string()));

    // Second image should not be hero
    assert!(!result.images[1].is_hero);
    assert_eq!(result.images[1].caption, Some("A secondary image".to_string()));
}

// ============================================================================
// STORY 5: FULL PIPELINE INTEGRATION TESTS
// ============================================================================

/// Comprehensive integration test for the complete image extraction pipeline.
/// Tests all ImageData fields: src, filename, alt, caption, is_hero
#[test]
fn full_image_pipeline_integration() {
    let html = r#"
        <html>
        <head>
            <title>Test Article</title>
            <meta property="og:image" content="https://cdn.example.com/uploads/hero-photo.jpg?v=2">
        </head>
        <body>
            <article>
                <h1>Article Title</h1>
                <p>Introduction paragraph with content.</p>

                <!-- Figure with caption - should match og:image by filename -->
                <figure>
                    <img src="https://images.example.com/hero-photo.jpg" alt="Hero image alt text">
                    <figcaption>This is the hero image caption.</figcaption>
                </figure>

                <p>More article text here.</p>

                <!-- Figure without caption -->
                <figure>
                    <img src="https://example.com/path/secondary.png?size=large" alt="Secondary alt">
                </figure>

                <!-- Standalone image (no figure) -->
                <img src="https://example.com/standalone.gif" alt="Standalone image">

                <!-- Lazy-loaded image -->
                <img data-src="https://example.com/lazy-loaded.webp" alt="Lazy loaded">

                <!-- Duplicate URL - should be deduplicated -->
                <img src="https://images.example.com/hero-photo.jpg" alt="Duplicate">

                <p>Conclusion paragraph.</p>
            </article>
        </body>
        </html>
    "#;

    let options = Options {
        include_images: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    // Should have 4 unique images (duplicate filtered)
    assert_eq!(result.images.len(), 4, "Expected 4 unique images");

    // Image 1: Hero image from figure with caption
    let hero = &result.images[0];
    assert_eq!(hero.src, "https://images.example.com/hero-photo.jpg");
    assert_eq!(hero.filename, "hero-photo.jpg");
    assert_eq!(hero.alt, Some("Hero image alt text".to_string()));
    assert_eq!(hero.caption, Some("This is the hero image caption.".to_string()));
    assert!(hero.is_hero, "First image should be hero (matches og:image filename)");

    // Image 2: Secondary from figure without caption
    let secondary = &result.images[1];
    assert_eq!(secondary.src, "https://example.com/path/secondary.png?size=large");
    assert_eq!(secondary.filename, "secondary.png");
    assert_eq!(secondary.alt, Some("Secondary alt".to_string()));
    assert_eq!(secondary.caption, None, "Figure without figcaption should have no caption");
    assert!(!secondary.is_hero);

    // Image 3: Standalone image
    let standalone = &result.images[2];
    assert_eq!(standalone.src, "https://example.com/standalone.gif");
    assert_eq!(standalone.filename, "standalone.gif");
    assert_eq!(standalone.alt, Some("Standalone image".to_string()));
    assert_eq!(standalone.caption, None, "Standalone image should have no caption");
    assert!(!standalone.is_hero);

    // Image 4: Lazy-loaded image
    let lazy = &result.images[3];
    assert_eq!(lazy.src, "https://example.com/lazy-loaded.webp");
    assert_eq!(lazy.filename, "lazy-loaded.webp");
    assert_eq!(lazy.alt, Some("Lazy loaded".to_string()));
    assert!(!lazy.is_hero);
}

/// Test that duplicate images are properly deduplicated
#[test]
fn pipeline_deduplicates_images() {
    let html = r#"
        <html><body>
            <article>
                <img src="https://example.com/same.jpg" alt="First occurrence">
                <img src="https://example.com/different.jpg" alt="Different">
                <img src="https://example.com/same.jpg" alt="Second occurrence">
                <img src="https://example.com/same.jpg" alt="Third occurrence">
            </article>
        </body></html>
    "#;

    let options = Options {
        include_images: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    // Should only have 2 unique images
    assert_eq!(result.images.len(), 2);
    assert!(images_contain_src(&result.images, "https://example.com/same.jpg"));
    assert!(images_contain_src(&result.images, "https://example.com/different.jpg"));

    // First occurrence's alt text should be preserved
    let same_img = result.images.iter().find(|i| i.src == "https://example.com/same.jpg").unwrap();
    assert_eq!(same_img.alt, Some("First occurrence".to_string()));
}

/// Test that images inside figures are not duplicated with standalone processing
#[test]
fn figure_images_not_duplicated() {
    let html = r#"
        <html><body>
            <article>
                <figure>
                    <img src="https://example.com/in-figure.jpg" alt="In figure">
                    <figcaption>Figure caption</figcaption>
                </figure>
            </article>
        </body></html>
    "#;

    let options = Options {
        include_images: true,
        ..Options::default()
    };

    let result = extract_with_options(html, &options).expect("extraction failed");

    // Should have exactly 1 image, not duplicated
    assert_eq!(result.images.len(), 1);
    assert_eq!(result.images[0].src, "https://example.com/in-figure.jpg");
    assert_eq!(result.images[0].caption, Some("Figure caption".to_string()));
}
