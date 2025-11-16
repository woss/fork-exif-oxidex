//! APE (Monkey's Audio) format parser
//!
//! Stub implementation - to be implemented in subsequent tasks.

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap};
use crate::error::Result;

/// APE parser
pub struct ApeParser;

impl FormatParser for ApeParser {
    fn parse(&self, _reader: &dyn FileReader) -> Result<MetadataMap> {
        Ok(MetadataMap::new())
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::APE)
    }
}
