//! WebP image format parser

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// WebP signature: "RIFF" + size + "WEBP"
const RIFF_SIGNATURE: &[u8] = b"RIFF";
const WEBP_SIGNATURE: &[u8] = b"WEBP";

/// WebP parser
pub struct WebPParser;

impl WebPParser {
    /// Verifies WebP signature
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 12 {
            return Ok(false);
        }
        let header = reader.read(0, 12)?;
        Ok(&header[0..4] == RIFF_SIGNATURE && &header[8..12] == WEBP_SIGNATURE)
    }
}

impl FormatParser for WebPParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid WebP signature"));
        }

        let mut metadata = MetadataMap::new();

        metadata.insert("FileType".to_string(), TagValue::String("WebP".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::WebP)
    }
}

/// Parses metadata from WebP files.
///
/// This is a convenience wrapper around WebPParser that provides a functional API.
pub fn parse_webp_metadata(reader: &dyn FileReader) -> std::result::Result<MetadataMap, String> {
    let parser = WebPParser;
    parser.parse(reader).map_err(|e| e.to_string())
}
