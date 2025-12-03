# Tag Database Architecture

The tag database provides O(1) lookup for 32,000+ metadata tag definitions auto-generated from ExifTool source.

## Overview

| Metric | Value |
|--------|-------|
| Total Tags | 32,677 |
| Modules Parsed | 140+ |
| Lookup Time | O(1) |
| Memory | ~5-10MB (lazy loaded) |

## Workspace Architecture

The tag database is implemented as a **separate workspace crate** (`oxidex-tags-*`) to solve debug build memory issues.

### Structure

```
oxidex/
├── oxidex-tags-core/     # Core types (TagDescriptor, TagId, etc.)
├── oxidex-tags-camera/   # Camera MakerNotes tags
├── oxidex-tags-media/    # Audio/video format tags
├── oxidex-tags-image/    # Image format tags (EXIF, PNG, etc.)
├── oxidex-tags-document/ # Document format tags (PDF, etc.)
├── oxidex-tags-specialty/# Specialized format tags (DICOM, etc.)
└── src/                  # Main crate (uses oxidex-tags-*)
```

### Profile Configuration

```toml
# In root Cargo.toml
[profile.dev.package.oxidex-tags-core]
opt-level = 2        # Always optimize tag crates
codegen-units = 16   # Parallel compilation

[profile.dev.package.oxidex-tags-camera]
opt-level = 2
codegen-units = 16

# ... similar for other tag crates
```

### Why Separate Crates?

- **Debug builds**: 100GB+ RAM → **11GB** (91% reduction)
- **Main crate stays in debug mode** (fast iteration)
- **Tag crates always optimized** (prevents OOM)
- **Industry-standard pattern** (used by rustc, diesel, syn)

## Tag Generation Pipeline

Tags are automatically generated during build from Perl ExifTool source:

```
1. Download → Fetches latest ExifTool master from GitHub
2. Discover → Finds all 140+ .pm Perl modules recursively
3. Parse    → Extracts tag definitions using regex patterns
4. Resolve  → Follows subdirectory references for nested tables
5. Generate → Creates Rust code organized by format family
```

### Generated Code Structure

Each format family gets its own module:

```rust
// oxidex-tags-camera/src/canon.rs
static TAGS: Lazy<Vec<TagDescriptor>> = Lazy::new(|| vec![
    TagDescriptor {
        tag_id: TagId::Numeric(0x0001),
        tag_name: "Canon:CameraSettings",
        format_family: "Canon",
        // ...
    },
    // ... 930 Canon tags
]);

pub fn get_tags() -> &'static HashMap<String, TagDescriptor> {
    static MAP: Lazy<HashMap<String, TagDescriptor>> = Lazy::new(|| {
        let mut map = HashMap::with_capacity(TAGS.len());
        for tag in TAGS.iter() {
            map.insert(tag.tag_name.clone(), tag.clone());
        }
        map
    });
    &MAP
}
```

## Supported Formats

### By Tag Count

| Module | Tags | Description |
|--------|------|-------------|
| DICOM | 3,149 | Medical imaging |
| NikonCustom | 3,512 | Nikon custom settings |
| Nikon | 2,398 | Nikon MakerNotes |
| Sony | 1,148 | Sony MakerNotes |
| QuickTime | 1,069 | Video metadata |
| Canon | 930 | Canon MakerNotes |
| Casio | 930 | Casio MakerNotes |
| Pentax | 876 | Pentax MakerNotes |
| EXIF | 718 | Core EXIF specification |

### By Category

**Standard Formats:**
- EXIF, GPS, XMP, IPTC, JFIF, TIFF

**MakerNotes (30+ vendors):**
- Canon, Nikon, Sony, Olympus, Panasonic, Pentax, FujiFilm
- Samsung, Minolta, Kodak, Casio, Ricoh, etc.

**Video:**
- QuickTime, MP4, Matroska, Flash, ASF, MPEG, H264

**Audio:**
- ID3, FLAC, Ogg, Vorbis, AAC, APE

**Specialized:**
- DICOM (medical), FITS (astronomy), MXF, PDF, PostScript

**RAW:**
- DNG, CR2, NEF, ARW, CanonRaw, SigmaRaw, MinoltaRaw

**Graphics:**
- PNG, GIF, BMP, PSD, JPEG, JPEG2000, OpenEXR, ICO

**Documents:**
- HTML, XML, SVG, VCard, LNK

## Usage

### Lookup by Tag Name

```rust
use oxidex::tag_db::get_tag_descriptor;

if let Some(tag) = get_tag_descriptor("EXIF:Make") {
    println!("Tag: {} (ID: {:?})", tag.tag_name, tag.tag_id);
}
```

### Get All Tags for Format

```rust
use oxidex_tags_camera::canon::get_tags;

for (name, descriptor) in get_tags().iter() {
    println!("{}: {:?}", name, descriptor.value_type);
}
```

## Rebuilding

To force regeneration:

```bash
# Remove generated files
rm -rf oxidex-tags-*/src/generated/

# Rebuild (triggers build.rs)
cargo build
```

The build script will:
1. Download ExifTool source (~10MB)
2. Parse 32,677 tag definitions
3. Generate optimized Rust code
4. Compile into workspace crates

## Performance

- **Lookup**: O(1) via HashMap
- **Memory**: ~5-10MB (heap-allocated lazily)
- **Build Time**: ~4 minutes (cached after first build)
- **Compilation**: Uses Lazy initialization to avoid static limits

## Build Requirements

### Memory

| Build Mode | Memory |
|------------|--------|
| Release | ~5GB |
| Debug (with workspace) | ~11GB |
| Debug (without workspace) | 100GB+ (OOM) |

**Recommendation**: Always use release builds for final testing.

### Commands

```bash
# Development build
cargo build

# Release build (recommended for testing)
cargo build --release

# Run tests (release recommended)
cargo test --release --workspace
```

## Parser Features

The Perl tag definition parser handles:

- Hash-based definitions: `0x0100 => { Name => 'ImageWidth', ... }`
- Simple definitions: `0x0100 => 'ImageWidth'`
- String-based tag IDs (hashed to numeric)
- Nested subdirectory references
- Writable type specifications
- Value type inference
- Multi-line definitions

## Known Limitations

- Some ExifTool composite tags are excluded (calculated values)
- Shortcut tags are excluded (aliases to other tags)
- Some tags have platform-specific or format-specific variations
