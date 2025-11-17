//! Wavefront OBJ 3D model parser

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

pub struct OBJParser;

impl OBJParser {
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 10 {
            return Ok(false);
        }
        let header = reader.read(0, 100.min(reader.size() as usize))?;
        let text = std::str::from_utf8(header).unwrap_or("");
        Ok(text.contains("v ") || text.contains("vn ") || text.contains("vt "))
    }
}

impl FormatParser for OBJParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid OBJ signature"));
        }
        let mut metadata = MetadataMap::new();
        metadata.insert("FileType".to_string(), TagValue::String("OBJ".to_string()));
        metadata.insert("FileSize".to_string(), TagValue::String(reader.size().to_string()));
        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::OBJ)
    }
}
