//! Apple (iPhone/iPad) tag registry
//!
//! This module provides a declarative registry for Apple MakerNote tags,
//! consolidating tag definitions and decoders for cleaner parser implementation.
//!
//! ## Tag Categories
//! - Computational Photography (HDR, Portrait Mode, Night Mode)
//! - Multi-camera metadata (lens identification)
//! - Scene detection and semantic styles
//! - Live Photo and burst mode metadata
//! - Device orientation and runtime flags

use super::super::shared::tag_registry::TagRegistry;

// Re-export existing decoders from apple.rs to avoid duplication
// These decoders are already defined using const_decoder! macro
use super::super::apple::{
    DECODE_HDR_TYPE, DECODE_LENS_MODEL, DECODE_PORTRAIT_MODE, DECODE_SCENE_TYPE,
    DECODE_SEMANTIC_STYLE,
};

// Apple MakerNote Tag IDs
const APPLE_HDR_IMAGE_TYPE: u16 = 0x000A;
const APPLE_BURST_UUID: u16 = 0x000B;
const APPLE_CONTENT_IDENTIFIER: u16 = 0x0011;
const APPLE_IMAGE_UNIQUE_ID: u16 = 0x0015;
const APPLE_LIVE_PHOTO_ID: u16 = 0x0017;
const APPLE_RUN_TIME: u16 = 0x001A;
const APPLE_ACCELERATION_VECTOR: u16 = 0x001B;
const APPLE_PORTRAIT_DATA: u16 = 0x0020;
const APPLE_FOCUS_DISTANCE_RANGE: u16 = 0x002B;
const APPLE_SEMANTIC_STYLE: u16 = 0x002E;
const APPLE_FRONT_FACING_CAMERA: u16 = 0x0032;
const APPLE_LENS_MODEL: u16 = 0x0035;
const APPLE_SMART_HDR_VERSION: u16 = 0x0037;
const APPLE_NIGHT_MODE: u16 = 0x0039;
const APPLE_SCENE_DETECTION: u16 = 0x003C;

// ============================================================================
// TAG REGISTRY
// ============================================================================

/// Creates the Apple tag registry with all tag definitions and decoders
///
/// This registry provides a centralized, declarative definition of all Apple
/// MakerNote tags, replacing scattered match statements with a clean lookup table.
///
/// # Returns
/// A TagRegistry instance with all Apple tags registered
///
/// # Example
/// ```ignore
/// let registry = apple_registry();
/// let hdr_type = registry.decode_i16(0x000A, 4); // "Smart HDR"
/// ```
pub fn apple_registry() -> TagRegistry {
    TagRegistry::new()
        // Computational photography tags with decoders
        .register_simple_i16(APPLE_HDR_IMAGE_TYPE, "HDRImageType", &DECODE_HDR_TYPE)
        .register_simple_i16(APPLE_PORTRAIT_DATA, "PortraitMode", &DECODE_PORTRAIT_MODE)
        .register_simple_i16(
            APPLE_SEMANTIC_STYLE,
            "SemanticStyle",
            &DECODE_SEMANTIC_STYLE,
        )
        .register_simple_i16(APPLE_SCENE_DETECTION, "SceneDetection", &DECODE_SCENE_TYPE)
        .register_simple_i16(APPLE_LENS_MODEL, "LensModel", &DECODE_LENS_MODEL)
        // Raw integer/string tags
        .register_raw(APPLE_SMART_HDR_VERSION, "SmartHDRVersion")
        .register_raw(APPLE_FOCUS_DISTANCE_RANGE, "FocusDistanceRange")
        .register_raw(APPLE_FRONT_FACING_CAMERA, "FacingCamera")
        .register_raw(APPLE_NIGHT_MODE, "NightMode")
        .register_raw(APPLE_RUN_TIME, "RunTimeFlags")
        // String tags (UUIDs, identifiers)
        .register_raw(APPLE_BURST_UUID, "BurstUUID")
        .register_raw(APPLE_CONTENT_IDENTIFIER, "ContentIdentifier")
        .register_raw(APPLE_IMAGE_UNIQUE_ID, "ImageUniqueID")
        .register_raw(APPLE_LIVE_PHOTO_ID, "LivePhotoVideoID")
        .register_raw(APPLE_ACCELERATION_VECTOR, "AccelerationVector")
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Decode front-facing camera flag to human-readable string
///
/// # Arguments
/// * `value` - Camera flag (1 = Front, 0 = Back)
///
/// # Returns
/// "Front" or "Back" based on flag value
#[inline]
pub fn decode_facing_camera(value: i16) -> String {
    if value == 1 {
        "Front".to_string()
    } else {
        "Back".to_string()
    }
}

/// Decode night mode flag to human-readable string
///
/// # Arguments
/// * `value` - Night mode flag (>0 = On, 0 = Off)
///
/// # Returns
/// "On" or "Off" based on flag value
#[inline]
pub fn decode_night_mode(value: i16) -> String {
    if value > 0 {
        "On".to_string()
    } else {
        "Off".to_string()
    }
}

/// Format runtime flags as hexadecimal string
///
/// # Arguments
/// * `value` - 32-bit runtime flags
///
/// # Returns
/// Hexadecimal representation (e.g., "0x00001234")
#[inline]
pub fn format_runtime_flags(value: u32) -> String {
    format!("0x{:08X}", value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = apple_registry();
        assert!(!registry.is_empty());
        assert!(registry.len() >= 15); // At least 15 tags registered
    }

    #[test]
    fn test_hdr_type_decoding() {
        let registry = apple_registry();
        assert_eq!(registry.decode_i16(APPLE_HDR_IMAGE_TYPE, 0), "Off");
        assert_eq!(registry.decode_i16(APPLE_HDR_IMAGE_TYPE, 4), "Smart HDR");
        assert_eq!(registry.decode_i16(APPLE_HDR_IMAGE_TYPE, 8), "Smart HDR 5");
    }

    #[test]
    fn test_portrait_mode_decoding() {
        let registry = apple_registry();
        assert_eq!(registry.decode_i16(APPLE_PORTRAIT_DATA, 0), "Off");
        assert_eq!(registry.decode_i16(APPLE_PORTRAIT_DATA, 1), "Natural Light");
        assert_eq!(registry.decode_i16(APPLE_PORTRAIT_DATA, 4), "Stage Light");
    }

    #[test]
    fn test_scene_detection_decoding() {
        let registry = apple_registry();
        assert_eq!(registry.decode_i16(APPLE_SCENE_DETECTION, 0), "None");
        assert_eq!(registry.decode_i16(APPLE_SCENE_DETECTION, 6), "Night");
        assert_eq!(registry.decode_i16(APPLE_SCENE_DETECTION, 8), "Food");
    }

    #[test]
    fn test_semantic_style_decoding() {
        let registry = apple_registry();
        assert_eq!(registry.decode_i16(APPLE_SEMANTIC_STYLE, 0), "Standard");
        assert_eq!(registry.decode_i16(APPLE_SEMANTIC_STYLE, 2), "Vibrant");
    }

    #[test]
    fn test_lens_model_decoding() {
        let registry = apple_registry();
        assert_eq!(
            registry.decode_i16(APPLE_LENS_MODEL, 0),
            "Wide (Main Camera)"
        );
        assert_eq!(registry.decode_i16(APPLE_LENS_MODEL, 1), "Telephoto");
        assert_eq!(registry.decode_i16(APPLE_LENS_MODEL, 2), "Ultra Wide");
    }

    #[test]
    fn test_tag_names() {
        let registry = apple_registry();
        assert_eq!(
            registry.get_tag_name(APPLE_HDR_IMAGE_TYPE),
            Some("HDRImageType")
        );
        assert_eq!(
            registry.get_tag_name(APPLE_PORTRAIT_DATA),
            Some("PortraitMode")
        );
        assert_eq!(
            registry.get_tag_name(APPLE_LIVE_PHOTO_ID),
            Some("LivePhotoVideoID")
        );
    }

    #[test]
    fn test_raw_tags() {
        let registry = apple_registry();
        // Raw tags should return value as-is
        assert_eq!(registry.decode_i16(APPLE_SMART_HDR_VERSION, 3), "3");
        assert_eq!(registry.decode_i16(APPLE_FOCUS_DISTANCE_RANGE, 150), "150");
    }

    #[test]
    fn test_helper_facing_camera() {
        assert_eq!(decode_facing_camera(1), "Front");
        assert_eq!(decode_facing_camera(0), "Back");
    }

    #[test]
    fn test_helper_night_mode() {
        assert_eq!(decode_night_mode(0), "Off");
        assert_eq!(decode_night_mode(1), "On");
        assert_eq!(decode_night_mode(5), "On");
    }

    #[test]
    fn test_helper_runtime_flags() {
        assert_eq!(format_runtime_flags(0x12345678), "0x12345678");
        assert_eq!(format_runtime_flags(0), "0x00000000");
    }
}
