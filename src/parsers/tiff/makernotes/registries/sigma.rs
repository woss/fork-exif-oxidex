//! Sigma tag registry
//!
//! Registry of all Sigma MakerNote tags with their metadata and decoders.
//! Supports Sigma SD series, DP series, and fp/fp L cameras.

use super::super::shared::tag_registry::TagRegistry;

// Re-export decoders from sigma.rs
use super::super::sigma::{
    DECODE_AF_MODE, DECODE_COLOR_MODE, DECODE_COLOR_SPACE, DECODE_DRIVE_MODE, DECODE_EXPOSURE_MODE,
    DECODE_FLASH_MODE, DECODE_METERING_MODE, DECODE_PICTURE_STYLE, DECODE_QUALITY,
    DECODE_RESOLUTION_MODE, DECODE_WHITE_BALANCE,
};

// Wrapper functions to convert SimpleValueDecoder to function pointers
fn decode_resolution_mode(value: i32) -> String {
    DECODE_RESOLUTION_MODE.decode(value)
}
fn decode_af_mode(value: i32) -> String {
    DECODE_AF_MODE.decode(value)
}
fn decode_white_balance(value: i32) -> String {
    DECODE_WHITE_BALANCE.decode(value)
}
fn decode_exposure_mode(value: i32) -> String {
    DECODE_EXPOSURE_MODE.decode(value)
}
fn decode_metering_mode(value: i32) -> String {
    DECODE_METERING_MODE.decode(value)
}
fn decode_drive_mode(value: i32) -> String {
    DECODE_DRIVE_MODE.decode(value)
}
fn decode_flash_mode(value: i32) -> String {
    DECODE_FLASH_MODE.decode(value)
}
fn decode_quality(value: i32) -> String {
    DECODE_QUALITY.decode(value)
}
fn decode_color_mode(value: i32) -> String {
    DECODE_COLOR_MODE.decode(value)
}
fn decode_color_space(value: i32) -> String {
    DECODE_COLOR_SPACE.decode(value)
}
fn decode_picture_style(value: i32) -> String {
    DECODE_PICTURE_STYLE.decode(value)
}

/// Create and return the Sigma tag registry
///
/// This registry contains all known Sigma MakerNote tags including:
/// - Basic camera settings (drive mode, focus, exposure)
/// - Image quality parameters (resolution, contrast, sharpness)
/// - Lens information (type, ID, model)
/// - Foveon X3 sensor specific tags
/// - Advanced features (flash, calibration)
pub fn sigma_registry() -> TagRegistry {
    TagRegistry::new()
        // Basic Camera Information
        .register_raw(0x0002, "SerialNumber")
        .register_i32(0x0003, "DriveMode", decode_drive_mode)
        .register_i32(0x0004, "ResolutionMode", decode_resolution_mode)
        .register_i32(0x0005, "AFMode", decode_af_mode)
        .register_raw(0x0006, "FocusSetting")
        .register_i32(0x0007, "WhiteBalance", decode_white_balance)
        .register_i32(0x0008, "ExposureMode", decode_exposure_mode)
        .register_i32(0x0009, "MeteringMode", decode_metering_mode)
        .register_raw(0x000A, "LensRange")
        .register_i32(0x000B, "ColorSpace", decode_color_space)
        .register_raw(0x000C, "ExposureCompensation")
        .register_raw(0x000D, "Contrast")
        .register_raw(0x000E, "Shadow")
        .register_raw(0x000F, "Highlight")
        .register_raw(0x0010, "Saturation")
        .register_raw(0x0011, "Sharpness")
        .register_raw(0x0012, "FillLight")
        .register_raw(0x0014, "ColorAdjustment")
        .register_raw(0x0015, "AdjustmentMode")
        // Image Quality and Processing
        .register_i32(0x0016, "Quality", decode_quality)
        .register_raw(0x0017, "Firmware")
        .register_raw(0x0018, "Software")
        .register_raw(0x0019, "AutoBracket")
        // Lens Information
        .register_raw(0x001A, "LensType")
        .register_raw(0x001B, "LensID") // Used for lens database lookup
        .register_raw(0x001C, "LensModel")
        // Camera-Specific Settings
        .register_raw(0x001D, "CameraTemperature")
        .register_i32(0x001E, "ColorMode", decode_color_mode)
        .register_i32(0x001F, "PictureStyle", decode_picture_style)
        // Foveon X3 Sensor Specific
        .register_raw(0x0020, "X3FillLight")
        .register_raw(0x0021, "ColorHue")
        .register_raw(0x0022, "HueAdjustment")
        // Advanced Features
        .register_raw(0x0030, "ShutterCount")
        .register_i32(0x0031, "FlashMode", decode_flash_mode)
        .register_raw(0x0032, "FlashExposureComp")
        .register_raw(0x0033, "FlashMeteringMode")
        // File Format and Compression
        .register_raw(0x0040, "FileFormat")
        .register_raw(0x0041, "Compression")
        // Calibration and Corrections
        .register_raw(0x0050, "Calibration")
        .register_raw(0x0051, "DustRemovalData")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = sigma_registry();

        // Verify key tags are registered
        assert!(registry.has_tag(0x0002)); // SerialNumber
        assert!(registry.has_tag(0x0008)); // ExposureMode
        assert!(registry.has_tag(0x001B)); // LensID
        assert!(registry.has_tag(0x0030)); // ShutterCount
    }

    #[test]
    fn test_registry_tag_names() {
        let registry = sigma_registry();

        // Verify tag names
        assert_eq!(registry.get_tag_name(0x0002), Some("SerialNumber"));
        assert_eq!(registry.get_tag_name(0x0003), Some("DriveMode"));
        assert_eq!(registry.get_tag_name(0x001B), Some("LensID"));
        assert_eq!(registry.get_tag_name(0x0050), Some("Calibration"));
    }

    #[test]
    fn test_unknown_tag() {
        let registry = sigma_registry();
        assert!(!registry.has_tag(0xFFFF));
        assert_eq!(registry.get_tag_name(0xFFFF), None);
    }
}
