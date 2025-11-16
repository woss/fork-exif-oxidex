#!/bin/bash
# Performance test for data.lfs directory
#
# This script benchmarks the exiftool-rs binary against the data.lfs directory
# containing diverse file formats from various camera manufacturers.

set -e

# Configuration
DATA_DIR="/Users/allen/Documents/git/examples/data.lfs"
EXIFTOOL_RS="./target/release/exiftool-rs"

# Verify prerequisites
if [ ! -d "$DATA_DIR" ]; then
    echo "Error: Data directory not found at $DATA_DIR"
    exit 1
fi

if [ ! -f "$EXIFTOOL_RS" ]; then
    echo "Error: exiftool-rs binary not found at $EXIFTOOL_RS"
    echo "Please build in release mode first: cargo build --release"
    exit 1
fi

# Count total files (excluding .git directory)
TOTAL_FILES=$(find "$DATA_DIR" -type f -not -path '*/.git/*' | wc -l | tr -d ' ')

echo "Performance Test - Recursive Processing"
echo "======================================="
echo "Testing directory: $DATA_DIR"
echo "Files to process: $TOTAL_FILES"
echo ""

# Run exiftool-rs performance test
echo "Running exiftool-rs..."
echo ""
time $EXIFTOOL_RS -r "$DATA_DIR" > /tmp/exiftool_rs_perf.txt 2>&1 || true

# Extract statistics from output
echo ""
echo "Results:"
tail -2 /tmp/exiftool_rs_perf.txt

echo ""
echo "Performance Test Complete"
