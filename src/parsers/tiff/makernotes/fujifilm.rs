//! Fujifilm MakerNote Parser
//!
//! Parses Fujifilm-specific EXIF MakerNote tags containing camera settings,
//! lens information, film simulation modes, and other proprietary metadata.
//!
//! Supports both X-series mirrorless cameras and GFX medium format cameras.
//!
//! Based on ExifTool's Fujifilm.pm module.

#![allow(dead_code)]
#![allow(unused_imports)]

use crate::error::{ExifToolError, Result};
use crate::io::EndianReader;
use crate::parsers::tiff::ifd_parser::{ByteOrder, IfdEntry};
use nom::{
    IResult,
    combinator::map,
    multi::count,
    number::complete::{be_u16, be_u32, le_u16, le_u32},
};
use std::collections::HashMap;

use super::fujifilm_lens_database::lookup_lens_name;
use super::shared::MakerNoteParser;
use super::shared::array_extractors::{extract_i16_array, extract_u16_array, extract_u32_array};
use crate::const_decoder;

// ===== Fujifilm MakerNote Tag IDs =====
// Based on ExifTool Fujifilm.pm tag definitions

// Basic Camera Information Tags
const FUJI_VERSION: u16 = 0x0000;
const FUJI_SERIAL_NUMBER: u16 = 0x0010;
const FUJI_QUALITY: u16 = 0x1000;
const FUJI_SHARPNESS: u16 = 0x1001;
const FUJI_WHITE_BALANCE: u16 = 0x1002;
const FUJI_SATURATION: u16 = 0x1003;
const FUJI_CONTRAST: u16 = 0x1004;
const FUJI_COLOR_TEMPERATURE: u16 = 0x1005;
const FUJI_CONTRAST_DETECTION_AF: u16 = 0x1006;
const FUJI_FLASH_MODE: u16 = 0x1010;
const FUJI_FLASH_EV: u16 = 0x1011;
const FUJI_MACRO: u16 = 0x1020;
const FUJI_FOCUS_MODE: u16 = 0x1021;
const FUJI_FOCUS_PIXEL: u16 = 0x1023;
const FUJI_SLOW_SYNC: u16 = 0x1030;
const FUJI_PICTURE_MODE: u16 = 0x1031;
const FUJI_EXR_AUTO: u16 = 0x1033;
const FUJI_EXR_MODE: u16 = 0x1034;
const FUJI_SHADOW_TONE: u16 = 0x1040;
const FUJI_HIGHLIGHT_TONE: u16 = 0x1041;
const FUJI_DIGITAL_ZOOM: u16 = 0x1044;
const FUJI_LENS_MODEL_NAME: u16 = 0x1050;

// Film Simulation and Color Tags
const FUJI_FILM_MODE: u16 = 0x1401;
const FUJI_DYNAMIC_RANGE: u16 = 0x1402;
const FUJI_DYNAMIC_RANGE_SETTING: u16 = 0x1403;
const FUJI_DEVELOPMENT_DYNAMIC_RANGE: u16 = 0x1404;
const FUJI_MIN_FOCAL_LENGTH: u16 = 0x1405;
const FUJI_MAX_FOCAL_LENGTH: u16 = 0x1406;
const FUJI_MAX_APERTURE_AT_MIN_FOCAL: u16 = 0x1407;
const FUJI_MAX_APERTURE_AT_MAX_FOCAL: u16 = 0x1408;

// Advanced Camera Settings
const FUJI_AUTO_DYNAMIC_RANGE: u16 = 0x140B;
const FUJI_FACES_DETECTED: u16 = 0x4100;
const FUJI_FACE_POSITIONS: u16 = 0x4103;
const FUJI_FACE_REC_INFO: u16 = 0x4282;
const FUJI_SHUTTER_TYPE: u16 = 0x1100;
const FUJI_BURST_MODE: u16 = 0x1101;
const FUJI_SEQUENCE_NUMBER: u16 = 0x1103;
const FUJI_BLUR_WARNING: u16 = 0x1300;
const FUJI_FOCUS_WARNING: u16 = 0x1301;
const FUJI_EXPOSURE_WARNING: u16 = 0x1302;
const FUJI_DYNAMIC_RANGE_WARNING: u16 = 0x1304;

// RAF (RAW) Image Tags
const FUJI_RAW_IMAGE_FULL_SIZE: u16 = 0xF000;
const FUJI_RAW_IMAGE_FULL_WIDTH: u16 = 0xF001;
const FUJI_RAW_IMAGE_FULL_HEIGHT: u16 = 0xF002;
const FUJI_RAW_IMAGE_ASPECT_RATIO: u16 = 0xF003;

// File and Image Information
const FUJI_FILE_SOURCE: u16 = 0x8000;
const FUJI_ORDER_NUMBER: u16 = 0x8002;
const FUJI_FRAME_NUMBER: u16 = 0x8003;
const FUJI_PARALLAX: u16 = 0xB211;

// Advanced Features
const FUJI_IMAGE_GENERATION: u16 = 0x1047;
const FUJI_RATING: u16 = 0x1431;
const FUJI_IMAGE_COUNT: u16 = 0x1438;
const FUJI_DRIVE_MODE: u16 = 0x1039;

// ===== NEW TAGS - Additional MakerNotes coverage =====

// Additional Image Quality Tags
const FUJI_WHITE_BALANCE_FINE_TUNE: u16 = 0x100A;
const FUJI_NOISE_REDUCTION: u16 = 0x100B;
const FUJI_HIGH_ISO_NOISE_REDUCTION: u16 = 0x100E;
const FUJI_AF_MODE: u16 = 0x1022;
const FUJI_EXR_MODE_SETTING: u16 = 0x1034; // Note: maps to 0x1034 (EXR_MODE is 0x1034 in original)
const FUJI_LENS_MODULATION_OPTIMIZER: u16 = 0x1045;
const FUJI_GRAIN_EFFECT_ROUGHNESS: u16 = 0x1046; // Note: different from IMAGE_GENERATION at 0x1047
const FUJI_COLOR_CHROME_EFFECT: u16 = 0x1048;
const FUJI_BW_ADJUSTMENT: u16 = 0x1049;
const FUJI_CROP_MODE: u16 = 0x104D;
const FUJI_COLOR_CHROME_FX_BLUE: u16 = 0x104E;

// Shooting Mode Tags
const FUJI_PIXEL_SHIFT_SHOTS: u16 = 0x1105;
const FUJI_PIXEL_SHIFT_OFFSET_NEW: u16 = 0x1106;
const FUJI_PANORAMA_ANGLE: u16 = 0x1153;
const FUJI_PANORAMA_DIRECTION: u16 = 0x1154;

// Advanced Filter Tags
const FUJI_ADVANCED_FILTER: u16 = 0x1201;
const FUJI_COLOR_MODE: u16 = 0x1210;

// Additional Dynamic Range Tags
const FUJI_IMAGE_STABILIZATION: u16 = 0x1422;
const FUJI_SCENE_RECOGNITION: u16 = 0x1425;
const FUJI_DRANGE_PRIORITY: u16 = 0x1443;
const FUJI_DRANGE_PRIORITY_AUTO: u16 = 0x1444;
const FUJI_DRANGE_PRIORITY_FIXED: u16 = 0x1445;

// Video Tags
const FUJI_VIDEO_RECORDING_MODE: u16 = 0x3803;
const FUJI_PERIPHERAL_LIGHTING: u16 = 0x3804;
const FUJI_VIDEO_COMPRESSION: u16 = 0x3806;
const FUJI_FRAME_RATE: u16 = 0x3820;
const FUJI_FRAME_WIDTH: u16 = 0x3821;
const FUJI_FRAME_HEIGHT: u16 = 0x3822;

// Additional Face Detection Tags
const FUJI_FACE_ELEMENT_SELECTED: u16 = 0x4005;
const FUJI_NUM_FACE_ELEMENTS: u16 = 0x4200;
const FUJI_FACE_ELEMENT_TYPES: u16 = 0x4201;
const FUJI_FACE_ELEMENT_POSITIONS: u16 = 0x4203;

// Fujifilm MakerNote header signature
// Fujifilm uses "FUJIFILM" followed by IFD offset
const FUJIFILM_HEADER: &[u8] = b"FUJIFILM";

// ============================================================================
// DECODERS - Fujifilm Value Decoders
// ============================================================================
// Following the shared decoder pattern from canon.rs and sony.rs
// Each decoder is a constant that implements the Decode trait

// Decodes Fujifilm quality setting to human-readable string
const_decoder!(pub
    DECODE_QUALITY, i32, [
        (1, "F (Fine)"),
        (2, "N (Normal)"),
        (3, "Fine"),
        (4, "Normal"),
        (5, "Fine+RAW"),
        (6, "Normal+RAW"),
    ]
);

// Decodes Fujifilm white balance setting to human-readable string
const_decoder!(pub
    DECODE_WHITE_BALANCE, i32, [
        (0x0000, "Auto"),
        (0x0001, "Auto (White Priority)"),
        (0x0002, "Auto (Ambience Priority)"),
        (0x0100, "Daylight"),
        (0x0200, "Cloudy"),
        (0x0300, "Daylight Fluorescent"),
        (0x0301, "Day White Fluorescent"),
        (0x0302, "White Fluorescent"),
        (0x0303, "Warm White Fluorescent"),
        (0x0304, "Living Room Warm White Fluorescent"),
        (0x0400, "Incandescent"),
        (0x0500, "Flash"),
        (0x0600, "Underwater"),
        (0x0F00, "Custom"),
        (0x0F01, "Custom2"),
        (0x0F02, "Custom3"),
        (0x0F03, "Custom4"),
        (0x0F04, "Custom5"),
        (0x0FF0, "Kelvin"),
    ]
);

// Decodes Fujifilm focus mode to human-readable string
const_decoder!(pub
    DECODE_FOCUS_MODE, i32, [
        (0, "Auto"),
        (1, "Manual"),
        (2, "AF-S (Single)"),
        (3, "AF-C (Continuous)"),
        (4, "AF-A (Automatic)"),
    ]
);

// Decodes Fujifilm flash mode to human-readable string
const_decoder!(pub
    DECODE_FLASH_MODE, i32, [
        (0, "Auto"),
        (1, "On"),
        (2, "Off"),
        (3, "Red-eye Reduction"),
        (4, "External"),
    ]
);

// Decodes Fujifilm film simulation mode to human-readable string
const_decoder!(pub
    DECODE_FILM_MODE, i32, [
        (0x0000, "F0/Standard (Provia)"),
        (0x0100, "F1/Studio Portrait"),
        (0x0110, "F1a/Studio Portrait Enhanced Saturation"),
        (0x0120, "F1b/Studio Portrait Smooth Skin Tone"),
        (0x0130, "F1c/Studio Portrait Increased Sharpness"),
        (0x0200, "F2/Fujichrome (Velvia)"),
        (0x0300, "F3/Studio Portrait Ex"),
        (0x0400, "F4/Velvia"),
        (0x0500, "Pro Neg. Std"),
        (0x0501, "Pro Neg. Hi"),
        (0x0600, "Classic Chrome"),
        (0x0700, "Eterna"),
        (0x0800, "Classic Negative"),
        (0x0900, "Bleach Bypass"),
        (0x0A00, "Nostalgic Neg."),
        (0x0B00, "Eterna Bleach Bypass"),
    ]
);

// Decodes Fujifilm dynamic range setting to human-readable string
const_decoder!(pub
    DECODE_DYNAMIC_RANGE, i32, [
        (1, "Standard (100%)"),
        (2, "Wide 1 (230%)"),
        (3, "Wide 2 (400%)"),
        (4, "Auto"),
    ]
);

// Decodes Fujifilm shutter type to human-readable string
const_decoder!(pub
    DECODE_SHUTTER_TYPE, i32, [
        (0, "Mechanical"),
        (1, "Electronic"),
        (2, "Electronic (Silent)"),
        (3, "Mechanical + Electronic"),
    ]
);

// Decodes Fujifilm burst mode to human-readable string
const_decoder!(pub
    DECODE_BURST_MODE, i32, [
        (0, "Off"),
        (1, "On (Low Speed)"),
        (2, "On (High Speed)"),
    ]
);

// Decodes Fujifilm picture mode to human-readable string
const_decoder!(pub
    DECODE_PICTURE_MODE, i32, [
        (0x0000, "Auto"),
        (0x0001, "Portrait"),
        (0x0002, "Landscape"),
        (0x0003, "Macro"),
        (0x0004, "Sports"),
        (0x0005, "Night Scene"),
        (0x0006, "Program AE"),
        (0x0007, "Aperture Priority AE"),
        (0x0008, "Shutter Priority AE"),
        (0x0009, "Manual"),
        (0x000A, "Portrait Enhancer"),
        (0x000B, "Natural Light"),
        (0x000D, "Beach"),
        (0x000E, "Snow"),
        (0x000F, "Fireworks"),
        (0x0010, "Underwater"),
        (0x0011, "Museum"),
        (0x0012, "Party"),
        (0x0013, "Flower"),
        (0x0014, "Text"),
        (0x0018, "Sunset"),
    ]
);

// Decodes Fujifilm drive mode to human-readable string
const_decoder!(pub
    DECODE_DRIVE_MODE, i32, [
        (0, "Single Frame"),
        (1, "Continuous Low"),
        (2, "Continuous High"),
        (3, "Bracketing"),
        (4, "Self-timer"),
        (5, "Remote"),
        (6, "Interval Timer"),
    ]
);

// Decodes Fujifilm EXR mode to human-readable string
const_decoder!(pub
    DECODE_EXR_MODE, i32, [
        (256, "HR (High Resolution)"),
        (512, "SN (Signal-to-Noise Priority)"),
        (768, "DR (Dynamic Range Priority)"),
    ]
);

// Decodes boolean/off-on value to human-readable string
const_decoder!(pub
    DECODE_OFF_ON, i32, [
        (0, "Off"),
        (1, "On"),
    ]
);

// ===== NEW DECODERS =====

// Decodes AF mode
const_decoder!(pub
    DECODE_AF_MODE, i32, [
        (0, "No"),
        (1, "Single Point"),
        (256, "Zone"),
        (512, "Wide/Tracking"),
    ]
);

// Decodes noise reduction
const_decoder!(pub
    DECODE_NOISE_REDUCTION, i32, [
        (0, "Normal"),
        (256, "Strong"),
        (512, "Weak"),
    ]
);

// Decodes high ISO noise reduction
const_decoder!(pub
    DECODE_HIGH_ISO_NR, i32, [
        (-2, "Very Weak"),
        (-1, "Weak"),
        (0, "Normal"),
        (1, "Strong"),
        (2, "Very Strong"),
    ]
);

// Decodes grain effect roughness / Color Chrome levels
const_decoder!(pub
    DECODE_EFFECT_STRENGTH, i32, [
        (0, "Off"),
        (32, "Weak"),
        (64, "Strong"),
    ]
);

// Decodes crop mode
const_decoder!(pub
    DECODE_CROP_MODE, i32, [
        (0, "None"),
        (1, "Sports Finder Mode"),
        (2, "1.25x Crop"),
        (4, "Digital Teleconverter x1.4"),
        (8, "Digital Teleconverter x2.0"),
    ]
);

// Decodes auto bracketing
const_decoder!(pub
    DECODE_AUTO_BRACKETING, i32, [
        (0, "Off"),
        (1, "On"),
        (2, "No Flash/Flash"),
        (3, "Film Simulation"),
        (4, "Dynamic Range"),
    ]
);

// Decodes panorama direction
const_decoder!(pub
    DECODE_PANORAMA_DIRECTION, i32, [
        (1, "Right"),
        (2, "Up"),
        (3, "Left"),
        (4, "Down"),
    ]
);

// Decodes advanced filter
const_decoder!(pub
    DECODE_ADVANCED_FILTER, i32, [
        (0x0000, "Off"),
        (0x0001, "Toy Camera"),
        (0x0002, "Miniature"),
        (0x0003, "Pop Color"),
        (0x0004, "High Key"),
        (0x0005, "Low Key"),
        (0x0006, "Dynamic Tone"),
        (0x0007, "Soft Focus"),
        (0x0008, "Partial Color (Red)"),
        (0x0009, "Partial Color (Yellow)"),
        (0x000A, "Partial Color (Green)"),
        (0x000B, "Partial Color (Blue)"),
        (0x000C, "Partial Color (Orange)"),
        (0x000D, "Partial Color (Purple)"),
        (0x0010, "Rich & Fine"),
    ]
);

// Decodes color mode
const_decoder!(pub
    DECODE_COLOR_MODE, i32, [
        (0, "Standard"),
        (16, "Chrome"),
        (32, "B&W"),
    ]
);

// Decodes image stabilization
const_decoder!(pub
    DECODE_IMAGE_STABILIZATION, i32, [
        (0, "None"),
        (1, "Optical"),
        (2, "Sensor-Shift"),
        (3, "Optical + Sensor-Shift"),
        (256, "Lens-Sensor Shift"),
        (512, "Lens-5-Axis"),
    ]
);

// Decodes scene recognition
const_decoder!(pub
    DECODE_SCENE_RECOGNITION, i32, [
        (0, "Unrecognized"),
        (0x100, "Portrait"),
        (0x103, "Night Portrait"),
        (0x105, "Backlit Portrait"),
        (0x200, "Landscape"),
        (0x300, "Night Scene"),
        (0x400, "Macro"),
    ]
);

// Decodes D-Range priority
const_decoder!(pub
    DECODE_DRANGE_PRIORITY, i32, [
        (0, "Auto"),
        (1, "Weak"),
        (2, "Strong"),
    ]
);

// Decodes video recording mode
const_decoder!(pub
    DECODE_VIDEO_RECORDING_MODE, i32, [
        (0, "Normal"),
        (1, "F-Log"),
        (2, "HLG"),
    ]
);

// Decodes video compression
const_decoder!(pub
    DECODE_VIDEO_COMPRESSION, i32, [
        (1, "H.264"),
        (2, "H.265"),
        (3, "ProRes"),
    ]
);

/// Represents a Fujifilm MakerNote parser
pub struct FujifilmParser;

impl MakerNoteParser for FujifilmParser {
    fn manufacturer_name(&self) -> &'static str {
        "Fujifilm"
    }

    fn tag_prefix(&self) -> &'static str {
        "Fujifilm:"
    }

    fn validate_header(&self, data: &[u8]) -> bool {
        // Fujifilm MakerNotes start with "FUJIFILM" (8 bytes) followed by offset
        data.len() >= 12 && &data[0..8] == FUJIFILM_HEADER
    }

    fn parse(
        &self,
        data: &[u8],
        _byte_order: ByteOrder,
        tags: &mut HashMap<String, String>,
    ) -> std::result::Result<(), String> {
        if data.is_empty() {
            return Ok(());
        }

        // Validate Fujifilm header
        if !self.validate_header(data) {
            return Err("Invalid Fujifilm MakerNote header".to_string());
        }

        // CRITICAL: Fujifilm MakerNotes ALWAYS use little-endian byte order,
        // regardless of the main EXIF byte order. This is a Fujifilm-specific
        // quirk that differs from most other camera manufacturers.
        let fuji_byte_order = ByteOrder::LittleEndian;

        // Fujifilm header structure:
        // - Bytes 0-7: "FUJIFILM" signature
        // - Bytes 8-11: IFD offset (4 bytes, little-endian, typically 0x0C = 12)
        // - Byte 12+: IFD data starts

        // Read IFD offset using little-endian byte order
        let reader = EndianReader::new(data, fuji_byte_order.to_io_byte_order());
        let ifd_offset = reader.u32_at(8).unwrap_or(0) as usize;

        // Fujifilm offsets are relative to the MakerNote start
        if ifd_offset >= data.len() {
            return Ok(());
        }

        let ifd_data = &data[ifd_offset..];

        // Parse IFD entry count using little-endian byte order
        if ifd_data.len() < 2 {
            return Ok(());
        }

        let ifd_reader = EndianReader::new(ifd_data, fuji_byte_order.to_io_byte_order());
        let entry_count = ifd_reader.u16_at(0).unwrap_or(0);

        // Parse IFD entries (always little-endian for Fujifilm)
        let entries_start = &ifd_data[2..];
        let entries = match parse_ifd_entries(entries_start, entry_count, fuji_byte_order) {
            Ok((_, entries)) => entries,
            Err(_) => return Ok(()), // Return empty on parse failure
        };

        // Extract tags from entries
        for entry in entries {
            match entry.tag_id {
                // String tags
                FUJI_VERSION | FUJI_SERIAL_NUMBER | FUJI_LENS_MODEL_NAME => {
                    if let Some(value) = extract_string_value(&entry, data) {
                        let tag_name = fujifilm_tag_to_name(entry.tag_id);
                        tags.insert(tag_name, value);
                    }
                }

                // Simple integer tags
                FUJI_SEQUENCE_NUMBER | FUJI_FRAME_NUMBER | FUJI_IMAGE_COUNT | FUJI_RATING => {
                    let value = entry.value_offset;
                    let tag_name = fujifilm_tag_to_name(entry.tag_id);
                    tags.insert(tag_name, value.to_string());
                }

                // Enumerated value tags with decoders
                FUJI_QUALITY => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Fujifilm:Quality".to_string(),
                        DECODE_QUALITY.decode(value).to_string(),
                    );
                }

                FUJI_WHITE_BALANCE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Fujifilm:WhiteBalance".to_string(),
                        DECODE_WHITE_BALANCE.decode(value).to_string(),
                    );
                }

                FUJI_FOCUS_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Fujifilm:FocusMode".to_string(),
                        DECODE_FOCUS_MODE.decode(value).to_string(),
                    );
                }

                FUJI_FLASH_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Fujifilm:FlashMode".to_string(),
                        DECODE_FLASH_MODE.decode(value).to_string(),
                    );
                }

                FUJI_FILM_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Fujifilm:FilmMode".to_string(),
                        DECODE_FILM_MODE.decode(value).to_string(),
                    );
                }

                FUJI_DYNAMIC_RANGE | FUJI_DYNAMIC_RANGE_SETTING => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Fujifilm:DynamicRange".to_string(),
                        DECODE_DYNAMIC_RANGE.decode(value).to_string(),
                    );
                }

                FUJI_SHUTTER_TYPE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Fujifilm:ShutterType".to_string(),
                        DECODE_SHUTTER_TYPE.decode(value).to_string(),
                    );
                }

                FUJI_BURST_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Fujifilm:BurstMode".to_string(),
                        DECODE_BURST_MODE.decode(value).to_string(),
                    );
                }

                FUJI_PICTURE_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Fujifilm:PictureMode".to_string(),
                        DECODE_PICTURE_MODE.decode(value).to_string(),
                    );
                }

                FUJI_DRIVE_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Fujifilm:DriveMode".to_string(),
                        DECODE_DRIVE_MODE.decode(value).to_string(),
                    );
                }

                FUJI_EXR_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Fujifilm:EXRMode".to_string(),
                        DECODE_EXR_MODE.decode(value).to_string(),
                    );
                }

                // Simple numeric tags with units
                FUJI_SHARPNESS | FUJI_SATURATION | FUJI_CONTRAST => {
                    let value = entry.value_offset as i32;
                    let tag_name = fujifilm_tag_to_name(entry.tag_id);
                    // Fujifilm uses scale: 0=normal, +values=more, -values=less
                    let description = match value {
                        v if v < 0 => format!("{} (Soft)", v),
                        0 => "0 (Normal)".to_string(),
                        v => format!("{} (Hard)", v),
                    };
                    tags.insert(tag_name, description);
                }

                FUJI_SHADOW_TONE | FUJI_HIGHLIGHT_TONE => {
                    let value = entry.value_offset as i32;
                    let tag_name = fujifilm_tag_to_name(entry.tag_id);
                    // Shadow/Highlight tone: -64 to +64 (0 = standard)
                    tags.insert(tag_name, format!("{:+}", value));
                }

                FUJI_COLOR_TEMPERATURE => {
                    let value = entry.value_offset;
                    if value > 0 {
                        tags.insert(
                            "Fujifilm:ColorTemperature".to_string(),
                            format!("{} K", value),
                        );
                    }
                }

                FUJI_FACES_DETECTED => {
                    let value = entry.value_offset;
                    tags.insert("Fujifilm:FacesDetected".to_string(), value.to_string());
                }

                // Boolean/On-Off tags
                FUJI_MACRO | FUJI_SLOW_SYNC | FUJI_EXR_AUTO | FUJI_AUTO_DYNAMIC_RANGE => {
                    let value = entry.value_offset as i32;
                    let tag_name = fujifilm_tag_to_name(entry.tag_id);
                    tags.insert(tag_name, DECODE_OFF_ON.decode(value).to_string());
                }

                // Warning flags
                FUJI_BLUR_WARNING
                | FUJI_FOCUS_WARNING
                | FUJI_EXPOSURE_WARNING
                | FUJI_DYNAMIC_RANGE_WARNING => {
                    let value = entry.value_offset as i32;
                    let tag_name = fujifilm_tag_to_name(entry.tag_id);
                    let warning = if value == 0 { "None" } else { "Warning" };
                    tags.insert(tag_name, warning.to_string());
                }

                // Lens focal length information
                FUJI_MIN_FOCAL_LENGTH | FUJI_MAX_FOCAL_LENGTH => {
                    let value = entry.value_offset as f32 / 10.0; // Stored in 0.1mm units
                    let tag_name = fujifilm_tag_to_name(entry.tag_id);
                    tags.insert(tag_name, format!("{:.1} mm", value));
                }

                FUJI_MAX_APERTURE_AT_MIN_FOCAL | FUJI_MAX_APERTURE_AT_MAX_FOCAL => {
                    let value = entry.value_offset as f32 / 100.0; // Stored in 0.01 units
                    let tag_name = fujifilm_tag_to_name(entry.tag_id);
                    tags.insert(tag_name, format!("f/{:.1}", value));
                }

                // RAW image dimensions
                FUJI_RAW_IMAGE_FULL_WIDTH | FUJI_RAW_IMAGE_FULL_HEIGHT => {
                    let value = entry.value_offset;
                    let tag_name = fujifilm_tag_to_name(entry.tag_id);
                    tags.insert(tag_name, format!("{} px", value));
                }

                // Digital zoom
                FUJI_DIGITAL_ZOOM => {
                    let value = entry.value_offset as f32 / 100.0; // Stored as percentage
                    tags.insert("Fujifilm:DigitalZoom".to_string(), format!("{:.2}x", value));
                }

                // Flash exposure compensation
                FUJI_FLASH_EV => {
                    // Flash EV stored as signed value in units of 1/3 EV
                    let raw_value = entry.value_offset as i32;
                    let ev_value = raw_value as f32 / 3.0;
                    tags.insert(
                        "Fujifilm:FlashExposureComp".to_string(),
                        format!("{:+.1} EV", ev_value),
                    );
                }

                // Focus pixel coordinates (array)
                FUJI_FOCUS_PIXEL => {
                    if let Some(array) = extract_u16_array(&entry, data, fuji_byte_order)
                        && array.len() >= 2
                    {
                        tags.insert(
                            "MakerNotes:FocusPixel".to_string(),
                            format!("X:{} Y:{}", array[0], array[1]),
                        );
                    }
                }

                // Face positions (array) - complex structure, basic extraction
                FUJI_FACE_POSITIONS => {
                    if let Some(array) = extract_u16_array(&entry, data, fuji_byte_order)
                        && !array.is_empty()
                    {
                        tags.insert(
                            "MakerNotes:FacePositions".to_string(),
                            format!("{} coordinates", array.len() / 4),
                        );
                    }
                }

                // ===== NEW TAG HANDLING =====

                // AF Mode
                FUJI_AF_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "MakerNotes:AFMode".to_string(),
                        DECODE_AF_MODE.decode(value).to_string(),
                    );
                }

                // Noise reduction tags
                FUJI_NOISE_REDUCTION => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "MakerNotes:NoiseReduction".to_string(),
                        DECODE_NOISE_REDUCTION.decode(value).to_string(),
                    );
                }

                FUJI_HIGH_ISO_NOISE_REDUCTION => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "MakerNotes:HighISONoiseReduction".to_string(),
                        DECODE_HIGH_ISO_NR.decode(value).to_string(),
                    );
                }

                // White balance fine tune
                FUJI_WHITE_BALANCE_FINE_TUNE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "MakerNotes:WhiteBalanceFineTune".to_string(),
                        format!("{:+}", value),
                    );
                }

                // Lens Modulation Optimizer
                FUJI_LENS_MODULATION_OPTIMIZER => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "MakerNotes:LensModulationOptimizer".to_string(),
                        DECODE_OFF_ON.decode(value).to_string(),
                    );
                }

                // Grain Effect Roughness
                FUJI_GRAIN_EFFECT_ROUGHNESS => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "MakerNotes:GrainEffectRoughness".to_string(),
                        DECODE_EFFECT_STRENGTH.decode(value).to_string(),
                    );
                }

                // Color Chrome Effect
                FUJI_COLOR_CHROME_EFFECT => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "MakerNotes:ColorChromeEffect".to_string(),
                        DECODE_EFFECT_STRENGTH.decode(value).to_string(),
                    );
                }

                // B&W Adjustment
                FUJI_BW_ADJUSTMENT => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "MakerNotes:BWAdjustment".to_string(),
                        format!("{:+}", value),
                    );
                }

                // Crop Mode
                FUJI_CROP_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "MakerNotes:CropMode".to_string(),
                        DECODE_CROP_MODE.decode(value).to_string(),
                    );
                }

                // Color Chrome FX Blue
                FUJI_COLOR_CHROME_FX_BLUE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "MakerNotes:ColorChromeFXBlue".to_string(),
                        DECODE_EFFECT_STRENGTH.decode(value).to_string(),
                    );
                }

                // Pixel Shift
                FUJI_PIXEL_SHIFT_SHOTS => {
                    let value = entry.value_offset;
                    tags.insert("MakerNotes:PixelShiftShots".to_string(), value.to_string());
                }

                FUJI_PIXEL_SHIFT_OFFSET_NEW => {
                    if let Some(array) = extract_u16_array(&entry, data, fuji_byte_order)
                        && array.len() >= 2
                    {
                        tags.insert(
                            "MakerNotes:PixelShiftOffset".to_string(),
                            format!("X:{} Y:{}", array[0], array[1]),
                        );
                    }
                }

                // Panorama tags
                FUJI_PANORAMA_ANGLE => {
                    let value = entry.value_offset;
                    tags.insert(
                        "MakerNotes:PanoramaAngle".to_string(),
                        format!("{} deg", value),
                    );
                }

                FUJI_PANORAMA_DIRECTION => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "MakerNotes:PanoramaDirection".to_string(),
                        DECODE_PANORAMA_DIRECTION.decode(value).to_string(),
                    );
                }

                // Advanced Filter
                FUJI_ADVANCED_FILTER => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "MakerNotes:AdvancedFilter".to_string(),
                        DECODE_ADVANCED_FILTER.decode(value).to_string(),
                    );
                }

                // Color Mode
                FUJI_COLOR_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "MakerNotes:ColorMode".to_string(),
                        DECODE_COLOR_MODE.decode(value).to_string(),
                    );
                }

                // Image Stabilization
                FUJI_IMAGE_STABILIZATION => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "MakerNotes:ImageStabilization".to_string(),
                        DECODE_IMAGE_STABILIZATION.decode(value).to_string(),
                    );
                }

                // Scene Recognition
                FUJI_SCENE_RECOGNITION => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "MakerNotes:SceneRecognition".to_string(),
                        DECODE_SCENE_RECOGNITION.decode(value).to_string(),
                    );
                }

                // D-Range Priority tags
                FUJI_DRANGE_PRIORITY => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "MakerNotes:DRangePriority".to_string(),
                        DECODE_DRANGE_PRIORITY.decode(value).to_string(),
                    );
                }

                FUJI_DRANGE_PRIORITY_AUTO => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "MakerNotes:DRangePriorityAuto".to_string(),
                        DECODE_DRANGE_PRIORITY.decode(value).to_string(),
                    );
                }

                FUJI_DRANGE_PRIORITY_FIXED => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "MakerNotes:DRangePriorityFixed".to_string(),
                        DECODE_DRANGE_PRIORITY.decode(value).to_string(),
                    );
                }

                // Video tags
                FUJI_VIDEO_RECORDING_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "MakerNotes:VideoRecordingMode".to_string(),
                        DECODE_VIDEO_RECORDING_MODE.decode(value).to_string(),
                    );
                }

                FUJI_PERIPHERAL_LIGHTING => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "MakerNotes:PeripheralLighting".to_string(),
                        DECODE_OFF_ON.decode(value).to_string(),
                    );
                }

                FUJI_VIDEO_COMPRESSION => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "MakerNotes:VideoCompression".to_string(),
                        DECODE_VIDEO_COMPRESSION.decode(value).to_string(),
                    );
                }

                FUJI_FRAME_RATE => {
                    let value = entry.value_offset as f32 / 1000.0;
                    tags.insert(
                        "MakerNotes:FrameRate".to_string(),
                        format!("{:.3} fps", value),
                    );
                }

                FUJI_FRAME_WIDTH => {
                    let value = entry.value_offset;
                    tags.insert("MakerNotes:FrameWidth".to_string(), format!("{} px", value));
                }

                FUJI_FRAME_HEIGHT => {
                    let value = entry.value_offset;
                    tags.insert(
                        "MakerNotes:FrameHeight".to_string(),
                        format!("{} px", value),
                    );
                }

                // Face element tags
                FUJI_FACE_ELEMENT_SELECTED => {
                    let value = entry.value_offset;
                    tags.insert(
                        "MakerNotes:FaceElementSelected".to_string(),
                        value.to_string(),
                    );
                }

                FUJI_NUM_FACE_ELEMENTS => {
                    let value = entry.value_offset;
                    tags.insert("MakerNotes:NumFaceElements".to_string(), value.to_string());
                }

                FUJI_FACE_ELEMENT_TYPES => {
                    if let Some(array) = extract_u16_array(&entry, data, fuji_byte_order)
                        && !array.is_empty()
                    {
                        let types: Vec<String> = array.iter().map(|v| v.to_string()).collect();
                        tags.insert("MakerNotes:FaceElementTypes".to_string(), types.join(", "));
                    }
                }

                FUJI_FACE_ELEMENT_POSITIONS => {
                    if let Some(array) = extract_u16_array(&entry, data, fuji_byte_order)
                        && !array.is_empty()
                    {
                        tags.insert(
                            "MakerNotes:FaceElementPositions".to_string(),
                            format!("{} coordinates", array.len() / 4),
                        );
                    }
                }

                // Other tags - skip unknown tags
                _ => continue,
            }
        }

        Ok(())
    }

    fn lookup_lens(&self, lens_id: u16) -> Option<String> {
        lookup_lens_name(lens_id)
    }
}

/// Maps Fujifilm MakerNote tag IDs to human-readable tag names
fn fujifilm_tag_to_name(tag_id: u16) -> String {
    let tag_name = match tag_id {
        FUJI_VERSION => "Version",
        FUJI_SERIAL_NUMBER => "SerialNumber",
        FUJI_QUALITY => "Quality",
        FUJI_SHARPNESS => "Sharpness",
        FUJI_WHITE_BALANCE => "WhiteBalance",
        FUJI_SATURATION => "Saturation",
        FUJI_CONTRAST => "Contrast",
        FUJI_COLOR_TEMPERATURE => "ColorTemperature",
        FUJI_FLASH_MODE => "FlashMode",
        FUJI_FLASH_EV => "FlashExposureComp",
        FUJI_MACRO => "Macro",
        FUJI_FOCUS_MODE => "FocusMode",
        FUJI_FOCUS_PIXEL => "FocusPixel",
        FUJI_SLOW_SYNC => "SlowSync",
        FUJI_PICTURE_MODE => "PictureMode",
        FUJI_EXR_AUTO => "EXRAuto",
        FUJI_EXR_MODE => "EXRMode",
        FUJI_SHADOW_TONE => "ShadowTone",
        FUJI_HIGHLIGHT_TONE => "HighlightTone",
        FUJI_DIGITAL_ZOOM => "DigitalZoom",
        FUJI_LENS_MODEL_NAME => "LensModelName",
        FUJI_FILM_MODE => "FilmMode",
        FUJI_DYNAMIC_RANGE => "DynamicRange",
        FUJI_DYNAMIC_RANGE_SETTING => "DynamicRangeSetting",
        FUJI_MIN_FOCAL_LENGTH => "MinFocalLength",
        FUJI_MAX_FOCAL_LENGTH => "MaxFocalLength",
        FUJI_MAX_APERTURE_AT_MIN_FOCAL => "MaxApertureAtMinFocal",
        FUJI_MAX_APERTURE_AT_MAX_FOCAL => "MaxApertureAtMaxFocal",
        FUJI_AUTO_DYNAMIC_RANGE => "AutoDynamicRange",
        FUJI_FACES_DETECTED => "FacesDetected",
        FUJI_FACE_POSITIONS => "FacePositions",
        FUJI_SHUTTER_TYPE => "ShutterType",
        FUJI_BURST_MODE => "BurstMode",
        FUJI_SEQUENCE_NUMBER => "SequenceNumber",
        FUJI_BLUR_WARNING => "BlurWarning",
        FUJI_FOCUS_WARNING => "FocusWarning",
        FUJI_EXPOSURE_WARNING => "ExposureWarning",
        FUJI_DYNAMIC_RANGE_WARNING => "DynamicRangeWarning",
        FUJI_RAW_IMAGE_FULL_WIDTH => "RawImageFullWidth",
        FUJI_RAW_IMAGE_FULL_HEIGHT => "RawImageFullHeight",
        FUJI_FRAME_NUMBER => "FrameNumber",
        FUJI_IMAGE_COUNT => "ImageCount",
        FUJI_DRIVE_MODE => "DriveMode",
        FUJI_RATING => "Rating",
        _ => return format!("Fujifilm:Unknown-{:#06X}", tag_id),
    };

    format!("Fujifilm:{}", tag_name)
}

/// Parses IFD entries in the specified byte order
fn parse_ifd_entries(
    input: &[u8],
    entry_count: u16,
    byte_order: ByteOrder,
) -> IResult<&[u8], Vec<IfdEntry>> {
    use nom::Parser;
    match byte_order {
        ByteOrder::LittleEndian => count(parse_ifd_entry_le, entry_count as usize).parse(input),
        ByteOrder::BigEndian => count(parse_ifd_entry_be, entry_count as usize).parse(input),
    }
}

/// Parses a single IFD entry in little-endian byte order
fn parse_ifd_entry_le(input: &[u8]) -> IResult<&[u8], IfdEntry> {
    use nom::Parser;
    map(
        |input| {
            let (input, tag_id) = le_u16(input)?;
            let (input, field_type) = le_u16(input)?;
            let (input, value_count) = le_u32(input)?;
            let (input, value_offset) = le_u32(input)?;
            Ok((input, (tag_id, field_type, value_count, value_offset)))
        },
        |(tag_id, field_type, value_count, value_offset)| IfdEntry {
            tag_id,
            field_type,
            value_count,
            value_offset,
        },
    )
    .parse(input)
}

/// Parses a single IFD entry in big-endian byte order
fn parse_ifd_entry_be(input: &[u8]) -> IResult<&[u8], IfdEntry> {
    use nom::Parser;
    map(
        |input| {
            let (input, tag_id) = be_u16(input)?;
            let (input, field_type) = be_u16(input)?;
            let (input, value_count) = be_u32(input)?;
            let (input, value_offset) = be_u32(input)?;
            Ok((input, (tag_id, field_type, value_count, value_offset)))
        },
        |(tag_id, field_type, value_count, value_offset)| IfdEntry {
            tag_id,
            field_type,
            value_count,
            value_offset,
        },
    )
    .parse(input)
}

/// Extracts string value from IFD entry
///
/// Handles both inline strings (≤4 bytes) and offset-based strings
fn extract_string_value(entry: &IfdEntry, full_data: &[u8]) -> Option<String> {
    let byte_count = entry.value_count as usize;

    // For inline strings (≤4 bytes), value is in value_offset field
    if byte_count <= 4 {
        let bytes = entry.value_offset.to_le_bytes();
        let s = std::str::from_utf8(&bytes[0..byte_count])
            .ok()?
            .trim_end_matches('\0')
            .trim();
        return Some(s.to_string());
    }

    // For longer strings, read from offset
    // Fujifilm offsets are relative to MakerNote start
    let offset = entry.value_offset as usize;

    if offset + byte_count <= full_data.len() {
        let bytes = &full_data[offset..offset + byte_count];
        let s = std::str::from_utf8(bytes)
            .ok()?
            .trim_end_matches('\0')
            .trim();
        return Some(s.to_string());
    }

    None
}

/// Public function to parse Fujifilm MakerNotes
///
/// This is the main entry point for parsing Fujifilm MakerNote data.
///
/// # Parameters
/// - `data`: Raw MakerNote data (including Fujifilm header)
/// - `byte_order`: Byte order for parsing multi-byte values
/// - `tags`: HashMap to populate with extracted tags
pub fn parse_fujifilm_makernotes(
    data: &[u8],
    byte_order: ByteOrder,
    tags: &mut HashMap<String, String>,
) {
    let parser = FujifilmParser;
    if let Err(e) = parser.parse(data, byte_order, tags) {
        eprintln!("Fujifilm MakerNotes parse error: {}", e);
    }
}

/// Checks if data appears to be a Fujifilm MakerNote
///
/// # Parameters
/// - `data`: Raw byte data to check
///
/// # Returns
/// `true` if the data appears to be a Fujifilm MakerNote, `false` otherwise
pub fn is_fujifilm_makernote(data: &[u8]) -> bool {
    data.len() >= 12 && &data[0..8] == FUJIFILM_HEADER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fujifilm_tag_ids() {
        assert_eq!(FUJI_VERSION, 0x0000);
        assert_eq!(FUJI_QUALITY, 0x1000);
        assert_eq!(FUJI_WHITE_BALANCE, 0x1002);
        assert_eq!(FUJI_FILM_MODE, 0x1401);
        assert_eq!(FUJI_SHUTTER_TYPE, 0x1100);
    }

    #[test]
    fn test_fujifilm_header_validation() {
        let parser = FujifilmParser;

        // Valid Fujifilm header
        let valid_header = b"FUJIFILM\x0C\x00\x00\x00extra data";
        assert!(parser.validate_header(valid_header));

        // Invalid header (wrong signature)
        let invalid = b"CANON\0\x00\x00\x00\x00\x00\x00";
        assert!(!parser.validate_header(invalid));

        // Too short
        let too_short = b"FUJIFILM\x0C";
        assert!(!parser.validate_header(too_short));
    }

    #[test]
    fn test_is_fujifilm_makernote() {
        assert!(is_fujifilm_makernote(b"FUJIFILM\x0C\x00\x00\x00test"));
        assert!(!is_fujifilm_makernote(b"NIKON\0\x00\x00"));
        assert!(!is_fujifilm_makernote(b"FUJIFILM\x0C")); // Too short
    }

    #[test]
    fn test_fujifilm_tag_to_name() {
        assert_eq!(fujifilm_tag_to_name(0x0000), "Fujifilm:Version");
        assert_eq!(fujifilm_tag_to_name(0x1000), "Fujifilm:Quality");
        assert_eq!(fujifilm_tag_to_name(0x1002), "Fujifilm:WhiteBalance");
        assert_eq!(fujifilm_tag_to_name(0x1401), "Fujifilm:FilmMode");
        assert_eq!(fujifilm_tag_to_name(0xFFFF), "Fujifilm:Unknown-0xFFFF");
    }

    #[test]
    fn test_decode_quality() {
        assert_eq!(DECODE_QUALITY.decode(1), "F (Fine)");
        assert_eq!(DECODE_QUALITY.decode(3), "Fine");
        assert_eq!(DECODE_QUALITY.decode(5), "Fine+RAW");
        assert_eq!(DECODE_QUALITY.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_white_balance() {
        assert_eq!(DECODE_WHITE_BALANCE.decode(0x0000), "Auto");
        assert_eq!(DECODE_WHITE_BALANCE.decode(0x0100), "Daylight");
        assert_eq!(DECODE_WHITE_BALANCE.decode(0x0200), "Cloudy");
        assert_eq!(DECODE_WHITE_BALANCE.decode(0x0400), "Incandescent");
        assert_eq!(DECODE_WHITE_BALANCE.decode(0x9999), "Unknown (39321)");
    }

    #[test]
    fn test_decode_focus_mode() {
        assert_eq!(DECODE_FOCUS_MODE.decode(0), "Auto");
        assert_eq!(DECODE_FOCUS_MODE.decode(1), "Manual");
        assert_eq!(DECODE_FOCUS_MODE.decode(2), "AF-S (Single)");
        assert_eq!(DECODE_FOCUS_MODE.decode(3), "AF-C (Continuous)");
        assert_eq!(DECODE_FOCUS_MODE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_film_mode() {
        assert_eq!(DECODE_FILM_MODE.decode(0x0000), "F0/Standard (Provia)");
        assert_eq!(DECODE_FILM_MODE.decode(0x0200), "F2/Fujichrome (Velvia)");
        assert_eq!(DECODE_FILM_MODE.decode(0x0600), "Classic Chrome");
        assert_eq!(DECODE_FILM_MODE.decode(0x0700), "Eterna");
        assert_eq!(DECODE_FILM_MODE.decode(0x0800), "Classic Negative");
        assert_eq!(DECODE_FILM_MODE.decode(0x9999), "Unknown (39321)");
    }

    #[test]
    fn test_decode_dynamic_range() {
        assert_eq!(DECODE_DYNAMIC_RANGE.decode(1), "Standard (100%)");
        assert_eq!(DECODE_DYNAMIC_RANGE.decode(2), "Wide 1 (230%)");
        assert_eq!(DECODE_DYNAMIC_RANGE.decode(3), "Wide 2 (400%)");
        assert_eq!(DECODE_DYNAMIC_RANGE.decode(4), "Auto");
        assert_eq!(DECODE_DYNAMIC_RANGE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_shutter_type() {
        assert_eq!(DECODE_SHUTTER_TYPE.decode(0), "Mechanical");
        assert_eq!(DECODE_SHUTTER_TYPE.decode(1), "Electronic");
        assert_eq!(DECODE_SHUTTER_TYPE.decode(2), "Electronic (Silent)");
        assert_eq!(DECODE_SHUTTER_TYPE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_picture_mode() {
        assert_eq!(DECODE_PICTURE_MODE.decode(0x0000), "Auto");
        assert_eq!(DECODE_PICTURE_MODE.decode(0x0001), "Portrait");
        assert_eq!(DECODE_PICTURE_MODE.decode(0x0002), "Landscape");
        assert_eq!(DECODE_PICTURE_MODE.decode(0x0006), "Program AE");
        assert_eq!(DECODE_PICTURE_MODE.decode(0x0009), "Manual");
    }

    #[test]
    fn test_parser_trait_implementation() {
        let parser = FujifilmParser;
        assert_eq!(parser.manufacturer_name(), "Fujifilm");
        assert_eq!(parser.tag_prefix(), "Fujifilm:");
    }

    #[test]
    fn test_lens_lookup() {
        let parser = FujifilmParser;

        // Test XF lens lookup
        assert!(parser.lookup_lens(35).is_some());
        assert_eq!(parser.lookup_lens(35), Some("XF 35mm f/1.4 R".to_string()));

        // Test GF lens lookup
        assert_eq!(
            parser.lookup_lens(63),
            Some("GF 63mm f/2.8 R WR".to_string())
        );

        // Test unknown lens
        assert_eq!(parser.lookup_lens(65000), None);
    }

    #[test]
    fn test_decode_off_on() {
        assert_eq!(DECODE_OFF_ON.decode(0), "Off");
        assert_eq!(DECODE_OFF_ON.decode(1), "On");
        assert_eq!(DECODE_OFF_ON.decode(2), "Unknown (2)");
    }

    #[test]
    fn test_decode_drive_mode() {
        assert_eq!(DECODE_DRIVE_MODE.decode(0), "Single Frame");
        assert_eq!(DECODE_DRIVE_MODE.decode(1), "Continuous Low");
        assert_eq!(DECODE_DRIVE_MODE.decode(2), "Continuous High");
        assert_eq!(DECODE_DRIVE_MODE.decode(4), "Self-timer");
    }

    #[test]
    fn test_decode_exr_mode() {
        assert_eq!(DECODE_EXR_MODE.decode(256), "HR (High Resolution)");
        assert_eq!(DECODE_EXR_MODE.decode(512), "SN (Signal-to-Noise Priority)");
        assert_eq!(DECODE_EXR_MODE.decode(768), "DR (Dynamic Range Priority)");
    }
}
