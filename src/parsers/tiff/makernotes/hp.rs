//! HP MakerNote parser
//!
//! Parses HP PhotoSmart digital camera-specific EXIF MakerNote tags.
//! HP (Hewlett-Packard) produced the PhotoSmart series of digital cameras
//! in the early 2000s before exiting the camera market.
//!
//! ## Supported Cameras
//! - PhotoSmart series (consumer point-and-shoot)
//! - PhotoSmart Pro series (prosumer models)
//!
//! ## Supported Features
//! - Camera model and firmware
//! - Image quality and size settings
//! - Flash and exposure modes
//! - Color settings
//! - Special effects
//!
//! ## Tag Structure
//! HP uses a simple proprietary tag structure.

#![allow(dead_code)]
#![allow(unused_imports)]

use crate::const_decoder;
use crate::parsers::tiff::ifd_parser::{ByteOrder, IfdEntry};
use std::collections::HashMap;

use super::shared::MakerNoteParser;

// HP MakerNote Tag IDs
const HP_MODEL: u16 = 0x0001; // Camera model
const HP_QUALITY: u16 = 0x0003; // Image quality
const HP_COLOR_MODE: u16 = 0x0005; // Color mode setting
const HP_FLASH_MODE: u16 = 0x0007; // Flash mode
const HP_WHITE_BALANCE: u16 = 0x0009; // White balance
const HP_SHARPNESS: u16 = 0x000B; // Sharpness setting

// Decodes HP image quality
const_decoder! {
    DECODE_QUALITY, u16, [
        (1, "Normal"),
        (2, "Fine"),
        (3, "Superfine"),
    ]
}

// Decodes HP color mode
const_decoder! {
    DECODE_COLOR_MODE, u16, [
        (0, "Color"),
        (1, "Black & White"),
        (2, "Sepia"),
    ]
}

// Extracts a u16 value from an IFD entry's value_offset field
// This handles the case where the value is stored inline in the offset field
// rather than as a pointer to external data
fn extract_u16_value(entry: &IfdEntry, _data: &[u8], byte_order: ByteOrder) -> Option<u16> {
    if entry.value_count != 1 {
        return None;
    }
    // Extract the u16 value from the appropriate bytes of the u32 value_offset
    // based on byte order. Little endian uses lower 16 bits, big endian uses upper 16 bits
    let value = match byte_order {
        ByteOrder::LittleEndian => (entry.value_offset & 0xFFFF) as u16,
        ByteOrder::BigEndian => ((entry.value_offset >> 16) & 0xFFFF) as u16,
    };
    Some(value)
}

/// Parser for HP MakerNotes
pub struct HpParser;

impl Default for HpParser {
    fn default() -> Self {
        Self::new()
    }
}

impl HpParser {
    /// Creates a new HP parser instance
    pub fn new() -> Self {
        HpParser
    }

    /// Parses a single HP MakerNote IFD entry and extracts its tag value
    ///
    /// This method handles the various HP-specific tag types and converts
    /// their raw values into human-readable strings using the appropriate
    /// decoder functions or inline logic
    fn parse_entry(
        &self,
        entry: &IfdEntry,
        data: &[u8],
        byte_order: ByteOrder,
        tags: &mut HashMap<String, String>,
    ) {
        match entry.tag_id {
            HP_QUALITY => {
                // Extract and decode image quality setting using const decoder
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert("HP:Quality".to_string(), DECODE_QUALITY.decode(value));
                }
            }
            HP_COLOR_MODE => {
                // Extract and decode color mode setting using const decoder
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert("HP:ColorMode".to_string(), DECODE_COLOR_MODE.decode(value));
                }
            }
            HP_FLASH_MODE => {
                // Flash mode is a simple boolean: 0 = Off, >0 = On
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    let mode = if value > 0 { "On" } else { "Off" };
                    tags.insert("HP:FlashMode".to_string(), mode.to_string());
                }
            }
            HP_SHARPNESS => {
                // Sharpness is stored as a numeric value
                if let Some(value) = extract_u16_value(entry, data, byte_order) {
                    tags.insert("HP:Sharpness".to_string(), value.to_string());
                }
            }
            _ => {}
        }
    }
}

impl MakerNoteParser for HpParser {
    fn manufacturer_name(&self) -> &'static str {
        "HP"
    }

    fn tag_prefix(&self) -> &'static str {
        "HP:"
    }

    /// Parses HP MakerNote data and extracts all available tags
    ///
    /// HP MakerNotes use a standard IFD format starting immediately at offset 0.
    /// This method reads the entry count, then iterates through all IFD entries,
    /// parsing each one according to its tag ID.
    ///
    /// # Arguments
    /// * `data` - The raw MakerNote data buffer
    /// * `byte_order` - The byte order to use for multi-byte values
    /// * `tags` - HashMap to populate with extracted tag name/value pairs
    ///
    /// # Returns
    /// * `Ok(())` if parsing succeeded
    /// * `Err(String)` if data is too short or entry count is invalid
    fn parse(
        &self,
        data: &[u8],
        byte_order: ByteOrder,
        tags: &mut HashMap<String, String>,
    ) -> Result<(), String> {
        // Validate minimum data length (2 bytes for entry count)
        if data.len() < 2 {
            return Err("HP MakerNote data too short".to_string());
        }

        let ifd_offset = 0;
        // Read the number of IFD entries from the first 2 bytes
        let entry_count = match byte_order {
            ByteOrder::LittleEndian => u16::from_le_bytes([data[ifd_offset], data[ifd_offset + 1]]),
            ByteOrder::BigEndian => u16::from_be_bytes([data[ifd_offset], data[ifd_offset + 1]]),
        };

        // Sanity check: entry count should be reasonable (HP cameras have few tags)
        if entry_count == 0 || entry_count > 500 {
            return Err(format!("Invalid entry count: {}", entry_count));
        }

        // Each IFD entry is 12 bytes: 2 (tag) + 2 (type) + 4 (count) + 4 (value/offset)
        let entry_size = 12;
        let mut offset = ifd_offset + 2;

        // Iterate through all IFD entries
        for _ in 0..entry_count {
            // Ensure we have enough data for a complete entry
            if offset + entry_size > data.len() {
                break;
            }

            // Parse the tag ID (2 bytes)
            let tag = match byte_order {
                ByteOrder::LittleEndian => u16::from_le_bytes([data[offset], data[offset + 1]]),
                ByteOrder::BigEndian => u16::from_be_bytes([data[offset], data[offset + 1]]),
            };

            // Parse the field type (2 bytes)
            let field_type = match byte_order {
                ByteOrder::LittleEndian => u16::from_le_bytes([data[offset + 2], data[offset + 3]]),
                ByteOrder::BigEndian => u16::from_be_bytes([data[offset + 2], data[offset + 3]]),
            };

            // Parse the value count (4 bytes)
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

            // Parse the value/offset field (4 bytes)
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

            // Create IFD entry structure and parse it
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
        assert_eq!(DECODE_QUALITY.decode(1), "Normal");
        assert_eq!(DECODE_QUALITY.decode(3), "Superfine");
    }

    #[test]
    fn test_hp_parser_trait() {
        let parser = HpParser::new();
        assert_eq!(parser.manufacturer_name(), "HP");
        assert_eq!(parser.tag_prefix(), "HP:");
    }

    #[test]
    fn test_parse_quality() {
        let parser = HpParser::new();
        let mut data = Vec::new();
        data.extend_from_slice(&[0x01, 0x00]);
        data.extend_from_slice(&[0x03, 0x00]);
        data.extend_from_slice(&[0x03, 0x00]);
        data.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);
        data.extend_from_slice(&[0x02, 0x00, 0x00, 0x00]);

        let mut tags = HashMap::new();
        let result = parser.parse(&data, ByteOrder::LittleEndian, &mut tags);
        assert!(result.is_ok());
        assert_eq!(tags.get("HP:Quality"), Some(&"Fine".to_string()));
    }
}
