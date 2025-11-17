use oxidex::parsers::format_detector::detect_format;
use oxidex::core::{FileFormat, FileReader};
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
fn test_detect_flac_by_magic() {
    let data = b"fLaC\x00\x00\x00\x22";
    let reader = TestReader::new(data.to_vec());
    let format = detect_format(&reader).unwrap();
    assert_eq!(format, FileFormat::FLAC);
}
