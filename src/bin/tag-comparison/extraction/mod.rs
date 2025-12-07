//! Tag extraction modules for both OxiDex and ExifTool

pub mod exiftool_extractor;
pub mod oxidex_extractor;

pub use exiftool_extractor::ExifToolExtractor;
pub use oxidex_extractor::OxiDexExtractor;

use crate::models::TagInfo;

/// Result of extracting tags from files
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    /// Tags extracted from files
    pub tags: Vec<TagInfo>,
    /// Number of files processed
    pub files_processed: usize,
}
