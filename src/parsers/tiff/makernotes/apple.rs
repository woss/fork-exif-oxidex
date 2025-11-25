//! Apple (iPhone/iPad) MakerNote parser
//!
//! Parses Apple-specific EXIF MakerNote tags containing computational photography
//! settings, multi-camera data, and iOS-specific metadata.
//!
//! ## Supported Features
//! - HDR processing mode
//! - Portrait Mode and depth data
//! - Live Photo status
//! - Scene detection
//! - Multi-camera lens identification
//! - Semantic Styles (Photographic Styles)
//! - Smart HDR version
//! - Night Mode
//!
//! ## Architecture
//! Apple's MakerNotes use a proprietary binary format with Apple-specific tags.
//! Unlike traditional camera manufacturers, Apple stores significant computational
//! photography metadata.

#![allow(dead_code)]
#![allow(unused_imports)]

use crate::const_decoder;
use crate::parsers::tiff::ifd_parser::{ByteOrder, IfdEntry};
use crate::parsers::tiff::makernotes::shared::ifd_parser_base::{
    parse_ifd_entries, IfdParserConfig,
};
use crate::parsers::tiff::makernotes::shared::value_extractors::{
    extract_i16_value, extract_string_with_byteorder, extract_u32_value,
};
use std::collections::HashMap;

use super::shared::array_extractors::extract_i16_array;
use super::shared::MakerNoteParser;

// Apple MakerNote Tag IDs
// Note: Apple's tag structure is proprietary and reverse-engineered
const APPLE_HDR_IMAGE_TYPE: u16 = 0x000A; // HDR processing mode (3=HDR, 4=HDR+)
const APPLE_BURST_UUID: u16 = 0x000B; // Burst mode unique ID
const APPLE_CONTENT_IDENTIFIER: u16 = 0x0011; // Media identifier
const APPLE_IMAGE_UNIQUE_ID: u16 = 0x0015; // Unique image ID
const APPLE_LIVE_PHOTO_ID: u16 = 0x0017; // Live Photo video identifier
const APPLE_RUN_TIME: u16 = 0x001A; // Runtime flags
const APPLE_ACCELERATION_VECTOR: u16 = 0x001B; // Device orientation data
const APPLE_PORTRAIT_DATA: u16 = 0x0020; // Portrait Mode information
const APPLE_FOCUS_DISTANCE_RANGE: u16 = 0x002B; // Focus distance range
const APPLE_SEMANTIC_STYLE: u16 = 0x002E; // Photographic Style setting
const APPLE_FRONT_FACING_CAMERA: u16 = 0x0032; // Front/back camera flag
const APPLE_LENS_MODEL: u16 = 0x0035; // Multi-camera lens identifier
const APPLE_SMART_HDR_VERSION: u16 = 0x0037; // Smart HDR version
const APPLE_NIGHT_MODE: u16 = 0x0039; // Night Mode status
const APPLE_SCENE_DETECTION: u16 = 0x003C; // Scene detection result

// Apple signature (not always present, but useful for validation)
const APPLE_SIGNATURE: &[u8] = b"Apple iOS";

// Decodes Apple HDR image type
// Public to allow re-use in registry module
const_decoder! {
    pub DECODE_HDR_TYPE, i16, [
        (0, "Off"),
        (1, "HDR"),
        (3, "Auto HDR"),
        (4, "Smart HDR"),
        (5, "Smart HDR 2"),
        (6, "Smart HDR 3"),
        (7, "Smart HDR 4"),
        (8, "Smart HDR 5"),
    ]
}

// Decodes Portrait Mode effect type
// Public to allow re-use in registry module
const_decoder! {
    pub DECODE_PORTRAIT_MODE, i16, [
        (0, "Off"),
        (1, "Natural Light"),
        (2, "Studio Light"),
        (3, "Contour Light"),
        (4, "Stage Light"),
        (5, "Stage Light Mono"),
        (6, "High-Key Light Mono"),
    ]
}

// Decodes scene detection type
// Public to allow re-use in registry module
const_decoder! {
    pub DECODE_SCENE_TYPE, i16, [
        (0, "None"),
        (1, "Sunset/Sunrise"),
        (2, "Blue Sky"),
        (3, "Snow"),
        (4, "Foliage"),
        (5, "Beach"),
        (6, "Night"),
        (7, "Fireworks"),
        (8, "Food"),
        (9, "Pet"),
        (10, "Document"),
    ]
}

// Decodes semantic style (Photographic Style)
// Public to allow re-use in registry module
const_decoder! {
    pub DECODE_SEMANTIC_STYLE, i16, [
        (0, "Standard"),
        (1, "Rich Contrast"),
        (2, "Vibrant"),
        (3, "Warm"),
        (4, "Cool"),
    ]
}

// Decodes lens model for multi-camera iPhones
// Public to allow re-use in registry module
const_decoder! {
    pub DECODE_LENS_MODEL, i16, [
        (0, "Wide (Main Camera)"),
        (1, "Telephoto"),
        (2, "Ultra Wide"),
        (3, "Front Camera"),
        (4, "Telephoto 2x"),
        (5, "Telephoto 3x"),
        (6, "Telephoto 5x"),
    ]
}

/// Apple MakerNote parser implementation
pub struct AppleParser;

impl Default for AppleParser {
    fn default() -> Self {
        Self::new()
    }
}

impl AppleParser {
    /// Creates a new Apple parser instance
    pub fn new() -> Self {
        AppleParser
    }

    /// Parse a single IFD entry and extract tag value using registry-based approach
    fn parse_entry(
        &self,
        entry: &IfdEntry,
        data: &[u8],
        byte_order: ByteOrder,
        tags: &mut HashMap<String, String>,
    ) {
        use super::registries::apple::{
            apple_registry, decode_facing_camera, decode_night_mode, format_runtime_flags,
        };

        let registry = apple_registry();
        let tag_id = entry.tag_id;

        // Handle tags based on type
        match tag_id {
            // i16 tags - use registry for decoding
            APPLE_HDR_IMAGE_TYPE
            | APPLE_PORTRAIT_DATA
            | APPLE_SEMANTIC_STYLE
            | APPLE_SCENE_DETECTION
            | APPLE_LENS_MODEL
            | APPLE_SMART_HDR_VERSION
            | APPLE_FOCUS_DISTANCE_RANGE => {
                if let Some(value) = extract_i16_value(entry, data, byte_order) {
                    if let Some(tag_name) = registry.get_tag_name(tag_id) {
                        let decoded = registry.decode_i16(tag_id, value);
                        tags.insert(format!("Apple:{}", tag_name), decoded);
                    }
                }
            }
            // i16 tags with custom logic
            APPLE_FRONT_FACING_CAMERA => {
                if let Some(value) = extract_i16_value(entry, data, byte_order) {
                    tags.insert(
                        "Apple:FacingCamera".to_string(),
                        decode_facing_camera(value),
                    );
                }
            }
            APPLE_NIGHT_MODE => {
                if let Some(value) = extract_i16_value(entry, data, byte_order) {
                    tags.insert("Apple:NightMode".to_string(), decode_night_mode(value));
                }
            }
            // u32 tags with custom formatting
            APPLE_RUN_TIME => {
                if let Some(value) = extract_u32_value(entry, data, byte_order) {
                    tags.insert(
                        "Apple:RunTimeFlags".to_string(),
                        format_runtime_flags(value),
                    );
                }
            }
            // String tags
            APPLE_BURST_UUID | APPLE_CONTENT_IDENTIFIER | APPLE_IMAGE_UNIQUE_ID => {
                if let Some(string_value) = extract_string_with_byteorder(entry, data, byte_order) {
                    if let Some(tag_name) = registry.get_tag_name(tag_id) {
                        tags.insert(format!("Apple:{}", tag_name), string_value);
                    }
                }
            }
            // Special case: LivePhoto detection
            APPLE_LIVE_PHOTO_ID => {
                if let Some(id) = extract_string_with_byteorder(entry, data, byte_order) {
                    tags.insert("Apple:LivePhotoVideoID".to_string(), id);
                    // Additional flag to indicate this is a Live Photo
                    tags.insert("Apple:LivePhoto".to_string(), "Yes".to_string());
                }
            }
            // Other tags not in registry - skip silently
            _ => {}
        }
    }
}

impl MakerNoteParser for AppleParser {
    fn manufacturer_name(&self) -> &'static str {
        "Apple"
    }

    fn tag_prefix(&self) -> &'static str {
        "Apple:"
    }

    fn parse(
        &self,
        data: &[u8],
        byte_order: ByteOrder,
        tags: &mut HashMap<String, String>,
    ) -> Result<(), String> {
        let config = IfdParserConfig {
            signature: Some(APPLE_SIGNATURE),
            signature_offset: 10, // "Apple iOS" (9) + 1 padding byte = 10
            max_entries: 500,
        };

        parse_ifd_entries(data, byte_order, &config, |entry, _ifd_data| {
            // Pass full data buffer to parse_entry as it expects absolute offsets
            self.parse_entry(entry, data, byte_order, tags);
        })
    }

    fn validate_header(&self, data: &[u8]) -> bool {
        // Accept data with or without Apple signature
        // Many Apple MakerNotes don't have a consistent header
        if data.len() >= 9 && &data[0..9] == APPLE_SIGNATURE {
            return true;
        }

        // Also accept if it looks like valid IFD data
        if data.len() >= 2 {
            let entry_count = u16::from_le_bytes([data[0], data[1]]);
            if entry_count > 0 && entry_count < 500 {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_hdr_type() {
        assert_eq!(DECODE_HDR_TYPE.decode(0), "Off");
        assert_eq!(DECODE_HDR_TYPE.decode(1), "HDR");
        assert_eq!(DECODE_HDR_TYPE.decode(4), "Smart HDR");
        assert_eq!(DECODE_HDR_TYPE.decode(8), "Smart HDR 5");
    }

    #[test]
    fn test_decode_portrait_mode() {
        assert_eq!(DECODE_PORTRAIT_MODE.decode(0), "Off");
        assert_eq!(DECODE_PORTRAIT_MODE.decode(1), "Natural Light");
        assert_eq!(DECODE_PORTRAIT_MODE.decode(4), "Stage Light");
    }

    #[test]
    fn test_decode_scene_type() {
        assert_eq!(DECODE_SCENE_TYPE.decode(0), "None");
        assert_eq!(DECODE_SCENE_TYPE.decode(6), "Night");
        assert_eq!(DECODE_SCENE_TYPE.decode(8), "Food");
    }

    #[test]
    fn test_decode_semantic_style() {
        assert_eq!(DECODE_SEMANTIC_STYLE.decode(0), "Standard");
        assert_eq!(DECODE_SEMANTIC_STYLE.decode(2), "Vibrant");
    }

    #[test]
    fn test_decode_lens_model() {
        assert_eq!(DECODE_LENS_MODEL.decode(0), "Wide (Main Camera)");
        assert_eq!(DECODE_LENS_MODEL.decode(1), "Telephoto");
        assert_eq!(DECODE_LENS_MODEL.decode(2), "Ultra Wide");
        assert_eq!(DECODE_LENS_MODEL.decode(6), "Telephoto 5x");
    }

    #[test]
    fn test_apple_parser_trait() {
        let parser = AppleParser::new();
        assert_eq!(parser.manufacturer_name(), "Apple");
        assert_eq!(parser.tag_prefix(), "Apple:");
    }

    #[test]
    fn test_validate_header_with_signature() {
        let parser = AppleParser::new();
        let mut data = Vec::new();
        data.extend_from_slice(b"Apple iOS");
        data.extend_from_slice(&[0x05, 0x00]); // 5 entries

        assert!(parser.validate_header(&data));
    }

    #[test]
    fn test_validate_header_without_signature() {
        let parser = AppleParser::new();
        let data = vec![0x05, 0x00]; // Just entry count

        assert!(parser.validate_header(&data));
    }

    #[test]
    fn test_parse_hdr_tag() {
        let parser = AppleParser::new();
        let mut data = Vec::new();

        // Create minimal IFD with one entry
        data.extend_from_slice(&[0x01, 0x00]); // 1 entry

        // HDR tag entry (tag=0x000A, type=3 (SHORT), count=1, value=4 (Smart HDR))
        data.extend_from_slice(&[0x0A, 0x00]); // Tag
        data.extend_from_slice(&[0x03, 0x00]); // Type: SHORT
        data.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // Count: 1
        data.extend_from_slice(&[0x04, 0x00, 0x00, 0x00]); // Value: 4 (inline)

        let mut tags = HashMap::new();
        let result = parser.parse(&data, ByteOrder::LittleEndian, &mut tags);

        assert!(result.is_ok());
        assert_eq!(
            tags.get("Apple:HDRImageType"),
            Some(&"Smart HDR".to_string())
        );
    }
}
