//! Web Open Font Format 2 (WOFF2) parser
//!
//! Implements basic metadata extraction from WOFF2 font files.

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// WOFF2 signature: "wOF2"
const WOFF2_SIGNATURE: &[u8] = b"wOF2";

/// WOFF2 parser for extracting metadata from Web Open Font Format 2 files
pub struct WOFF2Parser;

impl WOFF2Parser {
    /// Verifies WOFF2 signature
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 4 {
            return Ok(false);
        }

        let header = reader.read(0, 4)?;
        Ok(header == WOFF2_SIGNATURE)
    }

    /// Reads flavor (offset 4, 4 bytes)
    pub fn read_flavor(reader: &dyn FileReader) -> Result<String> {
        if reader.size() < 8 {
            return Ok("Unknown".to_string());
        }

        let flavor = reader.read(4, 4)?;
        if flavor == [0x00, 0x01, 0x00, 0x00] {
            Ok("TrueType".to_string())
        } else if flavor == b"OTTO" {
            Ok("CFF".to_string())
        } else {
            Ok("Unknown".to_string())
        }
    }
}

impl FormatParser for WOFF2Parser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid WOFF2 signature"));
        }

        let mut metadata = MetadataMap::new();

        metadata.insert(
            "FileType".to_string(),
            TagValue::String("WOFF2".to_string()),
        );
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );

        let flavor = Self::read_flavor(reader)?;
        metadata.insert("FontFlavor".to_string(), TagValue::String(flavor));

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::WOFF2)
    }
}

/// Parses metadata from WOFF2 files.
///
/// This is a convenience wrapper around WOFF2Parser that provides a functional API.
pub fn parse_woff2_metadata(reader: &dyn FileReader) -> std::result::Result<MetadataMap, String> {
    let parser = WOFF2Parser;
    parser.parse(reader).map_err(|e| e.to_string())
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
    fn test_woff2_signature() {
        let mut data = b"wOF2".to_vec();
        data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        let reader = TestReader::new(data);
        assert!(WOFF2Parser::verify_signature(&reader).unwrap());
    }
}
