//! AutoCAD DXF (Drawing Exchange Format) parser

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

pub struct DXFParser;

impl DXFParser {
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 20 {
            return Ok(false);
        }
        let header = reader.read(0, 20)?;
        let text = std::str::from_utf8(header).unwrap_or("");
        Ok(text.starts_with("0\n") && text.contains("SECTION"))
    }
}

impl FormatParser for DXFParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid DXF signature"));
        }
        let mut metadata = MetadataMap::new();
        metadata.insert("FileType".to_string(), TagValue::String("DXF".to_string()));
        metadata.insert("FileSize".to_string(), TagValue::String(reader.size().to_string()));
        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::DXF)
    }
}
