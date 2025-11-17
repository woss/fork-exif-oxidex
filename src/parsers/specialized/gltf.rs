//! glTF (GL Transmission Format) 3D model parser

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// Parser for glTF (GL Transmission Format) 3D model files
///
/// Extracts metadata from glTF JSON-based 3D scene description files.
pub struct GLTFParser;

impl GLTFParser {
    /// Verifies the glTF file by checking for JSON structure with "asset" field
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 20 {
            return Ok(false);
        }
        let header = reader.read(0, 100.min(reader.size() as usize))?;
        let text = std::str::from_utf8(header).unwrap_or("");
        Ok(text.contains("\"asset\"") && text.contains("{"))
    }
}

impl FormatParser for GLTFParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid GLTF signature"));
        }
        let mut metadata = MetadataMap::new();
        metadata.insert("FileType".to_string(), TagValue::String("GLTF".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );
        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::GLTF)
    }
}

/// Parses metadata from glTF files.
///
/// This is a convenience wrapper around GLTFParser that provides a functional API.
pub fn parse_gltf_metadata(reader: &dyn FileReader) -> std::result::Result<MetadataMap, String> {
    let parser = GLTFParser;
    parser.parse(reader).map_err(|e| e.to_string())
}
