//! ExifTool Compatibility Formatting Module
//!
//! This module provides a single entry point for transforming raw parsed metadata values
//! into ExifTool-compatible formatted strings. ExifTool is the de facto standard for
//! metadata extraction, and maintaining output compatibility ensures users can rely on
//! consistent behavior when migrating from or comparing against ExifTool.
//!
//! # Architecture
//!
//! The formatting pipeline consists of:
//!
//! 1. **`format_for_exiftool()`** - Main entry point that iterates over all metadata
//!    and applies tag-specific formatting rules.
//!
//! 2. **`format_tag_value()`** - Dispatch function that examines tag names and routes
//!    values to the appropriate formatter based on a priority-ordered rule set.
//!
//! 3. **Helper functions** - Tag name classification utilities that determine which
//!    formatting rules apply to each tag.
//!
//! # Formatting Rules (in priority order)
//!
//! 1. GPS latitude/longitude references (N/S/E/W -> North/South/East/West)
//! 2. GPS direction references (T/M -> True North/Magnetic North)
//! 3. GPS speed/distance references (K/M/N -> km/h/mph/knots)
//! 4. GPS status tags (GPSStatus, GPSMeasureMode, GPSDifferential)
//! 5. GPS altitude reference (binary 0x00/0x01 -> Above/Below Sea Level)
//! 6. GPS processing method (binary data with encoding prefix)
//! 7. Binary decoders (CFAPattern, SceneType, version bytes)
//! 8. Enum tags (ExposureProgram integer -> string description)
//! 9. Unit suffixes (FocalLength -> "X mm", GPSAltitude -> "X m")
//! 10. Default: return original value unchanged
//!
//! # Example
//!
//! ```rust,ignore
//! use oxidex::core::{MetadataMap, TagValue};
//! use oxidex::core::exiftool_compat::format_for_exiftool;
//!
//! let mut metadata = MetadataMap::new();
//! metadata.insert("EXIF:GPSLatitudeRef", TagValue::String("N".to_string()));
//! metadata.insert("EXIF:FocalLength", TagValue::String("50".to_string()));
//!
//! let formatted = format_for_exiftool(&metadata);
//!
//! assert_eq!(formatted.get_string("EXIF:GPSLatitudeRef"), Some("North"));
//! assert_eq!(formatted.get_string("EXIF:FocalLength"), Some("50 mm"));
//! ```

use crate::core::formatters::{
    decode_cfa_pattern, decode_gps_processing_method, decode_scene_type, decode_version_bytes,
    format_exposure_program, format_gps_altitude_ref, format_gps_direction_ref,
    format_gps_lat_ref, format_gps_lon_ref, format_gps_speed_ref, format_with_unit,
};
use crate::core::formatters::gps_speed_ref::format_gps_dest_distance_ref;
use crate::core::formatters::gps_status::{format_gps_differential, format_gps_measure_mode, format_gps_status};
use crate::core::{MetadataMap, TagValue};

// =============================================================================
// MAIN PUBLIC API
// =============================================================================

/// Transforms all values in a MetadataMap to ExifTool-compatible formatted strings.
///
/// This is the main entry point for ExifTool compatibility formatting. It iterates
/// over every tag in the input metadata and applies the appropriate formatting rule
/// based on the tag name. Values that don't match any formatting rule are passed
/// through unchanged.
///
/// # Arguments
///
/// * `metadata` - Reference to the source MetadataMap containing raw parsed values
///
/// # Returns
///
/// A new MetadataMap with all values formatted for ExifTool compatibility.
/// Tag names are preserved exactly as they appear in the input.
///
/// # Performance
///
/// This function creates a new MetadataMap rather than modifying in place to
/// maintain immutability semantics. For large metadata sets, the overhead is
/// minimal compared to the formatting operations themselves.
///
/// # Example
///
/// ```rust,ignore
/// use oxidex::core::{MetadataMap, TagValue};
/// use oxidex::core::exiftool_compat::format_for_exiftool;
///
/// let mut metadata = MetadataMap::new();
/// metadata.insert("EXIF:ExposureProgram", TagValue::Integer(2));
/// metadata.insert("GPS:GPSLatitudeRef", TagValue::String("N".to_string()));
///
/// let formatted = format_for_exiftool(&metadata);
///
/// // ExposureProgram 2 -> "Program AE"
/// assert_eq!(formatted.get_string("EXIF:ExposureProgram"), Some("Program AE"));
/// // GPSLatitudeRef "N" -> "North"
/// assert_eq!(formatted.get_string("GPS:GPSLatitudeRef"), Some("North"));
/// ```
pub fn format_for_exiftool(metadata: &MetadataMap) -> MetadataMap {
    let mut result = MetadataMap::with_capacity(metadata.len());

    for (tag_name, value) in metadata.iter() {
        let formatted_value = format_tag_value(tag_name, value);
        result.insert(tag_name.clone(), formatted_value);
    }

    result
}

// =============================================================================
// TAG VALUE DISPATCH
// =============================================================================

/// Formats a single tag value based on the tag name.
///
/// This function implements the priority-ordered dispatch logic that determines
/// which formatter to apply to a given tag value. The dispatch order is designed
/// to handle the most specific cases first, falling back to more general rules.
///
/// # Dispatch Priority
///
/// 1. GPS string references (GPSLatitudeRef, GPSLongitudeRef)
/// 2. GPS direction refs (GPSImgDirectionRef, GPSDestBearingRef, GPSTrackRef)
/// 3. GPS speed/distance refs (GPSSpeedRef, GPSDestDistanceRef)
/// 4. GPS status tags (GPSStatus, GPSMeasureMode, GPSDifferential)
/// 5. GPS altitude ref (GPSAltitudeRef for binary 0x00/0x01)
/// 6. GPS processing method (GPSProcessingMethod for binary data)
/// 7. Binary decoders (CFAPattern, SceneType, version tags)
/// 8. Enum tags (ExposureProgram)
/// 9. Unit suffixes (FocalLength, GPSAltitude)
/// 10. Default: return original value unchanged
///
/// # Arguments
///
/// * `tag_name` - The full tag name, optionally with family prefix (e.g., "EXIF:FocalLength")
/// * `value` - The raw TagValue to format
///
/// # Returns
///
/// A new TagValue containing the formatted result. If no formatting rule applies,
/// returns a clone of the original value.
pub fn format_tag_value(tag_name: &str, value: &TagValue) -> TagValue {
    let base_name = strip_family_prefix(tag_name);

    // ---------------------------------------------------------------------
    // Rule 1: GPS Latitude/Longitude References
    // Convert single-character direction codes to full names
    // ---------------------------------------------------------------------
    if is_gps_lat_ref(base_name) {
        if let Some(s) = value.as_string() {
            return TagValue::String(format_gps_lat_ref(s));
        }
    }

    if is_gps_lon_ref(base_name) {
        if let Some(s) = value.as_string() {
            return TagValue::String(format_gps_lon_ref(s));
        }
    }

    // ---------------------------------------------------------------------
    // Rule 2: GPS Direction References (True North / Magnetic North)
    // ---------------------------------------------------------------------
    if is_gps_direction_ref(base_name) {
        if let Some(s) = value.as_string() {
            return TagValue::String(format_gps_direction_ref(s));
        }
    }

    // ---------------------------------------------------------------------
    // Rule 3: GPS Speed and Distance References
    // ---------------------------------------------------------------------
    if is_gps_speed_ref(base_name) {
        if let Some(s) = value.as_string() {
            if let Some(formatted) = format_gps_speed_ref(s) {
                return TagValue::String(formatted);
            }
        }
    }

    if is_gps_dest_distance_ref(base_name) {
        if let Some(s) = value.as_string() {
            if let Some(formatted) = format_gps_dest_distance_ref(s) {
                return TagValue::String(formatted);
            }
        }
    }

    // ---------------------------------------------------------------------
    // Rule 4: GPS Status Tags (GPSStatus, GPSMeasureMode, GPSDifferential)
    // ---------------------------------------------------------------------
    if is_gps_status_tag(base_name) {
        if let Some(s) = value.as_string() {
            let formatted = match base_name {
                "GPSStatus" => format_gps_status(s),
                "GPSMeasureMode" => format_gps_measure_mode(s),
                "GPSDifferential" => format_gps_differential(s),
                _ => None,
            };
            if let Some(f) = formatted {
                return TagValue::String(f);
            }
        }
    }

    // ---------------------------------------------------------------------
    // Rule 5: GPS Altitude Reference (binary 0x00/0x01)
    // ---------------------------------------------------------------------
    if is_gps_altitude_ref(base_name) {
        // Handle string values ("0", "1", "\x00", "\x01")
        if let Some(s) = value.as_string() {
            if let Some(formatted) = format_gps_altitude_ref(s) {
                return TagValue::String(formatted);
            }
        }
        // Handle binary values (single byte)
        if let TagValue::Binary(data) = value {
            if !data.is_empty() {
                // Convert first byte to string for the formatter
                let byte_str = match data[0] {
                    0 => "0",
                    1 => "1",
                    _ => return value.clone(),
                };
                if let Some(formatted) = format_gps_altitude_ref(byte_str) {
                    return TagValue::String(formatted);
                }
            }
        }
        // Handle integer values
        if let Some(i) = value.as_integer() {
            let int_str = match i {
                0 => "0",
                1 => "1",
                _ => return value.clone(),
            };
            if let Some(formatted) = format_gps_altitude_ref(int_str) {
                return TagValue::String(formatted);
            }
        }
    }

    // ---------------------------------------------------------------------
    // Rule 6: GPS Processing Method (binary data with encoding prefix)
    // ---------------------------------------------------------------------
    if is_gps_processing_method(base_name) {
        if let TagValue::Binary(data) = value {
            let decoded = decode_gps_processing_method(data);
            if !decoded.is_empty() {
                return TagValue::String(decoded);
            }
        }
    }

    // ---------------------------------------------------------------------
    // Rule 7: Binary Decoders (CFAPattern, SceneType, version bytes)
    // ---------------------------------------------------------------------
    if is_cfa_pattern(base_name) {
        if let TagValue::Binary(data) = value {
            return TagValue::String(decode_cfa_pattern(data));
        }
    }

    if is_scene_type(base_name) {
        if let TagValue::Binary(data) = value {
            let decoded = decode_scene_type(data);
            if !decoded.is_empty() {
                return TagValue::String(decoded);
            }
        }
        // Also handle integer values for SceneType
        if let Some(i) = value.as_integer() {
            if i == 1 {
                return TagValue::String("Directly photographed".to_string());
            } else {
                return TagValue::String(format!("Unknown ({})", i));
            }
        }
    }

    if is_version_tag(base_name) {
        if let TagValue::Binary(data) = value {
            let decoded = decode_version_bytes(data);
            if !decoded.is_empty() {
                return TagValue::String(decoded);
            }
        }
    }

    // ---------------------------------------------------------------------
    // Rule 8: Enum Tags (ExposureProgram)
    // Convert integer enum values to human-readable strings
    // ---------------------------------------------------------------------
    if is_exposure_program(base_name) {
        if let Some(i) = value.as_integer() {
            // ExposureProgram values are typically small positive integers
            // Safe to cast from i64 to u32 for the formatter
            let formatted = format_exposure_program(i as u32);
            return TagValue::String(formatted);
        }
    }

    // ---------------------------------------------------------------------
    // Rule 9: Unit Suffixes (FocalLength -> mm, GPSAltitude -> m)
    // ---------------------------------------------------------------------
    if is_unit_suffix_tag(base_name) {
        // For string values, apply unit suffix directly
        if let Some(s) = value.as_string() {
            return TagValue::String(format_with_unit(tag_name, s));
        }
        // For numeric values, convert to string first then apply suffix
        if let Some(i) = value.as_integer() {
            let formatted = format_with_unit(tag_name, &i.to_string());
            return TagValue::String(formatted);
        }
        if let Some(f) = value.as_float() {
            // Format floats reasonably - avoid excessive decimal places
            let float_str = if f.fract() == 0.0 {
                format!("{:.0}", f)
            } else {
                format!("{}", f)
            };
            let formatted = format_with_unit(tag_name, &float_str);
            return TagValue::String(formatted);
        }
        // Handle Rational values
        if let TagValue::Rational { numerator, denominator } = value {
            if *denominator != 0 {
                let float_val = *numerator as f64 / *denominator as f64;
                let float_str = if float_val.fract() == 0.0 {
                    format!("{:.0}", float_val)
                } else {
                    format!("{}", float_val)
                };
                let formatted = format_with_unit(tag_name, &float_str);
                return TagValue::String(formatted);
            }
        }
    }

    // ---------------------------------------------------------------------
    // Rule 10: Default - Return original value unchanged
    // ---------------------------------------------------------------------
    value.clone()
}

// =============================================================================
// HELPER FUNCTIONS - Tag Name Classification
// =============================================================================

/// Strips the family/group prefix from a tag name.
///
/// Tag names may include a family prefix separated by a colon (e.g., "EXIF:Make",
/// "GPS:GPSLatitude", "XMP:Creator"). This function extracts just the base tag
/// name for comparison against formatting rules.
///
/// # Arguments
///
/// * `tag_name` - The full tag name, possibly with a family prefix
///
/// # Returns
///
/// The base tag name without any prefix. If there's no colon, returns the
/// original string unchanged.
///
/// # Examples
///
/// ```rust,ignore
/// assert_eq!(strip_family_prefix("EXIF:FocalLength"), "FocalLength");
/// assert_eq!(strip_family_prefix("GPS:GPSLatitude"), "GPSLatitude");
/// assert_eq!(strip_family_prefix("FocalLength"), "FocalLength");
/// ```
pub fn strip_family_prefix(tag_name: &str) -> &str {
    // Find the last colon to handle nested prefixes like "Composite:EXIF:Tag"
    tag_name.rsplit(':').next().unwrap_or(tag_name)
}

/// Checks if the tag is a GPS latitude reference (GPSLatitudeRef).
///
/// # Arguments
///
/// * `base_name` - The tag name without family prefix
///
/// # Returns
///
/// `true` if this tag should be formatted as a latitude reference
pub fn is_gps_lat_ref(base_name: &str) -> bool {
    base_name == "GPSLatitudeRef"
}

/// Checks if the tag is a GPS longitude reference (GPSLongitudeRef).
///
/// # Arguments
///
/// * `base_name` - The tag name without family prefix
///
/// # Returns
///
/// `true` if this tag should be formatted as a longitude reference
pub fn is_gps_lon_ref(base_name: &str) -> bool {
    base_name == "GPSLongitudeRef"
}

/// Checks if the tag is a GPS direction reference.
///
/// Direction reference tags indicate whether a direction measurement is relative
/// to True North or Magnetic North. Applicable tags:
/// - GPSImgDirectionRef
/// - GPSDestBearingRef
/// - GPSTrackRef
///
/// # Arguments
///
/// * `base_name` - The tag name without family prefix
///
/// # Returns
///
/// `true` if this tag should be formatted as a direction reference
pub fn is_gps_direction_ref(base_name: &str) -> bool {
    matches!(
        base_name,
        "GPSImgDirectionRef" | "GPSDestBearingRef" | "GPSTrackRef"
    )
}

/// Checks if the tag is a GPS speed reference (GPSSpeedRef).
///
/// # Arguments
///
/// * `base_name` - The tag name without family prefix
///
/// # Returns
///
/// `true` if this tag should be formatted as a speed reference
pub fn is_gps_speed_ref(base_name: &str) -> bool {
    base_name == "GPSSpeedRef"
}

/// Checks if the tag is a GPS destination distance reference (GPSDestDistanceRef).
///
/// # Arguments
///
/// * `base_name` - The tag name without family prefix
///
/// # Returns
///
/// `true` if this tag should be formatted as a distance reference
pub fn is_gps_dest_distance_ref(base_name: &str) -> bool {
    base_name == "GPSDestDistanceRef"
}

/// Checks if the tag is a GPS status-related tag.
///
/// Status tags include:
/// - GPSStatus (measurement active/void)
/// - GPSMeasureMode (2D/3D measurement)
/// - GPSDifferential (differential correction applied)
///
/// # Arguments
///
/// * `base_name` - The tag name without family prefix
///
/// # Returns
///
/// `true` if this tag should be formatted as a GPS status value
pub fn is_gps_status_tag(base_name: &str) -> bool {
    matches!(base_name, "GPSStatus" | "GPSMeasureMode" | "GPSDifferential")
}

/// Checks if the tag is a GPS altitude reference (GPSAltitudeRef).
///
/// # Arguments
///
/// * `base_name` - The tag name without family prefix
///
/// # Returns
///
/// `true` if this tag should be formatted as an altitude reference
pub fn is_gps_altitude_ref(base_name: &str) -> bool {
    base_name == "GPSAltitudeRef"
}

/// Checks if the tag is GPS processing method (GPSProcessingMethod).
///
/// # Arguments
///
/// * `base_name` - The tag name without family prefix
///
/// # Returns
///
/// `true` if this tag should be decoded as GPS processing method binary data
pub fn is_gps_processing_method(base_name: &str) -> bool {
    base_name == "GPSProcessingMethod"
}

/// Checks if the tag is a CFA pattern (CFAPattern).
///
/// # Arguments
///
/// * `base_name` - The tag name without family prefix
///
/// # Returns
///
/// `true` if this tag should be decoded as CFA pattern binary data
pub fn is_cfa_pattern(base_name: &str) -> bool {
    // Also handle alternate spellings
    matches!(base_name, "CFAPattern" | "CFAPattern2")
}

/// Checks if the tag is scene type (SceneType).
///
/// # Arguments
///
/// * `base_name` - The tag name without family prefix
///
/// # Returns
///
/// `true` if this tag should be decoded as scene type
pub fn is_scene_type(base_name: &str) -> bool {
    base_name == "SceneType"
}

/// Checks if the tag is a version tag that stores data as ASCII bytes.
///
/// Version tags include:
/// - InteropVersion
/// - ExifVersion
/// - FlashpixVersion
/// - GPSVersionID
///
/// # Arguments
///
/// * `base_name` - The tag name without family prefix
///
/// # Returns
///
/// `true` if this tag should be decoded as version bytes
pub fn is_version_tag(base_name: &str) -> bool {
    matches!(
        base_name,
        "InteropVersion" | "ExifVersion" | "FlashpixVersion" | "GPSVersionID"
    )
}

/// Checks if the tag is ExposureProgram.
///
/// # Arguments
///
/// * `base_name` - The tag name without family prefix
///
/// # Returns
///
/// `true` if this tag should be formatted as an exposure program enum
pub fn is_exposure_program(base_name: &str) -> bool {
    base_name == "ExposureProgram"
}

/// Checks if the tag requires a unit suffix.
///
/// Tags that require unit suffixes:
/// - FocalLength, FocalLengthIn35mmFormat -> "mm"
/// - GPSAltitude, SubjectDistance -> "m"
///
/// Note: GPSAltitude is handled here for the unit suffix, not the reference value.
///
/// # Arguments
///
/// * `base_name` - The tag name without family prefix
///
/// # Returns
///
/// `true` if this tag should have a unit suffix appended
pub fn is_unit_suffix_tag(base_name: &str) -> bool {
    matches!(
        base_name,
        "FocalLength"
            | "FocalLengthIn35mmFormat"
            | "FocalLength35efl"
            | "FocalLengthIn35mmFilm"
            | "GPSAltitude"
            | "SubjectDistance"
            | "HyperfocalDistance"
    )
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // strip_family_prefix tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_strip_family_prefix_with_prefix() {
        assert_eq!(strip_family_prefix("EXIF:Make"), "Make");
        assert_eq!(strip_family_prefix("GPS:GPSLatitude"), "GPSLatitude");
        assert_eq!(strip_family_prefix("XMP:Creator"), "Creator");
        assert_eq!(strip_family_prefix("IPTC:Keywords"), "Keywords");
    }

    #[test]
    fn test_strip_family_prefix_without_prefix() {
        assert_eq!(strip_family_prefix("Make"), "Make");
        assert_eq!(strip_family_prefix("FocalLength"), "FocalLength");
    }

    #[test]
    fn test_strip_family_prefix_nested() {
        // Should take the last segment after the final colon
        assert_eq!(strip_family_prefix("Composite:EXIF:Tag"), "Tag");
    }

    #[test]
    fn test_strip_family_prefix_empty() {
        assert_eq!(strip_family_prefix(""), "");
    }

    // -------------------------------------------------------------------------
    // GPS Latitude/Longitude Reference tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_gps_lat_ref_formatting() {
        let value = TagValue::String("N".to_string());
        let formatted = format_tag_value("GPS:GPSLatitudeRef", &value);
        assert_eq!(formatted.as_string(), Some("North"));

        let value = TagValue::String("S".to_string());
        let formatted = format_tag_value("GPSLatitudeRef", &value);
        assert_eq!(formatted.as_string(), Some("South"));
    }

    #[test]
    fn test_gps_lon_ref_formatting() {
        let value = TagValue::String("E".to_string());
        let formatted = format_tag_value("GPS:GPSLongitudeRef", &value);
        assert_eq!(formatted.as_string(), Some("East"));

        let value = TagValue::String("W".to_string());
        let formatted = format_tag_value("GPSLongitudeRef", &value);
        assert_eq!(formatted.as_string(), Some("West"));
    }

    // -------------------------------------------------------------------------
    // GPS Direction Reference tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_gps_direction_ref_formatting() {
        let value = TagValue::String("T".to_string());
        let formatted = format_tag_value("GPS:GPSImgDirectionRef", &value);
        assert_eq!(formatted.as_string(), Some("True North"));

        let value = TagValue::String("M".to_string());
        let formatted = format_tag_value("GPSTrackRef", &value);
        assert_eq!(formatted.as_string(), Some("Magnetic North"));

        let value = TagValue::String("T".to_string());
        let formatted = format_tag_value("GPSDestBearingRef", &value);
        assert_eq!(formatted.as_string(), Some("True North"));
    }

    // -------------------------------------------------------------------------
    // GPS Speed/Distance Reference tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_gps_speed_ref_formatting() {
        let value = TagValue::String("K".to_string());
        let formatted = format_tag_value("GPS:GPSSpeedRef", &value);
        assert_eq!(formatted.as_string(), Some("km/h"));

        let value = TagValue::String("M".to_string());
        let formatted = format_tag_value("GPSSpeedRef", &value);
        assert_eq!(formatted.as_string(), Some("mph"));

        let value = TagValue::String("N".to_string());
        let formatted = format_tag_value("GPSSpeedRef", &value);
        assert_eq!(formatted.as_string(), Some("knots"));
    }

    #[test]
    fn test_gps_dest_distance_ref_formatting() {
        let value = TagValue::String("K".to_string());
        let formatted = format_tag_value("GPS:GPSDestDistanceRef", &value);
        assert_eq!(formatted.as_string(), Some("Kilometers"));

        let value = TagValue::String("M".to_string());
        let formatted = format_tag_value("GPSDestDistanceRef", &value);
        assert_eq!(formatted.as_string(), Some("Miles"));

        let value = TagValue::String("N".to_string());
        let formatted = format_tag_value("GPSDestDistanceRef", &value);
        assert_eq!(formatted.as_string(), Some("Nautical Miles"));
    }

    // -------------------------------------------------------------------------
    // GPS Status Tags tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_gps_status_formatting() {
        let value = TagValue::String("A".to_string());
        let formatted = format_tag_value("GPS:GPSStatus", &value);
        assert_eq!(formatted.as_string(), Some("Measurement Active"));

        let value = TagValue::String("V".to_string());
        let formatted = format_tag_value("GPSStatus", &value);
        assert_eq!(formatted.as_string(), Some("Measurement Void"));
    }

    #[test]
    fn test_gps_measure_mode_formatting() {
        let value = TagValue::String("2".to_string());
        let formatted = format_tag_value("GPS:GPSMeasureMode", &value);
        assert_eq!(formatted.as_string(), Some("2-Dimensional Measurement"));

        let value = TagValue::String("3".to_string());
        let formatted = format_tag_value("GPSMeasureMode", &value);
        assert_eq!(formatted.as_string(), Some("3-Dimensional Measurement"));
    }

    #[test]
    fn test_gps_differential_formatting() {
        let value = TagValue::String("0".to_string());
        let formatted = format_tag_value("GPS:GPSDifferential", &value);
        assert_eq!(formatted.as_string(), Some("No Correction"));

        let value = TagValue::String("1".to_string());
        let formatted = format_tag_value("GPSDifferential", &value);
        assert_eq!(formatted.as_string(), Some("Differential Corrected"));
    }

    // -------------------------------------------------------------------------
    // GPS Altitude Reference tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_gps_altitude_ref_from_string() {
        let value = TagValue::String("0".to_string());
        let formatted = format_tag_value("GPS:GPSAltitudeRef", &value);
        assert_eq!(formatted.as_string(), Some("Above Sea Level"));

        let value = TagValue::String("1".to_string());
        let formatted = format_tag_value("GPSAltitudeRef", &value);
        assert_eq!(formatted.as_string(), Some("Below Sea Level"));
    }

    #[test]
    fn test_gps_altitude_ref_from_binary() {
        let value = TagValue::Binary(vec![0]);
        let formatted = format_tag_value("GPS:GPSAltitudeRef", &value);
        assert_eq!(formatted.as_string(), Some("Above Sea Level"));

        let value = TagValue::Binary(vec![1]);
        let formatted = format_tag_value("GPSAltitudeRef", &value);
        assert_eq!(formatted.as_string(), Some("Below Sea Level"));
    }

    #[test]
    fn test_gps_altitude_ref_from_integer() {
        let value = TagValue::Integer(0);
        let formatted = format_tag_value("GPS:GPSAltitudeRef", &value);
        assert_eq!(formatted.as_string(), Some("Above Sea Level"));

        let value = TagValue::Integer(1);
        let formatted = format_tag_value("GPSAltitudeRef", &value);
        assert_eq!(formatted.as_string(), Some("Below Sea Level"));
    }

    // -------------------------------------------------------------------------
    // GPS Processing Method tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_gps_processing_method_formatting() {
        // ASCII-encoded "GPS" method
        let data = b"ASCII\0\0\0GPS\0\0\0\0\0".to_vec();
        let value = TagValue::Binary(data);
        let formatted = format_tag_value("GPS:GPSProcessingMethod", &value);
        assert_eq!(formatted.as_string(), Some("GPS"));
    }

    // -------------------------------------------------------------------------
    // Binary Decoder tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_cfa_pattern_formatting() {
        // 2x2 RGGB Bayer pattern
        let data = vec![0, 2, 0, 2, 0, 1, 1, 2];
        let value = TagValue::Binary(data);
        let formatted = format_tag_value("EXIF:CFAPattern", &value);
        assert_eq!(formatted.as_string(), Some("[Red,Green][Green,Blue]"));
    }

    #[test]
    fn test_scene_type_formatting() {
        // Binary value 1 = "Directly photographed"
        let value = TagValue::Binary(vec![1]);
        let formatted = format_tag_value("EXIF:SceneType", &value);
        assert_eq!(formatted.as_string(), Some("Directly photographed"));
    }

    #[test]
    fn test_scene_type_from_integer() {
        let value = TagValue::Integer(1);
        let formatted = format_tag_value("EXIF:SceneType", &value);
        assert_eq!(formatted.as_string(), Some("Directly photographed"));

        let value = TagValue::Integer(5);
        let formatted = format_tag_value("SceneType", &value);
        assert_eq!(formatted.as_string(), Some("Unknown (5)"));
    }

    #[test]
    fn test_version_bytes_formatting() {
        let data = b"0100".to_vec();
        let value = TagValue::Binary(data);
        let formatted = format_tag_value("EXIF:InteropVersion", &value);
        assert_eq!(formatted.as_string(), Some("0100"));

        let data = b"0232".to_vec();
        let value = TagValue::Binary(data);
        let formatted = format_tag_value("ExifVersion", &value);
        assert_eq!(formatted.as_string(), Some("0232"));

        let data = b"0100".to_vec();
        let value = TagValue::Binary(data);
        let formatted = format_tag_value("FlashpixVersion", &value);
        assert_eq!(formatted.as_string(), Some("0100"));

        let data = b"0230".to_vec();
        let value = TagValue::Binary(data);
        let formatted = format_tag_value("GPSVersionID", &value);
        assert_eq!(formatted.as_string(), Some("0230"));
    }

    // -------------------------------------------------------------------------
    // Enum Tag tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_exposure_program_formatting() {
        let value = TagValue::Integer(0);
        let formatted = format_tag_value("EXIF:ExposureProgram", &value);
        assert_eq!(formatted.as_string(), Some("Not Defined"));

        let value = TagValue::Integer(1);
        let formatted = format_tag_value("ExposureProgram", &value);
        assert_eq!(formatted.as_string(), Some("Manual"));

        let value = TagValue::Integer(2);
        let formatted = format_tag_value("ExposureProgram", &value);
        assert_eq!(formatted.as_string(), Some("Program AE"));

        let value = TagValue::Integer(3);
        let formatted = format_tag_value("ExposureProgram", &value);
        assert_eq!(formatted.as_string(), Some("Aperture-priority AE"));

        let value = TagValue::Integer(99);
        let formatted = format_tag_value("ExposureProgram", &value);
        assert_eq!(formatted.as_string(), Some("Unknown (99)"));
    }

    // -------------------------------------------------------------------------
    // Unit Suffix tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_focal_length_unit_suffix() {
        let value = TagValue::String("50".to_string());
        let formatted = format_tag_value("EXIF:FocalLength", &value);
        assert_eq!(formatted.as_string(), Some("50 mm"));

        let value = TagValue::String("31".to_string());
        let formatted = format_tag_value("FocalLengthIn35mmFormat", &value);
        assert_eq!(formatted.as_string(), Some("31 mm"));
    }

    #[test]
    fn test_focal_length_from_integer() {
        let value = TagValue::Integer(50);
        let formatted = format_tag_value("EXIF:FocalLength", &value);
        assert_eq!(formatted.as_string(), Some("50 mm"));
    }

    #[test]
    fn test_focal_length_from_float() {
        let value = TagValue::Float(50.0);
        let formatted = format_tag_value("EXIF:FocalLength", &value);
        assert_eq!(formatted.as_string(), Some("50 mm"));

        let value = TagValue::Float(35.5);
        let formatted = format_tag_value("FocalLength", &value);
        assert_eq!(formatted.as_string(), Some("35.5 mm"));
    }

    #[test]
    fn test_focal_length_from_rational() {
        let value = TagValue::Rational {
            numerator: 500,
            denominator: 10,
        };
        let formatted = format_tag_value("EXIF:FocalLength", &value);
        assert_eq!(formatted.as_string(), Some("50 mm"));
    }

    #[test]
    fn test_gps_altitude_unit_suffix() {
        let value = TagValue::String("117".to_string());
        let formatted = format_tag_value("GPS:GPSAltitude", &value);
        assert_eq!(formatted.as_string(), Some("117 m"));
    }

    #[test]
    fn test_subject_distance_unit_suffix() {
        let value = TagValue::String("2.5".to_string());
        let formatted = format_tag_value("EXIF:SubjectDistance", &value);
        assert_eq!(formatted.as_string(), Some("2.5 m"));
    }

    // -------------------------------------------------------------------------
    // Default behavior tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_unknown_tag_passes_through() {
        let value = TagValue::String("Canon".to_string());
        let formatted = format_tag_value("EXIF:Make", &value);
        assert_eq!(formatted.as_string(), Some("Canon"));

        let value = TagValue::Integer(400);
        let formatted = format_tag_value("ISO", &value);
        assert_eq!(formatted.as_integer(), Some(400));
    }

    // -------------------------------------------------------------------------
    // format_for_exiftool integration tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_format_for_exiftool_basic() {
        let mut metadata = MetadataMap::new();
        metadata.insert("EXIF:Make", TagValue::String("Canon".to_string()));
        metadata.insert("EXIF:ExposureProgram", TagValue::Integer(2));
        metadata.insert("GPS:GPSLatitudeRef", TagValue::String("N".to_string()));
        metadata.insert("EXIF:FocalLength", TagValue::String("50".to_string()));

        let formatted = format_for_exiftool(&metadata);

        // Make should pass through unchanged
        assert_eq!(formatted.get_string("EXIF:Make"), Some("Canon"));
        // ExposureProgram should be formatted
        assert_eq!(formatted.get_string("EXIF:ExposureProgram"), Some("Program AE"));
        // GPSLatitudeRef should be formatted
        assert_eq!(formatted.get_string("GPS:GPSLatitudeRef"), Some("North"));
        // FocalLength should have unit suffix
        assert_eq!(formatted.get_string("EXIF:FocalLength"), Some("50 mm"));
    }

    #[test]
    fn test_format_for_exiftool_preserves_count() {
        let mut metadata = MetadataMap::new();
        metadata.insert("Tag1", TagValue::String("Value1".to_string()));
        metadata.insert("Tag2", TagValue::Integer(42));
        metadata.insert("Tag3", TagValue::Float(3.14));

        let formatted = format_for_exiftool(&metadata);

        assert_eq!(formatted.len(), 3);
    }

    #[test]
    fn test_format_for_exiftool_empty() {
        let metadata = MetadataMap::new();
        let formatted = format_for_exiftool(&metadata);
        assert!(formatted.is_empty());
    }

    // -------------------------------------------------------------------------
    // Helper function classification tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_is_gps_lat_lon_ref() {
        assert!(is_gps_lat_ref("GPSLatitudeRef"));
        assert!(is_gps_lon_ref("GPSLongitudeRef"));
        assert!(!is_gps_lat_ref("GPSLongitudeRef"));
        assert!(!is_gps_lon_ref("GPSLatitudeRef"));
    }

    #[test]
    fn test_is_gps_direction_ref() {
        assert!(is_gps_direction_ref("GPSImgDirectionRef"));
        assert!(is_gps_direction_ref("GPSDestBearingRef"));
        assert!(is_gps_direction_ref("GPSTrackRef"));
        assert!(!is_gps_direction_ref("GPSLatitudeRef"));
        assert!(!is_gps_direction_ref("GPSSpeedRef"));
    }

    #[test]
    fn test_is_gps_status_tag() {
        assert!(is_gps_status_tag("GPSStatus"));
        assert!(is_gps_status_tag("GPSMeasureMode"));
        assert!(is_gps_status_tag("GPSDifferential"));
        assert!(!is_gps_status_tag("GPSAltitude"));
    }

    #[test]
    fn test_is_version_tag() {
        assert!(is_version_tag("InteropVersion"));
        assert!(is_version_tag("ExifVersion"));
        assert!(is_version_tag("FlashpixVersion"));
        assert!(is_version_tag("GPSVersionID"));
        assert!(!is_version_tag("SomeOtherVersion"));
    }

    #[test]
    fn test_is_unit_suffix_tag() {
        assert!(is_unit_suffix_tag("FocalLength"));
        assert!(is_unit_suffix_tag("FocalLengthIn35mmFormat"));
        assert!(is_unit_suffix_tag("GPSAltitude"));
        assert!(is_unit_suffix_tag("SubjectDistance"));
        assert!(!is_unit_suffix_tag("ISO"));
        assert!(!is_unit_suffix_tag("Make"));
    }
}
