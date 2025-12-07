//! Tag extraction modules for both OxiDex and ExifTool

pub mod exiftool_extractor;
pub mod oxidex_extractor;

pub use exiftool_extractor::ExifToolExtractor;
pub use oxidex_extractor::OxiDexExtractor;
