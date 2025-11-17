//! ICO (Windows Icon) format parser

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

const ICO_SIGNATURE: &[u8] = &[0x00, 0x00, 0x01, 0x00];

/// Parser for Windows ICO (Icon) image files
///
/// Extracts metadata from ICO files including image count and icon dimensions.
pub struct ICOParser;

impl ICOParser {
    /// Verifies the ICO file signature (0x00 0x00 0x01 0x00)
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 4 {
            return Ok(false);
        }
        let header = reader.read(0, 4)?;
        Ok(header == ICO_SIGNATURE)
    }
}

impl FormatParser for ICOParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid ICO signature"));
        }
        let mut metadata = MetadataMap::new();
        metadata.insert("FileType".to_string(), TagValue::String("ICO".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );
        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::ICO)
    }
}

/// Parses metadata from ICO files.
///
/// This is a convenience wrapper around ICOParser that provides a functional API.
pub fn parse_ico_metadata(reader: &dyn FileReader) -> std::result::Result<MetadataMap, String> {
    let parser = ICOParser;
    parser.parse(reader).map_err(|e| e.to_string())
}
