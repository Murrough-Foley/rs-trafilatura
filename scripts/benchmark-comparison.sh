#!/bin/bash
# Benchmark comparison: rs-trafilatura vs go-trafilatura
#
# Usage: ./scripts/benchmark-comparison.sh
#
# Compares speed and memory usage between Rust and Go implementations
# using the article-extraction-benchmark corpus.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BENCHMARK_DIR="$PROJECT_ROOT/benchmarks/article-extraction-benchmark"
GO_TRAFILATURA_DIR="$PROJECT_ROOT/benchmarks/go-trafilatura"
HTML_DIR="$BENCHMARK_DIR/html"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== rs-trafilatura vs go-trafilatura Benchmark ===${NC}"
echo ""

# Check prerequisites
if [ ! -d "$HTML_DIR" ]; then
    echo -e "${RED}Error: HTML directory not found at $HTML_DIR${NC}"
    echo "Make sure the article-extraction-benchmark submodule is initialized."
    exit 1
fi

HTML_COUNT=$(find "$HTML_DIR" -name "*.html.gz" 2>/dev/null | wc -l)
echo "Found $HTML_COUNT HTML files in benchmark corpus"
echo ""

# =============================================================================
# Build both implementations
# =============================================================================

echo -e "${BLUE}Building implementations...${NC}"

# Build Rust (release mode)
echo "  Building rs-trafilatura (release)..."
cd "$PROJECT_ROOT"
cargo build --release --example benchmark_extract 2>/dev/null
RUST_BIN="$PROJECT_ROOT/target/release/examples/benchmark_extract"

# Build Go
echo "  Building go-trafilatura..."
cd "$GO_TRAFILATURA_DIR"
go build -o "$PROJECT_ROOT/target/go-trafilatura-bench" ./cmd/go-trafilatura 2>/dev/null || {
    echo -e "${RED}  Go build failed. Creating simple benchmark binary...${NC}"

    # Create a simple Go benchmark program
    cat > "/tmp/go-bench.go" << 'GOEOF'
package main

import (
    "compress/gzip"
    "fmt"
    "io"
    "os"
    "path/filepath"
    "time"

    "github.com/markusmobius/go-trafilatura"
)

func main() {
    if len(os.Args) < 2 {
        fmt.Println("Usage: go-bench <html-directory>")
        os.Exit(1)
    }

    htmlDir := os.Args[1]

    var files []string
    filepath.Walk(htmlDir, func(path string, info os.FileInfo, err error) error {
        if err == nil && filepath.Ext(path) == ".gz" {
            files = append(files, path)
        }
        return nil
    })

    fmt.Printf("Processing %d files...\n", len(files))

    start := time.Now()
    successCount := 0

    for _, file := range files {
        f, err := os.Open(file)
        if err != nil {
            continue
        }

        gz, err := gzip.NewReader(f)
        if err != nil {
            f.Close()
            continue
        }

        html, err := io.ReadAll(gz)
        gz.Close()
        f.Close()

        if err != nil {
            continue
        }

        opts := trafilatura.Options{
            IncludeImages: false,
            IncludeLinks:  false,
        }

        result, err := trafilatura.Extract(html, opts)
        if err == nil && result != nil && len(result.ContentText) > 0 {
            successCount++
        }
    }

    elapsed := time.Since(start)

    fmt.Printf("\nResults:\n")
    fmt.Printf("  Files processed: %d\n", len(files))
    fmt.Printf("  Successful: %d\n", successCount)
    fmt.Printf("  Total time: %v\n", elapsed)
    fmt.Printf("  Average: %.2f ms/file\n", float64(elapsed.Milliseconds())/float64(len(files)))
    fmt.Printf("  Throughput: %.1f files/sec\n", float64(len(files))/elapsed.Seconds())
}
GOEOF

    cd "$GO_TRAFILATURA_DIR"
    go build -o "$PROJECT_ROOT/target/go-trafilatura-bench" /tmp/go-bench.go 2>/dev/null || {
        echo -e "${RED}  Could not build Go benchmark. Skipping Go comparison.${NC}"
        GO_AVAILABLE=false
    }
}

GO_BIN="$PROJECT_ROOT/target/go-trafilatura-bench"
GO_AVAILABLE=true

if [ ! -f "$GO_BIN" ]; then
    GO_AVAILABLE=false
fi

cd "$PROJECT_ROOT"
echo ""

# =============================================================================
# Run Rust benchmark
# =============================================================================

echo -e "${BLUE}Running Rust benchmark...${NC}"
echo ""

# Use /usr/bin/time for memory measurement
if command -v /usr/bin/time &> /dev/null; then
    /usr/bin/time -v "$RUST_BIN" 2>&1 | tee /tmp/rust_bench.txt

    # Extract metrics
    RUST_TIME=$(grep "Elapsed (wall clock)" /tmp/rust_bench.txt | awk '{print $8}')
    RUST_MEM=$(grep "Maximum resident set size" /tmp/rust_bench.txt | awk '{print $6}')
    RUST_MEM_MB=$(echo "scale=2; $RUST_MEM / 1024" | bc)
else
    "$RUST_BIN"
    RUST_TIME="N/A"
    RUST_MEM_MB="N/A"
fi

echo ""

# =============================================================================
# Run Go benchmark (if available)
# =============================================================================

if [ "$GO_AVAILABLE" = true ]; then
    echo -e "${BLUE}Running Go benchmark...${NC}"
    echo ""

    if command -v /usr/bin/time &> /dev/null; then
        /usr/bin/time -v "$GO_BIN" "$HTML_DIR" 2>&1 | tee /tmp/go_bench.txt

        GO_TIME=$(grep "Elapsed (wall clock)" /tmp/go_bench.txt | awk '{print $8}')
        GO_MEM=$(grep "Maximum resident set size" /tmp/go_bench.txt | awk '{print $6}')
        GO_MEM_MB=$(echo "scale=2; $GO_MEM / 1024" | bc)
    else
        "$GO_BIN" "$HTML_DIR"
        GO_TIME="N/A"
        GO_MEM_MB="N/A"
    fi

    echo ""
fi

# =============================================================================
# Summary
# =============================================================================

echo -e "${BLUE}=== Summary ===${NC}"
echo ""
echo "Rust (rs-trafilatura):"
echo "  Wall time: $RUST_TIME"
echo "  Peak memory: ${RUST_MEM_MB} MB"
echo ""

if [ "$GO_AVAILABLE" = true ]; then
    echo "Go (go-trafilatura):"
    echo "  Wall time: $GO_TIME"
    echo "  Peak memory: ${GO_MEM_MB} MB"
    echo ""
fi

echo -e "${GREEN}Benchmark complete!${NC}"
