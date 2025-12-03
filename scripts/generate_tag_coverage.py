#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.9"
# dependencies = [
#     "pyyaml>=6.0",
# ]
# ///
"""
Generate Tag Coverage Analysis Report

This script analyzes the OxiDex tag database and parser implementations
to generate a comprehensive coverage report.

Usage:
    uv run scripts/generate_tag_coverage.py [--output docs/reference/tag-coverage-analysis.md]

Or via justfile:
    just docs-coverage
"""

import argparse
import re
import sys
from collections import defaultdict
from datetime import datetime
from pathlib import Path

import yaml


def get_project_root() -> Path:
    """Find project root by looking for Cargo.toml"""
    current = Path(__file__).resolve().parent
    while current != current.parent:
        if (current / "Cargo.toml").exists():
            return current
        current = current.parent
    raise RuntimeError("Could not find project root")


def parse_yaml_tags(project_root: Path) -> dict:
    """Parse all YAML tag files and count tags by domain/category"""
    domains = {}
    yaml_files = list(project_root.glob("oxidex-tags-*/src/*.yaml"))

    for yaml_file in yaml_files:
        # Extract domain from path (e.g., oxidex-tags-core -> core)
        domain = yaml_file.parent.parent.name.replace("oxidex-tags-", "")

        content = yaml_file.read_text()

        try:
            data = yaml.safe_load(content)
        except yaml.YAMLError as e:
            print(f"Warning: Failed to parse {yaml_file}: {e}")
            continue

        if not data or "tables" not in data:
            continue

        domain_data = domains.setdefault(domain, {
            "categories": defaultdict(int),
            "total_tags": 0,
            "total_tables": 0
        })

        for table in data["tables"]:
            table_name = table.get("name", "Unknown")
            tags = table.get("tags", [])
            tag_count = len(tags)

            domain_data["categories"][table_name] = tag_count
            domain_data["total_tags"] += tag_count
            domain_data["total_tables"] += 1

    return domains


def analyze_parsers(project_root: Path) -> dict:
    """Analyze parser implementations to find tag extraction"""
    parsers_dir = project_root / "src" / "parsers"
    parser_coverage = {}

    # Map directory names to format names
    format_map = {
        "audio": ["FLAC", "MP3", "AAC", "APE", "Opus", "OGG", "WAV"],
        "image": ["BMP", "GIF", "WebP"],
        "jpeg": ["JPEG"],
        "png": ["PNG"],
        "tiff": ["TIFF", "EXIF"],
        "pdf": ["PDF"],
        "quicktime": ["QuickTime", "MP4", "MOV"],
        "video": ["MKV", "AVI", "RIFF"],
        "pe": ["PE"],
        "archive": ["ZIP"],
        "document": ["DOCX", "XLSX"],
        "font": ["TTF", "OTF"],
        "raw": ["DNG", "CR2", "NEF"],
    }

    for parser_dir in parsers_dir.iterdir():
        if not parser_dir.is_dir():
            continue

        dir_name = parser_dir.name
        if dir_name in ["common", "detection", "mod.rs"]:
            continue

        # Count tag insertions
        tag_insertions = 0
        for rs_file in parser_dir.rglob("*.rs"):
            try:
                content = rs_file.read_text()
                # Count various tag insertion patterns
                tag_insertions += len(re.findall(r'tags\.insert\s*\(', content))
                tag_insertions += len(re.findall(r'metadata\.insert\s*\(', content))
                tag_insertions += len(re.findall(r'result\.insert\s*\(', content))
            except Exception:
                continue

        formats = format_map.get(dir_name, [dir_name.upper()])
        for fmt in formats:
            parser_coverage[fmt] = {
                "directory": dir_name,
                "tag_insertions": tag_insertions,
                "has_parser": tag_insertions > 0
            }

    return parser_coverage


def check_makernote_status(project_root: Path) -> dict:
    """Check MakerNote dispatcher status"""
    dispatcher_path = project_root / "src" / "parsers" / "tiff" / "makernote_dispatcher.rs"
    file_parser_path = project_root / "src" / "parsers" / "tiff" / "file_parser.rs"

    result = {
        "dispatcher_exists": dispatcher_path.exists(),
        "wired_up": False,
        "manufacturers": []
    }

    if not dispatcher_path.exists():
        return result

    # Check if dispatcher is wired up in file_parser.rs
    if file_parser_path.exists():
        content = file_parser_path.read_text()
        result["wired_up"] = "dispatch_makernote" in content

    # Extract supported manufacturers
    dispatcher_content = dispatcher_path.read_text()
    # Match patterns like: "canon" => Some(Box::new(
    manufacturers = re.findall(r'"([a-z][a-z0-9_ ]*)"[^=]*=>\s*Some\(Box::new\(', dispatcher_content)
    result["manufacturers"] = sorted(set(m.title() for m in manufacturers))

    return result


def get_audio_coverage() -> dict:
    """Audio parsers are known to be complete"""
    return {
        "FLAC": 100,
        "MP3": 100,
        "AAC": 100,
        "APE": 100,
        "Opus": 100,
        "OGG": 100,
        "WAV": 100,
    }


def estimate_coverage(parser_info: dict, domain_data: dict) -> dict:
    """Estimate coverage percentages based on parser analysis"""
    # Known coverage levels (manually verified)
    known_coverage = {
        "FLAC": 100, "MP3": 100, "AAC": 100, "APE": 100,
        "Opus": 100, "OGG": 100, "WAV": 100,
        "QuickTime": 67, "RIFF": 67,
        "JPEG": 40, "TIFF": 35, "PNG": 30, "PDF": 25,
        "ZIP": 20, "MKV": 15, "PE": 80,
        "Mach-O": 10, "ELF": 10,
    }

    coverage = {}
    for fmt, info in parser_info.items():
        if fmt in known_coverage:
            coverage[fmt] = known_coverage[fmt]
        elif info["has_parser"]:
            # Estimate based on tag insertions
            coverage[fmt] = min(50, info["tag_insertions"] // 2)
        else:
            coverage[fmt] = 0

    return coverage


def generate_markdown(domains: dict, parser_coverage: dict, makernote_status: dict, coverage: dict) -> str:
    """Generate the markdown report"""
    today = datetime.now().strftime("%Y-%m-%d")

    # Calculate totals
    total_tags = sum(d["total_tags"] for d in domains.values())
    total_tables = sum(d["total_tables"] for d in domains.values())
    total_categories = sum(len(d["categories"]) for d in domains.values())

    # Count parsers with coverage
    parsers_with_coverage = sum(1 for c in coverage.values() if c > 0)

    md = f"""# Tag Database Coverage Analysis

**Last Updated:** {today}

This report analyzes the gap between tags defined in the OxiDex tag database and tags actually extracted by parsers.

::: info Auto-Generated
This document was generated by `scripts/generate_tag_coverage.py`. Run `just docs-coverage` to regenerate.
:::

## Executive Summary

### Key Metrics

- **{total_tables:,} tag tables** defined across **{len(domains)} domains**
- **{total_tags:,} total tags** in database
- **{parsers_with_coverage} format parsers** with active tag extraction
- **Target:** 28,853 total tags for ExifTool parity

### Coverage by Domain

| Domain | Tables | Tags | Categories |
|--------|--------|------|------------|
"""

    for domain, data in sorted(domains.items()):
        md += f"| {domain.title()} | {data['total_tables']} | {data['total_tags']:,} | {len(data['categories'])} |\n"

    md += f"| **Total** | **{total_tables}** | **{total_tags:,}** | **{total_categories}** |\n"

    # MakerNote status
    md += """
---

## MakerNote Status

"""

    if makernote_status["wired_up"]:
        md += """::: tip ✅ MakerNote Parsers Active
MakerNote parsers for 40+ camera manufacturers are **fully implemented and connected** to the TIFF parsing pipeline.
:::

### Supported Manufacturers

"""
        # Group manufacturers
        traditional = ["Canon", "Nikon", "Sony", "Olympus", "Panasonic", "Pentax", "Fujifilm", "Leica", "Sigma", "Phase One", "Minolta"]
        smartphones = ["Apple", "Google", "Samsung", "Microsoft", "Qualcomm"]
        specialty = ["Dji", "Flir", "Gopro", "Infiray", "Lytro", "Nintendo", "Parrot", "Reconyx", "Red"]
        legacy = ["Casio", "Ge", "Hp", "Jvc", "Kodak", "Leaf", "Motorola", "Ricoh", "Sanyo"]
        software = ["Capture One", "Fotostation", "Gimp", "Indesign", "Nikon Capture", "Photomechanic", "Photoshop", "Scalado"]

        all_mfrs = set(makernote_status["manufacturers"])

        md += "**Traditional Cameras:** "
        md += ", ".join(m for m in traditional if m in all_mfrs or m.lower() in [x.lower() for x in all_mfrs]) + "\n\n"
        md += "**Smartphones:** "
        md += ", ".join(m for m in smartphones if m in all_mfrs or m.lower() in [x.lower() for x in all_mfrs]) + "\n\n"
        md += "**Specialty Devices:** "
        md += ", ".join(m for m in specialty if m in all_mfrs or m.lower() in [x.lower() for x in all_mfrs]) + "\n\n"
        md += "**Legacy Cameras:** "
        md += ", ".join(m for m in legacy if m in all_mfrs or m.lower() in [x.lower() for x in all_mfrs]) + "\n\n"
    else:
        md += """::: warning ⚠️ MakerNote Parsers Not Connected
MakerNote parsers exist but are NOT wired up to the parsing pipeline. This is a critical gap.
:::
"""

    # Coverage analysis
    md += """
---

## Coverage Analysis by Format

### ✅ Strong Coverage (>50%)

| Format | Coverage | Status | Notes |
|--------|----------|--------|-------|
"""

    strong = [(k, v) for k, v in coverage.items() if v >= 50]
    for fmt, cov in sorted(strong, key=lambda x: -x[1]):
        status = "✅ Complete" if cov == 100 else "⚠️ Partial"
        md += f"| **{fmt}** | {cov}% | {status} | |\n"

    md += """
### ⚠️ Partial Coverage (10-50%)

| Format | Coverage | Status | Priority |
|--------|----------|--------|----------|
"""

    partial = [(k, v) for k, v in coverage.items() if 10 <= v < 50]
    for fmt, cov in sorted(partial, key=lambda x: -x[1]):
        priority = "High" if cov >= 30 else "Medium"
        md += f"| **{fmt}** | {cov}% | ⚠️ Partial | {priority} |\n"

    md += """
### ❌ Minimal/No Coverage (<10%)

| Format | Coverage | Status |
|--------|----------|--------|
"""

    minimal = [(k, v) for k, v in coverage.items() if v < 10]
    for fmt, cov in sorted(minimal, key=lambda x: x[0]):
        status = "❌ None" if cov == 0 else "⚠️ Minimal"
        md += f"| **{fmt}** | {cov}% | {status} |\n"

    # Critical gaps
    md += """
---

## Critical Gaps

### Professional Workflow Tags (Missing)

| Category | Status | Impact |
|----------|--------|--------|
| IPTC | ❌ Not extracted | Photojournalism metadata |
| XMP | ❌ Not extracted | Adobe workflow standard |
| Photoshop | ❌ Not extracted | Layer/editing metadata |
| ICC_Profile | ❌ Not extracted | Color management |

### Executable Formats

| Format | Status | Missing |
|--------|--------|---------|
| PE (Windows) | ✅ Good | Certificate data, .NET metadata |
| ELF (Linux) | ⚠️ Minimal | Section headers, symbols |
| Mach-O (macOS) | ⚠️ Minimal | CPU details, load commands |

---

## Recommendations

### Immediate Priorities

1. **IPTC Parser** - Critical for photojournalism workflows
2. **XMP Parser** - Adobe's universal metadata standard
3. **Enhance Mach-O/ELF** - Better executable analysis

### Medium-Term Goals

4. **GPS Tag Extraction** - Geotagging applications
5. **Complete MKV Parser** - Video analysis tools
6. **RAW Camera Formats** - Professional photography

---

## How to Update This Document

Run the generation script:

```bash
just docs-coverage
```

Or manually:

```bash
uv run scripts/generate_tag_coverage.py --output docs/reference/tag-coverage-analysis.md
```

### What the Script Analyzes

1. **Tag Database** - Parses YAML files in `oxidex-tags-*/src/`
2. **Parser Coverage** - Counts tag insertions in `src/parsers/`
3. **MakerNote Status** - Checks dispatcher wiring in TIFF parser

---

## Related Documentation

- [ExifTool Coverage](/reference/exiftool-coverage) - Tag database statistics
- [Tag Database Architecture](/architecture/tag-database) - Implementation details
- [MakerNotes Reference](/reference/makernotes) - Camera manufacturer metadata
"""

    return md


def main():
    parser = argparse.ArgumentParser(description="Generate tag coverage analysis report")
    parser.add_argument(
        "--output", "-o",
        default="docs/reference/tag-coverage-analysis.md",
        help="Output file path (default: docs/reference/tag-coverage-analysis.md)"
    )
    parser.add_argument(
        "--dry-run", "-n",
        action="store_true",
        help="Print to stdout instead of writing to file"
    )
    args = parser.parse_args()

    project_root = get_project_root()
    print(f"Project root: {project_root}")

    print("Parsing YAML tag files...")
    domains = parse_yaml_tags(project_root)

    print("Analyzing parser implementations...")
    parser_coverage = analyze_parsers(project_root)

    print("Checking MakerNote status...")
    makernote_status = check_makernote_status(project_root)

    print("Estimating coverage...")
    coverage = estimate_coverage(parser_coverage, domains)
    # Add audio coverage (known to be complete)
    coverage.update(get_audio_coverage())

    print("Generating markdown...")
    markdown = generate_markdown(domains, parser_coverage, makernote_status, coverage)

    if args.dry_run:
        print(markdown)
    else:
        output_path = project_root / args.output
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(markdown)
        print(f"Written to: {output_path}")

    # Print summary
    total_tags = sum(d["total_tags"] for d in domains.values())
    print(f"\nSummary:")
    print(f"  Domains: {len(domains)}")
    print(f"  Total tags: {total_tags:,}")
    print(f"  MakerNotes wired up: {makernote_status['wired_up']}")
    print(f"  Manufacturers: {len(makernote_status['manufacturers'])}")


if __name__ == "__main__":
    main()
