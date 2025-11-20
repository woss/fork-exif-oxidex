//! Panasonic tag registry
//!
//! This module provides TagRegistry definitions for Panasonic MakerNotes.
//! Panasonic uses a straightforward tag structure with mostly simple value types
//! (strings, integers, and enumerated values) and no complex array-based tags.

use super::super::shared::{
    generic_decoders::*, tag_registry::TagRegistry,
};

// Re-export decoders from panasonic.rs
// These decoders are defined using const_decoder! macros in the main parser
use super::super::panasonic::{
    AF_AREA_MODE, BURST_MODE, CONTRAST_MODE, FACE_DETECTION, FILM_MODE, FOCUS_MODE,
    HDR, IMAGE_STABILIZATION, INTELLIGENT_AUTO, INTELLIGENT_D_RANGE,
    INTELLIGENT_EXPOSURE, INTELLIGENT_RESOLUTION, INTERNAL_ND_FILTER, LONG_EXPOSURE_NR,
    MACRO_MODE, NOISE_REDUCTION, PHOTO_STYLE, QUALITY, ROTATION, SHOOTING_MODE,
    WHITE_BALANCE,
};

// ============================================================================
// TAG REGISTRY
// ============================================================================

/// Create Panasonic tag registry with all tag definitions
///
/// This registry provides a centralized, declarative definition of all Panasonic
/// MakerNote tags including:
/// - Simple string tags (version, model, firmware, serial numbers, lens names)
/// - Simple integer tags (contrast, saturation, sharpness, RGB levels, angles)
/// - Enumerated tags with decoders (quality, white balance, focus mode, etc.)
///
/// # Returns
/// A fully configured TagRegistry ready for Panasonic MakerNote parsing
pub fn panasonic_registry() -> TagRegistry {
    TagRegistry::new()
        // String tags
        .register_string_tag(0x0001, "Version")
        .register_string_tag(0x0002, "CameraModel")
        .register_string_tag(0x0004, "FirmwareVersion")
        .register_string_tag(0x0025, "InternalSerialNumber")
        .register_string_tag(0x0052, "LensSerialNumber")

        // Enumerated tags with decoders
        .register_enum_tag_required(0x0003, "QualityMode", &QUALITY)
        .register_enum_tag_required(0x0007, "WhiteBalance", &WHITE_BALANCE)
        .register_enum_tag_required(0x000F, "FocusMode", &FOCUS_MODE)
        .register_enum_tag_required(0x0010, "AFAreaMode", &AF_AREA_MODE)
        .register_enum_tag_required(0x001A, "ImageStabilization", &IMAGE_STABILIZATION)
        .register_enum_tag_required(0x001C, "MacroMode", &MACRO_MODE)
        .register_enum_tag_required(0x001F, "ShootingMode", &SHOOTING_MODE)
        .register_enum_tag_required(0x002A, "BurstMode", &BURST_MODE)
        .register_enum_tag_required(0x002C, "ContrastMode", &CONTRAST_MODE)
        .register_enum_tag_required(0x002D, "NoiseReduction", &NOISE_REDUCTION)
        .register_enum_tag_required(0x0030, "Rotation", &ROTATION)
        .register_raw(0x0032, "ColorMode")
        .register_raw(0x0040, "Saturation")
        .register_raw(0x0041, "Sharpness")
        .register_enum_tag_required(0x0042, "FilmMode", &FILM_MODE)
        .register_enum_tag_required(0x004E, "FaceDetection", &FACE_DETECTION)
        .register_enum_tag_required(0x0049, "LongExposureNoiseReduction", &LONG_EXPOSURE_NR)
        .register_enum_tag_required(0x0055, "InternalNDFilter", &INTERNAL_ND_FILTER)
        .register_enum_tag_required(0x0059, "IntelligentExposure", &INTELLIGENT_EXPOSURE)
        .register_enum_tag_required(0x005D, "IntelligentResolution", &INTELLIGENT_RESOLUTION)
        .register_enum_tag_required(0x005E, "IntelligentDRange", &INTELLIGENT_D_RANGE)
        .register_enum_tag_required(0x0061, "PhotoStyle", &PHOTO_STYLE)
        .register_enum_tag_required(0x0079, "HDR", &HDR)
        .register_enum_tag_required(0x0080, "IntelligentAuto", &INTELLIGENT_AUTO)
        .register_enum_tag_required(0x8007, "FlashFired", &ON_OFF_I32)

        // Simple integer/numeric tags
        .register_integer_tag(0x0024, "FlashBias", None)
        .register_integer_tag(0x0029, "TimeSincePowerOn", None)
        .register_integer_tag(0x002B, "SequenceNumber", None)
        .register_integer_tag(0x0039, "Contrast", None)
        .register_integer_tag(0x0044, "ColorTempKelvin", None)
        .register_integer_tag(0x004B, "ImageWidth", None)
        .register_integer_tag(0x004C, "ImageHeight", None)
        .register_integer_tag(0x008A, "AccelerometerZ", None)
        .register_integer_tag(0x008B, "AccelerometerX", None)
        .register_integer_tag(0x008C, "AccelerometerY", None)
        .register_integer_tag(0x008D, "RollAngle", None)
        .register_integer_tag(0x008E, "PitchAngle", None)
        .register_integer_tag(0x8004, "WBRedLevel", None)
        .register_integer_tag(0x8005, "WBGreenLevel", None)
        .register_integer_tag(0x8006, "WBBlueLevel", None)
        .register_integer_tag(0x8000, "MakerNoteVersion", None)
        .register_integer_tag(0x8001, "SceneMode", None)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let _registry = panasonic_registry();
        // Verify registry can be created successfully
        assert!(true, "Registry created successfully");
    }

    #[test]
    fn test_registry_has_tags() {
        let registry = panasonic_registry();
        // Verify registry contains some expected tags
        assert!(!registry.is_empty(), "Registry should have tags");
    }
}
