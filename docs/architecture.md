---
stepsCompleted: [1, 2, 3, 4, 5, 6, 7, 8]
inputDocuments:
  - 'docs/prd.md'
  - 'docs/analysis/technical-research.md'
workflowType: 'architecture'
lastStep: 8
status: 'complete'
completedAt: '2025-12-15'
lastUpdated: '2025-12-19'
project_name: 'rs-trafilatura'
user_name: 'Foley'
date: '2025-12-15'
---

# Architecture Decision Document

_This document builds collaboratively through step-by-step discovery. Sections are appended as we work through each architectural decision together._

## Project Context Analysis

### Requirements Overview

**Functional Requirements (35 total):**

| Category | Count | Architectural Impact |
|----------|-------|---------------------|
| Content Extraction | 6 | Core extraction engine |
| Metadata Extraction | 11 | Extracted during content traversal (coupled) |
| Configuration | 7 | Options struct and extraction behavior control |
| API & Integration | 5 | Public API surface design |
| Error Handling | 4 | Error types and Result propagation |
| Quality Validation | 2 | Benchmark harness integration |

**Non-Functional Requirements (17 total):**

| Category | Key Constraints |
|----------|----------------|
| Accuracy | F-Score ≥ 0.90 on 983-page benchmark |
| Reliability | Zero panics on any input; graceful degradation |
| Performance | Phase 1: measure baseline only; Phase 2: optimize |
| Compatibility | Stable Rust 1.85+; Linux/macOS/Windows |
| Documentation | rustdoc, docs.rs, README examples |

### Scale & Complexity Assessment

- **Primary domain:** Rust library/crate (developer tool)
- **Complexity level:** Medium
- **Estimated modules:** 5 core modules (flat structure for Phase 1)
- **External dependencies:** 5 crates (scraper, regex, chrono, thiserror, encoding_rs)

### Phase 1 Module Strategy

**Start flat, let boundaries emerge:**

```
src/
├── lib.rs          # Public API: extract(), extract_with_options()
├── extract.rs      # Content + metadata extraction (coupled, single DOM pass)
├── options.rs      # Options struct
├── result.rs       # ExtractResult, Metadata structs
└── error.rs        # Error types with thiserror
```

Rationale: go-trafilatura's extraction and metadata are tightly coupled during the same DOM traversal. Artificial separation creates boundaries that don't match the algorithm. Refactor to more modules if natural boundaries emerge.

### Technical Constraints & Dependencies

**Hard Constraints:**
- Must use `scraper` crate (same foundation as dom_smoothie for Phase 2)
- Must achieve F-Score ≥ 0.90 before any release
- Must handle malformed HTML without panicking
- Must work across Linux, macOS, Windows

**Porting Constraints (Go → Rust):**
- Tree traversal: Go modifies DOM in-place; Rust requires collect-then-process
- Error handling: Go ignores errors with `_`; Rust must handle all `Result`
- String handling: Go strings are cheap; Rust needs ownership strategy
- Regex syntax: Minor differences require manual verification

### Cross-Cutting Concerns

| Concern | Affected Modules | Strategy |
|---------|-----------------|----------|
| **Error Handling** | All | Custom error enum with `thiserror`; all public APIs return `Result` |
| **DOM Traversal** | extract | Collect node handles first, process second |
| **String Ownership** | All | Phase 1: `String` everywhere; Phase 2: `Cow<str>` optimization |
| **Regex Patterns** | extract | Compile once with `std::sync::LazyLock` |
| **Encoding** | Input processing | `encoding_rs` for charset detection/conversion |
| **HTML Entities** | extract, output | `scraper` handles most; verify edge cases against benchmark |

### Benchmark Strategy

**Test Data:** Vendor 983 test files directly in repository
- Location: `tests/benchmark_suite/`
- Simple, always available, reproducible
- Can migrate to submodule later if repo size becomes concern

**F-Score Calculation:** Port scoring logic from benchmark harness
- Precision, recall, accuracy calculations in Rust
- Direct comparison with go-trafilatura results

### Phase 1 Performance Strategy

> **Measure, don't optimize.** Phase 1 establishes baseline metrics against benchmark suite. Performance optimization is explicitly Phase 2 scope. Do not optimize during Phase 1 code review.

---

## Go → Rust Porting Decisions

This section documents specific architectural decisions for translating Go patterns to idiomatic Rust.

### 1. Tree Traversal & DOM Manipulation

**Challenge:** Go uses mutable `*html.Node` pointers with in-place modification during iteration. Rust's ownership model forbids this.

**Go Pattern:**
```go
func processNode(node *html.Node) {
    for child := node.FirstChild; child != nil; child = child.NextSibling {
        if shouldRemove(child) {
            node.RemoveChild(child)  // Modify while iterating
        }
        processNode(child)
    }
}
```

**Decision: Collect-Then-Process Pattern**
```rust
fn process_node(node: &NodeRef) -> Vec<NodeId> {
    // PHASE 1: Collect what to remove
    let to_remove: Vec<NodeId> = node
        .children()
        .filter(|child| should_remove(child))
        .map(|child| child.id())
        .collect();

    // PHASE 2: Process remaining children recursively
    for child in node.children() {
        process_node(&child);
    }

    to_remove
}
```

**Open Question:** Investigate whether `scraper` supports node removal, or if we need to build a filtered tree.

### 2. Nil/Null Handling

**Challenge:** Go code uses implicit nil checks; Rust requires explicit `Option` handling.

**Decision: Use `Option<T>` with Fallback Chains**
```rust
// Metadata fields are Option<String> per PRD
metadata.author = extract_author_primary(&doc)
    .or_else(|| extract_author_fallback(&doc))
    .or_else(|| extract_author_meta(&doc));
```

**Rule:** All metadata fields return `Option<String>`. Empty string is not the same as None - None means "not found", empty string means "found but empty."

### 3. Error Handling

**Challenge:** Go often ignores errors with `_`; Rust requires handling every `Result`.

**Decision: Custom Error Enum with thiserror**
```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTML parsing failed: {0}")]
    ParseError(String),

    #[error("Encoding detection failed: {0}")]
    EncodingError(String),

    #[error("No extractable content found")]
    NoContent,

    #[error("Extraction failed: {0}")]
    ExtractionError(String),
}

pub type Result<T> = std::result::Result<T, Error>;
```

**Rule:** All public APIs return `Result<T, Error>`. Internal functions may use `Option` for "not found" vs `Result` for "failed."

### 4. String Handling & Ownership

**Challenge:** Go strings are immutable and cheap to pass. Rust requires explicit ownership decisions.

**Decision: Phase 1 Uses `String` Everywhere**
```rust
// Phase 1: Simple, correct, may allocate more than necessary
fn clean_text(s: &str) -> String {
    let s = s.trim();
    WHITESPACE_RE.replace_all(s, " ").into_owned()
}
```

**Phase 2 Optimization Path:**
```rust
// Phase 2: Cow<str> for copy-on-write efficiency
fn clean_text(s: &str) -> Cow<'_, str> {
    let trimmed = s.trim();
    if needs_normalization(trimmed) {
        Cow::Owned(WHITESPACE_RE.replace_all(trimmed, " ").into_owned())
    } else {
        Cow::Borrowed(trimmed)
    }
}
```

### 5. Regex Compilation

**Challenge:** Go compiles regexes at package init with `regexp.MustCompile`. Rust needs explicit lazy initialization.

**Decision: Use `std::sync::LazyLock` (MSRV 1.85+)**
```rust
use std::sync::LazyLock;
use regex::Regex;

static TITLE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<title[^>]*>(.*?)</title>").unwrap()
});
```

**Rule:** All regex patterns are static `LazyLock<Regex>`. Group related patterns in a `patterns` module if count exceeds ~10.

### 6. Concurrency Compatibility

**Challenge:** Go trivially spawns goroutines. Rust async is more complex.

**Decision: Ensure `Send + Sync` from Day 1**
- All public types must be `Send + Sync`
- No `Rc`, `RefCell` in public API (use `Arc`, `Mutex` if needed)
- Phase 1 is synchronous only
- Phase 2 may add async support

**Rule:** Run `cargo check` with `fn assert_send_sync<T: Send + Sync>() {}` tests for all public types.

### Summary: Locked Decisions

| Challenge | Decision | Phase |
|-----------|----------|-------|
| Tree traversal | Collect-then-process pattern | Phase 1 |
| Nil handling | `Option<T>` with `.or_else()` chains | Phase 1 |
| Errors | `thiserror` enum, ~4 variants | Phase 1 |
| Strings | `String` everywhere | Phase 1 |
| Strings | `Cow<str>` optimization | Phase 2 |
| Regex | `LazyLock<Regex>` statics | Phase 1 |
| Concurrency | `Send + Sync` all types | Phase 1 |
| Async | Add async support | Phase 2 |

---

## Starter Template Evaluation

### Primary Technology Domain

**Rust library/crate** - developer tool for content extraction

No complex scaffolding needed. Standard Cargo library structure with manual configuration.

### Starter Decision

**Selected: `cargo new rs-trafilatura --lib` with manual configuration**

**Rationale:**
- Porting established algorithm, not building new architecture
- Simple library crate, no web framework or database
- Standard Cargo structure is sufficient
- Can evolve to workspace for CLI crate in Phase 2

### Initialization Command

```bash
cargo new rs-trafilatura --lib
cd rs-trafilatura
```

### Rust Edition

**Edition:** 2024 (released February 2025 with Rust 1.85)

Rationale: Latest stable edition with improved features (async closures, let chains, better tooling). No reason to use older edition for greenfield project.

### Cargo.toml Configuration

```toml
[package]
name = "rs-trafilatura"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
description = "Rust port of trafilatura - web content extraction library"
license = "MIT OR Apache-2.0"
repository = "https://github.com/username/rs-trafilatura"
keywords = ["html", "content-extraction", "web-scraping", "trafilatura", "readability"]
categories = ["parser-implementations", "web-programming", "text-processing"]

[dependencies]
scraper = "0.25"
thiserror = "2.0"
regex = "1.11"
chrono = "0.4"
encoding_rs = "0.8"

[dev-dependencies]
criterion = "0.5"

[[bench]]
name = "benchmark"
harness = false

[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
all = "warn"
pedantic = "warn"
unwrap_used = "deny"
expect_used = "deny"
```

### Project Structure

```
rs-trafilatura/
├── Cargo.toml
├── rust-toolchain.toml      # Pin Rust version
├── .rustfmt.toml            # Formatting config
├── src/
│   ├── lib.rs               # Public API
│   ├── extract.rs           # Content + metadata extraction
│   ├── options.rs           # Options struct
│   ├── result.rs            # ExtractResult, Metadata
│   └── error.rs             # Error types
├── benches/
│   └── benchmark.rs         # Criterion benchmarks
├── tests/
│   ├── benchmark_suite/     # 983 vendored test files
│   └── integration.rs       # Integration tests
└── examples/
    └── basic.rs             # Usage example
```

### Tooling Configuration

**rust-toolchain.toml:**
```toml
[toolchain]
channel = "1.85"
components = ["rustfmt", "clippy"]
```

**Lint Policy:**
- `unsafe_code = "forbid"` - No unsafe code in library
- `unwrap_used = "deny"` - Enforce error handling (NFR4: no panics)
- `expect_used = "deny"` - Same rationale
- Clippy pedantic warnings enabled

### Architectural Decisions from Starter

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Rust Edition | 2024 | Latest stable, new features |
| MSRV | 1.85 | Required for Edition 2024 |
| License | MIT OR Apache-2.0 | Standard Rust dual license |
| Unsafe | Forbidden | Library must be safe |
| Panics | Denied via lint | NFR4 compliance |
| Benchmarking | Criterion | Standard Rust benchmarking |

**Note:** Project initialization should be the first implementation task.

---

## Core Architectural Decisions

### Decision Priority Analysis

**Critical Decisions (Block Implementation):**
- Public API surface: `extract(&str)` and `extract_with_options(&str, &Options)`
- Error handling: `thiserror` enum with 4 variants
- HTML parser: `scraper` 0.25

**Important Decisions (Shape Architecture):**
- Pattern organization: centralized `patterns.rs` module
- Options construction: public fields + `Default`
- Testing strategy: tests for accuracy, benches for performance
- Extension design: internal `Extractor` trait for Phase 2

**Deferred Decisions (Post-MVP):**
- Reader variant (`impl Read`) - reconsider with streaming parser
- Async support - Phase 2
- Public extractor trait - only if users need custom extractors

### Public API Design

**Input Strategy:** `&str` only
- Simple, honest API
- Matches go-trafilatura pattern
- Reader variant deferred - would just buffer internally with current parser

**Options Construction:** Public fields + `Default` trait
```rust
pub struct Options {
    pub include_comments: bool,
    pub include_tables: bool,
    pub include_images: bool,
    pub include_links: bool,
    pub favor_precision: bool,
    pub favor_recall: bool,
    pub target_language: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            include_comments: false,
            include_tables: true,
            include_images: false,
            include_links: false,
            favor_precision: false,
            favor_recall: false,
            target_language: None,
        }
    }
}
```

**Public Types Exposed:**
- `extract(html: &str) -> Result<ExtractResult>`
- `extract_with_options(html: &str, options: &Options) -> Result<ExtractResult>`
- `ExtractResult` - extraction output
- `Metadata` - metadata fields
- `Options` - configuration
- `Error` - error enum
- `Result<T>` - type alias

### Internal Architecture

**Pattern Organization:** Single `patterns.rs` module
- All regex patterns centralized in one file
- Easy to audit against go-trafilatura patterns
- Clear ownership and organization
- Compile with `LazyLock<Regex>`

**Module Responsibilities:**
| Module | Responsibility |
|--------|---------------|
| `lib.rs` | Public API, re-exports |
| `extract.rs` | Content + metadata extraction algorithm |
| `patterns.rs` | All compiled regex patterns |
| `options.rs` | Options struct + Default |
| `result.rs` | ExtractResult, Metadata structs |
| `error.rs` | Error enum with thiserror |

**Data Flow:**
```
html: &str
    │
    ▼
┌─────────────────┐
│ scraper::Html   │  Parse HTML
│ parse_document  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ extract.rs      │  Content extraction
│ + patterns.rs   │  + Metadata extraction
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ ExtractResult   │  Return structured result
│ + Metadata      │
└─────────────────┘
```

### Testing & Quality Strategy

**Test Organization:**
```
tests/
├── benchmark_suite/     # 983 vendored HTML files + expected outputs
├── accuracy_test.rs     # F-Score validation (must pass ≥ 0.90)
└── integration.rs       # API integration tests

benches/
└── performance.rs       # Criterion speed benchmarks
```

**Separation of Concerns:**
- `cargo test` - validates correctness (F-Score ≥ 0.90 is pass/fail)
- `cargo bench` - measures performance (informational)

**F-Score Calculation:** Port scoring logic from content-extractor-benchmark
- Precision, recall, accuracy metrics
- Compare extracted text vs expected text
- Report detailed results per test file

### Phase 2 Extension Points

**Internal Extractor Trait:**
```rust
// Private - not exposed to users
trait Extractor {
    fn extract(&self, doc: &Html, options: &Options) -> Result<ExtractResult>;
}

struct TrafilaturaExtractor;

impl Extractor for TrafilaturaExtractor {
    fn extract(&self, doc: &Html, options: &Options) -> Result<ExtractResult> {
        // Core trafilatura algorithm
    }
}

// Phase 2 addition:
// struct FallbackExtractor {
//     primary: TrafilaturaExtractor,
//     fallback: DomSmoothieExtractor,
// }
```

**Rationale:** Clean internal design allows adding dom_smoothie fallback without public API changes. Can expose trait publicly later if users need custom extractors.

### CI/CD Pipeline

**Matrix Testing:**
| Axis | Values |
|------|--------|
| OS | ubuntu-latest, macos-latest, windows-latest |
| Rust | stable, 1.85 (MSRV) |

**Pipeline Steps:**
```yaml
- cargo fmt --check           # Formatting
- cargo clippy -- -D warnings # Lints (deny warnings)
- cargo build                 # Compile
- cargo test                  # Unit + integration + F-Score
- cargo doc                   # Documentation builds
- cargo audit                 # Security vulnerabilities
```

**Quality Gates:**
- All tests pass including F-Score ≥ 0.90
- No clippy warnings
- No formatting issues
- Clean audit report

### Decision Summary Table

| Category | Decision | Rationale |
|----------|----------|-----------|
| API Input | `&str` only | Simple, honest, matches Go |
| Options | Public fields + Default | Idiomatic Rust, low ceremony |
| Patterns | Centralized `patterns.rs` | Easy audit, clear ownership |
| Testing | Tests for accuracy, benches for speed | Separation of concerns |
| Extension | Internal Extractor trait | Phase 2 ready, no API churn |
| CI | Comprehensive matrix | Library quality for crates.io |

---

## Implementation Patterns & Consistency Rules

### Critical Conflict Points Identified

**8 areas** where AI agents could make different implementation choices for rs-trafilatura.

### Naming Patterns

**Function Naming:**
| Context | Convention | Example |
|---------|------------|---------|
| Public extraction | `extract_*` | `extract(html)`, `extract_with_options(html, options)` |
| Internal extraction steps | `extract_*` | `extract_title(doc)`, `extract_author(doc)` |
| Validation/checking | `is_*` or `has_*` | `is_boilerplate(text)`, `has_content(node)` |
| Transformation | `clean_*` or `normalize_*` | `clean_text(s)`, `normalize_whitespace(s)` |
| Traversal helpers | `collect_*` or `find_*` | `collect_text_nodes(node)`, `find_main_content(doc)` |

**Type Naming:**
| Type | Convention | Example |
|------|------------|---------|
| Result types | `*Result` | `ExtractResult` |
| Configuration | `Options` or `Config` | `Options` (chosen) |
| Error enums | `Error` | `Error` |
| Internal structs | Descriptive | `ContentNode`, `TextBlock` |

**Regex Pattern Naming:**
```rust
// Pattern: SCREAMING_SNAKE_CASE with descriptive suffix
static TITLE_TAG: LazyLock<Regex> = ...;      // Matches <title> tags
static AUTHOR_META: LazyLock<Regex> = ...;    // Matches author meta tags
static BOILERPLATE_CLASS: LazyLock<Regex> = ...;  // Matches boilerplate class names
```

**Rule:** All regex patterns are `static LazyLock<Regex>` with SCREAMING_SNAKE_CASE names. Suffix indicates what it matches (TAG, META, CLASS, TEXT).

### Code Style Patterns

**Error Propagation:**
```rust
// CORRECT: Use ? operator with meaningful error context
fn extract_title(doc: &Html) -> Result<Option<String>> {
    let title_node = doc.select(&TITLE_SELECTOR).next();
    // Return Option wrapped in Result, don't convert None to Error here
    Ok(title_node.map(|n| n.text().collect()))
}

// INCORRECT: Explicit match when ? suffices
fn extract_title(doc: &Html) -> Result<Option<String>> {
    match doc.select(&TITLE_SELECTOR).next() {
        Some(node) => Ok(Some(node.text().collect())),
        None => Ok(None),
    }
}
```

**Iterator Patterns:**
```rust
// CORRECT: Functional iterator chains for transformations
let text: String = node
    .descendants()
    .filter_map(|n| n.value().as_text())
    .map(|t| t.trim())
    .filter(|t| !t.is_empty())
    .collect::<Vec<_>>()
    .join(" ");

// CORRECT: For loops when mutation or early exit needed
for child in node.children() {
    if is_boilerplate(&child) {
        to_remove.push(child.id());
    }
}
```

**String Handling (Phase 1):**
```rust
// CORRECT: Accept &str, return String
fn clean_text(s: &str) -> String {
    s.trim().to_string()
}

// INCORRECT: Accept String when &str works
fn clean_text(s: String) -> String {
    s.trim().to_string()
}
```

**Rule:** Functions accept `&str` and return `String`. Phase 2 may introduce `Cow<str>`.

### Documentation Patterns

**Module-Level Docs:**
```rust
//! # Extract
//!
//! Core content extraction algorithm ported from go-trafilatura.
//!
//! This module contains the main extraction logic that identifies
//! and extracts meaningful content from HTML documents.
```

**Function-Level Docs:**
```rust
/// Extracts main content from an HTML document.
///
/// # Arguments
///
/// * `html` - The HTML document as a string slice
///
/// # Returns
///
/// Returns `Ok(ExtractResult)` on success, or an `Error` if extraction fails.
///
/// # Example
///
/// ```
/// let result = rs_trafilatura::extract(html)?;
/// println!("Content: {}", result.content_text);
/// ```
pub fn extract(html: &str) -> Result<ExtractResult> {
    // ...
}
```

**Rule:** All public functions have full rustdoc with Arguments, Returns, and Example sections. Internal functions need only a single-line description.

### Test Patterns

**Test Naming:**
```rust
#[test]
fn extract_returns_title_from_title_tag() { }

#[test]
fn extract_returns_none_for_missing_author() { }

#[test]
fn extract_handles_malformed_html_gracefully() { }
```

**Pattern:** `<function>_<behavior>_<condition>` - describes what should happen.

**Test Organization:**
```
tests/
├── benchmark_suite/      # 983 vendored test files
│   ├── input/            # HTML input files
│   └── expected/         # Expected output files
├── accuracy_test.rs      # F-Score validation (single file)
└── integration.rs        # Public API tests

src/
├── lib.rs                # No tests here
├── extract.rs            # Unit tests in #[cfg(test)] mod tests
└── ...
```

**Rule:** Unit tests go inline with `#[cfg(test)]`. Integration tests go in `tests/`. Benchmark suite is separate.

**Assertion Style:**
```rust
// CORRECT: Descriptive assertion with context
assert_eq!(
    result.metadata.title,
    Some("Expected Title".to_string()),
    "Title extraction failed for test case: {}",
    test_name
);

// INCORRECT: Bare assertion without context
assert!(result.metadata.title.is_some());
```

### Process Patterns

**Error Creation:**
```rust
// CORRECT: Use thiserror variants with context
return Err(Error::ParseError(format!(
    "Failed to parse HTML: invalid UTF-8 at byte {}",
    position
)));

// INCORRECT: Generic error messages
return Err(Error::ParseError("parse failed".into()));
```

**Option vs Result Distinction:**
| Situation | Return Type | Example |
|-----------|-------------|---------|
| Value might not exist | `Option<T>` | `extract_author() -> Option<String>` |
| Operation might fail | `Result<T>` | `parse_date(s) -> Result<DateTime>` |
| Might not exist OR fail | `Result<Option<T>>` | `extract_title() -> Result<Option<String>>` |

**Logging (Phase 1):**
- No logging in library code
- Use `Result` and `Option` to communicate state
- Consider `tracing` crate for Phase 2 if debugging becomes necessary

### Module Organization Patterns

**Import Organization:**
```rust
// 1. Standard library
use std::sync::LazyLock;

// 2. External crates
use regex::Regex;
use scraper::{Html, Selector};

// 3. Internal modules
use crate::error::{Error, Result};
use crate::options::Options;
```

**Re-exports in lib.rs:**
```rust
// Public API - explicit re-exports
pub use error::{Error, Result};
pub use options::Options;
pub use result::{ExtractResult, Metadata};

// Public functions
pub fn extract(html: &str) -> Result<ExtractResult> { ... }
pub fn extract_with_options(html: &str, options: &Options) -> Result<ExtractResult> { ... }
```

**Rule:** Only re-export what users need. Keep internal modules private.

### Enforcement Guidelines

**All AI Agents MUST:**

1. **Follow naming conventions** exactly as specified in this document
2. **Use `?` operator** for error propagation (never explicit match for simple propagation)
3. **Accept `&str`** and return `String` in Phase 1
4. **Never use `.unwrap()` or `.expect()`** in library code (enforced by clippy lint)
5. **Write inline unit tests** with `#[cfg(test)]` blocks
6. **Document all public items** with rustdoc including examples
7. **Organize imports** in standard library → external → internal order
8. **Name regex patterns** with SCREAMING_SNAKE_CASE and descriptive suffixes

**Pattern Enforcement:**
- `cargo clippy -- -D warnings` catches style violations
- `cargo fmt --check` enforces formatting
- CI pipeline blocks merges that violate patterns
- Code review checks for naming consistency

### Pattern Examples

**Good Example - Extraction Function:**
```rust
/// Extracts the page title from the HTML document.
///
/// Searches for title in this order:
/// 1. `<title>` tag content
/// 2. `og:title` meta tag
/// 3. `<h1>` tag content
///
/// # Returns
///
/// `Ok(Some(title))` if found, `Ok(None)` if not present.
fn extract_title(doc: &Html) -> Result<Option<String>> {
    // Try <title> tag first
    if let Some(title) = doc.select(&TITLE_SELECTOR).next() {
        let text: String = title.text().collect();
        let cleaned = clean_text(&text);
        if !cleaned.is_empty() {
            return Ok(Some(cleaned));
        }
    }

    // Fallback to og:title
    Ok(extract_meta_content(doc, "og:title"))
}
```

**Anti-Patterns to Avoid:**

```rust
// ❌ WRONG: Using unwrap
let title = doc.select(&selector).next().unwrap();

// ❌ WRONG: Inconsistent naming
fn getTitle(doc: &Html) -> String { }  // camelCase
fn title_extract(doc: &Html) { }       // wrong order

// ❌ WRONG: Accepting String when &str works
fn clean_text(s: String) -> String { }

// ❌ WRONG: No documentation on public function
pub fn extract(html: &str) -> Result<ExtractResult> {
    // implementation
}

// ❌ WRONG: Explicit match for simple propagation
match some_result {
    Ok(v) => Ok(v),
    Err(e) => Err(e),
}
```

---

## Project Structure & Boundaries

### Complete Project Directory Structure

```
rs-trafilatura/
├── Cargo.toml                    # Package manifest (dependencies, metadata)
├── Cargo.lock                    # Dependency lock file (committed)
├── rust-toolchain.toml           # Pin Rust version to 1.85
├── .rustfmt.toml                 # Formatting configuration
├── .gitignore                    # Git ignore patterns
├── LICENSE-MIT                   # MIT license text
├── LICENSE-APACHE                # Apache 2.0 license text
├── README.md                     # Crate documentation and examples
├── CHANGELOG.md                  # Version history
│
├── .github/
│   └── workflows/
│       └── ci.yml                # CI pipeline (format, lint, test, doc)
│
├── src/
│   ├── lib.rs                    # Public API: extract(), extract_with_options()
│   │                             # Re-exports: Error, Result, Options, ExtractResult, Metadata
│   │
│   ├── extract.rs                # Core extraction algorithm
│   │                             # - Content extraction (FR1-6)
│   │                             # - Metadata extraction (FR7-17)
│   │                             # - Boilerplate detection
│   │                             # - Text cleaning and normalization
│   │
│   ├── patterns.rs               # All compiled regex patterns
│   │                             # - Static LazyLock<Regex> definitions
│   │                             # - Selector constants
│   │                             # - Pattern groups by purpose
│   │
│   ├── options.rs                # Configuration (FR18-24)
│   │                             # - Options struct with public fields
│   │                             # - Default implementation
│   │
│   ├── result.rs                 # Output types
│   │                             # - ExtractResult struct
│   │                             # - Metadata struct (15 fields)
│   │
│   └── error.rs                  # Error handling (FR30-33)
│                                 # - Error enum with thiserror
│                                 # - Result type alias
│
├── benches/
│   └── performance.rs            # Criterion benchmarks
│                                 # - Single document extraction
│                                 # - Batch extraction
│                                 # - Metadata-only extraction
│
├── tests/
│   ├── benchmark_suite/          # 983 vendored test files
│   │   ├── input/                # HTML input files
│   │   │   └── *.html            # 983 test HTML documents
│   │   └── expected/             # Expected extraction results
│   │       └── *.txt             # Expected text output per file
│   │
│   ├── accuracy_test.rs          # F-Score validation (FR34-35)
│   │                             # - Precision/recall calculation
│   │                             # - Batch test runner
│   │                             # - Results reporting
│   │
│   └── integration.rs            # Public API tests
│                                 # - extract() function tests
│                                 # - extract_with_options() tests
│                                 # - Options variations
│                                 # - Edge cases (empty, malformed)
│
└── examples/
    └── basic.rs                  # Usage example for docs.rs
                                  # - Simple extraction
                                  # - Options configuration
                                  # - Error handling
```

### Architectural Boundaries

**Library Boundary (Public API):**
```rust
// src/lib.rs - The ONLY public interface
pub fn extract(html: &str) -> Result<ExtractResult>
pub fn extract_with_options(html: &str, options: &Options) -> Result<ExtractResult>

// Public types (re-exported)
pub use error::{Error, Result};
pub use options::Options;
pub use result::{ExtractResult, Metadata};
```

**Internal Boundaries:**
| Module | Visibility | Dependencies |
|--------|------------|--------------|
| `lib.rs` | Public | extract, options, result, error |
| `extract.rs` | `pub(crate)` | patterns, options, result, error |
| `patterns.rs` | `pub(crate)` | None (leaf module) |
| `options.rs` | `pub(crate)` | None (leaf module) |
| `result.rs` | `pub(crate)` | chrono (for DateTime) |
| `error.rs` | `pub(crate)` | thiserror |

**Data Flow:**
```
User Input (html: &str)
        │
        ▼
┌─────────────────────────┐
│ lib.rs                  │
│ extract_with_options()  │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│ extract.rs              │
│ - Parse HTML (scraper)  │
│ - Identify content      │
│ - Extract metadata      │
│ - Clean text            │
└───────────┬─────────────┘
            │ Uses
            ▼
┌─────────────────────────┐
│ patterns.rs             │
│ - Compiled regex        │
│ - CSS selectors         │
└─────────────────────────┘
            │
            ▼
┌─────────────────────────┐
│ result.rs               │
│ ExtractResult {         │
│   content_text,         │
│   content_html,         │
│   metadata: Metadata    │
│ }                       │
└─────────────────────────┘
```

### Requirements to Structure Mapping

**Content Extraction (FR1-6):**
| Requirement | Implementation Location |
|-------------|------------------------|
| FR1: Extract main content text | `extract.rs::extract_content()` |
| FR2: Extract content as HTML | `extract.rs::extract_content_html()` |
| FR3: Remove boilerplate | `extract.rs::is_boilerplate()` |
| FR4: Identify article body | `extract.rs::find_main_content()` |
| FR5: Extract tables | `extract.rs` (via Options flag) |
| FR6: Extract comments | `extract.rs` (via Options flag) |

**Metadata Extraction (FR7-17):**
| Requirement | Implementation Location |
|-------------|------------------------|
| FR7: Title | `extract.rs::extract_title()` |
| FR8: Author | `extract.rs::extract_author()` |
| FR9: Publication date | `extract.rs::extract_date()` |
| FR10: Site name | `extract.rs::extract_sitename()` |
| FR11: Language | `extract.rs::extract_language()` |
| FR12-17: Other metadata | `extract.rs::extract_*()` functions |

**Configuration (FR18-24):**
| Requirement | Implementation Location |
|-------------|------------------------|
| FR18-19: Precision/recall | `options.rs::Options.favor_precision/recall` |
| FR20-24: Feature flags | `options.rs::Options.include_*` fields |

**Error Handling (FR30-33):**
| Requirement | Implementation Location |
|-------------|------------------------|
| FR30: Handle malformed HTML | `error.rs::Error::ParseError` |
| FR31: Handle encoding | `error.rs::Error::EncodingError` |
| FR32: Partial results | `extract.rs` returns partial on soft failures |
| FR33: Meaningful errors | `error.rs` variants with context |

**Quality Validation (FR34-35):**
| Requirement | Implementation Location |
|-------------|------------------------|
| FR34: Benchmark validation | `tests/accuracy_test.rs` |
| FR35: Output comparison | `tests/accuracy_test.rs::calculate_f_score()` |

### File Organization Patterns

**Configuration Files:**
| File | Purpose |
|------|---------|
| `Cargo.toml` | Dependencies, metadata, lint config |
| `rust-toolchain.toml` | Pin Rust 1.85 for Edition 2024 |
| `.rustfmt.toml` | Code formatting rules |
| `.gitignore` | Standard Rust ignores + build artifacts |

**Source Organization:**
- **Flat structure** - 5 modules in `src/`, no nested directories
- **Single responsibility** - each module has clear ownership
- **Leaf modules** - `patterns.rs`, `options.rs`, `result.rs`, `error.rs` have no internal dependencies

**Test Organization:**
| Location | Purpose |
|----------|---------|
| `src/*.rs` inline `#[cfg(test)]` | Unit tests for internals |
| `tests/integration.rs` | Public API tests |
| `tests/accuracy_test.rs` | F-Score validation |
| `tests/benchmark_suite/` | 983 vendored test files |
| `benches/performance.rs` | Speed benchmarks (Criterion) |

### Development Workflow Integration

**Development Commands:**
```bash
# Format code
cargo fmt

# Check lints
cargo clippy -- -D warnings

# Run unit + integration tests
cargo test

# Run F-Score accuracy tests
cargo test --test accuracy_test

# Run benchmarks
cargo bench

# Build documentation
cargo doc --open

# Full CI check
cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo doc
```

**CI Pipeline Steps:**
1. `cargo fmt --check` - Formatting
2. `cargo clippy -- -D warnings` - Lints
3. `cargo build` - Compilation
4. `cargo test` - All tests including F-Score
5. `cargo doc` - Documentation
6. `cargo audit` - Security audit

**Release Process:**
1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Run full CI checks
4. Tag release
5. `cargo publish`

---

## Architecture Validation Results

### Coherence Validation ✅

**Decision Compatibility:**
All technology choices work together without conflicts:
- scraper 0.25 + thiserror 2.0 + regex 1.11 + chrono 0.4 + encoding_rs 0.8 are all compatible
- Rust 2024 Edition requires MSRV 1.85 (aligned)
- LazyLock for static regex is stable in std since Rust 1.80
- scraper foundation enables seamless dom_smoothie integration in Phase 2

**Pattern Consistency:**
Implementation patterns fully support architectural decisions:
- Naming conventions (extract_*, is_*, clean_*) apply uniformly
- Error handling patterns (thiserror, Result<T>, no unwrap) are consistent
- String handling (&str input, String output) is uniform across all functions
- Test patterns (inline unit tests, separate integration tests) align with project structure

**Structure Alignment:**
Project structure supports all architectural decisions:
- Flat src/ structure matches the 5-module design
- tests/ organization supports accuracy validation requirement
- benches/ enables Phase 2 performance work
- examples/ supports documentation requirements

### Requirements Coverage Validation ✅

**Functional Requirements Coverage (35 FRs):**

| Category | FRs | Status | Implementation |
|----------|-----|--------|----------------|
| Content Extraction | FR1-6 | ✅ | `extract.rs` |
| Metadata Extraction | FR7-17 | ✅ | `extract.rs` (coupled) |
| Configuration | FR18-24 | ✅ | `options.rs` |
| API & Integration | FR25-29 | ✅ | `lib.rs` |
| Error Handling | FR30-33 | ✅ | `error.rs` |
| Quality Validation | FR34-35 | ✅ | `tests/accuracy_test.rs` |

**Non-Functional Requirements Coverage (17 NFRs):**

| Category | NFRs | Status | Implementation |
|----------|------|--------|----------------|
| Accuracy | NFR1-3 | ✅ | F-Score tests, benchmark suite |
| Reliability | NFR4-7 | ✅ | Clippy lints, error enum, graceful degradation |
| Performance | NFR8-10 | ✅ | Phase 1 baseline, Phase 2 optimization |
| Compatibility | NFR11-14 | ✅ | CI matrix (3 OS, stable + MSRV) |
| Documentation | NFR15-17 | ✅ | Rustdoc patterns, examples/ |

### Implementation Readiness Validation ✅

**Decision Completeness:**
- All critical decisions documented with specific versions
- Implementation patterns cover naming, code style, documentation, testing
- Consistency rules are clear (8 "MUST" rules for AI agents)
- Good/bad examples provided for each major pattern

**Structure Completeness:**
- Complete project tree with all files and directories
- Every module has defined responsibility
- Integration points (lib.rs as sole public interface) clearly specified
- Component boundaries defined with visibility table

**Pattern Completeness:**
- All potential conflict points addressed (naming, style, docs, tests)
- Error handling patterns (Option vs Result vs Result<Option>) specified
- Process patterns (CI pipeline, release process) documented
- Anti-patterns explicitly called out

### Gap Analysis Results

**Critical Gaps:** None identified

**Important Gaps:** None identified

**Nice-to-Have Enhancements:**
- `.rustfmt.toml` contents could be specified (default is acceptable)
- More regex pattern examples from go-trafilatura could be documented
- Phase 2 optimization targets could be more specific (deferred by design)

### Architecture Completeness Checklist

**✅ Requirements Analysis**
- [x] Project context thoroughly analyzed (Rust library, content extraction)
- [x] Scale and complexity assessed (Medium, 5 modules)
- [x] Technical constraints identified (scraper, no panics, cross-platform)
- [x] Cross-cutting concerns mapped (error handling, DOM traversal, strings)

**✅ Architectural Decisions**
- [x] Critical decisions documented with versions (scraper 0.25, Rust 1.85)
- [x] Technology stack fully specified (5 dependencies)
- [x] Integration patterns defined (internal Extractor trait for Phase 2)
- [x] Performance considerations addressed (Phase 1 measure, Phase 2 optimize)

**✅ Implementation Patterns**
- [x] Naming conventions established (extract_*, is_*, SCREAMING_CASE)
- [x] Structure patterns defined (flat modules, import ordering)
- [x] Communication patterns specified (data flow diagram)
- [x] Process patterns documented (CI pipeline, release process)

**✅ Project Structure**
- [x] Complete directory structure defined (all files specified)
- [x] Component boundaries established (visibility table)
- [x] Integration points mapped (lib.rs as sole interface)
- [x] Requirements to structure mapping complete (FR → location table)

### Architecture Readiness Assessment

**Overall Status:** READY FOR IMPLEMENTATION

**Confidence Level:** HIGH

The architecture is complete, coherent, and provides sufficient guidance for AI agents to implement consistently. All 35 functional requirements and 17 non-functional requirements have clear architectural support.

**Key Strengths:**
1. **Clear boundaries** - Single public interface (lib.rs), well-defined module responsibilities
2. **Proven foundation** - Direct port of go-trafilatura with known accuracy (0.906 F-Score)
3. **Quality gates** - Clippy lints, F-Score tests, CI pipeline block issues early
4. **Phase 2 ready** - Internal Extractor trait, dom_smoothie compatibility via scraper
5. **Comprehensive patterns** - 8 enforcement rules, good/bad examples for each pattern

**Areas for Future Enhancement:**
- Phase 2: `Cow<str>` optimization for reduced allocations
- Phase 2: dom_smoothie fallback integration
- Phase 2: Async support
- Future: CLI tool, batch processing

### Implementation Handoff

**AI Agent Guidelines:**
1. Follow all architectural decisions exactly as documented
2. Use implementation patterns consistently across all components
3. Respect project structure and boundaries
4. Refer to this document for all architectural questions
5. Never use `.unwrap()` or `.expect()` in library code
6. All public APIs must return `Result<T, Error>`

**First Implementation Priority:**
```bash
cargo new rs-trafilatura --lib
```

Then configure `Cargo.toml` per the Starter Template section and create the 5 source modules.

---

## Feature Comparison & Performance

### Output Format Comparison

| Format | Python trafilatura | Go-trafilatura | RS-trafilatura |
|--------|-------------------|----------------|----------------|
| Plain text (TXT) | ✅ | ✅ ContentText | ✅ content_text |
| HTML | ✅ | ✅ ContentNode | ✅ content_html |
| Markdown | ✅ | ❌ | ❌ |
| XML | ✅ | ❌ | ❌ |
| XMLTEI | ✅ | ❌ | ❌ |
| JSON | ✅ | ❌ | ❌ (struct is serde-compatible) |
| CSV | ✅ | ❌ | ❌ |

**Status:** RS-trafilatura is at feature parity with go-trafilatura for output formats. Both provide text and HTML output. The Python version has additional output format options (Markdown, XML, JSON, CSV, XMLTEI) that neither Go nor Rust ports implement.

### Options Comparison (RS vs Go)

| Option | Go-trafilatura | RS-trafilatura | Notes |
|--------|----------------|----------------|-------|
| EnableFallback | ✅ | ✅ use_readability_fallback | |
| Focus (Balanced/FavorRecall/FavorPrecision) | ✅ | ✅ favor_precision/favor_recall | |
| ExcludeComments | ✅ | ✅ include_comments (inverted) | |
| ExcludeTables | ✅ | ✅ include_tables (inverted) | |
| IncludeImages | ✅ | ✅ | |
| IncludeLinks | ✅ | ✅ | |
| BlacklistedAuthors | ✅ | ✅ author_blacklist | |
| Deduplicate | ✅ | ✅ | |
| TargetLanguage | ✅ | ✅ | |
| HasEssentialMetadata | ✅ | ❌ | Option to require date, title, url |
| MaxTreeSize | ✅ | ✅ max_tree_depth | |
| HtmlDateMode | ✅ | ❌ | Control over date extraction behavior |
| PruneSelector | ✅ | ❌ | User-provided CSS selector to prune elements |

**Missing from rs-trafilatura vs Go:**
1. **HasEssentialMetadata** - option to require date, title, url
2. **HtmlDateMode** - control over date extraction behavior (Fast/Extensive/Disabled)
3. **PruneSelector** - user-provided CSS selector to prune elements before extraction

### Performance Benchmarks

Performance comparison on 3,985 HTML documents from the web-content-extraction-benchmark dataset:

| Configuration | RS-trafilatura | GO-trafilatura | RS Speedup |
|--------------|----------------|----------------|------------|
| Single-threaded | 45.2s (88 docs/s) | 205.6s (19 docs/s) | **4.5x faster** |
| Multi-threaded (8 cores) | 11.5s (347 docs/s) | 63.5s (63 docs/s) | **5.5x faster** |

**Key Performance Insights:**
- RS-trafilatura uses Rayon for parallel processing; Go uses goroutines
- All regex patterns are pre-compiled using `LazyLock<Regex>`
- Rust's zero-cost abstractions provide significant performance advantages

### Potential Optimizations (Future Work)

The following optimizations have been identified but not yet implemented:

1. **Pre-compile CSS selectors** - Currently selectors are compiled per-extraction
2. **String interning** - Reduce allocations for repeated strings
3. **Arena allocators** - Reduce allocation overhead for DOM operations
4. **SIMD text processing** - Accelerate whitespace normalization
5. **Early termination algorithms** - Skip processing when sufficient content found
6. **Memory pooling** - Reuse buffers across extractions
7. **Profile-guided optimization (PGO)** - Compile-time optimization based on runtime profiles
8. **Selector caching** - Cache compiled selectors for repeated patterns

**Note:** Optimization is Phase 2 scope. Current focus is on accuracy and feature parity.

---

## Architecture Completion Summary

### Workflow Completion

**Architecture Decision Workflow:** COMPLETED ✅
**Total Steps Completed:** 8
**Date Completed:** 2025-12-15
**Document Location:** docs/architecture.md

### Final Architecture Deliverables

**📋 Complete Architecture Document**
- All architectural decisions documented with specific versions
- Implementation patterns ensuring AI agent consistency
- Complete project structure with all files and directories
- Requirements to architecture mapping
- Validation confirming coherence and completeness

**🏗️ Implementation Ready Foundation**
- 15+ architectural decisions made
- 8 implementation pattern categories defined
- 5 source modules + tests/benches/examples specified
- 52 requirements (35 FR + 17 NFR) fully supported

**📚 AI Agent Implementation Guide**
- Technology stack with verified versions (scraper 0.25, Rust 1.85, thiserror 2.0)
- Consistency rules that prevent implementation conflicts
- Project structure with clear boundaries
- Integration patterns and communication standards

### Development Sequence

1. Initialize project: `cargo new rs-trafilatura --lib`
2. Configure `Cargo.toml` per Starter Template section
3. Create 5 source modules: `lib.rs`, `extract.rs`, `patterns.rs`, `options.rs`, `result.rs`, `error.rs`
4. Set up CI pipeline in `.github/workflows/ci.yml`
5. Vendor 983 benchmark test files to `tests/benchmark_suite/`
6. Implement core extraction algorithm porting from go-trafilatura
7. Validate F-Score ≥ 0.90 before release

### Quality Assurance Checklist

**✅ Architecture Coherence**
- [x] All decisions work together without conflicts
- [x] Technology choices are compatible
- [x] Patterns support the architectural decisions
- [x] Structure aligns with all choices

**✅ Requirements Coverage**
- [x] All 35 functional requirements are supported
- [x] All 17 non-functional requirements are addressed
- [x] Cross-cutting concerns are handled
- [x] Integration points are defined

**✅ Implementation Readiness**
- [x] Decisions are specific and actionable
- [x] Patterns prevent agent conflicts
- [x] Structure is complete and unambiguous
- [x] Examples are provided for clarity

---

**Architecture Status:** READY FOR IMPLEMENTATION ✅

**Next Phase:** Begin implementation using the architectural decisions and patterns documented herein.

**Document Maintenance:** Update this architecture when major technical decisions are made during implementation.
