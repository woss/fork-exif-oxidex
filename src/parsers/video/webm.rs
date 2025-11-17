//! WebM video format parser
//!
//! Implements metadata extraction from WebM container format (a subset of Matroska).
//! WebM uses the same EBML structure as MKV but is restricted to VP8/VP9/AV1 video
//! and Vorbis/Opus audio codecs.
//!
//! # Supported Metadata
//!
//! - **EBML Header:** DocType verification ("webm")
//! - **Segment Info:** Duration, muxing application, writing application
//! - **Tags:** Title, Artist, Album (from SimpleTag elements)
//!
//! # ExifTool Compatibility
//!
//! Maps to ExifTool tags from `Matroska.pm` module (WebM is a Matroska profile):
//! - `Matroska:DocType` → "webm" from EBML header
//! - `Matroska:Duration` → Duration from SegmentInfo
//! - `Matroska:MuxingApp` → MuxingApp from SegmentInfo
//!
//! # File Structure
//!
//! ```text
//! [EBML Header - required]
//!   ├─ EBMLVersion
//!   ├─ DocType ("webm")
//!   └─ DocTypeVersion
//! [Segment - main container]
//!   ├─ Info (duration, muxing app)
//!   ├─ Tracks (VP8/VP9/AV1 + Vorbis/Opus)
//!   ├─ Tags (metadata)
//!   └─ Clusters (media data)
//! ```
//!
//! # References
//!
//! - WebM Spec: <https://www.webmproject.org/docs/container/>
//! - Matroska Spec: <https://www.matroska.org/technical/elements.html>
//! - ExifTool Source: `lib/Image/ExifTool/Matroska.pm`

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap};
use crate::error::{ExifToolError, Result};

/// EBML header signature (shared with MKV)
const EBML_SIGNATURE: &[u8] = b"\x1A\x45\xDF\xA3";

/// WebM parser
pub struct WebmParser;

impl FormatParser for WebmParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        // Verify EBML signature
        if reader.size() < 4 {
            return Err(ExifToolError::parse_error("File too small to be WebM"));
        }

        let header = reader.read(0, 4)?;
        if header != EBML_SIGNATURE {
            return Err(ExifToolError::parse_error(format!(
                "Invalid WebM/EBML signature: expected {:?}, found {:?}",
                EBML_SIGNATURE, header
            )));
        }

        // WebM files are Matroska files with DocType "webm"
        // For now, we just verify the EBML signature
        // Full EBML parsing will be shared with MKV parser in future tasks

        let metadata = MetadataMap::new();

        // TODO: Parse EBML header to verify DocType is "webm"
        // TODO: Parse SegmentInfo for duration, muxing app, etc.
        // TODO: Parse Tags segment for metadata
        // This will be implemented when we add full EBML parsing support

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::WEBM)
    }
}

/// Convenience function to parse WebM metadata from a reader.
///
/// This is a wrapper around `WebmParser::parse()` to provide a simpler API
/// for the operations module.
///
/// # Arguments
///
/// * `reader` - FileReader implementation providing access to the WebM file
///
/// # Returns
///
/// * `Ok(MetadataMap)` - Successfully extracted metadata
/// * `Err(String)` - Parse error message
pub fn parse_webm_metadata(reader: &dyn FileReader) -> std::result::Result<MetadataMap, String> {
    let parser = WebmParser;
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
    fn test_webm_signature_valid() {
        let data = b"\x1A\x45\xDF\xA3\x00\x00\x00\x00";
        let reader = TestReader::new(data);
        let parser = WebmParser;
        let result = parser.parse(&reader);
        assert!(result.is_ok());
    }

    #[test]
    fn test_webm_signature_invalid() {
        let data = b"INVALID DATA";
        let reader = TestReader::new(data);
        let parser = WebmParser;
        let result = parser.parse(&reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_webm_file_too_small() {
        let data = b"\x1A\x45";
        let reader = TestReader::new(data);
        let parser = WebmParser;
        let result = parser.parse(&reader);
        assert!(result.is_err());
    }
}
