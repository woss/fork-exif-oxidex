//! Adobe Photoshop (PSD) format parser

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

const PSD_SIGNATURE: &[u8] = b"8BPS";

pub struct PSDParser;

impl PSDParser {
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 4 {
            return Ok(false);
        }
        let header = reader.read(0, 4)?;
        Ok(header == PSD_SIGNATURE)
    }

    pub fn read_version(reader: &dyn FileReader) -> Result<u16> {
        if reader.size() < 6 {
            return Ok(0);
        }
        let version_bytes = reader.read(4, 2)?;
        Ok(u16::from_be_bytes([version_bytes[0], version_bytes[1]]))
    }
}

impl FormatParser for PSDParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid PSD signature"));
        }
        let mut metadata = MetadataMap::new();
        metadata.insert("FileType".to_string(), TagValue::String("PSD".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );

        let version = Self::read_version(reader)?;
        metadata.insert(
            "PSDVersion".to_string(),
            TagValue::String(version.to_string()),
        );

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::PSD)
    }
}
