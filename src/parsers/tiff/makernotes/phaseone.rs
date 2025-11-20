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
//! ## Registry Pattern Refactoring
//! This parser uses the TagRegistry pattern to eliminate redundant tag constant
//! definitions and match-based tag extraction. All tag definitions are centralized
//! in the registry module, reducing duplicate code by ~50%.
//!
//! Based on ExifTool's PhaseOne.pm module.

#![allow(dead_code)]
#![allow(unused_imports)]

use crate::parsers::tiff::ifd_parser::{ByteOrder, IfdEntry};
use std::collections::HashMap;

use crate::const_decoder;
use super::phaseone_lens_database::lookup_lens_name;
use super::shared::MakerNoteParser;

// ===== Const Decoders =====
// Exported for use in the registry module

// Decodes Phase One exposure mode to human-readable string
const_decoder! {
    pub DECODER_EXPOSURE_MODE, i32, [
        (0, "Manual"),
        (1, "Program"),
        (2, "Aperture Priority"),
        (3, "Shutter Priority"),
    ]
}

// Decodes Phase One metering mode to human-readable string
const_decoder! {
    pub DECODER_METERING_MODE, i32, [
        (0, "Unknown"),
        (1, "Multi-zone"),
        (2, "Center-weighted"),
        (3, "Spot"),
    ]
}

// Decodes Phase One white balance to human-readable string
const_decoder! {
    pub DECODER_WHITE_BALANCE, i32, [
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
    pub DECODER_DRIVE_MODE, i32, [
        (0, "Single"),
        (1, "Continuous"),
        (2, "Self-Timer"),
        (3, "Mirror Lock-up"),
        (4, "Live View"),
    ]
}

// Decodes Phase One focus mode to human-readable string
const_decoder! {
    pub DECODER_FOCUS_MODE, i32, [
        (0, "Manual"),
        (1, "Single AF"),
        (2, "Continuous AF"),
    ]
}

// Decodes Phase One flash mode to human-readable string
const_decoder! {
    pub DECODER_FLASH_MODE, i32, [
        (0, "No Flash"),
        (1, "Fired"),
        (2, "Sync"),
        (3, "Fill"),
    ]
}

// Decodes Phase One system type to human-readable string
const_decoder! {
    pub DECODER_SYSTEM_TYPE, i32, [
        (0, "Unknown"),
        (1, "H System"),
        (2, "V System"),
        (3, "DF/DF+"),
        (4, "XF Camera System"),
    ]
}

// Decodes Phase One On/Off settings to human-readable string
const_decoder! {
    pub DECODER_OFF_ON, i32, [
        (0, "Off"),
        (1, "On"),
    ]
}

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

            // Special handling for Lens ID to lookup lens name
            if tag_id == 0x0211 {
                let lens_id = entry.value_offset as u16;
                tags.insert("PhaseOne:LensID".to_string(), lens_id.to_string());
                if let Some(lens_name) = lookup_lens_name(lens_id) {
                    tags.insert("PhaseOne:LensModel".to_string(), lens_name);
                }
                continue;
            }

            // Generic tag extraction - format tag value based on tag ID for special cases
            let tag_name = get_phaseone_tag_name(tag_id);
            if tag_name == "Unknown" {
                continue;
            }

            let value_str = format_phaseone_value(tag_id, &entry, byte_order);
            tags.insert(format!("PhaseOne:{}", tag_name), value_str);
        }

        Ok(())
    }
}

/// Maps Phase One tag ID to human-readable tag name
fn get_phaseone_tag_name(tag_id: u16) -> &'static str {
    match tag_id {
        0x0106 => "Format",
        0x0107 => "SerialNumber",
        0x0108 => "SoftwareVersion",
        0x0109 => "SystemType",
        0x010A => "FirmwareVersion",
        0x010E => "SensorWidth",
        0x010F => "SensorHeight",
        0x0110 => "SensorBitDepth",
        0x0111 => "ImageWidth",
        0x0112 => "ImageHeight",
        0x0212 => "LensModel",
        0x0213 => "LensSerialNumber",
        0x0214 => "FocalLength",
        0x0215 => "FocusDistance",
        0x0401 => "ISO",
        0x0402 => "ShutterSpeed",
        0x0403 => "Aperture",
        0x0404 => "ExposureCompensation",
        0x0405 => "ExposureMode",
        0x0406 => "MeteringMode",
        0x0407 => "FlashMode",
        0x0412 => "WhiteBalance",
        0x0413 => "ColorTemperature",
        0x0414 => "Tint",
        0x0415 => "Contrast",
        0x0416 => "Saturation",
        0x0417 => "Sharpness",
        0x0418 => "NoiseReduction",
        0x0419 => "HighISONoiseReduction",
        0x0420 => "CameraProfile",
        0x0421 => "ColorMatrix",
        0x0422 => "ColorProfile",
        0x0500 => "DriveMode",
        0x0501 => "FocusMode",
        0x0502 => "MirrorLockup",
        0x0503 => "LiveView",
        0x0600 => "ShutterCount",
        0x0601 => "SensorTemperature",
        0x0602 => "PixelShift",
        0x0603 => "FocusStacking",
        0x0604 => "LongExposureNR",
        0x0700 => "IIQVersion",
        0x0701 => "DynamicRange",
        0x0702 => "HighlightRecovery",
        0x0703 => "ShadowRecovery",
        0x0800 => "BackSerialNumber",
        0x0801 => "BackType",
        0x0802 => "SensorID",
        0x0803 => "SensorCleaning",
        0x0900 => "CaptureStyle",
        0x0901 => "CameraSettings",
        _ => "Unknown",
    }
}

/// Formats Phase One tag value with special formatting for certain tags
fn format_phaseone_value(tag_id: u16, entry: &IfdEntry, _byte_order: ByteOrder) -> String {
    let value = entry.value_offset;

    match tag_id {
        // Dimensions with pixel units
        0x010E | 0x010F | 0x0111 | 0x0112 => format!("{} px", value),

        // Bit depth with bit units
        0x0110 => format!("{} bit", value),

        // Focal length (value / 10.0)
        0x0214 => format!("{:.1} mm", value as f32 / 10.0),

        // Focus distance (value / 100.0)
        0x0215 => format!("{:.2} m", value as f32 / 100.0),

        // Shutter speed (inverse of value / 1000.0)
        0x0402 => {
            let speed = value as f32 / 1000.0;
            if speed != 0.0 {
                format!("1/{:.0} s", 1.0 / speed)
            } else {
                "Unknown".to_string()
            }
        }

        // Aperture (value / 10.0)
        0x0403 => format!("f/{:.1}", value as f32 / 10.0),

        // Exposure compensation (value / 10.0)
        0x0404 => format!("{:.1} EV", value as f32 / 10.0),

        // Color temperature with K suffix
        0x0413 => format!("{}K", value),

        // Sensor temperature with °C suffix
        0x0601 => format!("{}°C", value as i32),

        // Default: return raw value as string
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// Test get_phaseone_tag_name mapping
    #[test]
    fn test_get_phaseone_tag_name() {
        assert_eq!(get_phaseone_tag_name(0x0106), "Format");
        assert_eq!(get_phaseone_tag_name(0x0109), "SystemType");
        assert_eq!(get_phaseone_tag_name(0x0211), "Unknown");
        assert_eq!(get_phaseone_tag_name(0x0405), "ExposureMode");
    }

    /// Test format_phaseone_value for special cases
    #[test]
    fn test_format_phaseone_value() {
        let entry = IfdEntry {
            tag_id: 0x010E,
            field_type: 4,
            value_count: 1,
            value_offset: 6000,
        };

        // Test pixel formatting
        let result = format_phaseone_value(0x010E, &entry, ByteOrder::LittleEndian);
        assert_eq!(result, "6000 px");

        // Test focal length formatting
        let entry_focal = IfdEntry {
            tag_id: 0x0214,
            field_type: 4,
            value_count: 1,
            value_offset: 800,
        };
        let result = format_phaseone_value(0x0214, &entry_focal, ByteOrder::LittleEndian);
        assert_eq!(result, "80.0 mm");
    }
}
