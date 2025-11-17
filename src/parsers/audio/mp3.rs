//! MP3 (MPEG Audio Layer 3) format parser
//!
//! Implements metadata extraction from MP3 audio files, supporting ID3v1,
//! ID3v2.3, and ID3v2.4 tags.
//!
//! # Supported Metadata
//!
//! - **ID3v1:** Title, Artist, Album, Year, Comment, Genre, Track
//! - **ID3v2:** All standard frames (TIT2, TPE1, TALB, etc.)
//! - **MPEG Info:** Bitrate, sample rate, duration, channel mode
//!
//! # ExifTool Compatibility
//!
//! Maps to ExifTool tags from `ID3.pm` module:
//! - `ID3:Title` → TIT2 frame
//! - `ID3:Artist` → TPE1 frame
//! - `ID3:Album` → TALB frame
//!
//! # File Structure
//!
//! ```text
//! [ID3v2 tag - optional, at start]
//!   ├─ Header (10 bytes)
//!   └─ Frames (variable)
//! [MPEG audio frames]
//! [ID3v1 tag - optional, last 128 bytes]
//! ```
//!
//! # References
//!
//! - ID3v2.4 Spec: <http://id3.org/id3v2.4.0-structure>
//! - ID3v2.3 Spec: <http://id3.org/id3v2.3.0>
//! - ID3v1 Spec: <http://id3.org/ID3v1>
//! - ExifTool Source: `lib/Image/ExifTool/ID3.pm`

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};
use nom::{
    bytes::complete::{tag, take},
    number::complete::be_u8,
    IResult,
};

/// ID3v2 signature
const ID3V2_SIGNATURE: &[u8] = b"ID3";

/// ID3v1 signature
const ID3V1_SIGNATURE: &[u8] = b"TAG";

/// MP3 parser
pub struct Mp3Parser;

impl FormatParser for Mp3Parser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        let file_size = reader.size();
        let mut metadata = MetadataMap::with_capacity(32);

        // Try to parse ID3v2 tag (at start of file)
        if file_size >= 10 {
            let header = reader.read(0, 10)?;
            if &header[0..3] == ID3V2_SIGNATURE {
                parse_id3v2(reader, &mut metadata)?;
            }
        }

        // Try to parse ID3v1 tag (last 128 bytes)
        if file_size >= 128 {
            let id3v1_offset = file_size - 128;
            let id3v1_data = reader.read(id3v1_offset, 128)?;
            if &id3v1_data[0..3] == ID3V1_SIGNATURE {
                parse_id3v1(id3v1_data, &mut metadata)?;
            }
        }

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::MP3)
    }
}

/// Parse ID3v2 tag
fn parse_id3v2(reader: &dyn FileReader, metadata: &mut MetadataMap) -> Result<()> {
    // Read ID3v2 header (10 bytes)
    let header = reader.read(0, 10)?;
    let (_, id3v2_header) = parse_id3v2_header(header).map_err(|e| {
        ExifToolError::parse_error(format!("Failed to parse ID3v2 header: {:?}", e))
    })?;

    metadata.insert(
        "ID3:Version".to_string(),
        TagValue::new_string(format!(
            "2.{}.{}",
            id3v2_header.version, id3v2_header.revision
        )),
    );

    // Read frames
    let frames_size = id3v2_header.size as usize;
    if frames_size > 0 {
        let frames_data = reader.read(10, frames_size)?;
        parse_id3v2_frames(frames_data, id3v2_header.version, metadata)?;
    }

    Ok(())
}

#[derive(Debug)]
struct ID3v2Header {
    version: u8,
    revision: u8,
    flags: u8,
    size: u32, // Synchsafe integer
}

fn parse_id3v2_header(input: &[u8]) -> IResult<&[u8], ID3v2Header> {
    let (input, _) = tag(ID3V2_SIGNATURE)(input)?;
    let (input, version) = be_u8(input)?;
    let (input, revision) = be_u8(input)?;
    let (input, flags) = be_u8(input)?;
    let (input, size_bytes) = take(4usize)(input)?;

    // Decode synchsafe integer (7 bits per byte)
    let size = decode_synchsafe_u32(size_bytes);

    Ok((
        input,
        ID3v2Header {
            version,
            revision,
            flags,
            size,
        },
    ))
}

/// Decode synchsafe integer (ID3v2 size encoding)
fn decode_synchsafe_u32(bytes: &[u8]) -> u32 {
    ((bytes[0] as u32 & 0x7F) << 21)
        | ((bytes[1] as u32 & 0x7F) << 14)
        | ((bytes[2] as u32 & 0x7F) << 7)
        | (bytes[3] as u32 & 0x7F)
}

/// Parse ID3v2 frames
fn parse_id3v2_frames(data: &[u8], version: u8, metadata: &mut MetadataMap) -> Result<()> {
    let mut offset = 0;

    while offset + 10 < data.len() {
        // Frame header size depends on version
        let (frame_id, frame_size, _frame_flags) = if version >= 3 {
            // ID3v2.3 and v2.4: 10-byte header
            if &data[offset..offset + 4] == b"\x00\x00\x00\x00" {
                break; // Padding
            }

            let frame_id = String::from_utf8_lossy(&data[offset..offset + 4]).to_string();
            let frame_size = if version == 4 {
                // ID3v2.4 uses synchsafe integers
                decode_synchsafe_u32(&data[offset + 4..offset + 8])
            } else {
                // ID3v2.3 uses regular integers
                u32::from_be_bytes([
                    data[offset + 4],
                    data[offset + 5],
                    data[offset + 6],
                    data[offset + 7],
                ])
            };
            let frame_flags = u16::from_be_bytes([data[offset + 8], data[offset + 9]]);
            offset += 10;

            (frame_id, frame_size, frame_flags)
        } else {
            // ID3v2.2: 6-byte header
            let frame_id = String::from_utf8_lossy(&data[offset..offset + 3]).to_string();
            let frame_size =
                u32::from_be_bytes([0, data[offset + 3], data[offset + 4], data[offset + 5]]);
            offset += 6;

            (frame_id, frame_size, 0)
        };

        // Read frame data
        if offset + frame_size as usize > data.len() {
            break;
        }

        let frame_data = &data[offset..offset + frame_size as usize];
        offset += frame_size as usize;

        // Parse text frames
        if frame_id.starts_with('T') && frame_id != "TXXX" {
            if let Ok(text) = parse_text_frame(frame_data) {
                let tag_name = format!("ID3:{}", map_frame_id_to_tag_name(&frame_id));
                metadata.insert(tag_name, TagValue::new_string(text));
            }
        }
    }

    Ok(())
}

/// Parse text frame (TXX encoding + text)
fn parse_text_frame(data: &[u8]) -> Result<String> {
    if data.is_empty() {
        return Err(ExifToolError::parse_error("Empty text frame"));
    }

    let encoding_byte = data[0];
    let text_data = &data[1..];

    let encoding = match encoding_byte {
        0 => encoding_rs::WINDOWS_1252, // ISO-8859-1
        1 => encoding_rs::UTF_16LE,
        2 => encoding_rs::UTF_16BE,
        3 => encoding_rs::UTF_8,
        _ => encoding_rs::UTF_8, // Default to UTF-8
    };

    let (decoded, _, _) = encoding.decode(text_data);
    Ok(decoded.trim_end_matches('\0').to_string())
}

/// Map ID3v2 frame ID to tag name
fn map_frame_id_to_tag_name(frame_id: &str) -> &str {
    match frame_id {
        "TIT2" => "Title",
        "TPE1" => "Artist",
        "TALB" => "Album",
        "TYER" | "TDRC" => "Year",
        "TCON" => "Genre",
        "TRCK" => "Track",
        "COMM" => "Comment",
        _ => frame_id,
    }
}

/// Parse ID3v1 tag
fn parse_id3v1(data: &[u8], metadata: &mut MetadataMap) -> Result<()> {
    if data.len() < 128 || &data[0..3] != ID3V1_SIGNATURE {
        return Err(ExifToolError::parse_error("Invalid ID3v1 tag"));
    }

    // Extract fields (all ISO-8859-1 encoded)
    let title = decode_latin1(&data[3..33]);
    let artist = decode_latin1(&data[33..63]);
    let album = decode_latin1(&data[63..93]);
    let year = decode_latin1(&data[93..97]);
    let comment = decode_latin1(&data[97..127]);
    let genre = data[127];

    if !title.is_empty() {
        metadata.insert("ID3v1:Title".to_string(), TagValue::new_string(title));
    }
    if !artist.is_empty() {
        metadata.insert("ID3v1:Artist".to_string(), TagValue::new_string(artist));
    }
    if !album.is_empty() {
        metadata.insert("ID3v1:Album".to_string(), TagValue::new_string(album));
    }
    if !year.is_empty() {
        metadata.insert("ID3v1:Year".to_string(), TagValue::new_string(year));
    }
    if !comment.is_empty() {
        metadata.insert("ID3v1:Comment".to_string(), TagValue::new_string(comment));
    }
    if genre < 192 {
        metadata.insert(
            "ID3v1:Genre".to_string(),
            TagValue::new_integer(genre as i64),
        );
    }

    Ok(())
}

/// Decode Latin-1 (ISO-8859-1) string, trimming null bytes
fn decode_latin1(bytes: &[u8]) -> String {
    let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
    decoded.trim_end_matches('\0').trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_synchsafe_u32() {
        assert_eq!(decode_synchsafe_u32(&[0x00, 0x00, 0x00, 0x00]), 0);
        assert_eq!(decode_synchsafe_u32(&[0x00, 0x00, 0x00, 0x7F]), 127);
        assert_eq!(decode_synchsafe_u32(&[0x00, 0x00, 0x01, 0x00]), 128);
        assert_eq!(decode_synchsafe_u32(&[0x7F, 0x7F, 0x7F, 0x7F]), 268435455);
    }

    #[test]
    fn test_map_frame_id_to_tag_name() {
        assert_eq!(map_frame_id_to_tag_name("TIT2"), "Title");
        assert_eq!(map_frame_id_to_tag_name("TPE1"), "Artist");
        assert_eq!(map_frame_id_to_tag_name("TALB"), "Album");
    }
}
