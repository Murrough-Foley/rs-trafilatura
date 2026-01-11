#!/usr/bin/env python3
"""Analyze precision issues by comparing rs_trafilatura with go_trafilatura output."""

import json
from pathlib import Path
from collections import Counter

def tokenize(text):
    """Simple tokenizer matching the benchmark's shingle approach."""
    if not text:
        return []
    text = ' '.join(text.lower().split())
    return text.split()

def get_shingles(tokens, n=4):
    """Get n-gram shingles from tokens."""
    if len(tokens) < n:
        return set([tuple(tokens)]) if tokens else set()
    return set(tuple(tokens[i:i+n]) for i in range(len(tokens) - n + 1))

def calculate_metrics(true_text, pred_text):
    """Calculate precision, recall, F1 for a single document."""
    true_tokens = tokenize(true_text)
    pred_tokens = tokenize(pred_text)

    true_shingles = get_shingles(true_tokens)
    pred_shingles = get_shingles(pred_tokens)

    if not pred_shingles:
        return {'precision': 0, 'recall': 0, 'f1': 0, 'pred_len': 0, 'true_len': len(true_tokens)}
    if not true_shingles:
        return {'precision': 0, 'recall': 1, 'f1': 0, 'pred_len': len(pred_tokens), 'true_len': 0}

    tp = len(true_shingles & pred_shingles)
    fp = len(pred_shingles - true_shingles)
    fn = len(true_shingles - pred_shingles)

    precision = tp / (tp + fp) if (tp + fp) > 0 else 0
    recall = tp / (tp + fn) if (tp + fn) > 0 else 0
    f1 = 2 * precision * recall / (precision + recall) if (precision + recall) > 0 else 0

    return {
        'precision': precision, 'recall': recall, 'f1': f1,
        'tp': tp, 'fp': fp, 'fn': fn,
        'pred_len': len(pred_tokens), 'true_len': len(true_tokens),
    }

def find_extra_content(true_text, pred_text):
    """Find words in prediction that are not in ground truth."""
    true_tokens = set(tokenize(true_text))
    pred_tokens = tokenize(pred_text)
    return [w for w in pred_tokens if w not in true_tokens]

def main():
    benchmark_dir = Path('benchmarks/article-extraction-benchmark')

    ground_truth = json.loads((benchmark_dir / 'ground-truth.json').read_text())
    rs_output = json.loads((benchmark_dir / 'output/rs_trafilatura.json').read_text())
    go_output = json.loads((benchmark_dir / 'output/go_trafilatura.json').read_text())

    results = []
    for file_id in ground_truth:
        true_text = ground_truth[file_id].get('articleBody', '')
        rs_text = rs_output.get(file_id, {}).get('articleBody', '')
        go_text = go_output.get(file_id, {}).get('articleBody', '')

        rs_m = calculate_metrics(true_text, rs_text)
        go_m = calculate_metrics(true_text, go_text)

        results.append({
            'file_id': file_id,
            'rs_precision': rs_m['precision'], 'rs_recall': rs_m['recall'],
            'go_precision': go_m['precision'], 'go_recall': go_m['recall'],
            'precision_gap': go_m['precision'] - rs_m['precision'],
            'rs_len': rs_m['pred_len'], 'go_len': go_m['pred_len'],
            'true_len': rs_m['true_len'],
        })

    results.sort(key=lambda x: x['precision_gap'], reverse=True)

    print("=" * 80)
    print("TOP 10 WORST PRECISION GAPS (rs vs go_trafilatura)")
    print("=" * 80)
    print(f"{'File ID':<40} {'RS Prec':>8} {'GO Prec':>8} {'Gap':>8} {'RS Len':>8} {'GO Len':>8}")
    print("-" * 80)
    for r in results[:10]:
        print(f"{r['file_id']:<40} {r['rs_precision']:>8.3f} {r['go_precision']:>8.3f} {r['precision_gap']:>8.3f} {r['rs_len']:>8} {r['go_len']:>8}")

    print("\n" + "=" * 80)
    print("COMMON BOILERPLATE IN WORST 20 PRECISION DOCS")
    print("=" * 80)

    all_extra = []
    for r in results[:20]:
        true_text = ground_truth[r['file_id']].get('articleBody', '')
        rs_text = rs_output.get(r['file_id'], {}).get('articleBody', '')
        all_extra.extend(find_extra_content(true_text, rs_text))

    counts = Counter(all_extra)
    print("\nMost common extra words:")
    for word, count in counts.most_common(30):
        if len(word) > 2:
            print(f"  {word}: {count}")

    print("\n" + "=" * 80)
    print("WORST DOC SAMPLE")
    print("=" * 80)
    w = results[0]
    print(f"\nFile: {w['file_id']}")
    print(f"RS Prec: {w['rs_precision']:.3f}, GO Prec: {w['go_precision']:.3f}")
    print(f"RS: {w['rs_len']} words, GO: {w['go_len']} words, Truth: {w['true_len']} words")

    rs_text = rs_output.get(w['file_id'], {}).get('articleBody', '')[:800]
    go_text = go_output.get(w['file_id'], {}).get('articleBody', '')[:800]
    print(f"\nRS first 800 chars:\n{rs_text}")
    print(f"\nGO first 800 chars:\n{go_text}")

if __name__ == '__main__':
    main()
