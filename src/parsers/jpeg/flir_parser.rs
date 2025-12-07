//! FLIR thermal imaging APP1 parser
//!
//! FLIR cameras embed thermal data in APP1 segments with "FLIR\x00" identifier.
//! This parser extracts camera model and thermal parameters from FLIR segments.

use crate::core::{MetadataMap, TagValue};
use crate::io::EndianReader;

/// Parse FLIR APP1 segment
///
/// FLIR segments contain thermal imaging metadata including camera model,
/// atmospheric temperature, emissivity, and other thermal parameters.
///
/// # Arguments
///
/// * `data` - Raw APP1 segment data (should start with "FLIR\x00")
/// * `metadata` - Metadata map to populate with extracted values
///
/// # Returns
///
/// Returns `Ok(())` if parsing succeeded, or an error message
pub fn parse_flir_segment(data: &[u8], metadata: &mut MetadataMap) -> Result<(), String> {
    // FLIR segments start with "FLIR\x00"
    if data.len() < 8 || &data[0..5] != b"FLIR\x00" {
        return Err("Not a FLIR segment".to_string());
    }

    // FLIR uses a proprietary FFF (FLIR File Format) structure
    // The format is complex, but we can extract some basic info

    // Skip the FLIR identifier and version info (first 8 bytes)
    let reader = EndianReader::little_endian(&data[8..]);

    // Try to extract camera model from segment
    // In FLIR FFF, the camera model is often at a fixed offset
    if data.len() >= 32 {
        // Attempt to read camera model string from offset 16-32
        if let Ok(model) = std::str::from_utf8(&data[16..32]) {
            let model = model.trim_end_matches('\x00').trim();
            if !model.is_empty() {
                metadata.insert(
                    "FLIR:CameraModel".to_string(),
                    TagValue::String(model.to_string()),
                );
            }
        }
    }

    // Parse FLIR record index if available
    // The FLIR FFF format contains multiple records with thermal data
    if data.len() > 64 {
        // Look for FFF record headers
        // Each record starts with a type identifier
        if let Some(record_type) = reader.u16_at(0) {
            if record_type > 0 {
                metadata.insert(
                    "FLIR:RecordType".to_string(),
                    TagValue::Integer(record_type as i64),
                );
            }
        }
    }

    // Note: Full FLIR FFF parsing would require:
    // - Parsing the record index table
    // - Extracting AtmosphericTemperature
    // - Extracting Emissivity
    // - Extracting ObjectDistance
    // - Extracting RelativeHumidity
    // - Extracting RawThermalImage dimensions
    // This is left as TODO for future enhancement

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flir_identification() {
        let mut data = Vec::new();
        data.extend_from_slice(b"FLIR\x00");
        data.extend_from_slice(&[0x01, 0x02, 0x03]);

        let mut metadata = MetadataMap::new();
        // Should not error on valid FLIR prefix
        let result = parse_flir_segment(&data, &mut metadata);
        assert!(result.is_ok());
    }

    #[test]
    fn test_flir_with_camera_model() {
        let mut data = Vec::new();
        data.extend_from_slice(b"FLIR\x00");
        data.extend_from_slice(&[0x00; 11]); // Padding to offset 16
        data.extend_from_slice(b"FLIR E60\x00\x00\x00\x00\x00\x00\x00\x00"); // Camera model at offset 16

        let mut metadata = MetadataMap::new();
        let result = parse_flir_segment(&data, &mut metadata);

        assert!(result.is_ok());
        assert_eq!(
            metadata.get_string("FLIR:CameraModel").as_deref(),
            Some("FLIR E60")
        );
    }

    #[test]
    fn test_non_flir_rejected() {
        let data = b"EXIF\x00\x00";
        let mut metadata = MetadataMap::new();
        let result = parse_flir_segment(data, &mut metadata);
        assert!(result.is_err());
    }

    #[test]
    fn test_flir_too_short() {
        let data = b"FLIR";
        let mut metadata = MetadataMap::new();
        let result = parse_flir_segment(data, &mut metadata);
        assert!(result.is_err());
    }
}
