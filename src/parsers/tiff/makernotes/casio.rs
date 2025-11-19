//! Casio MakerNote parser
//!
//! Parses Casio digital camera-specific EXIF MakerNote tags.
//! Casio was known for the Exilim series of ultra-compact digital cameras
//! with high-speed capture and unique features.
//!
//! ## Supported Cameras
//! - Exilim series (EX-Z, EX-S, EX-F)
//! - QV series (early digital cameras)
//! - GV series (with LCD viewfinder)
//!
//! ## Supported Features
//! - High-speed burst mode settings
//! - Best Shot scene selection
//! - Continuous shooting modes
//! - Image quality and sharpness
//! - Flash and focus settings
//! - Color mode and effects
//! - Digital zoom information
//!
//! ## Tag Structure
//! Casio uses a standard IFD format with manufacturer-specific tag IDs.

#![allow(dead_code)]
#![allow(unused_imports)]

use crate::const_decoder;
use crate::parsers::tiff::ifd_parser::{ByteOrder, IfdEntry};
use std::collections::HashMap;

use super::shared::MakerNoteParser;

// Casio MakerNote Tag IDs
const CASIO_RECORDING_MODE: u16 = 0x0001; // Recording mode
const CASIO_QUALITY: u16 = 0x0002; // Image quality
const CASIO_FOCUS_MODE: u16 = 0x0003; // Focus mode
const CASIO_FLASH_MODE: u16 = 0x0004; // Flash mode
const CASIO_FLASH_INTENSITY: u16 = 0x0005; // Flash intensity
const CASIO_WHITE_BALANCE: u16 = 0x0007; // White balance
const CASIO_DIGITAL_ZOOM: u16 = 0x000A; // Digital zoom
const CASIO_SHARPNESS: u16 = 0x000B; // Sharpness
const CASIO_CONTRAST: u16 = 0x000C; // Contrast
const CASIO_SATURATION: u16 = 0x000D; // Saturation
const CASIO_CCD_SENSITIVITY: u16 = 0x0014; // CCD ISO sensitivity
const CASIO_COLOR_MODE: u16 = 0x0015; // Color mode
const CASIO_ENHANCEMENT: u16 = 0x0016; // Image enhancement
const CASIO_COLOR_FILTER: u16 = 0x0017; // Color filter effect
const CASIO_CONTINUOUS_MODE: u16 = 0x001A; // Continuous shooting mode
const CASIO_BEST_SHOT_MODE: u16 = 0x001B; // Best Shot scene mode
const CASIO_SLOW_SHUTTER: u16 = 0x0020; // Slow shutter setting

// Decodes Casio recording mode
const_decoder! {
    DECODE_RECORDING_MODE, u16, [
        (1, "Single"),
        (2, "Panorama"),
        (3, "Night Scene"),
        (4, "Portrait"),
        (5, "Landscape"),
        (6, "Sports"),
    ]
}

// Decodes Casio image quality
const_decoder! {
    DECODE_QUALITY, u16, [
        (1, "Economy"),
        (2, "Normal"),
        (3, "Fine"),
    ]
}

// Decodes Casio focus mode
const_decoder! {
    DECODE_FOCUS_MODE, u16, [
        (0, "Normal"),
        (1, "Macro"),
        (2, "Super Macro"),
        (3, "Infinity"),
        (4, "Manual"),
    ]
}

// Decodes Casio flash mode
const_decoder! {
    DECODE_FLASH_MODE, u16, [
        (1, "Auto"),
        (2, "On"),
        (3, "Off"),
        (4, "Red-eye Reduction"),
        (5, "Slow Sync"),
    ]
}

// Decodes Casio white balance
const_decoder! {
    DECODE_WHITE_BALANCE, u16, [
        (0, "Auto"),
        (1, "Daylight"),
        (2, "Shade"),
        (3, "Tungsten"),
        (4, "Fluorescent"),
        (5, "Manual"),
    ]
}

// Decodes Casio color mode
const_decoder! {
    DECODE_COLOR_MODE, u16, [
        (0, "Off"),
        (1, "On"),
    ]
}

// Decodes Casio enhancement mode
const_decoder! {
    DECODE_ENHANCEMENT, u16, [
        (0, "Off"),
        (1, "Red"),
        (2, "Green"),
        (3, "Blue"),
        (4, "Flesh Tones"),
    ]
}

// Decodes Casio Best Shot mode
const_decoder! {
    DECODE_BEST_SHOT_MODE, u16, [
        (0, "Off"),
        (1, "Portrait"),
        (2, "Scenery"),
        (3, "Night Scene"),
        (4, "Night Scene Portrait"),
        (5, "Sunset"),
        (6, "High Sensitivity"),
        (7, "Children"),
        (8, "Sports"),
        (9, "Candlelight"),
        (10, "Fireworks"),
        (11, "Food"),
        (12, "Text"),
    ]
}

/// Extracts a 16-bit unsigned value from IFD entry
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

/// Casio MakerNote parser implementation
pub struct CasioParser;

impl Default for CasioParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CasioParser {
    /// Creates a new Casio parser instance
    pub fn new() -> Self {
        CasioParser
    }

    /// Parse a single IFD entry and extract tag value
    fn parse_entry(
        &self,
        entry: &IfdEntry,
        data: &[u8],
        byte_order: ByteOrder,
        tags: &mut HashMap<String, String>,
    ) {
        match entry.tag_id {
            CASIO_RECORDING_MODE => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert(
                        "Casio:RecordingMode".to_string(),
                        DECODE_RECORDING_MODE.decode(value),
                    );
                }
            }
            CASIO_QUALITY => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert("Casio:Quality".to_string(), DECODE_QUALITY.decode(value));
                }
            }
            CASIO_FOCUS_MODE => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert(
                        "Casio:FocusMode".to_string(),
                        DECODE_FOCUS_MODE.decode(value),
                    );
                }
            }
            CASIO_FLASH_MODE => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert(
                        "Casio:FlashMode".to_string(),
                        DECODE_FLASH_MODE.decode(value),
                    );
                }
            }
            CASIO_FLASH_INTENSITY => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert("Casio:FlashIntensity".to_string(), value.to_string());
                }
            }
            CASIO_WHITE_BALANCE => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert(
                        "Casio:WhiteBalance".to_string(),
                        DECODE_WHITE_BALANCE.decode(value),
                    );
                }
            }
            CASIO_DIGITAL_ZOOM => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert("Casio:DigitalZoom".to_string(), value.to_string());
                }
            }
            CASIO_SHARPNESS => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert("Casio:Sharpness".to_string(), value.to_string());
                }
            }
            CASIO_CONTRAST => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert("Casio:Contrast".to_string(), value.to_string());
                }
            }
            CASIO_SATURATION => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert("Casio:Saturation".to_string(), value.to_string());
                }
            }
            CASIO_CCD_SENSITIVITY => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert("Casio:CCDSensitivity".to_string(), value.to_string());
                }
            }
            CASIO_COLOR_MODE => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert(
                        "Casio:ColorMode".to_string(),
                        DECODE_COLOR_MODE.decode(value),
                    );
                }
            }
            CASIO_ENHANCEMENT => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert(
                        "Casio:Enhancement".to_string(),
                        DECODE_ENHANCEMENT.decode(value),
                    );
                }
            }
            CASIO_BEST_SHOT_MODE => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert(
                        "Casio:BestShotMode".to_string(),
                        DECODE_BEST_SHOT_MODE.decode(value),
                    );
                }
            }
            CASIO_CONTINUOUS_MODE => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    let mode = if value > 0 { "On" } else { "Off" };
                    tags.insert("Casio:ContinuousMode".to_string(), mode.to_string());
                }
            }
            CASIO_SLOW_SHUTTER => {
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    let status = if value > 0 { "On" } else { "Off" };
                    tags.insert("Casio:SlowShutter".to_string(), status.to_string());
                }
            }
            _ => {}
        }
    }
}

impl MakerNoteParser for CasioParser {
    fn manufacturer_name(&self) -> &'static str {
        "Casio"
    }

    fn tag_prefix(&self) -> &'static str {
        "Casio:"
    }

    fn parse(
        &self,
        data: &[u8],
        byte_order: ByteOrder,
        tags: &mut HashMap<String, String>,
    ) -> Result<(), String> {
        if data.len() < 2 {
            return Err("Casio MakerNote data too short".to_string());
        }

        let ifd_offset = 0;

        // Read number of IFD entries
        let entry_count = match byte_order {
            ByteOrder::LittleEndian => u16::from_le_bytes([data[ifd_offset], data[ifd_offset + 1]]),
            ByteOrder::BigEndian => u16::from_be_bytes([data[ifd_offset], data[ifd_offset + 1]]),
        };

        if entry_count == 0 || entry_count > 500 {
            return Err(format!("Invalid entry count: {}", entry_count));
        }

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
    fn test_decode_recording_mode() {
        assert_eq!(DECODE_RECORDING_MODE.decode(1), "Single");
        assert_eq!(DECODE_RECORDING_MODE.decode(4), "Portrait");
    }

    #[test]
    fn test_decode_quality() {
        assert_eq!(DECODE_QUALITY.decode(1), "Economy");
        assert_eq!(DECODE_QUALITY.decode(3), "Fine");
    }

    #[test]
    fn test_decode_focus_mode() {
        assert_eq!(DECODE_FOCUS_MODE.decode(1), "Macro");
        assert_eq!(DECODE_FOCUS_MODE.decode(2), "Super Macro");
    }

    #[test]
    fn test_decode_best_shot_mode() {
        assert_eq!(DECODE_BEST_SHOT_MODE.decode(0), "Off");
        assert_eq!(DECODE_BEST_SHOT_MODE.decode(10), "Fireworks");
    }

    #[test]
    fn test_casio_parser_trait() {
        let parser = CasioParser::new();
        assert_eq!(parser.manufacturer_name(), "Casio");
        assert_eq!(parser.tag_prefix(), "Casio:");
    }

    #[test]
    fn test_parse_quality_tag() {
        let parser = CasioParser::new();
        let mut data = Vec::new();

        data.extend_from_slice(&[0x01, 0x00]); // 1 entry
        data.extend_from_slice(&[0x02, 0x00]); // Tag: CASIO_QUALITY
        data.extend_from_slice(&[0x03, 0x00]); // Type: SHORT
        data.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // Count: 1
        data.extend_from_slice(&[0x02, 0x00, 0x00, 0x00]); // Value: 2 (Normal)

        let mut tags = HashMap::new();
        let result = parser.parse(&data, ByteOrder::LittleEndian, &mut tags);

        assert!(result.is_ok());
        assert_eq!(tags.get("Casio:Quality"), Some(&"Normal".to_string()));
    }

    #[test]
    fn test_parse_focus_mode_tag() {
        let parser = CasioParser::new();
        let mut data = Vec::new();

        data.extend_from_slice(&[0x01, 0x00]); // 1 entry
        data.extend_from_slice(&[0x03, 0x00]); // Tag: CASIO_FOCUS_MODE
        data.extend_from_slice(&[0x03, 0x00]); // Type: SHORT
        data.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // Count: 1
        data.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // Value: 1 (Macro)

        let mut tags = HashMap::new();
        let result = parser.parse(&data, ByteOrder::LittleEndian, &mut tags);

        assert!(result.is_ok());
        assert_eq!(tags.get("Casio:FocusMode"), Some(&"Macro".to_string()));
    }

    #[test]
    fn test_parse_best_shot_tag() {
        let parser = CasioParser::new();
        let mut data = Vec::new();

        data.extend_from_slice(&[0x01, 0x00]); // 1 entry
        data.extend_from_slice(&[0x1B, 0x00]); // Tag: CASIO_BEST_SHOT_MODE
        data.extend_from_slice(&[0x03, 0x00]); // Type: SHORT
        data.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // Count: 1
        data.extend_from_slice(&[0x08, 0x00, 0x00, 0x00]); // Value: 8 (Sports)

        let mut tags = HashMap::new();
        let result = parser.parse(&data, ByteOrder::LittleEndian, &mut tags);

        assert!(result.is_ok());
        assert_eq!(tags.get("Casio:BestShotMode"), Some(&"Sports".to_string()));
    }
}
