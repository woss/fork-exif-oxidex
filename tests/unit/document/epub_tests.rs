//! Tests for EPUB e-book parser

use oxidex::core::FormatParser;
use oxidex::io::BufferedReader;
use oxidex::parsers::document::EpubParser;

#[test]
fn test_epub_invalid() {
    let data = b"Not an EPUB file";
    let reader = BufferedReader::from_bytes(data);
    let parser = EpubParser;

    let result = parser.parse(&reader);
    assert!(result.is_err());
}
