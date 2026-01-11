//! F-Score calculation for accuracy benchmarking.
//!
//! This module provides F-Score calculation matching the methodology
//! used in content-extractor-benchmark for consistent accuracy metrics.

use std::collections::HashSet;

/// Result of F-Score calculation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FScore {
    /// Precision: TP / (TP + FP)
    pub precision: f64,
    /// Recall: TP / (TP + FN)
    pub recall: f64,
    /// F-Score: harmonic mean of precision and recall
    pub fscore: f64,
}

impl FScore {
    /// Create a new `FScore` result.
    #[must_use]
    pub fn new(precision: f64, recall: f64, fscore: f64) -> Self {
        Self {
            precision,
            recall,
            fscore,
        }
    }

    /// Perfect score (all metrics = 1.0).
    #[must_use]
    pub fn perfect() -> Self {
        Self::new(1.0, 1.0, 1.0)
    }

    /// Zero score (all metrics = 0.0).
    #[must_use]
    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }
}

/// Calculate F-Score between extracted and expected text.
///
/// This uses word-level comparison matching the content-extractor-benchmark methodology:
/// 1. Tokenize text into words (split on whitespace, lowercase)
/// 2. Calculate true positives (words in both sets)
/// 3. Calculate false positives (words in extracted but not expected)
/// 4. Calculate false negatives (words in expected but not extracted)
/// 5. Compute precision, recall, and F-Score
///
/// # Examples
///
/// ```
/// use rs_trafilatura::scoring::calculate_fscore;
///
/// let extracted = "The quick brown fox";
/// let expected = "The quick brown fox";
/// let score = calculate_fscore(extracted, expected);
/// assert_eq!(score.fscore, 1.0);
/// ```
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn calculate_fscore(extracted: &str, expected: &str) -> FScore {
    let extracted_words = tokenize(extracted);
    let expected_words = tokenize(expected);

    // Edge case: both empty
    if expected_words.is_empty() && extracted_words.is_empty() {
        return FScore::perfect();
    }

    // Edge case: one empty
    if expected_words.is_empty() || extracted_words.is_empty() {
        return FScore::zero();
    }

    // Convert to sets for comparison
    let extracted_set: HashSet<_> = extracted_words.iter().collect();
    let expected_set: HashSet<_> = expected_words.iter().collect();

    // Calculate metrics
    let true_positives = extracted_set.intersection(&expected_set).count() as f64;
    let false_positives = extracted_set.difference(&expected_set).count() as f64;
    let false_negatives = expected_set.difference(&extracted_set).count() as f64;

    // Precision: TP / (TP + FP)
    let precision = if true_positives + false_positives > 0.0 {
        true_positives / (true_positives + false_positives)
    } else {
        0.0
    };

    // Recall: TP / (TP + FN)
    let recall = if true_positives + false_negatives > 0.0 {
        true_positives / (true_positives + false_negatives)
    } else {
        0.0
    };

    // F-Score: harmonic mean of precision and recall
    let fscore = if precision + recall > 0.0 {
        2.0 * (precision * recall) / (precision + recall)
    } else {
        0.0
    };

    FScore::new(precision, recall, fscore)
}

/// Tokenize text into words.
///
/// Splits on whitespace, converts to lowercase, and filters out empty strings.
fn tokenize(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(str::to_lowercase)
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_returns_perfect_score() {
        let text = "The quick brown fox jumps over the lazy dog";
        let score = calculate_fscore(text, text);
        assert_eq!(score.precision, 1.0);
        assert_eq!(score.recall, 1.0);
        assert_eq!(score.fscore, 1.0);
    }

    #[test]
    fn partial_match_calculates_correctly() {
        let extracted = "The quick brown fox";
        let expected = "The quick brown fox jumps over the lazy dog";
        let score = calculate_fscore(extracted, expected);

        // All 4 extracted words are in expected: precision = 1.0
        assert_eq!(score.precision, 1.0);

        // 4 out of 8 unique expected words extracted: recall = 4/8 = 0.5
        // (Expected has 8 unique words: {the, quick, brown, fox, jumps, over, lazy, dog})
        assert_eq!(score.recall, 0.5);

        // F-Score = 2 * (1.0 * 0.5) / (1.0 + 0.5) = 2/3 ≈ 0.667
        assert!((score.fscore - 0.667).abs() < 0.01);
    }

    #[test]
    fn no_match_returns_zero_score() {
        let extracted = "completely different text";
        let expected = "The quick brown fox";
        let score = calculate_fscore(extracted, expected);
        assert_eq!(score.precision, 0.0);
        assert_eq!(score.recall, 0.0);
        assert_eq!(score.fscore, 0.0);
    }

    #[test]
    fn empty_extracted_returns_zero() {
        let score = calculate_fscore("", "The quick brown fox");
        assert_eq!(score.precision, 0.0);
        assert_eq!(score.recall, 0.0);
        assert_eq!(score.fscore, 0.0);
    }

    #[test]
    fn empty_expected_returns_zero() {
        let score = calculate_fscore("The quick brown fox", "");
        assert_eq!(score.precision, 0.0);
        assert_eq!(score.recall, 0.0);
        assert_eq!(score.fscore, 0.0);
    }

    #[test]
    fn both_empty_returns_perfect() {
        let score = calculate_fscore("", "");
        assert_eq!(score.precision, 1.0);
        assert_eq!(score.recall, 1.0);
        assert_eq!(score.fscore, 1.0);
    }

    #[test]
    fn case_insensitive_matching() {
        let extracted = "THE QUICK BROWN FOX";
        let expected = "the quick brown fox";
        let score = calculate_fscore(extracted, expected);
        assert_eq!(score.precision, 1.0);
        assert_eq!(score.recall, 1.0);
        assert_eq!(score.fscore, 1.0);
    }

    #[test]
    fn whitespace_normalization() {
        let extracted = "The   quick\tbrown\nfox";
        let expected = "The quick brown fox";
        let score = calculate_fscore(extracted, expected);
        assert_eq!(score.precision, 1.0);
        assert_eq!(score.recall, 1.0);
        assert_eq!(score.fscore, 1.0);
    }

    #[test]
    fn duplicate_words_counted_once() {
        let extracted = "fox fox fox fox";
        let expected = "fox";
        let score = calculate_fscore(extracted, expected);
        // Only unique words matter in set comparison
        assert_eq!(score.precision, 1.0);
        assert_eq!(score.recall, 1.0);
        assert_eq!(score.fscore, 1.0);
    }

    #[test]
    fn tokenize_splits_and_lowercases() {
        let tokens = tokenize("The QUICK Brown Fox");
        assert_eq!(tokens, vec!["the", "quick", "brown", "fox"]);
    }

    #[test]
    fn tokenize_handles_multiple_whitespace() {
        let tokens = tokenize("  The   quick  \t  brown\n\nfox  ");
        assert_eq!(tokens, vec!["the", "quick", "brown", "fox"]);
    }

    #[test]
    fn tokenize_empty_string() {
        let tokens = tokenize("");
        assert!(tokens.is_empty());
    }
}
