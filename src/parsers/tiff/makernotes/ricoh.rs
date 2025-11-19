//! Ricoh MakerNote parser
//!
//! Parses Ricoh digital camera-specific EXIF MakerNote tags.
//! Ricoh (and later Pentax Ricoh) produced compact cameras and
//! specialized models like the GR series and Theta 360 cameras.
//!
//! ## Supported Cameras
//! - GR Digital series (advanced compact)
//! - Caplio series (consumer compact)
//! - CX series (high-zoom compact)
//!
//! ## Supported Features
//! - Camera model and settings
//! - Exposure and focus modes
//! - Image quality settings
//! - Flash and white balance
//! - Special shooting modes
//!
//! ## Tag Structure
//! Ricoh uses a standard IFD format similar to Pentax.

#![allow(dead_code)]

use crate::parsers::tiff::ifd_parser::{ByteOrder, IfdEntry};
use once_cell::sync::Lazy;
use std::collections::HashMap;

use super::shared::ifd_parser_base::{parse_ifd_entries, IfdParserConfig};
use super::shared::tag_registry::TagRegistry;
use super::shared::MakerNoteParser;

use crate::const_decoder;

// Ricoh MakerNote Tag IDs
const RICOH_MODEL: u16 = 0x0001;
const RICOH_FIRMWARE: u16 = 0x0002;
const RICOH_SHOOTING_MODE: u16 = 0x0005;
const RICOH_FLASH_MODE: u16 = 0x000C;
const RICOH_FOCUS_MODE: u16 = 0x001D;
const RICOH_WHITE_BALANCE: u16 = 0x001E;
const RICOH_ISO_SETTING: u16 = 0x0022;
const RICOH_COLOR_MODE: u16 = 0x0034;
const RICOH_SHARPNESS: u16 = 0x0035;

// ============================================================================
// Declarative Decoder Definitions
// ============================================================================

const_decoder!(
    SHOOTING_MODE,
    u16,
    [
        (0, "Auto"),
        (1, "Program"),
        (2, "Aperture Priority"),
        (3, "Manual"),
    ]
);

const_decoder!(FLASH_MODE, u16, [(0, "Auto"), (1, "On"), (2, "Off"),]);

const_decoder!(
    WHITE_BALANCE,
    u16,
    [
        (0, "Auto"),
        (1, "Daylight"),
        (2, "Shade"),
        (3, "Fluorescent"),
        (4, "Tungsten"),
    ]
);

// ============================================================================
// Helper Functions
// ============================================================================

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

// ============================================================================
// Tag Registry
// ============================================================================

static TAG_REGISTRY: Lazy<TagRegistry> = Lazy::new(|| {
    TagRegistry::with_capacity(6)
        .register_simple_u16(RICOH_SHOOTING_MODE, "ShootingMode", &SHOOTING_MODE)
        .register_simple_u16(RICOH_FLASH_MODE, "FlashMode", &FLASH_MODE)
        .register_simple_u16(RICOH_WHITE_BALANCE, "WhiteBalance", &WHITE_BALANCE)
        .register_raw(RICOH_FOCUS_MODE, "FocusMode")
        .register_raw(RICOH_ISO_SETTING, "ISO")
        .register_raw(RICOH_SHARPNESS, "Sharpness")
});

// ============================================================================
// Parser Implementation
// ============================================================================

/// Parser for Ricoh MakerNotes
pub struct RicohParser;

impl Default for RicohParser {
    fn default() -> Self {
        Self::new()
    }
}

impl RicohParser {
    /// Creates a new RicohParser instance
    pub fn new() -> Self {
        RicohParser
    }

    fn parse_entry(
        &self,
        entry: &IfdEntry,
        data: &[u8],
        byte_order: ByteOrder,
        tags: &mut HashMap<String, String>,
    ) {
        if let Some(value) = extract_u16_value(entry, data, byte_order) {
            let tag_name = match TAG_REGISTRY.get_tag_name(entry.tag_id) {
                Some(name) => name,
                None => return,
            };

            let formatted_value = match entry.tag_id {
                RICOH_SHOOTING_MODE | RICOH_FLASH_MODE | RICOH_WHITE_BALANCE => {
                    TAG_REGISTRY.decode_u16(entry.tag_id, value)
                }
                RICOH_FOCUS_MODE => {
                    let mode = if value == 0 { "Auto" } else { "Manual" };
                    mode.to_string()
                }
                RICOH_ISO_SETTING | RICOH_SHARPNESS => value.to_string(),
                _ => return,
            };

            tags.insert(format!("Ricoh:{}", tag_name), formatted_value);
        }
    }
}

impl MakerNoteParser for RicohParser {
    fn manufacturer_name(&self) -> &'static str {
        "Ricoh"
    }

    fn tag_prefix(&self) -> &'static str {
        "Ricoh:"
    }

    fn parse(
        &self,
        data: &[u8],
        byte_order: ByteOrder,
        tags: &mut HashMap<String, String>,
    ) -> Result<(), String> {
        let config = IfdParserConfig {
            signature: None,
            signature_offset: 0,
            max_entries: 500,
        };

        parse_ifd_entries(data, byte_order, &config, |entry, parse_data| {
            self.parse_entry(entry, parse_data, byte_order, tags);
        })
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shooting_mode_decoder() {
        assert_eq!(SHOOTING_MODE.decode(0), "Auto");
        assert_eq!(SHOOTING_MODE.decode(1), "Program");
    }

    #[test]
    fn test_ricoh_parser_trait() {
        let parser = RicohParser::new();
        assert_eq!(parser.manufacturer_name(), "Ricoh");
        assert_eq!(parser.tag_prefix(), "Ricoh:");
    }

    #[test]
    fn test_parse_shooting_mode() {
        let parser = RicohParser::new();
        let mut data = Vec::new();
        data.extend_from_slice(&[0x01, 0x00]);
        data.extend_from_slice(&[0x05, 0x00]);
        data.extend_from_slice(&[0x03, 0x00]);
        data.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);
        data.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);

        let mut tags = HashMap::new();
        let result = parser.parse(&data, ByteOrder::LittleEndian, &mut tags);
        assert!(result.is_ok());
        assert_eq!(tags.get("Ricoh:ShootingMode"), Some(&"Program".to_string()));
    }

    #[test]
    fn test_tag_registry() {
        assert_eq!(
            TAG_REGISTRY.get_tag_name(RICOH_SHOOTING_MODE),
            Some("ShootingMode")
        );
        assert!(TAG_REGISTRY.has_tag(RICOH_FLASH_MODE));
    }
}
