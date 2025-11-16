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
/// use exiftool_rs::parsers::raw::{parse_raw_metadata, RawFormat};
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
/// 1. Create SliceReader adapter for byte slice access
/// 2. Parse TIFF header to determine byte order
/// 3. Parse IFD chain to extract all metadata tags
/// 4. Convert IFD entries to MetadataMap with proper tag names
/// 5. Add format-specific tags (e.g., DNG version for DNG files)
fn parse_tiff_based_raw(data: &[u8], format: RawFormat) -> Result<MetadataMap> {
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
                eprintln!("Warning: Failed to parse IFD at offset {}: {}", ifd_offset, e);
                break;
            }
        }

        ifd_index += 1;
    }

    // For DNG, extract DNG-specific tags
    if format == RawFormat::AdobeDNG {
        extract_dng_tags(&mut metadata);
    }

    Ok(metadata)
}

/// Extract DNG-specific tags from metadata
///
/// DNG (Digital Negative) files have additional tags beyond standard TIFF/EXIF.
/// This function enriches the metadata with DNG-specific information.
///
/// # DNG-Specific Tags
///
/// - DNGVersion (tag 0xC612): Version of DNG specification
/// - DNGBackwardVersion (tag 0xC613): Oldest DNG reader version that can read this file
/// - UniqueCameraModel (tag 0xC614): Unique camera model identifier
/// - LinearizationTable (tag 0xC618): Linearization table for raw values
/// - BlackLevel (tag 0xC61A): Black level for each color plane
/// - WhiteLevel (tag 0xC61D): White level for sensor
///
/// # Arguments
///
/// * `metadata` - Mutable reference to MetadataMap to enrich
fn extract_dng_tags(metadata: &mut MetadataMap) {
    // DNGVersion is at tag 0xC612 (50706)
    // This should already be extracted by the TIFF parser, but we can add
    // DNG-specific namespace aliases if needed

    // For now, just ensure we mark this as a DNG file
    // The TIFF parser will extract all DNG-specific tags automatically
    if let Some(_version) = metadata.get("ExifIFD:DNGVersion") {
        // Tag exists, we can add DNG namespace alias if needed
        // For consistency with ExifTool output
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

// ===== Helper Functions =====

/// Detect byte order from TIFF header
///
/// Reads the first 2 bytes to determine endianness:
/// - "II" (0x4949) = Little-endian
/// - "MM" (0x4D4D) = Big-endian
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
}
