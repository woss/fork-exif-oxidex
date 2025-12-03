# OxiDex

[![CI](https://github.com/swack-tools/oxidex/workflows/CI/badge.svg)](https://github.com/swack-tools/oxidex/actions)
[![Crates.io](https://img.shields.io/crates/v/oxidex.svg)](https://crates.io/crates/oxidex)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](LICENSE)

A high-performance Rust implementation of [ExifTool](https://exiftool.org/) for metadata extraction and manipulation.

## What is OxiDex?

OxiDex is a memory-safe, drop-in replacement for the Perl-based ExifTool. It provides the same comprehensive metadata support (32,677+ tags across 140+ formats) with significantly better performance through Rust's zero-cost abstractions and parallel processing.

## Why OxiDex?

- **3.7-9.7x faster** than Perl ExifTool ([see benchmarks](https://oxidex.net/performance/benchmarks))
- **Memory safe** - No buffer overflows, use-after-free, or data races
- **Drop-in compatible** - Same CLI arguments as original ExifTool
- **Cross-platform** - Static binaries for Linux, macOS, and Windows
- **Library + CLI** - Use as a Rust crate or standalone binary

## Quick Start
```

### Download Binary

Pre-built binaries available on the [Releases page](https://github.com/swack-tools/oxidex/releases).

## Usage

```bash
# Extract all metadata
oxidex photo.jpg

# Extract specific tags
oxidex -Make -Model -DateTimeOriginal photo.jpg

# Write metadata
oxidex -Artist="Your Name" photo.jpg

# Batch processing
oxidex -r /path/to/photos/

# JSON output
oxidex -json photo.jpg
```

## Documentation

- [User Guide](https://oxidex.net/) - Installation, usage, and format support
- [Benchmarks](https://oxidex.net/performance/#benchmark-results) - Performance comparison with Perl ExifTool
- [API Reference](https://docs.rs/oxidex) - Rust library documentation
- [GitHub Issues](https://github.com/swack-tools/oxidex/issues) - Bug reports and feature requests

## Contributing

Contributions welcome! Please ensure:
- Tests pass (`cargo test`)
- Code is formatted (`cargo fmt`)
- Clippy lints pass (`cargo clippy`)

## License

[GPL-3.0](LICENSE)

## Acknowledgments

Inspired by and compatible with [ExifTool](https://exiftool.org/) by Phil Harvey. OxiDex is an independent reimplementation.
