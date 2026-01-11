//! Diagnostic test to analyze author extraction differences between rs and go trafilatura

use rs_trafilatura::extract;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoTrafilaturaOutput {
    metadata: GoMetadata,
}

#[derive(Debug, Deserialize)]
struct GoMetadata {
    author: Option<String>,
}

fn normalize_author(s: &str) -> String {
    s.trim().to_lowercase()
}

#[test]
fn analyze_author_differences() {
    let suite_dir = Path::new("tests/benchmark_suite");
    let go_outputs_dir = suite_dir.join("go_outputs");

    let mut total = 0;
    let mut exact_match = 0;
    let mut both_have = 0;
    let mut rs_only = 0;
    let mut go_only = 0;
    let mut neither = 0;

    let mut mismatch_samples: Vec<(String, String, String)> = Vec::new();
    let mut go_only_samples: Vec<(String, String)> = Vec::new();

    let go_files: Vec<_> = fs::read_dir(&go_outputs_dir)
        .expect("failed to read go_outputs directory")
        .filter_map(std::result::Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();

    for entry in &go_files {
        let file_stem = entry
            .path()
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let html_path = suite_dir.join(format!("{file_stem}.html"));

        if !html_path.exists() {
            continue;
        }

        let html = match fs::read_to_string(&html_path) {
            Ok(h) => h,
            Err(_) => continue,
        };

        let go_content = match fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let go_output: GoTrafilaturaOutput = match serde_json::from_str(&go_content) {
            Ok(o) => o,
            Err(_) => continue,
        };

        let rs_result = match extract(&html) {
            Ok(r) => r,
            Err(_) => continue,
        };

        total += 1;

        let rs_author = rs_result.metadata.author;
        let go_author = go_output.metadata.author;

        match (&rs_author, &go_author) {
            (Some(rs), Some(go)) => {
                both_have += 1;
                if normalize_author(rs) == normalize_author(go) {
                    exact_match += 1;
                } else if mismatch_samples.len() < 20 {
                    mismatch_samples.push((file_stem.clone(), rs.clone(), go.clone()));
                }
            }
            (Some(_), None) => rs_only += 1,
            (None, Some(go)) => {
                go_only += 1;
                if go_only_samples.len() < 10 {
                    go_only_samples.push((file_stem.clone(), go.clone()));
                }
            }
            (None, None) => neither += 1,
        }
    }

    println!("\n=== Author Extraction Analysis ===");
    println!("Total files: {total}");
    println!("Both have author: {both_have}");
    println!("  - Exact match: {} ({:.1}%)", exact_match, f64::from(exact_match) / f64::from(total) * 100.0);
    println!("  - Mismatch: {}", both_have - exact_match);
    println!("rs-trafilatura only: {rs_only}");
    println!("go-trafilatura only: {go_only}");
    println!("Neither has author: {neither}");

    println!("\n--- Mismatch Samples ---");
    for (file, rs, go) in &mismatch_samples {
        println!("\nFile: {}...", &file[..file.len().min(25)]);
        println!("  rs: \"{rs}\"");
        println!("  go: \"{go}\"");
    }

    println!("\n--- go-only Samples (we're missing) ---");
    for (file, go) in &go_only_samples {
        println!("\nFile: {}...", &file[..file.len().min(25)]);
        println!("  go: \"{go}\"");
    }
}
