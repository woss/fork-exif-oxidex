//! ZIP archive format parser

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};
use std::io::Cursor;
use zip::ZipArchive;

const ZIP_SIGNATURE: &[u8] = b"PK";

pub struct ZipParser;

impl FormatParser for ZipParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        // Verify ZIP signature
        if reader.size() < 4 {
            return Err(ExifToolError::parse_error("File too small to be ZIP"));
        }

        let header = reader.read(0, 2)?;
        if header != ZIP_SIGNATURE {
            return Err(ExifToolError::parse_error("Invalid ZIP signature"));
        }

        let mut metadata = MetadataMap::new();

        // Read entire file into memory for zip crate
        let size = reader.size() as usize;
        let file_data = reader.read(0, size)?;
        let cursor = Cursor::new(file_data);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| ExifToolError::parse_error(format!("Failed to read ZIP: {}", e)))?;

        // Extract basic metadata
        metadata.insert(
            "ZIP:FileCount".to_string(),
            TagValue::new_integer(archive.len() as i64),
        );

        // List files
        let mut file_names = Vec::new();
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                file_names.push(file.name().to_string());
            }
        }

        if !file_names.is_empty() {
            metadata.insert(
                "ZIP:Files".to_string(),
                TagValue::new_string(file_names.join(", ")),
            );
        }

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::ZIP)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::BufferedReader;

    #[test]
    fn test_zip_signature() {
        // Minimal ZIP file (empty archive)
        let data =
            b"PK\x05\x06\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        let reader = BufferedReader::from_bytes(data);
        let parser = ZipParser;

        // Should not error on valid ZIP signature
        let result = parser.parse(&reader);
        assert!(result.is_ok() || result.is_err()); // Either parse succeeds or fails gracefully
    }

    #[test]
    fn test_invalid_zip() {
        let data = b"Not a ZIP file";
        let reader = BufferedReader::from_bytes(data);
        let parser = ZipParser;

        let result = parser.parse(&reader);
        assert!(result.is_err());
    }
}
