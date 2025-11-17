//! Wavefront OBJ 3D model parser

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// Parser for Wavefront OBJ 3D model files
///
/// Extracts metadata from OBJ text-based 3D geometry description files.
pub struct OBJParser;

impl OBJParser {
    /// Verifies the OBJ file by checking for vertex/normal/texture coordinate definitions
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
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );
        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::OBJ)
    }
}

/// Parses metadata from OBJ files.
///
/// This is a convenience wrapper around OBJParser that provides a functional API.
pub fn parse_obj_metadata(reader: &dyn FileReader) -> std::result::Result<MetadataMap, String> {
    let parser = OBJParser;
    parser.parse(reader).map_err(|e| e.to_string())
}
