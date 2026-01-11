//! Result types for extraction output.
//!
//! This module defines the structured output from content extraction,
//! including the main content and associated metadata.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Structured image data extracted from content.
///
/// Contains comprehensive metadata about each image found in the document,
/// matching the web-content-extraction-benchmark v2 schema.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageData {
    /// Full image URL (from `src` or `data-src` attribute).
    pub src: String,

    /// Filename extracted from URL (without query params/fragments).
    pub filename: String,

    /// Alt text from `<img alt="...">` attribute.
    pub alt: Option<String>,

    /// Caption text from associated `<figcaption>` element.
    pub caption: Option<String>,

    /// Whether this is the main/hero image for the page.
    pub is_hero: bool,
}

/// Result of content extraction from an HTML document.
///
/// Contains the extracted content in both text and HTML formats,
/// along with metadata about the document.
#[derive(Debug, Clone, Default)]
pub struct ExtractResult {
    /// Main content as plain text.
    pub content_text: String,

    /// Main content as HTML (preserves structure).
    pub content_html: Option<String>,

    // === EPIC-02: Markdown Output ===
    /// Main content as GitHub Flavored Markdown (if `output_markdown` enabled).
    ///
    /// Preserves document structure: headings, paragraphs, lists, tables,
    /// bold/italic, links, code blocks, and images.
    pub content_markdown: Option<String>,

    /// Comments section as plain text (if extraction enabled).
    pub comments_text: Option<String>,

    /// Comments section as HTML (if extraction enabled).
    pub comments_html: Option<String>,

    /// Images found in content with metadata (if `include_images` enabled).
    pub images: Vec<ImageData>,

    /// Extracted metadata about the document.
    pub metadata: Metadata,

    /// Warnings encountered during extraction.
    ///
    /// Non-fatal issues that occurred during extraction, such as:
    /// - Content extraction failed (metadata-only result)
    /// - Individual metadata fields failed to extract
    /// - Recoverable parsing errors
    pub warnings: Vec<String>,
}

/// Metadata extracted from an HTML document.
///
/// All fields are optional as metadata may not be present in all documents.
/// Fields match go-trafilatura's Metadata struct for compatibility.
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    /// Page title.
    pub title: Option<String>,

    /// Author name(s).
    pub author: Option<String>,

    /// Original URL of the document.
    pub url: Option<String>,

    /// Hostname extracted from URL.
    pub hostname: Option<String>,

    /// Page description (meta description).
    pub description: Option<String>,

    /// Site name (e.g., "New York Times").
    pub sitename: Option<String>,

    /// Publication or modification date.
    pub date: Option<DateTime<Utc>>,

    /// Content categories.
    pub categories: Vec<String>,

    /// Content tags.
    pub tags: Vec<String>,

    /// Document identifier.
    pub id: Option<String>,

    /// Content fingerprint/hash.
    pub fingerprint: Option<String>,

    /// License information.
    pub license: Option<String>,

    /// Detected content language (ISO 639-1 code).
    pub language: Option<String>,

    /// Main image URL.
    pub image: Option<String>,

    /// Page type classification (article, product, etc.).
    pub page_type: Option<String>,
}
