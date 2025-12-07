//! JPEG APP segment parsers (APP2, APP12, APP14, COM, DQT, SOF)
//!
//! This module provides parsers for various JPEG application-specific segments:
//! - APP2: ICC Profile
//! - APP12: Picture Info (Ducky)
//! - APP14: Adobe segment
//! - COM: JPEG Comment
//! - DQT: Quantization tables (for quality estimation)
//! - SOF: Start of Frame (component information)

use crate::core::{MetadataMap, TagValue};
use crate::io::EndianReader;

/// Parse ICC Profile (APP2) segment
///
/// ICC Profile segments start with "ICC_PROFILE\0"
pub fn parse_icc_profile_segment(data: &[u8], metadata: &mut MetadataMap) -> Result<(), String> {
    if data.len() < 14 {
        return Err("ICC Profile segment too short".to_string());
    }

    // Check for ICC_PROFILE identifier
    if &data[0..12] != b"ICC_PROFILE\0" {
        return Err("Invalid ICC Profile identifier".to_string());
    }

    // Sequence number (1-based)
    let sequence = data[12];
    // Total number of APP2 segments
    let total = data[13];

    metadata.insert(
        "ICC_Profile:ProfileSequence".to_string(),
        TagValue::String(format!("{} of {}", sequence, total)),
    );

    // If this is the first segment, extract profile header info
    if sequence == 1 && data.len() >= 128 + 14 {
        let profile_data = &data[14..];

        // Profile size (bytes 0-3)
        if profile_data.len() >= 4 {
            let reader = EndianReader::big_endian(profile_data);
            let size = reader.u32_at(0).unwrap_or(0);
            metadata.insert(
                "ICC_Profile:ProfileSize".to_string(),
                TagValue::Integer(size as i64),
            );
        }

        // Profile version (bytes 8-11)
        if profile_data.len() >= 11 {
            let version_major = profile_data[8];
            let version_minor = (profile_data[9] >> 4) & 0x0F;
            metadata.insert(
                "ICC_Profile:ProfileVersion".to_string(),
                TagValue::String(format!("{}.{}", version_major, version_minor)),
            );
        }

        // Profile class (bytes 12-15)
        if profile_data.len() >= 16 {
            if let Ok(class) = std::str::from_utf8(&profile_data[12..16]) {
                let class_desc = match class {
                    "scnr" => "Input Device Profile",
                    "mntr" => "Display Device Profile",
                    "prtr" => "Output Device Profile",
                    "link" => "DeviceLink Profile",
                    "spac" => "ColorSpace Conversion Profile",
                    "abst" => "Abstract Profile",
                    "nmcl" => "Named Color Profile",
                    _ => class,
                };
                metadata.insert(
                    "ICC_Profile:ProfileClass".to_string(),
                    TagValue::String(class_desc.to_string()),
                );
            }
        }

        // Color space (bytes 16-19)
        if profile_data.len() >= 20 {
            if let Ok(color_space) = std::str::from_utf8(&profile_data[16..20]) {
                metadata.insert(
                    "ICC_Profile:ColorSpace".to_string(),
                    TagValue::String(color_space.trim().to_string()),
                );
            }
        }
    }

    Ok(())
}

/// Parse Picture Info (Ducky) segment (APP12)
///
/// Ducky segments start with "Ducky"
pub fn parse_ducky_segment(data: &[u8], metadata: &mut MetadataMap) -> Result<(), String> {
    if data.len() < 5 {
        return Err("Ducky segment too short".to_string());
    }

    if &data[0..5] != b"Ducky" {
        return Err("Invalid Ducky identifier".to_string());
    }

    // Ducky format: sequence of tag-length-value triplets
    let mut offset = 5;
    while offset + 4 <= data.len() {
        let header_reader = EndianReader::big_endian(&data[offset..]);
        let tag_id = header_reader.u16_at(0).unwrap_or(0);
        let length = header_reader.u16_at(2).unwrap_or(0) as usize;
        offset += 4;

        if offset + length > data.len() {
            break;
        }

        let value_data = &data[offset..offset + length];

        // Parse common Ducky tags
        match tag_id {
            0x0001 => {
                // Quality
                if length >= 4 {
                    let value_reader = EndianReader::big_endian(value_data);
                    let quality = value_reader.i32_at(0).unwrap_or(0);
                    metadata.insert(
                        "Ducky:Quality".to_string(),
                        TagValue::Integer(quality as i64),
                    );
                }
            }
            0x0002 => {
                // Comment
                if let Ok(comment) = std::str::from_utf8(value_data) {
                    metadata.insert(
                        "Ducky:Comment".to_string(),
                        TagValue::String(comment.to_string()),
                    );
                }
            }
            0x0003 => {
                // Copyright
                if let Ok(copyright) = std::str::from_utf8(value_data) {
                    metadata.insert(
                        "Ducky:Copyright".to_string(),
                        TagValue::String(copyright.to_string()),
                    );
                }
            }
            _ => {
                // Unknown tag
                metadata.insert(
                    format!("Ducky:Tag_{:04X}", tag_id),
                    TagValue::Binary(value_data.to_vec()),
                );
            }
        }

        offset += length;
    }

    Ok(())
}

/// Parse Adobe segment (APP14)
///
/// Adobe segments start with "Adobe"
pub fn parse_adobe_segment(data: &[u8], metadata: &mut MetadataMap) -> Result<(), String> {
    if data.len() < 12 {
        return Err("Adobe segment too short".to_string());
    }

    if &data[0..5] != b"Adobe" {
        return Err("Invalid Adobe identifier".to_string());
    }

    let reader = EndianReader::big_endian(data);

    // DCT Encode Version (2 bytes at offset 5)
    let dct_encode_version = reader.u16_at(5).unwrap_or(0);
    metadata.insert(
        "Adobe:DCTEncodeVersion".to_string(),
        TagValue::Integer(dct_encode_version as i64),
    );

    // APP14 Flags0 (2 bytes at offset 7)
    let flags0 = reader.u16_at(7).unwrap_or(0);
    metadata.insert(
        "Adobe:APP14Flags0".to_string(),
        TagValue::Integer(flags0 as i64),
    );

    // APP14 Flags1 (2 bytes at offset 9)
    let flags1 = reader.u16_at(9).unwrap_or(0);
    metadata.insert(
        "Adobe:APP14Flags1".to_string(),
        TagValue::Integer(flags1 as i64),
    );

    // Color Transform (1 byte at offset 11)
    let color_transform = data[11];
    let transform_desc = match color_transform {
        0 => "Unknown (RGB or CMYK)",
        1 => "YCbCr",
        2 => "YCCK",
        _ => "Unknown",
    };
    metadata.insert(
        "Adobe:ColorTransform".to_string(),
        TagValue::String(transform_desc.to_string()),
    );

    Ok(())
}

/// Parse JPEG Comment segment (COM)
pub fn parse_comment_segment(data: &[u8], metadata: &mut MetadataMap) -> Result<(), String> {
    // Try to parse as UTF-8 text
    match std::str::from_utf8(data) {
        Ok(comment) => {
            metadata.insert(
                "JPEG:Comment".to_string(),
                TagValue::String(comment.to_string()),
            );
            Ok(())
        }
        Err(_) => {
            // If not valid UTF-8, store as binary
            metadata.insert("JPEG:Comment".to_string(), TagValue::Binary(data.to_vec()));
            Ok(())
        }
    }
}

/// Estimate JPEG quality from DQT (Define Quantization Table) segment
///
/// This uses a heuristic based on the quantization table values
pub fn estimate_quality_from_dqt(data: &[u8], metadata: &mut MetadataMap) -> Result<(), String> {
    if data.is_empty() {
        return Err("DQT segment is empty".to_string());
    }

    // Parse DQT header
    let precision_and_id = data[0];
    let _precision = (precision_and_id >> 4) & 0x0F; // 0 = 8-bit, 1 = 16-bit
    let _table_id = precision_and_id & 0x0F;

    // For 8-bit precision, we have 64 quantization values
    if data.len() < 65 {
        return Err("DQT segment too short".to_string());
    }

    // Calculate average quantization value (excluding first byte)
    let qvals = &data[1..65];
    let sum: u32 = qvals.iter().map(|&v| v as u32).sum();
    let avg = sum / 64;

    // Estimate quality using a simple heuristic
    // Lower quantization values = higher quality
    let quality = if avg <= 10 {
        95 + (10 - avg) as i64
    } else if avg <= 50 {
        85 - ((avg - 10) / 4) as i64
    } else {
        50 - ((avg - 50) / 2) as i64
    };

    let quality = quality.clamp(1, 100);

    metadata.insert(
        "JPEG:EstimatedQuality".to_string(),
        TagValue::Integer(quality),
    );

    Ok(())
}

/// Parse JFIF APP0 segment with extended fields
///
/// This parser handles both JFIF and JFXX (JFIF extension) segments.
pub fn parse_app0_extended(data: &[u8], metadata: &mut MetadataMap) -> Result<(), String> {
    if data.len() < 5 {
        return Err("APP0 segment too short".to_string());
    }

    // Check JFIF identifier
    if &data[0..5] == b"JFIF\x00" {
        // Already parsed by jfif_parser, but we can add JFXX extension support
        // Check if there's more data after standard JFIF header (14 bytes)
        if data.len() >= 14 {
            let thumbnail_width = data[12];
            let thumbnail_height = data[13];

            // If there's thumbnail data, note it
            if thumbnail_width > 0 && thumbnail_height > 0 {
                metadata.insert(
                    "JFIF:HasThumbnail".to_string(),
                    TagValue::String("Yes".to_string()),
                );
            }
        }
        return Ok(());
    }

    // Check for JFXX extension
    if &data[0..5] == b"JFXX\x00" {
        return parse_jfxx_segment(&data[5..], metadata);
    }

    Err("Not a JFIF/JFXX segment".to_string())
}

/// Parse JFXX (JFIF extension) segment
fn parse_jfxx_segment(data: &[u8], metadata: &mut MetadataMap) -> Result<(), String> {
    if data.is_empty() {
        return Err("JFXX segment empty".to_string());
    }

    let extension_code = data[0];
    let ext_type = match extension_code {
        0x10 => "Thumbnail JPEG",
        0x11 => "Thumbnail 1 byte/pixel",
        0x13 => "Thumbnail 3 bytes/pixel",
        _ => "Unknown",
    };

    metadata.insert(
        "JFIF:ThumbnailType".to_string(),
        TagValue::String(ext_type.to_string()),
    );

    Ok(())
}

/// Parse SOF (Start of Frame) segment for component information
pub fn parse_sof_segment(
    marker: u16,
    data: &[u8],
    metadata: &mut MetadataMap,
) -> Result<(), String> {
    if data.len() < 6 {
        return Err("SOF segment too short".to_string());
    }

    let reader = EndianReader::big_endian(data);

    // Sample precision (1 byte)
    let precision = data[0];
    metadata.insert(
        "JPEG:BitsPerSample".to_string(),
        TagValue::Integer(precision as i64),
    );

    // Image height (2 bytes)
    let height = reader.u16_at(1).unwrap_or(0);
    metadata.insert(
        "JPEG:ImageHeight".to_string(),
        TagValue::Integer(height as i64),
    );

    // Image width (2 bytes)
    let width = reader.u16_at(3).unwrap_or(0);
    metadata.insert(
        "JPEG:ImageWidth".to_string(),
        TagValue::Integer(width as i64),
    );

    // Number of components (1 byte)
    let num_components = data[5];
    metadata.insert(
        "JPEG:ColorComponents".to_string(),
        TagValue::Integer(num_components as i64),
    );

    // Encoding process
    let encoding = match marker {
        0xFFC0 => "Baseline DCT",
        0xFFC1 => "Extended Sequential DCT",
        0xFFC2 => "Progressive DCT",
        0xFFC3 => "Lossless",
        0xFFC5 => "Differential Sequential DCT",
        0xFFC6 => "Differential Progressive DCT",
        0xFFC7 => "Differential Lossless",
        0xFFC9 => "Extended Sequential DCT (Arithmetic)",
        0xFFCA => "Progressive DCT (Arithmetic)",
        0xFFCB => "Lossless (Arithmetic)",
        0xFFCD => "Differential Sequential DCT (Arithmetic)",
        0xFFCE => "Differential Progressive DCT (Arithmetic)",
        0xFFCF => "Differential Lossless (Arithmetic)",
        _ => "Unknown",
    };
    metadata.insert(
        "JPEG:EncodingProcess".to_string(),
        TagValue::String(encoding.to_string()),
    );

    // Parse component details
    let mut offset = 6;
    for i in 0..num_components {
        if offset + 3 > data.len() {
            break;
        }

        let component_id = data[offset];
        let sampling_factors = data[offset + 1];
        let h_sampling = (sampling_factors >> 4) & 0x0F;
        let v_sampling = sampling_factors & 0x0F;
        let _quant_table = data[offset + 2];

        // Component name
        let component_name = match component_id {
            1 => "Y",
            2 => "Cb",
            3 => "Cr",
            4 => "I",
            5 => "Q",
            _ => "Unknown",
        };

        metadata.insert(
            format!("JPEG:ComponentID_{}", i + 1),
            TagValue::String(component_name.to_string()),
        );

        metadata.insert(
            format!("JPEG:YCbCrSubSampling_{}", i + 1),
            TagValue::String(format!("{}x{}", h_sampling, v_sampling)),
        );

        offset += 3;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_icc_profile() {
        let mut data = Vec::new();
        data.extend_from_slice(b"ICC_PROFILE\0");
        data.push(1); // Sequence 1
        data.push(1); // Total 1

        // Minimal ICC profile header
        let mut profile_header = vec![0u8; 128];
        // Profile size
        profile_header[0..4].copy_from_slice(&[0x00, 0x00, 0x02, 0x00]);
        // Version 4.0
        profile_header[8] = 0x04;
        profile_header[9] = 0x00;
        // Profile class "mntr" (display)
        profile_header[12..16].copy_from_slice(b"mntr");
        // Color space "RGB "
        profile_header[16..20].copy_from_slice(b"RGB ");

        data.extend_from_slice(&profile_header);

        let mut metadata = MetadataMap::new();
        let result = parse_icc_profile_segment(&data, &mut metadata);

        assert!(result.is_ok());
        assert_eq!(
            metadata.get_string("ICC_Profile:ProfileClass").as_deref(),
            Some("Display Device Profile")
        );
        assert_eq!(
            metadata.get_string("ICC_Profile:ColorSpace").as_deref(),
            Some("RGB")
        );
    }

    #[test]
    fn test_parse_adobe_segment() {
        let data = [
            b'A', b'd', b'o', b'b', b'e', // Identifier
            0x00, 0x64, // DCT Encode Version: 100
            0x00, 0x00, // Flags0
            0x00, 0x00, // Flags1
            0x01, // Color Transform: YCbCr
        ];

        let mut metadata = MetadataMap::new();
        let result = parse_adobe_segment(&data, &mut metadata);

        assert!(result.is_ok());
        assert_eq!(metadata.get_integer("Adobe:DCTEncodeVersion"), Some(100));
        assert_eq!(
            metadata.get_string("Adobe:ColorTransform").as_deref(),
            Some("YCbCr")
        );
    }

    #[test]
    fn test_parse_comment_segment() {
        let data = b"This is a JPEG comment";

        let mut metadata = MetadataMap::new();
        let result = parse_comment_segment(data, &mut metadata);

        assert!(result.is_ok());
        assert_eq!(
            metadata.get_string("JPEG:Comment").as_deref(),
            Some("This is a JPEG comment")
        );
    }

    #[test]
    fn test_estimate_quality_high() {
        // Create a DQT with low values (high quality)
        let mut data = vec![0x00]; // Precision 0, table 0
        data.extend(vec![5u8; 64]); // Low quantization values

        let mut metadata = MetadataMap::new();
        let result = estimate_quality_from_dqt(&data, &mut metadata);

        assert!(result.is_ok());
        let quality = metadata.get_integer("JPEG:EstimatedQuality").unwrap();
        assert!(quality > 90);
    }

    #[test]
    fn test_parse_sof_baseline() {
        let data = [
            0x08, // Precision: 8 bits
            0x01, 0xE0, // Height: 480
            0x02, 0x80, // Width: 640
            0x03, // Components: 3 (YCbCr)
            // Component 1: Y
            0x01, 0x22, 0x00, // ID=1, sampling=2x2, quant=0
            // Component 2: Cb
            0x02, 0x11, 0x01, // ID=2, sampling=1x1, quant=1
            // Component 3: Cr
            0x03, 0x11, 0x01, // ID=3, sampling=1x1, quant=1
        ];

        let mut metadata = MetadataMap::new();
        let result = parse_sof_segment(0xFFC0, &data, &mut metadata);

        assert!(result.is_ok());
        assert_eq!(metadata.get_integer("JPEG:ImageHeight"), Some(480));
        assert_eq!(metadata.get_integer("JPEG:ImageWidth"), Some(640));
        assert_eq!(metadata.get_integer("JPEG:ColorComponents"), Some(3));
        assert_eq!(
            metadata.get_string("JPEG:EncodingProcess"),
            Some("Baseline DCT")
        );
        assert_eq!(metadata.get_string("JPEG:ComponentID_1"), Some("Y"));
    }
}
