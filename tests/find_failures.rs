use rs_trafilatura::extract;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Deserialize)]
struct GroundTruthEntry {
    #[serde(rename = "articleBody")]
    article_body: Option<String>,
}

#[test]
fn find_failed_extractions() {
    let ground_truth_path = "tests/benchmark_suite/ground-truth.json";
    let content = fs::read_to_string(ground_truth_path).expect("failed to read ground-truth.json");
    let ground_truth: HashMap<String, GroundTruthEntry> =
        serde_json::from_str(&content).expect("failed to parse ground-truth.json");

    let mut failed = Vec::new();
    let mut succeeded = Vec::new();

    for (file_stem, entry) in &ground_truth {
        if entry.article_body.is_none() {
            continue;
        }

        let html_path = format!("tests/benchmark_suite/{file_stem}.html");
        let html = if let Ok(content) = fs::read_to_string(&html_path) { content } else {
            failed.push((file_stem.clone(), "File not found".to_string()));
            continue;
        };

        match extract(&html) {
            Ok(result) => {
                if result.content_text.trim().is_empty() {
                    failed.push((file_stem.clone(), "Empty content".to_string()));
                } else {
                    succeeded.push(file_stem.clone());
                }
            }
            Err(e) => {
                failed.push((file_stem.clone(), format!("Error: {e:?}")));
            }
        }
    }

    println!("=== Extraction Results ===");
    println!("Succeeded: {}", succeeded.len());
    println!("Failed: {}", failed.len());
    println!("\n=== Failed Files ===");
    for (file, reason) in &failed {
        println!("{file}: {reason}");
    }
}
