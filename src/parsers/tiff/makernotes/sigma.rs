//! Sigma MakerNote Parser
//!
//! Parses Sigma-specific EXIF MakerNote tags containing camera settings,
//! lens information, image quality parameters, and other proprietary metadata.
//!
//! Supports Sigma cameras including:
//! - Sigma SD series (SD1, SD1 Merrill, SD15, SD14, SD10, SD9)
//! - Sigma DP series compacts (DP1, DP2, DP3, Quattro series)
//! - Sigma fp/fp L mirrorless cameras
//!
//! Based on ExifTool's Sigma.pm module.

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

use super::shared::array_extractors::{extract_i16_array, extract_u16_array, extract_u32_array};
use super::shared::MakerNoteParser;
use super::sigma_lens_database::lookup_lens_name;
use crate::const_decoder;

// ===== Sigma MakerNote Tag IDs =====
// Based on ExifTool Sigma.pm tag definitions

// Basic Camera Information Tags
const SIGMA_SERIAL_NUMBER: u16 = 0x0002;
const SIGMA_DRIVE_MODE: u16 = 0x0003;
const SIGMA_RESOLUTION_MODE: u16 = 0x0004;
const SIGMA_AF_MODE: u16 = 0x0005;
const SIGMA_FOCUS_SETTING: u16 = 0x0006;
const SIGMA_WHITE_BALANCE: u16 = 0x0007;
const SIGMA_EXPOSURE_MODE: u16 = 0x0008;
const SIGMA_METERING_MODE: u16 = 0x0009;
const SIGMA_LENS_RANGE: u16 = 0x000A;
const SIGMA_COLOR_SPACE: u16 = 0x000B;
const SIGMA_EXPOSURE_COMPENSATION: u16 = 0x000C;
const SIGMA_CONTRAST: u16 = 0x000D;
const SIGMA_SHADOW: u16 = 0x000E;
const SIGMA_HIGHLIGHT: u16 = 0x000F;
const SIGMA_SATURATION: u16 = 0x0010;
const SIGMA_SHARPNESS: u16 = 0x0011;
const SIGMA_FILL_LIGHT: u16 = 0x0012;
const SIGMA_COLOR_ADJUSTMENT: u16 = 0x0014;
const SIGMA_ADJUSTMENT_MODE: u16 = 0x0015;

// Image Quality and Processing
const SIGMA_QUALITY: u16 = 0x0016;
const SIGMA_FIRMWARE: u16 = 0x0017;
const SIGMA_SOFTWARE: u16 = 0x0018;
const SIGMA_AUTO_BRACKET: u16 = 0x0019;

// Lens Information
const SIGMA_LENS_TYPE: u16 = 0x001A;
const SIGMA_LENS_ID: u16 = 0x001B;
const SIGMA_LENS_MODEL: u16 = 0x001C;

// Camera-Specific Settings
const SIGMA_CAMERA_TEMPERATURE: u16 = 0x001D;
const SIGMA_COLOR_MODE: u16 = 0x001E;
const SIGMA_PICTURE_STYLE: u16 = 0x001F;

// Foveon X3 Sensor Specific Tags
const SIGMA_X3_FILL_LIGHT: u16 = 0x0020;
const SIGMA_COLOR_HUE: u16 = 0x0021;
const SIGMA_HUE_ADJUSTMENT: u16 = 0x0022;

// Advanced Features
const SIGMA_SHUTTER_COUNT: u16 = 0x0030;
const SIGMA_FLASH_MODE: u16 = 0x0031;
const SIGMA_FLASH_EXPOSURE_COMP: u16 = 0x0032;
const SIGMA_FLASH_METERING_MODE: u16 = 0x0033;

// File Format and Compression
const SIGMA_FILE_FORMAT: u16 = 0x0040;
const SIGMA_COMPRESSION: u16 = 0x0041;

// Calibration and Corrections
const SIGMA_CALIBRATION: u16 = 0x0050;
const SIGMA_DUST_REMOVAL_DATA: u16 = 0x0051;

// Sigma MakerNote header signature
// Sigma typically uses "SIGMA\0\0\0" or "FOVEON" headers
const SIGMA_HEADER: &[u8] = b"SIGMA\0\0\0";
const SIGMA_HEADER_FOVEON: &[u8] = b"FOVEON\0\0";

/// Checks if the provided data has a valid Sigma MakerNote header
///
/// # Arguments
/// * `data` - Raw MakerNote data to validate
///
/// # Returns
/// * `true` if data contains a valid Sigma header
/// * `false` otherwise
pub fn is_sigma_makernote(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }

    // Check for SIGMA header (8 bytes)
    if data.len() >= 8 && &data[0..8] == SIGMA_HEADER {
        return true;
    }

    // Check for FOVEON header (8 bytes)
    if data.len() >= 8 && &data[0..8] == SIGMA_HEADER_FOVEON {
        return true;
    }

    // Some Sigma cameras may have no header, check for valid IFD entry count
    if data.len() >= 2 {
        let entry_count = u16::from_le_bytes([data[0], data[1]]);
        // Reasonable entry count: 1-150 entries
        if entry_count > 0 && entry_count < 150 {
            return true;
        }
    }

    false
}

// ============================================================================
// DECODERS - Sigma Value Decoders
// ============================================================================
// Following the shared decoder pattern from canon.rs, sony.rs, and fujifilm.rs
// Each decoder is a constant that implements the Decode trait

// Decodes Sigma resolution mode to human-readable string
const_decoder! {
    DECODE_RESOLUTION_MODE, i32, [
        (0, "Low"),
        (1, "Medium"),
        (2, "High"),
        (3, "Ultra High"),
    ]
}

// Decodes Sigma AF mode to human-readable string
const_decoder! {
    DECODE_AF_MODE, i32, [
        (0, "Manual"),
        (1, "AF-S (Single)"),
        (2, "AF-C (Continuous)"),
        (3, "AF-A (Auto)"),
    ]
}

// Decodes Sigma white balance to human-readable string
const_decoder! {
    DECODE_WHITE_BALANCE, i32, [
        (0, "Auto"),
        (1, "Daylight"),
        (2, "Shade"),
        (3, "Cloudy"),
        (4, "Tungsten"),
        (5, "Fluorescent"),
        (6, "Flash"),
        (7, "Custom"),
        (8, "Color Temperature"),
    ]
}

// Decodes Sigma exposure mode to human-readable string
const_decoder! {
    DECODE_EXPOSURE_MODE, i32, [
        (0, "Auto"),
        (1, "Program"),
        (2, "Aperture Priority"),
        (3, "Shutter Priority"),
        (4, "Manual"),
    ]
}

// Decodes Sigma metering mode to human-readable string
const_decoder! {
    DECODE_METERING_MODE, i32, [
        (0, "Unknown"),
        (1, "Multi-segment"),
        (2, "Center-weighted Average"),
        (3, "Spot"),
        (4, "Average"),
    ]
}

// Decodes Sigma drive mode to human-readable string
const_decoder! {
    DECODE_DRIVE_MODE, i32, [
        (0, "Single"),
        (1, "Continuous"),
        (2, "Self-Timer"),
        (3, "Self-Timer (Multiple)"),
        (4, "Bracket"),
        (5, "Mirror Lock-up"),
    ]
}

// Decodes Sigma flash mode to human-readable string
const_decoder! {
    DECODE_FLASH_MODE, i32, [
        (0, "Off"),
        (1, "Auto"),
        (2, "On"),
        (3, "Red-eye Reduction"),
        (4, "Fill Flash"),
        (5, "Slow Sync"),
        (6, "Rear Curtain"),
        (7, "Wireless"),
    ]
}

// Decodes Sigma quality setting to human-readable string
const_decoder! {
    DECODE_QUALITY, i32, [
        (0, "Low"),
        (1, "Medium"),
        (2, "High"),
        (3, "RAW"),
        (4, "RAW + JPEG"),
    ]
}

// Decodes Sigma color mode to human-readable string
const_decoder! {
    DECODE_COLOR_MODE, i32, [
        (0, "Standard"),
        (1, "Vivid"),
        (2, "Neutral"),
        (3, "Portrait"),
        (4, "Landscape"),
        (5, "Monochrome"),
        (6, "Sepia"),
        (7, "FOV Classic Blue"),
        (8, "FOV Classic Yellow"),
    ]
}

// Decodes Sigma color space to human-readable string
const_decoder! {
    DECODE_COLOR_SPACE, i32, [
        (0, "sRGB"),
        (1, "Adobe RGB"),
    ]
}

// Decodes Sigma picture style to human-readable string
const_decoder! {
    DECODE_PICTURE_STYLE, i32, [
        (0, "Standard"),
        (1, "Vivid"),
        (2, "Neutral"),
        (3, "Portrait"),
        (4, "Landscape"),
        (5, "Monochrome"),
    ]
}

/// Sigma MakerNote Parser
///
/// Implements the MakerNoteParser trait for Sigma cameras.
pub struct SigmaMakerNoteParser;

impl MakerNoteParser for SigmaMakerNoteParser {
    fn manufacturer_name(&self) -> &'static str {
        "Sigma"
    }

    fn tag_prefix(&self) -> &'static str {
        "Sigma:"
    }

    fn validate_header(&self, data: &[u8]) -> bool {
        is_sigma_makernote(data)
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
        // Validate minimum data length
        if data.len() < 8 {
            return Err("Sigma MakerNote data too short".to_string());
        }

        // Skip header if present (both SIGMA and FOVEON headers are 8 bytes)
        let offset = if data.len() >= 8
            && (&data[0..8] == SIGMA_HEADER || &data[0..8] == SIGMA_HEADER_FOVEON)
        {
            8 // Skip "SIGMA\0\0\0" or "FOVEON\0\0"
        } else {
            0 // No header, start directly with IFD
        };

        // Ensure we have enough data after the header
        if offset >= data.len() {
            return Err("No data after Sigma header".to_string());
        }

        let ifd_data = &data[offset..];

        // Parse IFD entry count
        if ifd_data.len() < 2 {
            return Err("Insufficient data for IFD entry count".to_string());
        }

        let entry_count = match byte_order {
            ByteOrder::LittleEndian => u16::from_le_bytes([ifd_data[0], ifd_data[1]]),
            ByteOrder::BigEndian => u16::from_be_bytes([ifd_data[0], ifd_data[1]]),
        };

        // Validate entry count is reasonable
        if entry_count == 0 || entry_count > 200 {
            return Err(format!("Invalid Sigma IFD entry count: {}", entry_count));
        }

        // Each IFD entry is 12 bytes
        let required_size = 2 + (entry_count as usize * 12);
        if ifd_data.len() < required_size {
            return Err(format!(
                "Insufficient data for {} IFD entries (need {}, have {})",
                entry_count,
                required_size,
                ifd_data.len()
            ));
        }

        // Parse each IFD entry
        for i in 0..entry_count {
            let entry_offset = 2 + (i as usize * 12);
            let entry_data = &ifd_data[entry_offset..entry_offset + 12];

            // Parse IFD entry fields
            let tag_id = match byte_order {
                ByteOrder::LittleEndian => u16::from_le_bytes([entry_data[0], entry_data[1]]),
                ByteOrder::BigEndian => u16::from_be_bytes([entry_data[0], entry_data[1]]),
            };

            let format = match byte_order {
                ByteOrder::LittleEndian => u16::from_le_bytes([entry_data[2], entry_data[3]]),
                ByteOrder::BigEndian => u16::from_be_bytes([entry_data[2], entry_data[3]]),
            };

            let component_count = match byte_order {
                ByteOrder::LittleEndian => {
                    u32::from_le_bytes([entry_data[4], entry_data[5], entry_data[6], entry_data[7]])
                }
                ByteOrder::BigEndian => {
                    u32::from_be_bytes([entry_data[4], entry_data[5], entry_data[6], entry_data[7]])
                }
            };

            let value_offset = match byte_order {
                ByteOrder::LittleEndian => u32::from_le_bytes([
                    entry_data[8],
                    entry_data[9],
                    entry_data[10],
                    entry_data[11],
                ]),
                ByteOrder::BigEndian => u32::from_be_bytes([
                    entry_data[8],
                    entry_data[9],
                    entry_data[10],
                    entry_data[11],
                ]),
            };

            // Create IfdEntry for this tag
            let entry = IfdEntry {
                tag_id,
                field_type: format,
                value_count: component_count,
                value_offset,
            };

            // Extract and decode tag values based on tag ID
            match tag_id {
                // Serial number
                SIGMA_SERIAL_NUMBER => {
                    if entry.value_count <= 4 {
                        tags.insert(
                            "Sigma:SerialNumber".to_string(),
                            entry.value_offset.to_string(),
                        );
                    }
                }

                // Drive mode
                SIGMA_DRIVE_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Sigma:DriveMode".to_string(),
                        DECODE_DRIVE_MODE.decode(value).to_string(),
                    );
                }

                // Resolution mode
                SIGMA_RESOLUTION_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Sigma:ResolutionMode".to_string(),
                        DECODE_RESOLUTION_MODE.decode(value).to_string(),
                    );
                }

                // AF mode
                SIGMA_AF_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Sigma:AFMode".to_string(),
                        DECODE_AF_MODE.decode(value).to_string(),
                    );
                }

                // Focus setting
                SIGMA_FOCUS_SETTING => {
                    let value = entry.value_offset;
                    tags.insert("Sigma:FocusSetting".to_string(), value.to_string());
                }

                // White balance
                SIGMA_WHITE_BALANCE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Sigma:WhiteBalance".to_string(),
                        DECODE_WHITE_BALANCE.decode(value).to_string(),
                    );
                }

                // Exposure mode
                SIGMA_EXPOSURE_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Sigma:ExposureMode".to_string(),
                        DECODE_EXPOSURE_MODE.decode(value).to_string(),
                    );
                }

                // Metering mode
                SIGMA_METERING_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Sigma:MeteringMode".to_string(),
                        DECODE_METERING_MODE.decode(value).to_string(),
                    );
                }

                // Lens range (min-max focal length)
                SIGMA_LENS_RANGE => {
                    let value = entry.value_offset;
                    tags.insert("Sigma:LensRange".to_string(), value.to_string());
                }

                // Color space
                SIGMA_COLOR_SPACE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Sigma:ColorSpace".to_string(),
                        DECODE_COLOR_SPACE.decode(value).to_string(),
                    );
                }

                // Exposure compensation
                SIGMA_EXPOSURE_COMPENSATION => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Sigma:ExposureCompensation".to_string(),
                        format!("{:.1} EV", value as f32 / 10.0),
                    );
                }

                // Image processing parameters
                SIGMA_CONTRAST => {
                    let value = entry.value_offset as i32;
                    tags.insert("Sigma:Contrast".to_string(), value.to_string());
                }

                SIGMA_SHADOW => {
                    let value = entry.value_offset as i32;
                    tags.insert("Sigma:Shadow".to_string(), value.to_string());
                }

                SIGMA_HIGHLIGHT => {
                    let value = entry.value_offset as i32;
                    tags.insert("Sigma:Highlight".to_string(), value.to_string());
                }

                SIGMA_SATURATION => {
                    let value = entry.value_offset as i32;
                    tags.insert("Sigma:Saturation".to_string(), value.to_string());
                }

                SIGMA_SHARPNESS => {
                    let value = entry.value_offset as i32;
                    tags.insert("Sigma:Sharpness".to_string(), value.to_string());
                }

                SIGMA_FILL_LIGHT => {
                    let value = entry.value_offset as i32;
                    tags.insert("Sigma:FillLight".to_string(), value.to_string());
                }

                // X3 Fill light (Foveon sensor specific)
                SIGMA_X3_FILL_LIGHT => {
                    let value = entry.value_offset as i32;
                    tags.insert("Sigma:X3FillLight".to_string(), value.to_string());
                }

                // Color adjustments
                SIGMA_COLOR_ADJUSTMENT => {
                    let value = entry.value_offset as i32;
                    tags.insert("Sigma:ColorAdjustment".to_string(), value.to_string());
                }

                SIGMA_COLOR_HUE => {
                    let value = entry.value_offset as i32;
                    tags.insert("Sigma:ColorHue".to_string(), value.to_string());
                }

                SIGMA_HUE_ADJUSTMENT => {
                    let value = entry.value_offset as i32;
                    tags.insert("Sigma:HueAdjustment".to_string(), value.to_string());
                }

                // Adjustment mode
                SIGMA_ADJUSTMENT_MODE => {
                    let value = entry.value_offset;
                    tags.insert("Sigma:AdjustmentMode".to_string(), value.to_string());
                }

                // Quality
                SIGMA_QUALITY => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Sigma:Quality".to_string(),
                        DECODE_QUALITY.decode(value).to_string(),
                    );
                }

                // Firmware version
                SIGMA_FIRMWARE => {
                    if entry.value_count <= 4 {
                        tags.insert("Sigma:Firmware".to_string(), entry.value_offset.to_string());
                    }
                }

                // Software version
                SIGMA_SOFTWARE => {
                    if entry.value_count <= 4 {
                        tags.insert("Sigma:Software".to_string(), entry.value_offset.to_string());
                    }
                }

                // Auto bracket
                SIGMA_AUTO_BRACKET => {
                    let value = entry.value_offset as i32;
                    let bracket_str = match value {
                        0 => "Off",
                        1 => "On",
                        _ => "Unknown",
                    };
                    tags.insert("Sigma:AutoBracket".to_string(), bracket_str.to_string());
                }

                // Lens information
                SIGMA_LENS_TYPE => {
                    let value = entry.value_offset;
                    tags.insert("Sigma:LensType".to_string(), value.to_string());
                }

                SIGMA_LENS_ID => {
                    let lens_id = entry.value_offset as u16;
                    tags.insert("Sigma:LensID".to_string(), lens_id.to_string());

                    // Look up lens name from database
                    if let Some(lens_name) = lookup_lens_name(lens_id) {
                        tags.insert("Sigma:LensModel".to_string(), lens_name);
                    }
                }

                // Camera temperature
                SIGMA_CAMERA_TEMPERATURE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Sigma:CameraTemperature".to_string(),
                        format!("{}°C", value),
                    );
                }

                // Color mode
                SIGMA_COLOR_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Sigma:ColorMode".to_string(),
                        DECODE_COLOR_MODE.decode(value).to_string(),
                    );
                }

                // Picture style
                SIGMA_PICTURE_STYLE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Sigma:PictureStyle".to_string(),
                        DECODE_PICTURE_STYLE.decode(value).to_string(),
                    );
                }

                // Shutter count
                SIGMA_SHUTTER_COUNT => {
                    let value = entry.value_offset;
                    tags.insert("Sigma:ShutterCount".to_string(), value.to_string());
                }

                // Flash mode
                SIGMA_FLASH_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Sigma:FlashMode".to_string(),
                        DECODE_FLASH_MODE.decode(value).to_string(),
                    );
                }

                // Flash exposure compensation
                SIGMA_FLASH_EXPOSURE_COMP => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "Sigma:FlashExposureComp".to_string(),
                        format!("{:.1} EV", value as f32 / 10.0),
                    );
                }

                // Flash metering mode
                SIGMA_FLASH_METERING_MODE => {
                    let value = entry.value_offset;
                    tags.insert("Sigma:FlashMeteringMode".to_string(), value.to_string());
                }

                // File format
                SIGMA_FILE_FORMAT => {
                    let value = entry.value_offset;
                    tags.insert("Sigma:FileFormat".to_string(), value.to_string());
                }

                // Compression
                SIGMA_COMPRESSION => {
                    let value = entry.value_offset;
                    tags.insert("Sigma:Compression".to_string(), value.to_string());
                }

                _ => {
                    // Unknown tags - optionally store for debugging
                    // Uncomment to see all unknown tags:
                    // tags.insert(
                    //     format!("Sigma:Unknown-{:#06X}", entry.tag_id),
                    //     entry.value_offset.to_string(),
                    // );
                }
            }
        }

        Ok(())
    }
}

/// Maps Sigma tag ID to human-readable tag name
fn sigma_tag_to_name(tag_id: u16) -> String {
    let tag_name = match tag_id {
        SIGMA_SERIAL_NUMBER => "SerialNumber",
        SIGMA_DRIVE_MODE => "DriveMode",
        SIGMA_RESOLUTION_MODE => "ResolutionMode",
        SIGMA_AF_MODE => "AFMode",
        SIGMA_FOCUS_SETTING => "FocusSetting",
        SIGMA_WHITE_BALANCE => "WhiteBalance",
        SIGMA_EXPOSURE_MODE => "ExposureMode",
        SIGMA_METERING_MODE => "MeteringMode",
        SIGMA_LENS_RANGE => "LensRange",
        SIGMA_COLOR_SPACE => "ColorSpace",
        SIGMA_EXPOSURE_COMPENSATION => "ExposureCompensation",
        SIGMA_CONTRAST => "Contrast",
        SIGMA_SHADOW => "Shadow",
        SIGMA_HIGHLIGHT => "Highlight",
        SIGMA_SATURATION => "Saturation",
        SIGMA_SHARPNESS => "Sharpness",
        SIGMA_FILL_LIGHT => "FillLight",
        SIGMA_COLOR_ADJUSTMENT => "ColorAdjustment",
        SIGMA_QUALITY => "Quality",
        SIGMA_FIRMWARE => "Firmware",
        SIGMA_SOFTWARE => "Software",
        SIGMA_AUTO_BRACKET => "AutoBracket",
        SIGMA_LENS_TYPE => "LensType",
        SIGMA_LENS_ID => "LensID",
        SIGMA_LENS_MODEL => "LensModel",
        SIGMA_CAMERA_TEMPERATURE => "CameraTemperature",
        SIGMA_COLOR_MODE => "ColorMode",
        SIGMA_PICTURE_STYLE => "PictureStyle",
        SIGMA_SHUTTER_COUNT => "ShutterCount",
        SIGMA_FLASH_MODE => "FlashMode",
        SIGMA_FLASH_EXPOSURE_COMP => "FlashExposureComp",
        SIGMA_X3_FILL_LIGHT => "X3FillLight",
        SIGMA_COLOR_HUE => "ColorHue",
        SIGMA_HUE_ADJUSTMENT => "HueAdjustment",
        _ => return format!("Unknown-{:#06X}", tag_id),
    };
    tag_name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // Decoder Tests
    // ============================================================================
    // Tests verify that const_decoder! macros properly decode known values
    // and return "Unknown (value)" format for unknown values

    #[test]
    fn test_decode_resolution_mode() {
        assert_eq!(DECODE_RESOLUTION_MODE.decode(0), "Low");
        assert_eq!(DECODE_RESOLUTION_MODE.decode(1), "Medium");
        assert_eq!(DECODE_RESOLUTION_MODE.decode(2), "High");
        assert_eq!(DECODE_RESOLUTION_MODE.decode(3), "Ultra High");
        assert_eq!(DECODE_RESOLUTION_MODE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_af_mode() {
        assert_eq!(DECODE_AF_MODE.decode(0), "Manual");
        assert_eq!(DECODE_AF_MODE.decode(1), "AF-S (Single)");
        assert_eq!(DECODE_AF_MODE.decode(2), "AF-C (Continuous)");
        assert_eq!(DECODE_AF_MODE.decode(3), "AF-A (Auto)");
        assert_eq!(DECODE_AF_MODE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_white_balance() {
        assert_eq!(DECODE_WHITE_BALANCE.decode(0), "Auto");
        assert_eq!(DECODE_WHITE_BALANCE.decode(1), "Daylight");
        assert_eq!(DECODE_WHITE_BALANCE.decode(2), "Shade");
        assert_eq!(DECODE_WHITE_BALANCE.decode(3), "Cloudy");
        assert_eq!(DECODE_WHITE_BALANCE.decode(4), "Tungsten");
        assert_eq!(DECODE_WHITE_BALANCE.decode(5), "Fluorescent");
        assert_eq!(DECODE_WHITE_BALANCE.decode(6), "Flash");
        assert_eq!(DECODE_WHITE_BALANCE.decode(7), "Custom");
        assert_eq!(DECODE_WHITE_BALANCE.decode(8), "Color Temperature");
        assert_eq!(DECODE_WHITE_BALANCE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_exposure_mode() {
        assert_eq!(DECODE_EXPOSURE_MODE.decode(0), "Auto");
        assert_eq!(DECODE_EXPOSURE_MODE.decode(1), "Program");
        assert_eq!(DECODE_EXPOSURE_MODE.decode(2), "Aperture Priority");
        assert_eq!(DECODE_EXPOSURE_MODE.decode(3), "Shutter Priority");
        assert_eq!(DECODE_EXPOSURE_MODE.decode(4), "Manual");
        assert_eq!(DECODE_EXPOSURE_MODE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_metering_mode() {
        assert_eq!(DECODE_METERING_MODE.decode(0), "Unknown");
        assert_eq!(DECODE_METERING_MODE.decode(1), "Multi-segment");
        assert_eq!(DECODE_METERING_MODE.decode(2), "Center-weighted Average");
        assert_eq!(DECODE_METERING_MODE.decode(3), "Spot");
        assert_eq!(DECODE_METERING_MODE.decode(4), "Average");
        assert_eq!(DECODE_METERING_MODE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_drive_mode() {
        assert_eq!(DECODE_DRIVE_MODE.decode(0), "Single");
        assert_eq!(DECODE_DRIVE_MODE.decode(1), "Continuous");
        assert_eq!(DECODE_DRIVE_MODE.decode(2), "Self-Timer");
        assert_eq!(DECODE_DRIVE_MODE.decode(3), "Self-Timer (Multiple)");
        assert_eq!(DECODE_DRIVE_MODE.decode(4), "Bracket");
        assert_eq!(DECODE_DRIVE_MODE.decode(5), "Mirror Lock-up");
        assert_eq!(DECODE_DRIVE_MODE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_flash_mode() {
        assert_eq!(DECODE_FLASH_MODE.decode(0), "Off");
        assert_eq!(DECODE_FLASH_MODE.decode(1), "Auto");
        assert_eq!(DECODE_FLASH_MODE.decode(2), "On");
        assert_eq!(DECODE_FLASH_MODE.decode(3), "Red-eye Reduction");
        assert_eq!(DECODE_FLASH_MODE.decode(4), "Fill Flash");
        assert_eq!(DECODE_FLASH_MODE.decode(5), "Slow Sync");
        assert_eq!(DECODE_FLASH_MODE.decode(6), "Rear Curtain");
        assert_eq!(DECODE_FLASH_MODE.decode(7), "Wireless");
        assert_eq!(DECODE_FLASH_MODE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_quality() {
        assert_eq!(DECODE_QUALITY.decode(0), "Low");
        assert_eq!(DECODE_QUALITY.decode(1), "Medium");
        assert_eq!(DECODE_QUALITY.decode(2), "High");
        assert_eq!(DECODE_QUALITY.decode(3), "RAW");
        assert_eq!(DECODE_QUALITY.decode(4), "RAW + JPEG");
        assert_eq!(DECODE_QUALITY.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_color_mode() {
        assert_eq!(DECODE_COLOR_MODE.decode(0), "Standard");
        assert_eq!(DECODE_COLOR_MODE.decode(1), "Vivid");
        assert_eq!(DECODE_COLOR_MODE.decode(2), "Neutral");
        assert_eq!(DECODE_COLOR_MODE.decode(3), "Portrait");
        assert_eq!(DECODE_COLOR_MODE.decode(4), "Landscape");
        assert_eq!(DECODE_COLOR_MODE.decode(5), "Monochrome");
        assert_eq!(DECODE_COLOR_MODE.decode(6), "Sepia");
        assert_eq!(DECODE_COLOR_MODE.decode(7), "FOV Classic Blue");
        assert_eq!(DECODE_COLOR_MODE.decode(8), "FOV Classic Yellow");
        assert_eq!(DECODE_COLOR_MODE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_color_space() {
        assert_eq!(DECODE_COLOR_SPACE.decode(0), "sRGB");
        assert_eq!(DECODE_COLOR_SPACE.decode(1), "Adobe RGB");
        assert_eq!(DECODE_COLOR_SPACE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_decode_picture_style() {
        assert_eq!(DECODE_PICTURE_STYLE.decode(0), "Standard");
        assert_eq!(DECODE_PICTURE_STYLE.decode(1), "Vivid");
        assert_eq!(DECODE_PICTURE_STYLE.decode(2), "Neutral");
        assert_eq!(DECODE_PICTURE_STYLE.decode(3), "Portrait");
        assert_eq!(DECODE_PICTURE_STYLE.decode(4), "Landscape");
        assert_eq!(DECODE_PICTURE_STYLE.decode(5), "Monochrome");
        assert_eq!(DECODE_PICTURE_STYLE.decode(99), "Unknown (99)");
    }

    #[test]
    fn test_is_sigma_makernote() {
        // Test SIGMA header
        let sigma_header = b"SIGMA\0\0\0";
        assert!(is_sigma_makernote(sigma_header));

        // Test FOVEON header
        let foveon_header = b"FOVEON\0\0";
        assert!(is_sigma_makernote(foveon_header));

        // Test invalid header
        let invalid_header = b"INVALID\0";
        assert!(!is_sigma_makernote(invalid_header));

        // Test too short data
        let short_data = b"SIG";
        assert!(!is_sigma_makernote(short_data));

        // Test reasonable entry count (no header) - need at least 8 bytes
        let entry_count_data = [10u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8]; // 10 entries in little endian
        assert!(is_sigma_makernote(&entry_count_data));

        // Test unreasonable entry count - need at least 8 bytes
        let bad_entry_count = [200u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8]; // 200 entries
        assert!(!is_sigma_makernote(&bad_entry_count));
    }
}
