//! Document format parsers

pub mod ooxml;
pub mod iwork;
pub mod epub;

pub use ooxml::{DocxParser, XlsxParser, PptxParser};
pub use iwork::{PagesParser, NumbersParser, KeynoteParser};
pub use epub::EpubParser;
