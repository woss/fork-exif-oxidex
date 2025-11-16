# Camera Raw Test Fixtures

This directory contains minimal sample files for testing raw format support.

## Sample Files

Due to licensing and size constraints, we use minimal synthetic test files:

- `sample.dng` - Adobe DNG (TIFF-based)
- `sample-cr2-header.bin` - Canon CR2 header (first 4KB)
- `sample-nef-header.bin` - Nikon NEF header (first 4KB)

## Creating Test Files

For testing with real raw files:

1. Use your own camera raw files
2. Download sample files from camera manufacturers
3. Use ExifTool's sample files (https://exiftool.org/sample_images.html)

## Synthetic Test Files

The minimal test files contain:
- Valid magic bytes
- Minimal TIFF header structure
- Basic EXIF tags (Make, Model, DateTime)
- No actual image data (to keep repository size small)
