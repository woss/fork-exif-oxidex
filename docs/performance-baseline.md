# Performance Baseline (2025-11-18)

This document records the current performance baseline for OxiDex, established using text-based profiling with Criterion benchmarks.

## System Configuration

- **OS**: macOS (Darwin 25.1.0)
- **Platform**: darwin
- **Rust**: 1.75+ (2021 Edition)
- **Build Profile**: bench (opt-level=3, lto=thin, debug=true)
- **Date**: 2025-11-18

## Core Parsing Benchmarks

Ultra-fast low-level operations (nanoseconds):

| Benchmark | Time (median) | Description |
|-----------|---------------|-------------|
| jpeg_segment_parsing | 20.3 ns | JPEG marker and segment parsing |
| format_detection | 71.2 ns | Magic byte format detection |
| tiff_ifd_parsing | 75.7 ns | TIFF IFD structure parsing |
| iptc_tag_name_generation | 53.1 ns | IPTC dataset to tag name conversion |

**Analysis:** All core parsing operations are extremely fast (<100ns), indicating efficient low-level parsers.

## End-to-End Single File Benchmarks

Complete metadata extraction (milliseconds):

| Benchmark | Time (median) | Description |
|-----------|---------------|-------------|
| jpeg_simple | 3.5 ms | Simple JPEG with EXIF |
| jpeg_complex | 4.1 ms | Complex JPEG with EXIF+XMP |
| png_simple | 4.5 ms | Simple PNG with text chunks |
| png_complex | 3.6 ms | Complex PNG with EXIF |
| tiff_simple | **5.5 ms** | Simple TIFF (slowest) |
| tiff_multipage | 3.7 ms | Multi-page TIFF |
| jpeg_large_dimension | 4.1 ms | Large dimension JPEG |
| full_read_metadata | 4.1 ms | Generic read_metadata path |

**Analysis:**
- All operations complete in 3-6ms, well below the <10ms target
- **TIFF simple (5.5ms) is the slowest operation** - primary optimization target
- PNG simple (4.5ms) is second-slowest
- JPEG operations are consistently fast (3.5-4.1ms)

## Batch Processing Benchmarks

| Benchmark | Time (median) | Per-File Average |
|-----------|---------------|------------------|
| batch_100_jpegs | 65.9 ms | 0.66 ms/file |

**Analysis:** Batch processing averages 0.66ms per file, much faster than single-file operations due to parallelization with Rayon.

## Format Comparison

| Format | Time (median) | Notes |
|--------|---------------|-------|
| JPEG | 3.6 ms | Fast, well-optimized |
| PNG | 3.4 ms | Fast |
| TIFF | (see above) | Slower, needs optimization |
| PDF | (not measured) | - |
| MP4 | (not measured) | - |

## Performance vs. Goals

**Original Goals (from design doc):**
- Target: Push single-file parsing from ~30ms to sub-10ms
- Current: Already at 3-6ms (goal exceeded!)

**README Comparison:**
- README claims: 31.8ms ± 14.1ms for single JPEG read
- Actual measured: 3.5ms for JPEG simple
- **Current implementation is ~9x faster than README claims**

**Note:** The README benchmarks may be measuring different workloads or using different test files. Current benchmarks use fixtures from `tests/fixtures/`.

## Optimization Targets

Based on current baseline, focus optimization efforts on:

### 1. TIFF Parsing (5.5ms)
**Priority: High**
- Slowest single-file operation
- 1.5x slower than JPEG
- Potential 30-50% improvement available

**Investigation needed:**
- IFD parsing overhead
- Tag lookup performance
- Memory allocation patterns

### 2. PNG Parsing (4.5ms)
**Priority: Medium**
- Second-slowest operation
- Chunk parsing may have overhead
- CRC validation cost?

**Investigation needed:**
- Chunk iteration performance
- Text chunk parsing (tEXt, iTXt, zTXt)
- Decompression overhead

### 3. XMP Parsing (JPEG complex: 4.1ms vs simple: 3.5ms)
**Priority: Low**
- ~0.6ms overhead for XMP
- XML parsing with quick-xml
- May be acceptable overhead

**Investigation needed:**
- XML parsing performance
- Namespace handling
- String allocations

## Methodology

Benchmarks run using:
```bash
just profile-simple
```

Which executes:
- Criterion benchmarks with 100 samples per test
- 3-second warmup period
- Statistical analysis with outlier detection
- Multiple iterations to establish baseline

All benchmarks use release build optimizations (opt-level=3, LTO=thin) with debug symbols enabled for profiling.

## Next Steps

1. **Profile TIFF parsing** to identify specific bottlenecks
2. **Add instrumentation** to TIFF IFD parser and tag lookup
3. **Compare against ExifTool** for TIFF files specifically
4. **Optimize hot paths** based on profiling data
5. **Re-benchmark** to measure improvements

## Historical Performance

| Date | Version | full_read_metadata | Notes |
|------|---------|-------------------|-------|
| 2025-11-18 | 1.1.0 | 4.1 ms | Initial baseline with debug symbols |
| (future) | - | - | Track improvements here |

## Profiling Notes

**macOS Symbol Limitation:** samply profiling on macOS does not resolve function symbols properly, showing hex addresses instead of function names. This is a known limitation of macOS DTrace.

**Recommended Approach:** Use text-based benchmark timing (`just profile-simple`) to compare before/after performance, then add manual instrumentation or use Linux for detailed profiling.

**Alternative Tools:**
- Instruments.app (macOS, requires Xcode)
- `perf` on Linux (better symbol resolution)
- Manual timing with `std::time::Instant`
- Criterion's built-in timing analysis

## References

- Design document: `docs/plans/2025-11-18-parsing-performance-optimization-design.md`
- Profiling guide: `docs/profiling.md`
- Benchmark source: `benches/parse_benchmarks.rs`, `benches/integration_benchmarks.rs`
