//! Scalado Mobile Photo Editor MakerNote parser
//!
//! Parses Scalado photo editing metadata from mobile applications.
//! Scalado was a mobile imaging technology company acquired by Nokia,
//! with technology integrated into many smartphone camera apps.
//!
//! ## Supported Applications
//! - Scalado Album (legacy)
//! - Scalado PhotoBeamer
//! - Various OEM camera apps (Nokia, Sony Ericsson)
//!
//! ## Key Features
//! - Photo filters applied
//! - Auto-enhance settings
//! - Red-eye reduction
//! - Crop and straighten information
//! - Brightness/contrast adjustments
//! - Effects (vintage, sepia, etc.)
//! - Face detection results
//! - Panorama stitching metadata
//! - HDR processing info
//! - Touch-up areas
//!
//! ## Architecture
//! Scalado stores lightweight editing metadata optimized
//! for mobile devices and quick sharing workflows.

#![allow(dead_code)]
#![allow(unused_imports)]

use crate::const_decoder;
use crate::parsers::tiff::ifd_parser::{ByteOrder, IfdEntry};
use std::collections::HashMap;

use super::shared::array_extractors::extract_i16_array;
use super::shared::MakerNoteParser;

// Scalado MakerNote Tag IDs
const SCALADO_VERSION: u16 = 0x0001; // Scalado version
const SCALADO_FILTER_TYPE: u16 = 0x0010; // Filter type applied
const SCALADO_AUTO_ENHANCE: u16 = 0x0011; // Auto-enhance level
const SCALADO_RED_EYE: u16 = 0x0012; // Red-eye reduction applied
const SCALADO_BRIGHTNESS: u16 = 0x0020; // Brightness adjustment
const SCALADO_CONTRAST: u16 = 0x0021; // Contrast adjustment
const SCALADO_SATURATION: u16 = 0x0022; // Saturation adjustment
const SCALADO_CROP_APPLIED: u16 = 0x0030; // Crop applied flag
const SCALADO_STRAIGHTEN_ANGLE: u16 = 0x0031; // Straighten angle
const SCALADO_FACE_COUNT: u16 = 0x0040; // Faces detected
const SCALADO_PANORAMA: u16 = 0x0041; // Panorama stitched
const SCALADO_HDR: u16 = 0x0042; // HDR processing
const SCALADO_TOUCHUP_COUNT: u16 = 0x0043; // Touch-up areas

// Scalado signature
const SCALADO_SIGNATURE: &[u8] = b"Scalado";

// ============================================================================
// Declarative Decoder Definitions
// ============================================================================

// Decodes Scalado filter type
const_decoder! {
    DECODE_FILTER_TYPE, i16, [
        (0, "None"),
        (1, "Vintage"),
        (2, "Sepia"),
        (3, "Black & White"),
        (4, "Cool"),
        (5, "Warm"),
        (6, "Vivid"),
        (7, "Soft"),
    ]
}

// Decodes Scalado auto-enhance level
const_decoder! {
    DECODE_AUTO_ENHANCE, i16, [
        (0, "Off"),
        (1, "Low"),
        (2, "Medium"),
        (3, "High"),
    ]
}

// Formats adjustment percentage (-100 to +100) with proper +/- sign
// Used for brightness, contrast, and saturation adjustments
fn format_adjustment(value: i16) -> String {
    if value >= 0 {
        format!("+{}", value)
    } else {
        format!("{}", value)
    }
}

// Extracts an ASCII string from IFD entry
// Handles both inline strings (<=4 bytes stored in value_offset)
// and external strings (>4 bytes stored at offset in data buffer)
fn extract_string(entry: &IfdEntry, data: &[u8]) -> Option<String> {
    // Field type 2 indicates ASCII string
    if entry.field_type != 2 {
        return None;
    }

    let offset = entry.value_offset as usize;
    let count = entry.value_count as usize;

    // Handle inline strings (4 bytes or less)
    if count <= 4 {
        let bytes = entry.value_offset.to_le_bytes();
        let s = String::from_utf8_lossy(&bytes[..count.min(4)])
            .trim_end_matches('\0')
            .to_string();
        return if s.is_empty() { None } else { Some(s) };
    }

    // Handle external strings
    if offset + count > data.len() {
        return None;
    }

    let s = String::from_utf8_lossy(&data[offset..offset + count])
        .trim_end_matches('\0')
        .to_string();

    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Parser for Scalado MakerNotes
#[derive(Default)]
pub struct ScaladoParser;

impl ScaladoParser {
    /// Creates a new Scalado parser instance
    pub fn new() -> Self {
        ScaladoParser
    }
}

impl MakerNoteParser for ScaladoParser {
    fn manufacturer_name(&self) -> &'static str {
        "Scalado"
    }

    fn tag_prefix(&self) -> &'static str {
        "Scalado:"
    }

    fn validate_header(&self, data: &[u8]) -> bool {
        if data.len() < 7 {
            return false;
        }
        data.starts_with(SCALADO_SIGNATURE) || data.len() >= 8
    }

    /// Parses Scalado MakerNote data and extracts editing metadata
    ///
    /// Scalado MakerNotes may optionally start with a signature, followed by
    /// standard IFD format. This method handles both cases and extracts all
    /// photo editing metadata tags.
    ///
    /// # Arguments
    /// * `data` - The raw MakerNote data buffer
    /// * `byte_order` - The byte order to use for multi-byte values
    /// * `tags` - HashMap to populate with extracted tag name/value pairs
    ///
    /// # Returns
    /// * `Ok(())` if parsing succeeded
    /// * `Err(String)` if data is too short
    fn parse(
        &self,
        data: &[u8],
        byte_order: ByteOrder,
        tags: &mut HashMap<String, String>,
    ) -> Result<(), String> {
        // Validate minimum data length
        if data.len() < 8 {
            return Err("Scalado MakerNote data too short".to_string());
        }

        // Skip optional Scalado signature (7 bytes)
        let start_offset = if data.starts_with(SCALADO_SIGNATURE) {
            7
        } else {
            0
        };
        let parse_data = &data[start_offset..];

        // Need at least 2 bytes for entry count
        if parse_data.len() < 2 {
            return Ok(());
        }

        // Read the number of IFD entries
        let num_entries = match byte_order {
            ByteOrder::LittleEndian => u16::from_le_bytes([parse_data[0], parse_data[1]]),
            ByteOrder::BigEndian => u16::from_be_bytes([parse_data[0], parse_data[1]]),
        } as usize;

        // Sanity check: entry count should be reasonable
        if num_entries == 0 || num_entries > 100 {
            return Ok(());
        }

        let mut offset = 2;
        let entry_size = 12;

        // Iterate through all IFD entries
        for _ in 0..num_entries {
            // Ensure we have enough data for a complete entry
            if offset + entry_size > parse_data.len() {
                break;
            }

            let entry_data = &parse_data[offset..offset + entry_size];

            // Parse the tag ID (2 bytes)
            let tag = match byte_order {
                ByteOrder::LittleEndian => u16::from_le_bytes([entry_data[0], entry_data[1]]),
                ByteOrder::BigEndian => u16::from_be_bytes([entry_data[0], entry_data[1]]),
            };

            // Parse the field type (2 bytes)
            let field_type = match byte_order {
                ByteOrder::LittleEndian => u16::from_le_bytes([entry_data[2], entry_data[3]]),
                ByteOrder::BigEndian => u16::from_be_bytes([entry_data[2], entry_data[3]]),
            };

            // Parse the value count (4 bytes)
            let count = match byte_order {
                ByteOrder::LittleEndian => {
                    u32::from_le_bytes([entry_data[4], entry_data[5], entry_data[6], entry_data[7]])
                }
                ByteOrder::BigEndian => {
                    u32::from_be_bytes([entry_data[4], entry_data[5], entry_data[6], entry_data[7]])
                }
            };

            // Parse the value/offset field (4 bytes)
            let value_offset = match byte_order {
                ByteOrder::LittleEndian => u32::from_le_bytes([
                    entry_data[8],
                    entry_data[9],
                    entry_data[10],
                    entry_data[11],
                ]),
                ByteOrder::BigEndian => u32::from_be_bytes([
                    entry_data[8],
                    entry_data[9],
                    entry_data[10],
                    entry_data[11],
                ]),
            };

            // Create IFD entry structure
            let entry = IfdEntry {
                tag_id: tag,
                field_type,
                value_count: count,
                value_offset,
            };

            // Parse the tag based on its ID
            match tag {
                SCALADO_VERSION => {
                    // Version is stored as a string
                    if let Some(s) = extract_string(&entry, parse_data) {
                        tags.insert("Scalado:Version".to_string(), s);
                    }
                }

                _ => {
                    // All other tags are stored as i16 arrays
                    if let Some(array) = extract_i16_array(&entry, parse_data, byte_order) {
                        if let Some(&val) = array.first() {
                            // Decode the value based on the tag ID using const decoders
                            let (tag_name, formatted_value) = match tag {
                                SCALADO_FILTER_TYPE => {
                                    ("FilterType", DECODE_FILTER_TYPE.decode(val))
                                }
                                SCALADO_AUTO_ENHANCE => {
                                    ("AutoEnhance", DECODE_AUTO_ENHANCE.decode(val))
                                }
                                SCALADO_RED_EYE => (
                                    "RedEyeReduction",
                                    if val != 0 { "Yes" } else { "No" }.to_string(),
                                ),
                                SCALADO_BRIGHTNESS => ("Brightness", format_adjustment(val)),
                                SCALADO_CONTRAST => ("Contrast", format_adjustment(val)),
                                SCALADO_SATURATION => ("Saturation", format_adjustment(val)),
                                SCALADO_CROP_APPLIED => (
                                    "CropApplied",
                                    if val != 0 { "Yes" } else { "No" }.to_string(),
                                ),
                                SCALADO_STRAIGHTEN_ANGLE => {
                                    ("StraightenAngle", format!("{}°", val))
                                }
                                SCALADO_FACE_COUNT => ("FacesDetected", val.to_string()),
                                SCALADO_PANORAMA => {
                                    ("Panorama", if val != 0 { "Yes" } else { "No" }.to_string())
                                }
                                SCALADO_HDR => {
                                    ("HDR", if val != 0 { "Yes" } else { "No" }.to_string())
                                }
                                SCALADO_TOUCHUP_COUNT => ("TouchupCount", val.to_string()),
                                _ => continue,
                            };
                            tags.insert(format!("Scalado:{}", tag_name), formatted_value);
                        }
                    }
                }
            }

            offset += entry_size;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalado_parser_creation() {
        let parser = ScaladoParser::new();
        assert_eq!(parser.manufacturer_name(), "Scalado");
        assert_eq!(parser.tag_prefix(), "Scalado:");
    }

    #[test]
    fn test_decode_filter_type() {
        assert_eq!(DECODE_FILTER_TYPE.decode(1), "Vintage");
        assert_eq!(DECODE_FILTER_TYPE.decode(2), "Sepia");
        assert_eq!(DECODE_FILTER_TYPE.decode(6), "Vivid");
    }

    #[test]
    fn test_decode_auto_enhance() {
        assert_eq!(DECODE_AUTO_ENHANCE.decode(0), "Off");
        assert_eq!(DECODE_AUTO_ENHANCE.decode(2), "Medium");
        assert_eq!(DECODE_AUTO_ENHANCE.decode(3), "High");
    }

    #[test]
    fn test_format_adjustment() {
        assert_eq!(format_adjustment(25), "+25");
        assert_eq!(format_adjustment(-15), "-15");
    }
}
