//! AVI (Audio Video Interleave) format parser
//!
//! Implements metadata extraction from AVI video files using the RIFF
//! container format. Shares RIFF parsing logic with WAV parser.
//!
//! # Supported Metadata
//!
//! - **INFO Chunk:** INAM (Name), IART (Artist), ICRD (Creation Date), IGNR (Genre)
//! - **Stream Headers:** Video/audio codec information
//! - **Main Header:** Frame rate, dimensions, total frames
//!
//! # ExifTool Compatibility
//!
//! Maps to ExifTool tags from `RIFF.pm` module:
//! - `RIFF:Title` → INAM from INFO chunk
//! - `RIFF:Artist` → IART from INFO chunk
//! - `RIFF:FrameRate` → From main AVI header
//!
//! # File Structure
//!
//! ```text
//! [RIFF header - "RIFF" + size + "AVI "]
//! [LIST hdrl - Header list]
//!   ├─ avih (Main AVI header)
//!   └─ LIST strl (Stream headers)
//! [LIST INFO - Metadata (optional)]
//! [LIST movi - Movie data]
//! [idx1 - Index (optional)]
//! ```
//!
//! # References
//!
//! - AVI Spec: <https://msdn.microsoft.com/en-us/library/windows/desktop/dd318189>
//! - ExifTool Source: `lib/Image/ExifTool/RIFF.pm`

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// RIFF signature
const RIFF_SIGNATURE: &[u8] = b"RIFF";

/// AVI format identifier (note the space at the end)
const AVI_FORMAT: &[u8] = b"AVI ";

/// AVI parser
pub struct AviParser;

impl FormatParser for AviParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        // Verify RIFF/AVI signature
        if reader.size() < 12 {
            return Err(ExifToolError::parse_error("File too small to be AVI"));
        }

        let header = reader.read(0, 12)?;
        if &header[0..4] != RIFF_SIGNATURE {
            return Err(ExifToolError::parse_error(format!(
                "Invalid RIFF signature: expected {:?}, found {:?}",
                RIFF_SIGNATURE,
                &header[0..4]
            )));
        }

        if &header[8..12] != AVI_FORMAT {
            return Err(ExifToolError::parse_error(format!(
                "Invalid AVI format: expected {:?}, found {:?}",
                AVI_FORMAT,
                &header[8..12]
            )));
        }

        let mut metadata = MetadataMap::with_capacity(16);
        let file_size = reader.size();

        // Parse RIFF chunks (shared with WAV parser)
        parse_avi_chunks(reader, 12, file_size, &mut metadata)?;

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::AVI)
    }
}

/// Parse AVI RIFF chunks
fn parse_avi_chunks(
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
            b"LIST" => {
                // Parse LIST chunk
                if chunk_size >= 4 {
                    let list_type = reader.read(offset, 4)?;
                    match &list_type[..] {
                        b"hdrl" => {
                            // Header list - parse AVI header
                            parse_hdrl_list(reader, offset + 4, offset + chunk_size, metadata)?;
                        }
                        b"INFO" => {
                            // Metadata list - reuse WAV INFO parser
                            crate::parsers::audio::wav::parse_riff_chunks(
                                reader,
                                offset,
                                offset + chunk_size,
                                metadata,
                            )?;
                        }
                        _ => {
                            // Skip other LIST types (movi, etc.)
                        }
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

/// Parse hdrl LIST (header list with avih chunk)
fn parse_hdrl_list(
    reader: &dyn FileReader,
    start_offset: u64,
    end_offset: u64,
    metadata: &mut MetadataMap,
) -> Result<()> {
    let mut offset = start_offset;

    while offset + 8 < end_offset {
        // Read chunk header
        let chunk_header = reader.read(offset, 8)?;

        let chunk_id = &chunk_header[0..4];
        let chunk_size = u32::from_le_bytes([
            chunk_header[4],
            chunk_header[5],
            chunk_header[6],
            chunk_header[7],
        ]) as u64;

        offset += 8;

        if offset + chunk_size > end_offset {
            break;
        }

        // Parse avih (main AVI header)
        if chunk_id == b"avih" && chunk_size >= 56 {
            parse_avih_chunk(reader, offset, metadata)?;
        }

        // Move to next chunk
        offset += chunk_size;
        if chunk_size % 2 == 1 {
            offset += 1;
        }
    }

    Ok(())
}

/// Parse avih chunk (main AVI header)
fn parse_avih_chunk(
    reader: &dyn FileReader,
    offset: u64,
    metadata: &mut MetadataMap,
) -> Result<()> {
    let avih_data = reader.read(offset, 56)?;

    let microsec_per_frame =
        u32::from_le_bytes([avih_data[0], avih_data[1], avih_data[2], avih_data[3]]);
    let total_frames =
        u32::from_le_bytes([avih_data[16], avih_data[17], avih_data[18], avih_data[19]]);
    let width = u32::from_le_bytes([avih_data[32], avih_data[33], avih_data[34], avih_data[35]]);
    let height = u32::from_le_bytes([avih_data[36], avih_data[37], avih_data[38], avih_data[39]]);

    // Calculate frame rate from microseconds per frame
    if microsec_per_frame > 0 {
        let frame_rate = 1_000_000.0 / microsec_per_frame as f64;
        metadata.insert(
            "RIFF:FrameRate".to_string(),
            TagValue::new_string(format!("{:.2}", frame_rate)),
        );
    }

    metadata.insert(
        "RIFF:TotalFrames".to_string(),
        TagValue::new_integer(total_frames as i64),
    );
    metadata.insert(
        "RIFF:ImageWidth".to_string(),
        TagValue::new_integer(width as i64),
    );
    metadata.insert(
        "RIFF:ImageHeight".to_string(),
        TagValue::new_integer(height as i64),
    );

    // Calculate duration if we have frame rate and total frames
    if microsec_per_frame > 0 && total_frames > 0 {
        let duration_secs = (microsec_per_frame as f64 * total_frames as f64) / 1_000_000.0;
        metadata.insert(
            "RIFF:Duration".to_string(),
            TagValue::new_string(format!("{:.2}", duration_secs)),
        );
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
    fn test_avi_signature_valid() {
        // Minimal AVI file structure
        let mut data = vec![0u8; 100];
        data[0..4].copy_from_slice(b"RIFF");
        data[4..8].copy_from_slice(&100u32.to_le_bytes());
        data[8..12].copy_from_slice(b"AVI ");

        let reader = TestReader::new(&data);
        let parser = AviParser;
        let result = parser.parse(&reader);
        assert!(result.is_ok());
    }

    #[test]
    fn test_avi_signature_invalid_riff() {
        let data = b"INVALID DATA";
        let reader = TestReader::new(data);
        let parser = AviParser;
        let result = parser.parse(&reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_avi_signature_invalid_avi() {
        let mut data = vec![0u8; 12];
        data[0..4].copy_from_slice(b"RIFF");
        data[4..8].copy_from_slice(&100u32.to_le_bytes());
        data[8..12].copy_from_slice(b"WAVE"); // Wrong format type

        let reader = TestReader::new(&data);
        let parser = AviParser;
        let result = parser.parse(&reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_avi_file_too_small() {
        let data = b"RIFF";
        let reader = TestReader::new(data);
        let parser = AviParser;
        let result = parser.parse(&reader);
        assert!(result.is_err());
    }
}
