//! Tests for ZIP archive parser

use oxidex::core::FormatParser;
use oxidex::io::BufferedReader;
use oxidex::parsers::archive::ZipParser;

#[test]
fn test_zip_invalid_signature() {
    let data = b"Not a ZIP file";
    let reader = BufferedReader::from_bytes(data);
    let parser = ZipParser;

    let result = parser.parse(&reader);
    assert!(result.is_err());
}

#[test]
fn test_zip_too_small() {
    let data = b"PK";
    let reader = BufferedReader::from_bytes(data);
    let parser = ZipParser;

    let result = parser.parse(&reader);
    assert!(result.is_err());
}
