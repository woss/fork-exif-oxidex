//! Infrastructure: Format adapters
//!
//! This module contains format-specific parsers implementing the FormatParser trait.
//! Each format is organized as a separate submodule.

#![allow(dead_code)]

pub mod archive;
pub mod audio;
pub mod common;
pub mod document;
pub mod font;
pub mod format_detector;
pub mod icc_parser;
pub mod jpeg;
pub mod pdf;
pub mod pe;
pub mod png;
pub mod quicktime;
pub mod raw;
pub mod tiff;
pub mod video;
pub mod xmp;

// Re-export the format detection function for convenient access
pub use format_detector::detect_format;
