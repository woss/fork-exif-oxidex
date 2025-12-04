//! Text-based format parsers

#![allow(dead_code)]

pub mod txt;
pub mod vcf;

pub use txt::{parse_txt_metadata, TXTParser};
pub use vcf::VCFParser;
