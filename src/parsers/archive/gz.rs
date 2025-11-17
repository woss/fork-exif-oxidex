//! GZIP compressed file format parser
//!
//! Implements basic metadata extraction from GZIP files.

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// GZIP signature: 0x1F 0x8B
const GZ_SIGNATURE: &[u8] = &[0x1F, 0x8B];

/// GZIP compression method offset
const GZ_COMPRESSION_METHOD_OFFSET: u64 = 2;

/// GZIP parser for extracting metadata from compressed files
pub struct GZParser;

impl GZParser {
    /// Verifies GZIP signature
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 2 {
            return Ok(false);
        }

        let header = reader.read(0, 2)?;
        Ok(header == GZ_SIGNATURE)
    }

    /// Reads compression method (should be 8 for DEFLATE)
    pub fn read_compression_method(reader: &dyn FileReader) -> Result<u8> {
        if reader.size() < 3 {
            return Ok(0);
        }

        let method = reader.read(GZ_COMPRESSION_METHOD_OFFSET, 1)?;
        Ok(method[0])
    }

    /// Reads flags byte
    pub fn read_flags(reader: &dyn FileReader) -> Result<u8> {
        if reader.size() < 4 {
            return Ok(0);
        }

        let flags = reader.read(3, 1)?;
        Ok(flags[0])
    }
}

impl FormatParser for GZParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        // Verify signature
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid GZIP signature"));
        }

        let mut metadata = MetadataMap::new();

        metadata.insert("FileType".to_string(), TagValue::String("GZIP".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );

        let method = Self::read_compression_method(reader)?;
        let compression_name = match method {
            8 => "DEFLATE",
            _ => "Unknown",
        };
        metadata.insert(
            "CompressionMethod".to_string(),
            TagValue::String(compression_name.to_string()),
        );

        let flags = Self::read_flags(reader)?;
        metadata.insert(
            "Flags".to_string(),
            TagValue::String(format!("0x{:02X}", flags)),
        );

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::GZ)
    }
}

/// Standalone function for parsing GZIP metadata
///
/// This function provides a convenient interface for parsing GZIP compressed file metadata
/// by instantiating the GZParser and calling its parse method.
///
/// # Arguments
///
/// * `reader` - A FileReader providing access to the GZIP file data
///
/// # Returns
///
/// * `Ok(MetadataMap)` - Successfully extracted metadata
/// * `Err(String)` - Parse error description
pub fn parse_gz_metadata(
    reader: &dyn crate::core::FileReader,
) -> std::result::Result<MetadataMap, String> {
    let parser = GZParser;
    parser
        .parse(reader)
        .map_err(|e| format!("GZIP parse error: {}", e))
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
    fn test_gz_signature() {
        let data = vec![0x1F, 0x8B, 0x08, 0x00];
        let reader = TestReader::new(data);
        assert!(GZParser::verify_signature(&reader).unwrap());
    }

    #[test]
    fn test_gz_compression_method() {
        let data = vec![0x1F, 0x8B, 0x08, 0x00];
        let reader = TestReader::new(data);
        assert_eq!(GZParser::read_compression_method(&reader).unwrap(), 8);
    }
}
