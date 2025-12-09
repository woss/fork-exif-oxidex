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
use crate::io::EndianReader;
use nom::{
    IResult,
    bytes::complete::{tag, take},
    number::complete::be_u8,
};

/// ID3v2 signature
const ID3V2_SIGNATURE: &[u8] = b"ID3";

/// ID3v1 signature
const ID3V1_SIGNATURE: &[u8] = b"TAG";

/// MP3 parser
pub struct Mp3Parser;

/// Parses metadata from an MP3 file.
///
/// This is a convenience wrapper that creates an Mp3Parser instance and calls parse().
///
/// # Arguments
///
/// * `reader` - File reader providing access to the MP3 file data
///
/// # Returns
///
/// * `Ok(MetadataMap)` - Successfully extracted metadata
/// * `Err(String)` - Parse error message
pub fn parse_mp3_metadata(reader: &dyn FileReader) -> std::result::Result<MetadataMap, String> {
    let parser = Mp3Parser;
    parser.parse(reader).map_err(|e| e.to_string())
}

impl FormatParser for Mp3Parser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        let file_size = reader.size();
        let mut metadata = MetadataMap::with_capacity(32);
        let mut audio_start = 0u64;

        // Try to parse ID3v2 tag (at start of file)
        if file_size >= 10 {
            let header = reader.read(0, 10)?;
            if &header[0..3] == ID3V2_SIGNATURE {
                let id3v2_size = parse_id3v2(reader, &mut metadata)?;
                audio_start = 10 + id3v2_size as u64;
            }
        }

        // Parse MPEG audio frame header
        if audio_start < file_size {
            parse_mpeg_audio_frame(reader, audio_start, &mut metadata)?;
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

/// Parse ID3v2 tag, returns the total size of the ID3v2 tag data
fn parse_id3v2(reader: &dyn FileReader, metadata: &mut MetadataMap) -> Result<u32> {
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

    Ok(id3v2_header.size)
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
    let reader = EndianReader::big_endian(data);

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
                reader.u32_at(offset + 4).unwrap_or(0)
            };
            let frame_flags = reader.u16_at(offset + 8).unwrap_or(0);
            offset += 10;

            (frame_id, frame_size, frame_flags)
        } else {
            // ID3v2.2: 6-byte header (3-byte size, big-endian)
            let frame_id = String::from_utf8_lossy(&data[offset..offset + 3]).to_string();
            // ID3v2.2 uses 3 bytes for size, need to handle specially
            let frame_size = ((data[offset + 3] as u32) << 16)
                | ((data[offset + 4] as u32) << 8)
                | (data[offset + 5] as u32);
            offset += 6;

            (frame_id, frame_size, 0)
        };

        // Read frame data
        if offset + frame_size as usize > data.len() {
            break;
        }

        let frame_data = &data[offset..offset + frame_size as usize];
        offset += frame_size as usize;

        // Parse text frames (T* frames except TXXX/TXX which are user-defined)
        let is_text_frame = frame_id.starts_with('T')
            && frame_id != "TXXX"
            && frame_id != "TXX";

        // Parse comment frames (COM/COMM)
        let is_comment_frame = frame_id == "COM" || frame_id == "COMM";

        // Parse lyrics frames (ULT/USLT)
        let is_lyrics_frame = frame_id == "ULT" || frame_id == "USLT";

        // Parse picture frames (PIC/APIC)
        let is_picture_frame = frame_id == "PIC" || frame_id == "APIC";

        if is_text_frame
            && let Ok(text) = parse_text_frame(frame_data)
        {
            let tag_name = format!("ID3:{}", map_frame_id_to_tag_name(&frame_id));
            metadata.insert(tag_name, TagValue::new_string(text));
        } else if is_comment_frame
            && let Ok(text) = parse_comment_frame(frame_data)
        {
            metadata.insert("ID3:Comment".to_string(), TagValue::new_string(text));
        } else if is_lyrics_frame
            && let Ok(text) = parse_comment_frame(frame_data)
        {
            // Lyrics frame has same structure as comment frame
            metadata.insert("ID3:Lyrics".to_string(), TagValue::new_string(text));
        } else if is_picture_frame {
            let _ = parse_picture_frame(frame_data, version, metadata);
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

/// Parse comment frame (COM/COMM - encoding + language + short desc + text)
fn parse_comment_frame(data: &[u8]) -> Result<String> {
    if data.len() < 5 {
        return Err(ExifToolError::parse_error("Comment frame too short"));
    }

    let encoding_byte = data[0];
    // Skip language (3 bytes) and find short description null terminator
    let text_start = &data[4..];

    let encoding = match encoding_byte {
        0 => encoding_rs::WINDOWS_1252,
        1 => encoding_rs::UTF_16LE,
        2 => encoding_rs::UTF_16BE,
        3 => encoding_rs::UTF_8,
        _ => encoding_rs::UTF_8,
    };

    // For UTF-16 encodings, find double-null terminator
    let content_start = if encoding_byte == 1 || encoding_byte == 2 {
        // UTF-16: look for double null terminator
        let mut pos = 0;
        while pos + 1 < text_start.len() {
            if text_start[pos] == 0 && text_start[pos + 1] == 0 {
                pos += 2;
                break;
            }
            pos += 2;
        }
        pos
    } else {
        // Latin-1 or UTF-8: look for single null
        text_start.iter().position(|&b| b == 0).map_or(0, |p| p + 1)
    };

    let comment_data = &text_start[content_start..];
    let (decoded, _, _) = encoding.decode(comment_data);
    Ok(decoded.trim_end_matches('\0').to_string())
}

/// Map ID3v2 frame ID to tag name
/// Supports both ID3v2.2 (3-char) and ID3v2.3/v2.4 (4-char) frame IDs
fn map_frame_id_to_tag_name(frame_id: &str) -> &str {
    match frame_id {
        // ID3v2.3/v2.4 frame IDs (4 characters)
        "TIT1" => "Grouping",
        "TIT2" => "Title",
        "TIT3" => "Subtitle",
        "TPE1" => "Artist",
        "TPE2" => "Band",
        "TPE3" => "Conductor",
        "TPE4" => "Remixer",
        "TALB" => "Album",
        "TYER" => "Year",
        "TDRC" => "Year",
        "TDAT" => "Date",
        "TCON" => "Genre",
        "TRCK" => "Track",
        "TPOS" => "PartOfSet",
        "COMM" => "Comment",
        "TCOM" => "Composer",
        "TPUB" => "Publisher",
        "TCOP" => "Copyright",
        "TENC" => "EncodedBy",
        "TSSE" => "EncoderSettings",
        "TBPM" => "BeatsPerMinute",
        "TKEY" => "InitialKey",
        "TLAN" => "Language",
        "TLEN" => "Length",
        "TMED" => "OriginalMedia",
        "TOAL" => "OriginalAlbum",
        "TOFN" => "OriginalFilename",
        "TOLY" => "OriginalLyricist",
        "TOPE" => "OriginalArtist",
        "TORY" => "OriginalYear",
        "TEXT" => "Lyricist",
        "USLT" => "Lyrics",
        "WCOM" => "CommercialURL",
        "WCOP" => "CopyrightURL",
        "WOAF" => "FileURL",
        "WOAR" => "ArtistURL",
        "WOAS" => "SourceURL",
        "WORS" => "StationURL",
        "WPAY" => "PaymentURL",
        "WPUB" => "PublisherURL",

        // ID3v2.2 frame IDs (3 characters)
        "TT1" => "Grouping",
        "TT2" => "Title",
        "TT3" => "Subtitle",
        "TP1" => "Artist",
        "TP2" => "Band",
        "TP3" => "Conductor",
        "TP4" => "Remixer",
        "TAL" => "Album",
        "TYE" => "Year",
        "TDA" => "Date",
        "TCO" => "Genre",
        "TRK" => "Track",
        "TPA" => "PartOfSet",
        "COM" => "Comment",
        "TCM" => "Composer",
        "TPB" => "Publisher",
        "TCR" => "Copyright",
        "TEN" => "EncodedBy",
        "TSS" => "EncoderSettings",
        "TBP" => "BeatsPerMinute",
        "TKE" => "InitialKey",
        "TLA" => "Language",
        "TLE" => "Length",
        "TMT" => "OriginalMedia",
        "TOT" => "OriginalAlbum",
        "TOF" => "OriginalFilename",
        "TOL" => "OriginalLyricist",
        "TOA" => "OriginalArtist",
        "TOR" => "OriginalYear",
        "TXT" => "Lyricist",
        "ULT" => "Lyrics",
        "WCM" => "CommercialURL",
        "WCP" => "CopyrightURL",
        "WAF" => "FileURL",
        "WAR" => "ArtistURL",
        "WAS" => "SourceURL",
        "WPB" => "PublisherURL",

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

/// Parse MPEG audio frame header to extract audio properties
fn parse_mpeg_audio_frame(
    reader: &dyn FileReader,
    start_offset: u64,
    metadata: &mut MetadataMap,
) -> Result<()> {
    // Search for MPEG frame sync (11 bits set = 0xFF followed by 0xE* or 0xF*)
    // We need to scan a bit in case there's padding or junk after ID3 tag
    let max_search = 4096u64.min(reader.size().saturating_sub(start_offset));
    let search_data = reader.read(start_offset, max_search as usize)?;

    for i in 0..search_data.len().saturating_sub(4) {
        // Check for frame sync: 11 bits of 1s
        if search_data[i] == 0xFF && (search_data[i + 1] & 0xE0) == 0xE0 {
            let header = &search_data[i..i + 4];

            // Parse MPEG audio header bits
            // Byte 1: AAAAAAAA (8 sync bits)
            // Byte 2: AAABBCCD (3 sync, 2 version, 2 layer, 1 protection)
            // Byte 3: EEEEFFGH (4 bitrate, 2 sample rate, 1 padding, 1 private)
            // Byte 4: IIJJKLMM (2 channel mode, 2 mode ext, 1 copyright, 1 original, 2 emphasis)

            let version_bits = (header[1] >> 3) & 0x03;
            let layer_bits = (header[1] >> 1) & 0x03;
            let protection_bit = header[1] & 0x01;
            let bitrate_index = (header[2] >> 4) & 0x0F;
            let sample_rate_index = (header[2] >> 2) & 0x03;
            let channel_mode = (header[3] >> 6) & 0x03;
            let mode_extension = (header[3] >> 4) & 0x03;
            let copyright_bit = (header[3] >> 3) & 0x01;
            let original_bit = (header[3] >> 2) & 0x01;
            let emphasis_bits = header[3] & 0x03;

            // Validate: version 01 is reserved
            if version_bits == 0x01 {
                continue;
            }
            // Validate: layer 00 is reserved
            if layer_bits == 0x00 {
                continue;
            }
            // Validate: bitrate index 15 (0xF) is invalid
            if bitrate_index == 0x0F {
                continue;
            }
            // Validate: sample rate index 3 is reserved
            if sample_rate_index == 0x03 {
                continue;
            }

            // MPEG Audio Version
            let mpeg_version = match version_bits {
                0x00 => 2.5, // MPEG 2.5
                0x02 => 2.0, // MPEG 2
                0x03 => 1.0, // MPEG 1
                _ => continue,
            };
            // ExifTool reports version as integer (1 for MPEG 1, 2 for MPEG 2/2.5)
            let version_int = if mpeg_version == 1.0 {
                1
            } else {
                2
            };
            metadata.insert(
                "MPEG:MPEGAudioVersion".to_string(),
                TagValue::new_integer(version_int),
            );

            // Audio Layer
            let layer = match layer_bits {
                0x01 => 3, // Layer III
                0x02 => 2, // Layer II
                0x03 => 1, // Layer I
                _ => continue,
            };
            metadata.insert("MPEG:AudioLayer".to_string(), TagValue::new_integer(layer));

            // Protection (CRC)
            let _crc_protected = protection_bit == 0;

            // Bitrate (kbps)
            let bitrate = get_mpeg_bitrate(mpeg_version, layer, bitrate_index);
            if bitrate > 0 {
                metadata.insert(
                    "MPEG:AudioBitrate".to_string(),
                    TagValue::new_string(format!("{} kbps", bitrate)),
                );
            }

            // Sample Rate (Hz)
            let sample_rate = get_mpeg_sample_rate(mpeg_version, sample_rate_index);
            if sample_rate > 0 {
                metadata.insert(
                    "MPEG:SampleRate".to_string(),
                    TagValue::new_integer(sample_rate as i64),
                );
            }

            // Channel Mode
            let channel_mode_str = match channel_mode {
                0x00 => "Stereo",
                0x01 => "Joint Stereo",
                0x02 => "Dual Channel",
                0x03 => "Mono",
                _ => "Unknown",
            };
            metadata.insert(
                "MPEG:ChannelMode".to_string(),
                TagValue::new_string(channel_mode_str),
            );

            // Mode Extension (only meaningful for Joint Stereo)
            if channel_mode == 0x01 {
                // For Layer III joint stereo
                if layer == 3 {
                    let ms_stereo = (mode_extension & 0x02) != 0;
                    let intensity_stereo = (mode_extension & 0x01) != 0;
                    metadata.insert(
                        "MPEG:MSStereo".to_string(),
                        TagValue::new_string(if ms_stereo { "On" } else { "Off" }),
                    );
                    metadata.insert(
                        "MPEG:IntensityStereo".to_string(),
                        TagValue::new_string(if intensity_stereo { "On" } else { "Off" }),
                    );
                }
            }

            // Copyright
            metadata.insert(
                "MPEG:CopyrightFlag".to_string(),
                TagValue::new_string(if copyright_bit != 0 { "true" } else { "false" }),
            );

            // Original
            metadata.insert(
                "MPEG:OriginalMedia".to_string(),
                TagValue::new_string(if original_bit != 0 { "true" } else { "false" }),
            );

            // Emphasis
            let emphasis_str = match emphasis_bits {
                0x00 => "None",
                0x01 => "50/15 ms",
                0x02 => "Reserved",
                0x03 => "CCIT J.17",
                _ => "Unknown",
            };
            metadata.insert(
                "MPEG:Emphasis".to_string(),
                TagValue::new_string(emphasis_str),
            );

            // Found valid frame, stop searching
            break;
        }
    }

    Ok(())
}

/// Get MPEG bitrate in kbps based on version, layer, and bitrate index
fn get_mpeg_bitrate(version: f64, layer: i64, index: u8) -> u16 {
    // Bitrate tables (kbps)
    // MPEG 1, Layer I
    const V1_L1: [u16; 16] = [
        0, 32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448, 0,
    ];
    // MPEG 1, Layer II
    const V1_L2: [u16; 16] = [
        0, 32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384, 0,
    ];
    // MPEG 1, Layer III
    const V1_L3: [u16; 16] = [
        0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0,
    ];
    // MPEG 2/2.5, Layer I
    const V2_L1: [u16; 16] = [
        0, 32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256, 0,
    ];
    // MPEG 2/2.5, Layer II/III
    const V2_L23: [u16; 16] = [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0];

    let idx = index as usize;
    if idx >= 16 {
        return 0;
    }

    if version >= 1.0 && version < 2.0 {
        // MPEG 1
        match layer {
            1 => V1_L1[idx],
            2 => V1_L2[idx],
            3 => V1_L3[idx],
            _ => 0,
        }
    } else {
        // MPEG 2 or 2.5
        match layer {
            1 => V2_L1[idx],
            2 | 3 => V2_L23[idx],
            _ => 0,
        }
    }
}

/// Get MPEG sample rate in Hz based on version and sample rate index
fn get_mpeg_sample_rate(version: f64, index: u8) -> u32 {
    // Sample rate tables (Hz)
    const V1_RATES: [u32; 4] = [44100, 48000, 32000, 0];
    const V2_RATES: [u32; 4] = [22050, 24000, 16000, 0];
    const V25_RATES: [u32; 4] = [11025, 12000, 8000, 0];

    let idx = index as usize;
    if idx >= 4 {
        return 0;
    }

    if version >= 1.0 && version < 2.0 {
        V1_RATES[idx]
    } else if version >= 2.0 && version < 2.5 {
        V2_RATES[idx]
    } else {
        V25_RATES[idx]
    }
}

/// Parse APIC/PIC (picture) frame
fn parse_picture_frame(data: &[u8], version: u8, metadata: &mut MetadataMap) -> Result<()> {
    if data.is_empty() {
        return Ok(());
    }

    let encoding = data[0];
    let mut pos = 1;

    // MIME type
    let mime_type = if version >= 3 {
        // ID3v2.3/v2.4: null-terminated string
        let end = data[pos..].iter().position(|&b| b == 0).unwrap_or(data.len() - pos);
        let mime = String::from_utf8_lossy(&data[pos..pos + end]).to_string();
        pos += end + 1; // Skip null terminator
        mime
    } else {
        // ID3v2.2: 3-character image format (e.g., "JPG", "PNG")
        if pos + 3 > data.len() {
            return Ok(());
        }
        let fmt = String::from_utf8_lossy(&data[pos..pos + 3]).to_string();
        pos += 3;
        fmt
    };

    // Picture type
    if pos >= data.len() {
        return Ok(());
    }
    let picture_type = data[pos];
    pos += 1;

    let picture_type_str = match picture_type {
        0 => "Other",
        1 => "32x32 PNG Icon",
        2 => "Other Icon",
        3 => "Front Cover",
        4 => "Back Cover",
        5 => "Leaflet",
        6 => "Media",
        7 => "Lead Artist",
        8 => "Artist",
        9 => "Conductor",
        10 => "Band",
        11 => "Composer",
        12 => "Lyricist",
        13 => "Recording Location",
        14 => "During Recording",
        15 => "During Performance",
        16 => "Video Capture",
        17 => "A Bright Coloured Fish",
        18 => "Illustration",
        19 => "Band Logotype",
        20 => "Publisher Logotype",
        _ => "Other",
    };

    // Description (null-terminated, encoding-dependent)
    let description_end = if encoding == 1 || encoding == 2 {
        // UTF-16: look for double null
        let mut i = pos;
        while i + 1 < data.len() {
            if data[i] == 0 && data[i + 1] == 0 {
                break;
            }
            i += 2;
        }
        i
    } else {
        data[pos..].iter().position(|&b| b == 0).map_or(data.len(), |p| pos + p)
    };

    let description_bytes = &data[pos..description_end];
    let description = match encoding {
        0 => String::from_utf8_lossy(description_bytes).to_string(),
        1 | 2 => {
            let mut chars: Vec<u16> = Vec::new();
            for chunk in description_bytes.chunks(2) {
                if chunk.len() == 2 {
                    let c = if encoding == 1 {
                        ((chunk[1] as u16) << 8) | (chunk[0] as u16)
                    } else {
                        ((chunk[0] as u16) << 8) | (chunk[1] as u16)
                    };
                    chars.push(c);
                }
            }
            String::from_utf16_lossy(&chars)
        }
        3 => String::from_utf8_lossy(description_bytes).to_string(),
        _ => String::new(),
    };

    // Picture data size
    let _data_pos = description_end + if encoding == 1 || encoding == 2 { 2 } else { 1 };
    let picture_size = data.len().saturating_sub(_data_pos);

    // Store metadata
    let format_str = if mime_type.contains("jpeg") || mime_type == "JPG" {
        "JPG"
    } else if mime_type.contains("png") || mime_type == "PNG" {
        "PNG"
    } else if mime_type.contains("gif") || mime_type == "GIF" {
        "GIF"
    } else {
        &mime_type
    };

    metadata.insert(
        "ID3:PictureFormat".to_string(),
        TagValue::new_string(format_str),
    );
    metadata.insert(
        "ID3:PictureType".to_string(),
        TagValue::new_string(picture_type_str),
    );
    if !description.is_empty() && description.chars().all(|c| !c.is_control()) {
        metadata.insert(
            "ID3:PictureDescription".to_string(),
            TagValue::new_string(description.trim_end_matches('\0').to_string()),
        );
    }
    metadata.insert(
        "ID3:Picture".to_string(),
        TagValue::new_string(format!(
            "(Binary data {} bytes, use -b option to extract)",
            picture_size
        )),
    );

    Ok(())
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
