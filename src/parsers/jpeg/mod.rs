//! JPEG format parser
//!
//! Handles JPEG segment marker parsing, EXIF, XMP, and IPTC segment extraction.

#![allow(dead_code)]

pub mod app_parsers;
pub mod exif_parser;
pub mod flir_parser;
pub mod iptc_parser;
pub mod jfif_parser;
pub mod jpeg_hdr_parser;
pub mod segment_parser;
pub mod xmp_parser;

// Re-export segment parser types for convenient access
pub use segment_parser::{parse_segments, Segment};
