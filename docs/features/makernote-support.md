# MakerNote Support

Oxidex supports extracting manufacturer-specific metadata (MakerNotes) from JPEG and TIFF files for 40+ camera manufacturers and software applications.

## Supported Manufacturers

### Traditional Cameras
- **Canon** - EOS series, PowerShot, etc.
- **Nikon** - D-series, Z-series, Coolpix, etc.
- **Sony** - Alpha series, Cyber-shot, etc.
- **Olympus** - OM-D, PEN, Tough series
- **Panasonic** - Lumix series
- **Pentax** - K-series, Q-series
- **Fujifilm** - X-series, GFX, FinePix
- **Leica** - M-series, Q-series, SL-series
- **Sigma** - fp series, Foveon cameras
- **Phase One** - Medium format backs
- **Minolta** - Legacy Maxxum/Dynax series

### Smartphones
- **Apple** - iPhone series
- **Google** - Pixel series
- **Samsung** - Galaxy series
- **Microsoft** - Lumia, Surface cameras
- **Qualcomm** - Reference devices

### Specialty Devices
- **DJI** - Mavic, Phantom, Inspire drones
- **FLIR** - Thermal imaging cameras
- **GoPro** - Hero series action cameras
- **RED** - Digital cinema cameras
- **Reconyx** - Wildlife trail cameras
- **InfiRay** - Thermal cameras
- **Lytro** - Light field cameras
- **Nintendo** - 3DS cameras
- **Parrot** - Anafi, Bebop drones

### Legacy Cameras
- **Casio** - Exilim series
- **GE** - General Electric cameras
- **HP** - Photosmart series
- **JVC** - Digital cameras and camcorders
- **Kodak** - EasyShare, Professional series
- **Leaf** - Digital backs
- **Motorola** - Phone cameras
- **Ricoh** - GR series, Caplio
- **Sanyo** - Xacti series

### Software Applications
- **Capture One** - Phase One editing software
- **FotoStation** - FotoWare asset management
- **GIMP** - GNU Image Manipulation Program
- **Adobe InDesign** - Page layout software
- **Nikon Capture NX** - Nikon's editing software
- **Photo Mechanic** - Photo management software
- **Adobe Photoshop** - Image editing software
- **Scalado** - Mobile imaging software

**Total: 40+ supported manufacturers**

## How It Works

MakerNotes are proprietary binary data structures embedded in EXIF tag 0x927C. Each manufacturer uses a different format.

When Oxidex encounters a MakerNote tag during JPEG/TIFF parsing:

1. **Detects the camera make** from EXIF Make tag (0x010F)
2. **Dispatches to manufacturer parser** based on normalized make string
3. **Validates the header** (if parser provides validation)
4. **Extracts manufacturer-specific tags** (lens info, focus points, custom settings, etc.)
5. **Returns tags** with manufacturer prefix (e.g., "Canon:LensModel")

### Architecture

```
TIFF File Parser
    |
    +-- Read EXIF tags (including Make)
    |
    +-- Encounter MakerNote tag (0x927C)
    |
    +-- MakerNote Dispatcher
        |
        +-- Normalize make string (case-insensitive, trim whitespace)
        |
        +-- Match to parser:
            |
            +-- Canon Parser
            +-- Nikon Parser
            +-- Sony Parser
            +-- ... (40+ parsers)
            |
            +-- Parse binary data
            |
            +-- Extract tags with manufacturer prefix
```

## Tag Naming Convention

MakerNote tags use the format: `{Manufacturer}:{TagName}`

### Examples

**Camera Tags:**
- `Canon:LensModel` - Canon lens model string
- `Canon:FirmwareVersion` - Camera firmware version
- `Canon:SerialNumber` - Camera body serial number
- `Nikon:ShutterCount` - Nikon shutter actuation count
- `Sony:LensType` - Sony lens type ID
- `Olympus:FocusMode` - Focus mode setting
- `Pentax:ExposureCompensation` - Exposure compensation value

**Specialty Device Tags:**
- `DJI:FlightSpeed` - DJI drone flight speed
- `DJI:Altitude` - Drone altitude
- `FLIR:Temperature` - Thermal camera temperature reading
- `GoPro:FrameRate` - GoPro video frame rate
- `RED:Compression` - RED cinema camera compression ratio

**Software Tags:**
- `Photoshop:Quality` - JPEG quality setting
- `CaptureOne:Variant` - Capture One variant name
- `GIMP:Version` - GIMP version used

## Usage Examples

### CLI

```bash
# Extract all metadata including MakerNotes
oxidex photo.jpg

# Extract specific Canon MakerNote tags
oxidex -Canon:LensModel -Canon:SerialNumber canon_photo.jpg

# Extract Nikon-specific tags
oxidex -Nikon:ShutterCount -Nikon:LensType nikon_photo.jpg

# Filter for all Canon tags
oxidex photo.jpg | grep "Canon:"

# Extract DJI drone metadata
oxidex -DJI:FlightSpeed -DJI:Altitude -DJI:GimbalPitch drone_photo.jpg
```

### Rust API

```rust
use oxidex::core::MetadataMap;

// Extract metadata from a file
let metadata = MetadataMap::from_file("photo.jpg")?;

// Access standard EXIF tags
println!("Camera: {}", metadata.get("Make")?);
println!("Model: {}", metadata.get("Model")?);

// Access Canon-specific MakerNote tags
if let Ok(firmware) = metadata.get("Canon:FirmwareVersion") {
    println!("Canon Firmware: {}", firmware);
}
if let Ok(serial) = metadata.get("Canon:SerialNumber") {
    println!("Camera Serial: {}", serial);
}
if let Ok(lens) = metadata.get("Canon:LensModel") {
    println!("Lens: {}", lens);
}

// Access Nikon-specific tags
if let Ok(shutter_count) = metadata.get("Nikon:ShutterCount") {
    println!("Shutter Actuations: {}", shutter_count);
}
```

## Adding New Manufacturers

To add support for a new manufacturer:

### 1. Create Parser Module

Create a new file in `src/parsers/tiff/makernotes/{manufacturer}.rs`:

```rust
use crate::parsers::tiff::ifd_parser::ByteOrder;
use crate::parsers::tiff::makernotes::shared::MakerNoteParser;
use std::collections::HashMap;

pub struct ManufacturerParser;

impl MakerNoteParser for ManufacturerParser {
    fn validate_header(&self, data: &[u8]) -> bool {
        // Check for manufacturer-specific header signature
        data.len() >= 4 && &data[0..4] == b"MNFR"
    }

    fn parse(
        &self,
        data: &[u8],
        byte_order: ByteOrder,
        tags: &mut HashMap<String, String>,
    ) -> Result<(), String> {
        // Parse manufacturer-specific format
        // Extract tags and insert with prefix:
        tags.insert("Manufacturer:TagName".to_string(), "value".to_string());
        Ok(())
    }
}
```

### 2. Register in Module

Add to `src/parsers/tiff/makernotes/mod.rs`:

```rust
pub mod manufacturer;
```

### 3. Add to Dispatcher

Add to `src/parsers/tiff/makernote_dispatcher.rs` in the match statement:

```rust
"manufacturer" | "manufacturer inc." => {
    Some(Box::new(manufacturer::ManufacturerParser))
}
```

### 4. Add Tests

Add tests to verify parsing:

```rust
#[test]
fn test_manufacturer_parser() {
    let data = b"MNFRtest data";
    let mut tags = HashMap::new();
    let parser = ManufacturerParser;

    assert!(parser.validate_header(data));
    assert!(parser.parse(data, ByteOrder::LittleEndian, &mut tags).is_ok());
    assert!(!tags.is_empty());
}
```

### 5. Documentation

Update this document to include the new manufacturer in the supported list.

## Implementation Details

### Byte Order Handling

MakerNotes can use different byte orders than the main TIFF file:
- **Canon**: Typically little-endian
- **Nikon**: Uses "Nikon" header with embedded byte order
- **Olympus**: Uses TIFF-style byte order marker
- **Most others**: Inherit from parent TIFF

The dispatcher passes the TIFF byte order to each parser, which can override it if needed.

### Header Validation

Most manufacturers include a signature header in their MakerNote data:
- **Canon**: No specific header, starts with tag array
- **Nikon**: "Nikon\0" header followed by TIFF-like structure
- **Olympus**: "OLYMPUS\0" header
- **Panasonic**: "Panasonic\0" header
- **Sony**: No header, direct IFD structure

The `validate_header()` method allows each parser to verify data integrity before parsing.

### Error Handling

The dispatcher uses graceful error handling:
- Unknown manufacturers are silently skipped (not all cameras have MakerNotes)
- Invalid headers result in skipping (no tag extraction)
- Parse errors are logged but don't fail the entire EXIF read
- Malformed data returns Ok(()) with empty tag set

This ensures that MakerNote parsing errors don't prevent reading standard EXIF tags.

## Limitations

### Format Variations
- Not all camera makes have MakerNote parsers (unknown makes are silently skipped)
- Some manufacturers encrypt or obfuscate their MakerNote data (e.g., some Sony models)
- MakerNote formats may change between camera models within the same manufacturer
- Software-generated MakerNotes may have limited or non-standard metadata

### Data Quality
- MakerNote specifications are often undocumented or reverse-engineered
- Some tags may be camera model-specific and not present in all files
- Tag values may use proprietary encoding (e.g., bit flags, lookup tables)
- Date/time formats may vary from standard EXIF

### Performance
- MakerNote parsing adds overhead to EXIF reading (typically < 5% for most files)
- Large MakerNote blocks (e.g., Canon with extensive arrays) may impact performance
- Validation and header checking add minimal overhead but improve reliability

### Compatibility
- MakerNotes are manufacturer-specific and not standardized by EXIF
- Tag IDs may conflict between manufacturers (same ID, different meaning)
- Software updates may change MakerNote format without version indication

## Troubleshooting

### No MakerNote Tags Extracted

**Possible causes:**
1. Camera manufacturer not supported - check the supported list above
2. Invalid or corrupted MakerNote data - check file integrity
3. Encrypted MakerNote (some Sony, Samsung models)
4. Software-modified image with stripped MakerNotes

**Solutions:**
- Check `Make` tag value matches supported manufacturer
- Try with original unedited image from camera
- Enable verbose logging to see dispatcher output

### Incorrect Tag Values

**Possible causes:**
1. Byte order mismatch
2. Camera model using variant format
3. Tag registry out of date with latest camera models

**Solutions:**
- Compare with ExifTool output for validation
- File issue with sample image and camera model
- Check if camera firmware is recent (may use new format)

### Performance Issues

**Possible causes:**
1. Very large MakerNote blocks (e.g., embedded previews)
2. Parsing many files in sequence

**Solutions:**
- Use batch processing mode for multiple files
- Consider disabling MakerNote parsing if not needed
- File performance report with profiling data

## References

### Standards
- [EXIF 2.32 Specification](https://www.cipa.jp/std/documents/e/DC-X008-Translation-2019-E.pdf) - Official EXIF standard
- [TIFF 6.0 Specification](https://www.itu.int/itudoc/itu-t/com16/tiff-fx/docs/tiff6.pdf) - TIFF format (MakerNotes use TIFF-like IFD structures)

### Reference Implementations
- [ExifTool Tag Names](https://exiftool.org/TagNames/index.html) - Comprehensive tag database
- [ExifTool MakerNote Documentation](https://exiftool.org/TagNames/Canon.html) - Manufacturer-specific formats
- [libexif](https://libexif.github.io/) - Alternative EXIF library with MakerNote support

### Reverse Engineering Resources
- [Photography metadata research](https://photo.net/forums/) - Community reverse engineering
- [EXIF specification](https://www.exif.org/) - Official EXIF organization
- Camera manufacturer developer documentation (when available)

## Future Enhancements

### Planned Features
1. **Tag ID space optimization** - Dedicated range for MakerNote tags (0xE000-0xEFFF)
2. **Byte order auto-detection** - Detect MakerNote-specific byte order
3. **Lazy evaluation** - Parse MakerNotes only when accessed
4. **Caching** - Cache parsed MakerNotes for repeated access
5. **Streaming API** - Low-memory parsing for large MakerNote blocks

### Additional Manufacturers
- **Hasselblad** - H-series, X-series medium format
- **Blackmagic** - Cinema cameras
- **Arri** - Cinema cameras
- **Xiaomi** - Smartphone cameras
- **OnePlus** - Smartphone cameras

### Enhanced Capabilities
- **Embedded preview extraction** - Extract MakerNote preview images
- **Lens database integration** - Resolve lens IDs to names
- **GPS track parsing** - Extract GPS track data from action cameras
- **Video metadata** - MakerNote support for video files
- **Debugging tools** - MakerNote dump utility for analysis

## Contributing

We welcome contributions for new manufacturer parsers and improvements to existing ones:

1. Fork the repository
2. Create parser following the "Adding New Manufacturers" guide
3. Include test cases with sample images (if possible)
4. Submit pull request with documentation updates

Please include:
- Parser implementation
- Header validation logic
- Tag registry with descriptions
- Unit tests
- Sample image or hex dump (if shareable)

---

**Last Updated:** November 2025
**Oxidex Version:** 1.2.1+
**Supported Manufacturers:** 40+
**Estimated Tag Coverage:** 488+ manufacturer-specific tags
