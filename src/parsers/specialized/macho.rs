//! Mach-O (Mach Object) executable format parser

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

const MACHO_MAGIC_32: &[u8] = &[0xFE, 0xED, 0xFA, 0xCE];
const MACHO_MAGIC_64: &[u8] = &[0xFE, 0xED, 0xFA, 0xCF];
const MACHO_MAGIC_32_REV: &[u8] = &[0xCE, 0xFA, 0xED, 0xFE];
const MACHO_MAGIC_64_REV: &[u8] = &[0xCF, 0xFA, 0xED, 0xFE];

pub struct MachOParser;

impl MachOParser {
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 4 {
            return Ok(false);
        }
        let header = reader.read(0, 4)?;
        Ok(header == MACHO_MAGIC_32
            || header == MACHO_MAGIC_64
            || header == MACHO_MAGIC_32_REV
            || header == MACHO_MAGIC_64_REV)
    }

    pub fn read_arch(reader: &dyn FileReader) -> Result<&'static str> {
        if reader.size() < 4 {
            return Ok("Unknown");
        }
        let magic = reader.read(0, 4)?;
        if magic == MACHO_MAGIC_64 || magic == MACHO_MAGIC_64_REV {
            Ok("64-bit")
        } else {
            Ok("32-bit")
        }
    }
}

impl FormatParser for MachOParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid Mach-O signature"));
        }
        let mut metadata = MetadataMap::new();
        metadata.insert(
            "FileType".to_string(),
            TagValue::String("Mach-O".to_string()),
        );
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );

        let arch = Self::read_arch(reader)?;
        metadata.insert(
            "Architecture".to_string(),
            TagValue::String(arch.to_string()),
        );

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::MachO)
    }
}
