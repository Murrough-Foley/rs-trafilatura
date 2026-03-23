//! Batch HTML-to-Markdown extractor.
//! Reads all .html files from an input directory, extracts main content,
//! and writes .md files to an output directory.
//!
//! Usage: batch_markdown <input_dir> <output_dir>

use rs_trafilatura::{extract_with_options, Options};
use std::fs;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: batch_markdown <input_dir> <output_dir>");
        std::process::exit(1);
    }

    let input_dir = PathBuf::from(&args[1]);
    let output_dir = PathBuf::from(&args[2]);

    if !input_dir.is_dir() {
        eprintln!("Input directory does not exist: {}", input_dir.display());
        std::process::exit(1);
    }

    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    let options = Options {
        output_markdown: true,
        include_tables: true,
        include_links: true,
        include_formatting: true,
        ..Options::default()
    };

    let mut entries: Vec<_> = fs::read_dir(&input_dir)
        .expect("Failed to read input directory")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "html")
                .unwrap_or(false)
        })
        .collect();

    entries.sort_by_key(|e| e.file_name());

    let total = entries.len();
    let mut success = 0;
    let mut failed = 0;
    let mut empty = 0;

    for (i, entry) in entries.iter().enumerate() {
        let path = entry.path();
        let stem = path.file_stem().unwrap().to_string_lossy();
        let out_path = output_dir.join(format!("{}.md", stem));

        let html = match fs::read_to_string(&path) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("[{}/{}] ERROR reading {}: {}", i + 1, total, stem, e);
                failed += 1;
                continue;
            }
        };

        match extract_with_options(&html, &options) {
            Ok(result) => {
                // Prefer markdown, fall back to plain text
                let content = result
                    .content_markdown
                    .unwrap_or(result.content_text);

                if content.trim().is_empty() {
                    eprintln!(
                        "[{}/{}] EMPTY: {} (confidence: {:.2})",
                        i + 1,
                        total,
                        stem,
                        result.extraction_quality
                    );
                    empty += 1;
                    continue;
                }

                // Build markdown with frontmatter
                let mut md = String::new();
                md.push_str("---\n");
                if let Some(ref title) = result.metadata.title {
                    md.push_str(&format!("title: \"{}\"\n", title.replace('"', "\\\"")));
                }
                if let Some(ref author) = result.metadata.author {
                    md.push_str(&format!("author: \"{}\"\n", author.replace('"', "\\\"")));
                }
                if let Some(ref date) = result.metadata.date {
                    md.push_str(&format!("date: \"{}\"\n", date.to_rfc3339()));
                }
                md.push_str(&format!("source_file: \"{}\"\n", path.file_name().unwrap().to_string_lossy()));
                md.push_str(&format!("confidence: {:.2}\n", result.extraction_quality));
                if let Some(ref pt) = result.metadata.page_type {
                    md.push_str(&format!("page_type: \"{}\"\n", pt));
                }
                md.push_str("---\n\n");
                md.push_str(&content);

                fs::write(&out_path, &md).expect("Failed to write markdown file");
                println!(
                    "[{}/{}] OK: {}.md ({} chars, confidence: {:.2})",
                    i + 1,
                    total,
                    stem,
                    content.len(),
                    result.extraction_quality
                );
                success += 1;
            }
            Err(e) => {
                eprintln!("[{}/{}] EXTRACT ERROR {}: {}", i + 1, total, stem, e);
                failed += 1;
            }
        }
    }

    println!(
        "\nDone: {} success, {} empty, {} failed (of {} total)",
        success, empty, failed, total
    );
}
