//! STL (Stereolithography) 3D model parser

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// Parser for STL (Stereolithography) 3D model files
///
/// Extracts metadata from STL files used in 3D printing and CAD applications.
pub struct STLParser;

impl STLParser {
    /// Verifies the STL file signature (supports both ASCII and binary formats)
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 6 {
            return Ok(false);
        }
        let header = reader.read(0, 6)?;
        // ASCII STL starts with "solid " or binary STL (any 80-byte header)
        Ok(&header[0..5] == b"solid" || reader.size() >= 84)
    }

    /// Detects whether the STL file is in ASCII or binary format
    pub fn detect_format(reader: &dyn FileReader) -> Result<&'static str> {
        if reader.size() < 6 {
            return Ok("Unknown");
        }
        let header = reader.read(0, 6)?;
        if &header[0..5] == b"solid" {
            Ok("ASCII")
        } else {
            Ok("Binary")
        }
    }
}

impl FormatParser for STLParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid STL signature"));
        }
        let mut metadata = MetadataMap::new();
        metadata.insert("FileType".to_string(), TagValue::String("STL".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );

        let format = Self::detect_format(reader)?;
        metadata.insert(
            "STLFormat".to_string(),
            TagValue::String(format.to_string()),
        );

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::STL)
    }
}
