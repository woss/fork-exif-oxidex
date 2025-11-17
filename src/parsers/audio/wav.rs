//! WAV audio format parser
//!
//! Implements metadata extraction from WAV (RIFF WAVE) audio files,
//! parsing INFO chunks and other metadata containers.
//!
//! # Supported Metadata
//!
//! - **INFO Chunk:** INAM (Name), IART (Artist), ICRD (Creation Date), IGNR (Genre)
//! - **Format Info:** Sample rate, bits per sample, channels
//!
//! # ExifTool Compatibility
//!
//! Maps to ExifTool tags from `RIFF.pm` module:
//! - `RIFF:Title` → INAM from INFO chunk
//! - `RIFF:Artist` → IART from INFO chunk
//! - `RIFF:DateCreated` → ICRD from INFO chunk
//!
//! # File Structure
//!
//! ```text
//! [RIFF header - "RIFF" + size + "WAVE"]
//! [fmt  chunk - format information]
//! [data chunk - audio samples]
//! [INFO chunk - metadata (optional)]
//! [other chunks...]
//! ```
//!
//! # References
//!
//! - RIFF Spec: <https://www.mmsp.ece.mcgill.ca/Documents/AudioFormats/WAVE/WAVE.html>
//! - ExifTool Source: `lib/Image/ExifTool/RIFF.pm`

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};
use encoding_rs::WINDOWS_1252;

/// RIFF signature
const RIFF_SIGNATURE: &[u8] = b"RIFF";

/// WAVE format identifier
const WAVE_FORMAT: &[u8] = b"WAVE";

/// WAV parser
pub struct WavParser;

impl FormatParser for WavParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        // Verify RIFF/WAVE signature
        if reader.size() < 12 {
            return Err(ExifToolError::parse_error("File too small to be WAV"));
        }

        let header = reader.read(0, 12)?;
        if &header[0..4] != RIFF_SIGNATURE {
            return Err(ExifToolError::parse_error(format!(
                "Invalid RIFF signature: expected {:?}, found {:?}",
                RIFF_SIGNATURE,
                &header[0..4]
            )));
        }

        if &header[8..12] != WAVE_FORMAT {
            return Err(ExifToolError::parse_error(format!(
                "Invalid WAVE format: expected {:?}, found {:?}",
                WAVE_FORMAT,
                &header[8..12]
            )));
        }

        let mut metadata = MetadataMap::with_capacity(16);
        let file_size = reader.size();

        // Parse RIFF chunks
        parse_riff_chunks(reader, 12, file_size, &mut metadata)?;

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::WAV)
    }
}

/// Parse RIFF chunks starting from offset
pub(crate) fn parse_riff_chunks(
    reader: &dyn FileReader,
    start_offset: u64,
    end_offset: u64,
    metadata: &mut MetadataMap,
) -> Result<()> {
    let mut offset = start_offset;

    while offset + 8 < end_offset {
        // Read chunk header (4 byte ID + 4 byte size)
        let chunk_header = reader.read(offset, 8)?;

        let chunk_id = &chunk_header[0..4];
        let chunk_size = u32::from_le_bytes([
            chunk_header[4],
            chunk_header[5],
            chunk_header[6],
            chunk_header[7],
        ]) as u64;

        offset += 8;

        // Ensure chunk doesn't extend beyond file
        if offset + chunk_size > end_offset {
            break;
        }

        // Process specific chunks
        match chunk_id {
            b"fmt " => {
                // Parse format chunk
                if chunk_size >= 16 {
                    parse_fmt_chunk(reader, offset, metadata)?;
                }
            }
            b"LIST" => {
                // Parse LIST chunk (may contain INFO)
                if chunk_size >= 4 {
                    let list_type = reader.read(offset, 4)?;
                    if &list_type == b"INFO" {
                        parse_info_chunk(reader, offset + 4, offset + chunk_size, metadata)?;
                    }
                }
            }
            _ => {
                // Skip unknown chunks
            }
        }

        // Move to next chunk (align to even byte boundary)
        offset += chunk_size;
        if chunk_size % 2 == 1 {
            offset += 1; // RIFF chunks are word-aligned
        }
    }

    Ok(())
}

/// Parse fmt chunk (format information)
fn parse_fmt_chunk(reader: &dyn FileReader, offset: u64, metadata: &mut MetadataMap) -> Result<()> {
    let fmt_data = reader.read(offset, 16)?;

    let audio_format = u16::from_le_bytes([fmt_data[0], fmt_data[1]]);
    let num_channels = u16::from_le_bytes([fmt_data[2], fmt_data[3]]);
    let sample_rate = u32::from_le_bytes([fmt_data[4], fmt_data[5], fmt_data[6], fmt_data[7]]);
    let bits_per_sample = u16::from_le_bytes([fmt_data[14], fmt_data[15]]);

    metadata.insert(
        "RIFF:AudioFormat".to_string(),
        TagValue::new_integer(audio_format as i64),
    );
    metadata.insert(
        "RIFF:NumChannels".to_string(),
        TagValue::new_integer(num_channels as i64),
    );
    metadata.insert(
        "RIFF:SampleRate".to_string(),
        TagValue::new_integer(sample_rate as i64),
    );
    metadata.insert(
        "RIFF:BitsPerSample".to_string(),
        TagValue::new_integer(bits_per_sample as i64),
    );

    Ok(())
}

/// Parse INFO chunk (metadata tags)
fn parse_info_chunk(
    reader: &dyn FileReader,
    start_offset: u64,
    end_offset: u64,
    metadata: &mut MetadataMap,
) -> Result<()> {
    let mut offset = start_offset;

    while offset + 8 < end_offset {
        // Read tag header (4 byte ID + 4 byte size)
        let tag_header = reader.read(offset, 8)?;

        let tag_id = &tag_header[0..4];
        let tag_size =
            u32::from_le_bytes([tag_header[4], tag_header[5], tag_header[6], tag_header[7]])
                as usize;

        offset += 8;

        if offset + tag_size as u64 > end_offset {
            break;
        }

        // Read tag value
        let tag_value_bytes = reader.read(offset, tag_size)?;

        // Decode as Windows-1252 (null-terminated)
        let (tag_value, _, _) = WINDOWS_1252.decode(tag_value_bytes);
        let tag_value = tag_value.trim_end_matches('\0').trim();

        if !tag_value.is_empty() {
            // Map INFO tag IDs to readable names
            let tag_name = match tag_id {
                b"INAM" => "RIFF:Title",
                b"IART" => "RIFF:Artist",
                b"ICRD" => "RIFF:DateCreated",
                b"IGNR" => "RIFF:Genre",
                b"ICMT" => "RIFF:Comment",
                b"ICOP" => "RIFF:Copyright",
                b"ISFT" => "RIFF:Software",
                b"ISBJ" => "RIFF:Subject",
                _ => {
                    // Use raw tag ID for unknown tags
                    let _id_str = String::from_utf8_lossy(tag_id);
                    offset += tag_size as u64;
                    if tag_size % 2 == 1 {
                        offset += 1;
                    }
                    continue;
                }
            };

            metadata.insert(
                tag_name.to_string(),
                TagValue::new_string(tag_value.to_string()),
            );
        }

        // Move to next tag (align to even byte boundary)
        offset += tag_size as u64;
        if tag_size % 2 == 1 {
            offset += 1;
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
    fn test_wav_signature_valid() {
        // Minimal WAV file structure
        let mut data = vec![0u8; 100];
        data[0..4].copy_from_slice(b"RIFF");
        data[4..8].copy_from_slice(&44u32.to_le_bytes()); // file size - 8
        data[8..12].copy_from_slice(b"WAVE");

        let reader = TestReader::new(&data);
        let parser = WavParser;
        let result = parser.parse(&reader);
        assert!(result.is_ok());
    }

    #[test]
    fn test_wav_signature_invalid_riff() {
        let data = b"INVALID DATA";
        let reader = TestReader::new(data);
        let parser = WavParser;
        let result = parser.parse(&reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_wav_signature_invalid_wave() {
        let mut data = vec![0u8; 12];
        data[0..4].copy_from_slice(b"RIFF");
        data[4..8].copy_from_slice(&44u32.to_le_bytes());
        data[8..12].copy_from_slice(b"XXXX"); // Invalid WAVE signature

        let reader = TestReader::new(&data);
        let parser = WavParser;
        let result = parser.parse(&reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_wav_file_too_small() {
        let data = b"RIFF";
        let reader = TestReader::new(data);
        let parser = WavParser;
        let result = parser.parse(&reader);
        assert!(result.is_err());
    }
}
