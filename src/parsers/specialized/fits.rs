//! FITS (Flexible Image Transport System) parser

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

const FITS_SIGNATURE: &[u8] = b"SIMPLE";

/// Parser for FITS (Flexible Image Transport System) files
///
/// Extracts metadata from FITS astronomical data files used for scientific imaging.
pub struct FITSParser;

impl FITSParser {
    /// Verifies the FITS file signature ("SIMPLE")
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 6 {
            return Ok(false);
        }
        let header = reader.read(0, 6)?;
        Ok(header == FITS_SIGNATURE)
    }
}

impl FormatParser for FITSParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid FITS signature"));
        }
        let mut metadata = MetadataMap::new();
        metadata.insert("FileType".to_string(), TagValue::String("FITS".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );
        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::FITS)
    }
}
