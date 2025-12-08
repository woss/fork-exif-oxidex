//! APP segment parsers for JPEG files
//!
//! This module contains parsers for various APP segments beyond the standard
//! EXIF, XMP, and IPTC segments.

pub mod app10_hdr;
pub mod app11_jpeg_hdr;
pub mod app12_agfa;
pub mod app12_olympus;

// Re-export main parsing functions
pub use app10_hdr::parse_app10_hdr;
pub use app11_jpeg_hdr::parse_app11_jpeg_hdr;
pub use app12_agfa::parse_app12_agfa;
pub use app12_olympus::parse_app12_olympus;
