//! Minolta MakerNote parser
//!
//! Parses Minolta (and early Konica Minolta) camera-specific EXIF MakerNote tags.
//! Minolta was a major camera manufacturer from 1985-2006, later merged with Konica
//! to form Konica Minolta before Sony acquired the camera division in 2006.
//!
//! ## Supported Cameras
//! - Minolta Maxxum/Dynax series (film and early digital)
//! - DiMAGE digital camera series
//! - Early Konica Minolta models (7D, 5D)
//!
//! ## Supported Features
//! - Camera model and firmware
//! - Exposure settings and modes
//! - Focus mode and AF points
//! - Image quality and color settings
//! - Flash settings
//! - Lens information with database lookup
//! - White balance and metering
//!
//! ## Tag Structure
//! Minolta uses a standard IFD format with manufacturer-specific tag IDs.

#![allow(dead_code)]
#![allow(unused_imports)]

use crate::parsers::tiff::ifd_parser::{ByteOrder, IfdEntry};
use std::collections::HashMap;

use super::minolta_lens_database::lookup_minolta_lens;
use super::shared::MakerNoteParser;
use crate::const_decoder;

// Minolta MakerNote Tag IDs
const MINOLTA_CAMERA_SETTINGS_OLD: u16 = 0x0001; // Camera settings (old models)
const MINOLTA_CAMERA_SETTINGS: u16 = 0x0003; // Camera settings array
const MINOLTA_IMAGE_SIZE: u16 = 0x0040; // Image dimensions
const MINOLTA_IMAGE_QUALITY: u16 = 0x0041; // Quality setting
const MINOLTA_FLASH_MODE: u16 = 0x0042; // Flash mode
const MINOLTA_FLASH_EXPOSURE_COMP: u16 = 0x0043; // Flash compensation
const MINOLTA_TELECONVERTER: u16 = 0x0044; // Teleconverter used
const MINOLTA_WHITE_BALANCE: u16 = 0x0045; // White balance mode
const MINOLTA_BRIGHTNESS: u16 = 0x0046; // Scene brightness value
const MINOLTA_FOCUS_MODE: u16 = 0x0047; // Manual/Auto focus
const MINOLTA_FOCUS_DISTANCE: u16 = 0x0048; // Focus distance
const MINOLTA_ZOOM_POSITION: u16 = 0x004A; // Zoom position
const MINOLTA_MACRO_MODE: u16 = 0x004B; // Macro mode on/off
const MINOLTA_SHARPNESS: u16 = 0x004C; // Sharpness setting
const MINOLTA_CONTRAST: u16 = 0x004D; // Contrast setting
const MINOLTA_SATURATION: u16 = 0x004E; // Color saturation
const MINOLTA_COLOR_MODE: u16 = 0x0050; // Color mode
const MINOLTA_SCENE_MODE: u16 = 0x0052; // Scene mode selection
const MINOLTA_EXPOSURE_MODE: u16 = 0x0053; // Exposure program mode
const MINOLTA_LENS_ID: u16 = 0x0054; // Lens model ID
const MINOLTA_MIN_FOCAL_LENGTH: u16 = 0x0055; // Min focal length
const MINOLTA_MAX_FOCAL_LENGTH: u16 = 0x0056; // Max focal length
const MINOLTA_FIRMWARE_VERSION: u16 = 0x0058; // Camera firmware
const MINOLTA_AF_POINTS: u16 = 0x0059; // AF points used

// Decodes Minolta image quality setting
const_decoder! {
    DECODE_IMAGE_QUALITY, u16, [
        (0, "Raw"),
        (1, "Super Fine"),
        (2, "Fine"),
        (3, "Standard"),
        (4, "Economy"),
        (5, "Extra Fine"),
    ]
}

// Decodes Minolta flash mode
const_decoder! {
    DECODE_FLASH_MODE, u16, [
        (0, "Auto"),
        (1, "On"),
        (2, "Off"),
        (3, "Red-eye Reduction"),
        (4, "Slow Sync"),
        (5, "Rear Curtain Sync"),
        (6, "Fill Flash"),
    ]
}

// Decodes Minolta white balance mode
const_decoder! {
    DECODE_WHITE_BALANCE, u16, [
        (0, "Auto"),
        (1, "Daylight"),
        (2, "Cloudy"),
        (3, "Tungsten"),
        (4, "Fluorescent"),
        (5, "Flash"),
        (6, "Shade"),
        (7, "Custom"),
        (8, "Kelvin"),
    ]
}

// Decodes Minolta focus mode
const_decoder! {
    DECODE_FOCUS_MODE, u16, [
        (0, "Auto"),
        (1, "Manual"),
        (2, "AF-C (Continuous)"),
        (3, "AF-S (Single)"),
        (4, "AF-A (Automatic)"),
        (5, "DMF (Direct Manual Focus)"),
    ]
}

// Decodes Minolta color mode
const_decoder! {
    DECODE_COLOR_MODE, u16, [
        (0, "Natural"),
        (1, "Vivid"),
        (2, "Portrait"),
        (3, "Landscape"),
        (4, "Black & White"),
        (5, "Adobe RGB"),
        (6, "Neutral"),
    ]
}

// Decodes Minolta exposure mode
const_decoder! {
    DECODE_EXPOSURE_MODE, u16, [
        (0, "Program"),
        (1, "Aperture Priority"),
        (2, "Shutter Priority"),
        (3, "Manual"),
        (4, "Auto"),
        (5, "Portrait"),
        (6, "Landscape"),
        (7, "Sports"),
        (8, "Night Portrait"),
        (9, "Macro"),
    ]
}

// Decodes Minolta scene mode
const_decoder! {
    DECODE_SCENE_MODE, u16, [
        (0, "Standard"),
        (1, "Portrait"),
        (2, "Landscape"),
        (3, "Sports"),
        (4, "Sunset"),
        (5, "Night View"),
        (6, "Night Portrait"),
        (7, "Fireworks"),
        (8, "Food"),
        (9, "Text"),
    ]
}

/// Extracts a 16-bit unsigned value from IFD entry
///
/// # Arguments
/// * `entry` - IFD entry containing the value
/// * `_data` - Full MakerNote data buffer (unused for inline values)
/// * `byte_order` - Byte order for parsing
///
/// # Returns
/// Extracted value or None if invalid
fn extract_u16_value(entry: &IfdEntry, _data: &[u8], byte_order: ByteOrder) -> Option<u16> {
    if entry.value_count != 1 {
        return None;
    }

    // For SHORT type (count=1), value is inline in value_offset field
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
        // Inline string (stored in value_offset field)
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
        // External string (offset points to data)
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

/// Minolta MakerNote parser implementation
pub struct MinoltaParser;

impl Default for MinoltaParser {
    fn default() -> Self {
        Self::new()
    }
}

impl MinoltaParser {
    /// Creates a new Minolta parser instance
    pub fn new() -> Self {
        MinoltaParser
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
            MINOLTA_IMAGE_QUALITY => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert(
                        "Minolta:ImageQuality".to_string(),
                        DECODE_IMAGE_QUALITY.decode(value),
                    );
                }
            }
            MINOLTA_FLASH_MODE => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert(
                        "Minolta:FlashMode".to_string(),
                        DECODE_FLASH_MODE.decode(value),
                    );
                }
            }
            MINOLTA_FLASH_EXPOSURE_COMP => {
                if let Some(value) = extract_i16_value(entry, data, byte_order) {
                    let ev = value as f32 / 10.0;
                    tags.insert(
                        "Minolta:FlashExposureComp".to_string(),
                        format!("{:.1} EV", ev),
                    );
                }
            }
            MINOLTA_WHITE_BALANCE => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert(
                        "Minolta:WhiteBalance".to_string(),
                        DECODE_WHITE_BALANCE.decode(value),
                    );
                }
            }
            MINOLTA_FOCUS_MODE => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert(
                        "Minolta:FocusMode".to_string(),
                        DECODE_FOCUS_MODE.decode(value),
                    );
                }
            }
            MINOLTA_MACRO_MODE => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    let mode = if value > 0 { "On" } else { "Off" };
                    tags.insert("Minolta:MacroMode".to_string(), mode.to_string());
                }
            }
            MINOLTA_SHARPNESS => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert("Minolta:Sharpness".to_string(), value.to_string());
                }
            }
            MINOLTA_CONTRAST => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert("Minolta:Contrast".to_string(), value.to_string());
                }
            }
            MINOLTA_SATURATION => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert("Minolta:Saturation".to_string(), value.to_string());
                }
            }
            MINOLTA_COLOR_MODE => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert(
                        "Minolta:ColorMode".to_string(),
                        DECODE_COLOR_MODE.decode(value),
                    );
                }
            }
            MINOLTA_SCENE_MODE => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert(
                        "Minolta:SceneMode".to_string(),
                        DECODE_SCENE_MODE.decode(value),
                    );
                }
            }
            MINOLTA_EXPOSURE_MODE => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert(
                        "Minolta:ExposureMode".to_string(),
                        DECODE_EXPOSURE_MODE.decode(value),
                    );
                }
            }
            MINOLTA_LENS_ID => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    // Store the lens ID
                    tags.insert("Minolta:LensID".to_string(), format!("0x{:04X}", value));

                    // Lookup lens name from database
                    if let Some(lens_name) = lookup_minolta_lens(value) {
                        tags.insert("Minolta:LensType".to_string(), lens_name);
                    }
                }
            }
            MINOLTA_MIN_FOCAL_LENGTH => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert(
                        "Minolta:MinFocalLength".to_string(),
                        format!("{} mm", value),
                    );
                }
            }
            MINOLTA_MAX_FOCAL_LENGTH => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert(
                        "Minolta:MaxFocalLength".to_string(),
                        format!("{} mm", value),
                    );
                }
            }
            MINOLTA_FIRMWARE_VERSION => {
                if let Some(version) = extract_string(entry, data, byte_order) {
                    tags.insert("Minolta:FirmwareVersion".to_string(), version);
                }
            }
            MINOLTA_IMAGE_SIZE => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    let size_str = match value {
                        0 => "Full",
                        1 => "Medium",
                        2 => "Small",
                        _ => "Unknown",
                    };
                    tags.insert("Minolta:ImageSize".to_string(), size_str.to_string());
                }
            }
            MINOLTA_TELECONVERTER => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    let tc = match value {
                        0 => "None",
                        1 => "1.4x",
                        2 => "2.0x",
                        _ => "Unknown",
                    };
                    tags.insert("Minolta:Teleconverter".to_string(), tc.to_string());
                }
            }
            _ => {
                // Unknown tag - skip for now
            }
        }
    }
}

impl MakerNoteParser for MinoltaParser {
    fn manufacturer_name(&self) -> &'static str {
        "Minolta"
    }

    fn tag_prefix(&self) -> &'static str {
        "Minolta:"
    }

    fn parse(
        &self,
        data: &[u8],
        byte_order: ByteOrder,
        tags: &mut HashMap<String, String>,
    ) -> Result<(), String> {
        if data.len() < 2 {
            return Err("Minolta MakerNote data too short".to_string());
        }

        // Minolta MakerNotes typically start immediately with IFD
        // Some models may have a small header, but most don't
        let ifd_offset = 0;

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
        let entry_size = 12; // Standard IFD entry size
        let mut offset = ifd_offset + 2;

        for _ in 0..entry_count {
            if offset + entry_size > data.len() {
                break;
            }

            // Parse IFD entry manually
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

    fn lookup_lens(&self, lens_id: u16) -> Option<String> {
        lookup_minolta_lens(lens_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_image_quality() {
        assert_eq!(DECODE_IMAGE_QUALITY.decode(0), "Raw");
        assert_eq!(DECODE_IMAGE_QUALITY.decode(2), "Fine");
        assert_eq!(DECODE_IMAGE_QUALITY.decode(5), "Extra Fine");
        assert_eq!(DECODE_IMAGE_QUALITY.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_flash_mode() {
        assert_eq!(DECODE_FLASH_MODE.decode(0), "Auto");
        assert_eq!(DECODE_FLASH_MODE.decode(1), "On");
        assert_eq!(DECODE_FLASH_MODE.decode(3), "Red-eye Reduction");
        assert_eq!(DECODE_FLASH_MODE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_white_balance() {
        assert_eq!(DECODE_WHITE_BALANCE.decode(0), "Auto");
        assert_eq!(DECODE_WHITE_BALANCE.decode(3), "Tungsten");
        assert_eq!(DECODE_WHITE_BALANCE.decode(7), "Custom");
        assert_eq!(DECODE_WHITE_BALANCE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_focus_mode() {
        assert_eq!(DECODE_FOCUS_MODE.decode(0), "Auto");
        assert_eq!(DECODE_FOCUS_MODE.decode(2), "AF-C (Continuous)");
        assert_eq!(DECODE_FOCUS_MODE.decode(5), "DMF (Direct Manual Focus)");
        assert_eq!(DECODE_FOCUS_MODE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_color_mode() {
        assert_eq!(DECODE_COLOR_MODE.decode(0), "Natural");
        assert_eq!(DECODE_COLOR_MODE.decode(4), "Black & White");
        assert_eq!(DECODE_COLOR_MODE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_exposure_mode() {
        assert_eq!(DECODE_EXPOSURE_MODE.decode(0), "Program");
        assert_eq!(DECODE_EXPOSURE_MODE.decode(3), "Manual");
        assert_eq!(DECODE_EXPOSURE_MODE.decode(7), "Sports");
        assert_eq!(DECODE_EXPOSURE_MODE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_minolta_parser_trait() {
        let parser = MinoltaParser::new();
        assert_eq!(parser.manufacturer_name(), "Minolta");
        assert_eq!(parser.tag_prefix(), "Minolta:");
    }

    #[test]
    fn test_parse_image_quality_tag() {
        let parser = MinoltaParser::new();
        let mut data = Vec::new();

        // Create minimal IFD with one entry
        data.extend_from_slice(&[0x01, 0x00]); // 1 entry

        // Image quality tag entry (tag=0x0041, type=3 (SHORT), count=1, value=2 (Fine))
        data.extend_from_slice(&[0x41, 0x00]); // Tag
        data.extend_from_slice(&[0x03, 0x00]); // Type: SHORT
        data.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // Count: 1
        data.extend_from_slice(&[0x02, 0x00, 0x00, 0x00]); // Value: 2 (inline)

        let mut tags = HashMap::new();
        let result = parser.parse(&data, ByteOrder::LittleEndian, &mut tags);

        assert!(result.is_ok());
        assert_eq!(tags.get("Minolta:ImageQuality"), Some(&"Fine".to_string()));
    }

    #[test]
    fn test_lens_lookup() {
        let parser = MinoltaParser::new();
        assert_eq!(
            parser.lookup_lens(0x0100),
            Some("AF 50mm f/1.4".to_string())
        );
        assert_eq!(
            parser.lookup_lens(0x0200),
            Some("AF 28-70mm f/2.8 G".to_string())
        );
        assert_eq!(parser.lookup_lens(0xFFFF), None);
    }
}
