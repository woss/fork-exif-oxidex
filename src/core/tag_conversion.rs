//! Tag Value Conversion Utilities
//!
//! This module provides utilities for converting raw bytes from TIFF IFD entries
//! into strongly-typed TagValue instances. It handles all EXIF field types including
//! RATIONAL, SRATIONAL, SHORT, LONG, ASCII, and UNDEFINED.
//!
//! The conversion process includes:
//! - Type-specific handlers for each EXIF field type
//! - Special formatting for GPS coordinates, DateTime, and exposure settings
//! - Heuristic conversion for unknown or ambiguous types
//! - Utility functions for reading multi-byte values in different byte orders

use crate::core::TagValue;
use crate::error::Result;
use crate::parsers::common::exif_types::ExifType;
use crate::parsers::tiff::ifd_parser::ByteOrder;
use chrono;

// ============================================================================
// PUBLIC API
// ============================================================================

/// Converts raw bytes from IFD to a TagValue.
///
/// This function interprets raw bytes according to the EXIF field type,
/// converting them to the appropriate TagValue variant.
///
/// # Arguments
///
/// * `bytes` - The raw bytes to convert
/// * `field_type` - The EXIF field type (from IFD entry)
/// * `value_count` - The number of values (from IFD entry)
/// * `tag_id` - The tag ID (for enum mapping and special handling)
/// * `byte_order` - The byte order for interpreting multi-byte values
///
/// # Returns
///
/// A TagValue representing the data
pub fn raw_bytes_to_tag_value(
    bytes: &[u8],
    field_type: u16,
    value_count: u32,
    tag_id: u16,
    byte_order: ByteOrder,
) -> TagValue {
    // Try special tag handlers first (GPS_VERSION_ID, EXIF_VERSION, etc.)
    if let Some(value) = handle_special_byte_tags(tag_id, bytes) {
        return value;
    }

    // Try to convert field_type to ExifType
    if let Some(exif_type) = ExifType::from_u16(field_type) {
        match exif_type {
            // RATIONAL (type 5): two 32-bit unsigned integers (numerator/denominator)
            ExifType::Rational if bytes.len() >= 8 => {
                return handle_rational_type(bytes, value_count, tag_id, byte_order);
            }

            // SRATIONAL (type 10): two 32-bit signed integers (numerator/denominator)
            ExifType::SRational if bytes.len() >= 8 => {
                return handle_srational_type(bytes, value_count, byte_order);
            }

            // SHORT (type 3): unsigned 16-bit integers
            ExifType::Short if bytes.len() >= 2 => {
                return handle_short_type(bytes, value_count, byte_order);
            }

            // LONG (type 4): unsigned 32-bit integers
            ExifType::Long if bytes.len() >= 4 => {
                return handle_long_type(bytes, value_count, byte_order);
            }

            // ASCII (type 2): null-terminated string
            ExifType::Ascii => {
                return handle_ascii_type(bytes);
            }

            // BYTE (type 1) and UNDEFINED (type 7): binary or heuristic conversion
            ExifType::Byte | ExifType::Undefined => {
                // For UNDEFINED type, if no specific handler matched, return binary
                if field_type == 7 {
                    return TagValue::new_binary(bytes.to_vec());
                }
                // Fall through to heuristic conversion for BYTE type
            }

            _ => {
                // Fall through to heuristic conversion below
            }
        }
    }

    // Fallback heuristic conversion for unknown types or when type-specific logic doesn't apply
    heuristic_bytes_to_tag_value(bytes, byte_order)
}

/// Parses a string value to an appropriate TagValue.
///
/// Attempts to parse as integer first, then float, otherwise returns as string.
/// Used for XMP and IPTC metadata parsing.
pub fn parse_string_to_tag_value(value: &str) -> TagValue {
    if let Ok(int_val) = value.parse::<i64>() {
        TagValue::Integer(int_val)
    } else if let Ok(float_val) = value.parse::<f64>() {
        TagValue::Float(float_val)
    } else {
        TagValue::String(value.to_string())
    }
}

// ============================================================================
// SPECIAL TAG HANDLERS
// ============================================================================

/// Handles special byte-encoded tags that need custom formatting.
fn handle_special_byte_tags(tag_id: u16, bytes: &[u8]) -> Option<TagValue> {
    // Tag ID constants
    const GPS_VERSION_ID: u16 = 0x0000;
    const EXIF_VERSION: u16 = 0x9000;
    const COMPONENTS_CONFIGURATION: u16 = 0x9101;

    match tag_id {
        // GPS Version ID (4 bytes: major.minor.rev.0)
        GPS_VERSION_ID if bytes.len() >= 4 => Some(TagValue::new_string(format!(
            "{}.{}.{}.{}",
            bytes[0], bytes[1], bytes[2], bytes[3]
        ))),

        // Exif Version (4 bytes: ASCII "0232")
        EXIF_VERSION if bytes.len() >= 4 => {
            let version = String::from_utf8_lossy(&bytes[0..4]);
            Some(TagValue::new_string(version.to_string()))
        }

        // ComponentsConfiguration (4 bytes with component IDs)
        COMPONENTS_CONFIGURATION if bytes.len() >= 4 => {
            let component_names = bytes
                .iter()
                .take(4)
                .map(|&b| match b {
                    0 => "-",
                    1 => "Y",
                    2 => "Cb",
                    3 => "Cr",
                    4 => "R",
                    5 => "G",
                    6 => "B",
                    _ => "?",
                })
                .collect::<Vec<_>>();
            Some(TagValue::new_string(component_names.join(", ")))
        }

        _ => None,
    }
}

// ============================================================================
// TYPE-SPECIFIC HANDLERS
// ============================================================================

/// Handles RATIONAL type fields (type 5).
fn handle_rational_type(
    bytes: &[u8],
    value_count: u32,
    tag_id: u16,
    byte_order: ByteOrder,
) -> TagValue {
    // GPS coordinate tags (3 rationals: degrees, minutes, seconds)
    const GPS_LATITUDE: u16 = 0x0002;
    const GPS_LONGITUDE: u16 = 0x0004;
    const GPS_DEST_LATITUDE: u16 = 0x0014;
    const GPS_DEST_LONGITUDE: u16 = 0x0016;
    const GPS_ALTITUDE: u16 = 0x0006;
    const EXPOSURE_TIME: u16 = 0x829A;

    // Check if this is an array of rationals (count > 1)
    if value_count > 1 && bytes.len() >= (value_count as usize * 8) {
        // Special handling for GPS coordinates (3 rationals: degrees, minutes, seconds)
        if matches!(
            tag_id,
            GPS_LATITUDE | GPS_LONGITUDE | GPS_DEST_LATITUDE | GPS_DEST_LONGITUDE
        ) && value_count == 3
        {
            return format_gps_coordinate(bytes, byte_order);
        }

        // Parse array of rationals and format as space-separated decimals
        return parse_rational_array(bytes, value_count, byte_order);
    }

    // Single rational value - parse numerator and denominator
    let numerator = read_u32(&bytes[0..4], byte_order);
    let denominator = read_u32(&bytes[4..8], byte_order);

    // Special handling for GPS Altitude
    if tag_id == GPS_ALTITUDE && denominator != 0 {
        let value = numerator as f64 / denominator as f64;
        if value.fract() == 0.0 {
            return TagValue::new_string(format!("{} m", value as i32));
        } else {
            return TagValue::new_string(format!("{:.1} m", value));
        }
    }

    // Special handling for ExposureTime - format as fraction string
    if tag_id == EXPOSURE_TIME && denominator != 0 {
        let gcd_value = gcd(numerator, denominator);
        let simplified_num = numerator / gcd_value;
        let simplified_den = denominator / gcd_value;
        if simplified_den > 1 {
            return TagValue::new_string(format!("{}/{}", simplified_num, simplified_den));
        }
    }

    TagValue::new_rational(numerator as i32, denominator as i32)
}

/// Formats a GPS coordinate from 3 rational values.
fn format_gps_coordinate(bytes: &[u8], byte_order: ByteOrder) -> TagValue {
    let mut dms = Vec::new();
    for i in 0..3 {
        let offset = i * 8;
        let numerator = read_u32(&bytes[offset..offset + 4], byte_order);
        let denominator = read_u32(&bytes[offset + 4..offset + 8], byte_order);
        if denominator != 0 {
            dms.push(numerator as f64 / denominator as f64);
        } else {
            dms.push(numerator as f64);
        }
    }
    let formatted = format!("{} deg {}' {:.2}\"", dms[0] as i32, dms[1] as i32, dms[2]);
    TagValue::new_string(formatted)
}

/// Parses an array of rational values into a space-separated string.
fn parse_rational_array(bytes: &[u8], value_count: u32, byte_order: ByteOrder) -> TagValue {
    let mut values = Vec::new();
    for i in 0..value_count as usize {
        let offset = i * 8;
        let numerator = read_u32(&bytes[offset..offset + 4], byte_order);
        let denominator = read_u32(&bytes[offset + 4..offset + 8], byte_order);
        if denominator != 0 {
            values.push(numerator as f64 / denominator as f64);
        } else {
            values.push(numerator as f64);
        }
    }
    let formatted = values
        .iter()
        .map(|v| format!("{:.10}", v))
        .collect::<Vec<_>>()
        .join(" ");
    TagValue::new_string(formatted)
}

/// Handles SRATIONAL type fields (type 10).
fn handle_srational_type(bytes: &[u8], value_count: u32, byte_order: ByteOrder) -> TagValue {
    if value_count > 1 && bytes.len() >= (value_count as usize * 8) {
        return parse_srational_array(bytes, value_count, byte_order);
    }

    let numerator = read_i32(&bytes[0..4], byte_order);
    let denominator = read_i32(&bytes[4..8], byte_order);
    TagValue::new_rational(numerator, denominator)
}

/// Parses an array of signed rational values.
fn parse_srational_array(bytes: &[u8], value_count: u32, byte_order: ByteOrder) -> TagValue {
    let mut values = Vec::new();
    for i in 0..value_count as usize {
        let offset = i * 8;
        let numerator = read_i32(&bytes[offset..offset + 4], byte_order);
        let denominator = read_i32(&bytes[offset + 4..offset + 8], byte_order);
        if denominator != 0 {
            values.push(numerator as f64 / denominator as f64);
        } else {
            values.push(numerator as f64);
        }
    }
    let formatted = values
        .iter()
        .map(|v| format!("{:.10}", v))
        .collect::<Vec<_>>()
        .join(" ");
    TagValue::new_string(formatted)
}

/// Handles SHORT type fields (type 3).
fn handle_short_type(bytes: &[u8], value_count: u32, byte_order: ByteOrder) -> TagValue {
    if value_count > 1 && bytes.len() >= (value_count as usize * 2) {
        let mut values = Vec::new();
        for i in 0..value_count as usize {
            let offset = i * 2;
            let value = read_u16(&bytes[offset..offset + 2], byte_order);
            values.push(value.to_string());
        }
        return TagValue::new_string(values.join(" "));
    }

    let value = read_u16(&bytes[0..2], byte_order) as i64;
    TagValue::new_integer(value)
}

/// Handles LONG type fields (type 4).
fn handle_long_type(bytes: &[u8], value_count: u32, byte_order: ByteOrder) -> TagValue {
    if value_count > 1 && bytes.len() >= (value_count as usize * 4) {
        let mut values = Vec::new();
        for i in 0..value_count as usize {
            let offset = i * 4;
            let value = read_u32(&bytes[offset..offset + 4], byte_order);
            values.push(value.to_string());
        }
        return TagValue::new_string(values.join(" "));
    }

    let value = read_u32(&bytes[0..4], byte_order) as i64;
    TagValue::new_integer(value)
}

/// Handles ASCII type fields (type 2).
fn handle_ascii_type(bytes: &[u8]) -> TagValue {
    let s = String::from_utf8_lossy(bytes);
    let s = s.trim_end_matches('\0');
    if !s.is_empty() {
        if is_datetime_string(s) {
            if let Ok(dt) = parse_exif_datetime(s) {
                return TagValue::DateTime(dt);
            }
        }
        return TagValue::new_string(s.to_string());
    }
    TagValue::new_string(String::new())
}

/// Applies heuristic conversion for unknown or ambiguous byte sequences.
fn heuristic_bytes_to_tag_value(bytes: &[u8], byte_order: ByteOrder) -> TagValue {
    if bytes.len() == 2 {
        let value = read_u16(bytes, byte_order) as i64;
        return TagValue::new_integer(value);
    } else if bytes.len() == 4 {
        let null_count = bytes.iter().filter(|&&b| b == 0).count();
        let has_printable = bytes.iter().any(|&b| (32..=126).contains(&b));

        if null_count > 1 || !has_printable {
            let value = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as i64;
            return TagValue::new_integer(value);
        }

        if bytes.iter().all(|&b| (32..=126).contains(&b) || b == 0) {
            let s = String::from_utf8_lossy(bytes);
            let s = s.trim_end_matches('\0');
            if !s.is_empty() && s.len() >= 3 {
                return TagValue::new_string(s.to_string());
            }
        }

        let value = read_u32(bytes, byte_order) as i64;
        return TagValue::new_integer(value);
    }

    if bytes
        .iter()
        .all(|&b| (32..=126).contains(&b) || b == 0 || b == b'\n' || b == b'\r' || b == b'\t')
    {
        let s = String::from_utf8_lossy(bytes);
        let s = s.trim_end_matches('\0');
        if !s.is_empty() {
            if is_datetime_string(s) {
                if let Ok(dt) = parse_exif_datetime(s) {
                    return TagValue::DateTime(dt);
                }
            }
            return TagValue::new_string(s.to_string());
        }
    }

    TagValue::new_binary(bytes.to_vec())
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Checks if a string matches the EXIF DateTime format.
pub fn is_datetime_string(s: &str) -> bool {
    s.len() == 19
        && s.chars().filter(|&c| c == ':').count() == 4
        && s.chars().filter(|&c| c == ' ').count() == 1
        && s.chars().nth(4) == Some(':')
        && s.chars().nth(7) == Some(':')
        && s.chars().nth(10) == Some(' ')
        && s.chars().nth(13) == Some(':')
        && s.chars().nth(16) == Some(':')
}

/// Parses an EXIF DateTime string into a chrono::DateTime<Utc>.
pub fn parse_exif_datetime(s: &str) -> Result<chrono::DateTime<chrono::Utc>> {
    use crate::error::ExifToolError;
    use chrono::NaiveDateTime;

    let naive = NaiveDateTime::parse_from_str(s, "%Y:%m:%d %H:%M:%S")
        .map_err(|e| ExifToolError::parse_error(format!("Invalid DateTime: {}", e)))?;

    Ok(chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
        naive,
        chrono::Utc,
    ))
}

/// Reads an unsigned 16-bit integer from bytes.
pub fn read_u16(bytes: &[u8], byte_order: ByteOrder) -> u16 {
    match byte_order {
        ByteOrder::LittleEndian => u16::from_le_bytes([bytes[0], bytes[1]]),
        ByteOrder::BigEndian => u16::from_be_bytes([bytes[0], bytes[1]]),
    }
}

/// Reads an unsigned 32-bit integer from bytes.
pub fn read_u32(bytes: &[u8], byte_order: ByteOrder) -> u32 {
    match byte_order {
        ByteOrder::LittleEndian => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        ByteOrder::BigEndian => u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
    }
}

/// Reads a signed 32-bit integer from bytes.
pub fn read_i32(bytes: &[u8], byte_order: ByteOrder) -> i32 {
    match byte_order {
        ByteOrder::LittleEndian => i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        ByteOrder::BigEndian => i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
    }
}

/// Computes the Greatest Common Divisor using Euclid's algorithm.
fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}
