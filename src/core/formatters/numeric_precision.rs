//! Numeric precision formatting for EXIF rational values
//!
//! This module provides formatting functions to match ExifTool's numeric precision
//! for various EXIF rational tags. Different tags use different decimal precisions:
//!
//! - **High precision (9 decimals)**: BrightnessValue, DigitalZoomRatio, CompressedBitsPerPixel
//! - **GPS precision (2 decimals with trailing zeros)**: GPSDestBearing, GPSImgDirection, GPSSpeed, GPSTrack
//! - **Resolution precision (up to 10 decimals)**: XResolution, YResolution
//!
//! # Background
//!
//! OxiDex was originally outputting 6 decimal places for all rational values,
//! but ExifTool uses tag-specific precision. This module provides the logic to
//! format EXIF rationals to match ExifTool's output exactly.
//!
//! # Examples
//!
//! ```
//! use oxidex::core::formatters::numeric_precision::format_exif_rational;
//!
//! // High precision tags (9 decimal places)
//! assert_eq!(format_exif_rational("BrightnessValue", 3.617254236), "3.617254236");
//!
//! // GPS tags (2 decimal places with trailing zeros)
//! assert_eq!(format_exif_rational("GPSImgDirection", 45.5), "45.50");
//!
//! // Resolution tags (only as many decimals as needed, up to 10)
//! assert_eq!(format_exif_rational("XResolution", 72.0), "72");
//! ```

// ============================================================================
// PRECISION CONSTANTS
// ============================================================================

/// Tags that require 9 decimal places of precision.
///
/// These tags store computed APEX values or ratios where high precision
/// is meaningful for accuracy in exposure calculations or zoom factors.
pub const HIGH_PRECISION_9_TAGS: &[&str] = &[
    "BrightnessValue",
    "CompressedBitsPerPixel",
    "DigitalZoomRatio",
];

/// Tags that require exactly 2 decimal places with trailing zeros.
///
/// GPS directional and speed tags are displayed with consistent 2-decimal
/// formatting for readability and consistency with ExifTool output.
pub const GPS_2_DECIMAL_TAGS: &[&str] =
    &["GPSDestBearing", "GPSImgDirection", "GPSSpeed", "GPSTrack"];

/// Tags that may require up to 10 decimal places if precision is needed.
///
/// Resolution values typically display as integers when they are whole numbers,
/// but can have up to 10 decimal places for fractional DPI values.
pub const RESOLUTION_TAGS: &[&str] = &["XResolution", "YResolution"];

// ============================================================================
// MAIN FORMATTING FUNCTION
// ============================================================================

/// Format an EXIF rational value with tag-specific precision to match ExifTool output.
///
/// This function applies different decimal precision rules based on the tag name:
///
/// | Tag Category | Decimal Places | Trailing Zeros | Example |
/// |--------------|----------------|----------------|---------|
/// | BrightnessValue, DigitalZoomRatio, CompressedBitsPerPixel | 9 | Trimmed | "3.617254236" |
/// | GPSDestBearing, GPSImgDirection, GPSSpeed, GPSTrack | 2 | Preserved | "45.50" |
/// | XResolution, YResolution | Up to 10 | Trimmed | "72" or "300.5" |
/// | All other tags | 6 (default) | Trimmed | "3.5" |
///
/// # Arguments
///
/// * `tag_name` - The EXIF tag name. Supports both simple names ("BrightnessValue")
///   and fully-qualified names with group prefix ("EXIF:BrightnessValue").
/// * `value` - The floating-point value to format (typically computed from rational numerator/denominator).
///
/// # Returns
///
/// A string representation of the value with appropriate precision for the tag.
///
/// # Precision Rules
///
/// - **High precision tags**: Format with up to 9 decimal places, trimming trailing zeros
///   except when the result would be an integer (which displays without decimal point).
/// - **GPS tags**: Always format with exactly 2 decimal places, including trailing zeros.
/// - **Resolution tags**: Format with up to 10 decimal places, trimming trailing zeros.
///   Integer values display without decimal point.
/// - **Default**: Format with up to 6 decimal places, trimming trailing zeros.
///
/// # Examples
///
/// ```
/// use oxidex::core::formatters::numeric_precision::format_exif_rational;
///
/// // High precision (9 decimals, trailing zeros trimmed)
/// assert_eq!(format_exif_rational("BrightnessValue", 3.617254236), "3.617254236");
/// assert_eq!(format_exif_rational("DigitalZoomRatio", 1.0), "1");
/// assert_eq!(format_exif_rational("CompressedBitsPerPixel", 2.5), "2.5");
///
/// // GPS tags (exactly 2 decimals, trailing zeros preserved)
/// assert_eq!(format_exif_rational("GPSDestBearing", 123.45), "123.45");
/// assert_eq!(format_exif_rational("GPSImgDirection", 45.0), "45.00");
/// assert_eq!(format_exif_rational("GPSSpeed", 50.5), "50.50");
/// assert_eq!(format_exif_rational("GPSTrack", 180.0), "180.00");
///
/// // Resolution (up to 10 decimals, trailing zeros trimmed)
/// assert_eq!(format_exif_rational("XResolution", 72.0), "72");
/// assert_eq!(format_exif_rational("YResolution", 300.5), "300.5");
///
/// // Default behavior (6 decimals, trailing zeros trimmed)
/// assert_eq!(format_exif_rational("SomeOtherTag", 3.5), "3.5");
///
/// // Works with fully-qualified tag names
/// assert_eq!(format_exif_rational("EXIF:BrightnessValue", 3.617254236), "3.617254236");
/// assert_eq!(format_exif_rational("GPS:GPSImgDirection", 45.0), "45.00");
/// ```
pub fn format_exif_rational(tag_name: &str, value: f64) -> String {
    // Extract the base tag name (after the last colon if present).
    // This handles fully-qualified names like "EXIF:BrightnessValue" or "GPS:GPSSpeed".
    let base_name = tag_name.rsplit(':').next().unwrap_or(tag_name);

    // Apply tag-specific formatting rules based on the base tag name
    if HIGH_PRECISION_9_TAGS.contains(&base_name) {
        format_with_precision(value, 9)
    } else if GPS_2_DECIMAL_TAGS.contains(&base_name) {
        // GPS tags always show exactly 2 decimal places with trailing zeros
        format!("{:.2}", value)
    } else if RESOLUTION_TAGS.contains(&base_name) {
        format_with_precision(value, 10)
    } else {
        // Default: 6 decimal places with trailing zeros trimmed
        format_with_precision(value, 6)
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Format a value with a specified maximum precision, trimming trailing zeros.
///
/// This internal helper formats a floating-point value with up to `precision`
/// decimal places, then removes unnecessary trailing zeros and decimal points
/// to produce clean output.
///
/// # Arguments
///
/// * `value` - The floating-point value to format
/// * `precision` - Maximum number of decimal places to display
///
/// # Returns
///
/// A string representation with trailing zeros trimmed. Integer values
/// are displayed without a decimal point (e.g., "72" not "72.0").
///
/// # Examples
///
/// ```ignore
/// assert_eq!(format_with_precision(3.5, 9), "3.5");
/// assert_eq!(format_with_precision(72.0, 10), "72");
/// assert_eq!(format_with_precision(3.617254236, 9), "3.617254236");
/// ```
fn format_with_precision(value: f64, precision: usize) -> String {
    // Format with the specified precision
    let formatted = format!("{:.prec$}", value, prec = precision);

    // Trim trailing zeros and unnecessary decimal point for cleaner output.
    // This produces "3.5" instead of "3.500000000" and "72" instead of "72.0000000000".
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

/// Check if a tag requires high precision (9 decimal places).
///
/// # Arguments
///
/// * `tag_name` - The tag name, optionally with group prefix
///
/// # Returns
///
/// `true` if the tag is in the high precision category.
///
/// # Examples
///
/// ```
/// use oxidex::core::formatters::numeric_precision::is_high_precision_tag;
///
/// assert!(is_high_precision_tag("BrightnessValue"));
/// assert!(is_high_precision_tag("EXIF:DigitalZoomRatio"));
/// assert!(!is_high_precision_tag("FocalLength"));
/// ```
pub fn is_high_precision_tag(tag_name: &str) -> bool {
    let base_name = tag_name.rsplit(':').next().unwrap_or(tag_name);
    HIGH_PRECISION_9_TAGS.contains(&base_name)
}

/// Check if a tag requires GPS-style formatting (2 decimal places with trailing zeros).
///
/// # Arguments
///
/// * `tag_name` - The tag name, optionally with group prefix
///
/// # Returns
///
/// `true` if the tag is a GPS directional/speed tag.
///
/// # Examples
///
/// ```
/// use oxidex::core::formatters::numeric_precision::is_gps_2_decimal_tag;
///
/// assert!(is_gps_2_decimal_tag("GPSImgDirection"));
/// assert!(is_gps_2_decimal_tag("GPS:GPSSpeed"));
/// assert!(!is_gps_2_decimal_tag("GPSLatitude"));
/// ```
pub fn is_gps_2_decimal_tag(tag_name: &str) -> bool {
    let base_name = tag_name.rsplit(':').next().unwrap_or(tag_name);
    GPS_2_DECIMAL_TAGS.contains(&base_name)
}

/// Check if a tag is a resolution tag (XResolution, YResolution).
///
/// # Arguments
///
/// * `tag_name` - The tag name, optionally with group prefix
///
/// # Returns
///
/// `true` if the tag is a resolution tag.
///
/// # Examples
///
/// ```
/// use oxidex::core::formatters::numeric_precision::is_resolution_tag;
///
/// assert!(is_resolution_tag("XResolution"));
/// assert!(is_resolution_tag("EXIF:YResolution"));
/// assert!(!is_resolution_tag("FocalPlaneXResolution"));
/// ```
pub fn is_resolution_tag(tag_name: &str) -> bool {
    let base_name = tag_name.rsplit(':').next().unwrap_or(tag_name);
    RESOLUTION_TAGS.contains(&base_name)
}

// ============================================================================
// UNIT TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------------
    // High Precision Tags (9 decimal places)
    // ------------------------------------------------------------------------

    #[test]
    fn test_brightness_value_high_precision() {
        // BrightnessValue should show up to 9 decimal places
        assert_eq!(
            format_exif_rational("BrightnessValue", 3.617254236),
            "3.617254236"
        );
        // Trailing zeros should be trimmed
        assert_eq!(format_exif_rational("BrightnessValue", 3.5), "3.5");
        // Integer values should not show decimal point
        assert_eq!(format_exif_rational("BrightnessValue", 4.0), "4");
    }

    #[test]
    fn test_digital_zoom_ratio_high_precision() {
        // DigitalZoomRatio should show up to 9 decimal places
        assert_eq!(
            format_exif_rational("DigitalZoomRatio", 1.234567891),
            "1.234567891"
        );
        // Typical values
        assert_eq!(format_exif_rational("DigitalZoomRatio", 1.0), "1");
        assert_eq!(format_exif_rational("DigitalZoomRatio", 2.5), "2.5");
    }

    #[test]
    fn test_compressed_bits_per_pixel_high_precision() {
        // CompressedBitsPerPixel should show up to 9 decimal places
        assert_eq!(
            format_exif_rational("CompressedBitsPerPixel", 2.123456789),
            "2.123456789"
        );
        assert_eq!(format_exif_rational("CompressedBitsPerPixel", 3.0), "3");
    }

    // ------------------------------------------------------------------------
    // GPS Tags (2 decimal places with trailing zeros)
    // ------------------------------------------------------------------------

    #[test]
    fn test_gps_dest_bearing_2_decimals() {
        // GPSDestBearing should always show exactly 2 decimal places
        assert_eq!(format_exif_rational("GPSDestBearing", 123.45), "123.45");
        // Trailing zeros should be preserved
        assert_eq!(format_exif_rational("GPSDestBearing", 90.0), "90.00");
        assert_eq!(format_exif_rational("GPSDestBearing", 45.5), "45.50");
    }

    #[test]
    fn test_gps_img_direction_2_decimals() {
        // GPSImgDirection should always show exactly 2 decimal places
        assert_eq!(format_exif_rational("GPSImgDirection", 180.0), "180.00");
        assert_eq!(format_exif_rational("GPSImgDirection", 270.75), "270.75");
    }

    #[test]
    fn test_gps_speed_2_decimals() {
        // GPSSpeed should always show exactly 2 decimal places
        assert_eq!(format_exif_rational("GPSSpeed", 50.0), "50.00");
        assert_eq!(format_exif_rational("GPSSpeed", 65.5), "65.50");
        assert_eq!(format_exif_rational("GPSSpeed", 100.25), "100.25");
    }

    #[test]
    fn test_gps_track_2_decimals() {
        // GPSTrack should always show exactly 2 decimal places
        assert_eq!(format_exif_rational("GPSTrack", 0.0), "0.00");
        assert_eq!(format_exif_rational("GPSTrack", 359.99), "359.99");
    }

    // ------------------------------------------------------------------------
    // Resolution Tags (up to 10 decimal places)
    // ------------------------------------------------------------------------

    #[test]
    fn test_x_resolution_formatting() {
        // Integer resolution values should not show decimal point
        assert_eq!(format_exif_rational("XResolution", 72.0), "72");
        assert_eq!(format_exif_rational("XResolution", 300.0), "300");
        // Fractional values should show necessary precision
        assert_eq!(format_exif_rational("XResolution", 300.5), "300.5");
        // High precision if needed (up to 10 decimals)
        assert_eq!(
            format_exif_rational("XResolution", 72.1234567891),
            "72.1234567891"
        );
    }

    #[test]
    fn test_y_resolution_formatting() {
        // Same rules as XResolution
        assert_eq!(format_exif_rational("YResolution", 72.0), "72");
        assert_eq!(format_exif_rational("YResolution", 300.25), "300.25");
    }

    // ------------------------------------------------------------------------
    // Default Precision (6 decimal places)
    // ------------------------------------------------------------------------

    #[test]
    fn test_default_precision_for_unknown_tags() {
        // Unknown tags should use default 6 decimal places with trailing zeros trimmed
        assert_eq!(format_exif_rational("SomeUnknownTag", 3.5), "3.5");
        assert_eq!(format_exif_rational("AnotherTag", 3.123456), "3.123456");
        // Integer values should not show decimal point
        assert_eq!(format_exif_rational("AnyTag", 42.0), "42");
    }

    #[test]
    fn test_default_precision_truncates_beyond_6_decimals() {
        // Default should only show up to 6 decimal places
        // Note: values are rounded, not truncated
        assert_eq!(
            format_exif_rational("UnknownTag", 1.1234567890),
            "1.123457" // Rounded to 6 decimal places
        );
    }

    // ------------------------------------------------------------------------
    // Fully-Qualified Tag Names (with group prefix)
    // ------------------------------------------------------------------------

    #[test]
    fn test_fully_qualified_tag_names() {
        // Should extract base name and apply correct formatting
        assert_eq!(
            format_exif_rational("EXIF:BrightnessValue", 3.617254236),
            "3.617254236"
        );
        assert_eq!(format_exif_rational("GPS:GPSImgDirection", 45.0), "45.00");
        assert_eq!(format_exif_rational("EXIF:XResolution", 72.0), "72");
        // Multiple colons (edge case)
        assert_eq!(
            format_exif_rational("Group:SubGroup:BrightnessValue", 5.0),
            "5"
        );
    }

    // ------------------------------------------------------------------------
    // Edge Cases
    // ------------------------------------------------------------------------

    #[test]
    fn test_zero_values() {
        // Zero should format correctly for each tag type
        assert_eq!(format_exif_rational("BrightnessValue", 0.0), "0");
        assert_eq!(format_exif_rational("GPSSpeed", 0.0), "0.00");
        assert_eq!(format_exif_rational("XResolution", 0.0), "0");
        assert_eq!(format_exif_rational("UnknownTag", 0.0), "0");
    }

    #[test]
    fn test_negative_values() {
        // BrightnessValue can be negative
        assert_eq!(format_exif_rational("BrightnessValue", -2.5), "-2.5");
        assert_eq!(
            format_exif_rational("BrightnessValue", -3.617254236),
            "-3.617254236"
        );
    }

    #[test]
    fn test_very_small_values() {
        // Very small values should preserve precision where possible
        assert_eq!(
            format_exif_rational("BrightnessValue", 0.000000001),
            "0.000000001"
        );
        assert_eq!(format_exif_rational("GPSSpeed", 0.01), "0.01");
    }

    #[test]
    fn test_very_large_values() {
        // Large values should format correctly
        assert_eq!(
            format_exif_rational("BrightnessValue", 999999.123456789),
            "999999.123456789"
        );
        assert_eq!(format_exif_rational("GPSTrack", 999.99), "999.99");
    }

    // ------------------------------------------------------------------------
    // Helper Function Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_high_precision_tag() {
        assert!(is_high_precision_tag("BrightnessValue"));
        assert!(is_high_precision_tag("DigitalZoomRatio"));
        assert!(is_high_precision_tag("CompressedBitsPerPixel"));
        assert!(is_high_precision_tag("EXIF:BrightnessValue"));
        assert!(!is_high_precision_tag("FocalLength"));
        assert!(!is_high_precision_tag("GPSSpeed"));
    }

    #[test]
    fn test_is_gps_2_decimal_tag() {
        assert!(is_gps_2_decimal_tag("GPSDestBearing"));
        assert!(is_gps_2_decimal_tag("GPSImgDirection"));
        assert!(is_gps_2_decimal_tag("GPSSpeed"));
        assert!(is_gps_2_decimal_tag("GPSTrack"));
        assert!(is_gps_2_decimal_tag("GPS:GPSSpeed"));
        assert!(!is_gps_2_decimal_tag("GPSLatitude"));
        assert!(!is_gps_2_decimal_tag("BrightnessValue"));
    }

    #[test]
    fn test_is_resolution_tag() {
        assert!(is_resolution_tag("XResolution"));
        assert!(is_resolution_tag("YResolution"));
        assert!(is_resolution_tag("EXIF:XResolution"));
        assert!(!is_resolution_tag("FocalPlaneXResolution"));
        assert!(!is_resolution_tag("GPSSpeed"));
    }

    // ------------------------------------------------------------------------
    // Tag List Verification
    // ------------------------------------------------------------------------

    #[test]
    fn test_high_precision_tags_list() {
        assert_eq!(HIGH_PRECISION_9_TAGS.len(), 3);
        assert!(HIGH_PRECISION_9_TAGS.contains(&"BrightnessValue"));
        assert!(HIGH_PRECISION_9_TAGS.contains(&"CompressedBitsPerPixel"));
        assert!(HIGH_PRECISION_9_TAGS.contains(&"DigitalZoomRatio"));
    }

    #[test]
    fn test_gps_2_decimal_tags_list() {
        assert_eq!(GPS_2_DECIMAL_TAGS.len(), 4);
        assert!(GPS_2_DECIMAL_TAGS.contains(&"GPSDestBearing"));
        assert!(GPS_2_DECIMAL_TAGS.contains(&"GPSImgDirection"));
        assert!(GPS_2_DECIMAL_TAGS.contains(&"GPSSpeed"));
        assert!(GPS_2_DECIMAL_TAGS.contains(&"GPSTrack"));
    }

    #[test]
    fn test_resolution_tags_list() {
        assert_eq!(RESOLUTION_TAGS.len(), 2);
        assert!(RESOLUTION_TAGS.contains(&"XResolution"));
        assert!(RESOLUTION_TAGS.contains(&"YResolution"));
    }

    // ------------------------------------------------------------------------
    // Internal Helper Function Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_format_with_precision() {
        // Verify trailing zero trimming behavior
        assert_eq!(format_with_precision(3.5, 9), "3.5");
        assert_eq!(format_with_precision(72.0, 10), "72");
        assert_eq!(format_with_precision(3.617254236, 9), "3.617254236");
        assert_eq!(format_with_precision(1.0, 6), "1");
        assert_eq!(format_with_precision(0.0, 6), "0");
        assert_eq!(format_with_precision(-1.5, 6), "-1.5");
    }
}
