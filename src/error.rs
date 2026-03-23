//! Error types for rs-trafilatura.
//!
//! This module defines the error types returned by extraction operations.

/// Error type for extraction operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// HTML parsing failed.
    #[error("HTML parsing failed: {0}")]
    ParseError(String),

    /// Character encoding detection or conversion failed.
    #[error("Encoding detection failed: {0}")]
    EncodingError(String),

    /// No extractable content was found in the document.
    #[error("No extractable content found")]
    NoContent,

    /// General extraction failure.
    #[error("Extraction failed: {0}")]
    ExtractionError(String),
}

/// Result type alias for extraction operations.
pub type Result<T> = std::result::Result<T, Error>;
