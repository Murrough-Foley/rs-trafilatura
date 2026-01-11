//! LRU Cache for Text Deduplication
//!
//! Simple LRU (Least Recently Used) cache implementation for tracking seen text fragments
//! during content extraction. Prevents duplicate content from being included in output.
//!
//! This module re-exports `LruCache` from the `html-cleaning` crate.

// Re-export LruCache from html-cleaning for backward compatibility
pub use html_cleaning::dedup::LruCache;
