//! Main content extraction module.
//!
//! This module contains the extraction pipeline ported from go-trafilatura.
//!
//! # Module Structure
//!
//! - `tags`: Tag constants, catalogs, and helper functions
//! - `state`: Extraction state tracking (processed nodes, potential tags)
//! - `handlers`: Element handlers for titles, formatting, images, and code blocks
//! - `pruning`: Section pruning and boilerplate removal
//! - `comments`: Comment extraction from web pages
//! - `fallback`: Fallback extraction (JSON-LD baseline, Readability)
//! - `pipeline`: Main extraction pipeline orchestration
//!
//! # Usage
//!
//! ```rust,ignore
//! use rs_trafilatura::extractor::{pipeline, ExtractionState, tags, handlers, pruning};
//!
//! // Simple extraction
//! let doc = dom::parse(html);
//! let (result_doc, text) = pipeline::extract_content(&doc, &options);
//!
//! // Or use individual components:
//! let mut state = ExtractionState::new();
//! state.configure_from_options(&options);
//!
//! // Check if a tag is a potential content tag
//! if state.is_potential_tag("p") {
//!     // Process paragraph
//! }
//!
//! // Mark nodes as processed
//! state.mark_done(node_id);
//!
//! // Handle elements
//! if let Some(processed) = handlers::handle_titles(&element, &mut state, &opts) {
//!     // Use processed title
//! }
//!
//! // Prune unwanted sections
//! pruning::prune_unwanted_sections(&tree, state.potential_tags(), &opts);
//! ```

pub mod comments;
pub mod fallback;
pub mod handlers;
pub mod pipeline;
pub mod pruning;
pub mod state;
pub mod tags;

pub use comments::extract_comments;
pub use fallback::{baseline, candidate_is_usable, compare_external_extraction, extract_with_fallback};
pub use pipeline::extract_content;
pub use state::ExtractionState;
