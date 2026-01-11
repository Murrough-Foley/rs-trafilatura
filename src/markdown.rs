//! Markdown processing utilities.
//!
//! This module provides utilities for generating clean Markdown output,
//! including escaping special characters and converting HTML tables to GFM format.

/// Characters that have special meaning in Markdown and need escaping.
const MARKDOWN_SPECIAL_CHARS: &[char] = &['\\', '*', '_', '[', ']', '<', '>'];

/// Escape Markdown special characters in text content.
///
/// This function prevents accidental Markdown interpretation of content that
/// contains literal asterisks, underscores, brackets, etc.
///
/// # Arguments
///
/// * `text` - The text content to escape
/// * `in_code_block` - If true, skip escaping (code blocks preserve literal content)
///
/// # Characters Escaped
///
/// - `\` → `\\` (backslash)
/// - `*` → `\*` (asterisk - prevents italic/bold)
/// - `_` → `\_` (underscore - prevents italic/bold)
/// - `[` → `\[` (bracket - prevents links)
/// - `]` → `\]` (bracket - prevents links)
/// - `<` → `\<` (angle bracket - prevents HTML)
/// - `>` → `\>` (angle bracket - prevents blockquotes)
///
/// # Examples
///
/// ```
/// use rs_trafilatura::markdown::escape_markdown;
///
/// // Asterisks are escaped to prevent italic
/// assert_eq!(escape_markdown("*not italic*", false), r"\*not italic\*");
///
/// // Underscores are escaped to prevent italic
/// assert_eq!(escape_markdown("my_variable_name", false), r"my\_variable\_name");
///
/// // Code blocks are not escaped
/// assert_eq!(escape_markdown("*text*", true), "*text*");
/// ```
#[must_use]
pub fn escape_markdown(text: &str, in_code_block: bool) -> String {
    if in_code_block || text.is_empty() {
        return text.to_string();
    }

    let mut result = String::with_capacity(text.len() + text.len() / 4);

    for ch in text.chars() {
        if MARKDOWN_SPECIAL_CHARS.contains(&ch) {
            result.push('\\');
        }
        result.push(ch);
    }

    result
}

/// Post-process Markdown output to escape special characters in text content.
///
/// This function walks through Markdown content and escapes special characters
/// that appear outside of:
/// - Code blocks (fenced ``` or indented)
/// - Inline code (backticks)
/// - Already-escaped sequences
///
/// # Arguments
///
/// * `markdown` - The raw Markdown output from html-cleaning
///
/// # Returns
///
/// Markdown with properly escaped special characters in text content.
#[must_use]
pub fn post_process_markdown(markdown: &str) -> String {
    if markdown.is_empty() {
        return String::new();
    }

    let mut result = String::with_capacity(markdown.len() + markdown.len() / 8);
    let mut chars = markdown.chars().peekable();
    let mut in_fenced_code = false;
    let mut in_inline_code = false;
    let mut line_start = true;

    while let Some(ch) = chars.next() {
        // Track fenced code blocks (```)
        if line_start && ch == '`' {
            let mut backtick_count = 1;
            while chars.peek() == Some(&'`') {
                chars.next();
                backtick_count += 1;
            }

            if backtick_count >= 3 {
                in_fenced_code = !in_fenced_code;
                for _ in 0..backtick_count {
                    result.push('`');
                }
                continue;
            } else {
                // Not a fence, handle as inline code
                for _ in 0..backtick_count {
                    result.push('`');
                }
                in_inline_code = !in_inline_code;
                continue;
            }
        }

        // Track inline code
        if ch == '`' && !in_fenced_code {
            in_inline_code = !in_inline_code;
            result.push(ch);
            line_start = false;
            continue;
        }

        // Track line starts
        if ch == '\n' {
            result.push(ch);
            line_start = true;
            continue;
        }

        // If in code block or inline code, don't escape
        if in_fenced_code || in_inline_code {
            result.push(ch);
            line_start = false;
            continue;
        }

        // Skip already-escaped characters
        if ch == '\\' {
            result.push(ch);
            if let Some(&next) = chars.peek() {
                if MARKDOWN_SPECIAL_CHARS.contains(&next) {
                    result.push(chars.next().unwrap());
                }
            }
            line_start = false;
            continue;
        }

        // Don't escape markdown formatting that should be preserved
        // (bold, italic, links, headings)
        // We only escape special chars that appear as literal text

        // Check for patterns we should preserve:
        // - **bold** and *italic* (matched pairs)
        // - [link](url) format
        // - # headings at line start
        // - - or * list items at line start

        // Preserve heading markers at line start
        if line_start && ch == '#' {
            result.push(ch);
            line_start = false;
            continue;
        }

        // Preserve list markers at line start
        if line_start && (ch == '-' || ch == '*' || ch == '+') {
            if chars.peek() == Some(&' ') {
                result.push(ch);
                line_start = false;
                continue;
            }
        }

        // For asterisks and underscores, we need context-aware escaping
        // Don't escape if it's part of markdown formatting (matched pairs)
        // **bold**, *italic*, __strong__, _emphasis_
        if ch == '*' || ch == '_' {
            // Look ahead to detect patterns
            let mut peek_chars = chars.clone();
            let next1 = peek_chars.next();
            let next2 = peek_chars.next();

            // Check for **bold** or __strong__ (double char pattern)
            let is_double = next1 == Some(ch);

            // Check for *italic* or _emphasis_ (single char, surrounded by non-chars)
            // Look back at what's in result
            let prev = result.chars().last();
            let prev_is_space = prev.map_or(true, |c| c.is_whitespace());
            let prev_is_word = prev.map_or(false, |c| c.is_alphanumeric());

            // Look at what comes after the potential marker
            let after_marker = if is_double { next2 } else { next1 };
            let next_is_word = after_marker.map_or(false, |c| c.is_alphanumeric());
            let next_is_space = after_marker.map_or(true, |c| c.is_whitespace() || c == ch);

            if is_double {
                // ** or __ - likely bold/strong opening or closing
                // Push both characters
                result.push(ch);
                result.push(ch);
                // Consume the second char
                chars.next();
            } else if (prev_is_space || prev_is_word) && next_is_word {
                // Looks like *open* (italic/em) - preserve
                result.push(ch);
            } else if prev_is_word && (next_is_space || next1 == Some(ch)) {
                // Looks like *close* (italic/em) - preserve
                result.push(ch);
            } else {
                // Likely literal asterisk/underscore, escape
                result.push('\\');
                result.push(ch);
            }
            line_start = false;
            continue;
        }

        // Escape other special characters
        if MARKDOWN_SPECIAL_CHARS.contains(&ch) {
            result.push('\\');
        }
        result.push(ch);
        line_start = ch.is_whitespace();
    }

    result
}

/// Convert an HTML table to GitHub Flavored Markdown format.
///
/// # Arguments
///
/// * `table_html` - The HTML table content
///
/// # Returns
///
/// GFM table string with proper formatting.
///
/// # Example Output
///
/// ```text
/// | Header A | Header B |
/// |----------|----------|
/// | Cell 1   | Cell 2   |
/// ```
#[must_use]
pub fn html_table_to_markdown(table_html: &str) -> String {
    use dom_query::Document;

    let doc = Document::from(table_html);
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut has_header = false;
    let mut alignments: Vec<Alignment> = Vec::new();

    // Extract header row
    let thead = doc.select("thead tr");
    if thead.length() > 0 {
        has_header = true;
        for tr in thead.iter() {
            let mut row = Vec::new();
            for th in tr.select("th").iter() {
                let text = th.text().trim().to_string();
                let align = th.attr("align")
                    .map(|a| Alignment::from_str(&a))
                    .unwrap_or(Alignment::None);
                alignments.push(align);
                row.push(text);
            }
            if !row.is_empty() {
                rows.push(row);
            }
        }
    }

    // Extract body rows
    let tbody_rows = doc.select("tbody tr, table > tr");
    for tr in tbody_rows.iter() {
        let mut row = Vec::new();
        let cells = tr.select("td, th");
        for (i, cell) in cells.iter().enumerate() {
            let text = cell.text().trim().to_string();

            // Capture alignment from first row if no header
            if !has_header && rows.is_empty() {
                let align = cell.attr("align")
                    .map(|a| Alignment::from_str(&a))
                    .unwrap_or(Alignment::None);
                alignments.push(align);
            } else if i < alignments.len() && alignments[i] == Alignment::None {
                // Update alignment if not set
                if let Some(align_str) = cell.attr("align") {
                    alignments[i] = Alignment::from_str(&align_str);
                }
            }

            row.push(text);
        }
        if !row.is_empty() {
            rows.push(row);
        }
    }

    if rows.is_empty() {
        return String::new();
    }

    // Calculate column widths
    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut col_widths: Vec<usize> = vec![3; col_count]; // Minimum width for ---

    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_widths.len() {
                col_widths[i] = col_widths[i].max(cell.len());
            }
        }
    }

    // Ensure alignments vector is the right size
    while alignments.len() < col_count {
        alignments.push(Alignment::None);
    }

    // Build output
    let mut output = String::new();

    for (row_idx, row) in rows.iter().enumerate() {
        // Build row
        output.push('|');
        for (col_idx, cell) in row.iter().enumerate() {
            let width = col_widths.get(col_idx).copied().unwrap_or(3);
            output.push(' ');
            output.push_str(&pad_cell(cell, width, alignments.get(col_idx).copied().unwrap_or(Alignment::None)));
            output.push_str(" |");
        }
        // Pad missing cells
        for col_idx in row.len()..col_count {
            let width = col_widths.get(col_idx).copied().unwrap_or(3);
            output.push(' ');
            output.push_str(&" ".repeat(width));
            output.push_str(" |");
        }
        output.push('\n');

        // Add separator after header (first row if has_header, or we treat first row as header)
        if row_idx == 0 {
            output.push('|');
            for col_idx in 0..col_count {
                let width = col_widths.get(col_idx).copied().unwrap_or(3);
                let align = alignments.get(col_idx).copied().unwrap_or(Alignment::None);
                output.push_str(&format_separator(width, align));
                output.push('|');
            }
            output.push('\n');
        }
    }

    output
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Alignment {
    None,
    Left,
    Center,
    Right,
}

impl Alignment {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "left" => Self::Left,
            "center" => Self::Center,
            "right" => Self::Right,
            _ => Self::None,
        }
    }
}

fn format_separator(width: usize, align: Alignment) -> String {
    let dashes = width.max(3);
    match align {
        Alignment::Left => format!(":{}:", "-".repeat(dashes - 1)),
        Alignment::Center => format!(":{}:", "-".repeat(dashes.saturating_sub(2))),
        Alignment::Right => format!("{}:", "-".repeat(dashes - 1)),
        Alignment::None => format!(" {} ", "-".repeat(dashes)),
    }
}

fn pad_cell(text: &str, width: usize, align: Alignment) -> String {
    let text_len = text.chars().count();
    if text_len >= width {
        return text.to_string();
    }

    let padding = width - text_len;
    match align {
        Alignment::Right => format!("{}{}", " ".repeat(padding), text),
        Alignment::Center => {
            let left = padding / 2;
            let right = padding - left;
            format!("{}{}{}", " ".repeat(left), text, " ".repeat(right))
        }
        _ => format!("{}{}", text, " ".repeat(padding)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // escape_markdown tests
    // ============================================================================

    #[test]
    fn test_escape_asterisks() {
        assert_eq!(escape_markdown("*text*", false), r"\*text\*");
        assert_eq!(escape_markdown("**bold**", false), r"\*\*bold\*\*");
    }

    #[test]
    fn test_escape_underscores() {
        assert_eq!(escape_markdown("_text_", false), r"\_text\_");
        assert_eq!(escape_markdown("my_var_name", false), r"my\_var\_name");
    }

    #[test]
    fn test_escape_brackets() {
        assert_eq!(escape_markdown("[not a link]", false), r"\[not a link\]");
    }

    #[test]
    fn test_escape_backslash() {
        assert_eq!(escape_markdown(r"path\to\file", false), r"path\\to\\file");
    }

    #[test]
    fn test_escape_angle_brackets() {
        assert_eq!(escape_markdown("<html>", false), r"\<html\>");
    }

    #[test]
    fn test_no_escape_in_code_block() {
        assert_eq!(escape_markdown("*text*", true), "*text*");
        assert_eq!(escape_markdown("_var_", true), "_var_");
    }

    #[test]
    fn test_escape_empty_string() {
        assert_eq!(escape_markdown("", false), "");
    }

    #[test]
    fn test_escape_no_special_chars() {
        assert_eq!(escape_markdown("plain text", false), "plain text");
    }

    #[test]
    fn test_escape_mixed_content() {
        assert_eq!(
            escape_markdown("Use *asterisks* and _underscores_", false),
            r"Use \*asterisks\* and \_underscores\_"
        );
    }

    // ============================================================================
    // post_process_markdown tests
    // ============================================================================

    #[test]
    fn test_post_process_preserves_formatting() {
        // Bold and italic should be preserved when they look like formatting
        let input = "This is **bold** and *italic* text.";
        let result = post_process_markdown(input);
        eprintln!("Input:  {input}");
        eprintln!("Result: {result}");
        assert!(result.contains("**bold**"), "Expected **bold** but got: {result}");
        assert!(result.contains("*italic*"), "Expected *italic* but got: {result}");
    }

    #[test]
    fn test_post_process_preserves_code_blocks() {
        let input = "```\n*not escaped*\n```";
        let result = post_process_markdown(input);
        assert!(result.contains("*not escaped*"));
        assert!(!result.contains(r"\*"));
    }

    #[test]
    fn test_post_process_preserves_inline_code() {
        let input = "Use `*asterisks*` in code.";
        let result = post_process_markdown(input);
        assert!(result.contains("`*asterisks*`"));
    }

    #[test]
    fn test_post_process_preserves_headings() {
        let input = "# Heading\n## Subheading";
        let result = post_process_markdown(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_post_process_preserves_lists() {
        let input = "- Item 1\n* Item 2\n+ Item 3";
        let result = post_process_markdown(input);
        assert!(result.contains("- Item 1"));
        assert!(result.contains("* Item 2"));
    }

    // ============================================================================
    // html_table_to_markdown tests
    // ============================================================================

    #[test]
    fn test_simple_table() {
        let html = r#"<table>
            <tr><th>A</th><th>B</th></tr>
            <tr><td>1</td><td>2</td></tr>
        </table>"#;
        let result = html_table_to_markdown(html);
        assert!(result.contains("| A"));
        assert!(result.contains("| B"));
        assert!(result.contains("---"));
        assert!(result.contains("| 1"));
        assert!(result.contains("| 2"));
    }

    #[test]
    fn test_table_with_thead() {
        let html = r#"<table>
            <thead><tr><th>Header A</th><th>Header B</th></tr></thead>
            <tbody><tr><td>Cell 1</td><td>Cell 2</td></tr></tbody>
        </table>"#;
        let result = html_table_to_markdown(html);
        assert!(result.contains("Header A"));
        assert!(result.contains("Header B"));
        assert!(result.contains("Cell 1"));
        assert!(result.contains("Cell 2"));
    }

    #[test]
    fn test_table_alignment_left() {
        let html = r#"<table>
            <tr><th align="left">Left</th></tr>
            <tr><td>Data</td></tr>
        </table>"#;
        let result = html_table_to_markdown(html);
        assert!(result.contains(":--"));
    }

    #[test]
    fn test_table_alignment_center() {
        let html = r#"<table>
            <tr><th align="center">Center</th></tr>
            <tr><td>Data</td></tr>
        </table>"#;
        let result = html_table_to_markdown(html);
        assert!(result.contains(":") && result.contains("-"));
    }

    #[test]
    fn test_table_alignment_right() {
        let html = r#"<table>
            <tr><th align="right">Right</th></tr>
            <tr><td>Data</td></tr>
        </table>"#;
        let result = html_table_to_markdown(html);
        assert!(result.contains("--:"));
    }

    #[test]
    fn test_empty_table() {
        let html = "<table></table>";
        let result = html_table_to_markdown(html);
        assert!(result.is_empty());
    }

    #[test]
    fn test_table_uneven_rows() {
        let html = r#"<table>
            <tr><th>A</th><th>B</th><th>C</th></tr>
            <tr><td>1</td><td>2</td></tr>
        </table>"#;
        let result = html_table_to_markdown(html);
        // Should handle uneven rows without panicking
        assert!(result.contains("| A"));
    }
}
