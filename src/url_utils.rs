//! URL Utility Functions
//!
//! This module ports URL utilities from go-trafilatura's url.go.
//! It provides URL validation, resolution, and extraction utilities
//! needed for handling relative URLs in metadata and content.

use url::Url;

/// Check if a string is a valid absolute URL.
///
/// Go equivalent: `isAbsoluteURL(s)` (lines 15-35)
///
/// # Returns
/// * `(is_absolute, parsed_url)` - Whether URL is absolute and the parsed URL if valid
#[must_use]
pub fn is_absolute_url(s: &str) -> (bool, Option<Url>) {
    let s = s.trim();

    if s.is_empty() {
        return (false, None);
    }

    // Must start with http:// or https://
    if !s.starts_with("http://") && !s.starts_with("https://") {
        return (false, None);
    }

    match Url::parse(s) {
        Ok(url) => {
            // Verify it has a host
            if url.host().is_some() {
                (true, Some(url))
            } else {
                (false, None)
            }
        }
        Err(_) => (false, None),
    }
}

/// Convert a relative or absolute URL to absolute form.
///
/// Go equivalent: `createAbsoluteURL(url, base)` (lines 37-75)
///
/// # Arguments
/// * `url_str` - The URL to resolve (can be relative or absolute)
/// * `base` - The base URL for resolution
///
/// # Returns
/// * The absolute URL string, or the original if resolution fails
#[must_use]
pub fn create_absolute_url(url_str: &str, base: &Url) -> String {
    let url_str = url_str.trim();

    if url_str.is_empty() {
        return String::new();
    }

    // Preserve special URLs unchanged
    if url_str.starts_with("data:")
        || url_str.starts_with("javascript:")
        || url_str.starts_with("mailto:")
        || url_str.starts_with("tel:")
    {
        return url_str.to_string();
    }

    // If already absolute, return as-is
    let (is_abs, _) = is_absolute_url(url_str);
    if is_abs {
        return url_str.to_string();
    }

    // Resolve relative URL against base
    match base.join(url_str) {
        Ok(resolved) => resolved.to_string(),
        Err(_) => url_str.to_string(),
    }
}

/// Extract the hostname (domain) from a URL.
///
/// Go equivalent: `getDomainURL(url)` (lines 77-90)
///
/// # Returns
/// * The hostname, or empty string if invalid
#[must_use]
pub fn get_domain_url(url_str: &str) -> String {
    let (is_abs, parsed) = is_absolute_url(url_str);

    if !is_abs {
        return String::new();
    }

    parsed
        .and_then(|url| url.host_str().map(std::string::ToString::to_string))
        .unwrap_or_default()
}

/// Get the base URL (scheme + hostname) from a URL.
///
/// Go equivalent: `getBaseURL(url)` (lines 92-105)
///
/// # Returns
/// * The base URL in format `scheme://hostname`, or empty string if invalid
#[must_use]
pub fn get_base_url(url_str: &str) -> String {
    let (is_abs, parsed) = is_absolute_url(url_str);

    if !is_abs {
        return String::new();
    }

    if let Some(url) = parsed {
        if let Some(host) = url.host_str() {
            return format!("{}://{}", url.scheme(), host);
        }
    }

    String::new()
}

/// Validate a URL and convert to absolute if necessary.
///
/// Go equivalent: `validateURL(url, baseURL)` (lines 107-120)
///
/// # Arguments
/// * `url_str` - The URL to validate
/// * `base` - Optional base URL for relative resolution
///
/// # Returns
/// * `(resolved_url, is_valid)` - The resolved URL and whether it's valid
#[must_use]
pub fn validate_url(url_str: &str, base: Option<&Url>) -> (String, bool) {
    let url_str = url_str.trim();

    if url_str.is_empty() {
        return (String::new(), false);
    }

    // Check if already absolute
    let (is_abs, _) = is_absolute_url(url_str);
    if is_abs {
        return (url_str.to_string(), true);
    }

    // Try to resolve with base
    if let Some(base_url) = base {
        let resolved = create_absolute_url(url_str, base_url);
        let (is_valid, _) = is_absolute_url(&resolved);
        return (resolved, is_valid);
    }

    (url_str.to_string(), false)
}

/// Extract hostname from URL for metadata.
///
/// This is a convenience wrapper that handles URL parsing.
#[must_use]
pub fn extract_hostname(url_str: &str) -> Option<String> {
    let domain = get_domain_url(url_str);
    if domain.is_empty() {
        None
    } else {
        Some(domain)
    }
}

/// Parse a URL string into a Url object.
///
/// # Returns
/// * `Some(Url)` if valid absolute URL, `None` otherwise
#[must_use]
pub fn parse_url(url_str: &str) -> Option<Url> {
    let (is_abs, parsed) = is_absolute_url(url_str);
    if is_abs {
        parsed
    } else {
        None
    }
}

/// Normalize a URL by removing fragments and normalizing path.
#[must_use]
pub fn normalize_url(url_str: &str) -> String {
    let Some(mut url) = parse_url(url_str) else {
        return url_str.to_string();
    };

    // Remove fragment
    url.set_fragment(None);

    // Remove trailing slash from path (unless root)
    let path = url.path().to_string();
    if path.len() > 1 && path.ends_with('/') {
        url.set_path(&path[..path.len() - 1]);
    }

    url.to_string()
}

/// Check if two URLs point to the same page (ignoring fragments).
#[must_use]
pub fn urls_match(url1: &str, url2: &str) -> bool {
    let norm1 = normalize_url(url1);
    let norm2 = normalize_url(url2);
    norm1 == norm2
}

/// Extract filename from a URL, stripping query parameters and fragments.
///
/// # Arguments
/// * `url` - The URL to extract filename from (can be relative or absolute)
///
/// # Returns
/// * The filename portion of the URL path, or empty string if none found
///
/// # Examples
/// ```
/// use rs_trafilatura::url_utils::extract_filename;
///
/// assert_eq!(extract_filename("https://example.com/images/photo.jpg"), "photo.jpg");
/// assert_eq!(extract_filename("https://example.com/images/photo.jpg?v=123"), "photo.jpg");
/// assert_eq!(extract_filename("/path/to/image.png#section"), "image.png");
/// assert_eq!(extract_filename("https://example.com/"), "");
/// ```
#[must_use]
pub fn extract_filename(url: &str) -> String {
    let url = url.trim();

    if url.is_empty() {
        return String::new();
    }

    // Strip query parameters
    let without_query = url.split('?').next().unwrap_or(url);

    // Strip fragment identifiers
    let without_fragment = without_query.split('#').next().unwrap_or(without_query);

    // Get the last path segment
    let filename = without_fragment
        .split('/')
        .last()
        .unwrap_or("")
        .trim();

    // Don't return empty-looking filenames
    if filename.is_empty() || filename == "." || filename == ".." {
        return String::new();
    }

    filename.to_string()
}

/// Check if two URLs refer to the same image by comparing filenames.
///
/// Useful for matching og:image against content images when CDN URLs differ.
///
/// # Arguments
/// * `url1` - First URL to compare
/// * `url2` - Second URL to compare
///
/// # Returns
/// * `true` if filenames match (case-insensitive), `false` otherwise
#[must_use]
pub fn filenames_match(url1: &str, url2: &str) -> bool {
    let f1 = extract_filename(url1);
    let f2 = extract_filename(url2);

    if f1.is_empty() || f2.is_empty() {
        return false;
    }

    f1.eq_ignore_ascii_case(&f2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_absolute_url_valid() {
        let (is_abs, url) = is_absolute_url("https://example.com/path");
        assert!(is_abs);
        assert!(url.is_some());

        let (is_abs, url) = is_absolute_url("http://example.com");
        assert!(is_abs);
        assert!(url.is_some());
    }

    #[test]
    fn test_is_absolute_url_invalid() {
        let (is_abs, _) = is_absolute_url("/relative/path");
        assert!(!is_abs);

        let (is_abs, _) = is_absolute_url("example.com");
        assert!(!is_abs);

        let (is_abs, _) = is_absolute_url("");
        assert!(!is_abs);

        let (is_abs, _) = is_absolute_url("ftp://example.com");
        assert!(!is_abs); // Only http/https
    }

    #[test]
    fn test_is_absolute_url_with_whitespace() {
        let (is_abs, url) = is_absolute_url("  https://example.com/path  ");
        assert!(is_abs);
        assert!(url.is_some());
    }

    #[test]
    fn test_create_absolute_url_relative() {
        let base = Url::parse("https://example.com/articles/").ok();
        let base = base.as_ref().map_or_else(
            || panic!("Failed to parse base URL"),
            |b| b,
        );

        assert_eq!(
            create_absolute_url("page.html", base),
            "https://example.com/articles/page.html"
        );

        assert_eq!(
            create_absolute_url("/root/page.html", base),
            "https://example.com/root/page.html"
        );

        assert_eq!(
            create_absolute_url("../other/page.html", base),
            "https://example.com/other/page.html"
        );
    }

    #[test]
    fn test_create_absolute_url_already_absolute() {
        let base = Url::parse("https://example.com/").ok();
        let base = base.as_ref().map_or_else(
            || panic!("Failed to parse base URL"),
            |b| b,
        );

        assert_eq!(
            create_absolute_url("https://other.com/page", base),
            "https://other.com/page"
        );
    }

    #[test]
    fn test_create_absolute_url_special() {
        let base = Url::parse("https://example.com/").ok();
        let base = base.as_ref().map_or_else(
            || panic!("Failed to parse base URL"),
            |b| b,
        );

        assert_eq!(
            create_absolute_url("data:image/png;base64,abc", base),
            "data:image/png;base64,abc"
        );

        assert_eq!(
            create_absolute_url("javascript:void(0)", base),
            "javascript:void(0)"
        );

        assert_eq!(
            create_absolute_url("mailto:test@example.com", base),
            "mailto:test@example.com"
        );

        assert_eq!(
            create_absolute_url("tel:+1234567890", base),
            "tel:+1234567890"
        );
    }

    #[test]
    fn test_create_absolute_url_empty() {
        let base = Url::parse("https://example.com/").ok();
        let base = base.as_ref().map_or_else(
            || panic!("Failed to parse base URL"),
            |b| b,
        );

        assert_eq!(create_absolute_url("", base), "");
        assert_eq!(create_absolute_url("  ", base), "");
    }

    #[test]
    fn test_get_domain_url() {
        assert_eq!(get_domain_url("https://example.com/path"), "example.com");
        assert_eq!(get_domain_url("https://sub.example.com/"), "sub.example.com");
        assert_eq!(get_domain_url("/relative"), "");
        assert_eq!(get_domain_url(""), "");
    }

    #[test]
    fn test_get_base_url() {
        assert_eq!(
            get_base_url("https://example.com/path/to/page"),
            "https://example.com"
        );
        assert_eq!(
            get_base_url("http://example.com:8080/path"),
            "http://example.com" // Port not included (just host)
        );
        assert_eq!(get_base_url("/relative"), "");
        assert_eq!(get_base_url(""), "");
    }

    #[test]
    fn test_validate_url_absolute() {
        let (url, valid) = validate_url("https://example.com/page", None);
        assert!(valid);
        assert_eq!(url, "https://example.com/page");
    }

    #[test]
    fn test_validate_url_relative_with_base() {
        let base = Url::parse("https://example.com/articles/").ok();
        let (url, valid) = validate_url("/other/page", base.as_ref());
        assert!(valid);
        assert_eq!(url, "https://example.com/other/page");
    }

    #[test]
    fn test_validate_url_relative_no_base() {
        let (url, valid) = validate_url("/relative/path", None);
        assert!(!valid);
        assert_eq!(url, "/relative/path");
    }

    #[test]
    fn test_validate_url_empty() {
        let (url, valid) = validate_url("", None);
        assert!(!valid);
        assert_eq!(url, "");
    }

    #[test]
    fn test_extract_hostname() {
        assert_eq!(
            extract_hostname("https://www.example.com/page"),
            Some("www.example.com".to_string())
        );
        assert_eq!(extract_hostname("/relative"), None);
        assert_eq!(extract_hostname(""), None);
    }

    #[test]
    fn test_parse_url() {
        let url = parse_url("https://example.com/page");
        assert!(url.is_some());

        let url = parse_url("/relative");
        assert!(url.is_none());
    }

    #[test]
    fn test_normalize_url() {
        assert_eq!(
            normalize_url("https://example.com/page#section"),
            "https://example.com/page"
        );
        assert_eq!(
            normalize_url("https://example.com/path/"),
            "https://example.com/path"
        );
        assert_eq!(
            normalize_url("https://example.com/"),
            "https://example.com/"  // Root path preserved
        );
    }

    #[test]
    fn test_normalize_url_invalid() {
        // Invalid URLs returned as-is
        assert_eq!(normalize_url("/relative"), "/relative");
        assert_eq!(normalize_url(""), "");
    }

    #[test]
    fn test_urls_match() {
        assert!(urls_match(
            "https://example.com/page#section1",
            "https://example.com/page#section2"
        ));
        assert!(!urls_match(
            "https://example.com/page1",
            "https://example.com/page2"
        ));
    }

    #[test]
    fn test_urls_match_trailing_slash() {
        assert!(urls_match(
            "https://example.com/page/",
            "https://example.com/page"
        ));
    }

    #[test]
    fn test_extract_filename_basic() {
        assert_eq!(
            extract_filename("https://example.com/images/photo.jpg"),
            "photo.jpg"
        );
        assert_eq!(
            extract_filename("https://example.com/path/to/file.png"),
            "file.png"
        );
        assert_eq!(
            extract_filename("/relative/path/image.gif"),
            "image.gif"
        );
    }

    #[test]
    fn test_extract_filename_query_params() {
        assert_eq!(
            extract_filename("https://example.com/image.jpg?v=123"),
            "image.jpg"
        );
        assert_eq!(
            extract_filename("https://cdn.example.com/photo.webp?width=800&height=600"),
            "photo.webp"
        );
        assert_eq!(
            extract_filename("/image.png?timestamp=12345"),
            "image.png"
        );
    }

    #[test]
    fn test_extract_filename_fragment() {
        assert_eq!(
            extract_filename("https://example.com/image.jpg#section"),
            "image.jpg"
        );
        assert_eq!(
            extract_filename("/path/file.svg#icon"),
            "file.svg"
        );
    }

    #[test]
    fn test_extract_filename_query_and_fragment() {
        assert_eq!(
            extract_filename("https://example.com/img.jpg?v=1#top"),
            "img.jpg"
        );
    }

    #[test]
    fn test_extract_filename_edge_cases() {
        // Empty or whitespace
        assert_eq!(extract_filename(""), "");
        assert_eq!(extract_filename("   "), "");

        // Root path - no filename
        assert_eq!(extract_filename("https://example.com/"), "");
        assert_eq!(extract_filename("/"), "");

        // Just domain
        assert_eq!(extract_filename("https://example.com"), "example.com");

        // Trailing slash
        assert_eq!(extract_filename("https://example.com/path/"), "");

        // Dot segments
        assert_eq!(extract_filename("https://example.com/."), "");
        assert_eq!(extract_filename("https://example.com/.."), "");
    }

    #[test]
    fn test_extract_filename_special_chars() {
        assert_eq!(
            extract_filename("https://example.com/my%20image.jpg"),
            "my%20image.jpg"
        );
        assert_eq!(
            extract_filename("https://example.com/image-name_2024.jpg"),
            "image-name_2024.jpg"
        );
    }

    #[test]
    fn test_filenames_match_basic() {
        assert!(filenames_match(
            "https://example.com/images/photo.jpg",
            "https://cdn.other.com/uploads/photo.jpg"
        ));
        assert!(!filenames_match(
            "https://example.com/photo1.jpg",
            "https://example.com/photo2.jpg"
        ));
    }

    #[test]
    fn test_filenames_match_case_insensitive() {
        assert!(filenames_match(
            "https://example.com/Photo.JPG",
            "https://cdn.com/photo.jpg"
        ));
        assert!(filenames_match(
            "https://example.com/IMAGE.PNG",
            "https://other.com/image.png"
        ));
    }

    #[test]
    fn test_filenames_match_empty() {
        assert!(!filenames_match("", "https://example.com/image.jpg"));
        assert!(!filenames_match("https://example.com/", "https://example.com/"));
        assert!(!filenames_match("", ""));
    }
}
