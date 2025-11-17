//! Advanced image format parsers

pub mod avif;
pub mod bpg;
pub mod exr;
pub mod flif;
pub mod ico;
pub mod jxl;
pub mod psd;
pub mod svg;

pub use avif::AVIFParser;
pub use bpg::BPGParser;
pub use exr::EXRParser;
pub use flif::FLIFParser;
pub use ico::ICOParser;
pub use jxl::JXLParser;
pub use psd::PSDParser;
pub use svg::SVGParser;
