//! AVIF image format parser
//!
//! Implements basic metadata extraction from AVIF (AV1 Image File Format) files.

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// AVIF uses ISO BMFF with "ftyp" at offset 4 and "avif" brand
const FTYP_SIGNATURE: &[u8] = b"ftyp";

/// AVIF parser (ISO BMFF-based format)
pub struct AVIFParser;

impl AVIFParser {
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 12 {
            return Ok(false);
        }
        let header = reader.read(4, 8)?;
        Ok(&header[0..4] == FTYP_SIGNATURE && &header[4..8] == b"avif")
    }
}

impl FormatParser for AVIFParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid AVIF signature"));
        }
        let mut metadata = MetadataMap::new();
        metadata.insert("FileType".to_string(), TagValue::String("AVIF".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );
        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::AVIF)
    }
}
