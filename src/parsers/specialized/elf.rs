//! ELF (Executable and Linkable Format) parser

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

const ELF_SIGNATURE: &[u8] = &[0x7F, 0x45, 0x4C, 0x46]; // "\x7FELF"

pub struct ELFParser;

impl ELFParser {
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 4 {
            return Ok(false);
        }
        let header = reader.read(0, 4)?;
        Ok(header == ELF_SIGNATURE)
    }

    pub fn read_class(reader: &dyn FileReader) -> Result<&'static str> {
        if reader.size() < 5 {
            return Ok("Unknown");
        }
        let class_byte = reader.read(4, 1)?;
        match class_byte[0] {
            1 => Ok("32-bit"),
            2 => Ok("64-bit"),
            _ => Ok("Unknown"),
        }
    }
}

impl FormatParser for ELFParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid ELF signature"));
        }
        let mut metadata = MetadataMap::new();
        metadata.insert("FileType".to_string(), TagValue::String("ELF".to_string()));
        metadata.insert("FileSize".to_string(), TagValue::String(reader.size().to_string()));
        
        let class = Self::read_class(reader)?;
        metadata.insert("ELFClass".to_string(), TagValue::String(class.to_string()));
        
        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::ELF)
    }
}
