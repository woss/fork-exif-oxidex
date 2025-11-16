//! FLV (Flash Video) format parser
//!
//! Stub implementation - to be implemented in subsequent tasks.

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap};
use crate::error::Result;

/// FLV parser
pub struct FlvParser;

impl FormatParser for FlvParser {
    fn parse(&self, _reader: &dyn FileReader) -> Result<MetadataMap> {
        Ok(MetadataMap::new())
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::FLV)
    }
}
