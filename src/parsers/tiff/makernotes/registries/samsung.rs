//! Samsung MakerNote tag registry
//!
//! This module provides a centralized tag registry for Samsung Galaxy MakerNotes,
//! eliminating code duplication by consolidating all tag definitions and decoders
//! into a single static registry.
//!
//! ## Tag Categories
//! - Scene Optimizer AI detection
//! - Single Take mode information
//! - Expert RAW processing data
//! - Multi-Frame Processing details
//! - Director's View settings
//! - Pro mode parameters
//! - Object tracking data
//! - Night mode settings

use super::super::shared::generic_decoders::{SimpleValueDecoder, ON_OFF};
use super::super::shared::tag_registry::TagRegistry;
use once_cell::sync::Lazy;

// ============================================================================
// Tag ID Constants
// ============================================================================

/// AI Scene Optimizer enabled
pub const SAMSUNG_SCENE_OPTIMIZER: u16 = 0x0001;
/// Detected scene type
pub const SAMSUNG_SCENE_TYPE: u16 = 0x0002;
/// Single Take mode enabled
pub const SAMSUNG_SINGLE_TAKE: u16 = 0x0005;
/// Single Take frame number
pub const SAMSUNG_SINGLE_TAKE_FRAME: u16 = 0x0006;
/// Expert RAW mode enabled
pub const SAMSUNG_EXPERT_RAW: u16 = 0x0008;
/// Multi-frame noise reduction enabled
pub const SAMSUNG_MULTI_FRAME_NR: u16 = 0x000A;
/// Director's View multi-camera mode
pub const SAMSUNG_DIRECTORS_VIEW: u16 = 0x000C;
/// Pro mode manual controls enabled
pub const SAMSUNG_PRO_MODE: u16 = 0x000E;
/// Object tracking autofocus enabled
pub const SAMSUNG_OBJECT_TRACKING: u16 = 0x0010;
/// Night mode processing enabled
pub const SAMSUNG_NIGHT_MODE: u16 = 0x0012;
/// Night Hyperlapse mode enabled
pub const SAMSUNG_NIGHT_HYPERLAPSE: u16 = 0x0014;
/// Super Steady video stabilization
pub const SAMSUNG_SUPER_STEADY: u16 = 0x0016;
/// Food photography mode enabled
pub const SAMSUNG_FOOD_MODE: u16 = 0x0018;
/// Portrait Live Focus effect
pub const SAMSUNG_PORTRAIT_EFFECT: u16 = 0x001A;
/// Active camera lens identifier
pub const SAMSUNG_LENS_TYPE: u16 = 0x001C;
/// Digital zoom magnification level
pub const SAMSUNG_ZOOM_LEVEL: u16 = 0x001E;

// ============================================================================
// Decoders
// ============================================================================

/// Decoder for Scene Optimizer mode (Off/On/Auto)
pub const SCENE_OPTIMIZER: SimpleValueDecoder<i16> =
    SimpleValueDecoder::new(&[(0, "Off"), (1, "On"), (2, "Auto")]);

/// Decoder for AI scene detection result
pub const SCENE_TYPE: SimpleValueDecoder<i16> = SimpleValueDecoder::new(&[
    (0, "None"),
    (1, "Food"),
    (2, "Sunset"),
    (3, "Blue Sky"),
    (4, "Snow"),
    (5, "Greenery"),
    (6, "Beach"),
    (7, "Night"),
    (8, "Flower"),
    (9, "Indoor"),
    (10, "Pet"),
    (11, "Text"),
    (12, "Backlit"),
]);

/// Decoder for Single Take mode status
pub const SINGLE_TAKE: SimpleValueDecoder<i16> =
    SimpleValueDecoder::new(&[(0, "Off"), (1, "Recording"), (2, "Processing")]);

/// Decoder for Portrait mode effect type
pub const PORTRAIT_EFFECT: SimpleValueDecoder<i16> = SimpleValueDecoder::new(&[
    (0, "None"),
    (1, "Blur"),
    (2, "Spin"),
    (3, "Zoom"),
    (4, "Color Point"),
    (5, "Glitch"),
]);

/// Decoder for multi-camera lens type
pub const LENS_TYPE: SimpleValueDecoder<i16> = SimpleValueDecoder::new(&[
    (0, "Wide (Main)"),
    (1, "Ultra Wide"),
    (2, "Telephoto"),
    (3, "Front Camera"),
    (4, "Telephoto 3x"),
    (5, "Telephoto 10x"),
]);

/// Decodes digital zoom level (10 = 1.0x, 100 = 10.0x)
pub fn decode_zoom_level(value: i16) -> String {
    if value <= 0 {
        return "1.0x".to_string();
    }
    let zoom = value as f32 / 10.0;
    format!("{:.1}x", zoom)
}

/// Decodes binary on/off values (value > 0 = On)
pub fn decode_binary_onoff(value: i16) -> String {
    ON_OFF.decode(if value > 0 { 1 } else { 0 })
}

// ============================================================================
// Tag Registry
// ============================================================================

/// Static registry containing all Samsung MakerNote tag definitions
///
/// This registry eliminates code duplication by centralizing all tag definitions,
/// reducing the need for large match statements in the parser implementation.
pub static SAMSUNG_TAGS: Lazy<TagRegistry> = Lazy::new(|| {
    TagRegistry::with_capacity(20)
        // Tags with custom decoders
        .register_simple_i16(SAMSUNG_SCENE_OPTIMIZER, "SceneOptimizer", &SCENE_OPTIMIZER)
        .register_simple_i16(SAMSUNG_SCENE_TYPE, "SceneType", &SCENE_TYPE)
        .register_simple_i16(SAMSUNG_SINGLE_TAKE, "SingleTake", &SINGLE_TAKE)
        .register_simple_i16(SAMSUNG_PORTRAIT_EFFECT, "PortraitEffect", &PORTRAIT_EFFECT)
        .register_simple_i16(SAMSUNG_LENS_TYPE, "LensType", &LENS_TYPE)
        .register_i16(SAMSUNG_ZOOM_LEVEL, "ZoomLevel", decode_zoom_level)
        // Raw value tag (no decoder)
        .register_raw(SAMSUNG_SINGLE_TAKE_FRAME, "SingleTakeFrame")
        // Binary on/off tags
        .register_i16(SAMSUNG_EXPERT_RAW, "ExpertRAW", decode_binary_onoff)
        .register_i16(
            SAMSUNG_MULTI_FRAME_NR,
            "MultiFrameNoiseReduction",
            decode_binary_onoff,
        )
        .register_i16(SAMSUNG_DIRECTORS_VIEW, "DirectorsView", decode_binary_onoff)
        .register_i16(SAMSUNG_PRO_MODE, "ProMode", decode_binary_onoff)
        .register_i16(
            SAMSUNG_OBJECT_TRACKING,
            "ObjectTracking",
            decode_binary_onoff,
        )
        .register_i16(SAMSUNG_NIGHT_MODE, "NightMode", decode_binary_onoff)
        .register_i16(
            SAMSUNG_NIGHT_HYPERLAPSE,
            "NightHyperlapse",
            decode_binary_onoff,
        )
        .register_i16(SAMSUNG_SUPER_STEADY, "SuperSteady", decode_binary_onoff)
        .register_i16(SAMSUNG_FOOD_MODE, "FoodMode", decode_binary_onoff)
});

/// Returns a reference to the Samsung tag registry
///
/// This function provides access to the centralized tag registry,
/// allowing the parser to look up tag names and decoders efficiently.
pub fn samsung_registry() -> &'static TagRegistry {
    &SAMSUNG_TAGS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_optimizer_decoder() {
        assert_eq!(SCENE_OPTIMIZER.decode(0), "Off");
        assert_eq!(SCENE_OPTIMIZER.decode(1), "On");
        assert_eq!(SCENE_OPTIMIZER.decode(2), "Auto");
    }

    #[test]
    fn test_scene_type_decoder() {
        assert_eq!(SCENE_TYPE.decode(0), "None");
        assert_eq!(SCENE_TYPE.decode(1), "Food");
        assert_eq!(SCENE_TYPE.decode(7), "Night");
    }

    #[test]
    fn test_single_take_decoder() {
        assert_eq!(SINGLE_TAKE.decode(0), "Off");
        assert_eq!(SINGLE_TAKE.decode(1), "Recording");
    }

    #[test]
    fn test_portrait_effect_decoder() {
        assert_eq!(PORTRAIT_EFFECT.decode(0), "None");
        assert_eq!(PORTRAIT_EFFECT.decode(1), "Blur");
        assert_eq!(PORTRAIT_EFFECT.decode(4), "Color Point");
    }

    #[test]
    fn test_lens_type_decoder() {
        assert_eq!(LENS_TYPE.decode(0), "Wide (Main)");
        assert_eq!(LENS_TYPE.decode(1), "Ultra Wide");
        assert_eq!(LENS_TYPE.decode(5), "Telephoto 10x");
    }

    #[test]
    fn test_decode_zoom_level() {
        assert_eq!(decode_zoom_level(10), "1.0x");
        assert_eq!(decode_zoom_level(100), "10.0x");
        assert_eq!(decode_zoom_level(35), "3.5x");
    }

    #[test]
    fn test_decode_binary_onoff() {
        assert_eq!(decode_binary_onoff(0), "Off");
        assert_eq!(decode_binary_onoff(1), "On");
        assert_eq!(decode_binary_onoff(5), "On");
    }

    #[test]
    fn test_registry_has_all_tags() {
        let registry = samsung_registry();
        assert!(registry.has_tag(SAMSUNG_SCENE_OPTIMIZER));
        assert!(registry.has_tag(SAMSUNG_SCENE_TYPE));
        assert!(registry.has_tag(SAMSUNG_SINGLE_TAKE));
        assert!(registry.has_tag(SAMSUNG_SINGLE_TAKE_FRAME));
        assert!(registry.has_tag(SAMSUNG_EXPERT_RAW));
        assert!(registry.has_tag(SAMSUNG_MULTI_FRAME_NR));
        assert!(registry.has_tag(SAMSUNG_DIRECTORS_VIEW));
        assert!(registry.has_tag(SAMSUNG_PRO_MODE));
        assert!(registry.has_tag(SAMSUNG_OBJECT_TRACKING));
        assert!(registry.has_tag(SAMSUNG_NIGHT_MODE));
        assert!(registry.has_tag(SAMSUNG_NIGHT_HYPERLAPSE));
        assert!(registry.has_tag(SAMSUNG_SUPER_STEADY));
        assert!(registry.has_tag(SAMSUNG_FOOD_MODE));
        assert!(registry.has_tag(SAMSUNG_PORTRAIT_EFFECT));
        assert!(registry.has_tag(SAMSUNG_LENS_TYPE));
        assert!(registry.has_tag(SAMSUNG_ZOOM_LEVEL));
    }

    #[test]
    fn test_registry_count() {
        let registry = samsung_registry();
        assert_eq!(registry.len(), 16);
    }
}
