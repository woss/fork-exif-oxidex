//! Phase One MakerNote Parser
//!
//! Parses Phase One-specific EXIF MakerNote tags containing camera settings,
//! lens information, image quality parameters, and other proprietary metadata.
//!
//! Supports Phase One digital medium format cameras including:
//! - Phase One IQ4 series (150MP, 100MP)
//! - Phase One IQ3 series (100MP, 80MP, 60MP)
//! - Phase One IQ2 series
//! - Phase One IQ1 series (P-series digital backs)
//! - Leaf Credo digital backs (acquired by Phase One)
//!
//! Based on ExifTool's PhaseOne.pm module.

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

use super::phaseone_lens_database::lookup_lens_name;
use super::shared::array_extractors::{extract_i16_array, extract_u16_array, extract_u32_array};
use super::shared::MakerNoteParser;
use crate::const_decoder;

// ===== Phase One MakerNote Tag IDs =====
// Based on ExifTool PhaseOne.pm tag definitions

// Basic Camera Information Tags
const PHASEONE_FORMAT: u16 = 0x0106;
const PHASEONE_SERIAL_NUMBER: u16 = 0x0107;
const PHASEONE_SOFTWARE_VERSION: u16 = 0x0108;
const PHASEONE_SYSTEM_TYPE: u16 = 0x0109;
const PHASEONE_FIRMWARE_VERSION: u16 = 0x010A;
const PHASEONE_SENSOR_WIDTH: u16 = 0x010E;
const PHASEONE_SENSOR_HEIGHT: u16 = 0x010F;
const PHASEONE_SENSOR_BIT_DEPTH: u16 = 0x0110;
const PHASEONE_IMAGE_WIDTH: u16 = 0x0111;
const PHASEONE_IMAGE_HEIGHT: u16 = 0x0112;

// Lens Information
const PHASEONE_LENS_ID: u16 = 0x0211;
const PHASEONE_LENS_MODEL: u16 = 0x0212;
const PHASEONE_LENS_SERIAL_NUMBER: u16 = 0x0213;
const PHASEONE_FOCAL_LENGTH: u16 = 0x0214;
const PHASEONE_FOCUS_DISTANCE: u16 = 0x0215;

// Exposure Settings
const PHASEONE_ISO_SPEED: u16 = 0x0401;
const PHASEONE_SHUTTER_SPEED: u16 = 0x0402;
const PHASEONE_APERTURE: u16 = 0x0403;
const PHASEONE_EXPOSURE_COMPENSATION: u16 = 0x0404;
const PHASEONE_EXPOSURE_MODE: u16 = 0x0405;
const PHASEONE_METERING_MODE: u16 = 0x0406;
const PHASEONE_FLASH_MODE: u16 = 0x0407;

// Image Quality and Processing
const PHASEONE_WHITE_BALANCE: u16 = 0x0412;
const PHASEONE_COLOR_TEMPERATURE: u16 = 0x0413;
const PHASEONE_TINT: u16 = 0x0414;
const PHASEONE_CONTRAST: u16 = 0x0415;
const PHASEONE_SATURATION: u16 = 0x0416;
const PHASEONE_SHARPNESS: u16 = 0x0417;
const PHASEONE_NOISE_REDUCTION: u16 = 0x0418;
const PHASEONE_HIGH_ISO_NOISE_REDUCTION: u16 = 0x0419;

// Color Profile and Calibration
const PHASEONE_CAMERA_PROFILE: u16 = 0x0420;
const PHASEONE_COLOR_MATRIX: u16 = 0x0421;
const PHASEONE_COLOR_PROFILE: u16 = 0x0422;

// Capture Settings
const PHASEONE_DRIVE_MODE: u16 = 0x0500;
const PHASEONE_FOCUS_MODE: u16 = 0x0501;
const PHASEONE_MIRROR_LOCKUP: u16 = 0x0502;
const PHASEONE_LIVE_VIEW: u16 = 0x0503;

// Advanced Features
const PHASEONE_SHUTTER_COUNT: u16 = 0x0600;
const PHASEONE_SENSOR_TEMPERATURE: u16 = 0x0601;
const PHASEONE_PIXEL_SHIFT: u16 = 0x0602;
const PHASEONE_FOCUS_STACKING: u16 = 0x0603;
const PHASEONE_LONG_EXPOSURE_NR: u16 = 0x0604;

// IIQ (Intelligent Image Quality) Specific
const PHASEONE_IIQ_VERSION: u16 = 0x0700;
const PHASEONE_DYNAMIC_RANGE: u16 = 0x0701;
const PHASEONE_HIGHLIGHT_RECOVERY: u16 = 0x0702;
const PHASEONE_SHADOW_RECOVERY: u16 = 0x0703;

// Digital Back Metadata
const PHASEONE_BACK_SERIAL: u16 = 0x0800;
const PHASEONE_BACK_TYPE: u16 = 0x0801;
const PHASEONE_SENSOR_ID: u16 = 0x0802;
const PHASEONE_SENSOR_CLEANING: u16 = 0x0803;

// Tethered Capture
const PHASEONE_CAPTURE_STYLE: u16 = 0x0900;
const PHASEONE_CAMERA_SETTINGS: u16 = 0x0901;

// Phase One MakerNote header signature
// Phase One typically uses no header, or "Phase One" text
const PHASEONE_HEADER: &[u8] = b"Phase One";

/// Checks if the provided data has a valid Phase One MakerNote header
///
/// # Arguments
/// * `data` - Raw MakerNote data to validate
///
/// # Returns
/// * `true` if data contains a valid Phase One header or appears to be Phase One data
/// * `false` otherwise
pub fn is_phaseone_makernote(data: &[u8]) -> bool {
    if data.len() < 2 {
        return false;
    }

    // Check for "Phase One" header (9 bytes)
    if data.len() >= 9 && &data[0..9] == PHASEONE_HEADER {
        return true;
    }

    // Phase One often has no header, just IFD data
    // Check if first two bytes could be a valid entry count
    if data.len() >= 2 {
        let entry_count_le = u16::from_le_bytes([data[0], data[1]]);
        let entry_count_be = u16::from_be_bytes([data[0], data[1]]);

        // Reasonable entry count: 1-100 entries (Phase One typically has fewer tags)
        if (entry_count_le > 0 && entry_count_le < 100)
            || (entry_count_be > 0 && entry_count_be < 100)
        {
            return true;
        }
    }

    false
}

// ===== Const Decoders =====
// Using const_decoder! macro for compile-time value decoding

// Decodes Phase One exposure mode to human-readable string
const_decoder! {
    DECODER_EXPOSURE_MODE, i32, [
        (0, "Manual"),
        (1, "Program"),
        (2, "Aperture Priority"),
        (3, "Shutter Priority"),
    ]
}

// Decodes Phase One metering mode to human-readable string
const_decoder! {
    DECODER_METERING_MODE, i32, [
        (0, "Unknown"),
        (1, "Multi-zone"),
        (2, "Center-weighted"),
        (3, "Spot"),
    ]
}

// Decodes Phase One white balance to human-readable string
const_decoder! {
    DECODER_WHITE_BALANCE, i32, [
        (0, "Auto"),
        (1, "Daylight"),
        (2, "Cloudy"),
        (3, "Shade"),
        (4, "Tungsten"),
        (5, "Fluorescent"),
        (6, "Flash"),
        (7, "Custom"),
        (8, "Kelvin"),
    ]
}

// Decodes Phase One drive mode to human-readable string
const_decoder! {
    DECODER_DRIVE_MODE, i32, [
        (0, "Single"),
        (1, "Continuous"),
        (2, "Self-Timer"),
        (3, "Mirror Lock-up"),
        (4, "Live View"),
    ]
}

// Decodes Phase One focus mode to human-readable string
const_decoder! {
    DECODER_FOCUS_MODE, i32, [
        (0, "Manual"),
        (1, "Single AF"),
        (2, "Continuous AF"),
    ]
}

// Decodes Phase One flash mode to human-readable string
const_decoder! {
    DECODER_FLASH_MODE, i32, [
        (0, "No Flash"),
        (1, "Fired"),
        (2, "Sync"),
        (3, "Fill"),
    ]
}

// Decodes Phase One system type to human-readable string
const_decoder! {
    DECODER_SYSTEM_TYPE, i32, [
        (0, "Unknown"),
        (1, "H System"),
        (2, "V System"),
        (3, "DF/DF+"),
        (4, "XF Camera System"),
    ]
}

// Decodes Phase One On/Off settings to human-readable string
const_decoder! {
    DECODER_OFF_ON, i32, [
        (0, "Off"),
        (1, "On"),
    ]
}

/// Phase One MakerNote Parser
///
/// Implements the MakerNoteParser trait for Phase One cameras.
pub struct PhaseOneMakerNoteParser;

impl MakerNoteParser for PhaseOneMakerNoteParser {
    fn manufacturer_name(&self) -> &'static str {
        "PhaseOne"
    }

    fn tag_prefix(&self) -> &'static str {
        "PhaseOne:"
    }

    fn validate_header(&self, data: &[u8]) -> bool {
        is_phaseone_makernote(data)
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
        if data.len() < 2 {
            return Err("Phase One MakerNote data too short".to_string());
        }

        // Skip header if present
        let offset = if data.len() >= 9 && &data[0..9] == PHASEONE_HEADER {
            9 // Skip "Phase One"
        } else {
            0 // No header, start directly with IFD
        };

        // Ensure we have enough data after the header
        if offset >= data.len() {
            return Err("No data after Phase One header".to_string());
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
        if entry_count == 0 || entry_count > 150 {
            return Err(format!(
                "Invalid Phase One IFD entry count: {}",
                entry_count
            ));
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
                // Format/quality
                PHASEONE_FORMAT => {
                    let value = entry.value_offset;
                    tags.insert("PhaseOne:Format".to_string(), value.to_string());
                }

                // Serial numbers
                PHASEONE_SERIAL_NUMBER => {
                    if entry.value_count <= 4 {
                        tags.insert(
                            "PhaseOne:SerialNumber".to_string(),
                            entry.value_offset.to_string(),
                        );
                    }
                }

                PHASEONE_BACK_SERIAL => {
                    if entry.value_count <= 4 {
                        tags.insert(
                            "PhaseOne:BackSerialNumber".to_string(),
                            entry.value_offset.to_string(),
                        );
                    }
                }

                PHASEONE_LENS_SERIAL_NUMBER => {
                    if entry.value_count <= 4 {
                        tags.insert(
                            "PhaseOne:LensSerialNumber".to_string(),
                            entry.value_offset.to_string(),
                        );
                    }
                }

                // Software and firmware versions
                PHASEONE_SOFTWARE_VERSION => {
                    if entry.value_count <= 4 {
                        tags.insert(
                            "PhaseOne:SoftwareVersion".to_string(),
                            entry.value_offset.to_string(),
                        );
                    }
                }

                PHASEONE_FIRMWARE_VERSION => {
                    if entry.value_count <= 4 {
                        tags.insert(
                            "PhaseOne:FirmwareVersion".to_string(),
                            entry.value_offset.to_string(),
                        );
                    }
                }

                // System type
                PHASEONE_SYSTEM_TYPE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "PhaseOne:SystemType".to_string(),
                        DECODER_SYSTEM_TYPE.decode(value).to_string(),
                    );
                }

                // Sensor information
                PHASEONE_SENSOR_WIDTH => {
                    let value = entry.value_offset;
                    tags.insert("PhaseOne:SensorWidth".to_string(), format!("{} px", value));
                }

                PHASEONE_SENSOR_HEIGHT => {
                    let value = entry.value_offset;
                    tags.insert("PhaseOne:SensorHeight".to_string(), format!("{} px", value));
                }

                PHASEONE_SENSOR_BIT_DEPTH => {
                    let value = entry.value_offset;
                    tags.insert(
                        "PhaseOne:SensorBitDepth".to_string(),
                        format!("{} bit", value),
                    );
                }

                PHASEONE_SENSOR_ID => {
                    let value = entry.value_offset;
                    tags.insert("PhaseOne:SensorID".to_string(), value.to_string());
                }

                PHASEONE_SENSOR_TEMPERATURE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "PhaseOne:SensorTemperature".to_string(),
                        format!("{}°C", value),
                    );
                }

                // Image dimensions
                PHASEONE_IMAGE_WIDTH => {
                    let value = entry.value_offset;
                    tags.insert("PhaseOne:ImageWidth".to_string(), format!("{} px", value));
                }

                PHASEONE_IMAGE_HEIGHT => {
                    let value = entry.value_offset;
                    tags.insert("PhaseOne:ImageHeight".to_string(), format!("{} px", value));
                }

                // Lens information
                PHASEONE_LENS_ID => {
                    let lens_id = entry.value_offset as u16;
                    tags.insert("PhaseOne:LensID".to_string(), lens_id.to_string());

                    // Look up lens name from database
                    if let Some(lens_name) = lookup_lens_name(lens_id) {
                        tags.insert("PhaseOne:LensModel".to_string(), lens_name);
                    }
                }

                PHASEONE_FOCAL_LENGTH => {
                    let value = entry.value_offset as f32 / 10.0;
                    tags.insert(
                        "PhaseOne:FocalLength".to_string(),
                        format!("{:.1} mm", value),
                    );
                }

                PHASEONE_FOCUS_DISTANCE => {
                    let value = entry.value_offset as f32 / 100.0;
                    tags.insert(
                        "PhaseOne:FocusDistance".to_string(),
                        format!("{:.2} m", value),
                    );
                }

                // Exposure settings
                PHASEONE_ISO_SPEED => {
                    let value = entry.value_offset;
                    tags.insert("PhaseOne:ISO".to_string(), value.to_string());
                }

                PHASEONE_SHUTTER_SPEED => {
                    let value = entry.value_offset as f32 / 1000.0;
                    tags.insert(
                        "PhaseOne:ShutterSpeed".to_string(),
                        format!("1/{:.0} s", 1.0 / value),
                    );
                }

                PHASEONE_APERTURE => {
                    let value = entry.value_offset as f32 / 10.0;
                    tags.insert("PhaseOne:Aperture".to_string(), format!("f/{:.1}", value));
                }

                PHASEONE_EXPOSURE_COMPENSATION => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "PhaseOne:ExposureCompensation".to_string(),
                        format!("{:.1} EV", value as f32 / 10.0),
                    );
                }

                PHASEONE_EXPOSURE_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "PhaseOne:ExposureMode".to_string(),
                        DECODER_EXPOSURE_MODE.decode(value).to_string(),
                    );
                }

                PHASEONE_METERING_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "PhaseOne:MeteringMode".to_string(),
                        DECODER_METERING_MODE.decode(value).to_string(),
                    );
                }

                PHASEONE_FLASH_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "PhaseOne:FlashMode".to_string(),
                        DECODER_FLASH_MODE.decode(value).to_string(),
                    );
                }

                // Image quality and processing
                PHASEONE_WHITE_BALANCE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "PhaseOne:WhiteBalance".to_string(),
                        DECODER_WHITE_BALANCE.decode(value).to_string(),
                    );
                }

                PHASEONE_COLOR_TEMPERATURE => {
                    let value = entry.value_offset;
                    tags.insert(
                        "PhaseOne:ColorTemperature".to_string(),
                        format!("{}K", value),
                    );
                }

                PHASEONE_TINT => {
                    let value = entry.value_offset as i32;
                    tags.insert("PhaseOne:Tint".to_string(), value.to_string());
                }

                PHASEONE_CONTRAST => {
                    let value = entry.value_offset as i32;
                    tags.insert("PhaseOne:Contrast".to_string(), value.to_string());
                }

                PHASEONE_SATURATION => {
                    let value = entry.value_offset as i32;
                    tags.insert("PhaseOne:Saturation".to_string(), value.to_string());
                }

                PHASEONE_SHARPNESS => {
                    let value = entry.value_offset as i32;
                    tags.insert("PhaseOne:Sharpness".to_string(), value.to_string());
                }

                PHASEONE_NOISE_REDUCTION => {
                    let value = entry.value_offset as i32;
                    tags.insert("PhaseOne:NoiseReduction".to_string(), value.to_string());
                }

                PHASEONE_HIGH_ISO_NOISE_REDUCTION => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "PhaseOne:HighISONoiseReduction".to_string(),
                        value.to_string(),
                    );
                }

                PHASEONE_LONG_EXPOSURE_NR => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "PhaseOne:LongExposureNR".to_string(),
                        DECODER_OFF_ON.decode(value).to_string(),
                    );
                }

                // Capture settings
                PHASEONE_DRIVE_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "PhaseOne:DriveMode".to_string(),
                        DECODER_DRIVE_MODE.decode(value).to_string(),
                    );
                }

                PHASEONE_FOCUS_MODE => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "PhaseOne:FocusMode".to_string(),
                        DECODER_FOCUS_MODE.decode(value).to_string(),
                    );
                }

                PHASEONE_MIRROR_LOCKUP => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "PhaseOne:MirrorLockup".to_string(),
                        DECODER_OFF_ON.decode(value).to_string(),
                    );
                }

                PHASEONE_LIVE_VIEW => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "PhaseOne:LiveView".to_string(),
                        DECODER_OFF_ON.decode(value).to_string(),
                    );
                }

                // Advanced features
                PHASEONE_SHUTTER_COUNT => {
                    let value = entry.value_offset;
                    tags.insert("PhaseOne:ShutterCount".to_string(), value.to_string());
                }

                PHASEONE_PIXEL_SHIFT => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "PhaseOne:PixelShift".to_string(),
                        DECODER_OFF_ON.decode(value).to_string(),
                    );
                }

                PHASEONE_FOCUS_STACKING => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "PhaseOne:FocusStacking".to_string(),
                        DECODER_OFF_ON.decode(value).to_string(),
                    );
                }

                // IIQ specific
                PHASEONE_IIQ_VERSION => {
                    let value = entry.value_offset;
                    tags.insert("PhaseOne:IIQVersion".to_string(), value.to_string());
                }

                PHASEONE_DYNAMIC_RANGE => {
                    let value = entry.value_offset;
                    tags.insert("PhaseOne:DynamicRange".to_string(), value.to_string());
                }

                PHASEONE_HIGHLIGHT_RECOVERY => {
                    let value = entry.value_offset as i32;
                    tags.insert("PhaseOne:HighlightRecovery".to_string(), value.to_string());
                }

                PHASEONE_SHADOW_RECOVERY => {
                    let value = entry.value_offset as i32;
                    tags.insert("PhaseOne:ShadowRecovery".to_string(), value.to_string());
                }

                // Digital back information
                PHASEONE_BACK_TYPE => {
                    let value = entry.value_offset;
                    tags.insert("PhaseOne:BackType".to_string(), value.to_string());
                }

                PHASEONE_SENSOR_CLEANING => {
                    let value = entry.value_offset as i32;
                    tags.insert(
                        "PhaseOne:SensorCleaning".to_string(),
                        DECODER_OFF_ON.decode(value).to_string(),
                    );
                }

                _ => {
                    // Unknown tags - optionally store for debugging
                    // Uncomment to see all unknown tags:
                    // tags.insert(
                    //     format!("PhaseOne:Unknown-{:#06X}", entry.tag_id),
                    //     entry.value_offset.to_string(),
                    // );
                }
            }
        }

        Ok(())
    }
}

/// Maps Phase One tag ID to human-readable tag name
fn phaseone_tag_to_name(tag_id: u16) -> String {
    let tag_name = match tag_id {
        PHASEONE_FORMAT => "Format",
        PHASEONE_SERIAL_NUMBER => "SerialNumber",
        PHASEONE_SOFTWARE_VERSION => "SoftwareVersion",
        PHASEONE_SYSTEM_TYPE => "SystemType",
        PHASEONE_FIRMWARE_VERSION => "FirmwareVersion",
        PHASEONE_SENSOR_WIDTH => "SensorWidth",
        PHASEONE_SENSOR_HEIGHT => "SensorHeight",
        PHASEONE_SENSOR_BIT_DEPTH => "SensorBitDepth",
        PHASEONE_IMAGE_WIDTH => "ImageWidth",
        PHASEONE_IMAGE_HEIGHT => "ImageHeight",
        PHASEONE_LENS_ID => "LensID",
        PHASEONE_LENS_MODEL => "LensModel",
        PHASEONE_LENS_SERIAL_NUMBER => "LensSerialNumber",
        PHASEONE_FOCAL_LENGTH => "FocalLength",
        PHASEONE_FOCUS_DISTANCE => "FocusDistance",
        PHASEONE_ISO_SPEED => "ISO",
        PHASEONE_SHUTTER_SPEED => "ShutterSpeed",
        PHASEONE_APERTURE => "Aperture",
        PHASEONE_EXPOSURE_COMPENSATION => "ExposureCompensation",
        PHASEONE_EXPOSURE_MODE => "ExposureMode",
        PHASEONE_METERING_MODE => "MeteringMode",
        PHASEONE_FLASH_MODE => "FlashMode",
        PHASEONE_WHITE_BALANCE => "WhiteBalance",
        PHASEONE_COLOR_TEMPERATURE => "ColorTemperature",
        PHASEONE_SHUTTER_COUNT => "ShutterCount",
        PHASEONE_SENSOR_TEMPERATURE => "SensorTemperature",
        PHASEONE_BACK_SERIAL => "BackSerialNumber",
        PHASEONE_BACK_TYPE => "BackType",
        PHASEONE_SENSOR_ID => "SensorID",
        _ => return format!("Unknown-{:#06X}", tag_id),
    };
    tag_name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test DECODER_EXPOSURE_MODE with all known values and unknown values
    #[test]
    fn test_decoder_exposure_mode() {
        // Test known values
        assert_eq!(DECODER_EXPOSURE_MODE.decode(0), "Manual");
        assert_eq!(DECODER_EXPOSURE_MODE.decode(1), "Program");
        assert_eq!(DECODER_EXPOSURE_MODE.decode(2), "Aperture Priority");
        assert_eq!(DECODER_EXPOSURE_MODE.decode(3), "Shutter Priority");

        // Test unknown values - should return "Unknown (value)" format
        assert_eq!(DECODER_EXPOSURE_MODE.decode(4), "Unknown (4)");
        assert_eq!(DECODER_EXPOSURE_MODE.decode(99), "Unknown (99)");
        assert_eq!(DECODER_EXPOSURE_MODE.decode(-1), "Unknown (-1)");
    }

    /// Test DECODER_METERING_MODE with all known values and unknown values
    #[test]
    fn test_decoder_metering_mode() {
        // Test known values
        assert_eq!(DECODER_METERING_MODE.decode(0), "Unknown");
        assert_eq!(DECODER_METERING_MODE.decode(1), "Multi-zone");
        assert_eq!(DECODER_METERING_MODE.decode(2), "Center-weighted");
        assert_eq!(DECODER_METERING_MODE.decode(3), "Spot");

        // Test unknown values
        assert_eq!(DECODER_METERING_MODE.decode(4), "Unknown (4)");
        assert_eq!(DECODER_METERING_MODE.decode(99), "Unknown (99)");
    }

    /// Test DECODER_WHITE_BALANCE with all known values and unknown values
    #[test]
    fn test_decoder_white_balance() {
        // Test known values
        assert_eq!(DECODER_WHITE_BALANCE.decode(0), "Auto");
        assert_eq!(DECODER_WHITE_BALANCE.decode(1), "Daylight");
        assert_eq!(DECODER_WHITE_BALANCE.decode(2), "Cloudy");
        assert_eq!(DECODER_WHITE_BALANCE.decode(3), "Shade");
        assert_eq!(DECODER_WHITE_BALANCE.decode(4), "Tungsten");
        assert_eq!(DECODER_WHITE_BALANCE.decode(5), "Fluorescent");
        assert_eq!(DECODER_WHITE_BALANCE.decode(6), "Flash");
        assert_eq!(DECODER_WHITE_BALANCE.decode(7), "Custom");
        assert_eq!(DECODER_WHITE_BALANCE.decode(8), "Kelvin");

        // Test unknown values
        assert_eq!(DECODER_WHITE_BALANCE.decode(9), "Unknown (9)");
        assert_eq!(DECODER_WHITE_BALANCE.decode(99), "Unknown (99)");
    }

    /// Test DECODER_DRIVE_MODE with all known values and unknown values
    #[test]
    fn test_decoder_drive_mode() {
        // Test known values
        assert_eq!(DECODER_DRIVE_MODE.decode(0), "Single");
        assert_eq!(DECODER_DRIVE_MODE.decode(1), "Continuous");
        assert_eq!(DECODER_DRIVE_MODE.decode(2), "Self-Timer");
        assert_eq!(DECODER_DRIVE_MODE.decode(3), "Mirror Lock-up");
        assert_eq!(DECODER_DRIVE_MODE.decode(4), "Live View");

        // Test unknown values
        assert_eq!(DECODER_DRIVE_MODE.decode(5), "Unknown (5)");
        assert_eq!(DECODER_DRIVE_MODE.decode(99), "Unknown (99)");
    }

    /// Test DECODER_FOCUS_MODE with all known values and unknown values
    #[test]
    fn test_decoder_focus_mode() {
        // Test known values
        assert_eq!(DECODER_FOCUS_MODE.decode(0), "Manual");
        assert_eq!(DECODER_FOCUS_MODE.decode(1), "Single AF");
        assert_eq!(DECODER_FOCUS_MODE.decode(2), "Continuous AF");

        // Test unknown values
        assert_eq!(DECODER_FOCUS_MODE.decode(3), "Unknown (3)");
        assert_eq!(DECODER_FOCUS_MODE.decode(99), "Unknown (99)");
    }

    /// Test DECODER_FLASH_MODE with all known values and unknown values
    #[test]
    fn test_decoder_flash_mode() {
        // Test known values
        assert_eq!(DECODER_FLASH_MODE.decode(0), "No Flash");
        assert_eq!(DECODER_FLASH_MODE.decode(1), "Fired");
        assert_eq!(DECODER_FLASH_MODE.decode(2), "Sync");
        assert_eq!(DECODER_FLASH_MODE.decode(3), "Fill");

        // Test unknown values
        assert_eq!(DECODER_FLASH_MODE.decode(4), "Unknown (4)");
        assert_eq!(DECODER_FLASH_MODE.decode(99), "Unknown (99)");
    }

    /// Test DECODER_SYSTEM_TYPE with all known values and unknown values
    #[test]
    fn test_decoder_system_type() {
        // Test known values
        assert_eq!(DECODER_SYSTEM_TYPE.decode(0), "Unknown");
        assert_eq!(DECODER_SYSTEM_TYPE.decode(1), "H System");
        assert_eq!(DECODER_SYSTEM_TYPE.decode(2), "V System");
        assert_eq!(DECODER_SYSTEM_TYPE.decode(3), "DF/DF+");
        assert_eq!(DECODER_SYSTEM_TYPE.decode(4), "XF Camera System");

        // Test unknown values
        assert_eq!(DECODER_SYSTEM_TYPE.decode(5), "Unknown (5)");
        assert_eq!(DECODER_SYSTEM_TYPE.decode(99), "Unknown (99)");
    }

    /// Test DECODER_OFF_ON with all known values and unknown values
    #[test]
    fn test_decoder_off_on() {
        // Test known values
        assert_eq!(DECODER_OFF_ON.decode(0), "Off");
        assert_eq!(DECODER_OFF_ON.decode(1), "On");

        // Test unknown values
        assert_eq!(DECODER_OFF_ON.decode(2), "Unknown (2)");
        assert_eq!(DECODER_OFF_ON.decode(99), "Unknown (99)");
    }

    /// Test is_phaseone_makernote function with valid headers
    #[test]
    fn test_is_phaseone_makernote_valid() {
        // Test with "Phase One" header
        let data = b"Phase One\x00\x01\x02";
        assert!(is_phaseone_makernote(data));

        // Test with valid entry count (little endian)
        let data = &[0x05, 0x00]; // 5 entries
        assert!(is_phaseone_makernote(data));

        // Test with valid entry count (big endian)
        let data = &[0x00, 0x0A]; // 10 entries
        assert!(is_phaseone_makernote(data));
    }

    /// Test is_phaseone_makernote function with invalid data
    #[test]
    fn test_is_phaseone_makernote_invalid() {
        // Test with too short data
        let data = &[0x00];
        assert!(!is_phaseone_makernote(data));

        // Test with empty data
        let data = &[];
        assert!(!is_phaseone_makernote(data));

        // Test with invalid entry count (too high)
        let data = &[0xFF, 0xFF]; // 65535 entries (invalid)
        assert!(!is_phaseone_makernote(data));
    }

    /// Test PhaseOneMakerNoteParser trait methods
    #[test]
    fn test_parser_trait_methods() {
        let parser = PhaseOneMakerNoteParser;

        // Test manufacturer name
        assert_eq!(parser.manufacturer_name(), "PhaseOne");

        // Test tag prefix
        assert_eq!(parser.tag_prefix(), "PhaseOne:");

        // Test validate_header with valid data
        let valid_data = b"Phase One\x00\x01";
        assert!(parser.validate_header(valid_data));

        // Test validate_header with invalid data
        let invalid_data = &[];
        assert!(!parser.validate_header(invalid_data));
    }
}
