use oxidex::parsers::audio::flac::FlacParser;
use oxidex::core::{FormatParser, FileReader};
use std::io;

struct TestReader {
    data: Vec<u8>,
}

impl TestReader {
    fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl FileReader for TestReader {
    fn read(&self, offset: u64, length: usize) -> io::Result<&[u8]> {
        let start = offset as usize;
        let end = start + length;

        if end > self.data.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "read beyond end of file",
            ));
        }

        Ok(&self.data[start..end])
    }

    fn size(&self) -> u64 {
        self.data.len() as u64
    }
}

#[test]
fn test_flac_magic_bytes() {
    let data = b"fLaC\x00\x00\x00\x22..."; // Mock FLAC file
    let reader = TestReader::new(data.to_vec());
    let parser = FlacParser;
    let result = parser.parse(&reader);

    // Should succeed with valid magic bytes
    assert!(result.is_ok());
}

#[test]
fn test_flac_invalid_magic() {
    let data = b"INVALID";
    let reader = TestReader::new(data.to_vec());
    let parser = FlacParser;
    let result = parser.parse(&reader);

    // Should fail with invalid magic bytes
    assert!(result.is_err());
}
