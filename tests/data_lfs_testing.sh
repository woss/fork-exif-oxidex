#!/bin/bash
# Comprehensive test script for data.lfs directory

DATA_DIR="/Users/allen/Documents/git/examples/data.lfs"
EXIFTOOL_RS="./target/release/oxidex"
ERROR_LOG="tests/data_lfs_errors.log"
SUCCESS_LOG="tests/data_lfs_success.log"

# Clear previous logs
> "$ERROR_LOG"
> "$SUCCESS_LOG"

# Counters
total=0
success=0
errors=0

echo "Starting comprehensive test of data.lfs directory..."
echo "Total files to test: $(find "$DATA_DIR" -type f | wc -l)"
echo ""

# Process all files
find "$DATA_DIR" -type f | while read -r file; do
    total=$((total + 1))

    # Test the file
    if $EXIFTOOL_RS "$file" > /dev/null 2>&1; then
        success=$((success + 1))
        echo "$file" >> "$SUCCESS_LOG"
    else
        errors=$((errors + 1))
        echo "=== ERROR: $file ===" >> "$ERROR_LOG"
        $EXIFTOOL_RS "$file" 2>&1 >> "$ERROR_LOG"
        echo "" >> "$ERROR_LOG"
    fi

    # Progress indicator every 100 files
    if [ $((total % 100)) -eq 0 ]; then
        echo "Processed: $total files (Success: $success, Errors: $errors)"
    fi
done

echo ""
echo "Testing complete!"
echo "Total: $total"
echo "Success: $success"
echo "Errors: $errors"
echo ""
echo "Error details saved to: $ERROR_LOG"
echo "Success list saved to: $SUCCESS_LOG"
