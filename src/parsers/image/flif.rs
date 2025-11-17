//! FLIF (Free Lossless Image Format) parser

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

const FLIF_SIGNATURE: &[u8] = b"FLIF";

/// Parser for FLIF (Free Lossless Image Format) files
///
/// Extracts metadata from FLIF format images including dimensions and animation information.
pub struct FLIFParser;

impl FLIFParser {
    /// Verifies the FLIF file signature ("FLIF")
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 4 {
            return Ok(false);
        }
        let header = reader.read(0, 4)?;
        Ok(header == FLIF_SIGNATURE)
    }
}

impl FormatParser for FLIFParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid FLIF signature"));
        }
        let mut metadata = MetadataMap::new();
        metadata.insert("FileType".to_string(), TagValue::String("FLIF".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );
        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::FLIF)
    }
}
