//! TAR archive format parser
//!
//! Implements basic metadata extraction from TAR archive files.

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// TAR signature at offset 257: "ustar"
const TAR_SIGNATURE: &[u8] = b"ustar";
const TAR_SIGNATURE_OFFSET: u64 = 257;

/// TAR parser for extracting metadata from TAR archives
pub struct TARParser;

impl TARParser {
    /// Verifies TAR signature at offset 257
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < TAR_SIGNATURE_OFFSET + 5 {
            return Ok(false);
        }

        let signature = reader.read(TAR_SIGNATURE_OFFSET, 5)?;
        Ok(signature == TAR_SIGNATURE)
    }

    /// Reads TAR format version (POSIX ustar format has "00" after signature)
    pub fn read_version(reader: &dyn FileReader) -> Result<String> {
        if reader.size() < TAR_SIGNATURE_OFFSET + 7 {
            return Ok("Unknown".to_string());
        }

        let version = reader.read(TAR_SIGNATURE_OFFSET + 5, 2)?;
        if version == b"00" {
            Ok("POSIX".to_string())
        } else if version == b"\x00\x00" {
            Ok("GNU".to_string())
        } else {
            Ok("Unknown".to_string())
        }
    }
}

impl FormatParser for TARParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        // Verify signature
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid TAR signature"));
        }

        let mut metadata = MetadataMap::new();

        metadata.insert("FileType".to_string(), TagValue::String("TAR".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );

        let version = Self::read_version(reader)?;
        metadata.insert("TARFormat".to_string(), TagValue::String(version));

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::TAR)
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
    fn test_tar_signature() {
        let mut data = vec![0u8; 264];
        data[257..262].copy_from_slice(b"ustar");
        data[262..264].copy_from_slice(b"00");
        let reader = TestReader::new(data);
        assert!(TARParser::verify_signature(&reader).unwrap());
    }

    #[test]
    fn test_tar_posix_version() {
        let mut data = vec![0u8; 264];
        data[257..262].copy_from_slice(b"ustar");
        data[262..264].copy_from_slice(b"00");
        let reader = TestReader::new(data);
        assert_eq!(TARParser::read_version(&reader).unwrap(), "POSIX");
    }
}
