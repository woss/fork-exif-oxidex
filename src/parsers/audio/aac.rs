//! AAC (Advanced Audio Codec) format parser
//!
//! Implements metadata extraction from AAC audio files with ADTS (Audio Data
//! Transport Stream) headers.
//!
//! # Supported Metadata
//!
//! - **ADTS Header:** Profile, sample rate, channel configuration, bitrate
//! - **Frame Info:** Frame count, duration estimation
//!
//! # ExifTool Compatibility
//!
//! Maps to ExifTool tags from `AAC.pm` module:
//! - `AAC:AudioObjectType` → Profile from ADTS header
//! - `AAC:SampleRate` → Sample rate from ADTS header
//! - `AAC:ChannelConfiguration` → Channel config from ADTS header
//!
//! # File Structure
//!
//! ```text
//! [ADTS Frame 0]
//!   ├─ Sync Word: 0xFFF (12 bits)
//!   ├─ Header: Profile, sample rate, channels
//!   └─ AAC Data (variable)
//! [ADTS Frame 1]
//! [ADTS Frame 2]
//! ...
//! ```
//!
//! # References
//!
//! - ISO 13818-7: MPEG-2 Advanced Audio Coding (AAC)
//! - ExifTool Source: `lib/Image/ExifTool/AAC.pm`

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// ADTS sync word (12 bits: 0xFFF)
const ADTS_SYNC_WORD: u16 = 0xFFF;

/// AAC sample rate table (indexed by sample rate index)
const SAMPLE_RATES: [u32; 16] = [
    96000, 88200, 64000, 48000, 44100, 32000, 24000, 22050, 16000, 12000, 11025, 8000, 7350, 0, 0,
    0,
];

/// AAC profile names
const PROFILES: [&str; 4] = [
    "Main",
    "LC (Low Complexity)",
    "SSR (Scalable Sampling Rate)",
    "LTP (Long Term Prediction)",
];

/// AAC parser
pub struct AacParser;

/// Parses metadata from an AAC file.
///
/// This is a convenience wrapper that creates an AacParser instance and calls parse().
///
/// # Arguments
///
/// * `reader` - File reader providing access to the AAC file data
///
/// # Returns
///
/// * `Ok(MetadataMap)` - Successfully extracted metadata
/// * `Err(String)` - Parse error message
pub fn parse_aac_metadata(reader: &dyn FileReader) -> std::result::Result<MetadataMap, String> {
    let parser = AacParser;
    parser.parse(reader).map_err(|e| e.to_string())
}

impl FormatParser for AacParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        let file_size = reader.size();

        // Verify file is large enough for ADTS header
        if file_size < 7 {
            return Err(ExifToolError::parse_error("File too small to be AAC"));
        }

        // Read first ADTS frame header (7 bytes minimum)
        let header = reader.read(0, 7)?;

        // Verify ADTS sync word (0xFFF in first 12 bits)
        let sync = u16::from_be_bytes([header[0], header[1]]);
        if (sync >> 4) != ADTS_SYNC_WORD {
            return Err(ExifToolError::parse_error(format!(
                "Invalid AAC ADTS signature: expected sync word 0xFFF, found 0x{:03X}",
                sync >> 4
            )));
        }

        let mut metadata = MetadataMap::with_capacity(16);

        // Parse ADTS header
        let adts_info = parse_adts_header(header)?;

        // Add metadata
        metadata.insert(
            "AAC:AudioObjectType".to_string(),
            TagValue::new_string(adts_info.profile.to_string()),
        );
        metadata.insert(
            "AAC:SampleRate".to_string(),
            TagValue::new_integer(adts_info.sample_rate as i64),
        );
        metadata.insert(
            "AAC:ChannelConfiguration".to_string(),
            TagValue::new_integer(adts_info.channel_config as i64),
        );
        metadata.insert(
            "AAC:FrameLength".to_string(),
            TagValue::new_integer(adts_info.frame_length as i64),
        );

        // Estimate duration by counting frames (scan up to 1MB)
        let scan_size = 1_000_000u64.min(file_size);
        let mut frame_count = 0u64;
        let mut offset = 0u64;

        while offset + 7 < scan_size {
            // Verify sync word
            let sync_bytes = reader.read(offset, 2)?;
            let sync = u16::from_be_bytes([sync_bytes[0], sync_bytes[1]]);

            if (sync >> 4) != ADTS_SYNC_WORD {
                break;
            }

            // Read frame length from header
            let frame_header = reader.read(offset, 7)?;
            if let Ok(frame_info) = parse_adts_header(frame_header) {
                frame_count += 1;
                offset += frame_info.frame_length as u64;
            } else {
                break;
            }
        }

        if frame_count > 0 {
            metadata.insert(
                "AAC:FrameCount".to_string(),
                TagValue::new_integer(frame_count as i64),
            );

            // Estimate total frames from file size
            let avg_frame_size = offset / frame_count;
            if avg_frame_size > 0 {
                let estimated_total_frames = file_size / avg_frame_size;

                // Calculate duration (1024 samples per frame)
                let samples_per_frame = 1024u64;
                let total_samples = estimated_total_frames * samples_per_frame;
                let duration_secs = total_samples as f64 / adts_info.sample_rate as f64;

                metadata.insert(
                    "AAC:Duration".to_string(),
                    TagValue::new_string(format!("{:.2}", duration_secs)),
                );
            }
        }

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::AAC)
    }
}

/// ADTS header information
struct AdtsInfo {
    profile: &'static str,
    sample_rate: u32,
    channel_config: u8,
    frame_length: u16,
}

/// Parse ADTS header (7 bytes)
fn parse_adts_header(header: &[u8]) -> Result<AdtsInfo> {
    if header.len() < 7 {
        return Err(ExifToolError::parse_error("ADTS header too small"));
    }

    // Parse ADTS fields:
    // Byte 0-1: Sync word (12 bits) + MPEG version (1 bit) + Layer (2 bits) + Protection (1 bit)
    // Byte 2: Profile (2 bits) + Sample rate index (4 bits) + Private (1 bit) + Channel start (1 bit)
    // Byte 3: Channel (2 bits) + Originality (1 bit) + Home (1 bit) + Copyright (2 bits) + Frame length start (2 bits)
    // Byte 4-5: Frame length (11 bits continue) + Buffer fullness start (5 bits)
    // Byte 6: Buffer fullness (6 bits) + Frame count (2 bits)

    let profile_idx = ((header[2] >> 6) & 0x03) as usize;
    let sample_rate_idx = ((header[2] >> 2) & 0x0F) as usize;
    let channel_config = ((header[2] & 0x01) << 2) | ((header[3] >> 6) & 0x03);

    // Frame length (13 bits): bits from byte 3-5
    let frame_length =
        (((header[3] & 0x03) as u16) << 11) | ((header[4] as u16) << 3) | ((header[5] >> 5) as u16);

    // Validate frame length (must be at least 7 bytes for ADTS header)
    if frame_length < 7 {
        return Err(ExifToolError::parse_error(format!(
            "Invalid frame length: {} (must be at least 7)",
            frame_length
        )));
    }

    // Validate indices
    if sample_rate_idx >= SAMPLE_RATES.len() {
        return Err(ExifToolError::parse_error("Invalid sample rate index"));
    }

    let sample_rate = SAMPLE_RATES[sample_rate_idx];
    if sample_rate == 0 {
        return Err(ExifToolError::parse_error("Invalid sample rate"));
    }

    let profile = if profile_idx < PROFILES.len() {
        PROFILES[profile_idx]
    } else {
        "Unknown"
    };

    Ok(AdtsInfo {
        profile,
        sample_rate,
        channel_config,
        frame_length,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestReader;

    #[test]
    fn test_aac_adts_signature_valid() {
        // Create minimal AAC ADTS header
        // 0xFFF1 = sync word (0xFFF) + MPEG-4 (1) + Layer (00) + no CRC (1)
        let mut data = vec![0u8; 1000];
        data[0] = 0xFF; // Sync word high byte
        data[1] = 0xF1; // Sync word low nibble + flags
        data[2] = 0x50; // Profile=1 (LC), Sample rate=4 (44100), private=0, channel start=0
        data[3] = 0x80; // Channel=2 (stereo), other flags
                        // Frame length = 100 bytes (0x64 = 0b0000001100100)
                        // Bits 11-12 in byte 3 (lower 2 bits): 0b00
                        // Bits 3-10 in byte 4: 0b00001100 = 0x0C
                        // Bits 0-2 in byte 5 (upper 3 bits): 0b100 = 0x80
        data[4] = 0x0C; // Frame length middle byte
        data[5] = 0x80; // Frame length low bits + buffer fullness start
        data[6] = 0xFC; // Buffer fullness + frame count

        let reader = TestReader::new(data);
        let parser = AacParser;
        let result = parser.parse(&reader);
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert_eq!(
            metadata.get("AAC:SampleRate").unwrap().as_integer(),
            Some(44100)
        );
    }

    #[test]
    fn test_aac_signature_invalid() {
        let data = b"INVALID DATA";
        let reader = TestReader::from_slice(data);
        let parser = AacParser;
        let result = parser.parse(&reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_aac_file_too_small() {
        let data = b"\xFF\xF1";
        let reader = TestReader::from_slice(data);
        let parser = AacParser;
        let result = parser.parse(&reader);
        assert!(result.is_err());
    }
}
