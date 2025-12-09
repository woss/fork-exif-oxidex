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
use crate::io::EndianReader;
use encoding_rs::UTF_8;

/// OGG page signature
const OGG_SIGNATURE: &[u8] = b"OggS";

/// Vorbis identification header packet type
const VORBIS_ID_HEADER: u8 = 0x01;

/// Vorbis comment header packet type
const VORBIS_COMMENT_HEADER: u8 = 0x03;

/// OGG parser
pub struct OggParser;

/// Parses metadata from an Ogg Vorbis file.
///
/// This is a convenience wrapper that creates an OggParser instance and calls parse().
///
/// # Arguments
///
/// * `reader` - File reader providing access to the Ogg file data
///
/// # Returns
///
/// * `Ok(MetadataMap)` - Successfully extracted metadata
/// * `Err(String)` - Parse error message
pub fn parse_ogg_metadata(reader: &dyn FileReader) -> std::result::Result<MetadataMap, String> {
    let parser = OggParser;
    parser.parse(reader).map_err(|e| e.to_string())
}

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
            let _header_type = page_header[5];
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
                if page_body.len() >= 7
                    && &page_body[1..7] == b"vorbis"
                    && page_body[0] == VORBIS_COMMENT_HEADER
                {
                    // Parse Vorbis comments
                    parse_vorbis_comments(&page_body[7..], &mut metadata)?;
                    break; // Found comments, we're done
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

/// Maps Vorbis comment field names to ExifTool-compatible tag names
fn map_vorbis_field_name(field_name: &str) -> String {
    // Normalize field name to uppercase for matching
    let upper = field_name.to_uppercase();

    match upper.as_str() {
        // Core Vorbis tags
        "TITLE" => "Vorbis:Title".to_string(),
        "ARTIST" => "Vorbis:Artist".to_string(),
        "ALBUM" => "Vorbis:Album".to_string(),
        "TRACKNUMBER" => "Vorbis:TrackNumber".to_string(),
        "DATE" => "Vorbis:Date".to_string(),
        "GENRE" => "Vorbis:Genre".to_string(),
        "COMMENT" => "Vorbis:Comment".to_string(),
        "DESCRIPTION" => "Vorbis:Description".to_string(),
        "COPYRIGHT" => "Vorbis:Copyright".to_string(),
        "LICENSE" => "Vorbis:License".to_string(),
        "ORGANIZATION" => "Vorbis:Organization".to_string(),
        "PERFORMER" => "Vorbis:Performer".to_string(),
        "COMPOSER" => "Vorbis:Composer".to_string(),
        "CONDUCTOR" => "Vorbis:Conductor".to_string(),
        "ISRC" => "Vorbis:ISRC".to_string(),
        "LYRICS" => "Vorbis:Lyrics".to_string(),
        "ALBUMARTIST" => "Vorbis:AlbumArtist".to_string(),
        "DISCNUMBER" => "Vorbis:DiscNumber".to_string(),
        "TOTALTRACKS" => "Vorbis:TotalTracks".to_string(),
        "TOTALDISCS" => "Vorbis:TotalDiscs".to_string(),
        "ENCODER" => "Vorbis:Encoder".to_string(),
        "ENCODEDBY" | "ENCODED_BY" => "Vorbis:EncodedBy".to_string(),
        "CONTACT" => "Vorbis:Contact".to_string(),
        "LOCATION" => "Vorbis:Location".to_string(),
        "VERSION" => "Vorbis:Version".to_string(),
        // ReplayGain tags
        "REPLAYGAIN_TRACK_GAIN" => "Vorbis:ReplayGainTrackGain".to_string(),
        "REPLAYGAIN_TRACK_PEAK" => "Vorbis:ReplayGainTrackPeak".to_string(),
        "REPLAYGAIN_ALBUM_GAIN" => "Vorbis:ReplayGainAlbumGain".to_string(),
        "REPLAYGAIN_ALBUM_PEAK" => "Vorbis:ReplayGainAlbumPeak".to_string(),
        // Cover art
        "COVERART" => "Vorbis:CoverArt".to_string(),
        "COVERARTMIME" => "Vorbis:CoverArtMIMEType".to_string(),
        // Unknown - use raw name with Vorbis prefix
        _ => format!("Vorbis:{}", field_name),
    }
}

/// Parse Vorbis comment data
fn parse_vorbis_comments(data: &[u8], metadata: &mut MetadataMap) -> Result<()> {
    let mut offset = 0;
    let reader = EndianReader::little_endian(data);

    // Vendor string length (4 bytes, little-endian)
    if data.len() < 4 {
        return Err(ExifToolError::parse_error("Vorbis comment block too small"));
    }

    let vendor_length = reader.u32_at(offset).unwrap_or(0) as usize;
    offset += 4;

    // Read and store vendor string
    if offset + vendor_length > data.len() {
        return Err(ExifToolError::parse_error("Invalid vendor string length"));
    }
    let vendor_bytes = &data[offset..offset + vendor_length];
    let (vendor_str, _, _) = UTF_8.decode(vendor_bytes);
    if !vendor_str.is_empty() {
        metadata.insert(
            "Vorbis:Vendor".to_string(),
            TagValue::new_string(vendor_str.to_string()),
        );
    }
    offset += vendor_length;

    // User comment list length (4 bytes, little-endian)
    if offset + 4 > data.len() {
        return Err(ExifToolError::parse_error("Missing comment list length"));
    }

    let comment_count = reader.u32_at(offset).unwrap_or(0);
    offset += 4;

    // Parse each comment
    for _ in 0..comment_count {
        if offset + 4 > data.len() {
            break;
        }

        // Comment length (4 bytes, little-endian)
        let comment_length = reader.u32_at(offset).unwrap_or(0) as usize;
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

            // Map to ExifTool-compatible tag name
            let tag_name = map_vorbis_field_name(field_name);
            metadata.insert(tag_name, TagValue::new_string(field_value.to_string()));
        }

        offset += comment_length;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestReader;

    #[test]
    fn test_ogg_signature_valid() {
        // Minimal OGG page header
        let mut data = vec![0u8; 100];
        data[0..4].copy_from_slice(b"OggS");
        data[5] = 0x00; // header_type (unused but present)
        data[26] = 0; // segment count

        let reader = TestReader::from_slice(&data);
        let parser = OggParser;
        let result = parser.parse(&reader);
        assert!(result.is_ok());
    }

    #[test]
    fn test_ogg_signature_invalid() {
        let data = b"INVALID DATA";
        let reader = TestReader::from_slice(data);
        let parser = OggParser;
        let result = parser.parse(&reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_ogg_file_too_small() {
        let data = b"Ogg";
        let reader = TestReader::from_slice(data);
        let parser = OggParser;
        let result = parser.parse(&reader);
        assert!(result.is_err());
    }
}
