//! OpenEXR image format parser

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

const EXR_SIGNATURE: &[u8] = &[0x76, 0x2F, 0x31, 0x01];

pub struct EXRParser;

impl EXRParser {
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 4 {
            return Ok(false);
        }
        let header = reader.read(0, 4)?;
        Ok(header == EXR_SIGNATURE)
    }
}

impl FormatParser for EXRParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid EXR signature"));
        }
        let mut metadata = MetadataMap::new();
        metadata.insert("FileType".to_string(), TagValue::String("EXR".to_string()));
        metadata.insert("FileSize".to_string(), TagValue::String(reader.size().to_string()));
        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::EXR)
    }
}
