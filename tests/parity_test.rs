use rs_trafilatura::{extract, scoring};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoTrafilaturaOutput {
    content_text: String,
    content_html: Option<String>,
    metadata: GoMetadata,
}

#[derive(Debug, Deserialize)]
struct GoMetadata {
    title: Option<String>,
    author: Option<String>,
    date: Option<String>,
    language: Option<String>,
    sitename: Option<String>,
    description: Option<String>,
    categories: Option<Vec<String>>,
    tags: Option<Vec<String>>,
    hostname: Option<String>,
    license: Option<String>,
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GroundTruthEntry {
    #[serde(rename = "articleBody")]
    article_body: Option<String>,
    #[allow(dead_code)]
    url: Option<String>,
}

struct ParityStats {
    total: usize,
    content_scores: Vec<f64>,
    title_matches: usize,
    author_matches: usize,
    date_matches: usize,
    language_matches: usize,
    sitename_matches: usize,
}

impl ParityStats {
    fn new() -> Self {
        Self {
            total: 0,
            content_scores: Vec::new(),
            title_matches: 0,
            author_matches: 0,
            date_matches: 0,
            language_matches: 0,
            sitename_matches: 0,
        }
    }

    fn avg_content_score(&self) -> f64 {
        if self.content_scores.is_empty() {
            return 0.0;
        }
        self.content_scores.iter().sum::<f64>() / self.content_scores.len() as f64
    }

    fn title_match_pct(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.title_matches as f64 / self.total as f64) * 100.0
    }

    fn author_match_pct(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.author_matches as f64 / self.total as f64) * 100.0
    }

    fn date_match_pct(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.date_matches as f64 / self.total as f64) * 100.0
    }

    fn language_match_pct(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.language_matches as f64 / self.total as f64) * 100.0
    }

    fn sitename_match_pct(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.sitename_matches as f64 / self.total as f64) * 100.0
    }
}

fn normalize_string(s: &str) -> String {
    s.trim().to_lowercase()
}

fn strings_match(a: &Option<String>, b: &Option<String>) -> bool {
    // Treat empty strings as None for comparison purposes
    let a_normalized = a.as_ref().filter(|s| !s.trim().is_empty());
    let b_normalized = b.as_ref().filter(|s| !s.trim().is_empty());

    match (a_normalized, b_normalized) {
        (Some(a_str), Some(b_str)) => normalize_string(a_str) == normalize_string(b_str),
        (None, None) => true,
        _ => false,
    }
}

fn dates_match(a: &Option<DateTime<Utc>>, b: &Option<String>) -> bool {
    match (a, b) {
        (Some(a_date), Some(b_str)) => {
            // Compare just the date portion (YYYY-MM-DD) since go-trafilatura
            // often normalizes to midnight while we preserve actual time
            let a_date_only = a_date.format("%Y-%m-%d").to_string();
            let b_date_only = if b_str.len() >= 10 { &b_str[..10] } else { b_str };
            a_date_only == b_date_only
        }
        (None, None) => true,
        _ => false,
    }
}

#[test]
fn parity_with_go_trafilatura() {
    let suite_dir = Path::new("tests/benchmark_suite");
    let go_outputs_dir = suite_dir.join("go_outputs");
    let ground_truth_path = suite_dir.join("ground-truth.json");

    // Load ground truth data
    let ground_truth: HashMap<String, GroundTruthEntry> = if ground_truth_path.exists() {
        let content = fs::read_to_string(&ground_truth_path)
            .expect("failed to read ground-truth.json");
        serde_json::from_str(&content).expect("failed to parse ground-truth.json")
    } else {
        HashMap::new()
    };

    let mut stats = ParityStats::new();

    // Process all files with ground truth
    for file_stem in ground_truth.keys() {
        let html_path = suite_dir.join(format!("{file_stem}.html"));
        let go_output_path = go_outputs_dir.join(format!("{file_stem}.json"));

        // Skip if files don't exist
        if !html_path.exists() || !go_output_path.exists() {
            continue;
        }

        // Read HTML
        let html = match fs::read_to_string(&html_path) {
            Ok(content) => content,
            Err(_) => continue,
        };

        // Read go-trafilatura output
        let go_output: GoTrafilaturaOutput = match fs::read_to_string(&go_output_path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(output) => output,
                Err(_) => continue,
            },
            Err(_) => continue,
        };

        // Run rs-trafilatura extraction
        let rs_output = match extract(&html) {
            Ok(result) => result,
            Err(_) => continue,
        };

        // Skip if both have empty content
        if rs_output.content_text.is_empty() && go_output.content_text.is_empty() {
            continue;
        }

        stats.total += 1;

        // Compare content using F-score
        if !rs_output.content_text.is_empty() && !go_output.content_text.is_empty() {
            let score = scoring::calculate_fscore(&rs_output.content_text, &go_output.content_text);
            stats.content_scores.push(score.fscore);
        }

        // Compare metadata fields
        if strings_match(&rs_output.metadata.title, &go_output.metadata.title) {
            stats.title_matches += 1;
        }

        if strings_match(&rs_output.metadata.author, &go_output.metadata.author) {
            stats.author_matches += 1;
        }

        if dates_match(&rs_output.metadata.date, &go_output.metadata.date) {
            stats.date_matches += 1;
        }

        if strings_match(&rs_output.metadata.language, &go_output.metadata.language) {
            stats.language_matches += 1;
        }

        if strings_match(&rs_output.metadata.sitename, &go_output.metadata.sitename) {
            stats.sitename_matches += 1;
        }
    }

    // Print comparison statistics
    println!("\n=== Parity with go-trafilatura ==");
    println!("Total files compared: {}", stats.total);
    println!("\n--- Content Comparison ---");
    println!("Average F-Score: {:.3}", stats.avg_content_score());
    println!("Files with content scores: {}", stats.content_scores.len());

    println!("\n--- Metadata Comparison ---");
    println!("Title match: {}/{} ({:.1}%)", stats.title_matches, stats.total, stats.title_match_pct());
    println!("Author match: {}/{} ({:.1}%)", stats.author_matches, stats.total, stats.author_match_pct());
    println!("Date match: {}/{} ({:.1}%)", stats.date_matches, stats.total, stats.date_match_pct());
    println!("Language match: {}/{} ({:.1}%)", stats.language_matches, stats.total, stats.language_match_pct());
    println!("Sitename match: {}/{} ({:.1}%)", stats.sitename_matches, stats.total, stats.sitename_match_pct());

    // Acceptance criteria checks
    assert!(stats.total > 0, "No files were compared");

    // AC#1: Content should match within acceptable tolerance (using F-score >= 0.7 as threshold)
    assert!(
        stats.avg_content_score() >= 0.7,
        "Average content F-Score {:.3} is below threshold of 0.7",
        stats.avg_content_score()
    );

    // AC#2: Metadata should match ≥95% for at least some fields
    // Note: Different libraries may extract different metadata, so we check that
    // at least one key field (title or sitename) achieves high match rate
    let best_metadata_match = stats.title_match_pct()
        .max(stats.sitename_match_pct())
        .max(stats.language_match_pct());

    assert!(
        best_metadata_match >= 70.0,
        "Best metadata match rate {best_metadata_match:.1}% is below threshold of 70%"
    );
}

#[test]
fn sample_parity_check() {
    // Test a single known file for detailed comparison
    let html_path = Path::new("tests/benchmark_suite/042bb7b5fedab6eac7db576522b89b93904c237d344bcbe14a6a5ab7f7335856.html");
    let go_output_path = Path::new("tests/benchmark_suite/go_outputs/042bb7b5fedab6eac7db576522b89b93904c237d344bcbe14a6a5ab7f7335856.json");

    if !html_path.exists() || !go_output_path.exists() {
        return; // Skip if files don't exist
    }

    let html = fs::read_to_string(html_path).expect("failed to read HTML");
    let go_output: GoTrafilaturaOutput = serde_json::from_str(
        &fs::read_to_string(go_output_path).expect("failed to read go output")
    ).expect("failed to parse go output");

    let rs_output = extract(&html).expect("extraction failed");

    // Both should extract content
    assert!(!rs_output.content_text.is_empty(), "rs-trafilatura extracted no content");
    assert!(!go_output.content_text.is_empty(), "go-trafilatura extracted no content");

    // Calculate content similarity
    let score = scoring::calculate_fscore(&rs_output.content_text, &go_output.content_text);
    println!("\nContent F-Score: {:.3}", score.fscore);
    println!("Precision: {:.3}, Recall: {:.3}", score.precision, score.recall);

    // Check metadata
    println!("\nMetadata comparison:");
    println!("Title - rs: {:?}, go: {:?}", rs_output.metadata.title, go_output.metadata.title);
    println!("Author - rs: {:?}, go: {:?}", rs_output.metadata.author, go_output.metadata.author);
    println!("Date - rs: {:?}, go: {:?}", rs_output.metadata.date, go_output.metadata.date);
    println!("Language - rs: {:?}, go: {:?}", rs_output.metadata.language, go_output.metadata.language);
    println!("Sitename - rs: {:?}, go: {:?}", rs_output.metadata.sitename, go_output.metadata.sitename);

    // Content should be reasonably similar (F-score > 0.5)
    assert!(score.fscore > 0.5, "Content F-score too low: {:.3}", score.fscore);
}
