//! OpenType Font (OTF) format parser
//!
//! Implements basic metadata extraction from OpenType font files.

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// OTF signature: "OTTO"
const OTF_SIGNATURE: &[u8] = b"OTTO";

/// OTF parser for extracting metadata from OpenType fonts
pub struct OTFParser;

impl OTFParser {
    /// Verifies OTF signature
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 4 {
            return Ok(false);
        }

        let header = reader.read(0, 4)?;
        Ok(header == OTF_SIGNATURE)
    }

    /// Reads number of tables (offset 4, 2 bytes)
    pub fn read_num_tables(reader: &dyn FileReader) -> Result<u16> {
        if reader.size() < 6 {
            return Ok(0);
        }

        let num_tables_bytes = reader.read(4, 2)?;
        Ok(u16::from_be_bytes([
            num_tables_bytes[0],
            num_tables_bytes[1],
        ]))
    }
}

impl FormatParser for OTFParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid OTF signature"));
        }

        let mut metadata = MetadataMap::new();

        metadata.insert("FileType".to_string(), TagValue::String("OTF".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );

        let num_tables = Self::read_num_tables(reader)?;
        metadata.insert(
            "NumTables".to_string(),
            TagValue::String(num_tables.to_string()),
        );

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::OTF)
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
    fn test_otf_signature() {
        let mut data = b"OTTO".to_vec();
        data.extend_from_slice(&[0x00, 0x10]);
        let reader = TestReader::new(data);
        assert!(OTFParser::verify_signature(&reader).unwrap());
    }
}
