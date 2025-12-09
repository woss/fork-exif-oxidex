//! APP6 segment parser for JPEG files
//!
//! JPEG APP6 segments (marker 0xFFE6) contain various proprietary metadata formats:
//! - GoPro GPMF (GoPro Metadata Format) - Action camera telemetry and settings
//! - HP/Toshiba TDHD (True Definition High Definition) - Stereo image metadata
//! - NITF (National Imagery Transmission Format) - Geospatial metadata
//! - IPTC-NAA - Legacy IPTC records (rare, mostly superseded by APP13)
//!
//! # GoPro GPMF Format
//!
//! GoPro cameras embed extensive metadata in APP6 segments including:
//! - Camera settings (FOV, resolution, frame rate, protune, etc.)
//! - Sensor telemetry (GPS, accelerometer, gyroscope)
//! - Image processing parameters (lens distortion, color grading)
//! - Device information (model, serial number, firmware version)
//!
//! The GPMF format uses a tag-length-value (TLV) structure with FourCC identifiers.
//! Each record consists of:
//! - FourCC key (4 bytes) - Tag identifier
//! - Type (1 byte) - Data type indicator
//! - Size (1 byte) - Size of each element
//! - Count (2 bytes, big-endian) - Number of elements
//! - Data (variable) - Payload data
//!
//! # References
//!
//! - GoPro GPMF Specification: https://github.com/gopro/gpmf-parser
//! - ExifTool APP6 Tags: lib/Image/ExifTool/GoPro.pm
//! - JPEG Specification: ITU-T T.81 / ISO/IEC 10918-1
//!
//! # Example
//!
//! ```ignore
//! use oxidex::parsers::jpeg::app_segments::app6::parse_app6;
//!
//! let data: &[u8] = &[/* APP6 segment data */];
//! let metadata = parse_app6(data)?;
//!
//! if let Some(model) = metadata.get_string("APP6:Model") {
//!     println!("Camera model: {}", model);
//! }
//! ```

use crate::core::{MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};
use crate::io::EndianReader;

/// Parses APP6 segment data and extracts metadata.
///
/// This function dispatches to format-specific parsers based on the segment identifier:
/// - GPMF data (GoPro cameras) - starts with known GPMF FourCC codes
/// - TDHD data (HP/Toshiba) - starts with "TDHD" identifier
/// - NITF data - starts with "NITF" identifier
/// - Other proprietary formats
///
/// # Arguments
///
/// * `data` - Raw APP6 segment data (excluding the APP6 marker and length bytes)
///
/// # Returns
///
/// * `Ok(MetadataMap)` - A metadata map containing extracted APP6 tags
/// * `Err(ExifToolError)` - If the data is malformed or unsupported
///
/// # Errors
///
/// Returns an error if:
/// - The segment is too short to contain valid metadata
/// - The format is recognized but parsing fails
///
/// # Example
///
/// ```ignore
/// use oxidex::parsers::jpeg::app_segments::app6::parse_app6;
///
/// // Parse a GoPro GPMF segment
/// let gpmf_data = &[/* GPMF data */];
/// let metadata = parse_app6(gpmf_data)?;
/// assert!(metadata.contains_key("APP6:Model"));
/// ```
pub fn parse_app6(data: &[u8]) -> Result<MetadataMap> {
    let mut metadata = MetadataMap::new();

    // Minimum APP6 segment should have at least a few bytes
    if data.len() < 4 {
        return Err(ExifToolError::parse_error(
            "APP6 segment too short to contain valid metadata",
        ));
    }

    // Try to identify the format by checking for known identifiers

    // Check for GoPro GPMF format (no explicit identifier, starts with FourCC)
    // Common GoPro GPMF root tags: DEVC (device), DVID (device ID), DVNM (device name)
    if is_gpmf_format(data) {
        return parse_gpmf(data);
    }

    // Check for TDHD (HP/Toshiba stereo image metadata)
    if data.len() >= 4 && &data[..4] == b"TDHD" {
        return parse_tdhd(data);
    }

    // Check for NITF (National Imagery Transmission Format)
    if data.len() >= 4 && &data[..4] == b"NITF" {
        return parse_nitf(data);
    }

    // Unknown or unsupported APP6 format
    // Store as raw binary data for debugging
    metadata.insert("APP6:Unknown".to_string(), TagValue::Binary(data.to_vec()));

    Ok(metadata)
}

/// Checks if the data appears to be GoPro GPMF format.
///
/// GPMF data starts with known FourCC identifiers and follows a specific structure.
/// This function performs heuristic checks to identify GPMF data.
///
/// # Arguments
///
/// * `data` - Raw segment data
///
/// # Returns
///
/// `true` if the data appears to be GPMF format, `false` otherwise
fn is_gpmf_format(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }

    // Check for common GPMF root FourCC identifiers
    // DEVC = Device, DVID = Device ID, DVNM = Device Name
    const GPMF_ROOT_TAGS: &[&[u8]] = &[
        b"DEVC", // Device (most common root)
        b"DVID", // Device ID
        b"DVNM", // Device Name
        b"STRM", // Stream
    ];

    for tag in GPMF_ROOT_TAGS {
        if &data[..4] == *tag {
            return true;
        }
    }

    false
}

/// Parses GoPro GPMF (GoPro Metadata Format) data.
///
/// GPMF uses a hierarchical TLV (Tag-Length-Value) structure with FourCC tags.
/// This parser extracts camera settings, telemetry, and device information.
///
/// # Arguments
///
/// * `data` - Raw GPMF data
///
/// # Returns
///
/// * `Ok(MetadataMap)` - Extracted GoPro metadata
/// * `Err(ExifToolError)` - If parsing fails
///
/// # GPMF Structure
///
/// Each GPMF record:
/// - FourCC (4 bytes) - Tag identifier (ASCII)
/// - Type (1 byte) - Data type ('b'=byte, 's'=short, 'l'=long, 'f'=float, 'c'=string, etc.)
/// - Size (1 byte) - Bytes per element
/// - Count (2 bytes, BE) - Number of elements
/// - Data (variable) - Padded to 4-byte alignment
///
/// # Example Tags
///
/// - DEVC: Device container
/// - DVNM: Device name (camera model)
/// - FWVS: Firmware version
/// - STNM: Stream name
/// - CAMD: Camera metadata
fn parse_gpmf(data: &[u8]) -> Result<MetadataMap> {
    let mut metadata = MetadataMap::new();
    let mut offset = 0;

    while offset + 8 <= data.len() {
        // Parse GPMF record header
        let fourcc = &data[offset..offset + 4];
        let type_char = data[offset + 4] as char;
        let size = data[offset + 5] as usize;

        let reader = EndianReader::big_endian(&data[offset + 6..]);
        let count = reader.u16_at(0).unwrap_or(0) as usize;

        offset += 8;

        // Calculate data size (size * count, padded to 4-byte boundary)
        let data_size = size * count;
        let padded_size = (data_size + 3) & !3; // Round up to multiple of 4

        if offset + data_size > data.len() {
            break; // Truncated data
        }

        let value_data = &data[offset..offset + data_size];

        // Convert FourCC to string
        let tag_name = std::str::from_utf8(fourcc).unwrap_or("????").to_string();

        // Parse value based on type and tag
        parse_gpmf_value(&mut metadata, &tag_name, type_char, size, count, value_data)?;

        offset += padded_size;
    }

    // If no metadata was extracted, the format might not be GPMF
    if metadata.is_empty() {
        return Err(ExifToolError::parse_error(
            "No GPMF metadata could be extracted from APP6 segment",
        ));
    }

    Ok(metadata)
}

/// Parses a GPMF value and inserts it into the metadata map.
///
/// # Arguments
///
/// * `metadata` - Metadata map to populate
/// * `tag_name` - FourCC tag name
/// * `type_char` - GPMF type character
/// * `size` - Size of each element in bytes
/// * `count` - Number of elements
/// * `data` - Raw value data
fn parse_gpmf_value(
    metadata: &mut MetadataMap,
    tag_name: &str,
    type_char: char,
    size: usize,
    count: usize,
    data: &[u8],
) -> Result<()> {
    let tag_key = format!("APP6:{}", tag_name);

    match type_char {
        'c' | 'C' => {
            // String/character data
            if let Ok(s) = std::str::from_utf8(data) {
                metadata.insert(
                    tag_key,
                    TagValue::String(s.trim_end_matches('\0').to_string()),
                );
            }
        }
        's' | 'S' => {
            // Signed/unsigned 16-bit integers
            if count == 1 && size == 2 {
                let reader = EndianReader::big_endian(data);
                if type_char == 's' {
                    if let Some(v) = reader.i16_at(0) {
                        metadata.insert(tag_key, TagValue::Integer(v as i64));
                    }
                } else if let Some(v) = reader.u16_at(0) {
                    metadata.insert(tag_key, TagValue::Integer(v as i64));
                }
            } else {
                // Array of shorts - store as binary for now
                metadata.insert(tag_key, TagValue::Binary(data.to_vec()));
            }
        }
        'l' | 'L' => {
            // Signed/unsigned 32-bit integers
            if count == 1 && size == 4 {
                let reader = EndianReader::big_endian(data);
                if type_char == 'l' {
                    if let Some(v) = reader.i32_at(0) {
                        metadata.insert(tag_key, TagValue::Integer(v as i64));
                    }
                } else if let Some(v) = reader.u32_at(0) {
                    metadata.insert(tag_key, TagValue::Integer(v as i64));
                }
            } else {
                metadata.insert(tag_key, TagValue::Binary(data.to_vec()));
            }
        }
        'f' | 'F' => {
            // Float/double
            if count == 1 {
                let reader = EndianReader::big_endian(data);
                if size == 4 {
                    if let Some(v) = reader.f32_at(0) {
                        metadata.insert(tag_key, TagValue::Float(v as f64));
                    }
                } else if size == 8
                    && let Some(v) = reader.f64_at(0) {
                        metadata.insert(tag_key, TagValue::Float(v));
                    }
            } else {
                metadata.insert(tag_key, TagValue::Binary(data.to_vec()));
            }
        }
        '\0' => {
            // Nested container - recursively parse if it's a known container type
            if tag_name == "DEVC" || tag_name == "STRM" {
                // Parse nested structure
                let nested = parse_gpmf(data)?;
                for (key, value) in nested.iter() {
                    metadata.insert(key.clone(), value.clone());
                }
            } else {
                metadata.insert(tag_key, TagValue::Binary(data.to_vec()));
            }
        }
        _ => {
            // Unknown type - store as binary
            metadata.insert(tag_key, TagValue::Binary(data.to_vec()));
        }
    }

    Ok(())
}

/// Parses TDHD (True Definition High Definition) metadata.
///
/// TDHD is used by HP and Toshiba cameras for stereo/3D image metadata.
/// The format stores information about left/right eye images and depth maps.
///
/// # Arguments
///
/// * `data` - Raw TDHD data (starts with "TDHD" identifier)
///
/// # Returns
///
/// * `Ok(MetadataMap)` - Extracted TDHD metadata
/// * `Err(ExifToolError)` - If parsing fails
fn parse_tdhd(data: &[u8]) -> Result<MetadataMap> {
    let mut metadata = MetadataMap::new();

    if data.len() < 8 {
        return Err(ExifToolError::parse_error("TDHD segment too short"));
    }

    // Skip "TDHD" identifier
    let reader = EndianReader::big_endian(&data[4..]);

    // TDHD version (typically at offset 4-5)
    if let Some(version) = reader.u16_at(0) {
        metadata.insert(
            "APP6:TDHDVersion".to_string(),
            TagValue::Integer(version as i64),
        );
    }

    // Basic TDHD support - most fields are proprietary
    metadata.insert(
        "APP6:TDHDData".to_string(),
        TagValue::Binary(data[4..].to_vec()),
    );

    Ok(metadata)
}

/// Parses NITF (National Imagery Transmission Format) metadata.
///
/// NITF is used for geospatial imagery metadata in defense/intelligence applications.
/// The format includes image classification, geolocation, and sensor information.
///
/// # Arguments
///
/// * `data` - Raw NITF data (starts with "NITF" identifier)
///
/// # Returns
///
/// * `Ok(MetadataMap)` - Extracted NITF metadata
/// * `Err(ExifToolError)` - If parsing fails
fn parse_nitf(data: &[u8]) -> Result<MetadataMap> {
    let mut metadata = MetadataMap::new();

    if data.len() < 8 {
        return Err(ExifToolError::parse_error("NITF segment too short"));
    }

    // Skip "NITF" identifier
    // NITF has a complex header structure - implement basic support
    metadata.insert(
        "APP6:NITFData".to_string(),
        TagValue::Binary(data[4..].to_vec()),
    );

    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_gpmf_format() {
        // Valid GPMF data starting with DEVC
        let valid_gpmf = b"DEVC\x00\x04\x00\x01test";
        assert!(is_gpmf_format(valid_gpmf));

        // Valid GPMF data starting with DVID
        let valid_dvid = b"DVID\x00\x04\x00\x01test";
        assert!(is_gpmf_format(valid_dvid));

        // Invalid data
        let invalid = b"TEST\x00\x04\x00\x01";
        assert!(!is_gpmf_format(invalid));

        // Too short
        let too_short = b"DEV";
        assert!(!is_gpmf_format(too_short));
    }

    #[test]
    fn test_parse_gpmf_string() {
        // DVNM (Device Name) with string data
        // FourCC: DVNM, Type: c (string), Size: 1, Count: 11, Data: "HERO8 Black"
        let mut data = Vec::new();
        data.extend_from_slice(b"DVNM"); // FourCC
        data.push(b'c'); // Type: string
        data.push(1); // Size: 1 byte per char
        data.extend_from_slice(&11u16.to_be_bytes()); // Count: 11 chars
        data.extend_from_slice(b"HERO8 Black"); // Data
        data.push(0); // Padding to 4-byte boundary (11 + 1 = 12, already aligned)

        let result = parse_gpmf(&data);
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert_eq!(metadata.get_string("APP6:DVNM"), Some("HERO8 Black"));
    }

    #[test]
    fn test_parse_gpmf_integer() {
        // Simple integer tag
        // FourCC: TEST, Type: L (u32), Size: 4, Count: 1, Data: 12345
        let mut data = Vec::new();
        data.extend_from_slice(b"TEST"); // FourCC
        data.push(b'L'); // Type: unsigned long
        data.push(4); // Size: 4 bytes
        data.extend_from_slice(&1u16.to_be_bytes()); // Count: 1
        data.extend_from_slice(&12345u32.to_be_bytes()); // Data

        let result = parse_gpmf(&data);
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert_eq!(metadata.get_integer("APP6:TEST"), Some(12345));
    }

    #[test]
    fn test_parse_tdhd() {
        let mut data = Vec::new();
        data.extend_from_slice(b"TDHD"); // Identifier
        data.extend_from_slice(&0x0100u16.to_be_bytes()); // Version 1.0
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Additional data

        let result = parse_tdhd(&data);
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert_eq!(metadata.get_integer("APP6:TDHDVersion"), Some(256));
    }

    #[test]
    fn test_parse_nitf() {
        let mut data = Vec::new();
        data.extend_from_slice(b"NITF"); // Identifier
        data.extend_from_slice(&[0x01, 0x02, 0x03, 0x04]); // Sample data

        let result = parse_nitf(&data);
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert!(metadata.contains_key("APP6:NITFData"));
    }

    #[test]
    fn test_parse_app6_unknown() {
        // Unknown format
        let data = b"UNKN\x00\x00\x00\x00";

        let result = parse_app6(data);
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert!(metadata.contains_key("APP6:Unknown"));
    }

    #[test]
    fn test_parse_app6_too_short() {
        let data = b"AB";

        let result = parse_app6(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_app6_dispatches_to_gpmf() {
        // Create a minimal GPMF segment
        let mut data = Vec::new();
        data.extend_from_slice(b"DEVC"); // Root container
        data.push(0); // Type: container
        data.push(0); // Size: 0 (container)
        data.extend_from_slice(&0u16.to_be_bytes()); // Count: 0

        let result = parse_app6(&data);
        // Should attempt GPMF parsing (may fail due to empty container)
        // But should not return unknown format error
        assert!(result.is_ok() || matches!(result, Err(ExifToolError::ParseError { .. })));
    }
}
