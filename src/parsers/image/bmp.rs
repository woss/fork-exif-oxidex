//! BMP (Bitmap) image format parser

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// BMP signature: "BM" (0x42 0x4D)
const BMP_SIGNATURE: &[u8] = b"BM";

/// BMP parser for extracting metadata from Windows bitmap images
pub struct BMPParser;

impl BMPParser {
    /// Verifies BMP signature
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 2 {
            return Ok(false);
        }
        let header = reader.read(0, 2)?;
        Ok(header == BMP_SIGNATURE)
    }

    /// Reads image dimensions from BMP header
    /// Width at offset 18 (4 bytes), Height at offset 22 (4 bytes), both little-endian
    pub fn read_dimensions(reader: &dyn FileReader) -> Result<(i32, i32)> {
        if reader.size() < 26 {
            return Ok((0, 0));
        }
        let width_bytes = reader.read(18, 4)?;
        let height_bytes = reader.read(22, 4)?;
        let width = i32::from_le_bytes([
            width_bytes[0],
            width_bytes[1],
            width_bytes[2],
            width_bytes[3],
        ]);
        let height = i32::from_le_bytes([
            height_bytes[0],
            height_bytes[1],
            height_bytes[2],
            height_bytes[3],
        ]);
        Ok((width, height))
    }

    /// Reads bit depth from BMP header (offset 28, 2 bytes)
    pub fn read_bit_depth(reader: &dyn FileReader) -> Result<u16> {
        if reader.size() < 30 {
            return Ok(0);
        }
        let bits = reader.read(28, 2)?;
        Ok(u16::from_le_bytes([bits[0], bits[1]]))
    }
}

impl FormatParser for BMPParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid BMP signature"));
        }

        let mut metadata = MetadataMap::new();

        metadata.insert("FileType".to_string(), TagValue::String("BMP".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );

        let (width, height) = Self::read_dimensions(reader)?;
        metadata.insert(
            "ImageWidth".to_string(),
            TagValue::String(width.abs().to_string()),
        );
        metadata.insert(
            "ImageHeight".to_string(),
            TagValue::String(height.abs().to_string()),
        );

        let bit_depth = Self::read_bit_depth(reader)?;
        metadata.insert(
            "BitDepth".to_string(),
            TagValue::String(bit_depth.to_string()),
        );

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::BMP)
    }
}

/// Parses metadata from BMP files.
///
/// This is a convenience wrapper around BMPParser that provides a functional API.
pub fn parse_bmp_metadata(reader: &dyn FileReader) -> std::result::Result<MetadataMap, String> {
    let parser = BMPParser;
    parser.parse(reader).map_err(|e| e.to_string())
}
