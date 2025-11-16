//! OGG Vorbis audio format parser
//!
//! Stub implementation - to be implemented in subsequent tasks.

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap};
use crate::error::Result;

/// OGG parser
pub struct OggParser;

impl FormatParser for OggParser {
    fn parse(&self, _reader: &dyn FileReader) -> Result<MetadataMap> {
        Ok(MetadataMap::new())
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::OGG)
    }
}
