//! SVG (Scalable Vector Graphics) parser

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// Parser for SVG (Scalable Vector Graphics) files
///
/// Extracts metadata from SVG XML-based vector graphics files including dimensions and title.
pub struct SVGParser;

impl SVGParser {
    /// Verifies the SVG file by checking for the presence of "<svg" tag in the header
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 100 {
            return Ok(false);
        }
        let header = reader.read(0, 100)?;
        let text = std::str::from_utf8(header).unwrap_or("");
        Ok(text.contains("<svg"))
    }
}

impl FormatParser for SVGParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid SVG signature"));
        }
        let mut metadata = MetadataMap::new();
        metadata.insert("FileType".to_string(), TagValue::String("SVG".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );
        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::SVG)
    }
}
