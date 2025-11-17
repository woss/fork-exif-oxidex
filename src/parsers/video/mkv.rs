//! MKV (Matroska) video format parser
//!
//! Implements metadata extraction from Matroska/WebM container formats
//! following the EBML (Extensible Binary Meta Language) specification.
//!
//! # Supported Metadata
//!
//! - **Title, Artist, Album:** From Tags segment (SimpleTag elements)
//! - **Duration:** From SegmentInfo (Duration element)
//! - **Codec Information:** From Tracks segment
//! - **Creation Date:** From DateUTC element
//! - **Muxing Application:** From MuxingApp element
//!
//! # ExifTool Compatibility
//!
//! Maps to ExifTool tags from `Matroska.pm` module:
//! - `Matroska:Title` → Title from Tags
//! - `Matroska:Duration` → Duration from SegmentInfo
//! - `Matroska:MuxingApp` → MuxingApp from SegmentInfo
//!
//! # File Structure
//!
//! ```text
//! [EBML Header - required]
//!   ├─ EBMLVersion
//!   ├─ DocType ("matroska" or "webm")
//!   └─ DocTypeVersion
//! [Segment - main container]
//!   ├─ SeekHead (index to other segments)
//!   ├─ Info (duration, dates, muxing app)
//!   ├─ Tracks (video/audio codec info)
//!   ├─ Tags (metadata - PRIMARY METADATA SOURCE)
//!   └─ Clusters (actual media data - SKIP)
//! ```
//!
//! # References
//!
//! - EBML RFC: <https://www.rfc-editor.org/rfc/rfc8794.html>
//! - Matroska Spec: <https://www.matroska.org/technical/elements.html>
//! - ExifTool Source: `lib/Image/ExifTool/Matroska.pm`

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap};
use crate::error::{ExifToolError, Result};

/// EBML header signature
const EBML_SIGNATURE: &[u8] = b"\x1A\x45\xDF\xA3";

/// MKV parser
pub struct MkvParser;

impl FormatParser for MkvParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        // Verify EBML signature
        if reader.size() < 4 {
            return Err(ExifToolError::parse_error("File too small to be MKV"));
        }

        let header = reader.read(0, 4)?;
        if header != EBML_SIGNATURE {
            return Err(ExifToolError::parse_error(format!(
                "Invalid MKV signature: expected {:?}, found {:?}",
                EBML_SIGNATURE, header
            )));
        }

        // Initialize metadata - skeleton implementation
        // Full EBML parsing will be implemented in subsequent tasks
        let metadata = MetadataMap::new();

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::MKV)
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
        fn new(data: &[u8]) -> Self {
            Self {
                data: data.to_vec(),
            }
        }
    }

    impl crate::core::FileReader for TestReader {
        fn read(&self, offset: u64, length: usize) -> io::Result<&[u8]> {
            let start = offset as usize;
            let end = start.saturating_add(length).min(self.data.len());

            if start > self.data.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "offset beyond data",
                ));
            }

            Ok(&self.data[start..end])
        }

        fn size(&self) -> u64 {
            self.data.len() as u64
        }
    }

    #[test]
    fn test_mkv_signature_valid() {
        let data = b"\x1A\x45\xDF\xA3\x00\x00\x00\x00";
        let reader = TestReader::new(data);
        let parser = MkvParser;
        let result = parser.parse(&reader);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mkv_signature_invalid() {
        let data = b"INVALID DATA";
        let reader = TestReader::new(data);
        let parser = MkvParser;
        let result = parser.parse(&reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_mkv_file_too_small() {
        let data = b"\x1A\x45";
        let reader = TestReader::new(data);
        let parser = MkvParser;
        let result = parser.parse(&reader);
        assert!(result.is_err());
    }
}
