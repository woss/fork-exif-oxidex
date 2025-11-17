//! GIF image format parser
//!
//! Implements basic metadata extraction from GIF (Graphics Interchange Format) files.

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// GIF signature: "GIF87a" or "GIF89a"
const GIF87A_SIGNATURE: &[u8] = b"GIF87a";
const GIF89A_SIGNATURE: &[u8] = b"GIF89a";

/// GIF parser for extracting metadata from GIF images
pub struct GIFParser;

impl GIFParser {
    /// Verifies GIF signature
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 6 {
            return Ok(false);
        }
        let header = reader.read(0, 6)?;
        Ok(header == GIF87A_SIGNATURE || header == GIF89A_SIGNATURE)
    }

    /// Reads GIF version (87a or 89a)
    pub fn read_version(reader: &dyn FileReader) -> Result<&'static str> {
        if reader.size() < 6 {
            return Ok("Unknown");
        }
        let header = reader.read(0, 6)?;
        if header == GIF87A_SIGNATURE {
            Ok("87a")
        } else if header == GIF89A_SIGNATURE {
            Ok("89a")
        } else {
            Ok("Unknown")
        }
    }

    /// Reads image dimensions from GIF header (offset 6, 4 bytes: width, height in little-endian)
    pub fn read_dimensions(reader: &dyn FileReader) -> Result<(u16, u16)> {
        if reader.size() < 10 {
            return Ok((0, 0));
        }
        let dims = reader.read(6, 4)?;
        let width = u16::from_le_bytes([dims[0], dims[1]]);
        let height = u16::from_le_bytes([dims[2], dims[3]]);
        Ok((width, height))
    }
}

impl FormatParser for GIFParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid GIF signature"));
        }

        let mut metadata = MetadataMap::new();

        metadata.insert("FileType".to_string(), TagValue::String("GIF".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );

        let version = Self::read_version(reader)?;
        metadata.insert(
            "GIFVersion".to_string(),
            TagValue::String(version.to_string()),
        );

        let (width, height) = Self::read_dimensions(reader)?;
        metadata.insert(
            "ImageWidth".to_string(),
            TagValue::String(width.to_string()),
        );
        metadata.insert(
            "ImageHeight".to_string(),
            TagValue::String(height.to_string()),
        );

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::GIF)
    }
}

/// Parses metadata from GIF files.
pub fn parse_gif_metadata(reader: &dyn FileReader) -> std::result::Result<MetadataMap, String> {
    let parser = GIFParser;
    parser.parse(reader).map_err(|e| e.to_string())
}
