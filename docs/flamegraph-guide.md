# Flamegraph Profiling Guide

This guide explains how to use cargo-flamegraph for performance profiling on OxiDex.

## Overview

cargo-flamegraph generates visual flame graphs showing where your code spends time. It uses:
- **Linux**: `perf` (no sudo required)
- **macOS**: `dtrace` (requires sudo)

## Installation

```bash
cargo install flamegraph
```

## macOS Limitations

**Requires sudo:** On macOS, flamegraph uses dtrace which requires root privileges.

```bash
# This will prompt for password
cargo flamegraph --bench parse_benchmarks --root -o flamegraph.svg -- --bench full_read_metadata
```

**Accessibility Note:** Since flamegraph:
1. Requires interactive sudo (password prompt)
2. Generates visual SVG output (not screen-reader friendly)
3. May have symbol resolution issues on macOS

**We recommend using `just profile-simple` instead** for accessible, text-based profiling.

## Linux Usage (No sudo)

On Linux, flamegraph works without sudo:

```bash
# Profile a benchmark
cargo flamegraph --bench parse_benchmarks -o flamegraph.svg -- --bench full_read_metadata

# Profile the CLI binary
cargo flamegraph --bin oxidex -o flamegraph.svg -- tests/fixtures/jpeg/sample_with_exif.jpg

# Open the SVG in browser
firefox flamegraph.svg
```

## Converting SVG to Text (Accessibility)

If you have a flamegraph SVG file, you can extract text information:

```bash
# Extract function names and times from SVG
python3 parse_flamegraph.py flamegraph.svg
```

See `parse_flamegraph.py` for implementation.

## Flame Graph Interpretation

**Visual representation (for sighted users):**
- **Width** = time spent in function (wider = more time)
- **Height** = call stack depth (taller = deeper nesting)
- **Color** = hash of function name (consistent across runs with --deterministic)

**What to look for:**
- Wide bars at the bottom = hot leaf functions doing actual work
- Wide bars at the top = orchestration overhead
- Plateaus = function called many times from many places

## Text-Based Alternative

For accessible profiling without visuals:

```bash
# Run text-based profiling
just profile-simple

# Shows timing for each benchmark
# Look for highest times - those are optimization targets
```

This provides:
- Clear text output
- No sudo required
- Works on all platforms
- Screen-reader friendly
- Easier to automate

## Advanced Options

### Profile specific iterations

```bash
# Profile just the benchmark measurement phase
cargo flamegraph --bench parse_benchmarks -- --bench full_read_metadata --profile-time 10
```

### Inverted flame graph

```bash
# Show callers instead of callees
cargo flamegraph --inverted --bench parse_benchmarks -o inverted.svg -- --bench full_read_metadata
```

### Custom sampling frequency

```bash
# Sample at 4999 Hz (higher = more detail, more overhead)
cargo flamegraph -F 4999 --bench parse_benchmarks -o detailed.svg -- --bench full_read_metadata
```

## Comparison: Flamegraph vs Text-Based

| Feature | Flamegraph | Text-Based (`profile-simple`) |
|---------|------------|-------------------------------|
| **Accessibility** | Visual only | Screen-reader friendly |
| **macOS sudo** | Required | Not required |
| **Symbol resolution** | May fail on macOS | N/A (uses Criterion timing) |
| **Setup** | Complex | Simple |
| **Output** | SVG file | Terminal text |
| **Detail level** | Function-level | Benchmark-level |
| **Best for** | Finding hotspots | Comparing performance |

## Recommendation

**For OxiDex development:**
1. Start with `just profile-simple` to identify slow benchmarks
2. If you need function-level detail:
   - **macOS**: Add manual instrumentation or use Instruments.app
   - **Linux**: Use `cargo flamegraph` for detailed flame graphs
3. Compare before/after with text-based benchmarks

## See Also

- Text-based profiling: `docs/profiling.md`
- Performance baseline: `docs/performance-baseline.md`
- Manual instrumentation: Add `std::time::Instant` timing to specific functions
