//! Document format parsers

pub mod eml;
pub mod epub;
pub mod iwork;
pub mod ooxml;

pub use eml::EmlParser;
pub use epub::EpubParser;
pub use iwork::{KeynoteParser, NumbersParser, PagesParser};
pub use ooxml::{DocxParser, PptxParser, XlsxParser};
