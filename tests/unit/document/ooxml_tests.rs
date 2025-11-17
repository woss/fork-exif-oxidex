//! Tests for Office Open XML (DOCX, XLSX, PPTX) parsers

use oxidex::core::FormatParser;
use oxidex::io::BufferedReader;
use oxidex::parsers::document::{DocxParser, XlsxParser, PptxParser};

#[test]
fn test_docx_invalid() {
    let data = b"Not a DOCX file";
    let reader = BufferedReader::from_bytes(data);
    let parser = DocxParser;

    let result = parser.parse(&reader);
    assert!(result.is_err());
}

#[test]
fn test_xlsx_invalid() {
    let data = b"Not an XLSX file";
    let reader = BufferedReader::from_bytes(data);
    let parser = XlsxParser;

    let result = parser.parse(&reader);
    assert!(result.is_err());
}

#[test]
fn test_pptx_invalid() {
    let data = b"Not a PPTX file";
    let reader = BufferedReader::from_bytes(data);
    let parser = PptxParser;

    let result = parser.parse(&reader);
    assert!(result.is_err());
}
