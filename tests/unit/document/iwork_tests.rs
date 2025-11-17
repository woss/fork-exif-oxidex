//! Tests for Apple iWork (Pages, Numbers, Keynote) parsers

use oxidex::core::FormatParser;
use oxidex::io::BufferedReader;
use oxidex::parsers::document::{KeynoteParser, NumbersParser, PagesParser};

#[test]
fn test_pages_invalid() {
    let data = b"Not a Pages file";
    let reader = BufferedReader::from_bytes(data);
    let parser = PagesParser;

    let result = parser.parse(&reader);
    assert!(result.is_err());
}

#[test]
fn test_numbers_invalid() {
    let data = b"Not a Numbers file";
    let reader = BufferedReader::from_bytes(data);
    let parser = NumbersParser;

    let result = parser.parse(&reader);
    assert!(result.is_err());
}

#[test]
fn test_keynote_invalid() {
    let data = b"Not a Keynote file";
    let reader = BufferedReader::from_bytes(data);
    let parser = KeynoteParser;

    let result = parser.parse(&reader);
    assert!(result.is_err());
}
