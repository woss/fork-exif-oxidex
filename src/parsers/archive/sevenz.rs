//! 7z archive format parser
//!
//! Implements basic metadata extraction from 7-Zip archive files.

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// 7z signature: 0x37 0x7A 0xBC 0xAF 0x27 0x1C
const SEVENZ_SIGNATURE: &[u8] = &[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C];

/// 7z parser for extracting metadata from 7-Zip archives
pub struct SevenZParser;

impl SevenZParser {
    /// Verifies 7z signature
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 6 {
            return Ok(false);
        }

        let header = reader.read(0, 6)?;
        Ok(header == SEVENZ_SIGNATURE)
    }

    /// Reads version bytes (major.minor)
    pub fn read_version(reader: &dyn FileReader) -> Result<String> {
        if reader.size() < 8 {
            return Ok("Unknown".to_string());
        }

        let header = reader.read(0, 8)?;
        if header.len() >= 8 {
            let major = header[6];
            let minor = header[7];
            Ok(format!("{}.{}", major, minor))
        } else {
            Ok("Unknown".to_string())
        }
    }
}

impl FormatParser for SevenZParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        // Verify signature
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid 7z signature"));
        }

        let mut metadata = MetadataMap::new();

        // Extract version
        let version = Self::read_version(reader)?;
        metadata.insert("FileType".to_string(), TagValue::String("7z".to_string()));
        metadata.insert("7zVersion".to_string(), TagValue::String(version));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::SevenZ)
    }
}

/// Standalone function for parsing 7z metadata
///
/// This function provides a convenient interface for parsing 7-Zip archive metadata
/// by instantiating the SevenZParser and calling its parse method.
///
/// # Arguments
///
/// * `reader` - A FileReader providing access to the 7z file data
///
/// # Returns
///
/// * `Ok(MetadataMap)` - Successfully extracted metadata
/// * `Err(String)` - Parse error description
pub fn parse_7z_metadata(
    reader: &dyn crate::core::FileReader,
) -> std::result::Result<MetadataMap, String> {
    let parser = SevenZParser;
    parser
        .parse(reader)
        .map_err(|e| format!("7z parse error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    struct TestReader {
        data: Vec<u8>,
    }

    impl TestReader {
        fn new(data: Vec<u8>) -> Self {
            Self { data }
        }
    }

    impl FileReader for TestReader {
        fn read(&self, offset: u64, length: usize) -> io::Result<&[u8]> {
            let start = offset as usize;
            let end = start.saturating_add(length).min(self.data.len());
            if start > self.data.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "offset beyond end",
                ));
            }
            Ok(&self.data[start..end])
        }

        fn size(&self) -> u64 {
            self.data.len() as u64
        }
    }

    #[test]
    fn test_7z_signature() {
        let data = vec![0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C];
        let reader = TestReader::new(data);
        assert!(SevenZParser::verify_signature(&reader).unwrap());
    }

    #[test]
    fn test_7z_version() {
        let data = vec![0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C, 0x00, 0x04];
        let reader = TestReader::new(data);
        assert_eq!(SevenZParser::read_version(&reader).unwrap(), "0.4");
    }
}
