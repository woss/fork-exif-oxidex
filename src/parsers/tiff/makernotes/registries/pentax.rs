//! Pentax tag registry
//!
//! This module provides TagRegistry definitions for Pentax MakerNotes.
//! Pentax uses a straightforward tag structure with mostly simple value types
//! (strings, integers, and enumerated values) and optional array-based tags.

use super::super::shared::{
    generic_decoders::*, tag_registry::TagRegistry,
};

// Re-export decoders from pentax.rs
// These decoders are defined using const_decoder! macros in the main parser
use super::super::pentax::{
    AUTO_BRACKETING, COLOR_SPACE, CONTRAST, DRIVE_MODE, FLASH_MODE, FOCUS_MODE,
    IMAGE_SIZE, METERING_MODE, PICTURE_MODE, QUALITY, SATURATION, SHAKE_REDUCTION,
    SHARPNESS, WHITE_BALANCE, WHITE_BALANCE_MODE, WORLD_TIME_LOCATION,
    PIXEL_SHIFT_RESOLUTION,
};

// ============================================================================
// TAG REGISTRY
// ============================================================================

/// Create Pentax tag registry with all tag definitions
///
/// This registry provides a centralized, declarative definition of all Pentax
/// MakerNote tags including:
/// - Simple string tags (version, model, date, time, lens model)
/// - Simple integer tags (ISO, focal length, digital zoom, shutter count, etc.)
/// - Enumerated tags with decoders (quality, picture mode, flash, focus, etc.)
///
/// # Returns
/// A fully configured TagRegistry ready for Pentax MakerNote parsing
pub fn pentax_registry() -> TagRegistry {
    TagRegistry::new()
        // String tags
        .register_string_tag(0x0000, "Version")
        .register_string_tag(0x0006, "Date")
        .register_string_tag(0x0007, "Time")
        .register_string_tag(0x009F, "LensModel")

        // Enumerated tags with decoders
        .register_enum_tag(0x0001, "PentaxModelType")
        .register_enum_tag(0x0008, "Quality", &QUALITY)
        .register_enum_tag(0x000B, "PictureMode", &PICTURE_MODE)
        .register_enum_tag(0x000C, "FlashMode", &FLASH_MODE)
        .register_enum_tag(0x000D, "FocusMode", &FOCUS_MODE)
        .register_enum_tag(0x0017, "MeteringMode", &METERING_MODE)
        .register_enum_tag(0x0018, "AutoBracketing", &AUTO_BRACKETING)
        .register_enum_tag(0x0019, "WhiteBalance", &WHITE_BALANCE)
        .register_enum_tag(0x001A, "WhiteBalanceMode", &WHITE_BALANCE_MODE)
        .register_enum_tag(0x001F, "Saturation", &SATURATION)
        .register_enum_tag(0x0020, "Contrast", &CONTRAST)
        .register_enum_tag(0x0021, "Sharpness", &SHARPNESS)
        .register_enum_tag(0x0022, "WorldTimeLocation", &WORLD_TIME_LOCATION)
        .register_enum_tag(0x0032, "ImageProcessing")
        .register_enum_tag(0x0033, "PictureMode2", &PICTURE_MODE)
        .register_enum_tag(0x0034, "DriveMode", &DRIVE_MODE)
        .register_enum_tag(0x0037, "ColorSpace", &COLOR_SPACE)
        .register_enum_tag(0x003C, "ShakeReductionInfo", &SHAKE_REDUCTION)
        .register_enum_tag(0x0086, "PixelShiftResolution", &PIXEL_SHIFT_RESOLUTION)

        // Simple integer/numeric tags
        .register_integer_tag(0x0002, "PreviewImageSize")
        .register_integer_tag(0x0003, "PreviewImageLength")
        .register_integer_tag(0x0004, "PreviewImageStart")
        .register_integer_tag(0x0005, "PentaxModelID")
        .register_integer_tag(0x0009, "PentaxImageSize", &IMAGE_SIZE)
        .register_integer_tag(0x000E, "AFPointSelected")
        .register_integer_tag(0x000F, "AFPointInFocus")
        .register_integer_tag(0x0014, "ISOSpeed")
        .register_integer_tag(0x001B, "BlueBalance")
        .register_integer_tag(0x001C, "RedBalance")
        .register_integer_tag(0x001D, "FocalLength")
        .register_integer_tag(0x001E, "DigitalZoom")
        .register_integer_tag(0x0023, "HometownCity")
        .register_integer_tag(0x0024, "DestinationCity")
        .register_integer_tag(0x0025, "HometownDST")
        .register_integer_tag(0x0026, "DestinationDST")
        .register_integer_tag(0x0038, "ImageAreaOffset")
        .register_integer_tag(0x0039, "RawImageSize")
        .register_integer_tag(0x003B, "BatteryLevel")
        .register_integer_tag(0x003D, "ShutterCount")
        .register_integer_tag(0x0047, "CameraTemperature")
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let _registry = pentax_registry();
        // Verify registry can be created successfully
        assert!(true, "Registry created successfully");
    }

    #[test]
    fn test_registry_has_tags() {
        let registry = pentax_registry();
        // Verify registry contains some expected tags
        assert!(!registry.is_empty(), "Registry should have tags");
    }
}
