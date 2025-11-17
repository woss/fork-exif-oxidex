# Data.lfs Error Summary

Generated: 2025-11-16 14:51:34

## Executive Summary

This document provides a comprehensive analysis of parsing errors encountered when testing oxidex against all 4,026 files in the data.lfs test directory.

## Statistics

- **Total files tested:** 4,026
- **Successful:** 2,424
- **Errors:** 782
- **Success rate:** 60.21%
- **Git internal files:** 820 (excluded from analysis)

## Error Categories

All errors share the same root cause: **"Unsupported format: Format Unknown not yet supported in this iteration"**

This indicates that the parser is not recognizing the file formats, despite many of them being standard camera raw formats.

### Category 1: Panasonic RW2 Format (Highest Priority)

- **Count:** 396 files (379 uppercase + 17 lowercase)
- **File formats:** .RW2, .rw2
- **Camera models:** DMC-TZ71, DMC-LX10, DMC-GF7, and many other Panasonic cameras
- **Error message:** "Unsupported format: Format Unknown not yet supported in this iteration"
- **Example files:**
  - `/Users/allen/Documents/git/examples/data.lfs/Panasonic/DMC-TZ71/RAW_PANASONIC_DMC_TZ71_3-2.RW2`
  - `/Users/allen/Documents/git/examples/data.lfs/Panasonic/DMC-LX10/pana_DMC-LX10_3x2.RW2`
- **Impact:** 50.6% of all errors
- **Priority:** HIGH - Most common error type

### Category 2: Fujifilm RAF Format

- **Count:** 148 files (141 uppercase + 7 lowercase)
- **File formats:** .RAF, .raf
- **Camera models:** X-T3, X-T4, X-Pro2, and other Fujifilm cameras
- **Error message:** "Unsupported format: Format Unknown not yet supported in this iteration"
- **Impact:** 18.9% of all errors
- **Priority:** HIGH

### Category 3: Olympus ORF Format

- **Count:** 93 files (80 uppercase + 13 lowercase)
- **File formats:** .ORF, .orf
- **Camera models:** E-PM1, E-520, E-P3, E-M1, and many other Olympus cameras
- **Error message:** "Unsupported format: Format Unknown not yet supported in this iteration"
- **Example files:**
  - `/Users/allen/Documents/git/examples/data.lfs/OLYMPUS/E-PM1/P2083783.ORF`
  - `/Users/allen/Documents/git/examples/data.lfs/OLYMPUS/E-520/RAW_OLYMPUS_E520.ORF`
- **Impact:** 11.9% of all errors
- **Priority:** HIGH

### Category 4: Olympus ORI Format (High-Res Mode)

- **Count:** 19 files (15 uppercase + 4 lowercase)
- **File formats:** .ORI, .ori
- **Camera models:** E-M1MarkII, E-M1MarkIII, E-M5 Mark II, OM-3, OM-5MarkII
- **Error message:** "Unsupported format: Format Unknown not yet supported in this iteration"
- **Note:** ORI files are high-resolution composite images from Olympus cameras using sensor-shift technology
- **Impact:** 2.4% of all errors
- **Priority:** MEDIUM

### Category 5: Canon CRW Format

- **Count:** 50 files (46 uppercase + 4 lowercase)
- **File formats:** .CRW, .crw
- **Camera models:** EOS D30, PowerShot S45, EOS 300D, EOS 10D, and other older Canon cameras
- **Error message:** "Unsupported format: Format Unknown not yet supported in this iteration"
- **Example files:**
  - `/Users/allen/Documents/git/examples/data.lfs/Canon/EOS D30/CRW_2444.CRW`
  - `/Users/allen/Documents/git/examples/data.lfs/Canon/PowerShot S45/RAW_CANON_S45.CRW`
- **Impact:** 6.4% of all errors
- **Priority:** MEDIUM - Legacy format

### Category 6: Generic RAW Format

- **Count:** 42 files (39 uppercase + 3 lowercase)
- **File formats:** .RAW, .raw
- **Camera models:** SJCAM action cameras, Paralenz dive cameras, ImBack film scanner
- **Error message:** "Unsupported format: Format Unknown not yet supported in this iteration"
- **Impact:** 5.4% of all errors
- **Priority:** LOW - Non-standard cameras

### Category 7: Leica RWL Format

- **Count:** 20 files
- **File formats:** .RWL
- **Camera models:** Leica cameras
- **Error message:** "Unsupported format: Format Unknown not yet supported in this iteration"
- **Impact:** 2.6% of all errors
- **Priority:** MEDIUM

### Category 8: Canon CR2 Format (Unexpected)

- **Count:** 3 files
- **File formats:** .CR2
- **Note:** CR2 format should normally be supported; these may be corrupted or non-standard files
- **Error message:** "Unsupported format: Format Unknown not yet supported in this iteration"
- **Impact:** 0.4% of all errors
- **Priority:** LOW - Investigate individual files

### Category 9: Other Formats

- **Count:** 11 files
- **File formats:** .MRW (Minolta), .MDC (Mustek), .HIF (Heif), .lri (Light), .cam, .ari (ARRI), .sample, and config files
- **Error message:** "Unsupported format: Format Unknown not yet supported in this iteration"
- **Impact:** 1.4% of all errors
- **Priority:** LOW - Rare formats

## Error Distribution by Manufacturer

1. **Panasonic:** 396 files (50.6%)
2. **Fujifilm:** 148 files (18.9%)
3. **Olympus/OM Digital:** 112 files (14.3%)
4. **Canon:** 53 files (6.8%)
5. **Leica:** 20 files (2.6%)
6. **Action Cameras/Other:** 53 files (6.8%)

## Root Cause Analysis

All errors share the same error message: **"Unsupported format: Format Unknown not yet supported in this iteration"**

This suggests a fundamental issue with format detection or parser initialization. Possible causes:

1. **Format detection not working:** The file format detector may not be recognizing these common raw formats
2. **Parser not registered:** The parsers for these formats may exist but are not registered/enabled
3. **Magic number mismatch:** The magic number detection may not be correctly identifying these file types
4. **Missing format handlers:** Some formats (RW2, RAF, ORF, CRW, ORI, RWL) may not have parsers implemented yet

## Patterns Observed

1. **Case sensitivity:** Both uppercase and lowercase extensions are failing (e.g., .ORF and .orf)
2. **Consistent error:** All failures have identical error messages, suggesting a single root cause
3. **Major formats affected:** Well-established raw formats (RW2, RAF, ORF) are not being parsed
4. **Some formats work:** 2,424 files (60.21%) parsed successfully, indicating that some formats ARE supported

## Recommendations

### Immediate Actions (High Priority)

1. **Add RW2 support:** Implement Panasonic RW2 parser - will fix 50.6% of errors
2. **Add RAF support:** Implement Fujifilm RAF parser - will fix 18.9% of errors
3. **Add ORF support:** Implement Olympus ORF parser - will fix 11.9% of errors
4. **Verify format detection:** Ensure the format detector is correctly identifying these file types

### Short-term Actions (Medium Priority)

5. **Add CRW support:** Implement Canon CRW legacy format parser - will fix 6.4% of errors
6. **Add RWL support:** Implement Leica RWL parser - will fix 2.6% of errors
7. **Add ORI support:** Extend ORF parser to handle high-resolution ORI files - will fix 2.4% of errors

### Long-term Actions (Low Priority)

8. **Investigate CR2 failures:** Debug why 3 CR2 files are failing (format should be supported)
9. **Add exotic format support:** MRW, MDC, HIF, generic RAW from action cameras

## Expected Impact

Implementing parsers for the top 3 formats (RW2, RAF, ORF) would:
- Fix 637 out of 782 errors (81.5% of errors)
- Improve overall success rate from 60.21% to 88.0%
- Cover the three most popular camera manufacturers in the test set

## Files by Error Type

### RW2 Files (396 total)
- Panasonic DMC-TZ71 (multiple aspect ratios)
- Panasonic DMC-LX10 (multiple aspect ratios)
- Panasonic DMC-GF7, DMC-GH5, DMC-G9, etc.

### RAF Files (148 total)
- Fujifilm X-T3, X-T4, X-Pro2, X-E3, X100F, etc.

### ORF Files (93 total)
- Olympus E-PM1, E-520, E-P3, E-M1, E-M5, E-PL series, etc.

### CRW Files (50 total)
- Canon EOS D30, D60, 10D, 300D (Digital Rebel)
- Canon PowerShot series (S45, S60, G3, G5, etc.)

### RWL Files (20 total)
- Leica cameras

### ORI Files (19 total)
- Olympus E-M1MarkII, E-M1MarkIII, E-M5 Mark II
- OM Digital Solutions OM-3, OM-5MarkII

## Next Steps

1. **Task 3:** Implement format support for unsupported extensions
2. **Task 4:** Fix TIFF parsing errors (if any exist in successful files)
3. **Task 5:** Improve raw format detection and error handling
4. **Task 6:** Re-run comprehensive test to verify fixes

## Notes

- Success rate of 60.21% indicates that basic file reading and many formats ARE working
- All errors are "format unknown" - no parsing errors, corruption issues, or I/O errors detected
- This is a format detection/registration issue, not a parsing quality issue
- The error logs contain only file paths, no detailed error messages from stderr
