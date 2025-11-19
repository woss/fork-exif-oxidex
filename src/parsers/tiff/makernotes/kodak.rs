//! Kodak MakerNote parser
//!
//! Parses Kodak digital camera-specific EXIF MakerNote tags.
//! Kodak was a pioneer in digital photography, producing both consumer
//! and professional digital cameras from the 1990s through the 2000s.
//!
//! ## Supported Cameras
//! - EasyShare series (consumer point-and-shoot)
//! - DCS series (professional digital SLRs)
//! - Z-series (advanced zoom cameras)
//! - P-series (point-and-shoot)
//!
//! ## Supported Features
//! - Camera model and firmware
//! - Exposure settings and modes
//! - Focus mode and quality settings
//! - Flash settings
//! - White balance and color mode
//! - Image processing settings
//! - Scene capture modes
//!
//! ## Tag Structure
//! Kodak uses a custom tag structure with manufacturer-specific IDs.

#![allow(dead_code)]
#![allow(unused_imports)]

use crate::const_decoder;
use crate::parsers::tiff::ifd_parser::{ByteOrder, IfdEntry};
use std::collections::HashMap;

use super::shared::MakerNoteParser;

// Kodak MakerNote Tag IDs
const KODAK_MODEL: u16 = 0x0001; // Camera model
const KODAK_QUALITY: u16 = 0x0009; // Image quality
const KODAK_BURST_MODE: u16 = 0x000A; // Burst mode setting
const KODAK_SHUTTER_MODE: u16 = 0x000C; // Shutter mode
const KODAK_FOCUS_MODE: u16 = 0x000D; // Focus mode
const KODAK_WHITE_BALANCE: u16 = 0x000E; // White balance
const KODAK_FLASH_MODE: u16 = 0x0010; // Flash mode
const KODAK_FLASH_FIRED: u16 = 0x0011; // Flash fired status
const KODAK_ISO_SETTING: u16 = 0x0014; // ISO sensitivity
const KODAK_COLOR_MODE: u16 = 0x001A; // Color mode
const KODAK_SHARPNESS: u16 = 0x001C; // Sharpness setting
const KODAK_SATURATION: u16 = 0x001D; // Color saturation
const KODAK_CONTRAST: u16 = 0x001E; // Contrast setting
const KODAK_SCENE_MODE: u16 = 0x0020; // Scene capture mode
const KODAK_EXPOSURE_BIAS: u16 = 0x0022; // Exposure compensation
const KODAK_FIRMWARE: u16 = 0x0025; // Firmware version
const KODAK_TIME_ZONE: u16 = 0x0029; // Time zone offset

// Kodak signature for validation
const KODAK_SIGNATURE: &[u8] = b"KDK";

// Decodes Kodak image quality setting
const_decoder! {
    DECODE_QUALITY, u16, [
        (1, "Fine"),
        (2, "Normal"),
        (3, "Economy"),
        (4, "Best"),
    ]
}

// Decodes Kodak burst mode
const_decoder! {
    DECODE_BURST_MODE, u16, [
        (0, "Off"),
        (1, "On"),
        (2, "Continuous"),
    ]
}

// Decodes Kodak focus mode
const_decoder! {
    DECODE_FOCUS_MODE, u16, [
        (0, "Auto"),
        (1, "Manual"),
        (2, "Macro"),
        (3, "Infinity"),
        (4, "Multi-Zone"),
        (5, "Center"),
    ]
}

// Decodes Kodak white balance mode
const_decoder! {
    DECODE_WHITE_BALANCE, u16, [
        (0, "Auto"),
        (1, "Daylight"),
        (2, "Tungsten"),
        (3, "Fluorescent"),
        (4, "Flash"),
        (5, "Cloudy"),
        (6, "Shade"),
        (7, "Manual"),
    ]
}

// Decodes Kodak flash mode
const_decoder! {
    DECODE_FLASH_MODE, u16, [
        (0, "Auto"),
        (1, "Fill Flash"),
        (2, "Off"),
        (3, "Red-eye Reduction"),
        (4, "Slow Sync"),
    ]
}

// Decodes Kodak color mode
const_decoder! {
    DECODE_COLOR_MODE, u16, [
        (0, "Natural"),
        (1, "Vivid"),
        (2, "Black & White"),
        (3, "Sepia"),
        (4, "High Saturation"),
        (5, "Low Saturation"),
    ]
}

// Decodes Kodak scene mode
const_decoder! {
    DECODE_SCENE_MODE, u16, [
        (0, "Auto"),
        (1, "Portrait"),
        (2, "Landscape"),
        (3, "Sports"),
        (4, "Night"),
        (5, "Sunset"),
        (6, "Snow"),
        (7, "Beach"),
        (8, "Fireworks"),
        (9, "Text"),
    ]
}

/// Extracts a 16-bit unsigned value from IFD entry
///
/// # Arguments
/// * `entry` - IFD entry containing the value
/// * `_data` - Full MakerNote data buffer
/// * `byte_order` - Byte order for parsing
///
/// # Returns
/// Extracted value or None if invalid
fn extract_u16_value(entry: &IfdEntry, _data: &[u8], byte_order: ByteOrder) -> Option<u16> {
    if entry.value_count != 1 {
        return None;
    }

    let value = match byte_order {
        ByteOrder::LittleEndian => (entry.value_offset & 0xFFFF) as u16,
        ByteOrder::BigEndian => ((entry.value_offset >> 16) & 0xFFFF) as u16,
    };

    Some(value)
}

/// Extracts a signed 16-bit value from IFD entry
///
/// # Arguments
/// * `entry` - IFD entry containing the value
/// * `_data` - Full MakerNote data buffer
/// * `byte_order` - Byte order for parsing
///
/// # Returns
/// Extracted value or None if invalid
fn extract_i16_value(entry: &IfdEntry, _data: &[u8], byte_order: ByteOrder) -> Option<i16> {
    if entry.value_count != 1 {
        return None;
    }

    let value = match byte_order {
        ByteOrder::LittleEndian => (entry.value_offset & 0xFFFF) as i16,
        ByteOrder::BigEndian => ((entry.value_offset >> 16) & 0xFFFF) as i16,
    };

    Some(value)
}

/// Extracts an ASCII string from IFD entry
///
/// # Arguments
/// * `entry` - IFD entry containing the string
/// * `data` - Full MakerNote data buffer
/// * `byte_order` - Byte order for parsing
///
/// # Returns
/// Extracted string or None if invalid
fn extract_string(entry: &IfdEntry, data: &[u8], byte_order: ByteOrder) -> Option<String> {
    if entry.value_count == 0 {
        return None;
    }

    let value_bytes = if entry.value_count <= 4 {
        let mut bytes = Vec::new();
        for i in 0..entry.value_count as usize {
            let byte = match byte_order {
                ByteOrder::LittleEndian => ((entry.value_offset >> (i * 8)) & 0xFF) as u8,
                ByteOrder::BigEndian => ((entry.value_offset >> (24 - i * 8)) & 0xFF) as u8,
            };
            if byte == 0 {
                break;
            }
            bytes.push(byte);
        }
        bytes
    } else {
        let offset = entry.value_offset as usize;
        if offset >= data.len() {
            return None;
        }
        let end = std::cmp::min(offset + entry.value_count as usize, data.len());
        data[offset..end].to_vec()
    };

    if value_bytes.is_empty() {
        return None;
    }

    let string = String::from_utf8_lossy(&value_bytes)
        .trim_end_matches('\0')
        .to_string();

    if string.is_empty() {
        None
    } else {
        Some(string)
    }
}

/// Kodak MakerNote parser implementation
pub struct KodakParser;

impl Default for KodakParser {
    fn default() -> Self {
        Self::new()
    }
}

impl KodakParser {
    /// Creates a new Kodak parser instance
    pub fn new() -> Self {
        KodakParser
    }

    /// Parse a single IFD entry and extract tag value
    ///
    /// # Arguments
    /// * `entry` - IFD entry to parse
    /// * `data` - Full MakerNote data buffer
    /// * `byte_order` - Byte order for multi-byte values
    /// * `tags` - HashMap to insert extracted tags into
    fn parse_entry(
        &self,
        entry: &IfdEntry,
        data: &[u8],
        byte_order: ByteOrder,
        tags: &mut HashMap<String, String>,
    ) {
        let tag_id = entry.tag_id;

        match tag_id {
            KODAK_MODEL => {
                if let Some(model) = extract_string(entry, data, byte_order) {
                    tags.insert("Kodak:CameraModel".to_string(), model);
                }
            }
            KODAK_QUALITY => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert("Kodak:Quality".to_string(), DECODE_QUALITY.decode(value));
                }
            }
            KODAK_BURST_MODE => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert(
                        "Kodak:BurstMode".to_string(),
                        DECODE_BURST_MODE.decode(value),
                    );
                }
            }
            KODAK_FOCUS_MODE => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert(
                        "Kodak:FocusMode".to_string(),
                        DECODE_FOCUS_MODE.decode(value),
                    );
                }
            }
            KODAK_WHITE_BALANCE => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert(
                        "Kodak:WhiteBalance".to_string(),
                        DECODE_WHITE_BALANCE.decode(value),
                    );
                }
            }
            KODAK_FLASH_MODE => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert(
                        "Kodak:FlashMode".to_string(),
                        DECODE_FLASH_MODE.decode(value),
                    );
                }
            }
            KODAK_FLASH_FIRED => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    let fired = if value > 0 { "Yes" } else { "No" };
                    tags.insert("Kodak:FlashFired".to_string(), fired.to_string());
                }
            }
            KODAK_ISO_SETTING => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert("Kodak:ISO".to_string(), value.to_string());
                }
            }
            KODAK_COLOR_MODE => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert(
                        "Kodak:ColorMode".to_string(),
                        DECODE_COLOR_MODE.decode(value),
                    );
                }
            }
            KODAK_SHARPNESS => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert("Kodak:Sharpness".to_string(), value.to_string());
                }
            }
            KODAK_SATURATION => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert("Kodak:Saturation".to_string(), value.to_string());
                }
            }
            KODAK_CONTRAST => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert("Kodak:Contrast".to_string(), value.to_string());
                }
            }
            KODAK_SCENE_MODE => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert(
                        "Kodak:SceneMode".to_string(),
                        DECODE_SCENE_MODE.decode(value),
                    );
                }
            }
            KODAK_EXPOSURE_BIAS => {
                if let Some(value) = extract_i16_value(entry, data, byte_order) {
                    let ev = value as f32 / 10.0;
                    tags.insert("Kodak:ExposureBias".to_string(), format!("{:.1} EV", ev));
                }
            }
            KODAK_FIRMWARE => {
                if let Some(firmware) = extract_string(entry, data, byte_order) {
                    tags.insert("Kodak:FirmwareVersion".to_string(), firmware);
                }
            }
            _ => {
                // Unknown tag - skip
            }
        }
    }
}

impl MakerNoteParser for KodakParser {
    fn manufacturer_name(&self) -> &'static str {
        "Kodak"
    }

    fn tag_prefix(&self) -> &'static str {
        "Kodak:"
    }

    fn parse(
        &self,
        data: &[u8],
        byte_order: ByteOrder,
        tags: &mut HashMap<String, String>,
    ) -> Result<(), String> {
        if data.len() < 2 {
            return Err("Kodak MakerNote data too short".to_string());
        }

        // Kodak MakerNotes may start with "KDK" signature
        let ifd_offset = if data.len() >= 3 && &data[0..3] == KODAK_SIGNATURE {
            // Skip signature and padding
            8
        } else {
            0
        };

        if ifd_offset + 2 > data.len() {
            return Err("Invalid IFD offset".to_string());
        }

        // Read number of IFD entries
        let entry_count = match byte_order {
            ByteOrder::LittleEndian => u16::from_le_bytes([data[ifd_offset], data[ifd_offset + 1]]),
            ByteOrder::BigEndian => u16::from_be_bytes([data[ifd_offset], data[ifd_offset + 1]]),
        };

        if entry_count == 0 || entry_count > 500 {
            return Err(format!(
                "Invalid entry count: {} (expected 1-500)",
                entry_count
            ));
        }

        // Parse each IFD entry
        let entry_size = 12;
        let mut offset = ifd_offset + 2;

        for _ in 0..entry_count {
            if offset + entry_size > data.len() {
                break;
            }

            let tag = match byte_order {
                ByteOrder::LittleEndian => u16::from_le_bytes([data[offset], data[offset + 1]]),
                ByteOrder::BigEndian => u16::from_be_bytes([data[offset], data[offset + 1]]),
            };

            let field_type = match byte_order {
                ByteOrder::LittleEndian => u16::from_le_bytes([data[offset + 2], data[offset + 3]]),
                ByteOrder::BigEndian => u16::from_be_bytes([data[offset + 2], data[offset + 3]]),
            };

            let count = match byte_order {
                ByteOrder::LittleEndian => u32::from_le_bytes([
                    data[offset + 4],
                    data[offset + 5],
                    data[offset + 6],
                    data[offset + 7],
                ]),
                ByteOrder::BigEndian => u32::from_be_bytes([
                    data[offset + 4],
                    data[offset + 5],
                    data[offset + 6],
                    data[offset + 7],
                ]),
            };

            let value_offset = match byte_order {
                ByteOrder::LittleEndian => u32::from_le_bytes([
                    data[offset + 8],
                    data[offset + 9],
                    data[offset + 10],
                    data[offset + 11],
                ]),
                ByteOrder::BigEndian => u32::from_be_bytes([
                    data[offset + 8],
                    data[offset + 9],
                    data[offset + 10],
                    data[offset + 11],
                ]),
            };

            let entry = IfdEntry {
                tag_id: tag,
                field_type,
                value_count: count,
                value_offset,
            };

            self.parse_entry(&entry, data, byte_order, tags);

            offset += entry_size;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_quality() {
        assert_eq!(DECODE_QUALITY.decode(1), "Fine");
        assert_eq!(DECODE_QUALITY.decode(2), "Normal");
        assert_eq!(DECODE_QUALITY.decode(4), "Best");
        assert_eq!(DECODE_QUALITY.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_burst_mode() {
        assert_eq!(DECODE_BURST_MODE.decode(0), "Off");
        assert_eq!(DECODE_BURST_MODE.decode(2), "Continuous");
        assert_eq!(DECODE_BURST_MODE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_focus_mode() {
        assert_eq!(DECODE_FOCUS_MODE.decode(0), "Auto");
        assert_eq!(DECODE_FOCUS_MODE.decode(2), "Macro");
        assert_eq!(DECODE_FOCUS_MODE.decode(4), "Multi-Zone");
        assert_eq!(DECODE_FOCUS_MODE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_white_balance() {
        assert_eq!(DECODE_WHITE_BALANCE.decode(0), "Auto");
        assert_eq!(DECODE_WHITE_BALANCE.decode(2), "Tungsten");
        assert_eq!(DECODE_WHITE_BALANCE.decode(7), "Manual");
        assert_eq!(DECODE_WHITE_BALANCE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_color_mode() {
        assert_eq!(DECODE_COLOR_MODE.decode(0), "Natural");
        assert_eq!(DECODE_COLOR_MODE.decode(2), "Black & White");
        assert_eq!(DECODE_COLOR_MODE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_kodak_parser_trait() {
        let parser = KodakParser::new();
        assert_eq!(parser.manufacturer_name(), "Kodak");
        assert_eq!(parser.tag_prefix(), "Kodak:");
    }

    #[test]
    fn test_parse_quality_tag() {
        let parser = KodakParser::new();
        let mut data = Vec::new();

        data.extend_from_slice(&[0x01, 0x00]); // 1 entry
        data.extend_from_slice(&[0x09, 0x00]); // Tag: KODAK_QUALITY
        data.extend_from_slice(&[0x03, 0x00]); // Type: SHORT
        data.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // Count: 1
        data.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // Value: 1 (Fine)

        let mut tags = HashMap::new();
        let result = parser.parse(&data, ByteOrder::LittleEndian, &mut tags);

        assert!(result.is_ok());
        assert_eq!(tags.get("Kodak:Quality"), Some(&"Fine".to_string()));
    }

    #[test]
    fn test_parse_focus_mode_tag() {
        let parser = KodakParser::new();
        let mut data = Vec::new();

        data.extend_from_slice(&[0x01, 0x00]); // 1 entry
        data.extend_from_slice(&[0x0D, 0x00]); // Tag: KODAK_FOCUS_MODE
        data.extend_from_slice(&[0x03, 0x00]); // Type: SHORT
        data.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // Count: 1
        data.extend_from_slice(&[0x02, 0x00, 0x00, 0x00]); // Value: 2 (Macro)

        let mut tags = HashMap::new();
        let result = parser.parse(&data, ByteOrder::LittleEndian, &mut tags);

        assert!(result.is_ok());
        assert_eq!(tags.get("Kodak:FocusMode"), Some(&"Macro".to_string()));
    }

    #[test]
    fn test_parse_scene_mode_tag() {
        let parser = KodakParser::new();
        let mut data = Vec::new();

        data.extend_from_slice(&[0x01, 0x00]); // 1 entry
        data.extend_from_slice(&[0x20, 0x00]); // Tag: KODAK_SCENE_MODE
        data.extend_from_slice(&[0x03, 0x00]); // Type: SHORT
        data.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // Count: 1
        data.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // Value: 1 (Portrait)

        let mut tags = HashMap::new();
        let result = parser.parse(&data, ByteOrder::LittleEndian, &mut tags);

        assert!(result.is_ok());
        assert_eq!(tags.get("Kodak:SceneMode"), Some(&"Portrait".to_string()));
    }
}
