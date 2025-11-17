//! FLV (Flash Video) format parser
//!
//! Implements metadata extraction from FLV video files, parsing the FLV header
//! and onMetaData script data objects.
//!
//! # Supported Metadata
//!
//! - **FLV Header:** Version, flags (video/audio presence)
//! - **onMetaData:** Duration, width, height, framerate, videodatarate, audiodatarate
//!
//! # ExifTool Compatibility
//!
//! Maps to ExifTool tags from `Flash.pm` module:
//! - `Flash:Duration` → duration from onMetaData
//! - `Flash:ImageWidth` → width from onMetaData
//! - `Flash:ImageHeight` → height from onMetaData
//!
//! # File Structure
//!
//! ```text
//! [FLV Header - 9 bytes]
//!   ├─ Signature: "FLV" (3 bytes)
//!   ├─ Version: 1 byte
//!   ├─ Flags: 1 byte (audio/video)
//!   └─ DataOffset: 4 bytes (always 9)
//! [Previous Tag Size 0: 4 bytes]
//! [FLV Tags...]
//!   ├─ Tag Type (1 byte): Audio/Video/Script
//!   ├─ Data Size (3 bytes)
//!   ├─ Timestamp (3 bytes + 1 byte extended)
//!   ├─ Stream ID (3 bytes)
//!   └─ Data (variable)
//! ```
//!
//! # References
//!
//! - FLV Spec: Adobe FLV File Format Specification v10.1
//! - ExifTool Source: `lib/Image/ExifTool/Flash.pm`

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// FLV signature
const FLV_SIGNATURE: &[u8] = b"FLV";

/// FLV tag types
const TAG_TYPE_AUDIO: u8 = 8;
const TAG_TYPE_VIDEO: u8 = 9;
const TAG_TYPE_SCRIPT: u8 = 18;

/// FLV parser
pub struct FlvParser;

impl FormatParser for FlvParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        // Verify FLV signature
        if reader.size() < 9 {
            return Err(ExifToolError::parse_error("File too small to be FLV"));
        }

        let header = reader.read(0, 9)?;
        if &header[0..3] != FLV_SIGNATURE {
            return Err(ExifToolError::parse_error(format!(
                "Invalid FLV signature: expected {:?}, found {:?}",
                FLV_SIGNATURE,
                &header[0..3]
            )));
        }

        let mut metadata = MetadataMap::with_capacity(16);

        // Parse FLV header
        let version = header[3];
        let flags = header[4];
        let has_video = (flags & 0x01) != 0;
        let has_audio = (flags & 0x04) != 0;

        metadata.insert(
            "FLV:Version".to_string(),
            TagValue::new_integer(version as i64),
        );
        metadata.insert(
            "FLV:HasVideo".to_string(),
            TagValue::new_string(has_video.to_string()),
        );
        metadata.insert(
            "FLV:HasAudio".to_string(),
            TagValue::new_string(has_audio.to_string()),
        );

        // Look for onMetaData script tag (first script tag)
        // Skip: Previous Tag Size 0 (4 bytes after header)
        let mut offset = 13u64;
        let file_size = reader.size();

        // Search for first script tag (limited search to avoid scanning entire file)
        let max_search_offset = (offset + 10_000).min(file_size);

        while offset + 11 < max_search_offset {
            // Read tag header (11 bytes)
            let tag_header = reader.read(offset, 11)?;

            let tag_type = tag_header[0];
            let data_size =
                u32::from_be_bytes([0, tag_header[1], tag_header[2], tag_header[3]]) as u64;

            // Check if this is a script data tag
            if tag_type == TAG_TYPE_SCRIPT && data_size > 0 && data_size < 100_000 {
                // Read script data
                let script_data = reader.read(offset + 11, data_size as usize)?;

                // Parse onMetaData (simplified parsing)
                parse_on_metadata(script_data, &mut metadata)?;
                break;
            }

            // Move to next tag (tag header + data size + previous tag size)
            offset += 11 + data_size + 4;
        }

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::FLV)
    }
}

/// Parse onMetaData script data object (simplified AMF0 parsing)
fn parse_on_metadata(data: &[u8], metadata: &mut MetadataMap) -> Result<()> {
    // AMF0 format:
    // - 0x02 (String marker) + length (2 bytes) + "onMetaData"
    // - 0x08 (ECMA array marker) + count (4 bytes) + key-value pairs

    if data.is_empty() || data[0] != 0x02 {
        return Ok(()); // Not a string, skip
    }

    let mut offset = 1;

    // Skip first string (should be "onMetaData")
    if offset + 2 > data.len() {
        return Ok(());
    }
    let str_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
    offset += 2 + str_len;

    // Check for ECMA array marker
    if offset >= data.len() || data[offset] != 0x08 {
        return Ok(()); // Not an ECMA array
    }
    offset += 1;

    // Skip array count (4 bytes)
    if offset + 4 > data.len() {
        return Ok(());
    }
    offset += 4;

    // Parse key-value pairs
    while offset + 3 < data.len() {
        // Read key length
        let key_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;

        if key_len == 0 {
            break; // End of object marker
        }

        if offset + key_len > data.len() {
            break;
        }

        let key = String::from_utf8_lossy(&data[offset..offset + key_len]).to_string();
        offset += key_len;

        if offset >= data.len() {
            break;
        }

        // Read value type
        let value_type = data[offset];
        offset += 1;

        // Parse value based on type
        match value_type {
            0x00 => {
                // Number (8 bytes double)
                if offset + 8 > data.len() {
                    break;
                }
                let value_bytes = &data[offset..offset + 8];
                let value = f64::from_be_bytes([
                    value_bytes[0],
                    value_bytes[1],
                    value_bytes[2],
                    value_bytes[3],
                    value_bytes[4],
                    value_bytes[5],
                    value_bytes[6],
                    value_bytes[7],
                ]);
                offset += 8;

                // Map common metadata fields
                let tag_name = match key.as_str() {
                    "duration" => "FLV:Duration",
                    "width" => "FLV:ImageWidth",
                    "height" => "FLV:ImageHeight",
                    "framerate" => "FLV:FrameRate",
                    "videodatarate" => "FLV:VideoDataRate",
                    "audiodatarate" => "FLV:AudioDataRate",
                    _ => continue,
                };

                // Format numbers appropriately
                if key == "duration" || key == "framerate" || key.ends_with("datarate") {
                    metadata.insert(
                        tag_name.to_string(),
                        TagValue::new_string(format!("{:.2}", value)),
                    );
                } else {
                    metadata.insert(tag_name.to_string(), TagValue::new_integer(value as i64));
                }
            }
            0x01 => {
                // Boolean (1 byte)
                if offset + 1 > data.len() {
                    break;
                }
                offset += 1;
            }
            0x02 => {
                // String (2 bytes length + data)
                if offset + 2 > data.len() {
                    break;
                }
                let str_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
                offset += 2 + str_len;
            }
            _ => {
                // Unknown type, stop parsing
                break;
            }
        }
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
    fn test_flv_signature_valid() {
        // Minimal FLV header
        let mut data = vec![0u8; 100];
        data[0..3].copy_from_slice(b"FLV");
        data[3] = 1; // version
        data[4] = 0x05; // flags (has audio + video)
        data[5..9].copy_from_slice(&9u32.to_be_bytes()); // data offset

        let reader = TestReader::new(&data);
        let parser = FlvParser;
        let result = parser.parse(&reader);
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert_eq!(metadata.get("FLV:Version").unwrap().as_integer(), Some(1));
    }

    #[test]
    fn test_flv_signature_invalid() {
        let data = b"INVALID DATA";
        let reader = TestReader::new(data);
        let parser = FlvParser;
        let result = parser.parse(&reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_flv_file_too_small() {
        let data = b"FLV";
        let reader = TestReader::new(data);
        let parser = FlvParser;
        let result = parser.parse(&reader);
        assert!(result.is_err());
    }
}
