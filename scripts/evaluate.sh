#!/bin/bash
# scripts/evaluate.sh
#
# Runs the full evaluation workflow:
# 1. Builds and runs benchmark_extract to generate extraction output
# 2. Runs evaluate.py to compute F1/Precision/Recall metrics
#
# Usage: ./scripts/evaluate.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "=== rs-trafilatura Evaluation ==="
echo ""

# Step 1: Generate extraction output
echo "Step 1: Running benchmark extraction..."
cd "$PROJECT_ROOT"
cargo run --release --example benchmark_extract

# Step 2: Run evaluation
echo ""
echo "Step 2: Running evaluation..."
cd "$PROJECT_ROOT/benchmarks/article-extraction-benchmark"

if [ ! -f "evaluate.py" ]; then
    echo "Error: evaluate.py not found in benchmarks/article-extraction-benchmark/"
    echo "Make sure the benchmark submodule is initialized."
    exit 1
fi

python evaluate.py

echo ""
echo "=== Evaluation Complete ==="
