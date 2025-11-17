//! ISO 9660 filesystem image parser
//!
//! Implements basic metadata extraction from ISO disc images.

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// ISO 9660 signature at offset 32769: "CD001"
const ISO_SIGNATURE: &[u8] = b"CD001";
const ISO_SIGNATURE_OFFSET: u64 = 32769;

/// ISO parser for extracting metadata from ISO disc images
pub struct ISOParser;

impl ISOParser {
    /// Verifies ISO 9660 signature at offset 32769
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < ISO_SIGNATURE_OFFSET + 5 {
            return Ok(false);
        }

        let signature = reader.read(ISO_SIGNATURE_OFFSET, 5)?;
        Ok(signature == ISO_SIGNATURE)
    }

    /// Reads volume descriptor type (byte at offset 32768)
    pub fn read_descriptor_type(reader: &dyn FileReader) -> Result<u8> {
        if reader.size() < ISO_SIGNATURE_OFFSET {
            return Ok(0);
        }

        let descriptor = reader.read(32768, 1)?;
        Ok(descriptor[0])
    }
}

impl FormatParser for ISOParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        // Verify signature
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid ISO 9660 signature"));
        }

        let mut metadata = MetadataMap::new();

        metadata.insert("FileType".to_string(), TagValue::String("ISO".to_string()));
        metadata.insert("FileSize".to_string(), TagValue::String(reader.size().to_string()));

        // Descriptor type: 1=Primary, 2=Supplementary, 255=Terminator
        let descriptor_type = Self::read_descriptor_type(reader)?;
        metadata.insert("VolumeDescriptorType".to_string(), TagValue::String(descriptor_type.to_string()));

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::ISO)
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
    fn test_iso_signature() {
        let mut data = vec![0u8; 32774];
        data[32768] = 0x01; // Primary volume descriptor
        data[32769..32774].copy_from_slice(b"CD001");
        let reader = TestReader::new(data);
        assert!(ISOParser::verify_signature(&reader).unwrap());
    }
}
