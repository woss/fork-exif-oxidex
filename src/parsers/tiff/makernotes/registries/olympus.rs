//! Olympus tag registry with array schemas
//!
//! Supports both Four Thirds (E-series DSLRs) and Micro Four Thirds (OM-D, PEN) cameras.
//! Based on ExifTool's Olympus.pm module.

use crate::io::EndianReader;

use super::super::shared::{
    array_schemas::*, generic_decoders::ON_OFF_I32, tag_registry::TagRegistry,
};

// Re-export existing decoders from olympus.rs (need to make these public)
use super::super::olympus::{
    ART_FILTER_DECODER, COLOR_SPACE_DECODER, EXPOSURE_MODE_DECODER, FLASH_MODE_DECODER,
    FOCUS_MODE_DECODER, METERING_MODE_DECODER, NOISE_REDUCTION_DECODER, PICTURE_MODE_DECODER,
    SCENE_MODE_DECODER, WHITE_BALANCE_DECODER,
};

// ============================================================================
// ARRAY SCHEMAS
// ============================================================================

/// CameraSettings array schema (Tag 0x0003)
/// Contains 49+ camera configuration and image processing settings
static CAMERA_SETTINGS_SCHEMA: ArraySchema = ArraySchema {
    name: "CameraSettings",
    indices: &[
        ArrayIndexDef::raw(0, "PreviewImageValid"),
        ArrayIndexDef::raw(1, "PreviewImageStart"),
        ArrayIndexDef::raw(2, "PreviewImageLength"),
        ArrayIndexDef::with_i32_decoder(3, "ExposureMode", &EXPOSURE_MODE_DECODER),
        ArrayIndexDef::raw(4, "AELock"),
        ArrayIndexDef::with_i32_decoder(5, "MeteringMode", &METERING_MODE_DECODER),
        ArrayIndexDef::raw(6, "MacroMode"),
        ArrayIndexDef::with_i32_decoder(7, "FocusMode", &FOCUS_MODE_DECODER),
        ArrayIndexDef::raw(8, "FocusProcess"),
        ArrayIndexDef::raw(9, "AFSearch"),
        ArrayIndexDef::raw(10, "AFAreas"),
        ArrayIndexDef::raw(11, "AFPointSelected"),
        ArrayIndexDef::raw(12, "ExposureCompensation"),
        ArrayIndexDef::raw(13, "CenterWeightedArea"),
        ArrayIndexDef::raw(14, "AEBracketStep"),
        ArrayIndexDef::raw(15, "AEBracketXval"),
        ArrayIndexDef::with_i32_decoder(16, "FlashMode", &FLASH_MODE_DECODER),
        ArrayIndexDef::raw(17, "FlashExposureComp"),
        ArrayIndexDef::raw(18, "FlashRemoteControl"),
        ArrayIndexDef::raw(19, "FlashControlMode"),
        ArrayIndexDef::raw(20, "FlashIntensity"),
        ArrayIndexDef::with_i32_decoder(21, "WhiteBalance", &WHITE_BALANCE_DECODER),
        ArrayIndexDef::raw(22, "WhiteBalanceTemperature"),
        ArrayIndexDef::raw(23, "WhiteBalanceBracket"),
        ArrayIndexDef::raw(24, "CustomSaturation"),
        ArrayIndexDef::raw(25, "ModifiedSaturation"),
        ArrayIndexDef::raw(26, "ContrastSetting"),
        ArrayIndexDef::raw(27, "SharpnessSetting"),
        ArrayIndexDef::with_i32_decoder(28, "ColorSpace", &COLOR_SPACE_DECODER),
        ArrayIndexDef::with_i32_decoder(29, "SceneMode", &SCENE_MODE_DECODER),
        ArrayIndexDef::with_i32_decoder(30, "NoiseReduction", &NOISE_REDUCTION_DECODER),
        ArrayIndexDef::with_i32_decoder(31, "DistortionCorrection", &ON_OFF_I32),
        ArrayIndexDef::with_i32_decoder(32, "ShadingCompensation", &ON_OFF_I32),
        ArrayIndexDef::raw(33, "CompressionFactor"),
        ArrayIndexDef::raw(34, "Gradation"),
        ArrayIndexDef::with_i32_decoder(35, "PictureMode", &PICTURE_MODE_DECODER),
        ArrayIndexDef::raw(36, "PictureModeSaturation"),
        ArrayIndexDef::raw(37, "PictureModeContrast"),
        ArrayIndexDef::raw(38, "PictureModeSharpness"),
        ArrayIndexDef::raw(39, "PictureModeBWFilter"),
        ArrayIndexDef::raw(40, "PictureModeTone"),
        ArrayIndexDef::raw(41, "NoiseFilter"),
        ArrayIndexDef::with_i32_decoder(42, "ArtFilter", &ART_FILTER_DECODER),
        ArrayIndexDef::raw(43, "MagicFilter"),
        ArrayIndexDef::raw(44, "PictureModeEffect"),
        ArrayIndexDef::raw(45, "ToneCurve"),
        ArrayIndexDef::raw(46, "ToneLevel"),
        ArrayIndexDef::raw(47, "SharpnessFactor"),
        ArrayIndexDef::raw(48, "WBFRBBracket"),
    ],
};

/// Equipment array schema (Tag 0x0201)
/// Contains equipment information including body, lens, and flash details
/// Note: This is a byte array with complex internal structure
static EQUIPMENT_SCHEMA: ArraySchema = ArraySchema {
    name: "Equipment",
    indices: &[
        ArrayIndexDef::raw(0, "EquipmentVersion"),
        // Indices 1-19 are handled specially in parse_equipment function
        // due to complex byte-level parsing requirements
    ],
};

// ============================================================================
// TAG REGISTRY
// ============================================================================

/// Create Olympus tag registry with all tag definitions and array schemas
pub fn olympus_registry() -> TagRegistry {
    TagRegistry::new()
        // Simple string tags
        .register_raw(0x0005, "SoftwareRelease")
        .register_raw(0x0007, "CameraID")
        .register_raw(0x0404, "BodyFirmwareVersion")
        .register_raw(0x0206, "LensModel")
        // Simple integer tags
        .register_raw(0x0001, "Quality")
        .register_raw(0x0002, "MacroMode")
        .register_raw(0x0004, "DigitalZoom")
        .register_raw(0x0008, "ImageWidth")
        .register_raw(0x0009, "ImageHeight")
        .register_raw(0x000A, "OriginalManufacturerModel")
        // Array-based tags
        .register_array_schema(0x0003, &CAMERA_SETTINGS_SCHEMA)
        .register_array_schema(0x0201, &EQUIPMENT_SCHEMA)
        // Complex sub-IFD tags (not processed via simple schemas)
        .register_raw(0x0202, "CameraSettings2")
        .register_raw(0x0203, "RawDevelopment")
        .register_raw(0x0204, "ImageProcessing")
        .register_raw(0x0205, "FocusInfo")
        .register_raw(0x0207, "RawInfo")
        .register_raw(0x0208, "MainInfo")
        // Image preview/thumbnail tags
        .register_raw(0x0100, "PreviewImage")
        .register_raw(0x0104, "ThumbnailImage")
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Process Equipment array with special lens lookup handling
///
/// Equipment array has complex byte-level structure that requires custom parsing
/// beyond what ArraySchema can provide. This handles serial numbers, firmware
/// versions, lens data, and aperture/focal length information.
pub fn process_equipment_with_lens(
    array: &[u8],
    prefix: &str,
    lens_db: &impl super::super::shared::LensDatabase,
    byte_order: super::super::super::ifd_parser::ByteOrder,
    tags: &mut std::collections::HashMap<String, String>,
) {
    // Create EndianReader for the array data
    let reader = EndianReader::new(array, byte_order.to_io_byte_order());

    // Serial number (8 bytes starting at offset 2)
    if array.len() >= 10 {
        let serial_bytes = &array[2..10];
        if let Ok(serial) = std::str::from_utf8(serial_bytes) {
            let serial_str = serial.trim_end_matches('\0').trim();
            if !serial_str.is_empty() {
                tags.insert(format!("{}:SerialNumber", prefix), serial_str.to_string());
            }
        }
    }

    // Body firmware version (5 bytes starting at offset 10)
    if array.len() >= 15 {
        let fw_bytes = &array[10..15];
        if let Ok(fw) = std::str::from_utf8(fw_bytes) {
            let fw_str = fw.trim_end_matches('\0').trim();
            if !fw_str.is_empty() {
                tags.insert(
                    format!("{}:BodyFirmwareVersion", prefix),
                    fw_str.to_string(),
                );
            }
        }
    }

    // Lens type (2 bytes at offset 16) - uses lens database
    if array.len() >= 18 {
        let lens_id = reader.u16_at(16).unwrap_or(0);

        if lens_id != 0 {
            if let Some(lens_name) = lens_db.lookup(lens_id) {
                tags.insert(format!("{}:LensType", prefix), lens_name.to_string());
            } else {
                tags.insert(format!("{}:LensID", prefix), lens_id.to_string());
            }
        }
    }

    // Lens serial number (8 bytes at offset 18)
    if array.len() >= 26 {
        let lens_serial_bytes = &array[18..26];
        if let Ok(lens_serial) = std::str::from_utf8(lens_serial_bytes) {
            let lens_serial_str = lens_serial.trim_end_matches('\0').trim();
            if !lens_serial_str.is_empty() {
                tags.insert(
                    format!("{}:LensSerialNumber", prefix),
                    lens_serial_str.to_string(),
                );
            }
        }
    }

    // Max aperture at min focal (2 bytes at offset 52)
    if array.len() >= 54 {
        let max_ap_min = reader.u16_at(52).unwrap_or(0);
        if max_ap_min > 0 {
            let f_stop = (max_ap_min as f32) / 10.0;
            tags.insert(
                format!("{}:MaxApertureAtMinFocal", prefix),
                format!("f/{:.1}", f_stop),
            );
        }
    }

    // Max aperture at max focal (2 bytes at offset 54)
    if array.len() >= 56 {
        let max_ap_max = reader.u16_at(54).unwrap_or(0);
        if max_ap_max > 0 {
            let f_stop = (max_ap_max as f32) / 10.0;
            tags.insert(
                format!("{}:MaxApertureAtMaxFocal", prefix),
                format!("f/{:.1}", f_stop),
            );
        }
    }

    // Min focal length (2 bytes at offset 56)
    if array.len() >= 58 {
        let min_focal = reader.u16_at(56).unwrap_or(0);
        if min_focal > 0 {
            tags.insert(
                format!("{}:MinFocalLength", prefix),
                format!("{} mm", min_focal),
            );
        }
    }

    // Max focal length (2 bytes at offset 58)
    if array.len() >= 60 {
        let max_focal = reader.u16_at(58).unwrap_or(0);
        if max_focal > 0 {
            tags.insert(
                format!("{}:MaxFocalLength", prefix),
                format!("{} mm", max_focal),
            );
        }
    }
}
