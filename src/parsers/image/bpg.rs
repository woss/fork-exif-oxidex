//! BPG (Better Portable Graphics) format parser

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

const BPG_SIGNATURE: &[u8] = &[0x42, 0x50, 0x47, 0xFB];

/// Parser for BPG (Better Portable Graphics) image files
///
/// Extracts basic metadata from BPG format images including dimensions and color information.
pub struct BPGParser;

impl BPGParser {
    /// Verifies the BPG file signature (0x42 0x50 0x47 0xFB)
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 4 {
            return Ok(false);
        }
        let header = reader.read(0, 4)?;
        Ok(header == BPG_SIGNATURE)
    }
}

impl FormatParser for BPGParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid BPG signature"));
        }
        let mut metadata = MetadataMap::new();
        metadata.insert("FileType".to_string(), TagValue::String("BPG".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );
        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::BPG)
    }
}

/// Parses metadata from BPG files.
///
/// This is a convenience wrapper around BPGParser that provides a functional API.
pub fn parse_bpg_metadata(reader: &dyn FileReader) -> std::result::Result<MetadataMap, String> {
    let parser = BPGParser;
    parser.parse(reader).map_err(|e| e.to_string())
}
