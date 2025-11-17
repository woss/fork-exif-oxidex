//! APE (Monkey's Audio) format parser
//!
//! Implements metadata extraction from APE audio files, parsing the MAC header
//! and APEv2 tags.
//!
//! # Supported Metadata
//!
//! - **MAC Header:** Version, compression level, sample rate, channels, bits per sample
//! - **APEv2 Tags:** Artist, Album, Title, Genre, Year, Track, Comment
//!
//! # ExifTool Compatibility
//!
//! Maps to ExifTool tags from `APE.pm` module:
//! - `APE:Version` → File format version from MAC header
//! - `APE:CompressionLevel` → Compression level from MAC header
//! - `APE:SampleRate` → Sample rate from MAC header
//!
//! # File Structure
//!
//! ```text
//! [MAC Header - 76 bytes]
//!   ├─ Signature: "MAC " (4 bytes)
//!   ├─ Version: 2 bytes
//!   ├─ Compression level: 2 bytes
//!   └─ Audio properties...
//! [APE Frames]
//! [APEv2 Tag - at end of file]
//!   ├─ Preamble: "APETAGEX" (8 bytes)
//!   ├─ Version: 4 bytes (2000)
//!   └─ Tag items
//! ```
//!
//! # References
//!
//! - Monkey's Audio SDK
//! - APEv2 Specification
//! - ExifTool Source: `lib/Image/ExifTool/APE.pm`

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};
use encoding_rs::UTF_8;

/// MAC file signature
const MAC_SIGNATURE: &[u8] = b"MAC ";

/// APEv2 tag signature
const APEV2_SIGNATURE: &[u8] = b"APETAGEX";

/// APEv2 tag version
const APEV2_VERSION: u32 = 2000;

/// APE parser
pub struct ApeParser;

impl FormatParser for ApeParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        let file_size = reader.size();

        // Verify MAC signature
        if file_size < 76 {
            return Err(ExifToolError::parse_error("File too small to be APE"));
        }

        let header = reader.read(0, 76)?;
        if &header[0..4] != MAC_SIGNATURE {
            return Err(ExifToolError::parse_error(format!(
                "Invalid APE signature: expected {:?}, found {:?}",
                MAC_SIGNATURE,
                &header[0..4]
            )));
        }

        let mut metadata = MetadataMap::with_capacity(16);

        // Parse MAC header
        parse_mac_header(header, &mut metadata)?;

        // Look for APEv2 tag at end of file
        // APEv2 tags can be up to several KB, but typically are smaller
        // Read last 8KB to find the footer
        let footer_search_size = 8192u64.min(file_size);
        let footer_offset = file_size - footer_search_size;
        let footer_region = reader.read(footer_offset, footer_search_size as usize)?;

        // Search for APEv2 signature
        if let Some(tag_footer_pos) = find_apev2_footer(footer_region) {
            let tag_footer_offset = footer_offset + tag_footer_pos as u64;

            // Read APEv2 footer (32 bytes)
            let apev2_footer = reader.read(tag_footer_offset, 32)?;

            // Parse footer to get tag size
            if let Ok(tag_size) = parse_apev2_footer(apev2_footer) {
                // Read entire tag (including footer)
                let tag_start = tag_footer_offset - tag_size as u64;
                if tag_start < file_size && tag_size < 1_000_000 {
                    // Safety limit
                    let tag_data = reader.read(tag_start, (tag_size + 32) as usize)?;
                    parse_apev2_tag(tag_data, &mut metadata)?;
                }
            }
        }

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::APE)
    }
}

/// Parse MAC header (76 bytes)
fn parse_mac_header(header: &[u8], metadata: &mut MetadataMap) -> Result<()> {
    if header.len() < 76 {
        return Err(ExifToolError::parse_error("MAC header too small"));
    }

    // Version (bytes 4-5, little-endian)
    let version = u16::from_le_bytes([header[4], header[5]]);

    // Compression level (bytes 6-7, little-endian)
    let compression_level = u16::from_le_bytes([header[6], header[7]]);

    // Sample rate (bytes 16-19, little-endian) - offset depends on version
    // For v3.98+ (version >= 3980)
    let sample_rate = if version >= 3980 {
        u32::from_le_bytes([header[16], header[17], header[18], header[19]])
    } else {
        // Older versions have different layout
        u32::from_le_bytes([header[12], header[13], header[14], header[15]])
    };

    // Channels (bytes 22-23, little-endian) - varies by version
    let channels = if version >= 3980 {
        u16::from_le_bytes([header[22], header[23]])
    } else {
        u16::from_le_bytes([header[18], header[19]])
    };

    // Bits per sample (bytes 24-25, little-endian) - varies by version
    let bits_per_sample = if version >= 3980 {
        u16::from_le_bytes([header[24], header[25]])
    } else {
        u16::from_le_bytes([header[20], header[21]])
    };

    metadata.insert(
        "APE:Version".to_string(),
        TagValue::new_string(format!("{:.2}", version as f64 / 1000.0)),
    );

    let compression_name = match compression_level {
        1000 => "Fast",
        2000 => "Normal",
        3000 => "High",
        4000 => "Extra High",
        5000 => "Insane",
        _ => "Unknown",
    };
    metadata.insert(
        "APE:CompressionLevel".to_string(),
        TagValue::new_string(compression_name.to_string()),
    );

    metadata.insert(
        "APE:SampleRate".to_string(),
        TagValue::new_integer(sample_rate as i64),
    );
    metadata.insert(
        "APE:Channels".to_string(),
        TagValue::new_integer(channels as i64),
    );
    metadata.insert(
        "APE:BitsPerSample".to_string(),
        TagValue::new_integer(bits_per_sample as i64),
    );

    Ok(())
}

/// Find APEv2 footer in data
fn find_apev2_footer(data: &[u8]) -> Option<usize> {
    // Search for "APETAGEX" signature
    data.windows(8)
        .position(|window| window == APEV2_SIGNATURE)
}

/// Parse APEv2 footer to get tag size (32 bytes)
fn parse_apev2_footer(footer: &[u8]) -> Result<u32> {
    if footer.len() < 32 || &footer[0..8] != APEV2_SIGNATURE {
        return Err(ExifToolError::parse_error("Invalid APEv2 footer"));
    }

    // Version (bytes 8-11, little-endian)
    let version = u32::from_le_bytes([footer[8], footer[9], footer[10], footer[11]]);
    if version != APEV2_VERSION {
        return Err(ExifToolError::parse_error("Unsupported APEv2 version"));
    }

    // Tag size excluding footer (bytes 12-15, little-endian)
    let tag_size = u32::from_le_bytes([footer[12], footer[13], footer[14], footer[15]]);

    Ok(tag_size)
}

/// Parse APEv2 tag
fn parse_apev2_tag(data: &[u8], metadata: &mut MetadataMap) -> Result<()> {
    if data.len() < 32 {
        return Ok(());
    }

    // Verify header signature (APEv2 tags have both header and footer)
    if &data[0..8] != APEV2_SIGNATURE {
        return Ok(());
    }

    // Item count (bytes 16-19, little-endian)
    let item_count = u32::from_le_bytes([data[16], data[17], data[18], data[19]]);

    // Safety limit
    const MAX_ITEMS: u32 = 1000;
    let safe_item_count = item_count.min(MAX_ITEMS);

    // Parse tag items (start after 32-byte header)
    let mut offset = 32;

    for _ in 0..safe_item_count {
        if offset + 8 > data.len() {
            break;
        }

        // Item value size (4 bytes, little-endian)
        let value_size = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;

        // Item flags (4 bytes, little-endian)
        let _flags = u32::from_le_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]);

        offset += 8;

        // Read key (null-terminated UTF-8 string)
        let key_start = offset;
        let mut key_end = key_start;
        while key_end < data.len() && data[key_end] != 0 {
            key_end += 1;
        }

        if key_end >= data.len() {
            break;
        }

        let key_bytes = &data[key_start..key_end];
        let (key, _, _) = UTF_8.decode(key_bytes);

        offset = key_end + 1; // Skip null terminator

        // Read value
        if offset + value_size > data.len() {
            break;
        }

        let value_bytes = &data[offset..offset + value_size];
        let (value, _, _) = UTF_8.decode(value_bytes);

        // Store metadata with APE: prefix
        let tag_name = format!("APE:{}", key);
        metadata.insert(
            tag_name,
            TagValue::new_string(value.trim_end_matches('\0').to_string()),
        );

        offset += value_size;
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
    fn test_ape_signature_valid() {
        // Minimal APE file with MAC header
        let mut data = vec![0u8; 1000];
        data[0..4].copy_from_slice(b"MAC ");
        data[4..6].copy_from_slice(&3990u16.to_le_bytes()); // version 3.99
        data[6..8].copy_from_slice(&2000u16.to_le_bytes()); // normal compression
        data[16..20].copy_from_slice(&44100u32.to_le_bytes()); // sample rate
        data[22..24].copy_from_slice(&2u16.to_le_bytes()); // stereo
        data[24..26].copy_from_slice(&16u16.to_le_bytes()); // 16-bit

        let reader = TestReader::new(&data);
        let parser = ApeParser;
        let result = parser.parse(&reader);
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert_eq!(
            metadata.get("APE:SampleRate").unwrap().as_integer(),
            Some(44100)
        );
    }

    #[test]
    fn test_ape_signature_invalid() {
        let data = b"INVALID DATA";
        let reader = TestReader::new(data);
        let parser = ApeParser;
        let result = parser.parse(&reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_ape_file_too_small() {
        let data = b"MAC ";
        let reader = TestReader::new(data);
        let parser = ApeParser;
        let result = parser.parse(&reader);
        assert!(result.is_err());
    }
}
