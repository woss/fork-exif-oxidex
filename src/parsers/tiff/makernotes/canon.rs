//! Canon MakerNote parser
//!
//! Parses Canon-specific EXIF MakerNote tags containing camera settings,
//! lens information, focus data, and other proprietary metadata.

#![allow(dead_code)]
#![allow(unused_imports)]

use crate::error::{ExifToolError, Result};
use crate::parsers::tiff::ifd_parser::{ByteOrder, IfdEntry};
use crate::parsers::tiff::makernotes::shared::ifd_parser_base::{
    parse_ifd_entries, IfdParserConfig,
};
use nom::{
    combinator::map,
    multi::count,
    number::complete::{be_u16, be_u32, le_u16, le_u32},
    IResult,
};
use std::collections::HashMap;

use super::canon_lens_database::lookup_lens_name;
use super::shared::array_extractors::extract_i16_array;
use super::shared::value_extractors::{
    extract_inline_value, extract_integer_value, extract_string_value,
};
use super::shared::MakerNoteParser;
use crate::const_decoder;

// Canon MakerNote Tag IDs
const CANON_CAMERA_SETTINGS: u16 = 0x0001;
const CANON_FOCAL_LENGTH: u16 = 0x0002;
const CANON_SHOT_INFO: u16 = 0x0004;
const CANON_PANORAMA: u16 = 0x0005;
const CANON_IMAGE_TYPE: u16 = 0x0006;
const CANON_FIRMWARE_VERSION: u16 = 0x0007;
const CANON_FILE_NUMBER: u16 = 0x0008;
const CANON_OWNER_NAME: u16 = 0x0009;
const CANON_SERIAL_NUMBER: u16 = 0x000C;
const CANON_CAMERA_INFO: u16 = 0x000D;
const CANON_CUSTOM_FUNCTIONS: u16 = 0x000F;
const CANON_MODEL_ID: u16 = 0x0010;
const CANON_AF_INFO: u16 = 0x0012;
const CANON_AF_INFO2: u16 = 0x0026;
const CANON_FILE_INFO: u16 = 0x0093;
const CANON_LENS_MODEL: u16 = 0x0095;

// Canon signature (not always present)
const CANON_SIGNATURE: &[u8] = b"Canon";

// CameraSettings array (tag 0x0001) indices
// Array contains ~50 values with camera settings
// Reference: ExifTool Canon.pm CameraSettings table
const CAMERA_SETTINGS_MACRO_MODE: usize = 1;
const CAMERA_SETTINGS_SELF_TIMER: usize = 2;
const CAMERA_SETTINGS_QUALITY: usize = 3;
const CAMERA_SETTINGS_FLASH_MODE: usize = 4;
const CAMERA_SETTINGS_DRIVE_MODE: usize = 5;
const CAMERA_SETTINGS_FOCUS_MODE: usize = 7;
const CAMERA_SETTINGS_IMAGE_SIZE: usize = 10;
const CAMERA_SETTINGS_EASY_MODE: usize = 11;
const CAMERA_SETTINGS_CONTRAST: usize = 13;
const CAMERA_SETTINGS_SATURATION: usize = 14;
const CAMERA_SETTINGS_SHARPNESS: usize = 15;
const CAMERA_SETTINGS_ISO: usize = 16;
const CAMERA_SETTINGS_METERING_MODE: usize = 17;
const CAMERA_SETTINGS_FOCUS_TYPE: usize = 18;
const CAMERA_SETTINGS_AF_POINT: usize = 19;
const CAMERA_SETTINGS_EXPOSURE_MODE: usize = 20;
const CAMERA_SETTINGS_FLASH_ACTIVITY: usize = 28;
const CAMERA_SETTINGS_FOCUS_CONTINUOUS: usize = 32;

// ShotInfo array (tag 0x0004) indices
const SHOT_INFO_AUTO_ISO: usize = 1;
const SHOT_INFO_BASE_ISO: usize = 2;
const SHOT_INFO_MEASURED_EV: usize = 3;
const SHOT_INFO_TARGET_APERTURE: usize = 4;
const SHOT_INFO_TARGET_SHUTTER_SPEED: usize = 5;
const SHOT_INFO_WHITE_BALANCE: usize = 7;
const SHOT_INFO_SLOW_SHUTTER: usize = 8;
const SHOT_INFO_SEQUENCE_NUMBER: usize = 9;
const SHOT_INFO_FLASH_GUIDE_NUMBER: usize = 13;
const SHOT_INFO_AF_POINTS_USED: usize = 14;
const SHOT_INFO_FLASH_EXPOSURE_COMP: usize = 15;
const SHOT_INFO_AUTO_EXPOSURE_BRACKETING: usize = 16;
const SHOT_INFO_SUBJECT_DISTANCE: usize = 19;

// FileInfo array indices (tag 0x0093)
const FILE_INFO_FILE_NUMBER: usize = 1;
const FILE_INFO_SHUTTER_COUNT_LOW: usize = 2;
const FILE_INFO_SHUTTER_COUNT_HIGH: usize = 3;
const FILE_INFO_BRACKET_MODE: usize = 4;
const FILE_INFO_BRACKET_VALUE: usize = 5;
const FILE_INFO_LENS_ID: usize = 6;

// AFInfo array indices
const AF_INFO_NUM_AF_POINTS: usize = 1;
const AF_INFO_IMAGE_WIDTH: usize = 2;
const AF_INFO_IMAGE_HEIGHT: usize = 3;
const AF_INFO_AREA_WIDTH: usize = 4;
const AF_INFO_AREA_HEIGHT: usize = 5;
const AF_INFO_POINTS_IN_FOCUS: usize = 8;
const AF_INFO_POINTS_SELECTED: usize = 9;

// ============================================================================
// DECODERS - Canon Value Decoders
// ============================================================================
// Using const_decoder! macro for declarative, zero-overhead value decoding

// Canon macro mode decoder
// Public to allow re-use in registry module
const_decoder!(pub MACRO_MODE, i16, [(1, "Macro"), (2, "Normal"),]);

// Canon quality setting decoder
// Public to allow re-use in registry module
const_decoder!(
    pub QUALITY,
    i16,
    [
        (-1, "n/a"),
        (1, "Economy"),
        (2, "Normal"),
        (3, "Fine"),
        (4, "RAW"),
        (5, "Superfine"),
        (7, "CRAW"),
        (130, "Normal Movie"),
        (131, "Movie (2)"),
        (132, "Movie (3)"),
        (133, "Movie (4)"),
    ]
);

// Canon flash mode decoder
// Public to allow re-use in registry module
const_decoder!(
    pub FLASH_MODE,
    i16,
    [
        (0, "Off"),
        (1, "Auto"),
        (2, "On"),
        (3, "Red-eye Reduction"),
        (4, "Slow Sync"),
        (5, "Auto + Red-eye Reduction"),
        (6, "On + Red-eye Reduction"),
        (16, "External Flash"),
    ]
);

// Canon drive mode decoder
// Public to allow re-use in registry module
const_decoder!(
    pub DRIVE_MODE,
    i16,
    [
        (0, "Single"),
        (1, "Continuous"),
        (2, "Movie"),
        (4, "Continuous, Speed Priority"),
        (5, "Continuous, Low"),
        (6, "Continuous, High"),
    ]
);

// Canon focus mode decoder
// Public to allow re-use in registry module
const_decoder!(
    pub FOCUS_MODE,
    i16,
    [
        (0, "One-shot AF"),
        (1, "AI Servo AF"),
        (2, "AI Focus AF"),
        (3, "Manual Focus (3)"),
        (4, "Single"),
        (5, "Continuous"),
        (6, "Manual Focus (6)"),
        (16, "Pan Focus"),
    ]
);

// Canon metering mode decoder
// Public to allow re-use in registry module
const_decoder!(
    pub METERING_MODE,
    i16,
    [
        (3, "Evaluative"),
        (4, "Partial"),
        (5, "Center-weighted Average"),
    ]
);

// Canon exposure mode decoder
// Public to allow re-use in registry module
const_decoder!(
    pub EXPOSURE_MODE,
    i16,
    [
        (0, "Easy"),
        (1, "Program AE"),
        (2, "Shutter Priority"),
        (3, "Aperture Priority"),
        (4, "Manual"),
        (5, "Depth-of-field AE"),
        (6, "M-Dep"),
        (7, "Bulb"),
    ]
);

/// Represents a Canon MakerNote tag value
#[derive(Debug, Clone, PartialEq)]
pub enum CanonTagValue {
    /// Single integer value
    Integer(i32),
    /// String value (model name, firmware, etc.)
    String(String),
    /// Array of integers (camera settings, shot info)
    IntArray(Vec<i16>),
}

/// Maps Canon MakerNote tag IDs to human-readable tag names.
///
/// # Parameters
/// - `tag_id`: The Canon-specific tag ID
///
/// # Returns
/// Tag name in the format "Canon:TagName"
///
/// # Example
/// ```
/// use oxidex::parsers::tiff::makernotes::canon::canon_tag_to_name;
/// assert_eq!(canon_tag_to_name(0x0001), "Canon:CameraSettings");
/// ```
pub fn canon_tag_to_name(tag_id: u16) -> String {
    let tag_name = match tag_id {
        CANON_CAMERA_SETTINGS => "CameraSettings",
        CANON_FOCAL_LENGTH => "FocalLength",
        CANON_SHOT_INFO => "ShotInfo",
        CANON_PANORAMA => "Panorama",
        CANON_IMAGE_TYPE => "ImageType",
        CANON_FIRMWARE_VERSION => "FirmwareVersion",
        CANON_FILE_NUMBER => "FileNumber",
        CANON_OWNER_NAME => "OwnerName",
        CANON_SERIAL_NUMBER => "SerialNumber",
        CANON_CAMERA_INFO => "CameraInfo",
        CANON_CUSTOM_FUNCTIONS => "CustomFunctions",
        CANON_MODEL_ID => "CanonModelID",
        _ => return format!("Canon:Unknown-{:#06X}", tag_id),
    };

    format!("Canon:{}", tag_name)
}

/// Represents a Canon MakerNote parser
pub struct CanonParser;

impl MakerNoteParser for CanonParser {
    fn manufacturer_name(&self) -> &'static str {
        "Canon"
    }

    fn tag_prefix(&self) -> &'static str {
        "Canon:"
    }

    fn validate_header(&self, data: &[u8]) -> bool {
        is_canon_makernote(data)
    }

    fn parse(
        &self,
        data: &[u8],
        byte_order: ByteOrder,
        tags: &mut HashMap<String, String>,
    ) -> std::result::Result<(), String> {
        // Call the existing parse_canon_makernote function and handle Result conversion
        match parse_canon_makernote_impl(data, byte_order) {
            Ok(parsed_tags) => {
                tags.extend(parsed_tags);
                Ok(())
            }
            Err(e) => Err(format!("Canon MakerNote parse error: {}", e)),
        }
    }

    fn lookup_lens(&self, lens_id: u16) -> Option<String> {
        lookup_lens_name(lens_id)
    }
}

/// Checks if data appears to be a Canon MakerNote.
///
/// Canon MakerNotes may optionally start with "Canon" signature,
/// but always contain a valid IFD structure.
///
/// # Parameters
/// - `data`: Raw byte data to check
///
/// # Returns
/// `true` if the data appears to be a Canon MakerNote, `false` otherwise
pub fn is_canon_makernote(data: &[u8]) -> bool {
    if data.len() < 4 {
        return false;
    }

    // Check for optional Canon signature
    if data.starts_with(CANON_SIGNATURE) {
        return true;
    }

    // Check if it looks like an IFD (starts with entry count)
    // Valid IFD has at least 2 bytes for entry count
    // Try both little-endian and big-endian interpretations
    if data.len() >= 2 {
        let entry_count_le = u16::from_le_bytes([data[0], data[1]]);
        let entry_count_be = u16::from_be_bytes([data[0], data[1]]);

        // Reasonable entry count (Canon typically has 1-100 entries)
        // Accept if either byte order yields a reasonable count
        let is_reasonable = |count: u16| count > 0 && count < 200;

        return is_reasonable(entry_count_le) || is_reasonable(entry_count_be);
    }

    false
}

/// Internal implementation of Canon MakerNote parsing.
///
/// This parser extracts tags from Canon MakerNotes including simple tags
/// (strings and integers) and complex array tags (CameraSettings, ShotInfo, etc.).
///
/// # Parameters
/// - `data`: Raw MakerNote data (may include Canon signature)
/// - `byte_order`: Byte order for parsing (usually matches TIFF header)
///
/// # Returns
/// HashMap of tag names to string values
///
/// # Errors
/// Returns error if IFD parsing fails or data is invalid
fn parse_canon_makernote_impl(
    data: &[u8],
    byte_order: ByteOrder,
) -> Result<HashMap<String, String>> {
    if data.is_empty() {
        return Ok(HashMap::new());
    }

    let mut tags = HashMap::new();

    let config = IfdParserConfig {
        signature: Some(CANON_SIGNATURE),
        signature_offset: CANON_SIGNATURE.len(),
        max_entries: 200,
    };

    // Use shared IFD parser
    // Note: we don't propagate errors here to maintain existing behavior of
    // returning whatever tags we found even if parsing isn't perfect
    let _ = parse_ifd_entries(data, byte_order, &config, |entry, ifd_data| {
        match entry.tag_id {
            // Simple string tags (Phase 1)
            // These tags seem to use offsets relative to the IFD start
            CANON_IMAGE_TYPE | CANON_FIRMWARE_VERSION | CANON_OWNER_NAME | CANON_SERIAL_NUMBER => {
                if let Some(value) = extract_string_value(entry, ifd_data) {
                    let tag_name = canon_tag_to_name(entry.tag_id);
                    tags.insert(tag_name, value);
                }
            }

            // Simple integer tags (Phase 1)
            CANON_MODEL_ID | CANON_FILE_NUMBER => {
                if let Some(value) = extract_integer_value(entry) {
                    let tag_name = canon_tag_to_name(entry.tag_id);
                    tags.insert(tag_name, value);
                }
            }

            // CameraSettings array (Phase 2)
            CANON_CAMERA_SETTINGS => {
                if let Some(array) = extract_i16_array(entry, data, byte_order) {
                    // Extract specific settings from array using const decoders
                    if array.len() > CAMERA_SETTINGS_MACRO_MODE {
                        tags.insert(
                            "Canon:MacroMode".to_string(),
                            MACRO_MODE.decode(array[CAMERA_SETTINGS_MACRO_MODE]),
                        );
                    }
                    if array.len() > CAMERA_SETTINGS_QUALITY {
                        tags.insert(
                            "Canon:Quality".to_string(),
                            QUALITY.decode(array[CAMERA_SETTINGS_QUALITY]),
                        );
                    }
                    if array.len() > CAMERA_SETTINGS_FLASH_MODE {
                        tags.insert(
                            "Canon:FlashMode".to_string(),
                            FLASH_MODE.decode(array[CAMERA_SETTINGS_FLASH_MODE]),
                        );
                    }
                    if array.len() > CAMERA_SETTINGS_DRIVE_MODE {
                        tags.insert(
                            "Canon:DriveMode".to_string(),
                            DRIVE_MODE.decode(array[CAMERA_SETTINGS_DRIVE_MODE]),
                        );
                    }
                    if array.len() > CAMERA_SETTINGS_FOCUS_MODE {
                        tags.insert(
                            "Canon:FocusMode".to_string(),
                            FOCUS_MODE.decode(array[CAMERA_SETTINGS_FOCUS_MODE]),
                        );
                    }
                    if array.len() > CAMERA_SETTINGS_ISO {
                        tags.insert(
                            "Canon:ISO".to_string(),
                            array[CAMERA_SETTINGS_ISO].to_string(),
                        );
                    }
                    if array.len() > CAMERA_SETTINGS_METERING_MODE {
                        tags.insert(
                            "Canon:MeteringMode".to_string(),
                            METERING_MODE.decode(array[CAMERA_SETTINGS_METERING_MODE]),
                        );
                    }
                    if array.len() > CAMERA_SETTINGS_EXPOSURE_MODE {
                        tags.insert(
                            "Canon:ExposureMode".to_string(),
                            EXPOSURE_MODE.decode(array[CAMERA_SETTINGS_EXPOSURE_MODE]),
                        );
                    }
                }
            }

            // ShotInfo array (Phase 2)
            CANON_SHOT_INFO => {
                if let Some(array) = extract_i16_array(entry, data, byte_order) {
                    if array.len() > SHOT_INFO_AUTO_ISO {
                        tags.insert(
                            "Canon:AutoISO".to_string(),
                            array[SHOT_INFO_AUTO_ISO].to_string(),
                        );
                    }
                    if array.len() > SHOT_INFO_BASE_ISO {
                        tags.insert(
                            "Canon:BaseISO".to_string(),
                            array[SHOT_INFO_BASE_ISO].to_string(),
                        );
                    }
                    if array.len() > SHOT_INFO_MEASURED_EV {
                        tags.insert(
                            "Canon:MeasuredEV".to_string(),
                            array[SHOT_INFO_MEASURED_EV].to_string(),
                        );
                    }
                    if array.len() > SHOT_INFO_TARGET_APERTURE {
                        tags.insert(
                            "Canon:TargetAperture".to_string(),
                            array[SHOT_INFO_TARGET_APERTURE].to_string(),
                        );
                    }
                    if array.len() > SHOT_INFO_TARGET_SHUTTER_SPEED {
                        tags.insert(
                            "Canon:TargetShutterSpeed".to_string(),
                            array[SHOT_INFO_TARGET_SHUTTER_SPEED].to_string(),
                        );
                    }
                    if array.len() > SHOT_INFO_SUBJECT_DISTANCE {
                        let distance = array[SHOT_INFO_SUBJECT_DISTANCE];
                        tags.insert(
                            "Canon:SubjectDistance".to_string(),
                            format!("{} mm", distance),
                        );
                    }
                }
            }

            // FocalLength array (Phase 2)
            CANON_FOCAL_LENGTH => {
                if let Some(array) = extract_i16_array(entry, data, byte_order) {
                    // array[0] = focal type
                    // array[1] = focal length
                    if !array.is_empty() {
                        tags.insert("Canon:FocalType".to_string(), array[0].to_string());
                    }
                    if array.len() > 1 {
                        tags.insert("Canon:FocalLength".to_string(), format!("{} mm", array[1]));
                    }
                }
            }

            // LensModel tag (Phase 3) - ASCII string containing lens name
            CANON_LENS_MODEL => {
                // LensModel is an ASCII string tag
                if entry.field_type == 2 {
                    // ASCII type
                    let value_bytes = if entry.value_count <= 4 {
                        // Inline value
                        extract_inline_value(
                            entry.value_offset,
                            entry.value_count as usize,
                            byte_order,
                        )
                    } else {
                        // External value
                        if (entry.value_offset as usize) < data.len() {
                            let end = std::cmp::min(
                                (entry.value_offset as usize) + (entry.value_count as usize),
                                data.len(),
                            );
                            data[entry.value_offset as usize..end].to_vec()
                        } else {
                            Vec::new()
                        }
                    };

                    if !value_bytes.is_empty() {
                        let lens_model = String::from_utf8_lossy(&value_bytes)
                            .trim_end_matches('\0')
                            .to_string();
                        if !lens_model.is_empty() {
                            tags.insert("Canon:LensModel".to_string(), lens_model);
                        }
                    }
                }
            }

            // FileInfo array (Phase 3) - contains lens ID and shutter count
            CANON_FILE_INFO => {
                // FileInfo is a SHORT array
                if let Some(array) = extract_i16_array(entry, data, byte_order) {
                    // Extract lens ID (index 6)
                    if let Some(&lens_id) = array.get(FILE_INFO_LENS_ID) {
                        if lens_id > 0 {
                            // Look up lens name from database
                            if let Some(lens_name) = lookup_lens_name(lens_id as u16) {
                                tags.insert("Canon:LensType".to_string(), lens_name);
                            } else {
                                // Unknown lens - store ID
                                tags.insert("Canon:LensID".to_string(), lens_id.to_string());
                            }
                        }
                    }

                    // Extract shutter count (combine low and high words)
                    if let (Some(&low), Some(&high)) = (
                        array.get(FILE_INFO_SHUTTER_COUNT_LOW),
                        array.get(FILE_INFO_SHUTTER_COUNT_HIGH),
                    ) {
                        let shutter_count = ((high as u32) << 16) | (low as u32 & 0xFFFF);
                        if shutter_count > 0 {
                            tags.insert(
                                "Canon:ShutterCount".to_string(),
                                shutter_count.to_string(),
                            );
                        }
                    }
                }
            }

            // AFInfo array (Phase 3) - autofocus point information
            CANON_AF_INFO | CANON_AF_INFO2 => {
                // AFInfo is a SHORT array
                if let Some(array) = extract_i16_array(entry, data, byte_order) {
                    // Number of AF points
                    if let Some(&num_points) = array.get(AF_INFO_NUM_AF_POINTS) {
                        if num_points > 0 {
                            tags.insert("Canon:NumAFPoints".to_string(), num_points.to_string());
                        }
                    }

                    // AF area dimensions
                    if let Some(&width) = array.get(AF_INFO_IMAGE_WIDTH) {
                        if width > 0 {
                            tags.insert("Canon:AFImageWidth".to_string(), width.to_string());
                        }
                    }
                    if let Some(&height) = array.get(AF_INFO_IMAGE_HEIGHT) {
                        if height > 0 {
                            tags.insert("Canon:AFImageHeight".to_string(), height.to_string());
                        }
                    }

                    // AF points in focus (bitmask)
                    if let Some(&points_in_focus) = array.get(AF_INFO_POINTS_IN_FOCUS) {
                        tags.insert(
                            "Canon:AFPointsInFocus".to_string(),
                            points_in_focus.to_string(),
                        );
                    }

                    // AF points selected (bitmask)
                    if let Some(&points_selected) = array.get(AF_INFO_POINTS_SELECTED) {
                        tags.insert(
                            "Canon:AFPointsSelected".to_string(),
                            points_selected.to_string(),
                        );
                    }
                }
            }

            // Other array tags - skip for now (will add in future phases)
            _ => {}
        }
    });

    Ok(tags)
}

/// Parses Canon MakerNote data into a map of tag names to values.
///
/// This is the public API that delegates to the CanonParser trait implementation.
///
/// # Parameters
/// - `data`: Raw MakerNote data (may include Canon signature)
/// - `byte_order`: Byte order for parsing (usually matches TIFF header)
/// - `tags`: Mutable reference to HashMap to populate with extracted tags
///
/// # Example
/// ```ignore
/// use std::collections::HashMap;
/// use oxidex::parsers::tiff::ifd_parser::ByteOrder;
///
/// let mut tags = HashMap::new();
/// parse_canon_makernotes(&data, ByteOrder::LittleEndian, &mut tags);
/// ```
pub fn parse_canon_makernotes(
    data: &[u8],
    byte_order: ByteOrder,
    tags: &mut HashMap<String, String>,
) {
    let parser = CanonParser;
    if let Err(e) = parser.parse(data, byte_order, tags) {
        eprintln!("Canon MakerNotes parse error: {}", e);
    }
}

/// Extracts inline value bytes from the value_offset field.
///
/// For values that fit in 4 bytes or less, they are stored directly
/// in the value_offset field rather than at an external offset.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canon_tag_ids() {
        assert_eq!(CANON_CAMERA_SETTINGS, 0x0001);
        assert_eq!(CANON_FOCAL_LENGTH, 0x0002);
        assert_eq!(CANON_SHOT_INFO, 0x0004);
        assert_eq!(CANON_MODEL_ID, 0x0010);
    }

    #[test]
    fn test_canon_signature() {
        assert_eq!(CANON_SIGNATURE, b"Canon");
    }

    #[test]
    fn test_canon_tag_to_name() {
        assert_eq!(canon_tag_to_name(0x0001), "Canon:CameraSettings");
        assert_eq!(canon_tag_to_name(0x0002), "Canon:FocalLength");
        assert_eq!(canon_tag_to_name(0x0004), "Canon:ShotInfo");
        assert_eq!(canon_tag_to_name(0x0006), "Canon:ImageType");
        assert_eq!(canon_tag_to_name(0x0007), "Canon:FirmwareVersion");
        assert_eq!(canon_tag_to_name(0x0010), "Canon:CanonModelID");

        // Unknown tag
        assert_eq!(canon_tag_to_name(0xFFFF), "Canon:Unknown-0xFFFF");
    }

    #[test]
    fn test_is_canon_makernote() {
        // With Canon signature
        let data_with_sig = b"Canon\x00\x01\x00\x02\x00";
        assert!(is_canon_makernote(data_with_sig));

        // Without signature (starts with IFD)
        let data_without_sig = b"\x00\x01\x00\x02\x00";
        assert!(is_canon_makernote(data_without_sig));

        // Invalid data
        let invalid_data = b"Nikon";
        assert!(!is_canon_makernote(invalid_data));
    }

    #[test]
    fn test_parse_canon_makernote_basic() {
        // Create minimal Canon MakerNote with signature
        let mut data = Vec::new();

        // Canon signature (optional)
        data.extend_from_slice(b"Canon");

        // Simple IFD with one entry (little-endian format)
        data.extend_from_slice(&[
            0x01, 0x00, // Number of entries: 1 (little-endian)
            // Entry 1: ImageType (0x0006)
            0x06, 0x00, // Tag ID: 0x0006 (little-endian)
            0x02, 0x00, // Type: 2 = ASCII string (little-endian)
            0x0B, 0x00, 0x00, 0x00, // Count: 11 bytes (little-endian)
            0x12, 0x00, 0x00, 0x00, // Offset to data: 0x12 (18 bytes from IFD start)
            // Next IFD offset
            0x00, 0x00, 0x00, 0x00,
            // String data at offset 0x12 from IFD start (= byte 23 from data start)
            b'I', b'M', b'G', b':', b'E', b'O', b'S', b' ', b'R', b'5', 0x00,
        ]);

        let result = parse_canon_makernote_impl(&data, ByteOrder::LittleEndian);
        assert!(result.is_ok());

        let tags = result.unwrap();
        assert!(!tags.is_empty());
        assert_eq!(tags.get("Canon:ImageType"), Some(&"IMG:EOS R5".to_string()));
    }

    #[test]
    fn test_extract_i16_array_inline() {
        // Test inline array (count * 2 <= 4 bytes)
        let entry = IfdEntry {
            tag_id: CANON_FOCAL_LENGTH,
            field_type: 3, // SHORT
            value_count: 2,
            value_offset: 0x0064_0032, // Two shorts: 50, 100 (little-endian)
        };

        let result = extract_i16_array(&entry, &[], ByteOrder::LittleEndian);
        assert_eq!(result, Some(vec![50, 100]));
    }

    #[test]
    fn test_extract_i16_array_offset() {
        // Test offset-based array (count * 2 > 4 bytes)
        let entry = IfdEntry {
            tag_id: CANON_CAMERA_SETTINGS,
            field_type: 3, // SHORT
            value_count: 4,
            value_offset: 0, // Offset to data
        };

        // Data at offset 0: [1, 2, 3, 4] as little-endian shorts
        let data = vec![
            0x01, 0x00, // 1
            0x02, 0x00, // 2
            0x03, 0x00, // 3
            0x04, 0x00, // 4
        ];

        let result = extract_i16_array(&entry, &data, ByteOrder::LittleEndian);
        assert_eq!(result, Some(vec![1, 2, 3, 4]));
    }

    #[test]
    fn test_extract_i16_array_big_endian() {
        let entry = IfdEntry {
            tag_id: CANON_CAMERA_SETTINGS,
            field_type: 3,
            value_count: 3, // Use 3 values to force offset-based reading (>4 bytes)
            value_offset: 0,
        };

        // Big-endian data: [256, 512, 768]
        let data = vec![
            0x01, 0x00, // 256 (big-endian)
            0x02, 0x00, // 512 (big-endian)
            0x03, 0x00, // 768 (big-endian)
        ];

        let result = extract_i16_array(&entry, &data, ByteOrder::BigEndian);
        assert_eq!(result, Some(vec![256, 512, 768]));
    }

    #[test]
    fn test_camera_settings_indices() {
        // Verify key CameraSettings array indices are defined correctly
        assert_eq!(CAMERA_SETTINGS_MACRO_MODE, 1);
        assert_eq!(CAMERA_SETTINGS_SELF_TIMER, 2);
        assert_eq!(CAMERA_SETTINGS_QUALITY, 3);
        assert_eq!(CAMERA_SETTINGS_FLASH_MODE, 4);
        assert_eq!(CAMERA_SETTINGS_DRIVE_MODE, 5);
        assert_eq!(CAMERA_SETTINGS_FOCUS_MODE, 7);
        assert_eq!(CAMERA_SETTINGS_IMAGE_SIZE, 10);
        assert_eq!(CAMERA_SETTINGS_EASY_MODE, 11);
        assert_eq!(CAMERA_SETTINGS_CONTRAST, 13);
        assert_eq!(CAMERA_SETTINGS_SATURATION, 14);
        assert_eq!(CAMERA_SETTINGS_SHARPNESS, 15);
        assert_eq!(CAMERA_SETTINGS_ISO, 16);
        assert_eq!(CAMERA_SETTINGS_METERING_MODE, 17);
        assert_eq!(CAMERA_SETTINGS_FOCUS_TYPE, 18);
        assert_eq!(CAMERA_SETTINGS_AF_POINT, 19);
        assert_eq!(CAMERA_SETTINGS_EXPOSURE_MODE, 20);
        assert_eq!(CAMERA_SETTINGS_FLASH_ACTIVITY, 28);
        assert_eq!(CAMERA_SETTINGS_FOCUS_CONTINUOUS, 32);
    }

    #[test]
    fn test_decode_macro_mode() {
        assert_eq!(MACRO_MODE.decode(1), "Macro");
        assert_eq!(MACRO_MODE.decode(2), "Normal");
        assert_eq!(MACRO_MODE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_quality() {
        assert_eq!(QUALITY.decode(2), "Normal");
        assert_eq!(QUALITY.decode(3), "Fine");
        assert_eq!(QUALITY.decode(5), "Superfine");
        assert_eq!(QUALITY.decode(130), "Normal Movie");
        assert_eq!(QUALITY.decode(131), "Movie (2)");
        assert_eq!(QUALITY.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_flash_mode() {
        assert_eq!(FLASH_MODE.decode(0), "Off");
        assert_eq!(FLASH_MODE.decode(1), "Auto");
        assert_eq!(FLASH_MODE.decode(2), "On");
        assert_eq!(FLASH_MODE.decode(3), "Red-eye Reduction");
        assert_eq!(FLASH_MODE.decode(4), "Slow Sync");
        assert_eq!(FLASH_MODE.decode(5), "Auto + Red-eye Reduction");
        assert_eq!(FLASH_MODE.decode(6), "On + Red-eye Reduction");
        assert_eq!(FLASH_MODE.decode(16), "External Flash");
        assert_eq!(FLASH_MODE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_drive_mode() {
        assert_eq!(DRIVE_MODE.decode(0), "Single");
        assert_eq!(DRIVE_MODE.decode(1), "Continuous");
        assert_eq!(DRIVE_MODE.decode(2), "Movie");
        assert_eq!(DRIVE_MODE.decode(4), "Continuous, Speed Priority");
        assert_eq!(DRIVE_MODE.decode(5), "Continuous, Low");
        assert_eq!(DRIVE_MODE.decode(6), "Continuous, High");
        assert_eq!(DRIVE_MODE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_focus_mode() {
        assert_eq!(FOCUS_MODE.decode(0), "One-shot AF");
        assert_eq!(FOCUS_MODE.decode(1), "AI Servo AF");
        assert_eq!(FOCUS_MODE.decode(2), "AI Focus AF");
        assert_eq!(FOCUS_MODE.decode(3), "Manual Focus (3)");
        assert_eq!(FOCUS_MODE.decode(4), "Single");
        assert_eq!(FOCUS_MODE.decode(5), "Continuous");
        assert_eq!(FOCUS_MODE.decode(6), "Manual Focus (6)");
        assert_eq!(FOCUS_MODE.decode(16), "Pan Focus");
        assert_eq!(FOCUS_MODE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_metering_mode() {
        assert_eq!(METERING_MODE.decode(3), "Evaluative");
        assert_eq!(METERING_MODE.decode(4), "Partial");
        assert_eq!(METERING_MODE.decode(5), "Center-weighted Average");
        assert_eq!(METERING_MODE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_exposure_mode() {
        assert_eq!(EXPOSURE_MODE.decode(0), "Easy");
        assert_eq!(EXPOSURE_MODE.decode(1), "Program AE");
        assert_eq!(EXPOSURE_MODE.decode(2), "Shutter Priority");
        assert_eq!(EXPOSURE_MODE.decode(3), "Aperture Priority");
        assert_eq!(EXPOSURE_MODE.decode(4), "Manual");
        assert_eq!(EXPOSURE_MODE.decode(5), "Depth-of-field AE");
        assert_eq!(EXPOSURE_MODE.decode(6), "M-Dep");
        assert_eq!(EXPOSURE_MODE.decode(7), "Bulb");
        assert_eq!(EXPOSURE_MODE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_parse_camera_settings_array() {
        // Create Canon MakerNote with CameraSettings array
        let mut data = Vec::new();

        // Canon signature
        data.extend_from_slice(b"Canon");

        // IFD: 1 entry (CameraSettings)
        data.extend_from_slice(&[0x01, 0x00]); // Entry count (LE)

        // IFD Entry for CameraSettings (tag 0x0001)
        data.extend_from_slice(&[0x01, 0x00]); // Tag: CameraSettings
        data.extend_from_slice(&[0x03, 0x00]); // Type: SHORT
        data.extend_from_slice(&[0x15, 0x00, 0x00, 0x00]); // Count: 21 values
        data.extend_from_slice(&[0x17, 0x00, 0x00, 0x00]); // Offset: 23 (5 sig + 2 count + 12 entry + 4 next = 23)

        // Next IFD offset
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

        // CameraSettings array data at offset 20 (21 i16 values)
        let settings: Vec<i16> = vec![
            21, // [0] Array length
            2,  // [1] Macro mode: Normal
            0,  // [2] Self-timer: Off
            3,  // [3] Quality: Fine
            2,  // [4] Flash mode: On
            0,  // [5] Drive mode: Single
            0,  // [6] (unused)
            0,  // [7] Focus mode: One-shot AF
            0,  // [8] (unused)
            0,  // [9] (unused)
            1,  // [10] Image size: Large
            0,  // [11] Easy mode: Off
            0,  // [12] (unused)
            0,  // [13] Contrast: Normal
            0,  // [14] Saturation: Normal
            0,  // [15] Sharpness: Normal
            80, // [16] ISO: 80
            3,  // [17] Metering mode: Evaluative
            0,  // [18] Focus type
            0,  // [19] AF point
            1,  // [20] Exposure mode: Program AE
        ];

        for value in settings {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let result = parse_canon_makernote_impl(&data, ByteOrder::LittleEndian).unwrap();

        // Verify extracted values
        assert_eq!(result.get("Canon:MacroMode"), Some(&"Normal".to_string()));
        assert_eq!(result.get("Canon:Quality"), Some(&"Fine".to_string()));
        assert_eq!(result.get("Canon:FlashMode"), Some(&"On".to_string()));
        assert_eq!(result.get("Canon:DriveMode"), Some(&"Single".to_string()));
        assert_eq!(
            result.get("Canon:FocusMode"),
            Some(&"One-shot AF".to_string())
        );
        assert_eq!(
            result.get("Canon:MeteringMode"),
            Some(&"Evaluative".to_string())
        );
        assert_eq!(
            result.get("Canon:ExposureMode"),
            Some(&"Program AE".to_string())
        );
        assert_eq!(result.get("Canon:ISO"), Some(&"80".to_string()));
    }

    #[test]
    fn test_shot_info_indices() {
        assert_eq!(SHOT_INFO_AUTO_ISO, 1);
        assert_eq!(SHOT_INFO_BASE_ISO, 2);
        assert_eq!(SHOT_INFO_MEASURED_EV, 3);
        assert_eq!(SHOT_INFO_TARGET_APERTURE, 4);
        assert_eq!(SHOT_INFO_TARGET_SHUTTER_SPEED, 5);
        assert_eq!(SHOT_INFO_WHITE_BALANCE, 7);
        assert_eq!(SHOT_INFO_SLOW_SHUTTER, 8);
        assert_eq!(SHOT_INFO_SEQUENCE_NUMBER, 9);
        assert_eq!(SHOT_INFO_FLASH_GUIDE_NUMBER, 13);
        assert_eq!(SHOT_INFO_AF_POINTS_USED, 14);
        assert_eq!(SHOT_INFO_FLASH_EXPOSURE_COMP, 15);
        assert_eq!(SHOT_INFO_AUTO_EXPOSURE_BRACKETING, 16);
        assert_eq!(SHOT_INFO_SUBJECT_DISTANCE, 19);
    }

    #[test]
    fn test_parse_shot_info_array() {
        let mut data = Vec::new();
        data.extend_from_slice(b"Canon");
        data.extend_from_slice(&[0x01, 0x00]); // 1 entry

        // ShotInfo tag (0x0004)
        data.extend_from_slice(&[0x04, 0x00]); // Tag
        data.extend_from_slice(&[0x03, 0x00]); // Type: SHORT
        data.extend_from_slice(&[0x14, 0x00, 0x00, 0x00]); // Count: 20
        data.extend_from_slice(&[0x17, 0x00, 0x00, 0x00]); // Offset: 23
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Next IFD

        // ShotInfo array (20 values)
        let shot_info: Vec<i16> = vec![
            20,  // [0] Array length
            100, // [1] Auto ISO
            100, // [2] Base ISO
            128, // [3] Measured EV
            160, // [4] Target aperture (f/5.6)
            96,  // [5] Target shutter speed (1/60)
            0,   // [6] (unused)
            0,   // [7] White balance: Auto
            0,   // [8] Slow shutter: Off
            0,   // [9] Sequence number
            0, 0, 0, 0, // [10-13]
            0, // [14] AF points used
            0, // [15] Flash exposure comp
            0, // [16] Auto exposure bracketing
            0, 0,    // [17-18]
            1000, // [19] Subject distance (mm)
        ];

        for value in shot_info {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let result = parse_canon_makernote_impl(&data, ByteOrder::LittleEndian).unwrap();

        assert_eq!(result.get("Canon:AutoISO"), Some(&"100".to_string()));
        assert_eq!(result.get("Canon:BaseISO"), Some(&"100".to_string()));
        assert_eq!(result.get("Canon:MeasuredEV"), Some(&"128".to_string()));
        assert_eq!(result.get("Canon:TargetAperture"), Some(&"160".to_string()));
        assert_eq!(
            result.get("Canon:TargetShutterSpeed"),
            Some(&"96".to_string())
        );
        assert_eq!(
            result.get("Canon:SubjectDistance"),
            Some(&"1000 mm".to_string())
        );
    }

    #[test]
    fn test_parse_focal_length_array() {
        let mut data = Vec::new();
        data.extend_from_slice(b"Canon");
        data.extend_from_slice(&[0x01, 0x00]); // 1 entry

        // FocalLength tag (0x0002)
        data.extend_from_slice(&[0x02, 0x00]); // Tag
        data.extend_from_slice(&[0x03, 0x00]); // Type: SHORT
        data.extend_from_slice(&[0x04, 0x00, 0x00, 0x00]); // Count: 4
        data.extend_from_slice(&[0x17, 0x00, 0x00, 0x00]); // Offset: 23
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Next IFD

        // FocalLength array: [focal_type, focal_length, focal_plane_x_size, focal_plane_y_size]
        // focal_type: 2 (35mm equivalent available)
        // focal_length: 50mm (stored as 50)
        // focal_units: typically stored separately
        let focal_data: Vec<i16> = vec![2, 50, 0, 0];

        for value in focal_data {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let result = parse_canon_makernote_impl(&data, ByteOrder::LittleEndian).unwrap();

        assert_eq!(result.get("Canon:FocalType"), Some(&"2".to_string()));
        assert_eq!(result.get("Canon:FocalLength"), Some(&"50 mm".to_string()));
    }

    #[test]
    fn test_parse_lens_model_tag() {
        let mut data = Vec::new();
        data.extend_from_slice(b"Canon");
        data.extend_from_slice(&[0x01, 0x00]); // 1 entry

        // LensModel tag (0x0095)
        data.extend_from_slice(&[0x95, 0x00]); // Tag
        data.extend_from_slice(&[0x02, 0x00]); // Type: ASCII
        data.extend_from_slice(&[0x1E, 0x00, 0x00, 0x00]); // Count: 30 chars
        data.extend_from_slice(&[0x17, 0x00, 0x00, 0x00]); // Offset: 23
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Next IFD

        // Lens model string: "Canon EF 24-70mm f/2.8L II USM\0"
        let lens_name = b"Canon EF 24-70mm f/2.8L II USM\0";
        data.extend_from_slice(lens_name);

        let result = parse_canon_makernote_impl(&data, ByteOrder::LittleEndian).unwrap();

        assert_eq!(
            result.get("Canon:LensModel"),
            Some(&"Canon EF 24-70mm f/2.8L II USM".to_string())
        );
    }

    #[test]
    fn test_parse_file_info_with_lens_id() {
        let mut data = Vec::new();
        data.extend_from_slice(b"Canon");
        data.extend_from_slice(&[0x01, 0x00]); // 1 entry

        // FileInfo tag (0x0093)
        data.extend_from_slice(&[0x93, 0x00]); // Tag
        data.extend_from_slice(&[0x03, 0x00]); // Type: SHORT
        data.extend_from_slice(&[0x10, 0x00, 0x00, 0x00]); // Count: 16
        data.extend_from_slice(&[0x17, 0x00, 0x00, 0x00]); // Offset: 23
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Next IFD

        // FileInfo array (16 values)
        // Based on ExifTool Canon.pm: LensID is at index 6
        let file_info: Vec<i16> = vec![
            16,  // [0] Array length
            0,   // [1] File number
            0,   // [2] Shutter count low
            0,   // [3] Shutter count high
            0,   // [4] Bracket mode
            0,   // [5] Bracket value
            368, // [6] LensID: Canon EF 24-70mm f/2.8L II USM
            0, 0, 0, 0, 0, 0, 0, 0, 0, // [7-15]
        ];

        for value in file_info {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let result = parse_canon_makernote_impl(&data, ByteOrder::LittleEndian).unwrap();

        // Should extract lens name from database
        assert_eq!(
            result.get("Canon:LensType"),
            Some(&"Canon EF 24-70mm f/2.8L II USM".to_string())
        );
    }

    #[test]
    fn test_parse_af_info_array() {
        let mut data = Vec::new();
        data.extend_from_slice(b"Canon");
        data.extend_from_slice(&[0x01, 0x00]); // 1 entry

        // AFInfo tag (0x0012 or 0x0026)
        data.extend_from_slice(&[0x26, 0x00]); // Tag: AFInfo2
        data.extend_from_slice(&[0x03, 0x00]); // Type: SHORT
        data.extend_from_slice(&[0x14, 0x00, 0x00, 0x00]); // Count: 20
        data.extend_from_slice(&[0x17, 0x00, 0x00, 0x00]); // Offset: 23
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Next IFD

        // AFInfo array
        // Based on ExifTool: NumAFPoints at index 1, AFImageWidth at 2, AFImageHeight at 3
        let af_info: Vec<i16> = vec![
            20,     // [0] Array length
            45,     // [1] NumAFPoints (e.g., 45-point AF system)
            5568,   // [2] AFImageWidth
            3712,   // [3] AFImageHeight
            9,      // [4] AFAreaWidth
            9,      // [5] AFAreaHeight
            2784,   // [6] AFAreaXPositions (center)
            1856,   // [7] AFAreaYPositions (center)
            0x0001, // [8] AFPointsInFocus (bit 0 set = center point)
            0x0001, // [9] AFPointsSelected
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // [10-19]
        ];

        for value in af_info {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let result = parse_canon_makernote_impl(&data, ByteOrder::LittleEndian).unwrap();

        assert_eq!(result.get("Canon:NumAFPoints"), Some(&"45".to_string()));
        assert_eq!(result.get("Canon:AFImageWidth"), Some(&"5568".to_string()));
        assert_eq!(result.get("Canon:AFImageHeight"), Some(&"3712".to_string()));
        assert_eq!(result.get("Canon:AFPointsInFocus"), Some(&"1".to_string()));
    }

    #[test]
    fn test_parser_trait_implementation() {
        let parser = CanonParser;
        assert_eq!(parser.manufacturer_name(), "Canon");
        assert_eq!(parser.tag_prefix(), "Canon:");
    }

    #[test]
    fn test_validate_header() {
        let parser = CanonParser;

        // Test with Canon signature
        let with_signature = b"Canon\x00\x01\x00extra";
        assert!(parser.validate_header(with_signature));

        // Test without signature but valid IFD (reasonable entry count)
        let without_signature = b"\x05\x00extra_data_here_to_make_it_longer";
        assert!(parser.validate_header(without_signature));

        // Test invalid data (unreasonable entry count)
        let invalid = b"\xFF\xFF";
        assert!(!parser.validate_header(invalid));

        // Test too short data
        let too_short = b"\x01";
        assert!(!parser.validate_header(too_short));
    }

    #[test]
    fn test_lens_lookup() {
        let parser = CanonParser;

        // Test EF lens lookup
        assert!(parser.lookup_lens(368).is_some());
        assert_eq!(
            parser.lookup_lens(368),
            Some("Canon EF 24-70mm f/2.8L II USM".to_string())
        );

        // Test unknown lens
        assert_eq!(parser.lookup_lens(65000), None);
    }
}
