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

/// Convenience function to parse AVI metadata from a reader.
///
/// This is a wrapper around `AviParser::parse()` to provide a simpler API
/// for the operations module.
///
/// # Arguments
///
/// * `reader` - FileReader implementation providing access to the AVI file
///
/// # Returns
///
/// * `Ok(MetadataMap)` - Successfully extracted metadata
/// * `Err(String)` - Parse error message
pub fn parse_avi_metadata(reader: &dyn FileReader) -> std::result::Result<MetadataMap, String> {
    let parser = AviParser;
    parser.parse(reader).map_err(|e| e.to_string())
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
                    match list_type as &[u8] {
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

/// Parse hdrl LIST (header list with avih chunk and stream headers)
fn parse_hdrl_list(
    reader: &dyn FileReader,
    start_offset: u64,
    end_offset: u64,
    metadata: &mut MetadataMap,
) -> Result<()> {
    let mut offset = start_offset;
    let mut stream_count = 0;

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
        // Parse strl LIST (stream list)
        else if chunk_id == b"LIST" && chunk_size >= 4 {
            let list_type = reader.read(offset, 4)?;
            if list_type == b"strl" {
                stream_count += 1;
                parse_stream_list(
                    reader,
                    offset + 4,
                    offset + chunk_size,
                    stream_count,
                    metadata,
                )?;
            }
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

/// Parse strl LIST (stream list with strh and strf chunks)
fn parse_stream_list(
    reader: &dyn FileReader,
    start_offset: u64,
    end_offset: u64,
    stream_num: usize,
    metadata: &mut MetadataMap,
) -> Result<()> {
    let mut offset = start_offset;
    let stream_prefix = format!("RIFF:Stream{}:", stream_num);
    let mut stream_type: Option<[u8; 4]> = None;

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

        match chunk_id {
            b"strh" => {
                // Parse stream header
                if chunk_size >= 56 {
                    stream_type = parse_stream_header(reader, offset, &stream_prefix, metadata)?;
                }
            }
            b"strf" => {
                // Parse stream format (depends on stream type)
                if let Some(stype) = stream_type {
                    parse_stream_format(
                        reader,
                        offset,
                        chunk_size,
                        &stype,
                        &stream_prefix,
                        metadata,
                    )?;
                }
            }
            b"strn" => {
                // Parse stream name
                if let Ok(name_bytes) = reader.read(offset, chunk_size as usize) {
                    let name = String::from_utf8_lossy(name_bytes)
                        .trim_end_matches('\0')
                        .trim()
                        .to_string();
                    if !name.is_empty() {
                        metadata.insert(
                            format!("{}StreamName", stream_prefix),
                            TagValue::new_string(name),
                        );
                    }
                }
            }
            _ => {}
        }

        // Move to next chunk
        offset += chunk_size;
        if chunk_size % 2 == 1 {
            offset += 1;
        }
    }

    Ok(())
}

/// Parse strh chunk (stream header)
fn parse_stream_header(
    reader: &dyn FileReader,
    offset: u64,
    stream_prefix: &str,
    metadata: &mut MetadataMap,
) -> Result<Option<[u8; 4]>> {
    let strh_data = reader.read(offset, 56)?;

    let stream_type = [strh_data[0], strh_data[1], strh_data[2], strh_data[3]];
    let codec_fourcc = [strh_data[4], strh_data[5], strh_data[6], strh_data[7]];
    let scale = u32::from_le_bytes([strh_data[20], strh_data[21], strh_data[22], strh_data[23]]);
    let rate = u32::from_le_bytes([strh_data[24], strh_data[25], strh_data[26], strh_data[27]]);
    let length = u32::from_le_bytes([strh_data[32], strh_data[33], strh_data[34], strh_data[35]]);

    // Stream type
    let type_str = match &stream_type {
        b"vids" => "Video",
        b"auds" => "Audio",
        b"txts" => "Text",
        b"mids" => "MIDI",
        _ => "Unknown",
    };
    metadata.insert(
        format!("{}StreamType", stream_prefix),
        TagValue::new_string(type_str.to_string()),
    );

    // Codec FourCC
    let fourcc_str = String::from_utf8_lossy(&codec_fourcc).to_string();
    if !fourcc_str.trim().is_empty() {
        metadata.insert(
            format!("{}CodecFourCC", stream_prefix),
            TagValue::new_string(fourcc_str.clone()),
        );

        // Also add as generic codec for first video/audio stream
        if stream_prefix.contains("Stream1:") {
            if stream_type == *b"vids" {
                metadata.insert(
                    "RIFF:VideoCodec".to_string(),
                    TagValue::new_string(fourcc_str),
                );
            } else if stream_type == *b"auds" {
                metadata.insert(
                    "RIFF:AudioCodec".to_string(),
                    TagValue::new_string(fourcc_str),
                );
            }
        }
    }

    // Calculate frame rate or sample rate
    if rate > 0 && scale > 0 {
        let fps = rate as f64 / scale as f64;
        if stream_type == *b"vids" {
            metadata.insert(
                format!("{}FrameRate", stream_prefix),
                TagValue::new_string(format!("{:.3}", fps)),
            );
        } else if stream_type == *b"auds" {
            metadata.insert(
                format!("{}SampleRate", stream_prefix),
                TagValue::new_string(format!("{:.0}", fps)),
            );
        }
    }

    // Stream length (in scale units)
    if length > 0 {
        metadata.insert(
            format!("{}StreamLength", stream_prefix),
            TagValue::new_integer(length as i64),
        );

        // Calculate duration
        if rate > 0 && scale > 0 {
            let duration_secs = (length as f64 * scale as f64) / rate as f64;
            metadata.insert(
                format!("{}Duration", stream_prefix),
                TagValue::new_string(format!("{:.3}", duration_secs)),
            );
        }
    }

    Ok(Some(stream_type))
}

/// Parse strf chunk (stream format - depends on stream type)
fn parse_stream_format(
    reader: &dyn FileReader,
    offset: u64,
    size: u64,
    stream_type: &[u8; 4],
    stream_prefix: &str,
    metadata: &mut MetadataMap,
) -> Result<()> {
    match stream_type {
        b"vids" => {
            // Video format (BITMAPINFOHEADER)
            if size >= 40 {
                parse_video_format(reader, offset, stream_prefix, metadata)?;
            }
        }
        b"auds" => {
            // Audio format (WAVEFORMATEX)
            if size >= 16 {
                parse_audio_format(reader, offset, stream_prefix, metadata)?;
            }
        }
        _ => {}
    }

    Ok(())
}

/// Parse video format (BITMAPINFOHEADER)
fn parse_video_format(
    reader: &dyn FileReader,
    offset: u64,
    stream_prefix: &str,
    metadata: &mut MetadataMap,
) -> Result<()> {
    let bih_data = reader.read(offset, 40)?;

    let width = u32::from_le_bytes([bih_data[4], bih_data[5], bih_data[6], bih_data[7]]);
    let height = u32::from_le_bytes([bih_data[8], bih_data[9], bih_data[10], bih_data[11]]);
    let bit_count = u16::from_le_bytes([bih_data[14], bih_data[15]]);
    let compression = [bih_data[16], bih_data[17], bih_data[18], bih_data[19]];

    metadata.insert(
        format!("{}ImageWidth", stream_prefix),
        TagValue::new_integer(width as i64),
    );
    metadata.insert(
        format!("{}ImageHeight", stream_prefix),
        TagValue::new_integer(height as i64),
    );
    metadata.insert(
        format!("{}BitDepth", stream_prefix),
        TagValue::new_integer(bit_count as i64),
    );

    // Compression FourCC
    let compression_str = String::from_utf8_lossy(&compression).to_string();
    if !compression_str.trim().is_empty() && compression != [0, 0, 0, 0] {
        metadata.insert(
            format!("{}Compression", stream_prefix),
            TagValue::new_string(compression_str),
        );
    }

    Ok(())
}

/// Parse audio format (WAVEFORMATEX)
fn parse_audio_format(
    reader: &dyn FileReader,
    offset: u64,
    stream_prefix: &str,
    metadata: &mut MetadataMap,
) -> Result<()> {
    let wfx_data = reader.read(offset, 16)?;

    let format_tag = u16::from_le_bytes([wfx_data[0], wfx_data[1]]);
    let channels = u16::from_le_bytes([wfx_data[2], wfx_data[3]]);
    let samples_per_sec = u32::from_le_bytes([wfx_data[4], wfx_data[5], wfx_data[6], wfx_data[7]]);
    let avg_bytes_per_sec =
        u32::from_le_bytes([wfx_data[8], wfx_data[9], wfx_data[10], wfx_data[11]]);
    let bits_per_sample = u16::from_le_bytes([wfx_data[14], wfx_data[15]]);

    // Audio format tag
    let format_name = match format_tag {
        0x0001 => "PCM",
        0x0002 => "ADPCM",
        0x0003 => "IEEE Float",
        0x0006 => "A-Law",
        0x0007 => "Mu-Law",
        0x0011 => "IMA ADPCM",
        0x0016 => "G.723 ADPCM",
        0x0031 => "GSM 6.10",
        0x0050 => "MPEG",
        0x0055 => "MP3",
        0x0161 => "WMA v1",
        0x0162 => "WMA v2",
        _ => "Unknown",
    };
    metadata.insert(
        format!("{}AudioFormat", stream_prefix),
        TagValue::new_string(format_name.to_string()),
    );
    metadata.insert(
        format!("{}AudioFormatTag", stream_prefix),
        TagValue::new_integer(format_tag as i64),
    );

    metadata.insert(
        format!("{}NumChannels", stream_prefix),
        TagValue::new_integer(channels as i64),
    );
    metadata.insert(
        format!("{}SampleRate", stream_prefix),
        TagValue::new_integer(samples_per_sec as i64),
    );
    metadata.insert(
        format!("{}AvgBytesPerSec", stream_prefix),
        TagValue::new_integer(avg_bytes_per_sec as i64),
    );

    if bits_per_sample > 0 {
        metadata.insert(
            format!("{}BitsPerSample", stream_prefix),
            TagValue::new_integer(bits_per_sample as i64),
        );
    }

    // Calculate bitrate
    let bitrate_kbps = (avg_bytes_per_sec * 8) / 1000;
    if bitrate_kbps > 0 {
        metadata.insert(
            format!("{}Bitrate", stream_prefix),
            TagValue::new_string(format!("{} kbps", bitrate_kbps)),
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestReader;

    #[test]
    fn test_avi_signature_valid() {
        // Minimal AVI file structure
        let mut data = vec![0u8; 100];
        data[0..4].copy_from_slice(b"RIFF");
        data[4..8].copy_from_slice(&100u32.to_le_bytes());
        data[8..12].copy_from_slice(b"AVI ");

        let reader = TestReader::from_slice(&data);
        let parser = AviParser;
        let result = parser.parse(&reader);
        assert!(result.is_ok());
    }

    #[test]
    fn test_avi_signature_invalid_riff() {
        let data = b"INVALID DATA";
        let reader = TestReader::from_slice(data);
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

        let reader = TestReader::from_slice(&data);
        let parser = AviParser;
        let result = parser.parse(&reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_avi_file_too_small() {
        let data = b"RIFF";
        let reader = TestReader::from_slice(data);
        let parser = AviParser;
        let result = parser.parse(&reader);
        assert!(result.is_err());
    }
}
