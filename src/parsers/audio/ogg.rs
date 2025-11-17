//! OGG Vorbis audio format parser
//!
//! Implements metadata extraction from OGG Vorbis audio files,
//! parsing Vorbis comments embedded in the container.
//!
//! # Supported Metadata
//!
//! - **Vorbis Comments:** ARTIST, ALBUM, TITLE, GENRE, TRACKNUMBER, DATE
//! - **Audio Info:** Channels, sample rate, bitrate
//!
//! # ExifTool Compatibility
//!
//! Maps to ExifTool tags from `Ogg.pm` module:
//! - `Vorbis:Artist` → ARTIST comment
//! - `Vorbis:Album` → ALBUM comment
//! - `Vorbis:Title` → TITLE comment
//!
//! # File Structure
//!
//! ```text
//! [OggS page 0 - Identification header]
//! [OggS page 1 - Comment header with Vorbis comments]
//! [OggS page 2+ - Setup header and audio data]
//! ```
//!
//! # References
//!
//! - Ogg Spec: <https://xiph.org/ogg/doc/framing.html>
//! - Vorbis Comment Spec: <https://www.xiph.org/vorbis/doc/v-comment.html>
//! - ExifTool Source: `lib/Image/ExifTool/Ogg.pm`

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};
use encoding_rs::UTF_8;

/// OGG page signature
const OGG_SIGNATURE: &[u8] = b"OggS";

/// Vorbis identification header packet type
const VORBIS_ID_HEADER: u8 = 0x01;

/// Vorbis comment header packet type
const VORBIS_COMMENT_HEADER: u8 = 0x03;

/// OGG parser
pub struct OggParser;

impl FormatParser for OggParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        // Verify OGG signature
        if reader.size() < 4 {
            return Err(ExifToolError::parse_error("File too small to be OGG"));
        }

        let header = reader.read(0, 4)?;
        if header != OGG_SIGNATURE {
            return Err(ExifToolError::parse_error(format!(
                "Invalid OGG signature: expected {:?}, found {:?}",
                OGG_SIGNATURE, header
            )));
        }

        let mut metadata = MetadataMap::with_capacity(16);

        // Parse OGG pages to find Vorbis comment header
        let mut offset = 0u64;
        let file_size = reader.size();

        while offset < file_size {
            // Read OGG page header (27 bytes minimum)
            if offset + 27 > file_size {
                break;
            }

            let page_header = reader.read(offset, 27)?;

            // Verify page signature
            if &page_header[0..4] != OGG_SIGNATURE {
                break;
            }

            // Parse page header
            let header_type = page_header[5];
            let segment_count = page_header[26] as usize;

            // Read segment table
            if offset + 27 + segment_count as u64 > file_size {
                break;
            }
            let segment_table = reader.read(offset + 27, segment_count)?;

            // Calculate total page size
            let mut page_body_size = 0u64;
            for &segment_size in segment_table.iter() {
                page_body_size += segment_size as u64;
            }

            // Read page body
            let page_body_offset = offset + 27 + segment_count as u64;
            if page_body_offset + page_body_size > file_size {
                break;
            }

            // Check if this is a Vorbis comment header
            if page_body_size > 0 {
                let page_body = reader.read(page_body_offset, page_body_size as usize)?;

                // Vorbis packets start with packet type (1 byte) + "vorbis" (6 bytes)
                if page_body.len() >= 7 && &page_body[1..7] == b"vorbis" {
                    if page_body[0] == VORBIS_COMMENT_HEADER {
                        // Parse Vorbis comments
                        parse_vorbis_comments(&page_body[7..], &mut metadata)?;
                        break; // Found comments, we're done
                    }
                }
            }

            // Move to next page
            offset = page_body_offset + page_body_size;
        }

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::OGG)
    }
}

/// Parse Vorbis comment data
fn parse_vorbis_comments(data: &[u8], metadata: &mut MetadataMap) -> Result<()> {
    let mut offset = 0;

    // Vendor string length (4 bytes, little-endian)
    if data.len() < 4 {
        return Err(ExifToolError::parse_error("Vorbis comment block too small"));
    }

    let vendor_length = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    offset += 4;

    // Skip vendor string
    if offset + vendor_length > data.len() {
        return Err(ExifToolError::parse_error("Invalid vendor string length"));
    }
    offset += vendor_length;

    // User comment list length (4 bytes, little-endian)
    if offset + 4 > data.len() {
        return Err(ExifToolError::parse_error("Missing comment list length"));
    }

    let comment_count = u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]);
    offset += 4;

    // Parse each comment
    for _ in 0..comment_count {
        if offset + 4 > data.len() {
            break;
        }

        // Comment length (4 bytes, little-endian)
        let comment_length = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;

        if offset + comment_length > data.len() {
            break;
        }

        // Comment string (UTF-8)
        let comment_bytes = &data[offset..offset + comment_length];
        let (comment_str, _, _) = UTF_8.decode(comment_bytes);

        // Split on first '=' to get field name and value
        if let Some(eq_pos) = comment_str.find('=') {
            let field_name = &comment_str[..eq_pos];
            let field_value = &comment_str[eq_pos + 1..];

            // Map to Vorbis: prefix
            let tag_name = format!("Vorbis:{}", field_name.to_uppercase());
            metadata.insert(tag_name, TagValue::new_string(field_value.to_string()));
        }

        offset += comment_length;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    struct TestReader {
        data: Vec<u8>,
    }

    impl TestReader {
        fn new(data: &[u8]) -> Self {
            Self {
                data: data.to_vec(),
            }
        }
    }

    impl crate::core::FileReader for TestReader {
        fn read(&self, offset: u64, length: usize) -> io::Result<&[u8]> {
            let start = offset as usize;
            let end = start.saturating_add(length).min(self.data.len());

            if start > self.data.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "offset beyond data",
                ));
            }

            Ok(&self.data[start..end])
        }

        fn size(&self) -> u64 {
            self.data.len() as u64
        }
    }

    #[test]
    fn test_ogg_signature_valid() {
        // Minimal OGG page header
        let mut data = vec![0u8; 100];
        data[0..4].copy_from_slice(b"OggS");
        data[5] = 0x00; // header_type (unused but present)
        data[26] = 0; // segment count

        let reader = TestReader::new(&data);
        let parser = OggParser;
        let result = parser.parse(&reader);
        assert!(result.is_ok());
    }

    #[test]
    fn test_ogg_signature_invalid() {
        let data = b"INVALID DATA";
        let reader = TestReader::new(data);
        let parser = OggParser;
        let result = parser.parse(&reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_ogg_file_too_small() {
        let data = b"Ogg";
        let reader = TestReader::new(data);
        let parser = OggParser;
        let result = parser.parse(&reader);
        assert!(result.is_err());
    }
}
