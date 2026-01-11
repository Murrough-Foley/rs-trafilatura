//! Element Tree (etree) Utilities
//!
//! Provides higher-level tree manipulation utilities with support for the text/tail model.
//! This model is critical for preserving whitespace and text flow during content extraction.
//!
//! ## Text vs Tail
//!
//! In lxml-style processing (used by go-trafilatura), elements have:
//! - **Text**: Text content BEFORE the first child element
//! - **Tail**: Text content AFTER the element's closing tag
//!
//! ```html
//! <div>
//!   TEXT HERE          <!-- This is div's "text" -->
//!   <span>inner</span>
//!   TAIL HERE          <!-- This is span's "tail" -->
//! </div>
//! ```
//!
//! This module re-exports functions from the `html-cleaning` crate.

// Re-export all tree functions from html-cleaning for backward compatibility
pub use html_cleaning::tree::{
    append, element, extend, is_void_element, iter, iter_descendants, iter_text, remove,
    set_tail, set_text, strip, strip_elements, strip_tags, sub_element, tail, tail_nodes, text,
    Document,
};
