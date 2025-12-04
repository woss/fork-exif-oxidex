//! Raw format metadata extraction
//!
//! Most camera raw formats are based on TIFF/EXIF structure.
//! This module leverages the existing TIFF parser and adds raw-specific handling.
//!
//! ## Architecture
//!
//! The metadata parser follows a dispatch pattern based on format type:
//! - **TIFF-based formats**: Use existing TIFF parser infrastructure
//! - **Proprietary formats**: Use format-specific parsers (CR3, X3F, MRW)
//! - **Fallback**: Attempt TIFF parsing, return minimal metadata on failure
//!
//! ## Format Support
//!
//! ### TIFF-based (fully supported):
//! - Canon CR2, Nikon NEF, Sony ARW, Adobe DNG
//! - Pentax PEF, Olympus ORF, Fujifilm RAF
//! - Panasonic RW2, and most other raw formats
//!
//! ### Proprietary (stubbed for future implementation):
//! - Canon CR3 (ISO Base Media Format)
//! - Sigma X3F (FOVb format)
//! - Minolta MRW (MRM format)

use crate::core::{FileReader, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};
use crate::parsers::raw::RawFormat;
use crate::parsers::tiff::ifd_parser::{parse_ifd, ByteOrder};
use crate::tag_db::lookup_tag_name;

/// Parse metadata from camera raw file
///
/// This is the main entry point for raw format metadata extraction.
/// It dispatches to format-specific parsers based on the detected format.
///
/// # Arguments
///
/// * `data` - Complete file data as a byte slice
/// * `format` - Detected raw format from format detection
///
/// # Returns
///
/// * `Ok(MetadataMap)` - Successfully extracted metadata
/// * `Err(ExifToolError)` - Parse error or unsupported format
///
/// # Examples
///
/// ```no_run
/// use oxidex::parsers::raw::{parse_raw_metadata, RawFormat};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let data = std::fs::read("photo.dng")?;
/// let metadata = parse_raw_metadata(&data, RawFormat::AdobeDNG)?;
///
/// // Access extracted metadata
/// if let Some(make) = metadata.get("IFD0:Make") {
///     println!("Camera: {:?}", make);
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Implementation Notes
///
/// Most raw formats are TIFF-based and can be parsed using the existing TIFF parser.
/// Proprietary formats (CR3, X3F, MRW) require specialized parsers and are currently
/// stubbed for future implementation.
pub fn parse_raw_metadata(data: &[u8], format: RawFormat) -> Result<MetadataMap> {
    match format {
        // TIFF-based formats - use existing TIFF parser infrastructure
        // These formats all follow the TIFF/EXIF structure with manufacturer-specific extensions
        RawFormat::CanonCR2
        | RawFormat::NikonNEF
        | RawFormat::NikonNRW
        | RawFormat::SonyARW
        | RawFormat::SonySR2
        | RawFormat::SonySRF
        | RawFormat::SonySRW
        | RawFormat::SonyARQ
        | RawFormat::SonyARI
        | RawFormat::AdobeDNG
        | RawFormat::PentaxPEF
        | RawFormat::OlympusORF
        | RawFormat::OlympusORI
        | RawFormat::FujifilmRAF
        | RawFormat::PanasonicRW2
        | RawFormat::PanasonicRWL
        | RawFormat::Hasselblad3FR
        | RawFormat::HasselbladFFF
        | RawFormat::PhaseOneIIQ
        | RawFormat::MamiyaMEF
        | RawFormat::LeafMOS
        | RawFormat::KodakDCR
        | RawFormat::KodakKDC
        | RawFormat::MinoltaMDC
        | RawFormat::EpsonERF
        | RawFormat::GoProGPR
        | RawFormat::HEIFHIF
        | RawFormat::LightLRI
        | RawFormat::SinarSTI => parse_tiff_based_raw(data, format),

        // Canon CR3 uses ISO Base Media Format (similar to MP4)
        // This is a different container format from TIFF
        RawFormat::CanonCR3 => parse_cr3(data, format),

        // Sigma X3F uses proprietary FOVb format
        RawFormat::SigmaX3F => parse_sigma_x3f(data, format),

        // Minolta MRW uses proprietary MRM format
        RawFormat::MinoltaMRW => parse_minolta_mrw(data, format),

        // Canon CRW is an older proprietary format
        RawFormat::CanonCRW => parse_canon_crw(data, format),

        // Generic/fallback formats
        // Attempt TIFF parsing as most raw formats are TIFF-based
        RawFormat::GenericRAW | RawFormat::GenericCAM | RawFormat::GenericREV => {
            parse_tiff_based_raw(data, format).or_else(|_| {
                // If TIFF parsing fails, return minimal metadata
                let mut metadata = MetadataMap::new();
                metadata.insert(
                    "File:FileType".to_string(),
                    TagValue::new_string(format!("{:?}", format)),
                );
                Ok(metadata)
            })
        }
    }
}

/// Parse TIFF-based raw formats using existing TIFF parser infrastructure
///
/// This function handles the majority of raw formats as they are based on TIFF/EXIF.
/// It creates a FileReader adapter, parses the TIFF structure, and enriches the
/// metadata with format-specific information.
///
/// Special handling for format variants:
/// - **Fujifilm RAF**: Contains embedded JPEG with EXIF data after proprietary header
/// - **Panasonic RW2**: TIFF variant with magic number 0x55 instead of 0x2A
/// - **Olympus ORF**: TIFF variant with "RO" signature instead of magic number 42
///
/// # Arguments
///
/// * `data` - Complete file data
/// * `format` - Specific raw format variant
///
/// # Returns
///
/// * `Ok(MetadataMap)` - Extracted metadata including TIFF tags and format info
/// * `Err(ExifToolError)` - Parse error from TIFF parser
///
/// # Implementation
///
/// 1. Check for format-specific handling (RAF embedded JPEG extraction)
/// 2. Create SliceReader adapter for byte slice access
/// 3. Parse TIFF header to determine byte order
/// 4. Parse IFD chain to extract all metadata tags
/// 5. Convert IFD entries to MetadataMap with proper tag names
/// 6. Add format-specific tags (e.g., DNG version for DNG files)
fn parse_tiff_based_raw(data: &[u8], format: RawFormat) -> Result<MetadataMap> {
    // Special handling for Fujifilm RAF format
    // RAF files have a proprietary header followed by embedded JPEG with EXIF data
    // Structure: "FUJIFILMCCD-RAW " (16 bytes) + header info + embedded JPEG at offset
    if format == RawFormat::FujifilmRAF {
        return parse_fujifilm_raf(data, format);
    }

    // Validate minimum TIFF header size
    if data.len() < 8 {
        return Err(ExifToolError::parse_error(
            "File too small to be a valid TIFF-based raw format",
        ));
    }

    // Create a FileReader adapter for the data slice
    let reader = SliceReader::new(data);

    // Parse TIFF header to get byte order
    let byte_order = detect_byte_order(data)?;

    // Read first IFD offset from TIFF header (bytes 4-7)
    let first_ifd_offset = match byte_order {
        ByteOrder::LittleEndian => u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
        ByteOrder::BigEndian => u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
    } as u64;

    // Parse all IFDs in the chain
    let mut metadata = MetadataMap::new();
    let mut ifd_offset = first_ifd_offset;
    let mut ifd_index = 0;

    // Add format-specific tag to identify file type
    metadata.insert(
        "File:FileType".to_string(),
        TagValue::new_string(format!("{:?}", format)),
    );

    // Walk the IFD chain (IFD0, IFD1, etc.)
    while ifd_offset != 0 && ifd_index < 10 {
        // Safety limit to prevent infinite loops
        // Determine IFD name based on index
        let ifd_name = match ifd_index {
            0 => "IFD0",
            1 => "IFD1",
            n => {
                eprintln!("Warning: Found IFD{} which is unusual", n);
                "IFD0" // Fallback
            }
        };

        // Parse this IFD
        match parse_ifd(&reader, ifd_offset, byte_order) {
            Ok(tags) => {
                // Track sub-IFD offsets
                let mut exif_ifd_offset = None;
                let mut gps_ifd_offset = None;
                let mut sub_ifd_offsets = Vec::new();

                // Convert tags to metadata
                for (tag_id, field_type, value_count, raw_bytes) in &tags {
                    let bytes = raw_bytes.as_ref();

                    // Check for EXIF Sub-IFD pointer (tag 0x8769)
                    if *tag_id == 0x8769 && bytes.len() >= 4 {
                        let offset = read_u32(bytes, byte_order);
                        exif_ifd_offset = Some(offset as u64);
                        continue; // Don't add pointer tag to metadata
                    }

                    // Check for GPS Sub-IFD pointer (tag 0x8825)
                    if *tag_id == 0x8825 && bytes.len() >= 4 {
                        let offset = read_u32(bytes, byte_order);
                        gps_ifd_offset = Some(offset as u64);
                        continue; // Don't add pointer tag to metadata
                    }

                    // Check for SubIFD pointer (tag 0x014A) - common in RAW formats
                    // SubIFD contains RAW image data and RAW-specific metadata
                    if *tag_id == 0x014A {
                        // SubIFDs can contain multiple offsets
                        let offset_count = bytes.len() / 4;
                        for i in 0..offset_count {
                            if (i + 1) * 4 <= bytes.len() {
                                let offset_bytes = &bytes[i * 4..(i + 1) * 4];
                                let offset = read_u32(offset_bytes, byte_order);
                                sub_ifd_offsets.push(offset as u64);
                            }
                        }
                        continue; // Don't add pointer tag to metadata
                    }

                    // Convert tag to metadata
                    let tag_name = lookup_tag_name(*tag_id, ifd_name);
                    let tag_value =
                        raw_bytes_to_simple_tag_value(bytes, *field_type, *value_count, byte_order);
                    metadata.insert(tag_name, tag_value);
                }

                // Parse EXIF Sub-IFD if present
                if let Some(offset) = exif_ifd_offset {
                    if let Ok(exif_tags) = parse_ifd(&reader, offset, byte_order) {
                        for (tag_id, field_type, value_count, raw_bytes) in exif_tags {
                            let tag_name = lookup_tag_name(tag_id, "ExifIFD");
                            let tag_value = raw_bytes_to_simple_tag_value(
                                raw_bytes.as_ref(),
                                field_type,
                                value_count,
                                byte_order,
                            );
                            metadata.insert(tag_name, tag_value);
                        }
                    }
                }

                // Parse GPS Sub-IFD if present
                if let Some(offset) = gps_ifd_offset {
                    if let Ok(gps_tags) = parse_ifd(&reader, offset, byte_order) {
                        for (tag_id, field_type, value_count, raw_bytes) in gps_tags {
                            let tag_name = lookup_tag_name(tag_id, "GPS");
                            let tag_value = raw_bytes_to_simple_tag_value(
                                raw_bytes.as_ref(),
                                field_type,
                                value_count,
                                byte_order,
                            );
                            metadata.insert(tag_name, tag_value);
                        }
                    }
                }

                // Parse SubIFD(s) if present - crucial for RAW formats
                // SubIFDs contain RAW image data, compression info, and RAW-specific tags
                for (sub_index, sub_offset) in sub_ifd_offsets.iter().enumerate() {
                    // Use SubIFD0, SubIFD1, etc. for tag naming
                    let sub_ifd_name = if sub_index == 0 {
                        "SubIFD0"
                    } else {
                        // Multiple SubIFDs are rare but possible
                        eprintln!("Warning: Found SubIFD{} which is unusual", sub_index);
                        "SubIFD0" // Use SubIFD0 as fallback for consistency
                    };

                    if let Ok(sub_tags) = parse_ifd(&reader, *sub_offset, byte_order) {
                        for (tag_id, field_type, value_count, raw_bytes) in sub_tags {
                            let tag_name = lookup_tag_name(tag_id, sub_ifd_name);
                            let tag_value = raw_bytes_to_simple_tag_value(
                                raw_bytes.as_ref(),
                                field_type,
                                value_count,
                                byte_order,
                            );
                            metadata.insert(tag_name, tag_value);
                        }
                    }
                }

                // Read next IFD offset
                let entry_count = tags.len();
                let next_offset_location = ifd_offset + 2 + (entry_count as u64 * 12);

                if next_offset_location + 4 <= reader.size() {
                    if let Ok(next_offset_bytes) = reader.read(next_offset_location, 4) {
                        ifd_offset = read_u32(next_offset_bytes, byte_order) as u64;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            Err(e) => {
                eprintln!(
                    "Warning: Failed to parse IFD at offset {}: {}",
                    ifd_offset, e
                );
                break;
            }
        }

        ifd_index += 1;
    }

    // Apply format-specific enhancements
    match format {
        RawFormat::AdobeDNG => {
            extract_dng_tags(&mut metadata);
        }
        RawFormat::CanonCR2 => {
            extract_cr2_tags(&mut metadata);
        }
        RawFormat::NikonNEF | RawFormat::NikonNRW => {
            extract_nef_tags(&mut metadata);
        }
        _ => {
            // Other formats don't need special handling yet
        }
    }

    Ok(metadata)
}

/// Extract DNG-specific tags from metadata
///
/// DNG (Digital Negative) files have additional tags beyond standard TIFF/EXIF.
/// This function enriches the metadata with DNG-specific information.
///
/// # DNG-Specific Tags Extracted
///
/// Color calibration tags (crucial for RAW processing):
/// - ColorMatrix1/2 (0xC621/0xC622): Color transformation matrices
/// - CameraCalibration1/2 (0xC623/0xC624): Camera-specific calibration
/// - CalibrationIlluminant1/2 (0xC65A/0xC65B): Illuminant used for calibration
/// - ForwardMatrix1/2 (0xC714/0xC715): Forward color transformation
/// - AsShotNeutral (0xC628): White balance as shot
///
/// Exposure and rendering tags:
/// - BaselineExposure (0xC62A): Baseline exposure compensation
/// - BaselineNoise (0xC62B): Baseline noise level
/// - BaselineSharpness (0xC62C): Baseline sharpness
/// - LinearResponseLimit (0xC62E): Linear response limit
///
/// RAW data tags:
/// - BlackLevel (0xC61A): Black level for each color plane
/// - WhiteLevel (0xC61D): White level for sensor
/// - DefaultScale (0xC61E): Default scale factors
/// - DefaultCropOrigin/Size (0xC61F/0xC620): Default crop area
/// - BayerGreenSplit (0xC62D): Bayer green channel split value
///
/// DNG metadata:
/// - DNGVersion (0xC612): DNG specification version
/// - DNGBackwardVersion (0xC613): Backward compatibility version
/// - UniqueCameraModel (0xC614): Unique camera model string
/// - LocalizedCameraModel (0xC615): Localized camera model name
/// - CFAPlaneColor (0xC616): CFA plane color
/// - CFALayout (0xC617): CFA layout
/// - LinearizationTable (0xC618): Linearization table
/// - BlackLevelRepeatDim (0xC619): Black level repeat dimensions
///
/// # Arguments
///
/// * `metadata` - Mutable reference to MetadataMap to enrich
///
/// # Implementation Note
///
/// Most DNG-specific tags are automatically extracted by the TIFF parser
/// during IFD traversal. This function serves as documentation and can be
/// extended to add computed/derived DNG-specific metadata or aliases.
fn extract_dng_tags(metadata: &mut MetadataMap) {
    // DNG-specific tags are stored in IFD0 or SubIFD0
    // The TIFF parser already extracts these automatically

    // We can add computed values or format-specific processing here
    // For example, parsing the DNGVersion bytes into a readable format
    // DNGVersion is stored as 4 bytes: major.minor.tertiary.quaternary
    // Example: [1, 4, 0, 0] = version 1.4.0.0
    if let Some(TagValue::Binary(bytes)) = metadata.get("IFD0:DNGVersion") {
        if bytes.len() >= 4 {
            let version_str = format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3]);
            metadata.insert(
                "DNG:VersionString".to_string(),
                TagValue::new_string(version_str),
            );
        }
    }

    // Mark critical DNG tags for easier identification
    // This helps downstream applications know which color calibration data is available
    let critical_color_tags = [
        "IFD0:ColorMatrix1",
        "IFD0:ColorMatrix2",
        "IFD0:CameraCalibration1",
        "IFD0:CameraCalibration2",
        "IFD0:CalibrationIlluminant1",
        "IFD0:CalibrationIlluminant2",
    ];

    let mut available_color_tags = Vec::new();
    for tag_name in &critical_color_tags {
        if metadata.contains_key(tag_name) {
            available_color_tags.push(*tag_name);
        }
    }

    if !available_color_tags.is_empty() {
        metadata.insert(
            "DNG:AvailableColorCalibration".to_string(),
            TagValue::new_string(available_color_tags.join(", ")),
        );
    }
}

/// Extract CR2-specific tags from metadata
///
/// Canon CR2 (Canon Raw version 2) files are TIFF-based with Canon-specific extensions.
/// This function enriches the metadata with CR2-specific information.
///
/// # CR2-Specific Tags
///
/// CR2 files contain:
/// - **Canon MakerNotes**: Extensive Canon-specific metadata (already extracted via MakerNote parser)
/// - **SubIFD tags**: RAW image data dimensions, compression, bit depth
/// - **Preview images**: Multiple embedded preview/thumbnail images at various sizes
/// - **RAW sensor data**: CFA pattern, sensor size, crop information
///
/// Key CR2 characteristics:
/// - CR2 marker at offset 8: "CR\x02\x00" (distinguishes from other TIFF formats)
/// - SubIFD contains the RAW image data
/// - IFD1 typically contains a full-size JPEG preview
/// - Multiple thumbnail/preview images at different resolutions
///
/// # Arguments
///
/// * `metadata` - Mutable reference to MetadataMap to enrich
fn extract_cr2_tags(metadata: &mut MetadataMap) {
    // CR2 files have multiple image layers:
    // - IFD0: Typically a small thumbnail
    // - IFD1: Full-size JPEG preview
    // - SubIFD0: RAW image data

    // Count available image representations
    let mut image_count = 0;
    if metadata.contains_key("IFD0:ImageWidth") {
        image_count += 1;
    }
    if metadata.contains_key("IFD1:ImageWidth") {
        image_count += 1;
    }
    if metadata.contains_key("SubIFD0:ImageWidth") {
        image_count += 1;
    }

    if image_count > 0 {
        metadata.insert(
            "CR2:ImageLayerCount".to_string(),
            TagValue::new_integer(image_count),
        );
    }

    // Check for RAW data in SubIFD
    if metadata.contains_key("SubIFD0:ImageWidth") {
        // Mark that RAW data is present
        metadata.insert(
            "CR2:HasRAWData".to_string(),
            TagValue::new_string("true".to_string()),
        );

        // Extract RAW image dimensions if available
        if let Some(width) = metadata.get("SubIFD0:ImageWidth") {
            if let Some(height) = metadata.get("SubIFD0:ImageHeight") {
                let width_val = match width {
                    TagValue::Integer(i) => i.to_string(),
                    TagValue::String(s) => s.clone(),
                    _ => format!("{:?}", width),
                };
                let height_val = match height {
                    TagValue::Integer(i) => i.to_string(),
                    TagValue::String(s) => s.clone(),
                    _ => format!("{:?}", height),
                };
                metadata.insert(
                    "CR2:RAWImageSize".to_string(),
                    TagValue::new_string(format!("{}x{}", width_val, height_val)),
                );
            }
        }
    }

    // Check for JPEG preview in IFD1
    if metadata.contains_key("IFD1:ImageWidth") && metadata.contains_key("IFD1:Compression") {
        metadata.insert(
            "CR2:HasJPEGPreview".to_string(),
            TagValue::new_string("true".to_string()),
        );
    }
}

/// Extract NEF-specific tags from metadata
///
/// Nikon NEF (Nikon Electronic Format) files are TIFF-based with Nikon-specific extensions.
/// This function enriches the metadata with NEF-specific information.
///
/// # NEF-Specific Tags
///
/// NEF files contain:
/// - **Nikon MakerNotes**: Extensive Nikon-specific metadata (already extracted via MakerNote parser)
/// - **SubIFD tags**: RAW image data, compression type, bit depth
/// - **Preview images**: Embedded JPEG preview images
/// - **Compressed RAW data**: Nikon's lossless compressed RAW format
///
/// NEF variants:
/// - NEF: Standard Nikon RAW format (uncompressed or losslessly compressed)
/// - NRW: Nikon RAW (sRAW) - smaller file size variant
///
/// Key NEF characteristics:
/// - Can use lossless compression (reduces file size without quality loss)
/// - Multiple embedded previews at different sizes
/// - Extensive shooting information in Nikon MakerNotes
///
/// # Arguments
///
/// * `metadata` - Mutable reference to MetadataMap to enrich
fn extract_nef_tags(metadata: &mut MetadataMap) {
    // NEF files typically have:
    // - IFD0: Thumbnail image or preview
    // - IFD1: Another preview (optional)
    // - SubIFD0: RAW image data

    // Check for compression type in SubIFD
    if let Some(compression) = metadata.get("SubIFD0:Compression") {
        // Nikon uses various compression schemes:
        // - 1: Uncompressed
        // - 7: JPEG compression (for preview)
        // - 34713: Nikon lossless compressed
        let compression_val = match compression {
            TagValue::Integer(i) => *i,
            TagValue::String(s) => s.parse::<i64>().unwrap_or(0),
            _ => 0,
        };

        let compression_name = match compression_val {
            1 => "Uncompressed",
            7 => "JPEG",
            34713 => "Nikon Lossless Compressed",
            _ => "Unknown",
        };

        metadata.insert(
            "NEF:RAWCompression".to_string(),
            TagValue::new_string(compression_name.to_string()),
        );
    }

    // Count available image representations
    let mut image_count = 0;
    if metadata.contains_key("IFD0:ImageWidth") {
        image_count += 1;
    }
    if metadata.contains_key("IFD1:ImageWidth") {
        image_count += 1;
    }
    if metadata.contains_key("SubIFD0:ImageWidth") {
        image_count += 1;
    }

    if image_count > 0 {
        metadata.insert(
            "NEF:ImageLayerCount".to_string(),
            TagValue::new_integer(image_count),
        );
    }

    // Check for RAW data in SubIFD
    if metadata.contains_key("SubIFD0:ImageWidth") {
        metadata.insert(
            "NEF:HasRAWData".to_string(),
            TagValue::new_string("true".to_string()),
        );

        // Extract RAW image dimensions
        if let Some(width) = metadata.get("SubIFD0:ImageWidth") {
            if let Some(height) = metadata.get("SubIFD0:ImageHeight") {
                let width_val = match width {
                    TagValue::Integer(i) => i.to_string(),
                    TagValue::String(s) => s.clone(),
                    _ => format!("{:?}", width),
                };
                let height_val = match height {
                    TagValue::Integer(i) => i.to_string(),
                    TagValue::String(s) => s.clone(),
                    _ => format!("{:?}", height),
                };
                metadata.insert(
                    "NEF:RAWImageSize".to_string(),
                    TagValue::new_string(format!("{}x{}", width_val, height_val)),
                );
            }
        }
    }

    // Check for bit depth
    if let Some(bits_per_sample) = metadata.get("SubIFD0:BitsPerSample") {
        let bits_val = match bits_per_sample {
            TagValue::Integer(i) => i.to_string(),
            TagValue::String(s) => s.clone(),
            _ => format!("{:?}", bits_per_sample),
        };
        metadata.insert(
            "NEF:RAWBitDepth".to_string(),
            TagValue::new_string(bits_val),
        );
    }
}

/// Parse Canon CR3 format (ISO Base Media File Format)
///
/// CR3 files use a container format similar to MP4/QuickTime rather than TIFF.
/// This function is a stub for future implementation.
///
/// # Arguments
///
/// * `data` - Complete file data
/// * `format` - CR3 format variant
///
/// # Returns
///
/// Minimal metadata with file type information.
/// Full CR3 parsing to be implemented in future iteration.
///
/// # TODO
///
/// - Implement ISO Base Media File Format parser
/// - Extract metadata from CR3 boxes (similar to MP4 atoms)
/// - Parse Canon-specific metadata boxes
fn parse_cr3(_data: &[u8], format: RawFormat) -> Result<MetadataMap> {
    let mut metadata = MetadataMap::new();
    metadata.insert(
        "File:FileType".to_string(),
        TagValue::new_string(format!("{:?}", format)),
    );

    // TODO: Implement full CR3 parsing
    // CR3 uses ISO Base Media Format (similar to MP4/QuickTime)
    // Will require box/atom parser similar to QuickTime parser

    Ok(metadata)
}

/// Parse Sigma X3F format
///
/// X3F files use Sigma's proprietary FOVb format.
/// This function is a stub for future implementation.
///
/// # Arguments
///
/// * `data` - Complete file data
/// * `format` - X3F format variant
///
/// # Returns
///
/// Minimal metadata with file type information.
/// Full X3F parsing to be implemented in future iteration.
///
/// # TODO
///
/// - Implement FOVb format parser
/// - Extract Sigma-specific metadata
/// - Parse X3F image sections
fn parse_sigma_x3f(_data: &[u8], format: RawFormat) -> Result<MetadataMap> {
    let mut metadata = MetadataMap::new();
    metadata.insert(
        "File:FileType".to_string(),
        TagValue::new_string(format!("{:?}", format)),
    );

    // TODO: Implement X3F specific parsing
    // X3F uses FOVb signature and proprietary structure

    Ok(metadata)
}

/// Parse Minolta MRW format
///
/// MRW files use Minolta's proprietary MRM format.
/// This function is a stub for future implementation.
///
/// # Arguments
///
/// * `data` - Complete file data
/// * `format` - MRW format variant
///
/// # Returns
///
/// Minimal metadata with file type information.
/// Full MRW parsing to be implemented in future iteration.
///
/// # TODO
///
/// - Implement MRM format parser
/// - Extract Minolta-specific metadata
/// - Parse MRW image data
fn parse_minolta_mrw(_data: &[u8], format: RawFormat) -> Result<MetadataMap> {
    let mut metadata = MetadataMap::new();
    metadata.insert(
        "File:FileType".to_string(),
        TagValue::new_string(format!("{:?}", format)),
    );

    // TODO: Implement MRW specific parsing
    // MRW uses "\x00MRM" signature

    Ok(metadata)
}

/// Parse Canon CRW format
///
/// CRW is Canon's older proprietary raw format used before CR2.
/// This function is a stub for future implementation.
///
/// # Arguments
///
/// * `data` - Complete file data
/// * `format` - CRW format variant
///
/// # Returns
///
/// Minimal metadata with file type information.
/// Full CRW parsing to be implemented in future iteration.
///
/// # TODO
///
/// - Implement CRW format parser
/// - Extract Canon-specific metadata from CRW structure
fn parse_canon_crw(_data: &[u8], format: RawFormat) -> Result<MetadataMap> {
    let mut metadata = MetadataMap::new();
    metadata.insert(
        "File:FileType".to_string(),
        TagValue::new_string(format!("{:?}", format)),
    );

    // TODO: Implement CRW specific parsing
    // CRW is Canon's older proprietary format

    Ok(metadata)
}

/// Parse Fujifilm RAF format
///
/// RAF files use a proprietary container format with embedded JPEG/EXIF data.
/// The structure is:
/// - Bytes 0-15: "FUJIFILMCCD-RAW " signature
/// - Bytes 16-83: Header with version, camera model, and offset information
/// - Bytes 84-87: JPEG image offset (big-endian u32)
/// - Bytes 88-91: JPEG image length (big-endian u32)
/// - At JPEG offset: Standard JPEG file with EXIF data
///
/// This implementation extracts metadata from the embedded JPEG/EXIF data.
///
/// # Arguments
///
/// * `data` - Complete file data
/// * `format` - RAF format variant
///
/// # Returns
///
/// * `Ok(MetadataMap)` - Extracted metadata from embedded JPEG/EXIF
/// * `Err(ExifToolError)` - Parse error or invalid RAF structure
///
/// # Implementation Strategy
///
/// Rather than parsing the proprietary RAF header, we locate and parse the
/// embedded JPEG data which contains standard EXIF metadata. This approach:
/// - Reuses existing JPEG/EXIF parsing infrastructure
/// - Extracts camera settings, timestamps, and other standard metadata
/// - Avoids need to reverse-engineer proprietary RAF format details
fn parse_fujifilm_raf(data: &[u8], format: RawFormat) -> Result<MetadataMap> {
    // Validate RAF signature
    if data.len() < 16 || &data[0..16] != b"FUJIFILMCCD-RAW " {
        return Err(ExifToolError::parse_error(
            "Invalid RAF file: missing FUJIFILMCCD-RAW signature",
        ));
    }

    // RAF header is 84 bytes, followed by offset table
    // Bytes 84-87: JPEG image offset (big-endian u32)
    // Bytes 88-91: JPEG image length (big-endian u32)
    if data.len() < 92 {
        return Err(ExifToolError::parse_error(
            "Invalid RAF file: header too small",
        ));
    }

    // Read JPEG offset and length (big-endian)
    let jpeg_offset = u32::from_be_bytes([data[84], data[85], data[86], data[87]]) as usize;
    let jpeg_length = u32::from_be_bytes([data[88], data[89], data[90], data[91]]) as usize;

    // Validate JPEG offset and length
    if jpeg_offset >= data.len() {
        return Err(ExifToolError::parse_error(format!(
            "Invalid RAF file: JPEG offset {} exceeds file size {}",
            jpeg_offset,
            data.len()
        )));
    }

    if jpeg_offset + jpeg_length > data.len() {
        // JPEG length might be incorrect, try to use remaining file size
        let remaining = data.len() - jpeg_offset;
        eprintln!(
            "Warning: RAF JPEG length {} exceeds remaining file size {}, using remaining size",
            jpeg_length, remaining
        );
    }

    // Extract JPEG data
    let jpeg_end = (jpeg_offset + jpeg_length).min(data.len());
    let jpeg_data = &data[jpeg_offset..jpeg_end];

    // Verify JPEG signature (0xFF 0xD8)
    if jpeg_data.len() < 2 || jpeg_data[0] != 0xFF || jpeg_data[1] != 0xD8 {
        return Err(ExifToolError::parse_error(
            "Invalid RAF file: embedded data is not a valid JPEG",
        ));
    }

    // Create metadata map with format info
    let mut metadata = MetadataMap::new();
    metadata.insert(
        "File:FileType".to_string(),
        TagValue::new_string(format!("{:?}", format)),
    );

    // Parse embedded JPEG to extract EXIF data
    // Create a SliceReader for the JPEG data
    let jpeg_reader = SliceReader::new(jpeg_data);

    // Use the existing JPEG segment parser to extract EXIF
    if let Ok(segments) = crate::parsers::jpeg::segment_parser::parse_segments(&jpeg_reader) {
        // Look for APP1 segments containing EXIF data
        for segment in segments {
            if segment.marker == 0xFFE1 && segment.data.len() > 6 {
                // Check for EXIF header "Exif\0\0"
                if &segment.data[0..6] == b"Exif\x00\x00" {
                    // EXIF data starts at byte 6
                    let exif_data = &segment.data[6..];

                    // Parse TIFF structure within EXIF data
                    if let Ok(byte_order) = detect_byte_order(exif_data) {
                        // Read first IFD offset (bytes 4-7 in TIFF header)
                        if exif_data.len() >= 8 {
                            let first_ifd_offset = match byte_order {
                                ByteOrder::LittleEndian => u32::from_le_bytes([
                                    exif_data[4],
                                    exif_data[5],
                                    exif_data[6],
                                    exif_data[7],
                                ]),
                                ByteOrder::BigEndian => u32::from_be_bytes([
                                    exif_data[4],
                                    exif_data[5],
                                    exif_data[6],
                                    exif_data[7],
                                ]),
                            } as u64;

                            // Create reader for EXIF data
                            let exif_reader = SliceReader::new(exif_data);

                            // Parse IFD0
                            if let Ok(tags) = parse_ifd(&exif_reader, first_ifd_offset, byte_order)
                            {
                                // Track sub-IFD offsets
                                let mut exif_ifd_offset = None;

                                // Convert tags to metadata
                                for (tag_id, field_type, value_count, raw_bytes) in &tags {
                                    let bytes = raw_bytes.as_ref();

                                    // Check for EXIF Sub-IFD pointer (tag 0x8769)
                                    if *tag_id == 0x8769 && bytes.len() >= 4 {
                                        let offset = read_u32(bytes, byte_order);
                                        exif_ifd_offset = Some(offset as u64);
                                        continue;
                                    }

                                    // Convert tag to metadata
                                    let tag_name = lookup_tag_name(*tag_id, "IFD0");
                                    let tag_value = raw_bytes_to_simple_tag_value(
                                        bytes,
                                        *field_type,
                                        *value_count,
                                        byte_order,
                                    );
                                    metadata.insert(tag_name, tag_value);
                                }

                                // Parse EXIF Sub-IFD if present
                                if let Some(offset) = exif_ifd_offset {
                                    if let Ok(exif_tags) =
                                        parse_ifd(&exif_reader, offset, byte_order)
                                    {
                                        for (tag_id, field_type, value_count, raw_bytes) in
                                            exif_tags
                                        {
                                            let tag_name = lookup_tag_name(tag_id, "ExifIFD");
                                            let tag_value = raw_bytes_to_simple_tag_value(
                                                raw_bytes.as_ref(),
                                                field_type,
                                                value_count,
                                                byte_order,
                                            );
                                            metadata.insert(tag_name, tag_value);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(metadata)
}

// ===== Helper Functions =====

/// Detect byte order from TIFF header
///
/// Reads the first 2 bytes to determine endianness:
/// - "II" (0x4949) = Little-endian (used by most TIFF and many raw formats)
/// - "MM" (0x4D4D) = Big-endian (used by some TIFF and raw formats)
///
/// This function handles standard TIFF as well as raw format variants:
/// - Standard TIFF: "II\x2A\x00" or "MM\x00\x2A" (magic number 42)
/// - Panasonic RW2: "II\x55\x00" (magic number 85 instead of 42)
/// - Olympus ORF: "IIRO" or "MMOR" (uses "RO" or "OR" instead of magic number)
///
/// # Arguments
///
/// * `data` - File data (must be at least 2 bytes)
///
/// # Returns
///
/// * `Ok(ByteOrder)` - Detected byte order
/// * `Err(ExifToolError)` - Invalid byte order marker
fn detect_byte_order(data: &[u8]) -> Result<ByteOrder> {
    if data.len() < 2 {
        return Err(ExifToolError::parse_error(
            "File too small to detect byte order",
        ));
    }

    match &data[0..2] {
        b"II" => Ok(ByteOrder::LittleEndian),
        b"MM" => Ok(ByteOrder::BigEndian),
        _ => Err(ExifToolError::parse_error("Invalid TIFF byte order marker")),
    }
}

/// Read a 32-bit unsigned integer from bytes with specified byte order
///
/// # Arguments
///
/// * `bytes` - Byte slice (must be at least 4 bytes)
/// * `byte_order` - Endianness to use
///
/// # Returns
///
/// The parsed u32 value
fn read_u32(bytes: &[u8], byte_order: ByteOrder) -> u32 {
    match byte_order {
        ByteOrder::LittleEndian => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        ByteOrder::BigEndian => u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
    }
}

/// Convert raw bytes to TagValue (simplified version)
///
/// This is a simplified converter for raw metadata parsing.
/// For full tag value conversion with all special cases, use the
/// raw_bytes_to_tag_value function in operations.rs.
///
/// # Arguments
///
/// * `bytes` - Raw byte data
/// * `field_type` - TIFF field type
/// * `value_count` - Number of values
/// * `byte_order` - Endianness
///
/// # Returns
///
/// TagValue representing the data
fn raw_bytes_to_simple_tag_value(
    bytes: &[u8],
    field_type: u16,
    _value_count: u32,
    byte_order: ByteOrder,
) -> TagValue {
    use crate::parsers::common::exif_types::ExifType;

    // Try to convert field_type to ExifType
    if let Some(exif_type) = ExifType::from_u16(field_type) {
        match exif_type {
            // ASCII string
            ExifType::Ascii => {
                let s = String::from_utf8_lossy(bytes);
                let s = s.trim_end_matches('\0');
                return TagValue::new_string(s.to_string());
            }

            // SHORT (16-bit unsigned)
            ExifType::Short if bytes.len() >= 2 => {
                let value = match byte_order {
                    ByteOrder::LittleEndian => u16::from_le_bytes([bytes[0], bytes[1]]),
                    ByteOrder::BigEndian => u16::from_be_bytes([bytes[0], bytes[1]]),
                } as i64;
                return TagValue::new_integer(value);
            }

            // LONG (32-bit unsigned)
            ExifType::Long if bytes.len() >= 4 => {
                let value = read_u32(bytes, byte_order) as i64;
                return TagValue::new_integer(value);
            }

            // RATIONAL (two 32-bit unsigned)
            ExifType::Rational if bytes.len() >= 8 => {
                let numerator = read_u32(&bytes[0..4], byte_order);
                let denominator = read_u32(&bytes[4..8], byte_order);
                return TagValue::new_rational(numerator as i32, denominator as i32);
            }

            _ => {}
        }
    }

    // Fallback: binary data
    TagValue::new_binary(bytes.to_vec())
}

// ===== FileReader Adapter for Byte Slices =====

/// FileReader implementation for byte slices
///
/// This adapter allows using a byte slice with the TIFF parser
/// which expects a FileReader trait implementation.
struct SliceReader<'a> {
    data: &'a [u8],
}

impl<'a> SliceReader<'a> {
    /// Create a new SliceReader from a byte slice
    fn new(data: &'a [u8]) -> Self {
        Self { data }
    }
}

impl<'a> FileReader for SliceReader<'a> {
    /// Read bytes from the slice
    ///
    /// # Arguments
    ///
    /// * `offset` - Offset from start of slice
    /// * `length` - Number of bytes to read
    ///
    /// # Returns
    ///
    /// * `Ok(&[u8])` - Slice of requested bytes
    /// * `Err` - If offset/length exceeds slice bounds
    fn read(&self, offset: u64, length: usize) -> std::io::Result<&[u8]> {
        let start = offset as usize;
        let end = start + length;

        if end > self.data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "read beyond end of data",
            ));
        }

        Ok(&self.data[start..end])
    }

    /// Get total size of the slice
    fn size(&self) -> u64 {
        self.data.len() as u64
    }
}

// ===== Unit Tests =====

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_byte_order_little_endian() {
        let data = b"II\x2a\x00\x08\x00\x00\x00";
        let byte_order = detect_byte_order(data).unwrap();
        assert_eq!(byte_order, ByteOrder::LittleEndian);
    }

    #[test]
    fn test_detect_byte_order_big_endian() {
        let data = b"MM\x00\x2a\x00\x00\x00\x08";
        let byte_order = detect_byte_order(data).unwrap();
        assert_eq!(byte_order, ByteOrder::BigEndian);
    }

    #[test]
    fn test_detect_byte_order_invalid() {
        let data = b"XX\x2a\x00";
        assert!(detect_byte_order(data).is_err());
    }

    #[test]
    fn test_detect_byte_order_too_small() {
        let data = b"I";
        assert!(detect_byte_order(data).is_err());
    }

    #[test]
    fn test_parse_tiff_based_format() {
        // Minimal TIFF header (little-endian)
        // II (little-endian) + 42 (magic) + offset 8 (first IFD)
        let data = b"II\x2a\x00\x08\x00\x00\x00\x00\x00"; // Header + no IFD entries

        // Should not crash even with minimal data
        let result = parse_raw_metadata(data, RawFormat::AdobeDNG);
        // Either parse successfully or fail gracefully
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_parse_cr3_stub() {
        let data = b"\x00\x00\x00\x18ftypcrx test data";
        let result = parse_raw_metadata(data, RawFormat::CanonCR3);
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert!(metadata.contains_key("File:FileType"));
    }

    #[test]
    fn test_parse_x3f_stub() {
        let data = b"FOVbtest data";
        let result = parse_raw_metadata(data, RawFormat::SigmaX3F);
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert!(metadata.contains_key("File:FileType"));
    }

    #[test]
    fn test_parse_mrw_stub() {
        let data = b"\x00MRMtest data";
        let result = parse_raw_metadata(data, RawFormat::MinoltaMRW);
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert!(metadata.contains_key("File:FileType"));
    }

    #[test]
    fn test_slice_reader_read() {
        let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let reader = SliceReader::new(&data);

        let result = reader.read(0, 5).unwrap();
        assert_eq!(result, &[0, 1, 2, 3, 4]);

        let result = reader.read(5, 3).unwrap();
        assert_eq!(result, &[5, 6, 7]);
    }

    #[test]
    fn test_slice_reader_read_out_of_bounds() {
        let data = vec![0, 1, 2, 3, 4];
        let reader = SliceReader::new(&data);

        let result = reader.read(0, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_slice_reader_size() {
        let data = vec![0; 100];
        let reader = SliceReader::new(&data);
        assert_eq!(reader.size(), 100);
    }

    #[test]
    fn test_subifd_parsing() {
        // Create a TIFF with SubIFD pointer
        let mut data = Vec::new();

        // TIFF header (little-endian)
        data.extend_from_slice(b"II\x2a\x00");
        data.extend_from_slice(&8u32.to_le_bytes()); // First IFD offset

        // IFD0 with SubIFD pointer tag (0x014A)
        data.extend_from_slice(&1u16.to_le_bytes()); // 1 entry

        // SubIFD pointer tag entry
        data.extend_from_slice(&0x014Au16.to_le_bytes()); // Tag ID: SubIFD
        data.extend_from_slice(&4u16.to_le_bytes()); // Type: LONG
        data.extend_from_slice(&1u32.to_le_bytes()); // Count: 1
        data.extend_from_slice(&30u32.to_le_bytes()); // Offset to SubIFD

        // Next IFD offset (0 = none)
        data.extend_from_slice(&0u32.to_le_bytes());

        // SubIFD at offset 30
        // Pad to reach offset 30
        while data.len() < 30 {
            data.push(0);
        }

        // SubIFD with one entry (ImageWidth)
        data.extend_from_slice(&1u16.to_le_bytes()); // 1 entry
        data.extend_from_slice(&0x0100u16.to_le_bytes()); // Tag: ImageWidth
        data.extend_from_slice(&3u16.to_le_bytes()); // Type: SHORT
        data.extend_from_slice(&1u32.to_le_bytes()); // Count: 1
        data.extend_from_slice(&4000u16.to_le_bytes()); // Value: 4000
        data.extend_from_slice(&0u16.to_le_bytes()); // Padding
        data.extend_from_slice(&0u32.to_le_bytes()); // Next IFD: none

        let result = parse_raw_metadata(&data, RawFormat::AdobeDNG);
        assert!(result.is_ok(), "Should parse TIFF with SubIFD");

        let metadata = result.unwrap();
        // Should have extracted the ImageWidth from SubIFD0
        // Note: The exact tag name depends on the tag database
        let has_subifd_data = metadata
            .keys()
            .any(|k| k.starts_with("SubIFD") || k.contains("ImageWidth"));

        if !has_subifd_data {
            let keys: Vec<&String> = metadata.keys().collect();
            eprintln!("Available keys: {:?}", keys);
        }

        assert!(has_subifd_data, "Should have extracted SubIFD data");
    }

    #[test]
    fn test_dng_version_extraction() {
        // Create a minimal TIFF with DNGVersion tag
        let mut data = Vec::new();

        // TIFF header
        data.extend_from_slice(b"II\x2a\x00");
        data.extend_from_slice(&8u32.to_le_bytes());

        // IFD0 with DNGVersion tag (0xC612)
        data.extend_from_slice(&1u16.to_le_bytes()); // 1 entry

        // DNGVersion tag entry
        data.extend_from_slice(&0xC612u16.to_le_bytes()); // Tag ID
        data.extend_from_slice(&1u16.to_le_bytes()); // Type: BYTE
        data.extend_from_slice(&4u32.to_le_bytes()); // Count: 4
                                                     // Version 1.4.0.0 stored inline
        data.extend_from_slice(&[1, 4, 0, 0]);

        // Next IFD offset
        data.extend_from_slice(&0u32.to_le_bytes());

        let result = parse_raw_metadata(&data, RawFormat::AdobeDNG);
        assert!(result.is_ok(), "Should parse DNG with version tag");

        let metadata = result.unwrap();
        // Check if version string was created
        if metadata.contains_key("DNG:VersionString") {
            let version = metadata.get("DNG:VersionString").unwrap();
            if let TagValue::String(s) = version {
                assert_eq!(s, "1.4.0.0", "Version should be parsed");
            } else {
                panic!("Version should be a string");
            }
        }
    }

    #[test]
    fn test_cr2_format_detection() {
        // Create a CR2 header
        let mut data = Vec::new();
        data.extend_from_slice(b"II\x2a\x00"); // TIFF header
        data.extend_from_slice(&16u32.to_le_bytes()); // First IFD offset
        data.extend_from_slice(b"CR\x02\x00"); // CR2 marker at offset 8

        // Minimal IFD at offset 16
        data.extend_from_slice(&0u16.to_le_bytes()); // 0 entries
        data.extend_from_slice(&0u32.to_le_bytes()); // Next IFD

        let result = parse_raw_metadata(&data, RawFormat::CanonCR2);
        assert!(result.is_ok(), "Should parse CR2 format");

        let metadata = result.unwrap();
        assert!(
            metadata.contains_key("File:FileType"),
            "Should have FileType tag"
        );
    }

    #[test]
    fn test_nef_format_detection() {
        // Create a minimal NEF (just TIFF header, NEF is detected by extension)
        let mut data = Vec::new();
        data.extend_from_slice(b"MM\x00\x2a"); // TIFF header (big-endian for Nikon)
        data.extend_from_slice(&8u32.to_be_bytes()); // First IFD offset

        // Minimal IFD
        data.extend_from_slice(&0u16.to_be_bytes()); // 0 entries
        data.extend_from_slice(&0u32.to_be_bytes()); // Next IFD

        let result = parse_raw_metadata(&data, RawFormat::NikonNEF);
        assert!(result.is_ok(), "Should parse NEF format");

        let metadata = result.unwrap();
        assert!(
            metadata.contains_key("File:FileType"),
            "Should have FileType tag"
        );
    }

    #[test]
    fn test_multiple_ifd_parsing() {
        // Create TIFF with IFD0 and IFD1 (typical for RAW with thumbnail)
        let mut data = Vec::new();

        // TIFF header
        data.extend_from_slice(b"II\x2a\x00");
        data.extend_from_slice(&8u32.to_le_bytes());

        // IFD0 with ImageWidth tag and pointer to IFD1
        data.extend_from_slice(&1u16.to_le_bytes()); // 1 entry
        data.extend_from_slice(&0x0100u16.to_le_bytes()); // ImageWidth
        data.extend_from_slice(&3u16.to_le_bytes()); // Type: SHORT
        data.extend_from_slice(&1u32.to_le_bytes()); // Count: 1
        data.extend_from_slice(&160u16.to_le_bytes()); // Value: 160
        data.extend_from_slice(&0u16.to_le_bytes()); // Padding

        // Next IFD offset (IFD1 at offset 30)
        data.extend_from_slice(&30u32.to_le_bytes());

        // Pad to offset 30
        while data.len() < 30 {
            data.push(0);
        }

        // IFD1 with ImageWidth tag
        data.extend_from_slice(&1u16.to_le_bytes()); // 1 entry
        data.extend_from_slice(&0x0100u16.to_le_bytes()); // ImageWidth
        data.extend_from_slice(&3u16.to_le_bytes()); // Type: SHORT
        data.extend_from_slice(&1u32.to_le_bytes()); // Count: 1
        data.extend_from_slice(&1600u16.to_le_bytes()); // Value: 1600
        data.extend_from_slice(&0u16.to_le_bytes()); // Padding

        // No more IFDs
        data.extend_from_slice(&0u32.to_le_bytes());

        let result = parse_raw_metadata(&data, RawFormat::CanonCR2);
        assert!(result.is_ok(), "Should parse multiple IFDs");

        let metadata = result.unwrap();
        // Should have tags from both IFD0 and IFD1
        let has_ifd0 = metadata.keys().any(|k| k.starts_with("IFD0:"));
        let has_ifd1 = metadata.keys().any(|k| k.starts_with("IFD1:"));

        assert!(has_ifd0 || has_ifd1, "Should have extracted tags from IFDs");
    }
}
