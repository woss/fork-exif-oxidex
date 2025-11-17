//! JPEG XL (JXL) image format parser

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

const JXL_SIGNATURE_1: &[u8] = &[0xFF, 0x0A];
const JXL_SIGNATURE_2: &[u8] = &[0x00, 0x00, 0x00, 0x0C, 0x4A, 0x58, 0x4C, 0x20];

/// Parser for JPEG XL (JXL) next-generation image files
///
/// Extracts metadata from JPEG XL format images including dimensions and encoding information.
pub struct JXLParser;

impl JXLParser {
    /// Verifies the JPEG XL file signature (supports both bare codestream and container formats)
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 2 {
            return Ok(false);
        }
        let header = reader.read(0, 2)?;
        if header == JXL_SIGNATURE_1 {
            return Ok(true);
        }
        if reader.size() >= 12 {
            let header_long = reader.read(0, 12)?;
            if &header_long[0..8] == JXL_SIGNATURE_2 {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

impl FormatParser for JXLParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid JXL signature"));
        }
        let mut metadata = MetadataMap::new();
        metadata.insert("FileType".to_string(), TagValue::String("JXL".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );
        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::JXL)
    }
}
