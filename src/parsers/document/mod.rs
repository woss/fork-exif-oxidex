//! Document format parsers

pub mod epub;
pub mod iwork;
pub mod ooxml;

pub use epub::EpubParser;
pub use iwork::{KeynoteParser, NumbersParser, PagesParser};
pub use ooxml::{DocxParser, PptxParser, XlsxParser};
