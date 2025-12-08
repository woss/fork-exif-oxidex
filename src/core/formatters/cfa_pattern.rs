//! CFA (Color Filter Array) Pattern decoder
//!
//! This module provides decoding for CFAPattern EXIF tags (tag ID 0xA302).
//! The CFA pattern describes the arrangement of color filters on image sensors,
//! which is essential for demosaicing raw image data.
//!
//! # Binary Format
//!
//! The CFAPattern data is structured as follows:
//! - Bytes 0-1: Pattern width (horizontal repeat count) as big-endian u16
//! - Bytes 2-3: Pattern height (vertical repeat count) as big-endian u16
//! - Bytes 4+: Color values for each position in row-major order (width * height bytes)
//!
//! # Color Values
//!
//! | Value | Color   |
//! |-------|---------|
//! | 0     | Red     |
//! | 1     | Green   |
//! | 2     | Blue    |
//! | 3     | Cyan    |
//! | 4     | Magenta |
//! | 5     | Yellow  |
//! | 6     | White   |
//!
//! # Common Bayer Patterns
//!
//! Most digital cameras use a 2x2 Bayer pattern with one of these arrangements:
//! - RGGB: `[Red,Green][Green,Blue]` - Most common (Canon, Sony)
//! - BGGR: `[Blue,Green][Green,Red]` - Nikon, Fuji
//! - GRBG: `[Green,Red][Blue,Green]` - Some Kodak sensors
//! - GBRG: `[Green,Blue][Red,Green]` - Less common
//!
//! # ExifTool Compatibility
//!
//! This decoder produces output matching ExifTool's format:
//! - ExifTool outputs: `[Red,Green][Green,Blue]`
//! - OxiDex previously output: `[Binary data]`
//! - This module fixes that discrepancy

/// Color names for CFA pattern values as defined in EXIF specification.
///
/// Index corresponds to the byte value in the CFA pattern data:
/// - 0 = Red
/// - 1 = Green
/// - 2 = Blue
/// - 3 = Cyan
/// - 4 = Magenta
/// - 5 = Yellow
/// - 6 = White
const CFA_COLOR_NAMES: [&str; 7] = ["Red", "Green", "Blue", "Cyan", "Magenta", "Yellow", "White"];

/// Decode CFAPattern binary data to a human-readable string.
///
/// Converts raw CFA pattern bytes into ExifTool-compatible format like
/// `[Red,Green][Green,Blue]` for a standard 2x2 Bayer RGGB pattern.
///
/// # Arguments
///
/// * `data` - Raw binary data containing the CFA pattern. Must be at least 4 bytes
///   (width + height headers) plus width * height bytes for the pattern values.
///
/// # Returns
///
/// A string representation of the CFA pattern in ExifTool format.
/// Returns `"[Invalid CFA data]"` if the data is malformed or too short.
///
/// # Format Details
///
/// - First 4 bytes are width (2 bytes, big-endian) and height (2 bytes, big-endian)
/// - Remaining bytes are color values in row-major order
/// - Output format: `[Color,Color][Color,Color]` for 2x2 patterns
///
/// # Examples
///
/// ```
/// use oxidex::core::formatters::cfa_pattern::decode_cfa_pattern;
///
/// // 2x2 RGGB Bayer pattern (most common)
/// let rggb = [0, 2, 0, 2, 0, 1, 1, 2];  // width=2, height=2, Red,Green,Green,Blue
/// assert_eq!(decode_cfa_pattern(&rggb), "[Red,Green][Green,Blue]");
///
/// // 2x2 BGGR pattern
/// let bggr = [0, 2, 0, 2, 2, 1, 1, 0];  // Blue,Green,Green,Red
/// assert_eq!(decode_cfa_pattern(&bggr), "[Blue,Green][Green,Red]");
///
/// // Invalid data (too short)
/// assert_eq!(decode_cfa_pattern(&[0, 2]), "[Invalid CFA data]");
/// ```
pub fn decode_cfa_pattern(data: &[u8]) -> String {
    // Validate minimum header size: need at least 4 bytes for width and height
    if data.len() < 4 {
        return "[Invalid CFA data]".to_string();
    }

    // Parse width and height from big-endian u16 values
    // These represent the horizontal and vertical repeat counts of the pattern
    let width = u16::from_be_bytes([data[0], data[1]]) as usize;
    let height = u16::from_be_bytes([data[2], data[3]]) as usize;

    // Validate dimensions are reasonable (non-zero and not excessively large)
    // Most CFA patterns are 2x2, but some sensors may use larger patterns
    if width == 0 || height == 0 || width > 16 || height > 16 {
        return "[Invalid CFA data]".to_string();
    }

    // Calculate required data size and validate we have enough bytes
    let pattern_size = width * height;
    let required_length = 4 + pattern_size;
    if data.len() < required_length {
        return "[Invalid CFA data]".to_string();
    }

    // Build the output string row by row
    // Each row is enclosed in brackets with comma-separated color names
    let mut result = String::with_capacity(pattern_size * 8); // Estimate capacity

    for row in 0..height {
        result.push('[');

        for col in 0..width {
            // Calculate byte index in the pattern data (after 4-byte header)
            let byte_index = 4 + row * width + col;
            let color_value = data[byte_index] as usize;

            // Add comma separator between colors in the same row
            if col > 0 {
                result.push(',');
            }

            // Look up color name, defaulting to "Unknown" for undefined values
            let color_name = CFA_COLOR_NAMES.get(color_value).unwrap_or(&"Unknown");
            result.push_str(color_name);
        }

        result.push(']');
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Standard Bayer Pattern Tests ====================
    // These are the four common 2x2 Bayer patterns used by digital cameras

    #[test]
    fn test_rggb_bayer_pattern() {
        // RGGB: Most common pattern (Canon, Sony, most DSLRs)
        // Layout: R G
        //         G B
        let data = [0, 2, 0, 2, 0, 1, 1, 2]; // width=2, height=2, R,G,G,B
        assert_eq!(decode_cfa_pattern(&data), "[Red,Green][Green,Blue]");
    }

    #[test]
    fn test_bggr_bayer_pattern() {
        // BGGR: Used by Nikon, Fuji, and others
        // Layout: B G
        //         G R
        let data = [0, 2, 0, 2, 2, 1, 1, 0]; // width=2, height=2, B,G,G,R
        assert_eq!(decode_cfa_pattern(&data), "[Blue,Green][Green,Red]");
    }

    #[test]
    fn test_grbg_bayer_pattern() {
        // GRBG: Used by some Kodak sensors
        // Layout: G R
        //         B G
        let data = [0, 2, 0, 2, 1, 0, 2, 1]; // width=2, height=2, G,R,B,G
        assert_eq!(decode_cfa_pattern(&data), "[Green,Red][Blue,Green]");
    }

    #[test]
    fn test_gbrg_bayer_pattern() {
        // GBRG: Less common arrangement
        // Layout: G B
        //         R G
        let data = [0, 2, 0, 2, 1, 2, 0, 1]; // width=2, height=2, G,B,R,G
        assert_eq!(decode_cfa_pattern(&data), "[Green,Blue][Red,Green]");
    }

    // ==================== Extended Color Tests ====================

    #[test]
    fn test_pattern_with_cyan_magenta_yellow() {
        // Some sensors use CMY instead of RGB
        let data = [0, 2, 0, 2, 3, 4, 5, 6]; // Cyan,Magenta,Yellow,White
        assert_eq!(decode_cfa_pattern(&data), "[Cyan,Magenta][Yellow,White]");
    }

    #[test]
    fn test_pattern_with_unknown_color() {
        // Color values outside the defined range should show as "Unknown"
        let data = [0, 2, 0, 2, 7, 8, 9, 10]; // All undefined values
        assert_eq!(
            decode_cfa_pattern(&data),
            "[Unknown,Unknown][Unknown,Unknown]"
        );
    }

    #[test]
    fn test_mixed_known_and_unknown_colors() {
        // Mix of valid and invalid color indices
        let data = [0, 2, 0, 2, 0, 255, 1, 128]; // Red, Unknown, Green, Unknown
        assert_eq!(decode_cfa_pattern(&data), "[Red,Unknown][Green,Unknown]");
    }

    // ==================== Non-Standard Pattern Sizes ====================

    #[test]
    fn test_single_row_pattern() {
        // 4x1 pattern (single row)
        let data = [0, 4, 0, 1, 0, 1, 1, 2]; // width=4, height=1, R,G,G,B
        assert_eq!(decode_cfa_pattern(&data), "[Red,Green,Green,Blue]");
    }

    #[test]
    fn test_single_column_pattern() {
        // 1x4 pattern (single column)
        let data = [0, 1, 0, 4, 0, 1, 1, 2]; // width=1, height=4
        assert_eq!(decode_cfa_pattern(&data), "[Red][Green][Green][Blue]");
    }

    #[test]
    fn test_3x3_pattern() {
        // 3x3 pattern (non-standard size)
        let data = [
            0, 3, 0, 3, // width=3, height=3
            0, 1, 2, // Row 1: R, G, B
            1, 2, 0, // Row 2: G, B, R
            2, 0, 1, // Row 3: B, R, G
        ];
        assert_eq!(
            decode_cfa_pattern(&data),
            "[Red,Green,Blue][Green,Blue,Red][Blue,Red,Green]"
        );
    }

    #[test]
    fn test_1x1_pattern() {
        // Minimal valid pattern (single color)
        let data = [0, 1, 0, 1, 0]; // width=1, height=1, Red
        assert_eq!(decode_cfa_pattern(&data), "[Red]");
    }

    // ==================== Error Handling Tests ====================

    #[test]
    fn test_empty_data() {
        assert_eq!(decode_cfa_pattern(&[]), "[Invalid CFA data]");
    }

    #[test]
    fn test_insufficient_header() {
        // Less than 4 bytes for header
        assert_eq!(decode_cfa_pattern(&[0, 2]), "[Invalid CFA data]");
        assert_eq!(decode_cfa_pattern(&[0, 2, 0]), "[Invalid CFA data]");
    }

    #[test]
    fn test_zero_width() {
        // Width of 0 is invalid
        let data = [0, 0, 0, 2, 0, 1]; // width=0, height=2
        assert_eq!(decode_cfa_pattern(&data), "[Invalid CFA data]");
    }

    #[test]
    fn test_zero_height() {
        // Height of 0 is invalid
        let data = [0, 2, 0, 0, 0, 1]; // width=2, height=0
        assert_eq!(decode_cfa_pattern(&data), "[Invalid CFA data]");
    }

    #[test]
    fn test_insufficient_pattern_data() {
        // Header indicates 2x2 pattern but only 3 color bytes provided
        let data = [0, 2, 0, 2, 0, 1, 1]; // Missing one byte
        assert_eq!(decode_cfa_pattern(&data), "[Invalid CFA data]");
    }

    #[test]
    fn test_excessive_dimensions() {
        // Dimensions larger than reasonable limit (16x16)
        let data = [0, 100, 0, 2]; // width=100, height=2
        assert_eq!(decode_cfa_pattern(&data), "[Invalid CFA data]");

        let data2 = [0, 2, 0, 100]; // width=2, height=100
        assert_eq!(decode_cfa_pattern(&data2), "[Invalid CFA data]");
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_extra_trailing_data() {
        // Extra bytes after the pattern should be ignored
        let data = [0, 2, 0, 2, 0, 1, 1, 2, 255, 255, 255];
        assert_eq!(decode_cfa_pattern(&data), "[Red,Green][Green,Blue]");
    }

    #[test]
    fn test_exact_length_data() {
        // Exactly the right number of bytes
        let data = [0, 2, 0, 2, 0, 1, 1, 2];
        assert_eq!(decode_cfa_pattern(&data), "[Red,Green][Green,Blue]");
    }

    #[test]
    fn test_all_same_color() {
        // Pattern with all the same color (unusual but valid)
        let data = [0, 2, 0, 2, 1, 1, 1, 1]; // All green
        assert_eq!(decode_cfa_pattern(&data), "[Green,Green][Green,Green]");
    }
}
