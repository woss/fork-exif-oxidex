//! JPEG-HDR APP11 segment parser
//!
//! This module provides comprehensive parsing for JPEG-HDR metadata stored in APP11 segments.
//! JPEG-HDR is a backward-compatible extension to JPEG for storing High Dynamic Range (HDR)
//! images. The format stores a tone-mapped base image in standard JPEG format along with
//! metadata in APP11 segments that enable HDR reconstruction.
//!
//! # Format Overview
//!
//! JPEG-HDR segments contain:
//! - Version information (JPEG-HDRVersion)
//! - Tone mapping coefficients (Alpha, Beta)
//! - Correction method indicator
//! - Luminance range parameters (Ln0, Ln1)
//! - Signal-to-noise information (S2n)
//! - Optional ratio image data for HDR reconstruction
//!
//! # Segment Structure
//!
//! JPEG-HDR segments typically begin with one of these identifiers:
//! - `HDR_RI\0` (HDR Ratio Image) - Contains ratio image data
//! - `JPEG-HDR` - Contains HDR parameters and metadata
//!
//! # References
//!
//! - Ward, G. & Simmons, M. (2004). "JPEG-HDR: A Backwards-Compatible, High Dynamic Range
//!   Extension to JPEG"

use crate::core::{MetadataMap, TagValue};
use crate::error::Result;
use crate::io::EndianReader;

/// Minimum segment size required for parsing (identifier + basic header)
const MIN_SEGMENT_SIZE: usize = 6;

/// HDR Ratio Image identifier ("HDR_RI\0")
const HDR_RI_IDENTIFIER: &[u8] = b"HDR_RI";

/// Generic JPEG-HDR identifier
const JPEG_HDR_IDENTIFIER: &[u8] = b"JPEG-HDR";

/// Correction method identifiers used in JPEG-HDR tone mapping
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrectionMethod {
    /// No correction applied
    None,
    /// Multiplicative correction
    Multiplicative,
    /// Additive correction
    Additive,
    /// Logarithmic correction (most common for HDR)
    Logarithmic,
    /// Gamma correction
    Gamma,
    /// Unknown or proprietary method
    Unknown(u8),
}

impl CorrectionMethod {
    /// Converts a byte value to a CorrectionMethod enum variant
    ///
    /// # Arguments
    ///
    /// * `value` - The raw byte value from the segment
    ///
    /// # Returns
    ///
    /// The corresponding CorrectionMethod variant
    fn from_byte(value: u8) -> Self {
        match value {
            0 => CorrectionMethod::None,
            1 => CorrectionMethod::Multiplicative,
            2 => CorrectionMethod::Additive,
            3 => CorrectionMethod::Logarithmic,
            4 => CorrectionMethod::Gamma,
            other => CorrectionMethod::Unknown(other),
        }
    }

    /// Returns a human-readable description of the correction method
    fn description(&self) -> String {
        match self {
            CorrectionMethod::None => "None".to_string(),
            CorrectionMethod::Multiplicative => "Multiplicative".to_string(),
            CorrectionMethod::Additive => "Additive".to_string(),
            CorrectionMethod::Logarithmic => "Logarithmic".to_string(),
            CorrectionMethod::Gamma => "Gamma".to_string(),
            CorrectionMethod::Unknown(v) => format!("Unknown ({})", v),
        }
    }
}

/// Parsed JPEG-HDR parameters
///
/// This structure holds all the HDR-related parameters extracted from an APP11 segment.
/// These parameters are used to reconstruct the full HDR image from the tone-mapped base.
#[derive(Debug, Clone)]
pub struct JpegHdrParameters {
    /// JPEG-HDR format version (major.minor)
    pub version: Option<(u8, u8)>,
    /// Alpha coefficient for tone mapping (exposure adjustment)
    pub alpha: Option<f32>,
    /// Beta coefficient for tone mapping (contrast adjustment)
    pub beta: Option<f32>,
    /// Method used for HDR correction/reconstruction
    pub correction_method: Option<CorrectionMethod>,
    /// Lower luminance bound (Ln0) in log space
    pub ln0: Option<f32>,
    /// Upper luminance bound (Ln1) in log space
    pub ln1: Option<f32>,
    /// Signal-to-noise ratio estimate
    pub s2n: Option<f32>,
    /// Size of ratio image data in bytes (if present)
    pub ratio_image_size: Option<usize>,
    /// Indicates if this segment contains ratio image data
    pub has_ratio_image: bool,
}

impl Default for JpegHdrParameters {
    fn default() -> Self {
        Self {
            version: None,
            alpha: None,
            beta: None,
            correction_method: None,
            ln0: None,
            ln1: None,
            s2n: None,
            ratio_image_size: None,
            has_ratio_image: false,
        }
    }
}

/// Parses a JPEG-HDR APP11 segment and returns extracted metadata.
///
/// This function analyzes APP11 segment data to extract JPEG-HDR metadata including
/// version information, tone mapping parameters, and ratio image indicators.
///
/// # Arguments
///
/// * `data` - Raw APP11 segment data (excluding the APP11 marker and length bytes)
///
/// # Returns
///
/// * `Ok(MetadataMap)` - Successfully parsed metadata with JPEG-HDR tags
/// * `Err` - If the segment format is not recognized or parsing fails
///
/// # Supported Identifiers
///
/// - `HDR_RI` - HDR Ratio Image segment containing reconstruction data
/// - `JPEG-HDR` - Generic JPEG-HDR parameter segment
///
/// # Example
///
/// ```ignore
/// use oxidex::parsers::jpeg::app_segments::app11_jpeg_hdr::parse_app11_jpeg_hdr;
///
/// let segment_data = &[/* APP11 segment bytes */];
/// match parse_app11_jpeg_hdr(segment_data) {
///     Ok(metadata) => {
///         if let Some(version) = metadata.get_string("JPEG-HDR:Version") {
///             println!("JPEG-HDR Version: {}", version);
///         }
///     }
///     Err(e) => eprintln!("Failed to parse JPEG-HDR segment: {}", e),
/// }
/// ```
pub fn parse_app11_jpeg_hdr(data: &[u8]) -> Result<MetadataMap> {
    let mut metadata = MetadataMap::new();

    // Validate minimum segment size
    if data.len() < MIN_SEGMENT_SIZE {
        return Err(crate::error::ExifToolError::parse_error(
            "APP11 segment too short for JPEG-HDR parsing",
        ));
    }

    // Detect segment type and dispatch to appropriate parser
    if data.len() >= HDR_RI_IDENTIFIER.len()
        && &data[..HDR_RI_IDENTIFIER.len()] == HDR_RI_IDENTIFIER
    {
        // HDR Ratio Image segment
        parse_hdr_ratio_image_segment(&data[HDR_RI_IDENTIFIER.len()..], &mut metadata)?;
    } else if data.len() >= JPEG_HDR_IDENTIFIER.len()
        && &data[..JPEG_HDR_IDENTIFIER.len()] == JPEG_HDR_IDENTIFIER
    {
        // Generic JPEG-HDR parameter segment
        parse_jpeg_hdr_parameter_segment(&data[JPEG_HDR_IDENTIFIER.len()..], &mut metadata)?;
    } else {
        return Err(crate::error::ExifToolError::parse_error(
            "Not a recognized JPEG-HDR segment (expected HDR_RI or JPEG-HDR identifier)",
        ));
    }

    Ok(metadata)
}

/// Parses an HDR Ratio Image (HDR_RI) segment.
///
/// HDR_RI segments contain the ratio image data used to reconstruct the full HDR
/// image from the tone-mapped base JPEG. The format is:
///
/// ```text
/// Offset  Size  Description
/// 0       2     Version (major.minor as bytes)
/// 2       1     Correction method
/// 3       4     Ln0 (float32, luminance lower bound)
/// 7       4     Ln1 (float32, luminance upper bound)
/// 11      4     Alpha (float32, exposure coefficient)
/// 15      4     Beta (float32, contrast coefficient)
/// 19      4     S2n (float32, signal-to-noise)
/// 23      N     Ratio image data
/// ```
///
/// # Arguments
///
/// * `data` - Segment data after the "HDR_RI" identifier
/// * `metadata` - MetadataMap to populate with extracted values
fn parse_hdr_ratio_image_segment(data: &[u8], metadata: &mut MetadataMap) -> Result<()> {
    // Mark the segment format type
    metadata.insert(
        "JPEG-HDR:Format".to_string(),
        TagValue::String("Ratio Image".to_string()),
    );

    // We need at least enough data for the header (version + correction + parameters)
    // Minimum header size: 2 (version) + 1 (correction) + 4*5 (five float32 params) = 23 bytes
    const HEADER_SIZE: usize = 23;

    if data.is_empty() {
        // Empty segment after identifier - still valid but no parameters
        return Ok(());
    }

    let reader = EndianReader::big_endian(data);

    // Parse version (2 bytes: major, minor)
    if data.len() >= 2 {
        let version_major = reader.u8_at(0).unwrap_or(0);
        let version_minor = reader.u8_at(1).unwrap_or(0);

        metadata.insert(
            "JPEG-HDR:Version".to_string(),
            TagValue::String(format!("{}.{}", version_major, version_minor)),
        );
    }

    // Parse correction method (1 byte at offset 2)
    if data.len() >= 3 {
        let correction_byte = reader.u8_at(2).unwrap_or(0);
        let correction_method = CorrectionMethod::from_byte(correction_byte);

        metadata.insert(
            "JPEG-HDR:CorrectionMethod".to_string(),
            TagValue::String(correction_method.description()),
        );
    }

    // Parse floating-point parameters (all big-endian float32)
    // These represent the tone mapping curve parameters

    // Ln0 - Lower luminance bound (offset 3)
    if data.len() >= 7 {
        if let Some(ln0) = reader.f32_at(3) {
            // Validate the value is reasonable (should be in log space, typically negative)
            if ln0.is_finite() {
                metadata.insert("JPEG-HDR:Ln0".to_string(), TagValue::Float(ln0 as f64));
            }
        }
    }

    // Ln1 - Upper luminance bound (offset 7)
    if data.len() >= 11 {
        if let Some(ln1) = reader.f32_at(7) {
            if ln1.is_finite() {
                metadata.insert("JPEG-HDR:Ln1".to_string(), TagValue::Float(ln1 as f64));
            }
        }
    }

    // Alpha - Exposure coefficient (offset 11)
    if data.len() >= 15 {
        if let Some(alpha) = reader.f32_at(11) {
            if alpha.is_finite() {
                metadata.insert("JPEG-HDR:Alpha".to_string(), TagValue::Float(alpha as f64));
            }
        }
    }

    // Beta - Contrast coefficient (offset 15)
    if data.len() >= 19 {
        if let Some(beta) = reader.f32_at(15) {
            if beta.is_finite() {
                metadata.insert("JPEG-HDR:Beta".to_string(), TagValue::Float(beta as f64));
            }
        }
    }

    // S2n - Signal-to-noise estimate (offset 19)
    if data.len() >= 23 {
        if let Some(s2n) = reader.f32_at(19) {
            if s2n.is_finite() && s2n >= 0.0 {
                metadata.insert("JPEG-HDR:S2n".to_string(), TagValue::Float(s2n as f64));
            }
        }
    }

    // Check for ratio image data
    if data.len() > HEADER_SIZE {
        let ratio_image_size = data.len() - HEADER_SIZE;
        metadata.insert(
            "JPEG-HDR:RatioImageSize".to_string(),
            TagValue::Integer(ratio_image_size as i64),
        );
        metadata.insert(
            "JPEG-HDR:HasRatioImage".to_string(),
            TagValue::String("Yes".to_string()),
        );
    } else {
        metadata.insert(
            "JPEG-HDR:HasRatioImage".to_string(),
            TagValue::String("No".to_string()),
        );
    }

    Ok(())
}

/// Parses a generic JPEG-HDR parameter segment.
///
/// This handles segments that begin with the "JPEG-HDR" identifier and contain
/// HDR parameters in a more flexible format. The structure may vary by implementation.
///
/// # Format Variants
///
/// The JPEG-HDR specification allows for some flexibility in how parameters are encoded.
/// This parser handles the most common format:
///
/// ```text
/// Offset  Size  Description
/// 0       1     Version byte
/// 1       1     Sub-version or flags
/// 2       1     Correction method
/// 3       4     Alpha (float32)
/// 7       4     Beta (float32)
/// 11      4     Ln0 (float32, optional)
/// 15      4     Ln1 (float32, optional)
/// ```
///
/// # Arguments
///
/// * `data` - Segment data after the "JPEG-HDR" identifier
/// * `metadata` - MetadataMap to populate with extracted values
fn parse_jpeg_hdr_parameter_segment(data: &[u8], metadata: &mut MetadataMap) -> Result<()> {
    // Mark the segment format type
    metadata.insert(
        "JPEG-HDR:Format".to_string(),
        TagValue::String("HDR Parameters".to_string()),
    );

    if data.is_empty() {
        return Ok(());
    }

    let reader = EndianReader::big_endian(data);

    // Parse version information
    // First byte is typically major version, second byte is minor/flags
    if !data.is_empty() {
        let version_byte = reader.u8_at(0).unwrap_or(0);

        if version_byte > 0 {
            // Check for sub-version byte
            let sub_version = if data.len() > 1 {
                reader.u8_at(1).unwrap_or(0)
            } else {
                0
            };

            metadata.insert(
                "JPEG-HDR:Version".to_string(),
                TagValue::String(format!("{}.{}", version_byte, sub_version)),
            );

            // Also store as integer for compatibility with existing parser
            metadata.insert(
                "JPEG-HDR:HDRVersion".to_string(),
                TagValue::Integer(version_byte as i64),
            );
        }
    }

    // Parse correction method if present (offset 2)
    if data.len() >= 3 {
        let correction_byte = reader.u8_at(2).unwrap_or(0);
        let correction_method = CorrectionMethod::from_byte(correction_byte);

        metadata.insert(
            "JPEG-HDR:CorrectionMethod".to_string(),
            TagValue::String(correction_method.description()),
        );
    }

    // Parse Alpha coefficient (offset 3)
    if data.len() >= 7 {
        if let Some(alpha) = reader.f32_at(3) {
            if alpha.is_finite() {
                metadata.insert("JPEG-HDR:Alpha".to_string(), TagValue::Float(alpha as f64));
            }
        }
    }

    // Parse Beta coefficient (offset 7)
    if data.len() >= 11 {
        if let Some(beta) = reader.f32_at(7) {
            if beta.is_finite() {
                metadata.insert("JPEG-HDR:Beta".to_string(), TagValue::Float(beta as f64));
            }
        }
    }

    // Parse optional Ln0 (offset 11)
    if data.len() >= 15 {
        if let Some(ln0) = reader.f32_at(11) {
            if ln0.is_finite() {
                metadata.insert("JPEG-HDR:Ln0".to_string(), TagValue::Float(ln0 as f64));
            }
        }
    }

    // Parse optional Ln1 (offset 15)
    if data.len() >= 19 {
        if let Some(ln1) = reader.f32_at(15) {
            if ln1.is_finite() {
                metadata.insert("JPEG-HDR:Ln1".to_string(), TagValue::Float(ln1 as f64));
            }
        }
    }

    // Parse optional S2n (offset 19)
    if data.len() >= 23 {
        if let Some(s2n) = reader.f32_at(19) {
            if s2n.is_finite() && s2n >= 0.0 {
                metadata.insert("JPEG-HDR:S2n".to_string(), TagValue::Float(s2n as f64));
            }
        }
    }

    // Store exposure compensation for backward compatibility
    // This is derived from the Alpha value when available
    if let Some(alpha_val) = metadata.get_float("JPEG-HDR:Alpha") {
        // Convert alpha to exposure stops (EV) for user-friendly display
        // alpha typically represents a multiplier, so log2 gives stops
        if alpha_val > 0.0 {
            let exposure_ev = (alpha_val as f64).log2();
            if exposure_ev.is_finite() {
                metadata.insert(
                    "JPEG-HDR:ExposureCompensation".to_string(),
                    TagValue::Float(exposure_ev),
                );
            }
        }
    }

    Ok(())
}

/// Extracts JPEG-HDR parameters into a structured format.
///
/// This is a convenience function that parses the segment data and returns
/// a structured `JpegHdrParameters` object instead of a MetadataMap.
///
/// # Arguments
///
/// * `data` - Raw APP11 segment data
///
/// # Returns
///
/// * `Ok(JpegHdrParameters)` - Structured HDR parameters
/// * `Err` - If parsing fails
///
/// # Example
///
/// ```ignore
/// use oxidex::parsers::jpeg::app_segments::app11_jpeg_hdr::extract_hdr_parameters;
///
/// let params = extract_hdr_parameters(segment_data)?;
/// if let Some((major, minor)) = params.version {
///     println!("JPEG-HDR version: {}.{}", major, minor);
/// }
/// if params.has_ratio_image {
///     println!("Contains ratio image data");
/// }
/// ```
pub fn extract_hdr_parameters(data: &[u8]) -> Result<JpegHdrParameters> {
    let metadata = parse_app11_jpeg_hdr(data)?;
    let mut params = JpegHdrParameters::default();

    // Extract version
    if let Some(version_str) = metadata.get_string("JPEG-HDR:Version") {
        let parts: Vec<&str> = version_str.split('.').collect();
        if parts.len() >= 2 {
            if let (Ok(major), Ok(minor)) = (parts[0].parse::<u8>(), parts[1].parse::<u8>()) {
                params.version = Some((major, minor));
            }
        }
    }

    // Extract floating-point parameters
    params.alpha = metadata.get_float("JPEG-HDR:Alpha").map(|v| v as f32);
    params.beta = metadata.get_float("JPEG-HDR:Beta").map(|v| v as f32);
    params.ln0 = metadata.get_float("JPEG-HDR:Ln0").map(|v| v as f32);
    params.ln1 = metadata.get_float("JPEG-HDR:Ln1").map(|v| v as f32);
    params.s2n = metadata.get_float("JPEG-HDR:S2n").map(|v| v as f32);

    // Extract correction method
    if let Some(method_str) = metadata.get_string("JPEG-HDR:CorrectionMethod") {
        params.correction_method = Some(match method_str {
            "None" => CorrectionMethod::None,
            "Multiplicative" => CorrectionMethod::Multiplicative,
            "Additive" => CorrectionMethod::Additive,
            "Logarithmic" => CorrectionMethod::Logarithmic,
            "Gamma" => CorrectionMethod::Gamma,
            _ => CorrectionMethod::Unknown(0),
        });
    }

    // Extract ratio image information
    params.ratio_image_size = metadata
        .get_integer("JPEG-HDR:RatioImageSize")
        .map(|v| v as usize);
    params.has_ratio_image = metadata
        .get_string("JPEG-HDR:HasRatioImage")
        .map(|s| s == "Yes")
        .unwrap_or(false);

    Ok(params)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test segment with HDR_RI identifier
    fn create_hdr_ri_segment(version: (u8, u8), correction: u8, params: &[f32]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(HDR_RI_IDENTIFIER);
        data.push(version.0);
        data.push(version.1);
        data.push(correction);

        for param in params {
            data.extend_from_slice(&param.to_be_bytes());
        }

        data
    }

    /// Helper to create a test segment with JPEG-HDR identifier
    fn create_jpeg_hdr_segment(version: (u8, u8), correction: u8, params: &[f32]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(JPEG_HDR_IDENTIFIER);
        data.push(version.0);
        data.push(version.1);
        data.push(correction);

        for param in params {
            data.extend_from_slice(&param.to_be_bytes());
        }

        data
    }

    #[test]
    fn test_parse_hdr_ri_segment_with_full_parameters() {
        // Create a segment with version 1.0, logarithmic correction, and full parameters
        let segment = create_hdr_ri_segment(
            (1, 0),
            3,                              // Logarithmic
            &[-5.0, 10.0, 1.5, 0.8, 100.0], // Ln0, Ln1, Alpha, Beta, S2n
        );

        let result = parse_app11_jpeg_hdr(&segment);
        assert!(result.is_ok(), "Parsing should succeed");

        let metadata = result.unwrap();

        assert_eq!(
            metadata.get_string("JPEG-HDR:Format"),
            Some("Ratio Image"),
            "Format should be Ratio Image"
        );

        assert_eq!(
            metadata.get_string("JPEG-HDR:Version"),
            Some("1.0"),
            "Version should be 1.0"
        );

        assert_eq!(
            metadata.get_string("JPEG-HDR:CorrectionMethod"),
            Some("Logarithmic"),
            "Correction method should be Logarithmic"
        );

        // Check floating-point parameters with tolerance
        let ln0 = metadata.get_float("JPEG-HDR:Ln0").unwrap();
        assert!(
            (ln0 - (-5.0)).abs() < 0.001,
            "Ln0 should be approximately -5.0"
        );

        let ln1 = metadata.get_float("JPEG-HDR:Ln1").unwrap();
        assert!(
            (ln1 - 10.0).abs() < 0.001,
            "Ln1 should be approximately 10.0"
        );

        let alpha = metadata.get_float("JPEG-HDR:Alpha").unwrap();
        assert!(
            (alpha - 1.5).abs() < 0.001,
            "Alpha should be approximately 1.5"
        );

        let beta = metadata.get_float("JPEG-HDR:Beta").unwrap();
        assert!(
            (beta - 0.8).abs() < 0.001,
            "Beta should be approximately 0.8"
        );

        let s2n = metadata.get_float("JPEG-HDR:S2n").unwrap();
        assert!(
            (s2n - 100.0).abs() < 0.001,
            "S2n should be approximately 100.0"
        );

        assert_eq!(
            metadata.get_string("JPEG-HDR:HasRatioImage"),
            Some("No"),
            "Should not have ratio image data"
        );
    }

    #[test]
    fn test_parse_hdr_ri_segment_with_ratio_image() {
        // Create segment with parameters and additional ratio image data
        let mut segment = create_hdr_ri_segment(
            (1, 2),
            1, // Multiplicative
            &[-3.0, 8.0, 2.0, 1.0, 50.0],
        );

        // Add some dummy ratio image data
        segment.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34, 0x56, 0x78]);

        let result = parse_app11_jpeg_hdr(&segment);
        assert!(result.is_ok());

        let metadata = result.unwrap();

        assert_eq!(
            metadata.get_string("JPEG-HDR:HasRatioImage"),
            Some("Yes"),
            "Should have ratio image data"
        );

        assert_eq!(
            metadata.get_integer("JPEG-HDR:RatioImageSize"),
            Some(8),
            "Ratio image should be 8 bytes"
        );
    }

    #[test]
    fn test_parse_jpeg_hdr_parameter_segment() {
        let segment = create_jpeg_hdr_segment(
            (2, 1),
            4,                             // Gamma correction
            &[1.8, 0.9, -2.5, 7.0, 200.0], // Alpha, Beta, Ln0, Ln1, S2n
        );

        let result = parse_app11_jpeg_hdr(&segment);
        assert!(result.is_ok());

        let metadata = result.unwrap();

        assert_eq!(
            metadata.get_string("JPEG-HDR:Format"),
            Some("HDR Parameters"),
            "Format should be HDR Parameters"
        );

        assert_eq!(
            metadata.get_string("JPEG-HDR:Version"),
            Some("2.1"),
            "Version should be 2.1"
        );

        assert_eq!(
            metadata.get_integer("JPEG-HDR:HDRVersion"),
            Some(2),
            "HDRVersion integer should be 2"
        );

        assert_eq!(
            metadata.get_string("JPEG-HDR:CorrectionMethod"),
            Some("Gamma"),
            "Correction method should be Gamma"
        );
    }

    #[test]
    fn test_correction_method_parsing() {
        // Test each correction method
        let test_cases = [
            (0u8, "None"),
            (1u8, "Multiplicative"),
            (2u8, "Additive"),
            (3u8, "Logarithmic"),
            (4u8, "Gamma"),
            (99u8, "Unknown (99)"),
        ];

        for (byte_val, expected_desc) in test_cases {
            let method = CorrectionMethod::from_byte(byte_val);
            assert_eq!(
                method.description(),
                expected_desc,
                "CorrectionMethod {} should have description '{}'",
                byte_val,
                expected_desc
            );
        }
    }

    #[test]
    fn test_segment_too_short() {
        let short_data = &[0x48, 0x44, 0x52]; // "HDR" - too short
        let result = parse_app11_jpeg_hdr(short_data);

        assert!(result.is_err(), "Should fail for segment that is too short");
    }

    #[test]
    fn test_unrecognized_identifier() {
        let unknown_segment = b"UNKNOWN_IDENTIFIER_DATA";
        let result = parse_app11_jpeg_hdr(unknown_segment);

        assert!(
            result.is_err(),
            "Should fail for unrecognized segment identifier"
        );
    }

    #[test]
    fn test_minimal_hdr_ri_segment() {
        // Just the identifier and version, no parameters
        let mut minimal = Vec::new();
        minimal.extend_from_slice(HDR_RI_IDENTIFIER);
        minimal.push(1); // version major
        minimal.push(0); // version minor

        let result = parse_app11_jpeg_hdr(&minimal);
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert_eq!(metadata.get_string("JPEG-HDR:Version"), Some("1.0"));
        assert_eq!(metadata.get_string("JPEG-HDR:Format"), Some("Ratio Image"));
    }

    #[test]
    fn test_extract_hdr_parameters_structured() {
        let segment = create_hdr_ri_segment(
            (1, 5),
            3,                             // Logarithmic
            &[-4.0, 9.0, 1.2, 0.95, 75.0], // Ln0, Ln1, Alpha, Beta, S2n
        );

        let result = extract_hdr_parameters(&segment);
        assert!(result.is_ok());

        let params = result.unwrap();

        assert_eq!(params.version, Some((1, 5)));
        assert_eq!(
            params.correction_method,
            Some(CorrectionMethod::Logarithmic)
        );

        // Check parameters with tolerance
        assert!(params.alpha.is_some());
        assert!((params.alpha.unwrap() - 1.2).abs() < 0.001);

        assert!(params.beta.is_some());
        assert!((params.beta.unwrap() - 0.95).abs() < 0.001);

        assert!(params.ln0.is_some());
        assert!((params.ln0.unwrap() - (-4.0)).abs() < 0.001);

        assert!(params.ln1.is_some());
        assert!((params.ln1.unwrap() - 9.0).abs() < 0.001);

        assert!(params.s2n.is_some());
        assert!((params.s2n.unwrap() - 75.0).abs() < 0.001);

        assert!(!params.has_ratio_image);
    }

    #[test]
    fn test_nan_and_infinity_handling() {
        // Create segment with NaN and infinity values - these should be filtered out
        let mut segment = Vec::new();
        segment.extend_from_slice(HDR_RI_IDENTIFIER);
        segment.push(1);
        segment.push(0);
        segment.push(0); // No correction

        // Add NaN for Ln0
        segment.extend_from_slice(&f32::NAN.to_be_bytes());
        // Add Infinity for Ln1
        segment.extend_from_slice(&f32::INFINITY.to_be_bytes());
        // Add valid Alpha
        segment.extend_from_slice(&1.5f32.to_be_bytes());

        let result = parse_app11_jpeg_hdr(&segment);
        assert!(result.is_ok());

        let metadata = result.unwrap();

        // NaN and Infinity values should NOT be included
        assert!(
            metadata.get_float("JPEG-HDR:Ln0").is_none(),
            "NaN should be filtered out"
        );
        assert!(
            metadata.get_float("JPEG-HDR:Ln1").is_none(),
            "Infinity should be filtered out"
        );

        // Valid value should be included
        assert!(
            metadata.get_float("JPEG-HDR:Alpha").is_some(),
            "Valid float should be included"
        );
    }

    #[test]
    fn test_negative_s2n_handling() {
        // S2n should be non-negative, test that negative values are filtered
        let mut segment = Vec::new();
        segment.extend_from_slice(HDR_RI_IDENTIFIER);
        segment.push(1);
        segment.push(0);
        segment.push(0); // No correction

        // Ln0, Ln1, Alpha, Beta
        segment.extend_from_slice(&0.0f32.to_be_bytes());
        segment.extend_from_slice(&1.0f32.to_be_bytes());
        segment.extend_from_slice(&1.0f32.to_be_bytes());
        segment.extend_from_slice(&1.0f32.to_be_bytes());

        // Negative S2n (invalid)
        segment.extend_from_slice(&(-10.0f32).to_be_bytes());

        let result = parse_app11_jpeg_hdr(&segment);
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert!(
            metadata.get_float("JPEG-HDR:S2n").is_none(),
            "Negative S2n should be filtered out"
        );
    }

    #[test]
    fn test_exposure_compensation_calculation() {
        // Test that exposure compensation is correctly derived from Alpha
        let segment = create_jpeg_hdr_segment(
            (1, 0),
            0,
            &[2.0, 1.0, 0.0, 0.0, 0.0], // Alpha = 2.0 means +1 EV
        );

        let result = parse_app11_jpeg_hdr(&segment);
        assert!(result.is_ok());

        let metadata = result.unwrap();

        if let Some(ev) = metadata.get_float("JPEG-HDR:ExposureCompensation") {
            // Alpha of 2.0 should give log2(2.0) = 1.0 EV
            assert!(
                (ev - 1.0).abs() < 0.001,
                "Exposure compensation should be approximately 1.0 EV"
            );
        }
    }

    #[test]
    fn test_empty_segment_after_identifier() {
        // Just the identifier with no data after
        let segment = HDR_RI_IDENTIFIER;
        let result = parse_app11_jpeg_hdr(segment);
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert_eq!(metadata.get_string("JPEG-HDR:Format"), Some("Ratio Image"));
    }

    #[test]
    fn test_partial_parameters() {
        // Segment with only some parameters present
        let mut segment = Vec::new();
        segment.extend_from_slice(HDR_RI_IDENTIFIER);
        segment.push(1);
        segment.push(0);
        segment.push(3); // Logarithmic

        // Only Ln0
        segment.extend_from_slice(&(-2.5f32).to_be_bytes());

        let result = parse_app11_jpeg_hdr(&segment);
        assert!(result.is_ok());

        let metadata = result.unwrap();

        assert!(
            metadata.get_float("JPEG-HDR:Ln0").is_some(),
            "Ln0 should be present"
        );
        assert!(
            metadata.get_float("JPEG-HDR:Ln1").is_none(),
            "Ln1 should not be present"
        );
        assert!(
            metadata.get_float("JPEG-HDR:Alpha").is_none(),
            "Alpha should not be present"
        );
    }
}
