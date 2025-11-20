# Refactoring Plan: src/core/operations.rs

## Current State
- **Complexity**: 204
- **Grade**: C (67)
- **Lines of Code**: 2054
- **Issues**: File is too large with multiple responsibilities

## Complexity Hotspots

### 1. Format Parser Dispatch (Lines 444-522)
- Large match statement with 40+ arms
- Each format requires different parsing logic
- **Complexity contribution**: ~50

### 2. JPEG Metadata Parsing (Lines 559-858)
- Multiple segment processors (JFIF, EXIF, XMP, IPTC, ICC)
- Complex IFD parsing and tag extraction
- **Complexity contribution**: ~60

### 3. TIFF Metadata Parsing (Lines 877-1192)
- IFD chain traversal
- Sub-IFD processing (EXIF, GPS, MakerNote)
- **Complexity contribution**: ~50

### 4. Tag Value Conversion (Lines 1283-1754)
- Complex type detection heuristics
- Special handlers for GPS, DateTime, Rational types
- Multiple branch points for different field types
- **Complexity contribution**: ~40

## Refactoring Strategy

### Phase 1: Extract Format Dispatch Module
**File**: `src/core/format_dispatch.rs`
- Move `dispatch_format_parser()` function
- Move `convert_string_error()` helper
- **Complexity reduction**: ~50

### Phase 2: Extract JPEG Parser Module
**File**: `src/core/parsers/jpeg_operations.rs`
- Move `parse_jpeg_metadata()` and all segment processors
- Move `process_jfif_segments()`
- Move `process_exif_segments()` and `process_ifd0_tags()`
- Move `process_xmp_segments()`
- Move `process_iptc_segments()`
- Move `process_icc_segments()`
- **Complexity reduction**: ~60

### Phase 3: Extract TIFF Parser Module
**File**: `src/core/parsers/tiff_operations.rs`
- Move `parse_tiff_metadata()` and IFD chain processing
- Move `parse_ifd_chain()`
- Move `process_tiff_ifd_tags()`
- Move `parse_exif_subifd()`, `parse_gps_subifd()`
- Move `parse_makernote_if_canon()`
- Move `parse_casio_cam_metadata()`
- **Complexity reduction**: ~50

### Phase 4: Extract Tag Conversion Module
**File**: `src/core/tag_conversion.rs`
- Move `raw_bytes_to_tag_value()` and all type handlers
- Move `handle_special_byte_tags()`
- Move `handle_rational_type()`, `handle_srational_type()`
- Move `handle_short_type()`, `handle_long_type()`, `handle_ascii_type()`
- Move `heuristic_bytes_to_tag_value()`
- Move all utility functions (read_u16, read_u32, read_i32, gcd, etc.)
- **Complexity reduction**: ~40

### Phase 5: Keep Core Operations Clean
**File**: `src/core/operations.rs` (refactored)
- Keep public API functions: `read_metadata()`, `write_metadata()`, `modify_tag()`, `copy_metadata()`
- Import from extracted modules
- **Final complexity**: ~20-30

## Expected Outcome
- **operations.rs complexity**: 204 → ~25 (87% reduction)
- **operations.rs lines**: 2054 → ~400 (80% reduction)
- **operations.rs grade**: C → A
- **New modules created**: 4 focused modules with complexity < 50 each
- **Maintainability**: Much improved, each module has single responsibility

## Implementation Order
1. Create `src/core/tag_conversion.rs` (no dependencies on other extracted code)
2. Create `src/core/parsers/tiff_operations.rs` (uses tag_conversion)
3. Create `src/core/parsers/jpeg_operations.rs` (uses tag_conversion)
4. Create `src/core/format_dispatch.rs` (uses jpeg and tiff operations)
5. Refactor `src/core/operations.rs` to use extracted modules
6. Run tests to verify no regression

## Notes
- All existing tests should pass without modification
- Public API remains unchanged
- Internal implementation is reorganized for better maintainability
- Each new module will have its own tests section
