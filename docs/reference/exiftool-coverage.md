# ExifTool Tag Coverage

This document details OxiDex's coverage of ExifTool's tag database.

## Summary

| Metric | Value |
|--------|-------|
| Total Tags | 32,677 |
| Modules Parsed | 140+ |
| Coverage | 113% of documented tags* |

*ExifTool officially documents ~28,853 unique tags. OxiDex parses 32,677 due to including variant definitions.

## Module Categories

### Base Format Modules (15)

Core formats with broad support:

| Module | Tags | Description |
|--------|------|-------------|
| Exif.pm | ~3,732 | Core EXIF tags |
| GPS.pm | ~267 | GPS location data |
| XMP.pm | ~2,012 | XMP metadata |
| IPTC.pm | ~720 | Press/media metadata |
| PDF.pm | ~334 | PDF documents |
| QuickTime.pm | ~6,567 | MOV/MP4 video |
| Photoshop.pm | ~550 | Photoshop metadata |
| PNG.pm | ~100 | PNG images |
| JFIF.pm | ~50 | JPEG header |
| JPEG.pm | ~200 | JPEG metadata |
| TIFF.pm | ~400 | TIFF format |
| ICC_Profile.pm | ~150 | Color profiles |
| PostScript.pm | ~100 | PostScript/EPS |
| RIFF.pm | ~400 | RIFF/AVI/WAV |
| MakerNotes.pm | ~200 | Generic MakerNotes |

### MakerNotes Modules (30+)

Camera manufacturer-specific metadata:

| Module | Tags | Description |
|--------|------|-------------|
| Canon.pm | ~7,379 | Canon cameras |
| Nikon.pm | ~9,586 | Nikon cameras |
| Sony.pm | ~7,810 | Sony cameras |
| Pentax.pm | ~4,777 | Pentax cameras |
| Olympus.pm | ~3,194 | Olympus cameras |
| Minolta.pm | ~2,098 | Minolta cameras |
| Panasonic.pm | ~1,977 | Panasonic cameras |
| FujiFilm.pm | ~1,177 | FujiFilm cameras |
| Samsung.pm | ~1,012 | Samsung cameras |
| Sigma.pm | ~647 | Sigma cameras |
| Casio.pm | ~500 | Casio cameras |
| Kodak.pm | ~400 | Kodak cameras |
| Ricoh.pm | ~350 | Ricoh cameras |
| PhaseOne.pm | ~800 | Phase One cameras |
| HP.pm | ~200 | HP cameras |
| JVC.pm | ~250 | JVC cameras |
| Sanyo.pm | ~300 | Sanyo cameras |
| Motorola.pm | ~150 | Motorola cameras |
| Reconyx.pm | ~200 | Trail cameras |

**Specialized MakerNotes:**
- CanonCustom.pm (~1,500) - Canon custom functions
- CanonVRD.pm (~300) - Canon VRD settings
- NikonCapture.pm (~200) - Nikon Capture software
- NikonCustom.pm (~500) - Nikon custom settings

### Extended Image Formats (25+)

| Module | Tags | Description |
|--------|------|-------------|
| DNG.pm | ~115 | Adobe Digital Negative |
| FlashPix.pm | ~200 | FlashPix format |
| Jpeg2000.pm | ~150 | JPEG 2000 |
| GIF.pm | ~50 | GIF images |
| BMP.pm | ~40 | Windows Bitmap |
| OpenEXR.pm | ~100 | HDR images |
| DjVu.pm | ~80 | DjVu documents |
| BigTIFF.pm | ~60 | Large TIFF |

### Media Formats (23)

| Module | Tags | Description |
|--------|------|-------------|
| Matroska.pm | ~641 | MKV/WebM |
| ID3.pm | ~200 | MP3 ID3 tags |
| FLAC.pm | ~150 | FLAC audio |
| Vorbis.pm | ~100 | Ogg Vorbis |
| Opus.pm | ~80 | Opus audio |
| ASF.pm | ~300 | WMA/WMV |
| MPEG.pm | ~250 | MPEG video |
| M2TS.pm | ~150 | Blu-ray |
| Flash.pm | ~200 | SWF format |
| AIFF.pm | ~100 | Audio files |
| AAC.pm | ~80 | AAC audio |
| APE.pm | ~60 | Monkey's Audio |

### Specialized Formats (15)

| Module | Tags | Description |
|--------|------|-------------|
| FLIR.pm | ~822 | Thermal imaging |
| DICOM.pm | ~500 | Medical imaging |
| FITS.pm | ~200 | Astronomy |
| DJI.pm | ~300 | DJI drones |
| GoPro.pm | ~250 | Action cameras |
| Parrot.pm | ~200 | Parrot drones |
| Apple.pm | ~300 | Apple devices |
| Microsoft.pm | ~250 | Microsoft metadata |
| Google.pm | ~100 | Google metadata |
| RED.pm | ~200 | RED cinema |

### Document Formats (15)

| Module | Tags | Description |
|--------|------|-------------|
| HTML.pm | ~150 | HTML metadata |
| OOXML.pm | ~300 | Office Open XML |
| iWork.pm | ~150 | Apple iWork |
| InDesign.pm | ~200 | Adobe InDesign |
| Font.pm | ~150 | Font metadata |
| ZIP.pm | ~100 | Archives |
| EXE.pm | ~200 | Executables |

## Tag Count Notes

### Why Counts Differ

ExifTool officially documents ~28,853 unique tags, but parsing yields ~81,000+ tag-like definitions because:

1. **Multiple definitions per tag**: Hash assignments have multiple `=>` per logical tag
2. **Nested structures**: Perl hashes for subtables count each key-value pair
3. **Conditional definitions**: Tags with format variants
4. **Code vs data**: Not all `=>` occurrences are tag definitions

Our actual unique tag count (32,677) is closer to the official count after deduplication.

### Excluded Tags

Some ExifTool tags are excluded by design:

- **Composite tags**: Calculated values (Aperture from FNumber, etc.)
- **Shortcut tags**: Aliases to other tags
- **Internal tags**: ExifTool operational tags

## Coverage by Use Case

| Use Case | Coverage | Notes |
|----------|----------|-------|
| JPEG photos | 95%+ | EXIF, XMP, IPTC, MakerNotes |
| RAW photos | 90%+ | DNG, CR2, NEF, ARW, etc. |
| Video files | 85%+ | QuickTime, Matroska, RIFF |
| Audio files | 85%+ | ID3, FLAC, Vorbis, AAC |
| PDF documents | 95%+ | Info dict, XMP |
| Office docs | 80%+ | OOXML, iWork |
| Medical (DICOM) | 90%+ | Standard DICOM tags |

## Adding Coverage

To improve coverage for a specific format:

1. Identify missing tags from ExifTool output
2. Add tag definitions to appropriate tag crate
3. Run build to regenerate database
4. Test with sample files

See [Tag Database Architecture](/architecture/tag-database) for implementation details.
