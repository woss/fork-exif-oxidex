//! OpenEXR image format parser

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

const EXR_SIGNATURE: &[u8] = &[0x76, 0x2F, 0x31, 0x01];

/// Parser for OpenEXR high dynamic range image files
///
/// Extracts metadata from OpenEXR format images including dimensions, compression,
/// channels, and various header attributes.
pub struct EXRParser;

impl EXRParser {
    /// Verifies the OpenEXR file signature (0x76 0x2F 0x31 0x01)
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 4 {
            return Ok(false);
        }
        let header = reader.read(0, 4)?;
        Ok(header == EXR_SIGNATURE)
    }

    /// Reads and parses the version field (4 bytes at offset 4)
    fn read_version_flags(reader: &dyn FileReader) -> Result<(u8, bool, bool, bool, bool)> {
        if reader.size() < 8 {
            return Err(ExifToolError::parse_error("File too small for version field"));
        }
        let version_bytes = reader.read(4, 4)?;
        let flags = u32::from_le_bytes([version_bytes[0], version_bytes[1], version_bytes[2], version_bytes[3]]);
        Ok((version_bytes[0], (flags & 0x200) != 0, (flags & 0x400) != 0,
            (flags & 0x800) != 0, (flags & 0x1000) != 0))
    }

    /// Reads a null-terminated string from the given offset
    fn read_null_terminated_string(reader: &dyn FileReader, offset: u64) -> Result<(String, u64)> {
        let mut bytes = Vec::new();
        let mut pos = offset;
        let chunk_size: usize = 64;

        loop {
            if pos >= reader.size() {
                return Err(ExifToolError::parse_error("Unexpected EOF reading string"));
            }
            let remaining = reader.size() - pos;
            let to_read = chunk_size.min(remaining as usize);
            let data = reader.read(pos, to_read)?;

            for &byte in data.iter() {
                if byte == 0 {
                    let string = String::from_utf8_lossy(&bytes).to_string();
                    return Ok((string, pos - offset + 1));
                }
                bytes.push(byte);
                pos += 1;
            }
        }
    }

    /// Parses header attributes starting at offset 8
    fn parse_attributes(reader: &dyn FileReader, metadata: &mut MetadataMap) -> Result<()> {
        let mut offset: u64 = 8;

        loop {
            if offset >= reader.size() {
                break;
            }

            // Check for null byte terminator
            let byte = reader.read(offset, 1)?;
            if byte[0] == 0 {
                break;
            }

            // Read attribute name
            let (name, name_len) = Self::read_null_terminated_string(reader, offset)?;
            offset += name_len;

            // Read attribute type
            let (attr_type, type_len) = Self::read_null_terminated_string(reader, offset)?;
            offset += type_len;

            // Read attribute size
            if offset + 4 > reader.size() {
                break;
            }
            let size_bytes = reader.read(offset, 4)?;
            let size = u32::from_le_bytes([size_bytes[0], size_bytes[1], size_bytes[2], size_bytes[3]]);
            offset += 4;

            // Read attribute value
            if offset + size as u64 > reader.size() {
                break;
            }
            let value_bytes = reader.read(offset, size as usize)?;
            offset += size as u64;

            // Parse specific attributes
            Self::parse_attribute(metadata, &name, &attr_type, &value_bytes)?;
        }

        Ok(())
    }

    /// Parses a specific attribute based on its type
    fn parse_attribute(metadata: &mut MetadataMap, name: &str, attr_type: &str, value: &[u8]) -> Result<()> {
        match (name, attr_type) {
            ("dataWindow" | "displayWindow", "box2i") if value.len() >= 16 => {
                let x_min = i32::from_le_bytes([value[0], value[1], value[2], value[3]]);
                let y_min = i32::from_le_bytes([value[4], value[5], value[6], value[7]]);
                let x_max = i32::from_le_bytes([value[8], value[9], value[10], value[11]]);
                let y_max = i32::from_le_bytes([value[12], value[13], value[14], value[15]]);

                if name == "dataWindow" {
                    let width = (x_max - x_min + 1) as u32;
                    let height = (y_max - y_min + 1) as u32;
                    metadata.insert("ImageWidth".to_string(), TagValue::Integer(width as i64));
                    metadata.insert("ImageHeight".to_string(), TagValue::Integer(height as i64));
                    metadata.insert("DataWindow".to_string(),
                        TagValue::String(format!("{},{} {},{}", x_min, y_min, x_max, y_max)));
                } else {
                    metadata.insert("DisplayWindow".to_string(),
                        TagValue::String(format!("{},{} {},{}", x_min, y_min, x_max, y_max)));
                }
            }
            ("compression", "compression") if !value.is_empty() => {
                let comp_name = match value[0] {
                    0 => "None", 1 => "RLE", 2 => "ZIPS", 3 => "ZIP", 4 => "PIZ",
                    5 => "PXR24", 6 => "B44", 7 => "B44A", 8 => "DWAA", 9 => "DWAB",
                    _ => "Unknown",
                };
                metadata.insert("Compression".to_string(), TagValue::String(comp_name.to_string()));
            }
            ("lineOrder", "lineOrder") if !value.is_empty() => {
                let order_name = match value[0] {
                    0 => "Increasing Y", 1 => "Decreasing Y", 2 => "Random Y", _ => "Unknown",
                };
                metadata.insert("LineOrder".to_string(), TagValue::String(order_name.to_string()));
            }
            ("pixelAspectRatio" | "screenWindowWidth", "float") if value.len() >= 4 => {
                let bits = u32::from_le_bytes([value[0], value[1], value[2], value[3]]);
                let float_val = f32::from_bits(bits) as f64;
                let key = if name == "pixelAspectRatio" { "PixelAspectRatio" } else { "ScreenWindowWidth" };
                metadata.insert(key.to_string(), TagValue::Float(float_val));
            }
            ("screenWindowCenter", "v2f") if value.len() >= 8 => {
                let x = f32::from_bits(u32::from_le_bytes([value[0], value[1], value[2], value[3]]));
                let y = f32::from_bits(u32::from_le_bytes([value[4], value[5], value[6], value[7]]));
                metadata.insert("ScreenWindowCenter".to_string(), TagValue::String(format!("{},{}", x, y)));
            }
            (name @ ("owner" | "comments" | "capDate" | "utcOffset"), "string") => {
                if let Ok(string_val) = std::str::from_utf8(value) {
                    let key = match name {
                        "owner" => "Owner",
                        "comments" => "Comments",
                        "capDate" => "CaptureDate",
                        "utcOffset" => "UTCOffset",
                        _ => name,
                    };
                    metadata.insert(key.to_string(), TagValue::String(string_val.trim_end_matches('\0').to_string()));
                }
            }
            ("channels", "chlist") => {
                // Parse channel list - format: name\0 pixel_type(4) pLinear(1) reserved(3) xSampling(4) ySampling(4)
                let channels = Self::parse_channel_list(value);
                if !channels.is_empty() {
                    metadata.insert("Channels".to_string(), TagValue::String(channels.join(", ")));
                }
            }
            _ => {} // Ignore other attributes for now
        }
        Ok(())
    }

    /// Parses channel list from chlist attribute
    fn parse_channel_list(data: &[u8]) -> Vec<String> {
        let mut channels = Vec::new();
        let mut offset = 0;

        while offset < data.len() && data[offset] != 0 {
            let mut name_bytes = Vec::new();
            while offset < data.len() && data[offset] != 0 {
                name_bytes.push(data[offset]);
                offset += 1;
            }
            if offset >= data.len() { break; }
            offset += 17; // Skip null terminator + 16 bytes (pixel_type, pLinear, reserved, xSampling, ySampling)

            if let Ok(name) = String::from_utf8(name_bytes) {
                channels.push(name);
            }
        }
        channels
    }
}

impl FormatParser for EXRParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid EXR signature"));
        }

        let mut metadata = MetadataMap::new();

        // Basic file info
        metadata.insert("FileType".to_string(), TagValue::String("EXR".to_string()));
        metadata.insert("FileSize".to_string(), TagValue::String(reader.size().to_string()));

        // Parse version and flags
        let (version, tiled, long_names, deep_data, multipart) = Self::read_version_flags(reader)?;
        metadata.insert("EXRVersion".to_string(), TagValue::Integer(version as i64));

        for (flag, name) in [(tiled, "Tiled"), (long_names, "LongNames"),
                             (deep_data, "DeepData"), (multipart, "Multipart")] {
            if flag {
                metadata.insert(name.to_string(), TagValue::String("Yes".to_string()));
            }
        }

        // Parse header attributes
        Self::parse_attributes(reader, &mut metadata)?;

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::EXR)
    }
}

/// Parses metadata from OpenEXR files.
///
/// This is a convenience wrapper around EXRParser that provides a functional API.
pub fn parse_exr_metadata(reader: &dyn FileReader) -> std::result::Result<MetadataMap, String> {
    let parser = EXRParser;
    parser.parse(reader).map_err(|e| e.to_string())
}
