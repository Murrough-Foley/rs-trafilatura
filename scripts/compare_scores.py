#!/usr/bin/env python3
"""Compare rs_trafilatura vs go_trafilatura per-file scores to identify problem files."""

import json
import re
from collections import Counter
from pathlib import Path
from typing import Dict, List, Tuple

def load_json(path: Path) -> Dict:
    with path.open('rt', encoding='utf8') as f:
        return json.load(f)

_TOKEN_RE = re.compile(r'\w+', re.UNICODE | re.MULTILINE | re.IGNORECASE | re.DOTALL)

def tokenize(text: str) -> List[str]:
    return _TOKEN_RE.findall(text or '')

def string_shingle_matching(true: str, pred: str, ngram_n: int = 4) -> Tuple[float, float, float]:
    """Compute TP/FP/FN across shingles."""
    true_tokens = tokenize(true)
    pred_tokens = tokenize(pred)
    
    def all_shingles(tokens, n):
        result = []
        for i in range(0, max(1, len(tokens) - n + 1)):
            shingle = tuple(tokens[i: i + n])
            if shingle:
                result.append(shingle)
        return dict(Counter(result))
    
    true_shingles = all_shingles(true_tokens, ngram_n)
    pred_shingles = all_shingles(pred_tokens, ngram_n)
    
    tp = fp = fn = 0.0
    for key in (set(true_shingles) | set(pred_shingles)):
        true_count = true_shingles.get(key, 0)
        pred_count = pred_shingles.get(key, 0)
        tp += min(true_count, pred_count)
        fp += max(0, pred_count - true_count)
        fn += max(0, true_count - pred_count)
    
    s = tp + fp + fn
    if s > 0:
        tp, fp, fn = tp / s, fp / s, fn / s
    return tp, fp, fn

def f1_score(tp, fp, fn):
    precision = tp / (tp + fp) if (tp + fp) > 0 else 0
    recall = tp / (tp + fn) if (tp + fn) > 0 else 0
    if precision + recall == 0:
        return 0
    return 2 * precision * recall / (precision + recall)

def main():
    base = Path(__file__).parent.parent / 'benchmarks' / 'article-extraction-benchmark'
    
    ground_truth = load_json(base / 'ground-truth.json')
    rs_output = load_json(base / 'output' / 'rs_trafilatura.json')
    go_output = load_json(base / 'output' / 'go_trafilatura.json')
    
    results = []
    
    for key in ground_truth.keys():
        true_text = ground_truth[key].get('articleBody', '')
        rs_text = rs_output[key].get('articleBody', '')
        go_text = go_output[key].get('articleBody', '')
        
        rs_tp, rs_fp, rs_fn = string_shingle_matching(true_text, rs_text)
        go_tp, go_fp, go_fn = string_shingle_matching(true_text, go_text)
        
        rs_f1 = f1_score(rs_tp, rs_fp, rs_fn)
        go_f1 = f1_score(go_tp, go_fp, go_fn)
        
        rs_precision = rs_tp / (rs_tp + rs_fp) if (rs_tp + rs_fp) > 0 else 0
        rs_recall = rs_tp / (rs_tp + rs_fn) if (rs_tp + rs_fn) > 0 else 0
        
        go_precision = go_tp / (go_tp + go_fp) if (go_tp + go_fp) > 0 else 0
        go_recall = go_tp / (go_tp + go_fn) if (go_tp + go_fn) > 0 else 0
        
        results.append({
            'file': key,
            'rs_f1': rs_f1,
            'go_f1': go_f1,
            'f1_gap': go_f1 - rs_f1,
            'rs_precision': rs_precision,
            'rs_recall': rs_recall,
            'go_precision': go_precision,
            'go_recall': go_recall,
            'true_len': len(true_text),
            'rs_len': len(rs_text),
            'go_len': len(go_text),
        })
    
    # Sort by F1 gap (worst first)
    results.sort(key=lambda x: -x['f1_gap'])
    
    print("=" * 100)
    print("WORST PERFORMING FILES (rs_trafilatura vs go_trafilatura)")
    print("=" * 100)
    print(f"{'File':<50} {'RS F1':>8} {'GO F1':>8} {'Gap':>8} {'RS Prec':>8} {'RS Rec':>8}")
    print("-" * 100)
    
    for r in results[:30]:  # Top 30 worst
        print(f"{r['file']:<50} {r['rs_f1']:>8.3f} {r['go_f1']:>8.3f} {r['f1_gap']:>8.3f} {r['rs_precision']:>8.3f} {r['rs_recall']:>8.3f}")
    
    print("\n" + "=" * 100)
    print("SUMMARY STATISTICS")
    print("=" * 100)
    
    # Files where rs is significantly worse
    bad_files = [r for r in results if r['f1_gap'] > 0.1]
    print(f"Files with F1 gap > 0.1: {len(bad_files)}")
    
    # Categorize issues
    precision_issues = [r for r in results if r['rs_precision'] < r['go_precision'] - 0.1]
    recall_issues = [r for r in results if r['rs_recall'] < r['go_recall'] - 0.1]
    
    print(f"Files with precision deficit > 0.1: {len(precision_issues)}")
    print(f"Files with recall deficit > 0.1: {len(recall_issues)}")
    
    # Files where rs extracts nothing
    empty_rs = [r for r in results if r['rs_len'] == 0]
    print(f"Files where rs_trafilatura extracts nothing: {len(empty_rs)}")
    if empty_rs:
        print("  Empty files:", [r['file'] for r in empty_rs[:10]])
    
    # Files where rs extracts way more than go (precision issue)
    over_extract = [r for r in results if r['rs_len'] > r['go_len'] * 2 and r['go_len'] > 100]
    print(f"Files where rs extracts >2x go length: {len(over_extract)}")
    
    # Save detailed results
    output_path = Path(__file__).parent / 'comparison_results.json'
    with open(output_path, 'w') as f:
        json.dump(results, f, indent=2)
    print(f"\nDetailed results saved to: {output_path}")

if __name__ == '__main__':
    main()
