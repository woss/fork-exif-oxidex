//! Fujifilm tag registry
//!
//! This module provides TagRegistry definitions for Fujifilm MakerNotes.
//! Fujifilm uses a comprehensive tag structure with simple value types
//! (strings, integers, and enumerated values) and some array-based tags.

use super::super::shared::{
    generic_decoders::*, tag_registry::TagRegistry,
};

// Re-export decoders from fujifilm.rs
// These decoders are defined using const_decoder! macros in the main parser
use super::super::fujifilm::{
    DECODE_BURST_MODE, DECODE_DRIVE_MODE, DECODE_DYNAMIC_RANGE, DECODE_EXR_MODE,
    DECODE_FILM_MODE, DECODE_FLASH_MODE, DECODE_FOCUS_MODE, DECODE_OFF_ON,
    DECODE_PICTURE_MODE, DECODE_QUALITY, DECODE_SHUTTER_TYPE, DECODE_WHITE_BALANCE,
};

// ============================================================================
// TAG REGISTRY
// ============================================================================

/// Create Fujifilm tag registry with all tag definitions
///
/// This registry provides a centralized, declarative definition of all Fujifilm
/// MakerNote tags including:
/// - Simple string tags (version, serial number, lens model name)
/// - Simple integer tags (color temperature, focal lengths, apertures, ratings)
/// - Enumerated tags with decoders (quality, white balance, focus mode, etc.)
///
/// # Returns
/// A fully configured TagRegistry ready for Fujifilm MakerNote parsing
pub fn fujifilm_registry() -> TagRegistry {
    TagRegistry::new()
        // String tags
        .register_string_tag(0x0000, "Version")
        .register_string_tag(0x0010, "SerialNumber")
        .register_string_tag(0x1050, "LensModelName")

        // Enumerated tags with decoders
        .register_enum_tag(0x1000, "Quality", &DECODE_QUALITY)
        .register_enum_tag(0x1001, "Sharpness")
        .register_enum_tag(0x1002, "WhiteBalance", &DECODE_WHITE_BALANCE)
        .register_enum_tag(0x1003, "Saturation")
        .register_enum_tag(0x1004, "Contrast")
        .register_enum_tag(0x1010, "FlashMode", &DECODE_FLASH_MODE)
        .register_enum_tag(0x1020, "Macro", &DECODE_OFF_ON)
        .register_enum_tag(0x1021, "FocusMode", &DECODE_FOCUS_MODE)
        .register_enum_tag(0x1030, "SlowSync", &DECODE_OFF_ON)
        .register_enum_tag(0x1031, "PictureMode", &DECODE_PICTURE_MODE)
        .register_enum_tag(0x1033, "EXRAuto")
        .register_enum_tag(0x1034, "EXRMode", &DECODE_EXR_MODE)
        .register_enum_tag(0x1039, "DriveMode", &DECODE_DRIVE_MODE)
        .register_enum_tag(0x1100, "ShutterType", &DECODE_SHUTTER_TYPE)
        .register_enum_tag(0x1101, "BurstMode", &DECODE_BURST_MODE)
        .register_enum_tag(0x1300, "BlurWarning")
        .register_enum_tag(0x1301, "FocusWarning")
        .register_enum_tag(0x1302, "ExposureWarning")
        .register_enum_tag(0x1304, "DynamicRangeWarning")
        .register_enum_tag(0x1401, "FilmMode", &DECODE_FILM_MODE)
        .register_enum_tag(0x1402, "DynamicRange", &DECODE_DYNAMIC_RANGE)
        .register_enum_tag(0x1403, "DynamicRangeSetting", &DECODE_DYNAMIC_RANGE)
        .register_enum_tag(0x1404, "DevelopmentDynamicRange", &DECODE_DYNAMIC_RANGE)
        .register_enum_tag(0x140B, "AutoDynamicRange")

        // Simple integer/numeric tags
        .register_integer_tag(0x1005, "ColorTemperature")
        .register_integer_tag(0x1006, "ContrastDetectionAF")
        .register_integer_tag(0x1011, "FlashEV")
        .register_integer_tag(0x1023, "FocusPixel")
        .register_integer_tag(0x1040, "ShadowTone")
        .register_integer_tag(0x1041, "HighlightTone")
        .register_integer_tag(0x1044, "DigitalZoom")
        .register_integer_tag(0x1047, "ImageGeneration")
        .register_integer_tag(0x1103, "SequenceNumber")
        .register_integer_tag(0x1405, "MinFocalLength")
        .register_integer_tag(0x1406, "MaxFocalLength")
        .register_integer_tag(0x1407, "MaxApertureAtMinFocal")
        .register_integer_tag(0x1408, "MaxApertureAtMaxFocal")
        .register_integer_tag(0x1431, "Rating")
        .register_integer_tag(0x1438, "ImageCount")
        .register_integer_tag(0x4100, "FacesDetected")
        .register_integer_tag(0x8000, "FileSource")
        .register_integer_tag(0x8002, "OrderNumber")
        .register_integer_tag(0x8003, "FrameNumber")
        .register_integer_tag(0xB211, "Parallax")
        .register_integer_tag(0xF000, "RAWImageFullSize")
        .register_integer_tag(0xF001, "RAWImageFullWidth")
        .register_integer_tag(0xF002, "RAWImageFullHeight")
        .register_integer_tag(0xF003, "RAWImageAspectRatio")
        .register_integer_tag(0x9650, "PixelShiftOffset")
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let _registry = fujifilm_registry();
        // Verify registry can be created successfully
        assert!(true, "Registry created successfully");
    }

    #[test]
    fn test_registry_has_tags() {
        let registry = fujifilm_registry();
        // Verify registry contains some expected tags
        assert!(!registry.is_empty(), "Registry should have tags");
    }
}
