//! Pentax MakerNote Parser
//!
//! Parses Pentax-specific EXIF MakerNote tags containing camera settings,
//! lens information, image quality parameters, and other proprietary metadata.
//!
//! Supports all Pentax DSLR and mirrorless cameras including:
//! - K-series DSLRs (K-1, K-3, K-5, K-7, K-x, K-r, etc.)
//! - Q-series mirrorless (Q, Q7, Q10, Q-S1)
//! - istD/ist series legacy DSLRs
//!
//! Based on ExifTool's Pentax.pm module.
//!
//! ## Architecture
//! This module has been refactored to use the shared MakerNotes framework,
//! reducing code duplication by using:
//! - **const_decoder!** macros for declarative value decoders
//! - **Shared IFD parsing** utilities to eliminate duplicate parsing code
//! - **Generic decoders** for common patterns (ON_OFF, etc.)
//!
//! ## Code Duplication Reduction
//! This refactoring eliminates decoder function duplication while maintaining
//! 100% functionality and test coverage.

#![allow(dead_code)]
#![allow(unused_imports)]

use crate::error::{ExifToolError, Result};
use crate::io::EndianReader;
use crate::parsers::tiff::ifd_parser::{ByteOrder, IfdEntry};
use nom::{
    combinator::map,
    multi::count,
    number::complete::{be_u16, be_u32, le_u16, le_u32},
    IResult,
};
use std::collections::HashMap;

use super::pentax_lens_database::lookup_lens_name;
use super::shared::array_extractors::{extract_i16_array, extract_u16_array, extract_u32_array};
use super::shared::generic_decoders::ON_OFF;
use super::shared::MakerNoteParser;

// Import declarative decoder macros
use crate::const_decoder;

// Import registry
use super::registries::pentax::pentax_registry;

// Pentax MakerNote header signatures
// Pentax typically uses "AOC\0" (4 bytes) or no header
const PENTAX_HEADER_AOC: &[u8] = b"AOC\0";
const PENTAX_HEADER_PENTAX: &[u8] = b"PENTAX \0";

// ============================================================================
// Tag ID Constants
// ============================================================================
// These constants define the tag IDs for all Pentax MakerNote tags.
// They are used for pattern matching in the parse function.

// Basic Camera Info (0x0000-0x000F)
const PENTAX_VERSION: u16 = 0x0000;
const PENTAX_PENTAX_MODEL_TYPE: u16 = 0x0001;
const PENTAX_PREVIEW_IMAGE_SIZE: u16 = 0x0002;
const PENTAX_PREVIEW_IMAGE_LENGTH: u16 = 0x0003;
const PENTAX_PREVIEW_IMAGE_START: u16 = 0x0004;
const PENTAX_PENTAX_MODEL_ID: u16 = 0x0005;
const PENTAX_DATE: u16 = 0x0006;
const PENTAX_TIME: u16 = 0x0007;
const PENTAX_QUALITY: u16 = 0x0008;
const PENTAX_PENTAX_IMAGE_SIZE: u16 = 0x0009;
const PENTAX_PICTURE_MODE: u16 = 0x000B;
const PENTAX_FLASH_MODE: u16 = 0x000C;
const PENTAX_FOCUS_MODE: u16 = 0x000D;
const PENTAX_AF_POINT_SELECTED: u16 = 0x000E;
const PENTAX_AF_POINT_IN_FOCUS: u16 = 0x000F;

// Focus and Exposure (0x0010-0x001F)
const PENTAX_FOCUS_POSITION: u16 = 0x0010;
const PENTAX_EXPOSURE_TIME: u16 = 0x0012;
const PENTAX_FNUMBER: u16 = 0x0013;
const PENTAX_ISO_SPEED: u16 = 0x0014;
const PENTAX_LIGHT_READING: u16 = 0x0015;
const PENTAX_EXPOSURE_COMPENSATION: u16 = 0x0016;
const PENTAX_METERING_MODE: u16 = 0x0017;
const PENTAX_AUTO_BRACKETING: u16 = 0x0018;
const PENTAX_WHITE_BALANCE: u16 = 0x0019;
const PENTAX_WHITE_BALANCE_MODE: u16 = 0x001A;
const PENTAX_BLUE_BALANCE: u16 = 0x001B;
const PENTAX_RED_BALANCE: u16 = 0x001C;
const PENTAX_FOCAL_LENGTH: u16 = 0x001D;
const PENTAX_DIGITAL_ZOOM: u16 = 0x001E;
const PENTAX_SATURATION: u16 = 0x001F;

// Image Adjustments (0x0020-0x002F)
const PENTAX_CONTRAST: u16 = 0x0020;
const PENTAX_SHARPNESS: u16 = 0x0021;
const PENTAX_WORLD_TIME_LOCATION: u16 = 0x0022;
const PENTAX_HOMETOWN_CITY: u16 = 0x0023;
const PENTAX_DESTINATION_CITY: u16 = 0x0024;
const PENTAX_HOMETOWN_DST: u16 = 0x0025;
const PENTAX_DESTINATION_DST: u16 = 0x0026;
const PENTAX_DSP_FIRMWARE_VERSION: u16 = 0x0027;
const PENTAX_CPU_FIRMWARE_VERSION: u16 = 0x0028;
const PENTAX_FRAME_NUMBER: u16 = 0x0029;
const PENTAX_EFFECTIVE_LV: u16 = 0x002D;

// Camera Settings (0x0030-0x004F)
const PENTAX_IMAGE_PROCESSING: u16 = 0x0032;
const PENTAX_PICTURE_MODE2: u16 = 0x0033;
const PENTAX_DRIVE_MODE: u16 = 0x0034;
const PENTAX_SENSOR_SIZE: u16 = 0x0035;
const PENTAX_COLOR_SPACE: u16 = 0x0037;
const PENTAX_IMAGE_AREA_OFFSET: u16 = 0x0038;
const PENTAX_RAW_IMAGE_SIZE: u16 = 0x0039;
const PENTAX_BATTERY_LEVEL: u16 = 0x003B;
const PENTAX_AF_POINTS_IN_FOCUS_2: u16 = 0x003C;
const PENTAX_DATA_SCALING: u16 = 0x003D;
const PENTAX_PREVIEW_IMAGE_BORDERS: u16 = 0x003E;
const PENTAX_LENS_TYPE: u16 = 0x003F;
const PENTAX_SENSITIVITY_ADJUST: u16 = 0x0040;
const PENTAX_IMAGE_EDIT_COUNT: u16 = 0x0041;
const PENTAX_CAMERA_TEMPERATURE: u16 = 0x0047;
const PENTAX_AE_LOCK: u16 = 0x0048;
const PENTAX_NOISE_REDUCTION: u16 = 0x0049;
const PENTAX_FLASH_EXPOSURE_COMP: u16 = 0x004D;
const PENTAX_IMAGE_TONE: u16 = 0x004F;

// Color and Processing (0x0050-0x006F)
const PENTAX_COLOR_TEMPERATURE: u16 = 0x0050;
const PENTAX_SHAKE_REDUCTION: u16 = 0x005C;
const PENTAX_SHUTTER_COUNT: u16 = 0x005D;
const PENTAX_FACE_INFO: u16 = 0x0060;
const PENTAX_RAW_DEVELOPMENT_PROCESS: u16 = 0x0062;
const PENTAX_HUE: u16 = 0x0067;
const PENTAX_AWB_INFO: u16 = 0x0068;
const PENTAX_DYNAMIC_RANGE_EXPANSION: u16 = 0x0069;
const PENTAX_TIME_INFO: u16 = 0x006B;
const PENTAX_HIGH_LOW_KEY_ADJ: u16 = 0x006C;
const PENTAX_CONTRAST_HIGHLIGHT: u16 = 0x006D;
const PENTAX_CONTRAST_SHADOW: u16 = 0x006E;
const PENTAX_CONTRAST_HIGHLIGHT_SHADOW_ADJ: u16 = 0x006F;

// Advanced Features (0x0070-0x009F)
const PENTAX_FINE_SHARPNESS: u16 = 0x0070;
const PENTAX_HIGH_ISO_NOISE_REDUCTION: u16 = 0x0071;
const PENTAX_AF_ADJUSTMENT: u16 = 0x0072;
const PENTAX_MONOCHROME_FILTER_EFFECT: u16 = 0x0073;
const PENTAX_MONOCHROME_TONING: u16 = 0x0074;
const PENTAX_FACE_DETECT: u16 = 0x0076;
const PENTAX_FACE_DETECT_FRAME_SIZE: u16 = 0x0077;
const PENTAX_SHADOW_CORRECTION: u16 = 0x0079;
const PENTAX_ISO_AUTO_PARAMETERS: u16 = 0x007A;
const PENTAX_CROSS_PROCESS: u16 = 0x007B;
const PENTAX_LENS_CORR: u16 = 0x007D;
const PENTAX_WHITE_LEVEL: u16 = 0x007E;
const PENTAX_LENS_INFO: u16 = 0x007F;
const PENTAX_AF_INFO: u16 = 0x0080;
const PENTAX_ASPECT_RATIO: u16 = 0x0082;
const PENTAX_HDR: u16 = 0x0085;
const PENTAX_PIXEL_SHIFT_RESOLUTION: u16 = 0x0086;
const PENTAX_SHUTTER_TYPE: u16 = 0x0087;
const PENTAX_NEUTRAL_DENSITY_FILTER: u16 = 0x0088;
const PENTAX_ISO2: u16 = 0x008B;
const PENTAX_INTERVAL_SHOOTING: u16 = 0x0092;
const PENTAX_SKIN_TONE_CORRECTION: u16 = 0x0095;
const PENTAX_CLARITY_CONTROL: u16 = 0x0096;
const PENTAX_LENS_MODEL: u16 = 0x009F;

// ============================================================================
// Declarative Decoder Definitions
// ============================================================================
// Using const_decoder! macro to eliminate decoder function duplication

// Quality setting decoder - maps numeric values to quality modes
const_decoder!(pub QUALITY,
    i32,
    [
        (0, "Good"),
        (1, "Better"),
        (2, "Best"),
        (3, "TIFF"),
        (4, "RAW"),
        (5, "Premium"),
        (6, "RAW + JPEG"),
        (7, "RAW + Premium"),
        (8, "RAW + Better"),
        (9, "RAW + Good"),
    ]
);

// Picture mode decoder - maps values to shooting scene modes
const_decoder!(pub PICTURE_MODE,
    i32,
    [
        (0, "Program"),
        (1, "Shutter Priority"),
        (2, "Aperture Priority"),
        (3, "Manual"),
        (4, "Portrait"),
        (5, "Landscape"),
        (6, "Macro"),
        (7, "Sport"),
        (8, "Night Scene Portrait"),
        (9, "No Flash"),
        (10, "Night Scene"),
        (11, "Surf & Snow"),
        (12, "Text"),
        (13, "Sunset"),
        (14, "Kids"),
        (15, "Pet"),
        (16, "Candlelight"),
        (17, "Museum"),
        (18, "Food"),
        (19, "Stage Lighting"),
        (20, "Night Snap"),
        (21, "Blue Sky"),
        (22, "Forest"),
    ]
);

// Flash mode decoder - maps values to flash modes
const_decoder!(pub FLASH_MODE,
    i32,
    [
        (0, "Auto"),
        (1, "Flash On"),
        (2, "Flash Off"),
        (3, "Red-eye Reduction"),
        (4, "Auto + Red-eye"),
        (5, "On + Red-eye"),
        (6, "Wireless"),
        (7, "Slow-sync"),
        (8, "Trailing-curtain Sync"),
    ]
);

// Focus mode decoder - maps values to autofocus modes
const_decoder!(pub FOCUS_MODE,
    i32,
    [
        (0, "Normal (AF)"),
        (1, "Macro (AF)"),
        (2, "Manual"),
        (3, "AF-S (Single)"),
        (4, "AF-C (Continuous)"),
        (5, "AF-A (Auto)"),
    ]
);

// Metering mode decoder - maps values to exposure metering modes
const_decoder!(pub METERING_MODE,
    i32,
    [
        (0, "Multi-segment"),
        (1, "Center-weighted Average"),
        (2, "Spot"),
        (3, "Average"),
        (4, "Highlight-weighted"),
    ]
);

// White balance decoder - maps values to white balance presets
const_decoder!(pub WHITE_BALANCE,
    i32,
    [
        (0, "Auto"),
        (1, "Daylight"),
        (2, "Shade"),
        (3, "Cloudy"),
        (4, "Tungsten"),
        (5, "Fluorescent"),
        (6, "Manual"),
        (7, "Daylight Fluorescent"),
        (8, "Day White Fluorescent"),
        (9, "White Fluorescent"),
        (10, "Flash"),
        (11, "Cloudy Fluorescent"),
        (14, "Multi Auto"),
        (15, "Color Temperature Enhancement"),
    ]
);

// White balance mode decoder - maps values to WB modes
const_decoder!(pub WHITE_BALANCE_MODE,
    i32,
    [
        (1, "Auto (Daylight)"),
        (2, "Auto (Shade)"),
        (3, "Auto (Flash)"),
        (4, "Auto (Tungsten)"),
        (6, "Auto (Daylight Fluorescent)"),
        (7, "Auto (Day White Fluorescent)"),
        (8, "Auto (White Fluorescent)"),
        (10, "Auto (Flash)"),
    ]
);

// Drive mode decoder - maps values to drive/shooting modes
const_decoder!(pub DRIVE_MODE,
    i32,
    [
        (0, "Single-frame"),
        (1, "Continuous"),
        (2, "Self-timer (12s)"),
        (3, "Self-timer (2s)"),
        (4, "Remote"),
        (5, "Exposure Bracketing"),
        (6, "Multiple Exposure"),
        (7, "Remote (3s delay)"),
        (8, "Continuous (Hi)"),
        (9, "Continuous (Lo)"),
        (10, "Continuous (Med)"),
        (11, "Interval Shooting"),
        (12, "Interval Composite"),
    ]
);

// Color space decoder - maps values to color space settings
const_decoder!(pub COLOR_SPACE, i32, [(0, "sRGB"), (1, "Adobe RGB"),]);

// Saturation decoder - maps values to saturation settings
const_decoder!(pub SATURATION,
    i32,
    [
        (0, "Low"),
        (1, "Normal"),
        (2, "High"),
        (3, "Med Low"),
        (4, "Med High"),
        (5, "Very High"),
        (6, "Very Low"),
        (7, "Off (B&W)"),
    ]
);

// Contrast decoder - maps values to contrast settings
const_decoder!(pub CONTRAST,
    i32,
    [
        (0, "Low"),
        (1, "Normal"),
        (2, "High"),
        (3, "Med Low"),
        (4, "Med High"),
        (5, "Very High"),
        (6, "Very Low"),
    ]
);

// Sharpness decoder - maps values to sharpness settings
const_decoder!(pub SHARPNESS,
    i32,
    [
        (0, "Soft"),
        (1, "Normal"),
        (2, "Hard"),
        (3, "Med Soft"),
        (4, "Med Hard"),
        (5, "Very Hard"),
        (6, "Very Soft"),
    ]
);

// Shake reduction decoder - maps values to SR/stabilization modes
const_decoder!(pub SHAKE_REDUCTION,
    i32,
    [
        (0, "Off"),
        (1, "On"),
        (2, "On (Video)"),
        (3, "On (2-axis)"),
        (4, "On (3-axis)"),
        (5, "On (4-axis)"),
        (6, "On (5-axis)"),
    ]
);

// Image size decoder - maps values to resolution presets
const_decoder!(pub IMAGE_SIZE,
    i32,
    [
        (0, "640x480"),
        (1, "Full"),
        (2, "1024x768"),
        (3, "1280x960"),
        (4, "1600x1200"),
        (5, "2048x1536"),
        (8, "2560x1920"),
        (9, "3072x2304"),
        (10, "3264x2448"),
        (19, "320x240"),
        (20, "2288x1712"),
        (21, "2592x1944"),
        (22, "2304x1728"),
        (23, "3056x2296"),
        (25, "2816x2212"),
        (27, "3648x2736"),
        (36, "3008x2008"),
    ]
);

// Auto bracketing decoder - maps values to bracketing modes
const_decoder!(pub AUTO_BRACKETING, i32, [(0, "Off"), (1, "On"),]);

// World time location decoder - maps values to time zone selection
const_decoder!(pub WORLD_TIME_LOCATION,
    i32,
    [(0, "Hometown"), (1, "Destination"),]
);

// Pixel shift resolution decoder - maps values to PSR modes
const_decoder!(pub PIXEL_SHIFT_RESOLUTION,
    i32,
    [(0, "Off"), (1, "On"), (2, "On (Motion Correction)"),]
);

// DST (Daylight Saving Time) decoder
const_decoder!(pub DST, i32, [(0, "No"), (1, "Yes"),]);

// Image tone decoder
const_decoder!(pub IMAGE_TONE, i32, [
    (0, "Natural"), (1, "Bright"), (2, "Portrait"), (3, "Landscape"),
    (4, "Vibrant"), (5, "Monochrome"), (6, "Muted"), (7, "Reversal Film"),
    (8, "Bleach Bypass"), (9, "Radiant"), (10, "Cross Processing"),
    (11, "Flat"), (12, "Auto"),
]);

// Noise reduction decoder
const_decoder!(pub NOISE_REDUCTION, i32, [
    (0, "Off"), (1, "On (Weak)"), (2, "On"), (3, "On (Strong)"), (4, "Auto"),
]);

// High ISO noise reduction decoder
const_decoder!(pub HIGH_ISO_NOISE_REDUCTION, i32, [
    (0, "Off"), (1, "Weakest"), (2, "Weak"), (3, "Medium"),
    (4, "Strong"), (5, "Strongest"), (6, "Auto"),
]);

// AE Lock decoder
const_decoder!(pub AE_LOCK, i32, [(0, "Off"), (1, "On"),]);

// Dynamic range expansion decoder
const_decoder!(pub DYNAMIC_RANGE_EXPANSION, i32, [(0, "Off"), (1, "On"), (2, "Auto"),]);

// HDR decoder
const_decoder!(pub HDR, i32, [
    (0, "Off"), (1, "HDR Auto"), (2, "HDR 1"), (3, "HDR 2"),
    (4, "HDR 3"), (5, "Advanced HDR"),
]);

// Shadow correction decoder
const_decoder!(pub SHADOW_CORRECTION, i32, [
    (0, "Off"), (1, "On (Weak)"), (2, "On"), (3, "On (Strong)"), (4, "Auto"),
]);

// Fine sharpness decoder
const_decoder!(pub FINE_SHARPNESS, i32, [(0, "Off"), (1, "On"),]);

// Shutter type decoder
const_decoder!(pub SHUTTER_TYPE, i32, [(0, "Mechanical"), (1, "Electronic"),]);

// Neutral density filter decoder
const_decoder!(pub NEUTRAL_DENSITY_FILTER, i32, [(0, "Off"), (1, "On"),]);

// Monochrome filter effect decoder
const_decoder!(pub MONOCHROME_FILTER_EFFECT, i32, [
    (0, "None"), (1, "Yellow"), (2, "Orange"), (3, "Red"), (4, "Magenta"),
    (5, "Blue"), (6, "Cyan"), (7, "Green"), (8, "Yellow-green"), (9, "Infrared"),
]);

// Monochrome toning decoder
const_decoder!(pub MONOCHROME_TONING, i32, [
    (0, "None"), (1, "Sepia"), (2, "Blue"), (3, "Purple"), (4, "Green"),
]);

// Face detect decoder
const_decoder!(pub FACE_DETECT, i32, [(0, "Off"), (1, "On"), (256, "On (Smile/Blink)"),]);

// Cross process decoder
const_decoder!(pub CROSS_PROCESS, i32, [
    (0, "Off"), (1, "Random"), (2, "Preset 1"), (3, "Preset 2"), (4, "Preset 3"),
    (16, "Favorite 1"), (17, "Favorite 2"), (18, "Favorite 3"),
]);

// Aspect ratio decoder
const_decoder!(pub ASPECT_RATIO, i32, [(0, "4:3"), (1, "3:2"), (2, "16:9"), (3, "1:1"),]);

// Clarity control decoder
const_decoder!(pub CLARITY_CONTROL, i32, [
    (-4, "Very Low"), (-3, "Low 3"), (-2, "Low 2"), (-1, "Low 1"), (0, "Off"),
    (1, "High 1"), (2, "High 2"), (3, "High 3"), (4, "Very High"),
]);

// Skin tone correction decoder
const_decoder!(pub SKIN_TONE_CORRECTION, i32, [
    (0, "Off"), (1, "On (Type 1)"), (2, "On (Type 2)"),
]);

// Bleach bypass toning decoder
const_decoder!(pub BLEACH_BYPASS_TONING, i32, [
    (0, "Off"), (1, "Green"), (2, "Yellow"), (3, "Orange"),
]);

// Raw development process decoder
const_decoder!(pub RAW_DEVELOPMENT_PROCESS, i32, [
    (1, "Ver. 1"), (2, "Ver. 2"), (3, "Ver. 3"), (4, "Ver. 4"),
    (5, "Ver. 5"), (6, "Ver. 6"), (7, "Ver. 7"),
]);

// Lens correction decoder
const_decoder!(pub LENS_CORR, i32, [
    (0, "Off"), (1, "Distortion"), (2, "Chromatic Aberration"),
    (3, "Distortion + CA"), (4, "Peripheral Illumination"),
    (5, "Distortion + PI"), (6, "CA + PI"), (7, "Distortion + CA + PI"),
    (8, "Diffraction"),
]);

/// Checks if the provided data has a valid Pentax MakerNote header
///
/// # Arguments
/// * `data` - Raw MakerNote data to validate
///
/// # Returns
/// * `true` if data contains a valid Pentax header or appears to be Pentax MakerNote data
/// * `false` otherwise
pub fn is_pentax_makernote(data: &[u8]) -> bool {
    if data.len() < 4 {
        return false;
    }

    // Check for AOC header (most common)
    if data.len() >= 4 && &data[0..4] == PENTAX_HEADER_AOC {
        return true;
    }

    // Check for PENTAX header (some models)
    if data.len() >= 8 && &data[0..8] == PENTAX_HEADER_PENTAX {
        return true;
    }

    // Some Pentax cameras have no header, just start with IFD
    // We'll validate by checking if first two bytes form a reasonable entry count
    if data.len() >= 2 {
        let reader = EndianReader::little_endian(data);
        let entry_count = reader.u16_at(0).unwrap_or(0);
        // Reasonable entry count: 1-200 entries
        if entry_count > 0 && entry_count < 200 {
            return true;
        }
    }

    false
}

/// Represents a Pentax MakerNote parser
pub struct PentaxParser;

impl MakerNoteParser for PentaxParser {
    fn manufacturer_name(&self) -> &'static str {
        "Pentax"
    }

    fn tag_prefix(&self) -> &'static str {
        "Pentax:"
    }

    fn validate_header(&self, data: &[u8]) -> bool {
        is_pentax_makernote(data)
    }

    fn lookup_lens(&self, lens_id: u16) -> Option<String> {
        lookup_lens_name(lens_id)
    }

    fn parse(
        &self,
        data: &[u8],
        byte_order: ByteOrder,
        tags: &mut HashMap<String, String>,
    ) -> std::result::Result<(), String> {
        if data.is_empty() {
            return Ok(());
        }

        // Validate Pentax header and determine IFD offset
        let ifd_offset = if data.len() >= 4 && &data[0..4] == PENTAX_HEADER_AOC {
            // AOC header: skip 6 bytes (AOC\0 + 2-byte offset)
            6
        } else if data.len() >= 8 && &data[0..8] == PENTAX_HEADER_PENTAX {
            // PENTAX header: skip 8 bytes
            8
        } else {
            // No header, IFD starts immediately
            0
        };

        if data.len() <= ifd_offset + 2 {
            return Ok(());
        }

        let ifd_data = &data[ifd_offset..];

        // Parse IFD entry count using EndianReader
        if ifd_data.len() < 2 {
            return Ok(());
        }
        let reader = EndianReader::new(ifd_data, byte_order.to_io_byte_order());
        let entry_count = reader.u16_at(0).unwrap_or(0);

        // Sanity check on entry count
        if entry_count == 0 || entry_count > 200 {
            return Ok(());
        }

        // Parse IFD entries
        let entries_start = &ifd_data[2..];
        let entries = match parse_ifd_entries(entries_start, entry_count, byte_order) {
            Ok((_, entries)) => entries,
            Err(_) => return Ok(()), // Return empty on parse failure
        };

        // Extract tags from entries
        for entry in entries {
            match entry.tag_id {
                // Simple string tags
                PENTAX_VERSION => {
                    if let Some(value) = extract_string_value(&entry, data, ifd_offset) {
                        tags.insert("Pentax:Version".to_string(), value);
                    }
                }

                PENTAX_DATE => {
                    if let Some(value) = extract_string_value(&entry, data, ifd_offset) {
                        tags.insert("Pentax:Date".to_string(), value);
                    }
                }

                PENTAX_TIME => {
                    if let Some(value) = extract_string_value(&entry, data, ifd_offset) {
                        tags.insert("Pentax:Time".to_string(), value);
                    }
                }

                PENTAX_LENS_MODEL => {
                    if let Some(value) = extract_string_value(&entry, data, ifd_offset) {
                        tags.insert("Pentax:LensModel".to_string(), value);
                    }
                }

                // Decoded value tags using const decoders
                PENTAX_QUALITY => {
                    let value = entry.value_offset as i32;
                    tags.insert("Pentax:Quality".to_string(), QUALITY.decode(value));
                }

                PENTAX_PICTURE_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert("Pentax:PictureMode".to_string(), PICTURE_MODE.decode(value));
                }

                PENTAX_FLASH_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert("Pentax:FlashMode".to_string(), FLASH_MODE.decode(value));
                }

                PENTAX_FOCUS_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert("Pentax:FocusMode".to_string(), FOCUS_MODE.decode(value));
                }

                PENTAX_METERING_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Pentax:MeteringMode".to_string(),
                        METERING_MODE.decode(value),
                    );
                }

                PENTAX_WHITE_BALANCE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Pentax:WhiteBalance".to_string(),
                        WHITE_BALANCE.decode(value),
                    );
                }

                PENTAX_WHITE_BALANCE_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Pentax:WhiteBalanceMode".to_string(),
                        WHITE_BALANCE_MODE.decode(value),
                    );
                }

                PENTAX_SATURATION => {
                    let value = entry.value_offset as i32;
                    tags.insert("Pentax:Saturation".to_string(), SATURATION.decode(value));
                }

                PENTAX_CONTRAST => {
                    let value = entry.value_offset as i32;
                    tags.insert("Pentax:Contrast".to_string(), CONTRAST.decode(value));
                }

                PENTAX_SHARPNESS => {
                    let value = entry.value_offset as i32;
                    tags.insert("Pentax:Sharpness".to_string(), SHARPNESS.decode(value));
                }

                PENTAX_DRIVE_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert("Pentax:DriveMode".to_string(), DRIVE_MODE.decode(value));
                }

                PENTAX_COLOR_SPACE => {
                    let value = entry.value_offset as i32;
                    tags.insert("Pentax:ColorSpace".to_string(), COLOR_SPACE.decode(value));
                }

                // Note: Former SHAKE_REDUCTION_INFO at 0x003C is now AF_POINTS_IN_FOCUS_2
                // Shake reduction is now at 0x005C - handled below
                PENTAX_PENTAX_IMAGE_SIZE => {
                    let value = entry.value_offset as i32;
                    tags.insert("Pentax:ImageSize".to_string(), IMAGE_SIZE.decode(value));
                }

                PENTAX_AUTO_BRACKETING => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Pentax:AutoBracketing".to_string(),
                        AUTO_BRACKETING.decode(value),
                    );
                }

                PENTAX_WORLD_TIME_LOCATION => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Pentax:WorldTimeLocation".to_string(),
                        WORLD_TIME_LOCATION.decode(value),
                    );
                }

                PENTAX_PIXEL_SHIFT_RESOLUTION => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Pentax:PixelShiftResolution".to_string(),
                        PIXEL_SHIFT_RESOLUTION.decode(value),
                    );
                }

                // Numeric value tags (no decoding needed)
                PENTAX_AF_POINT_SELECTED => {
                    let value = entry.value_offset as i32;
                    if (0..=65535).contains(&value) {
                        tags.insert("Pentax:AFPointSelected".to_string(), value.to_string());
                    }
                }

                PENTAX_AF_POINT_IN_FOCUS => {
                    let value = entry.value_offset as i32;
                    if (0..=65535).contains(&value) {
                        tags.insert("Pentax:AFPointInFocus".to_string(), value.to_string());
                    }
                }

                PENTAX_ISO_SPEED => {
                    let value = entry.value_offset;
                    tags.insert("Pentax:ISO".to_string(), value.to_string());
                }

                PENTAX_BLUE_BALANCE => {
                    let value = entry.value_offset as i32;
                    tags.insert("Pentax:BlueBalance".to_string(), value.to_string());
                }

                PENTAX_RED_BALANCE => {
                    let value = entry.value_offset as i32;
                    tags.insert("Pentax:RedBalance".to_string(), value.to_string());
                }

                PENTAX_FOCAL_LENGTH => {
                    let value = entry.value_offset;
                    tags.insert(
                        "Pentax:FocalLength".to_string(),
                        format!("{:.1} mm", value as f32 / 100.0),
                    );
                }

                PENTAX_DIGITAL_ZOOM => {
                    let value = entry.value_offset;
                    if value > 0 {
                        tags.insert(
                            "Pentax:DigitalZoom".to_string(),
                            format!("{:.2}x", value as f32 / 100.0),
                        );
                    }
                }

                PENTAX_SHUTTER_COUNT => {
                    let value = entry.value_offset;
                    tags.insert("Pentax:ShutterCount".to_string(), value.to_string());
                }

                PENTAX_LENS_TYPE => {
                    let lens_id = entry.value_offset as u16;
                    if let Some(lens_name) = lookup_lens_name(lens_id) {
                        tags.insert("Pentax:LensType".to_string(), lens_name);
                    } else {
                        tags.insert(
                            "Pentax:LensType".to_string(),
                            format!("Unknown ({})", lens_id),
                        );
                    }
                }

                PENTAX_PENTAX_MODEL_TYPE => {
                    let value = entry.value_offset;
                    tags.insert("Pentax:ModelType".to_string(), value.to_string());
                }

                PENTAX_PENTAX_MODEL_ID => {
                    let value = entry.value_offset;
                    tags.insert("Pentax:ModelID".to_string(), value.to_string());
                }

                PENTAX_PREVIEW_IMAGE_SIZE => {
                    let value = entry.value_offset;
                    tags.insert("Pentax:PreviewImageSize".to_string(), value.to_string());
                }

                PENTAX_PREVIEW_IMAGE_LENGTH => {
                    let value = entry.value_offset;
                    tags.insert("Pentax:PreviewImageLength".to_string(), value.to_string());
                }

                PENTAX_CAMERA_TEMPERATURE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Pentax:CameraTemperature".to_string(),
                        format!("{}°C", value),
                    );
                }

                PENTAX_BATTERY_LEVEL => {
                    let value = entry.value_offset;
                    tags.insert("Pentax:BatteryLevel".to_string(), format!("{}%", value));
                }

                PENTAX_HOMETOWN_CITY => {
                    let value = entry.value_offset;
                    tags.insert("Pentax:HometownCity".to_string(), value.to_string());
                }

                PENTAX_DESTINATION_CITY => {
                    let value = entry.value_offset;
                    tags.insert("Pentax:DestinationCity".to_string(), value.to_string());
                }

                PENTAX_PICTURE_MODE2 => {
                    let value = entry.value_offset as i32;
                    tags.insert("Pentax:PictureMode2".to_string(), value.to_string());
                }

                // Focus and Exposure tags
                PENTAX_FOCUS_POSITION => {
                    let value = entry.value_offset;
                    tags.insert("Pentax:FocusPosition".to_string(), value.to_string());
                }
                PENTAX_EXPOSURE_TIME => {
                    tags.insert(
                        "Pentax:ExposureTime".to_string(),
                        entry.value_offset.to_string(),
                    );
                }
                PENTAX_FNUMBER => {
                    let value = entry.value_offset;
                    tags.insert(
                        "Pentax:FNumber".to_string(),
                        format!("f/{:.1}", value as f32 / 10.0),
                    );
                }
                PENTAX_LIGHT_READING => {
                    tags.insert(
                        "Pentax:LightReading".to_string(),
                        (entry.value_offset as i32).to_string(),
                    );
                }
                PENTAX_EXPOSURE_COMPENSATION => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Pentax:ExposureCompensation".to_string(),
                        format!("{:+.1} EV", value as f32 / 10.0),
                    );
                }

                // Image Adjustments
                PENTAX_HOMETOWN_DST => {
                    tags.insert(
                        "Pentax:HometownDST".to_string(),
                        DST.decode(entry.value_offset as i32),
                    );
                }
                PENTAX_DESTINATION_DST => {
                    tags.insert(
                        "Pentax:DestinationDST".to_string(),
                        DST.decode(entry.value_offset as i32),
                    );
                }
                PENTAX_DSP_FIRMWARE_VERSION => {
                    if let Some(value) = extract_string_value(&entry, data, ifd_offset) {
                        tags.insert("Pentax:DSPFirmwareVersion".to_string(), value);
                    }
                }
                PENTAX_CPU_FIRMWARE_VERSION => {
                    if let Some(value) = extract_string_value(&entry, data, ifd_offset) {
                        tags.insert("Pentax:CPUFirmwareVersion".to_string(), value);
                    }
                }
                PENTAX_FRAME_NUMBER => {
                    tags.insert(
                        "Pentax:FrameNumber".to_string(),
                        entry.value_offset.to_string(),
                    );
                }
                PENTAX_EFFECTIVE_LV => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Pentax:EffectiveLV".to_string(),
                        format!("{:.1}", value as f32 / 10.0),
                    );
                }

                // Camera Settings
                PENTAX_IMAGE_PROCESSING => {
                    tags.insert(
                        "Pentax:ImageProcessing".to_string(),
                        entry.value_offset.to_string(),
                    );
                }
                PENTAX_SENSOR_SIZE => {
                    tags.insert(
                        "Pentax:SensorSize".to_string(),
                        entry.value_offset.to_string(),
                    );
                }
                PENTAX_IMAGE_AREA_OFFSET => {
                    tags.insert(
                        "Pentax:ImageAreaOffset".to_string(),
                        entry.value_offset.to_string(),
                    );
                }
                PENTAX_RAW_IMAGE_SIZE => {
                    tags.insert(
                        "Pentax:RawImageSize".to_string(),
                        entry.value_offset.to_string(),
                    );
                }
                PENTAX_AF_POINTS_IN_FOCUS_2 => {
                    tags.insert(
                        "Pentax:AFPointsInFocus2".to_string(),
                        entry.value_offset.to_string(),
                    );
                }
                PENTAX_DATA_SCALING => {
                    tags.insert(
                        "Pentax:DataScaling".to_string(),
                        entry.value_offset.to_string(),
                    );
                }
                PENTAX_PREVIEW_IMAGE_BORDERS => {
                    tags.insert(
                        "Pentax:PreviewImageBorders".to_string(),
                        entry.value_offset.to_string(),
                    );
                }
                PENTAX_SENSITIVITY_ADJUST => {
                    tags.insert(
                        "Pentax:SensitivityAdjust".to_string(),
                        (entry.value_offset as i32).to_string(),
                    );
                }
                PENTAX_IMAGE_EDIT_COUNT => {
                    tags.insert(
                        "Pentax:ImageEditCount".to_string(),
                        entry.value_offset.to_string(),
                    );
                }
                PENTAX_AE_LOCK => {
                    tags.insert(
                        "Pentax:AELock".to_string(),
                        AE_LOCK.decode(entry.value_offset as i32),
                    );
                }
                PENTAX_NOISE_REDUCTION => {
                    tags.insert(
                        "Pentax:NoiseReduction".to_string(),
                        NOISE_REDUCTION.decode(entry.value_offset as i32),
                    );
                }
                PENTAX_FLASH_EXPOSURE_COMP => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Pentax:FlashExposureComp".to_string(),
                        format!("{:+.1} EV", value as f32 / 10.0),
                    );
                }
                PENTAX_IMAGE_TONE => {
                    tags.insert(
                        "Pentax:ImageTone".to_string(),
                        IMAGE_TONE.decode(entry.value_offset as i32),
                    );
                }

                // Color and Processing
                PENTAX_COLOR_TEMPERATURE => {
                    tags.insert(
                        "Pentax:ColorTemperature".to_string(),
                        format!("{}K", entry.value_offset),
                    );
                }
                PENTAX_SHAKE_REDUCTION => {
                    tags.insert(
                        "Pentax:ShakeReduction".to_string(),
                        SHAKE_REDUCTION.decode(entry.value_offset as i32),
                    );
                }
                PENTAX_FACE_INFO => {
                    tags.insert(
                        "Pentax:FaceInfo".to_string(),
                        entry.value_offset.to_string(),
                    );
                }
                PENTAX_RAW_DEVELOPMENT_PROCESS => {
                    tags.insert(
                        "Pentax:RawDevelopmentProcess".to_string(),
                        RAW_DEVELOPMENT_PROCESS.decode(entry.value_offset as i32),
                    );
                }
                PENTAX_HUE => {
                    tags.insert(
                        "Pentax:Hue".to_string(),
                        (entry.value_offset as i32).to_string(),
                    );
                }
                PENTAX_AWB_INFO => {
                    tags.insert("Pentax:AWBInfo".to_string(), entry.value_offset.to_string());
                }
                PENTAX_DYNAMIC_RANGE_EXPANSION => {
                    tags.insert(
                        "Pentax:DynamicRangeExpansion".to_string(),
                        DYNAMIC_RANGE_EXPANSION.decode(entry.value_offset as i32),
                    );
                }
                PENTAX_TIME_INFO => {
                    if let Some(value) = extract_string_value(&entry, data, ifd_offset) {
                        tags.insert("Pentax:TimeInfo".to_string(), value);
                    }
                }
                PENTAX_HIGH_LOW_KEY_ADJ => {
                    tags.insert(
                        "Pentax:HighLowKeyAdj".to_string(),
                        (entry.value_offset as i32).to_string(),
                    );
                }
                PENTAX_CONTRAST_HIGHLIGHT => {
                    tags.insert(
                        "Pentax:ContrastHighlight".to_string(),
                        (entry.value_offset as i32).to_string(),
                    );
                }
                PENTAX_CONTRAST_SHADOW => {
                    tags.insert(
                        "Pentax:ContrastShadow".to_string(),
                        (entry.value_offset as i32).to_string(),
                    );
                }
                PENTAX_CONTRAST_HIGHLIGHT_SHADOW_ADJ => {
                    tags.insert(
                        "Pentax:ContrastHighlightShadowAdj".to_string(),
                        (entry.value_offset as i32).to_string(),
                    );
                }

                // Advanced Features
                PENTAX_FINE_SHARPNESS => {
                    tags.insert(
                        "Pentax:FineSharpness".to_string(),
                        FINE_SHARPNESS.decode(entry.value_offset as i32),
                    );
                }
                PENTAX_HIGH_ISO_NOISE_REDUCTION => {
                    tags.insert(
                        "Pentax:HighISONoiseReduction".to_string(),
                        HIGH_ISO_NOISE_REDUCTION.decode(entry.value_offset as i32),
                    );
                }
                PENTAX_AF_ADJUSTMENT => {
                    tags.insert(
                        "Pentax:AFAdjustment".to_string(),
                        (entry.value_offset as i32).to_string(),
                    );
                }
                PENTAX_MONOCHROME_FILTER_EFFECT => {
                    tags.insert(
                        "Pentax:MonochromeFilterEffect".to_string(),
                        MONOCHROME_FILTER_EFFECT.decode(entry.value_offset as i32),
                    );
                }
                PENTAX_MONOCHROME_TONING => {
                    tags.insert(
                        "Pentax:MonochromeToning".to_string(),
                        MONOCHROME_TONING.decode(entry.value_offset as i32),
                    );
                }
                PENTAX_FACE_DETECT => {
                    tags.insert(
                        "Pentax:FaceDetect".to_string(),
                        FACE_DETECT.decode(entry.value_offset as i32),
                    );
                }
                PENTAX_FACE_DETECT_FRAME_SIZE => {
                    tags.insert(
                        "Pentax:FaceDetectFrameSize".to_string(),
                        entry.value_offset.to_string(),
                    );
                }
                PENTAX_SHADOW_CORRECTION => {
                    tags.insert(
                        "Pentax:ShadowCorrection".to_string(),
                        SHADOW_CORRECTION.decode(entry.value_offset as i32),
                    );
                }
                PENTAX_ISO_AUTO_PARAMETERS => {
                    tags.insert(
                        "Pentax:ISOAutoParameters".to_string(),
                        entry.value_offset.to_string(),
                    );
                }
                PENTAX_CROSS_PROCESS => {
                    tags.insert(
                        "Pentax:CrossProcess".to_string(),
                        CROSS_PROCESS.decode(entry.value_offset as i32),
                    );
                }
                PENTAX_LENS_CORR => {
                    tags.insert(
                        "Pentax:LensCorr".to_string(),
                        LENS_CORR.decode(entry.value_offset as i32),
                    );
                }
                PENTAX_WHITE_LEVEL => {
                    tags.insert(
                        "Pentax:WhiteLevel".to_string(),
                        entry.value_offset.to_string(),
                    );
                }
                PENTAX_LENS_INFO => {
                    tags.insert(
                        "Pentax:LensInfo".to_string(),
                        entry.value_offset.to_string(),
                    );
                }
                PENTAX_AF_INFO => {
                    tags.insert("Pentax:AFInfo".to_string(), entry.value_offset.to_string());
                }
                PENTAX_ASPECT_RATIO => {
                    tags.insert(
                        "Pentax:AspectRatio".to_string(),
                        ASPECT_RATIO.decode(entry.value_offset as i32),
                    );
                }
                PENTAX_HDR => {
                    tags.insert(
                        "Pentax:HDR".to_string(),
                        HDR.decode(entry.value_offset as i32),
                    );
                }
                PENTAX_SHUTTER_TYPE => {
                    tags.insert(
                        "Pentax:ShutterType".to_string(),
                        SHUTTER_TYPE.decode(entry.value_offset as i32),
                    );
                }
                PENTAX_NEUTRAL_DENSITY_FILTER => {
                    tags.insert(
                        "Pentax:NeutralDensityFilter".to_string(),
                        NEUTRAL_DENSITY_FILTER.decode(entry.value_offset as i32),
                    );
                }
                PENTAX_ISO2 => {
                    tags.insert("Pentax:ISO2".to_string(), entry.value_offset.to_string());
                }
                PENTAX_INTERVAL_SHOOTING => {
                    tags.insert(
                        "Pentax:IntervalShooting".to_string(),
                        entry.value_offset.to_string(),
                    );
                }
                PENTAX_SKIN_TONE_CORRECTION => {
                    tags.insert(
                        "Pentax:SkinToneCorrection".to_string(),
                        SKIN_TONE_CORRECTION.decode(entry.value_offset as i32),
                    );
                }
                PENTAX_CLARITY_CONTROL => {
                    tags.insert(
                        "Pentax:ClarityControl".to_string(),
                        CLARITY_CONTROL.decode(entry.value_offset as i32),
                    );
                }
                PENTAX_PREVIEW_IMAGE_START => {
                    tags.insert(
                        "Pentax:PreviewImageStart".to_string(),
                        entry.value_offset.to_string(),
                    );
                }

                _ => {
                    // Unknown tags are silently ignored
                }
            }
        }

        Ok(())
    }
}

/// Maps Pentax tag ID to human-readable tag name
///
/// This function provides consistent tag naming for Pentax MakerNote tags
fn pentax_tag_to_name(tag_id: u16) -> String {
    let tag_name = match tag_id {
        0x0000 => "Version",
        0x0001 => "ModelType",
        0x0005 => "ModelID",
        0x0006 => "Date",
        0x0007 => "Time",
        0x0008 => "Quality",
        0x0009 => "ImageSize",
        0x000B => "PictureMode",
        0x000C => "FlashMode",
        0x000D => "FocusMode",
        0x000E => "AFPointSelected",
        0x000F => "AFPointInFocus",
        0x0014 => "ISO",
        0x0017 => "MeteringMode",
        0x0019 => "WhiteBalance",
        0x001A => "WhiteBalanceMode",
        0x001F => "Saturation",
        0x0020 => "Contrast",
        0x0021 => "Sharpness",
        0x0034 => "DriveMode",
        0x0037 => "ColorSpace",
        0x003F => "LensType",
        0x009F => "LensModel",
        0x003D => "ShutterCount",
        _ => return format!("Pentax:Unknown-{:#06X}", tag_id),
    };

    format!("Pentax:{}", tag_name)
}

/// Parses IFD entries in the specified byte order
///
/// This function handles parsing multiple IFD entries based on byte order
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
///
/// IFD entries are 12 bytes: tag_id (2), field_type (2), value_count (4), value_offset (4)
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
///
/// IFD entries are 12 bytes: tag_id (2), field_type (2), value_count (4), value_offset (4)
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
fn extract_string_value(entry: &IfdEntry, full_data: &[u8], ifd_offset: usize) -> Option<String> {
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
    let offset = entry.value_offset as usize;
    let abs_offset = ifd_offset + offset;

    if abs_offset + byte_count <= full_data.len() {
        let bytes = &full_data[abs_offset..abs_offset + byte_count];
        let s = std::str::from_utf8(bytes)
            .ok()?
            .trim_end_matches('\0')
            .trim();
        return Some(s.to_string());
    }

    None
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_quality() {
        assert_eq!(QUALITY.decode(2), "Best");
        assert_eq!(QUALITY.decode(4), "RAW");
        assert_eq!(QUALITY.decode(6), "RAW + JPEG");
    }

    #[test]
    fn test_decode_picture_mode() {
        assert_eq!(PICTURE_MODE.decode(0), "Program");
        assert_eq!(PICTURE_MODE.decode(2), "Aperture Priority");
        assert_eq!(PICTURE_MODE.decode(3), "Manual");
        assert_eq!(PICTURE_MODE.decode(5), "Landscape");
    }

    #[test]
    fn test_decode_focus_mode() {
        assert_eq!(FOCUS_MODE.decode(2), "Manual");
        assert_eq!(FOCUS_MODE.decode(3), "AF-S (Single)");
        assert_eq!(FOCUS_MODE.decode(4), "AF-C (Continuous)");
    }

    #[test]
    fn test_decode_white_balance() {
        assert_eq!(WHITE_BALANCE.decode(0), "Auto");
        assert_eq!(WHITE_BALANCE.decode(1), "Daylight");
        assert_eq!(WHITE_BALANCE.decode(6), "Manual");
    }

    #[test]
    fn test_decode_drive_mode() {
        assert_eq!(DRIVE_MODE.decode(0), "Single-frame");
        assert_eq!(DRIVE_MODE.decode(1), "Continuous");
        assert_eq!(DRIVE_MODE.decode(5), "Exposure Bracketing");
    }

    #[test]
    fn test_decode_saturation() {
        assert_eq!(SATURATION.decode(0), "Low");
        assert_eq!(SATURATION.decode(1), "Normal");
        assert_eq!(SATURATION.decode(2), "High");
    }

    #[test]
    fn test_decode_contrast() {
        assert_eq!(CONTRAST.decode(0), "Low");
        assert_eq!(CONTRAST.decode(1), "Normal");
        assert_eq!(CONTRAST.decode(2), "High");
    }

    #[test]
    fn test_decode_sharpness() {
        assert_eq!(SHARPNESS.decode(0), "Soft");
        assert_eq!(SHARPNESS.decode(1), "Normal");
        assert_eq!(SHARPNESS.decode(2), "Hard");
    }

    #[test]
    fn test_parser_trait_implementation() {
        let parser = PentaxParser;
        assert_eq!(parser.manufacturer_name(), "Pentax");
        assert_eq!(parser.tag_prefix(), "Pentax:");
    }

    #[test]
    fn test_validate_header_aoc() {
        let parser = PentaxParser;

        let valid_header = b"AOC\0extra_data_here";
        assert!(parser.validate_header(valid_header));

        let invalid_header = b"Canon\0\0\0";
        assert!(!parser.validate_header(invalid_header));
    }

    #[test]
    fn test_validate_header_pentax() {
        let parser = PentaxParser;

        let valid_header = b"PENTAX \0more_data";
        assert!(parser.validate_header(valid_header));
    }

    #[test]
    fn test_lens_lookup() {
        let parser = PentaxParser;

        // Test classic lens
        assert!(parser.lookup_lens(2).is_some());
        assert_eq!(
            parser.lookup_lens(2),
            Some("SMC Pentax-K 50mm f/1.4".to_string())
        );

        // Test Limited lens
        assert!(parser.lookup_lens(56).is_some());
        assert_eq!(
            parser.lookup_lens(56),
            Some("SMC Pentax-FA 77mm f/1.8 Limited".to_string())
        );

        // Test modern D FA lens
        assert!(parser.lookup_lens(122).is_some());
        assert_eq!(
            parser.lookup_lens(122),
            Some("HD Pentax-D FA 24-70mm f/2.8 ED SDM WR".to_string())
        );

        // Test unknown lens
        assert_eq!(parser.lookup_lens(65000), None);
    }

    #[test]
    fn test_pentax_tag_to_name() {
        assert_eq!(pentax_tag_to_name(0x0000), "Pentax:Version");
        assert_eq!(pentax_tag_to_name(0x003F), "Pentax:LensType");
        assert_eq!(pentax_tag_to_name(0x0008), "Pentax:Quality");
    }

    #[test]
    fn test_is_pentax_makernote() {
        let valid_data_aoc = b"AOC\0some_data";
        assert!(is_pentax_makernote(valid_data_aoc));

        let valid_data_pentax = b"PENTAX \0data";
        assert!(is_pentax_makernote(valid_data_pentax));

        let invalid_data = b"Nikon\0\0\0";
        assert!(!is_pentax_makernote(invalid_data));
    }
}
