//! Character encoding detection and transcoding.
//!
//! This module handles various character encodings commonly found in web pages,
//! detecting the charset from HTML meta tags and converting to UTF-8.

use encoding_rs::{Encoding, UTF_8};
use regex::Regex;
use std::sync::LazyLock;

// Module-level regex patterns for charset detection
// These are compiled once at first use and reused throughout the program lifetime

/// Match `<meta charset="...">` tag
#[allow(clippy::expect_used)]
static CHARSET_META_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)<meta[^>]+charset\s*=\s*["']?([^"'\s>]+)"#).expect("valid regex")
});

/// Match `<meta http-equiv="Content-Type" content="...; charset=...">` tag
#[allow(clippy::expect_used)]
static CONTENT_TYPE_CHARSET_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)<meta[^>]+http-equiv\s*=\s*["']?content-type["']?[^>]+content\s*=\s*["']?[^"'>]*;\s*charset\s*=\s*([^"'\s>]+)"#).expect("valid regex")
});

/// Detect character encoding from HTML bytes.
///
/// Looks for charset declarations in the following order:
/// 1. `<meta charset="...">`
/// 2. `<meta http-equiv="Content-Type" content="...; charset=...">`
/// 3. Defaults to UTF-8 if no declaration found
///
/// Only examines the first 1024 bytes for performance.
#[must_use]
pub fn detect_encoding(html: &[u8]) -> &'static Encoding {
    // Only look at first 1024 bytes for performance
    let head = &html[..html.len().min(1024)];

    // Convert to string with lossy conversion to search for meta tags
    let head_str = String::from_utf8_lossy(head);

    // Try <meta charset="...">
    if let Some(charset) = extract_charset(&head_str) {
        if let Some(encoding) = Encoding::for_label(charset.as_bytes()) {
            return encoding;
        }
    }

    // Try <meta http-equiv="Content-Type" content="...; charset=...">
    if let Some(charset) = extract_content_type_charset(&head_str) {
        if let Some(encoding) = Encoding::for_label(charset.as_bytes()) {
            return encoding;
        }
    }

    // Default to UTF-8 (standard web default)
    UTF_8
}

/// Extract charset from `<meta charset="...">` tag.
fn extract_charset(html: &str) -> Option<String> {
    CHARSET_META_RE
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// Extract charset from `<meta http-equiv="Content-Type" content="...; charset=...">` tag.
fn extract_content_type_charset(html: &str) -> Option<String> {
    CONTENT_TYPE_CHARSET_RE
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// Transcode HTML bytes to UTF-8 string.
///
/// Detects the encoding and converts to UTF-8, using lossy conversion
/// to handle invalid characters gracefully (replacing them with �).
///
/// # Examples
///
/// ```
/// use rs_trafilatura::encoding::transcode_to_utf8;
///
/// let html = b"<html><body>Hello, World!</body></html>";
/// let utf8_str = transcode_to_utf8(html);
/// assert!(utf8_str.contains("Hello, World!"));
/// ```
#[must_use]
pub fn transcode_to_utf8(html: &[u8]) -> String {
    let encoding = detect_encoding(html);

    if encoding == UTF_8 {
        // Fast path for UTF-8: just do lossy conversion
        return String::from_utf8_lossy(html).into_owned();
    }

    // Decode from detected encoding to UTF-8
    // Use lossy conversion to handle invalid characters
    let (decoded, _encoding_used, _had_errors) = encoding.decode(html);

    // Note: We don't panic on errors, just return the decoded string
    // with invalid characters replaced by the Unicode replacement character (�)
    decoded.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_utf8_from_meta_charset() {
        let html = br#"<html><head><meta charset="utf-8"></head><body>Test</body></html>"#;
        let encoding = detect_encoding(html);
        assert_eq!(encoding, UTF_8);
    }

    #[test]
    fn detect_iso88591_from_meta_charset() {
        let html = br#"<html><head><meta charset="ISO-8859-1"></head><body>Test</body></html>"#;
        let encoding = detect_encoding(html);
        // encoding_rs maps ISO-8859-1 to windows-1252 per WHATWG spec
        // (they are functionally equivalent for web content)
        assert_eq!(encoding.name(), "windows-1252");
    }

    #[test]
    fn detect_windows1252_from_meta_charset() {
        let html = br#"<html><head><meta charset="windows-1252"></head><body>Test</body></html>"#;
        let encoding = detect_encoding(html);
        assert_eq!(encoding.name(), "windows-1252");
    }

    #[test]
    fn detect_charset_from_content_type() {
        let html = br#"<html><head><meta http-equiv="Content-Type" content="text/html; charset=ISO-8859-1"></head><body>Test</body></html>"#;
        let encoding = detect_encoding(html);
        // encoding_rs maps ISO-8859-1 to windows-1252 per WHATWG spec
        assert_eq!(encoding.name(), "windows-1252");
    }

    #[test]
    fn default_to_utf8_when_no_charset() {
        let html = b"<html><body>Test</body></html>";
        let encoding = detect_encoding(html);
        assert_eq!(encoding, UTF_8);
    }

    #[test]
    fn transcode_utf8_passthrough() {
        let html = b"<html><body>Hello, World!</body></html>";
        let result = transcode_to_utf8(html);
        assert_eq!(result, "<html><body>Hello, World!</body></html>");
    }

    #[test]
    fn transcode_iso88591_to_utf8() {
        // ISO-8859-1 encoded HTML with special character (é = 0xE9)
        let html = b"<html><head><meta charset=\"ISO-8859-1\"></head><body>Caf\xE9</body></html>";
        let result = transcode_to_utf8(html);
        assert!(result.contains("Café"));
    }

    #[test]
    fn transcode_windows1252_to_utf8() {
        // Windows-1252 encoded HTML with smart quote (" = 0x93)
        let html = b"<html><head><meta charset=\"windows-1252\"></head><body>\x93Hello\x94</body></html>";
        let result = transcode_to_utf8(html);
        // Windows-1252 0x93/0x94 are left/right double quotes
        assert!(result.contains("\u{201C}Hello\u{201D}"));
    }

    #[test]
    fn handle_invalid_encoding_gracefully() {
        // Invalid UTF-8 sequence
        let html = b"<html><body>Test \xFF\xFE Invalid</body></html>";
        let result = transcode_to_utf8(html);
        // Should contain replacement characters but not panic
        assert!(result.contains("Test"));
        assert!(result.contains("Invalid"));
    }

    #[test]
    fn extract_charset_case_insensitive() {
        let html = "<HTML><HEAD><META CHARSET=\"UTF-8\"></HEAD></HTML>";
        let charset = extract_charset(html);
        assert_eq!(charset, Some("UTF-8".to_string()));
    }

    #[test]
    fn extract_charset_with_quotes() {
        let html = r#"<meta charset="utf-8">"#;
        let charset = extract_charset(html);
        assert_eq!(charset, Some("utf-8".to_string()));
    }

    #[test]
    fn extract_charset_without_quotes() {
        let html = "<meta charset=utf-8>";
        let charset = extract_charset(html);
        assert_eq!(charset, Some("utf-8".to_string()));
    }

    #[test]
    fn extract_content_type_charset_standard() {
        let html = r#"<meta http-equiv="Content-Type" content="text/html; charset=ISO-8859-1">"#;
        let charset = extract_content_type_charset(html);
        assert_eq!(charset, Some("ISO-8859-1".to_string()));
    }

    #[test]
    fn extract_content_type_charset_case_insensitive() {
        let html = r#"<META HTTP-EQUIV="content-type" CONTENT="text/html; CHARSET=UTF-8">"#;
        let charset = extract_content_type_charset(html);
        assert_eq!(charset, Some("UTF-8".to_string()));
    }
}
