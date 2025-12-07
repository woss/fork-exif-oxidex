//! Tag extraction modules for both OxiDex and ExifTool

pub mod oxidex_extractor;
pub mod exiftool_extractor;

pub use oxidex_extractor::OxiDexExtractor;
pub use exiftool_extractor::ExifToolExtractor;
