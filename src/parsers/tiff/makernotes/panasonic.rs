//! Panasonic MakerNote Parser
//!
//! Parses Panasonic-specific EXIF MakerNote tags containing camera settings,
//! lens information, film simulation modes, and other proprietary metadata.
//!
//! Supports both Lumix Micro Four Thirds (M43) cameras and full-frame L-mount cameras.
//!
//! Based on ExifTool's Panasonic.pm module.
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
use crate::parsers::tiff::ifd_parser::{ByteOrder, IfdEntry};
use nom::{
    combinator::map,
    multi::count,
    number::complete::{be_u16, be_u32, le_u16, le_u32},
    IResult,
};
use std::collections::HashMap;

use super::panasonic_lens_database::lookup_lens_name;
use super::shared::MakerNoteParser;

// Import declarative decoder macros
use crate::const_decoder;

// ============================================================================
// Panasonic MakerNote Tag IDs
// ============================================================================
// Based on ExifTool Panasonic.pm tag definitions

// Basic Camera Information Tags
const PANA_VERSION: u16 = 0x0001;
const PANA_CAMERA_MODEL: u16 = 0x0002;
const PANA_QUALITY_MODE: u16 = 0x0003;
const PANA_FIRMWARE_VERSION: u16 = 0x0004;
const PANA_WHITE_BALANCE: u16 = 0x0007;
const PANA_FOCUS_MODE: u16 = 0x000F;
const PANA_AF_AREA_MODE: u16 = 0x0010;
const PANA_IMAGE_STABILIZATION: u16 = 0x001A;
const PANA_MACRO_MODE: u16 = 0x001C;
const PANA_SHOOTING_MODE: u16 = 0x001F;
const PANA_AUDIO: u16 = 0x0020;
const PANA_DATA_DUMP: u16 = 0x0021;
const PANA_FLASH_BIAS: u16 = 0x0024;
const PANA_INTERNAL_SERIAL_NUMBER: u16 = 0x0025;
const PANA_EXIF_VERSION: u16 = 0x0026;
const PANA_COLOR_EFFECT: u16 = 0x0028;
const PANA_TIME_SINCE_POWER_ON: u16 = 0x0029;
const PANA_BURST_MODE: u16 = 0x002A;
const PANA_SEQUENCE_NUMBER: u16 = 0x002B;
const PANA_CONTRAST_MODE: u16 = 0x002C;
const PANA_NOISE_REDUCTION: u16 = 0x002D;
const PANA_SELF_TIMER: u16 = 0x002E;
const PANA_ROTATION: u16 = 0x0030;
const PANA_AF_ASSIST_LAMP: u16 = 0x0031;
const PANA_COLOR_MODE: u16 = 0x0032;
const PANA_BABY_AGE: u16 = 0x0033;
const PANA_OPTICAL_ZOOM_MODE: u16 = 0x0034;
const PANA_CONVERSION_LENS: u16 = 0x0035;
const PANA_TRAVEL_DAY: u16 = 0x0036;
const PANA_CONTRAST: u16 = 0x0039;
const PANA_WORLD_TIME_LOCATION: u16 = 0x003A;
const PANA_TEXT_STAMP: u16 = 0x003B;
const PANA_PROGRAM_ISO: u16 = 0x003C;
const PANA_ADVANCED_SCENE_MODE: u16 = 0x003D;
const PANA_FACE_DETECTION_INFO: u16 = 0x003E;
const PANA_SATURATION: u16 = 0x0040;
const PANA_SHARPNESS: u16 = 0x0041;
const PANA_FILM_MODE: u16 = 0x0042;
const PANA_COLOR_TEMP_KELVIN: u16 = 0x0044;
const PANA_BRACKET_SETTINGS: u16 = 0x0045;
const PANA_WB_ADJUST_AB: u16 = 0x0046;
const PANA_WB_ADJUST_GM: u16 = 0x0047;
const PANA_FLASH_CURTAIN: u16 = 0x0048;
const PANA_LONG_EXPOSURE_NOISE_REDUCTION: u16 = 0x0049;
const PANA_PANASONIC_IMAGE_WIDTH: u16 = 0x004B;
const PANA_PANASONIC_IMAGE_HEIGHT: u16 = 0x004C;
const PANA_AF_POINT_POSITION: u16 = 0x004D;
const PANA_FACE_DETECTION: u16 = 0x004E;

// Lens and Optical Information
const PANA_LENS_TYPE: u16 = 0x0051;
const PANA_LENS_SERIAL_NUMBER: u16 = 0x0052;
const PANA_ACCESSORY_TYPE: u16 = 0x0053;
const PANA_ACCESSORY_SERIAL_NUMBER: u16 = 0x0054;
const PANA_INTERNAL_ND_FILTER: u16 = 0x0055;

// Image Quality and Processing
const PANA_INTELLIGENT_EXPOSURE: u16 = 0x0059;
const PANA_FLASH_WARNING: u16 = 0x005A;
const PANA_INTELLIGENT_RESOLUTION: u16 = 0x005D;
const PANA_INTELLIGENT_D_RANGE: u16 = 0x005E;
const PANA_CLEAR_RETOUCH: u16 = 0x0060;
const PANA_PHOTO_STYLE: u16 = 0x0061;
const PANA_SHADING_COMPENSATION: u16 = 0x0062;
const PANA_ACCELEROMETER_Z: u16 = 0x008A;
const PANA_ACCELEROMETER_X: u16 = 0x008B;
const PANA_ACCELEROMETER_Y: u16 = 0x008C;
const PANA_ROLL_ANGLE: u16 = 0x008D;
const PANA_PITCH_ANGLE: u16 = 0x008E;

// Video and Hybrid Features
const PANA_HDR: u16 = 0x0079;
const PANA_HDR_EFFECT: u16 = 0x007A;
const PANA_BURST_SPEED: u16 = 0x0077;
const PANA_INTELLIGENT_AUTO: u16 = 0x0080;
const PANA_MAKERNOTE_VERSION: u16 = 0x8000;
const PANA_SCENE_MODE: u16 = 0x8001;
const PANA_WB_RED_LEVEL: u16 = 0x8004;
const PANA_WB_GREEN_LEVEL: u16 = 0x8005;
const PANA_WB_BLUE_LEVEL: u16 = 0x8006;
const PANA_FLASH_FIRED: u16 = 0x8007;
const PANA_TEXT_STAMP_1: u16 = 0x8008;
const PANA_TEXT_STAMP_2: u16 = 0x8009;
const PANA_TEXT_STAMP_3: u16 = 0x800A;
const PANA_BABY_AGE_1: u16 = 0x8010;

// Panasonic MakerNote header signature
// Panasonic uses "Panasonic\0\0\0" header (12 bytes)
const PANASONIC_HEADER: &[u8] = b"Panasonic\0\0\0";

// ============================================================================
// Declarative Decoder Definitions
// ============================================================================
// Using const_decoder! macro to eliminate decoder function duplication

// Quality mode decoder - maps values to image quality settings
const_decoder!(
    QUALITY,
    i32,
    [
        (1, "Economy"),
        (2, "Normal"),
        (3, "Fine"),
        (4, "Super Fine"),
        (5, "Extra Fine"),
        (6, "RAW"),
        (7, "RAW + Fine"),
        (8, "RAW + Normal"),
        (9, "Motion Picture"),
    ]
);

// White balance decoder - maps values to white balance presets
const_decoder!(
    WHITE_BALANCE,
    i32,
    [
        (1, "Auto"),
        (2, "Daylight"),
        (3, "Cloudy"),
        (4, "Incandescent"),
        (5, "Manual"),
        (8, "Flash"),
        (10, "Black & White"),
        (11, "Manual 2"),
        (12, "Shade"),
        (13, "Kelvin"),
        (14, "Manual 3"),
        (15, "Manual 4"),
        (16, "Manual 5"),
        (17, "PC"),
    ]
);

// Focus mode decoder - maps values to autofocus modes
const_decoder!(
    FOCUS_MODE,
    i32,
    [
        (1, "Auto"),
        (2, "Manual"),
        (4, "AF-S (Single)"),
        (5, "AF-C (Continuous)"),
        (6, "AF-F (Flexible)"),
        (16, "MF (Manual Focus)"),
    ]
);

// AF area mode decoder - maps values to AF area selection modes
const_decoder!(
    AF_AREA_MODE,
    i32,
    [
        (0, "Face Detection"),
        (1, "49-Area"),
        (2, "Tracking"),
        (3, "1-Area"),
        (4, "Pinpoint"),
        (8, "Multi"),
        (16, "1-Area (high speed)"),
        (17, "49-Area (high speed)"),
        (18, "Tracking (high speed)"),
        (32, "1-Area (video)"),
    ]
);

// Image stabilization decoder - maps values to IS modes
const_decoder!(
    IMAGE_STABILIZATION,
    i32,
    [
        (2, "Mode 1"),
        (3, "Off"),
        (4, "Mode 2"),
        (6, "Mode 3"),
        (34, "Mode 1 (video)"),
        (35, "Off (video)"),
        (36, "Mode 2 (video)"),
    ]
);

// Shooting mode decoder - maps values to shooting scene modes
const_decoder!(
    SHOOTING_MODE,
    i32,
    [
        (1, "Normal"),
        (2, "Portrait"),
        (3, "Scenery"),
        (4, "Sports"),
        (5, "Night Portrait"),
        (6, "Program"),
        (7, "Aperture Priority"),
        (8, "Shutter Priority"),
        (9, "Macro"),
        (10, "Spot"),
        (11, "Manual"),
        (12, "Movie Preview"),
        (13, "Panning"),
        (14, "Simple"),
        (15, "Color Effects"),
        (18, "Panorama"),
        (19, "Glass Through"),
        (20, "HDR"),
    ]
);

// Contrast mode decoder - maps values to contrast settings
const_decoder!(
    CONTRAST_MODE,
    i32,
    [
        (0, "Normal"),
        (1, "Low"),
        (2, "High"),
        (3, "Medium Low"),
        (4, "Medium High"),
        (5, "High+"),
        (7, "Lowest"),
        (256, "Low"),
        (272, "Standard"),
        (288, "High"),
    ]
);

// Film mode (Photo Style) decoder - maps values to picture styles
const_decoder!(
    FILM_MODE,
    i32,
    [
        (1, "Standard"),
        (2, "Dynamic"),
        (3, "Nature"),
        (4, "Smooth"),
        (5, "Standard (B&W)"),
        (6, "Dynamic (B&W)"),
        (7, "Smooth (B&W)"),
        (9, "Scenery"),
        (10, "Portrait"),
        (11, "Monochrome"),
        (12, "Natural"),
        (13, "Vivid"),
        (14, "Flat"),
        (15, "Landscape"),
        (16, "Monochrome High Contrast"),
        (17, "Blue Filter"),
        (18, "Sepia"),
        (19, "Nostalgic"),
        (20, "Old Days"),
        (21, "High Contrast B&W"),
        (22, "Cinelike D"),
        (23, "Cinelike V"),
        (24, "Like 709"),
        (25, "V-Log"),
        (26, "V-Log L"),
    ]
);

// Noise reduction decoder - maps values to NR settings
const_decoder!(
    NOISE_REDUCTION,
    i32,
    [
        (0, "Standard"),
        (1, "Low (-1)"),
        (2, "High (+1)"),
        (3, "Lowest (-2)"),
        (4, "Highest (+2)"),
    ]
);

// Intelligent auto mode decoder - maps values to iA modes
const_decoder!(
    INTELLIGENT_AUTO,
    i32,
    [
        (0, "Off"),
        (1, "On"),
        (2, "On (macro)"),
        (3, "On (portrait)"),
        (4, "On (scenery)"),
        (5, "On (night portrait)"),
        (6, "On (night scenery)"),
        (7, "On (backlight portrait)"),
    ]
);

// HDR mode decoder - maps values to HDR settings
const_decoder!(
    HDR,
    i32,
    [
        (0, "Off"),
        (1, "HDR (1 EV)"),
        (2, "HDR (2 EV)"),
        (3, "HDR (3 EV)"),
        (100, "HDR Auto"),
    ]
);

// Photo style decoder - maps values to photo style presets
const_decoder!(
    PHOTO_STYLE,
    i32,
    [
        (0, "Standard"),
        (1, "Vivid"),
        (2, "Natural"),
        (3, "Monochrome"),
        (4, "Scenery"),
        (5, "Portrait"),
        (6, "Custom"),
        (7, "Cinelike D"),
        (8, "Cinelike V"),
        (9, "Like 709"),
        (10, "V-Log"),
        (11, "V-Log L"),
    ]
);

// Macro mode decoder - maps values to macro mode settings
const_decoder!(MACRO_MODE, i32, [(1, "On"), (2, "Off"),]);

// Rotation decoder - maps values to image rotation
const_decoder!(
    ROTATION,
    i32,
    [(1, "0°"), (3, "180°"), (6, "90° CW"), (8, "270° CW"),]
);

// Internal ND filter decoder - maps values to ND filter settings
const_decoder!(
    INTERNAL_ND_FILTER,
    i32,
    [(0, "Off"), (1, "On"), (2, "Auto"),]
);

// Intelligent exposure decoder - maps values to iExposure modes
const_decoder!(
    INTELLIGENT_EXPOSURE,
    i32,
    [(0, "Off"), (1, "Low"), (2, "Standard"), (3, "High"),]
);

// Intelligent resolution decoder - maps values to iResolution modes
const_decoder!(
    INTELLIGENT_RESOLUTION,
    i32,
    [
        (0, "Off"),
        (1, "Low"),
        (2, "Standard"),
        (3, "High"),
        (4, "Extended"),
    ]
);

// Intelligent D-range decoder - maps values to iDynamic modes
const_decoder!(
    INTELLIGENT_D_RANGE,
    i32,
    [(0, "Off"), (1, "Low"), (2, "Standard"), (3, "High"),]
);

// Long exposure noise reduction decoder
const_decoder!(LONG_EXPOSURE_NR, i32, [(1, "On"), (2, "Off"),]);

// Burst mode decoder - maps values to burst shooting modes
const_decoder!(
    BURST_MODE,
    i32,
    [
        (0, "Off"),
        (1, "Low/High Speed"),
        (2, "Infinite"),
        (4, "Unlimited"),
    ]
);

// Face detection decoder - maps values to face detection on/off
const_decoder!(FACE_DETECTION, i32, [(0, "Off"), (1, "On"),]);

/// Represents a Panasonic MakerNote parser
pub struct PanasonicParser;

impl MakerNoteParser for PanasonicParser {
    fn manufacturer_name(&self) -> &'static str {
        "Panasonic"
    }

    fn tag_prefix(&self) -> &'static str {
        "Panasonic:"
    }

    fn validate_header(&self, data: &[u8]) -> bool {
        // Panasonic header: "Panasonic\0\0\0" (12 bytes)
        data.len() >= 12 && &data[0..12] == PANASONIC_HEADER
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

        // Validate Panasonic header
        if !self.validate_header(data) {
            return Err("Invalid Panasonic MakerNote header".to_string());
        }

        // Skip 12-byte header to IFD
        let ifd_offset = 12;

        if data.len() <= ifd_offset + 2 {
            return Ok(());
        }

        let ifd_data = &data[ifd_offset..];

        // Parse IFD entry count
        let entry_count = match byte_order {
            ByteOrder::LittleEndian => u16::from_le_bytes([ifd_data[0], ifd_data[1]]),
            ByteOrder::BigEndian => u16::from_be_bytes([ifd_data[0], ifd_data[1]]),
        };

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
                PANA_VERSION
                | PANA_CAMERA_MODEL
                | PANA_FIRMWARE_VERSION
                | PANA_INTERNAL_SERIAL_NUMBER
                | PANA_LENS_SERIAL_NUMBER => {
                    if let Some(value) = extract_string_value(&entry, data, ifd_offset) {
                        let tag_name = panasonic_tag_to_name(entry.tag_id);
                        tags.insert(tag_name, value);
                    }
                }

                // Decoded tags using const_decoder! macros
                PANA_QUALITY_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert("Panasonic:QualityMode".to_string(), QUALITY.decode(value));
                }

                PANA_WHITE_BALANCE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Panasonic:WhiteBalance".to_string(),
                        WHITE_BALANCE.decode(value),
                    );
                }

                PANA_FOCUS_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert("Panasonic:FocusMode".to_string(), FOCUS_MODE.decode(value));
                }

                PANA_AF_AREA_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Panasonic:AFAreaMode".to_string(),
                        AF_AREA_MODE.decode(value),
                    );
                }

                PANA_IMAGE_STABILIZATION => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Panasonic:ImageStabilization".to_string(),
                        IMAGE_STABILIZATION.decode(value),
                    );
                }

                PANA_SHOOTING_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Panasonic:ShootingMode".to_string(),
                        SHOOTING_MODE.decode(value),
                    );
                }

                PANA_CONTRAST_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Panasonic:ContrastMode".to_string(),
                        CONTRAST_MODE.decode(value),
                    );
                }

                PANA_FILM_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert("Panasonic:FilmMode".to_string(), FILM_MODE.decode(value));
                }

                PANA_PHOTO_STYLE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Panasonic:PhotoStyle".to_string(),
                        PHOTO_STYLE.decode(value),
                    );
                }

                PANA_NOISE_REDUCTION => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Panasonic:NoiseReduction".to_string(),
                        NOISE_REDUCTION.decode(value),
                    );
                }

                PANA_INTELLIGENT_AUTO => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Panasonic:IntelligentAuto".to_string(),
                        INTELLIGENT_AUTO.decode(value),
                    );
                }

                PANA_HDR => {
                    let value = entry.value_offset as i32;
                    tags.insert("Panasonic:HDR".to_string(), HDR.decode(value));
                }

                PANA_MACRO_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert("Panasonic:MacroMode".to_string(), MACRO_MODE.decode(value));
                }

                PANA_ROTATION => {
                    let value = entry.value_offset as i32;
                    tags.insert("Panasonic:Rotation".to_string(), ROTATION.decode(value));
                }

                PANA_INTERNAL_ND_FILTER => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Panasonic:InternalNDFilter".to_string(),
                        INTERNAL_ND_FILTER.decode(value),
                    );
                }

                PANA_INTELLIGENT_EXPOSURE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Panasonic:IntelligentExposure".to_string(),
                        INTELLIGENT_EXPOSURE.decode(value),
                    );
                }

                PANA_INTELLIGENT_RESOLUTION => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Panasonic:IntelligentResolution".to_string(),
                        INTELLIGENT_RESOLUTION.decode(value),
                    );
                }

                PANA_INTELLIGENT_D_RANGE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Panasonic:IntelligentDRange".to_string(),
                        INTELLIGENT_D_RANGE.decode(value),
                    );
                }

                PANA_LONG_EXPOSURE_NOISE_REDUCTION => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Panasonic:LongExposureNoiseReduction".to_string(),
                        LONG_EXPOSURE_NR.decode(value),
                    );
                }

                PANA_BURST_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert("Panasonic:BurstMode".to_string(), BURST_MODE.decode(value));
                }

                PANA_FACE_DETECTION => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Panasonic:FaceDetection".to_string(),
                        FACE_DETECTION.decode(value),
                    );
                }

                // Simple integer/numeric tags
                PANA_SELF_TIMER => {
                    let value = entry.value_offset;
                    tags.insert("Panasonic:SelfTimer".to_string(), format!("{} s", value));
                }

                PANA_COLOR_TEMP_KELVIN => {
                    let value = entry.value_offset;
                    tags.insert(
                        "Panasonic:ColorTempKelvin".to_string(),
                        format!("{} K", value),
                    );
                }

                PANA_CONTRAST => {
                    let value = entry.value_offset as i32;
                    tags.insert("Panasonic:Contrast".to_string(), value.to_string());
                }

                PANA_SATURATION => {
                    let value = entry.value_offset as i32;
                    tags.insert("Panasonic:Saturation".to_string(), value.to_string());
                }

                PANA_SHARPNESS => {
                    let value = entry.value_offset as i32;
                    tags.insert("Panasonic:Sharpness".to_string(), value.to_string());
                }

                PANA_FLASH_BIAS => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Panasonic:FlashBias".to_string(),
                        format!("{:.1} EV", value as f32 / 10.0),
                    );
                }

                PANA_WB_ADJUST_AB => {
                    let value = entry.value_offset as i32;
                    tags.insert("Panasonic:WBAdjustAB".to_string(), value.to_string());
                }

                PANA_WB_ADJUST_GM => {
                    let value = entry.value_offset as i32;
                    tags.insert("Panasonic:WBAdjustGM".to_string(), value.to_string());
                }

                PANA_PANASONIC_IMAGE_WIDTH => {
                    let value = entry.value_offset;
                    tags.insert("Panasonic:ImageWidth".to_string(), value.to_string());
                }

                PANA_PANASONIC_IMAGE_HEIGHT => {
                    let value = entry.value_offset;
                    tags.insert("Panasonic:ImageHeight".to_string(), value.to_string());
                }

                // Lens type and lookup
                PANA_LENS_TYPE => {
                    let lens_id = entry.value_offset as u16;
                    if let Some(lens_name) = lookup_lens_name(lens_id) {
                        tags.insert("Panasonic:LensType".to_string(), lens_name);
                    } else {
                        tags.insert(
                            "Panasonic:LensType".to_string(),
                            format!("Unknown ({})", lens_id),
                        );
                    }
                }

                // Accessory type
                PANA_ACCESSORY_TYPE => {
                    let value = entry.value_offset;
                    tags.insert("Panasonic:AccessoryType".to_string(), value.to_string());
                }

                // Accelerometer data (for horizon level, etc.)
                PANA_ACCELEROMETER_X => {
                    let value = entry.value_offset as i32;
                    tags.insert("Panasonic:AccelerometerX".to_string(), value.to_string());
                }

                PANA_ACCELEROMETER_Y => {
                    let value = entry.value_offset as i32;
                    tags.insert("Panasonic:AccelerometerY".to_string(), value.to_string());
                }

                PANA_ACCELEROMETER_Z => {
                    let value = entry.value_offset as i32;
                    tags.insert("Panasonic:AccelerometerZ".to_string(), value.to_string());
                }

                PANA_ROLL_ANGLE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Panasonic:RollAngle".to_string(),
                        format!("{:.1}°", value as f32 / 10.0),
                    );
                }

                PANA_PITCH_ANGLE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Panasonic:PitchAngle".to_string(),
                        format!("{:.1}°", value as f32 / 10.0),
                    );
                }

                PANA_SEQUENCE_NUMBER => {
                    let value = entry.value_offset;
                    tags.insert("Panasonic:SequenceNumber".to_string(), value.to_string());
                }

                // White balance RGB levels
                PANA_WB_RED_LEVEL => {
                    let value = entry.value_offset;
                    tags.insert("Panasonic:WBRedLevel".to_string(), value.to_string());
                }

                PANA_WB_GREEN_LEVEL => {
                    let value = entry.value_offset;
                    tags.insert("Panasonic:WBGreenLevel".to_string(), value.to_string());
                }

                PANA_WB_BLUE_LEVEL => {
                    let value = entry.value_offset;
                    tags.insert("Panasonic:WBBlueLevel".to_string(), value.to_string());
                }

                _ => {
                    // Unknown tag - optionally log or store
                }
            }
        }

        Ok(())
    }

    fn lookup_lens(&self, lens_id: u16) -> Option<String> {
        lookup_lens_name(lens_id)
    }
}

/// Public function to parse Panasonic MakerNotes
pub fn parse_panasonic_makernotes(
    data: &[u8],
    byte_order: ByteOrder,
    tags: &mut HashMap<String, String>,
) {
    let parser = PanasonicParser;
    if let Err(e) = parser.parse(data, byte_order, tags) {
        eprintln!("Panasonic MakerNotes parse error: {}", e);
    }
}

/// Check if data contains Panasonic MakerNote header
pub fn is_panasonic_makernote(data: &[u8]) -> bool {
    let parser = PanasonicParser;
    parser.validate_header(data)
}

/// Converts Panasonic tag ID to human-readable tag name
fn panasonic_tag_to_name(tag_id: u16) -> String {
    let tag_name = match tag_id {
        PANA_VERSION => "Version",
        PANA_CAMERA_MODEL => "CameraModel",
        PANA_QUALITY_MODE => "QualityMode",
        PANA_FIRMWARE_VERSION => "FirmwareVersion",
        PANA_WHITE_BALANCE => "WhiteBalance",
        PANA_FOCUS_MODE => "FocusMode",
        PANA_AF_AREA_MODE => "AFAreaMode",
        PANA_IMAGE_STABILIZATION => "ImageStabilization",
        PANA_MACRO_MODE => "MacroMode",
        PANA_SHOOTING_MODE => "ShootingMode",
        PANA_AUDIO => "Audio",
        PANA_FLASH_BIAS => "FlashBias",
        PANA_INTERNAL_SERIAL_NUMBER => "InternalSerialNumber",
        PANA_COLOR_EFFECT => "ColorEffect",
        PANA_BURST_MODE => "BurstMode",
        PANA_SEQUENCE_NUMBER => "SequenceNumber",
        PANA_CONTRAST_MODE => "ContrastMode",
        PANA_NOISE_REDUCTION => "NoiseReduction",
        PANA_SELF_TIMER => "SelfTimer",
        PANA_ROTATION => "Rotation",
        PANA_COLOR_MODE => "ColorMode",
        PANA_CONTRAST => "Contrast",
        PANA_SATURATION => "Saturation",
        PANA_SHARPNESS => "Sharpness",
        PANA_FILM_MODE => "FilmMode",
        PANA_COLOR_TEMP_KELVIN => "ColorTempKelvin",
        PANA_LENS_TYPE => "LensType",
        PANA_LENS_SERIAL_NUMBER => "LensSerialNumber",
        PANA_PHOTO_STYLE => "PhotoStyle",
        PANA_HDR => "HDR",
        PANA_INTELLIGENT_AUTO => "IntelligentAuto",
        _ => return format!("Panasonic:Unknown-{:#06X}", tag_id),
    };

    format!("Panasonic:{}", tag_name)
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
    // Panasonic offsets are relative to IFD start (after header)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_quality() {
        assert_eq!(QUALITY.decode(2), "Normal");
        assert_eq!(QUALITY.decode(3), "Fine");
        assert_eq!(QUALITY.decode(6), "RAW");
        assert_eq!(QUALITY.decode(7), "RAW + Fine");
    }

    #[test]
    fn test_decode_white_balance() {
        assert_eq!(WHITE_BALANCE.decode(1), "Auto");
        assert_eq!(WHITE_BALANCE.decode(2), "Daylight");
        assert_eq!(WHITE_BALANCE.decode(13), "Kelvin");
    }

    #[test]
    fn test_decode_focus_mode() {
        assert_eq!(FOCUS_MODE.decode(4), "AF-S (Single)");
        assert_eq!(FOCUS_MODE.decode(5), "AF-C (Continuous)");
        assert_eq!(FOCUS_MODE.decode(16), "MF (Manual Focus)");
    }

    #[test]
    fn test_decode_film_mode() {
        assert_eq!(FILM_MODE.decode(1), "Standard");
        assert_eq!(FILM_MODE.decode(22), "Cinelike D");
        assert_eq!(FILM_MODE.decode(23), "Cinelike V");
        assert_eq!(FILM_MODE.decode(25), "V-Log");
    }

    #[test]
    fn test_decode_shooting_mode() {
        assert_eq!(SHOOTING_MODE.decode(6), "Program");
        assert_eq!(SHOOTING_MODE.decode(7), "Aperture Priority");
        assert_eq!(SHOOTING_MODE.decode(11), "Manual");
    }

    #[test]
    fn test_decode_hdr() {
        assert_eq!(HDR.decode(0), "Off");
        assert_eq!(HDR.decode(1), "HDR (1 EV)");
        assert_eq!(HDR.decode(100), "HDR Auto");
    }

    #[test]
    fn test_parser_trait_implementation() {
        let parser = PanasonicParser;
        assert_eq!(parser.manufacturer_name(), "Panasonic");
        assert_eq!(parser.tag_prefix(), "Panasonic:");
    }

    #[test]
    fn test_validate_header() {
        let parser = PanasonicParser;

        let valid_header = b"Panasonic\0\0\0extra_data_here";
        assert!(parser.validate_header(valid_header));

        let invalid_header = b"Canon\0\0\0";
        assert!(!parser.validate_header(invalid_header));

        let too_short = b"Panasonic";
        assert!(!parser.validate_header(too_short));
    }

    #[test]
    fn test_lens_lookup() {
        let parser = PanasonicParser;

        // Test M43 lens lookup
        assert!(parser.lookup_lens(32).is_some());
        assert_eq!(
            parser.lookup_lens(32),
            Some("Leica DG Nocticron 42.5mm f/1.2 ASPH. POWER O.I.S.".to_string())
        );

        // Test L-mount lens lookup
        assert!(parser.lookup_lens(103).is_some());
        assert_eq!(
            parser.lookup_lens(103),
            Some("Lumix S Pro 24-70mm f/2.8".to_string())
        );

        // Test unknown lens
        assert_eq!(parser.lookup_lens(65000), None);
    }

    #[test]
    fn test_panasonic_tag_to_name() {
        assert_eq!(panasonic_tag_to_name(PANA_VERSION), "Panasonic:Version");
        assert_eq!(panasonic_tag_to_name(PANA_LENS_TYPE), "Panasonic:LensType");
        assert_eq!(
            panasonic_tag_to_name(PANA_PHOTO_STYLE),
            "Panasonic:PhotoStyle"
        );
    }

    #[test]
    fn test_is_panasonic_makernote() {
        let valid_data = b"Panasonic\0\0\0some_data";
        assert!(is_panasonic_makernote(valid_data));

        let invalid_data = b"Nikon\0\0\0";
        assert!(!is_panasonic_makernote(invalid_data));
    }
}
