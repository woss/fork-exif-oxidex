//! AutoCAD DWG format parser

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// Parser for AutoCAD DWG (Drawing) files
///
/// Extracts metadata from DWG files including version information and file properties.
pub struct DWGParser;

impl DWGParser {
    /// Verifies the DWG file signature by checking for "AC" prefix followed by version number
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 6 {
            return Ok(false);
        }
        let header = reader.read(0, 6)?;
        // DWG versions: AC1012, AC1014, AC1015, AC1018, AC1021, AC1024, AC1027, AC1032
        Ok(&header[0..2] == b"AC" && header[2] >= b'1' && header[3] >= b'0')
    }

    /// Reads the AutoCAD version string from the file header
    pub fn read_version(reader: &dyn FileReader) -> Result<String> {
        if reader.size() < 6 {
            return Ok("Unknown".to_string());
        }
        let version = reader.read(0, 6)?;
        Ok(String::from_utf8_lossy(version).to_string())
    }
}

impl FormatParser for DWGParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid DWG signature"));
        }
        let mut metadata = MetadataMap::new();
        metadata.insert("FileType".to_string(), TagValue::String("DWG".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );

        let version = Self::read_version(reader)?;
        metadata.insert("DWGVersion".to_string(), TagValue::String(version));

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::DWG)
    }
}
