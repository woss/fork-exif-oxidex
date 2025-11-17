//! RAR archive format parser
//!
//! Implements basic metadata extraction from RAR archive files.

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// RAR signature: "Rar!" (0x52 0x61 0x72 0x21)
const RAR_SIGNATURE: &[u8] = b"Rar!";

/// RAR5 signature (additional marker at offset 7)
const RAR5_MARKER: u8 = 0x01;

/// RAR parser for extracting metadata from RAR archives
pub struct RARParser;

impl RARParser {
    /// Verifies RAR signature
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 7 {
            return Ok(false);
        }

        let header = reader.read(0, 7)?;
        Ok(header.starts_with(RAR_SIGNATURE))
    }

    /// Detects RAR version (4.x or 5.0)
    pub fn detect_version(reader: &dyn FileReader) -> Result<&'static str> {
        if reader.size() < 8 {
            return Ok("Unknown");
        }

        let header = reader.read(0, 8)?;
        if header.len() >= 7 && &header[0..4] == RAR_SIGNATURE {
            // RAR5 has 0x01 at offset 7
            if header.len() >= 8 && header[7] == RAR5_MARKER {
                Ok("5.0")
            } else {
                Ok("4.x")
            }
        } else {
            Ok("Unknown")
        }
    }
}

impl FormatParser for RARParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        // Verify signature
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid RAR signature"));
        }

        let mut metadata = MetadataMap::new();

        // Detect version
        let version = Self::detect_version(reader)?;
        metadata.insert("FileType".to_string(), TagValue::String("RAR".to_string()));
        metadata.insert(
            "RARVersion".to_string(),
            TagValue::String(version.to_string()),
        );
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::RAR)
    }
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
    fn test_rar_signature() {
        let mut data = b"Rar!".to_vec();
        data.extend_from_slice(&[0x1A, 0x07, 0x00]);
        let reader = TestReader::new(data);
        assert!(RARParser::verify_signature(&reader).unwrap());
    }

    #[test]
    fn test_rar5_detection() {
        let mut data = b"Rar!".to_vec();
        data.extend_from_slice(&[0x1A, 0x07, 0x01, 0x01]);
        let reader = TestReader::new(data);
        assert_eq!(RARParser::detect_version(&reader).unwrap(), "5.0");
    }
}
