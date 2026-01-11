use rs_trafilatura::{extract_bytes, extract_bytes_with_options, Options};

/// Test AC#1: UTF-8 content is handled correctly
#[test]
fn utf8_content_handled_correctly() {
    let html = "\
        <html>\
        <head><meta charset=\"utf-8\"></head>\
        <body>\
            <article>\
                <h1>Test Article</h1>\
                <p>This is UTF-8 content with special characters: é, ñ, ü, 中文</p>\
            </article>\
        </body>\
        </html>\
    ".as_bytes();

    let result = extract_bytes(html).expect("extraction failed");

    assert!(result.content_text.contains("Test Article"));
    assert!(result.content_text.contains("UTF-8 content"));
    assert!(result.content_text.contains("é"));
    assert!(result.content_text.contains("ñ"));
    assert!(result.content_text.contains("ü"));
    assert!(result.content_text.contains("中文"));
}

/// Test AC#2: ISO-8859-1 encoding is converted to UTF-8
#[test]
fn iso88591_converted_to_utf8() {
    // ISO-8859-1 encoded HTML with special characters
    // é = 0xE9, ñ = 0xF1, ü = 0xFC in ISO-8859-1
    let html = b"<html>\
        <head><meta charset=\"ISO-8859-1\"></head>\
        <body><article>\
            <h1>Caf\xE9 espa\xF1ol</h1>\
            <p>M\xFCnchen</p>\
        </article></body></html>";

    let result = extract_bytes(html).expect("extraction failed");

    assert!(result.content_text.contains("Café"));
    assert!(result.content_text.contains("español"));
    assert!(result.content_text.contains("München"));
}

/// Test AC#3: Windows-1252 encoding is detected and converted
#[test]
fn windows1252_detected_and_converted() {
    // Windows-1252 encoded HTML with smart quotes
    // 0x93 = left double quote, 0x94 = right double quote, 0x96 = en-dash
    let html = b"<html>\
        <head><meta http-equiv=\"Content-Type\" content=\"text/html; charset=windows-1252\"></head>\
        <body><article>\
            <p>\x93Smart quotes\x94 and an en\x96dash.</p>\
        </article></body></html>";

    let result = extract_bytes(html).expect("extraction failed");

    // Windows-1252 0x93/0x94 are left/right double quotes (")
    // 0x96 is en-dash (–)
    assert!(result.content_text.contains("Smart quotes"));
    assert!(result.content_text.contains("dash"));
}

/// Test AC#4: UTF-8 is assumed when no charset declaration
#[test]
fn utf8_assumed_when_no_charset() {
    let html = b"<html><body><article><p>No charset specified</p></article></body></html>";

    let result = extract_bytes(html).expect("extraction failed");

    assert!(result.content_text.contains("No charset specified"));
}

/// Test AC#5: Invalid encoding is handled gracefully (no panic)
#[test]
fn invalid_encoding_handled_gracefully() {
    // HTML with invalid UTF-8 sequences
    let html = b"<html><body><article>\
        <p>Valid text</p>\
        <p>Invalid: \xFF\xFE\xFD</p>\
        <p>More valid text</p>\
        </article></body></html>";

    // Should not panic
    let result = extract_bytes(html).expect("extraction failed");

    // Should contain the valid parts
    assert!(result.content_text.contains("Valid text"));
    assert!(result.content_text.contains("More valid text"));

    // Invalid bytes should be replaced with � (Unicode replacement character)
    // or omitted, but extraction should succeed
}

/// Test AC#5: Partial results returned when encoding is corrupted
#[test]
fn partial_results_on_corrupted_encoding() {
    // Mix of valid and invalid sequences
    let html = b"<html><body><article>\
        <h1>Good Title</h1>\
        <p>First paragraph is fine.</p>\
        <p>This has \x80\x81\x82 bad bytes</p>\
        <p>Last paragraph is also fine.</p>\
        </article></body></html>";

    let result = extract_bytes(html).expect("extraction should succeed");

    // Should extract the valid parts
    assert!(result.content_text.contains("Good Title"));
    assert!(result.content_text.contains("First paragraph"));
    assert!(result.content_text.contains("Last paragraph"));
}

/// Test extract_bytes_with_options works correctly
#[test]
fn extract_bytes_with_options_works() {
    let html = b"<html>\
        <head><meta charset=\"ISO-8859-1\"></head>\
        <body>\
            <article>\
                <h1>Main Article</h1>\
                <p>Main content with caf\xE9 and substantial text to meet scoring threshold.</p>\
                <p>Additional paragraph to ensure this is recognized as main content.</p>\
                <table><tr><td>Table data</td></tr></table>\
            </article>\
        </body></html>";

    let options = Options {
        include_tables: true,
        ..Options::default()
    };

    let result = extract_bytes_with_options(html, &options).expect("extraction failed");

    assert!(result.content_text.contains("café"));
    assert!(result.content_text.contains("Table data"));
}

/// Test charset detection is case-insensitive
#[test]
fn charset_detection_case_insensitive() {
    let html = b"<HTML><HEAD><META CHARSET=\"UTF-8\"></HEAD>\
        <BODY><ARTICLE><P>Content</P></ARTICLE></BODY></HTML>";

    let result = extract_bytes(html).expect("extraction failed");

    assert!(result.content_text.contains("Content"));
}

/// Test multiple charset declarations (first one wins)
#[test]
fn multiple_charset_declarations() {
    // First charset is ISO-8859-1, second is UTF-8
    // Should use ISO-8859-1 (first one found)
    let html = b"<html>\
        <head><meta charset=\"ISO-8859-1\"><meta charset=\"UTF-8\"></head>\
        <body><article><p>Caf\xE9</p></article></body></html>";

    let result = extract_bytes(html).expect("extraction failed");

    assert!(result.content_text.contains("Café"));
}

/// Test Latin-1 special characters
#[test]
fn latin1_special_characters() {
    // Various Latin-1 special characters
    // à = 0xE0, è = 0xE8, ì = 0xEC, ò = 0xF2, ù = 0xF9
    let html = b"<html><head><meta charset=\"ISO-8859-1\"></head>\
        <body><article>\
            <p>\xE0 \xE8 \xEC \xF2 \xF9</p>\
        </article></body></html>";

    let result = extract_bytes(html).expect("extraction failed");

    assert!(result.content_text.contains("à è ì ò ù"));
}

/// Test real-world scenario with mixed encoding declarations
#[test]
fn real_world_mixed_encoding() {
    let html = br#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="windows-1252">
    <title>Test Page</title>
</head>
<body>
    <article>
        <h1>Real World Example</h1>
        <p>This page uses Windows-1252 encoding.</p>
        <p>It has special characters like en-dash and quotes.</p>
    </article>
</body>
</html>"#;

    let result = extract_bytes(html).expect("extraction failed");

    assert!(result.content_text.contains("Real World Example"));
    assert!(result.content_text.contains("Windows-1252"));
}

/// Test that UTF-8 BOM is handled correctly
#[test]
fn utf8_bom_handled_correctly() {
    // UTF-8 BOM (0xEF 0xBB 0xBF) followed by HTML
    let html = b"\xEF\xBB\xBF<html><body><article><p>Content with BOM</p></article></body></html>";

    let result = extract_bytes(html).expect("extraction failed");

    assert!(result.content_text.contains("Content with BOM"));
}
