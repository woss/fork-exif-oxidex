//! HDF5 (Hierarchical Data Format) parser

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

const HDF5_SIGNATURE: &[u8] = &[0x89, 0x48, 0x44, 0x46, 0x0D, 0x0A, 0x1A, 0x0A];

pub struct HDF5Parser;

impl HDF5Parser {
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 8 {
            return Ok(false);
        }
        let header = reader.read(0, 8)?;
        Ok(header == HDF5_SIGNATURE)
    }
}

impl FormatParser for HDF5Parser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid HDF5 signature"));
        }
        let mut metadata = MetadataMap::new();
        metadata.insert("FileType".to_string(), TagValue::String("HDF5".to_string()));
        metadata.insert("FileSize".to_string(), TagValue::String(reader.size().to_string()));
        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::HDF5)
    }
}
