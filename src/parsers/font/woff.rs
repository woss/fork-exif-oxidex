//! Web Open Font Format (WOFF) parser
//!
//! Implements basic metadata extraction from WOFF font files.

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// WOFF signature: "wOFF"
const WOFF_SIGNATURE: &[u8] = b"wOFF";

/// WOFF parser for extracting metadata from Web Open Fonts
pub struct WOFFParser;

impl WOFFParser {
    /// Verifies WOFF signature
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 4 {
            return Ok(false);
        }

        let header = reader.read(0, 4)?;
        Ok(header == WOFF_SIGNATURE)
    }

    /// Reads flavor (offset 4, 4 bytes) - indicates original font type
    pub fn read_flavor(reader: &dyn FileReader) -> Result<String> {
        if reader.size() < 8 {
            return Ok("Unknown".to_string());
        }

        let flavor = reader.read(4, 4)?;
        if flavor == &[0x00, 0x01, 0x00, 0x00] {
            Ok("TrueType".to_string())
        } else if flavor == b"OTTO" {
            Ok("CFF".to_string())
        } else {
            Ok("Unknown".to_string())
        }
    }
}

impl FormatParser for WOFFParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid WOFF signature"));
        }

        let mut metadata = MetadataMap::new();

        metadata.insert("FileType".to_string(), TagValue::String("WOFF".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );

        let flavor = Self::read_flavor(reader)?;
        metadata.insert("FontFlavor".to_string(), TagValue::String(flavor));

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::WOFF)
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
    fn test_woff_signature() {
        let mut data = b"wOFF".to_vec();
        data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        let reader = TestReader::new(data);
        assert!(WOFFParser::verify_signature(&reader).unwrap());
    }

    #[test]
    fn test_woff_truetype_flavor() {
        let mut data = b"wOFF".to_vec();
        data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        let reader = TestReader::new(data);
        assert_eq!(WOFFParser::read_flavor(&reader).unwrap(), "TrueType");
    }
}
