//! MXF (Material eXchange Format) parser
//!
//! Implements metadata extraction from MXF video files used in professional
//! video production environments. MXF is based on SMPTE standards.
//!
//! # Supported Metadata
//!
//! - **Header Partition:** Format version, operational pattern
//! - **Identification Set:** Application info, SDK version
//! - **Source Package:** Track and component info
//! - **Timecode:** Start timecode, frame rate
//!
//! # File Structure
//!
//! MXF uses KLV (Key-Length-Value) triplets:
//! ```text
//! [Key - 16 bytes]
//!   └─ Universal Label (UL) identifying the element type
//! [Length - 1-9 bytes]
//!   └─ BER-encoded length
//! [Value - variable]
//!   └─ Element data
//! ```
//!
//! # References
//!
//! - SMPTE ST 377-1 MXF File Format Specification
//! - ExifTool Source: `lib/Image/ExifTool/MXF.pm`

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// MXF Partition Pack Key prefix (first 13 bytes)
/// 06.0E.2B.34.02.05.01.01.0D.01.02.01.01
const MXF_PARTITION_PACK_PREFIX: [u8; 13] = [
    0x06, 0x0E, 0x2B, 0x34, 0x02, 0x05, 0x01, 0x01, 0x0D, 0x01, 0x02, 0x01, 0x01,
];

/// Identification Set UL prefix
const IDENTIFICATION_SET_UL: [u8; 13] = [
    0x06, 0x0E, 0x2B, 0x34, 0x02, 0x53, 0x01, 0x01, 0x0D, 0x01, 0x01, 0x01, 0x01,
];

/// MXF parser
pub struct MxfParser;

impl MxfParser {
    /// Verify MXF file signature
    pub fn verify_signature(data: &[u8]) -> bool {
        // MXF starts with KLV with UL prefix 06.0E.2B.34
        data.len() >= 16 && data[0] == 0x06 && data[1] == 0x0E && data[2] == 0x2B && data[3] == 0x34
    }

    /// Decode BER-encoded length
    /// Returns (length, bytes consumed)
    fn decode_ber_length(data: &[u8]) -> Option<(u64, usize)> {
        if data.is_empty() {
            return None;
        }

        let first = data[0];
        if first < 128 {
            // Short form: length is the byte itself
            Some((first as u64, 1))
        } else {
            // Long form: first byte indicates number of length bytes
            let len_bytes = (first & 0x7F) as usize;
            if len_bytes > 8 || data.len() <= len_bytes {
                return None;
            }
            let mut length: u64 = 0;
            for byte in &data[1..=len_bytes] {
                length = (length << 8) | (*byte as u64);
            }
            Some((length, 1 + len_bytes))
        }
    }

    /// Parse a 16-byte Universal Label to identify the element
    fn identify_ul(key: &[u8; 16]) -> ULType {
        // Check for partition pack
        if key[..13] == MXF_PARTITION_PACK_PREFIX {
            return match key[13] {
                0x02 => ULType::HeaderPartitionPack,
                0x03 => ULType::BodyPartitionPack,
                0x04 => ULType::FooterPartitionPack,
                _ => ULType::Unknown,
            };
        }

        // Check for identification set (local set)
        if key[..13] == IDENTIFICATION_SET_UL && key[13] == 0x30 {
            return ULType::IdentificationSet;
        }

        // Check for preface set
        if key[..13] == IDENTIFICATION_SET_UL && key[13] == 0x2F {
            return ULType::PrefaceSet;
        }

        // Check for content storage
        if key[..13] == IDENTIFICATION_SET_UL && key[13] == 0x18 {
            return ULType::ContentStorageSet;
        }

        // Check for material package
        if key[..13] == IDENTIFICATION_SET_UL && key[13] == 0x36 {
            return ULType::MaterialPackageSet;
        }

        // Check for source package
        if key[..13] == IDENTIFICATION_SET_UL && key[13] == 0x37 {
            return ULType::SourcePackageSet;
        }

        // Check for track sets
        if key[..13] == IDENTIFICATION_SET_UL {
            return match key[13] {
                0x3A => ULType::EventTrackSet,
                0x3B => ULType::StaticTrackSet,
                0x3D => ULType::TimelineTrackSet,
                _ => ULType::Unknown,
            };
        }

        // Check for essence descriptors
        if key[0..4] == [0x06, 0x0E, 0x2B, 0x34] && key[4..8] == [0x02, 0x53, 0x01, 0x01] {
            if key[8..12] == [0x0D, 0x01, 0x01, 0x01] && key[12] == 0x01 {
                return match key[13] {
                    0x25 => ULType::FileDescriptor,
                    0x27 => ULType::GenericPictureDescriptor,
                    0x28 => ULType::CDCIDescriptor,
                    0x29 => ULType::RGBADescriptor,
                    0x42 => ULType::GenericSoundDescriptor,
                    0x47 => ULType::AES3Descriptor,
                    0x48 => ULType::WaveAudioDescriptor,
                    _ => ULType::Unknown,
                };
            }
        }

        // Check for timecode component
        if key[0..4] == [0x06, 0x0E, 0x2B, 0x34] && key[4..8] == [0x02, 0x53, 0x01, 0x01] {
            if key[8..12] == [0x0D, 0x01, 0x01, 0x01] && key[12] == 0x01 && key[13] == 0x14 {
                return ULType::TimecodeComponent;
            }
        }

        // Check for sequence
        if key[0..4] == [0x06, 0x0E, 0x2B, 0x34] && key[4..8] == [0x02, 0x53, 0x01, 0x01] {
            if key[8..12] == [0x0D, 0x01, 0x01, 0x01] && key[12] == 0x01 && key[13] == 0x0F {
                return ULType::SequenceSet;
            }
        }

        ULType::Unknown
    }

    /// Parse header partition pack
    fn parse_header_partition(
        data: &[u8],
        metadata: &mut MetadataMap,
    ) -> std::result::Result<(), String> {
        if data.len() < 24 {
            return Ok(());
        }

        // Major/Minor version at offset 0-1
        let major = u16::from_be_bytes([data[0], data[1]]);
        let minor = u16::from_be_bytes([data[2], data[3]]);
        metadata.insert(
            "MXF:MXFVersion".to_string(),
            TagValue::new_string(format!("{}.{}", major, minor)),
        );

        // KAG Size at offset 4-7
        let kag_size = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        if kag_size > 0 {
            metadata.insert(
                "MXF:KAGSize".to_string(),
                TagValue::new_integer(kag_size as i64),
            );
        }

        Ok(())
    }

    /// Parse identification set
    fn parse_identification_set(data: &[u8], metadata: &mut MetadataMap) {
        let mut offset = 0;

        while offset + 4 < data.len() {
            // Local tag (2 bytes) + length (2 bytes)
            let tag = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
            offset += 4;

            if offset + len > data.len() {
                break;
            }

            let value_data = &data[offset..offset + len];
            offset += len;

            match tag {
                // Company Name - 0x3C01
                0x3C01 => {
                    if let Some(s) = parse_utf16_string(value_data) {
                        metadata.insert(
                            "MXF:ApplicationSupplierName".to_string(),
                            TagValue::new_string(s),
                        );
                    }
                }
                // Product Name - 0x3C02
                0x3C02 => {
                    if let Some(s) = parse_utf16_string(value_data) {
                        metadata.insert("MXF:ApplicationName".to_string(), TagValue::new_string(s));
                    }
                }
                // Product Version - 0x3C03
                0x3C03 => {
                    if value_data.len() >= 10 {
                        let maj = u16::from_be_bytes([value_data[0], value_data[1]]);
                        let min = u16::from_be_bytes([value_data[2], value_data[3]]);
                        let patch = u16::from_be_bytes([value_data[4], value_data[5]]);
                        let build = u16::from_be_bytes([value_data[6], value_data[7]]);
                        let release = value_data[9]; // Release type

                        let release_str = match release {
                            0 => "unknown",
                            1 => "released",
                            2 => "development",
                            3 => "patch level",
                            4 => "beta",
                            5 => "private build",
                            _ => "unknown",
                        };

                        metadata.insert(
                            "MXF:SDKVersion".to_string(),
                            TagValue::new_string(format!("{}.{}", maj, min)),
                        );
                        metadata.insert(
                            "MXF:ToolkitVersion".to_string(),
                            TagValue::new_string(format!(
                                "{}.{}.{}.{} {}",
                                maj, min, patch, build, release_str
                            )),
                        );
                    }
                }
                // Version String - 0x3C04
                0x3C04 => {
                    if let Some(s) = parse_utf16_string(value_data) {
                        metadata.insert(
                            "MXF:ApplicationVersionString".to_string(),
                            TagValue::new_string(s),
                        );
                    }
                }
                // Platform - 0x3C08
                0x3C08 => {
                    if let Some(s) = parse_utf16_string(value_data) {
                        metadata.insert(
                            "MXF:ApplicationPlatform".to_string(),
                            TagValue::new_string(s),
                        );
                    }
                }
                // Modification Date - 0x3C06
                0x3C06 => {
                    if let Some(ts) = parse_mxf_timestamp(value_data) {
                        metadata.insert(
                            "MXF:ModifyDate".to_string(),
                            TagValue::new_string(ts.clone()),
                        );
                        metadata.insert(
                            "MXF:PackageLastModifyDate".to_string(),
                            TagValue::new_string(ts),
                        );
                    }
                }
                _ => {}
            }
        }
    }

    /// Parse preface set for creation date and other info
    fn parse_preface_set(data: &[u8], metadata: &mut MetadataMap) {
        let mut offset = 0;

        while offset + 4 < data.len() {
            let tag = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
            offset += 4;

            if offset + len > data.len() {
                break;
            }

            let value_data = &data[offset..offset + len];
            offset += len;

            match tag {
                // Last Modified Date - 0x3B02
                0x3B02 => {
                    if let Some(ts) = parse_mxf_timestamp(value_data) {
                        metadata.insert(
                            "MXF:ContainerLastModifyDate".to_string(),
                            TagValue::new_string(ts),
                        );
                    }
                }
                // Version - 0x3B05
                0x3B05 => {
                    if value_data.len() >= 2 {
                        let version = u16::from_be_bytes([value_data[0], value_data[1]]);
                        let major = version >> 8;
                        let minor = version & 0xFF;
                        metadata.insert(
                            "MXF:FileFormatVersion".to_string(),
                            TagValue::new_string(format!("{}.{}", major, minor)),
                        );
                    }
                }
                _ => {}
            }
        }
    }

    /// Parse timeline track set for frame rate and origin
    fn parse_timeline_track_set(data: &[u8], metadata: &mut MetadataMap) {
        let mut offset = 0;

        while offset + 4 < data.len() {
            let tag = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
            offset += 4;

            if offset + len > data.len() {
                break;
            }

            let value_data = &data[offset..offset + len];
            offset += len;

            match tag {
                // Edit Rate - 0x4B01
                0x4B01 => {
                    if value_data.len() >= 8 {
                        let num = i32::from_be_bytes([
                            value_data[0],
                            value_data[1],
                            value_data[2],
                            value_data[3],
                        ]);
                        let den = i32::from_be_bytes([
                            value_data[4],
                            value_data[5],
                            value_data[6],
                            value_data[7],
                        ]);
                        if den != 0 {
                            // Only set if not already set
                            if !metadata.contains_key("MXF:EditRate") {
                                metadata.insert(
                                    "MXF:EditRate".to_string(),
                                    TagValue::new_integer(num as i64),
                                );
                            }
                        }
                    }
                }
                // Origin - 0x4B02
                0x4B02 => {
                    if value_data.len() >= 8 {
                        let origin = i64::from_be_bytes([
                            value_data[0],
                            value_data[1],
                            value_data[2],
                            value_data[3],
                            value_data[4],
                            value_data[5],
                            value_data[6],
                            value_data[7],
                        ]);
                        metadata.insert(
                            "MXF:Origin".to_string(),
                            TagValue::new_string(format!("{} s", origin)),
                        );
                    }
                }
                // Track ID - 0x4801
                0x4801 => {
                    if value_data.len() >= 4 {
                        let track_id = u32::from_be_bytes([
                            value_data[0],
                            value_data[1],
                            value_data[2],
                            value_data[3],
                        ]);
                        // Only set first track ID found
                        if !metadata.contains_key("MXF:TrackID") {
                            metadata.insert(
                                "MXF:TrackID".to_string(),
                                TagValue::new_integer(track_id as i64),
                            );
                        }
                    }
                }
                // Track Number - 0x4804
                0x4804 => {
                    if value_data.len() >= 4 {
                        let track_num = u32::from_be_bytes([
                            value_data[0],
                            value_data[1],
                            value_data[2],
                            value_data[3],
                        ]);
                        if !metadata.contains_key("MXF:TrackNumber") {
                            metadata.insert(
                                "MXF:TrackNumber".to_string(),
                                TagValue::new_integer(track_num as i64),
                            );
                        }
                    }
                }
                // Track Name - 0x4802
                0x4802 => {
                    if let Some(s) = parse_utf16_string(value_data) {
                        if !metadata.contains_key("MXF:TrackName") {
                            metadata.insert("MXF:TrackName".to_string(), TagValue::new_string(s));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Parse timecode component for start timecode
    fn parse_timecode_component(data: &[u8], metadata: &mut MetadataMap) {
        let mut offset = 0;

        while offset + 4 < data.len() {
            let tag = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
            offset += 4;

            if offset + len > data.len() {
                break;
            }

            let value_data = &data[offset..offset + len];
            offset += len;

            match tag {
                // Start Timecode - 0x1501
                0x1501 => {
                    if value_data.len() >= 8 {
                        let start = i64::from_be_bytes([
                            value_data[0],
                            value_data[1],
                            value_data[2],
                            value_data[3],
                            value_data[4],
                            value_data[5],
                            value_data[6],
                            value_data[7],
                        ]);
                        metadata.insert(
                            "MXF:StartTimecode".to_string(),
                            TagValue::new_string(format!("{} s", start)),
                        );
                    }
                }
                // Rounded Timecode Base - 0x1502
                0x1502 => {
                    if value_data.len() >= 2 {
                        let base = u16::from_be_bytes([value_data[0], value_data[1]]);
                        metadata.insert(
                            "MXF:RoundedTimecodeTimebase".to_string(),
                            TagValue::new_integer(base as i64),
                        );
                    }
                }
                // Drop Frame - 0x1503
                0x1503 => {
                    if !value_data.is_empty() {
                        let drop = value_data[0] != 0;
                        metadata.insert(
                            "MXF:DropFrame".to_string(),
                            TagValue::new_string(if drop { "true" } else { "false" }),
                        );
                    }
                }
                _ => {}
            }
        }
    }

    /// Parse sequence set for duration
    fn parse_sequence_set(data: &[u8], metadata: &mut MetadataMap) {
        let mut offset = 0;

        while offset + 4 < data.len() {
            let tag = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
            offset += 4;

            if offset + len > data.len() {
                break;
            }

            let value_data = &data[offset..offset + len];
            offset += len;

            // Duration - 0x0202
            if tag == 0x0202 && value_data.len() >= 8 {
                let duration = i64::from_be_bytes([
                    value_data[0],
                    value_data[1],
                    value_data[2],
                    value_data[3],
                    value_data[4],
                    value_data[5],
                    value_data[6],
                    value_data[7],
                ]);
                // Only set first duration found
                if !metadata.contains_key("MXF:Duration") {
                    metadata.insert(
                        "MXF:Duration".to_string(),
                        TagValue::new_string(format!("{} s", duration)),
                    );
                }
            }

            // Data definition for component type
            if tag == 0x0201 && value_data.len() >= 16 {
                // Check if this is sound essence
                if value_data[12] == 0x01 && value_data[13] == 0x02 {
                    if !metadata.contains_key("MXF:ComponentDataDefinition") {
                        metadata.insert(
                            "MXF:ComponentDataDefinition".to_string(),
                            TagValue::new_string("Sound Essence Track"),
                        );
                    }
                } else if value_data[12] == 0x01 && value_data[13] == 0x01 {
                    if !metadata.contains_key("MXF:ComponentDataDefinition") {
                        metadata.insert(
                            "MXF:ComponentDataDefinition".to_string(),
                            TagValue::new_string("Picture Essence Track"),
                        );
                    }
                }
            }
        }
    }

    /// Parse wave audio descriptor
    fn parse_wave_audio_descriptor(data: &[u8], metadata: &mut MetadataMap) {
        let mut offset = 0;

        while offset + 4 < data.len() {
            let tag = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
            offset += 4;

            if offset + len > data.len() {
                break;
            }

            let value_data = &data[offset..offset + len];
            offset += len;

            match tag {
                // Audio sampling rate - 0x3D03
                0x3D03 => {
                    if value_data.len() >= 8 {
                        let num = i32::from_be_bytes([
                            value_data[0],
                            value_data[1],
                            value_data[2],
                            value_data[3],
                        ]);
                        metadata.insert(
                            "MXF:AudioSampleRate".to_string(),
                            TagValue::new_integer(num as i64),
                        );
                    }
                }
                // Locked/Unlocked - 0x3D02
                0x3D02 => {
                    if !value_data.is_empty() {
                        let locked = value_data[0] != 0;
                        metadata.insert(
                            "MXF:LockedIndicator".to_string(),
                            TagValue::new_string(if locked { "true" } else { "false" }),
                        );
                    }
                }
                // Channel count - 0x3D07
                0x3D07 => {
                    if value_data.len() >= 4 {
                        let channels = u32::from_be_bytes([
                            value_data[0],
                            value_data[1],
                            value_data[2],
                            value_data[3],
                        ]);
                        metadata.insert(
                            "MXF:ChannelCount".to_string(),
                            TagValue::new_integer(channels as i64),
                        );
                    }
                }
                // Quantization bits - 0x3D01
                0x3D01 => {
                    if value_data.len() >= 4 {
                        let bits = u32::from_be_bytes([
                            value_data[0],
                            value_data[1],
                            value_data[2],
                            value_data[3],
                        ]);
                        metadata.insert(
                            "MXF:BitsPerAudioSample".to_string(),
                            TagValue::new_integer(bits as i64),
                        );
                    }
                }
                // Block align - 0x3D0A
                0x3D0A => {
                    if value_data.len() >= 2 {
                        let align = u16::from_be_bytes([value_data[0], value_data[1]]);
                        metadata.insert(
                            "MXF:BlockAlign".to_string(),
                            TagValue::new_integer(align as i64),
                        );
                    }
                }
                // Avg bytes per second - 0x3D09
                0x3D09 => {
                    if value_data.len() >= 4 {
                        let bps = u32::from_be_bytes([
                            value_data[0],
                            value_data[1],
                            value_data[2],
                            value_data[3],
                        ]);
                        metadata.insert(
                            "MXF:AverageBytesPerSecond".to_string(),
                            TagValue::new_integer(bps as i64),
                        );
                    }
                }
                // Sample rate - 0x3001
                0x3001 => {
                    if value_data.len() >= 8 {
                        let num = i32::from_be_bytes([
                            value_data[0],
                            value_data[1],
                            value_data[2],
                            value_data[3],
                        ]);
                        metadata.insert(
                            "MXF:SampleRate".to_string(),
                            TagValue::new_integer(num as i64),
                        );
                    }
                }
                // Essence length - 0x3002
                0x3002 => {
                    if value_data.len() >= 8 {
                        let len = i64::from_be_bytes([
                            value_data[0],
                            value_data[1],
                            value_data[2],
                            value_data[3],
                            value_data[4],
                            value_data[5],
                            value_data[6],
                            value_data[7],
                        ]);
                        metadata.insert(
                            "MXF:EssenceLength".to_string(),
                            TagValue::new_string(format!("{} s", len)),
                        );
                    }
                }
                _ => {}
            }
        }
    }

    /// Parse file descriptor for linked track
    fn parse_file_descriptor(data: &[u8], metadata: &mut MetadataMap) {
        let mut offset = 0;

        while offset + 4 < data.len() {
            let tag = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
            offset += 4;

            if offset + len > data.len() {
                break;
            }

            let value_data = &data[offset..offset + len];
            offset += len;

            match tag {
                // Linked Track ID - 0x3006
                0x3006 => {
                    if value_data.len() >= 4 {
                        let track_id = u32::from_be_bytes([
                            value_data[0],
                            value_data[1],
                            value_data[2],
                            value_data[3],
                        ]);
                        metadata.insert(
                            "MXF:LinkedTrackID".to_string(),
                            TagValue::new_integer(track_id as i64),
                        );
                    }
                }
                // Essence Stream ID - 0x3004
                0x3004 => {
                    if value_data.len() >= 4 {
                        let stream_id = u32::from_be_bytes([
                            value_data[0],
                            value_data[1],
                            value_data[2],
                            value_data[3],
                        ]);
                        metadata.insert(
                            "MXF:EssenceStreamID".to_string(),
                            TagValue::new_integer(stream_id as i64),
                        );
                    }
                }
                _ => {}
            }
        }
    }

    /// Parse source package for creation date
    fn parse_source_package(data: &[u8], metadata: &mut MetadataMap) {
        let mut offset = 0;

        while offset + 4 < data.len() {
            let tag = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
            offset += 4;

            if offset + len > data.len() {
                break;
            }

            let value_data = &data[offset..offset + len];
            offset += len;

            // Package Creation Date - 0x4404
            if tag == 0x4404 {
                if let Some(ts) = parse_mxf_timestamp(value_data) {
                    metadata.insert("MXF:CreateDate".to_string(), TagValue::new_string(ts));
                }
            }
        }
    }
}

/// Type of Universal Label element
#[derive(Debug, Clone, Copy, PartialEq)]
enum ULType {
    HeaderPartitionPack,
    BodyPartitionPack,
    FooterPartitionPack,
    IdentificationSet,
    PrefaceSet,
    ContentStorageSet,
    MaterialPackageSet,
    SourcePackageSet,
    EventTrackSet,
    StaticTrackSet,
    TimelineTrackSet,
    FileDescriptor,
    GenericPictureDescriptor,
    CDCIDescriptor,
    RGBADescriptor,
    GenericSoundDescriptor,
    AES3Descriptor,
    WaveAudioDescriptor,
    TimecodeComponent,
    SequenceSet,
    Unknown,
}

/// Parse UTF-16 BE string from MXF
fn parse_utf16_string(data: &[u8]) -> Option<String> {
    if data.len() < 2 || data.len() % 2 != 0 {
        return None;
    }

    let mut chars: Vec<u16> = Vec::with_capacity(data.len() / 2);
    for chunk in data.chunks(2) {
        let c = u16::from_be_bytes([chunk[0], chunk[1]]);
        if c == 0 {
            break; // Null terminator
        }
        chars.push(c);
    }

    String::from_utf16(&chars).ok()
}

/// Parse MXF timestamp (8 bytes)
fn parse_mxf_timestamp(data: &[u8]) -> Option<String> {
    if data.len() < 8 {
        return None;
    }

    let year = u16::from_be_bytes([data[0], data[1]]);
    let month = data[2];
    let day = data[3];
    let hour = data[4];
    let minute = data[5];
    let second = data[6];
    let msec = data[7];

    // Format: YYYY:MM:DD HH:MM:SS.mmm
    Some(format!(
        "{:04}:{:02}:{:02} {:02}:{:02}:{:02}.{:03}",
        year,
        month,
        day,
        hour,
        minute,
        second,
        msec as u32 * 4 // MXF uses 1/250ths of a second
    ))
}

impl FormatParser for MxfParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        let file_size = reader.size();
        if file_size < 32 {
            return Err(ExifToolError::parse_error("File too small for MXF"));
        }

        // Read initial header for validation
        let header = reader.read(0, 16)?;
        if !Self::verify_signature(header) {
            return Err(ExifToolError::parse_error("Invalid MXF signature"));
        }

        let mut metadata = MetadataMap::with_capacity(32);

        // Read up to first 256KB for metadata parsing
        let read_size = std::cmp::min(file_size as usize, 262144);
        let data = reader.read(0, read_size)?;

        let mut offset = 0;

        // Parse KLV triplets
        while offset + 20 < data.len() {
            // Ensure we have a valid UL (starts with 06.0E.2B.34)
            if data[offset] != 0x06
                || data[offset + 1] != 0x0E
                || data[offset + 2] != 0x2B
                || data[offset + 3] != 0x34
            {
                offset += 1;
                continue;
            }

            // Read 16-byte key
            let key: [u8; 16] = data[offset..offset + 16].try_into().unwrap_or([0; 16]);
            offset += 16;

            // Decode BER length
            let Some((length, len_bytes)) = Self::decode_ber_length(&data[offset..]) else {
                break;
            };
            offset += len_bytes;

            // Limit value size for safety
            let value_len = length as usize;
            if value_len > 1_000_000 || offset + value_len > data.len() {
                offset = data.len();
                break;
            }

            let value_data = &data[offset..offset + value_len];
            offset += value_len;

            // Parse based on UL type
            match Self::identify_ul(&key) {
                ULType::HeaderPartitionPack => {
                    let _ = Self::parse_header_partition(value_data, &mut metadata);
                }
                ULType::IdentificationSet => {
                    Self::parse_identification_set(value_data, &mut metadata);
                }
                ULType::PrefaceSet => {
                    Self::parse_preface_set(value_data, &mut metadata);
                }
                ULType::TimelineTrackSet => {
                    Self::parse_timeline_track_set(value_data, &mut metadata);
                }
                ULType::TimecodeComponent => {
                    Self::parse_timecode_component(value_data, &mut metadata);
                }
                ULType::SequenceSet => {
                    Self::parse_sequence_set(value_data, &mut metadata);
                }
                ULType::WaveAudioDescriptor | ULType::AES3Descriptor => {
                    Self::parse_wave_audio_descriptor(value_data, &mut metadata);
                }
                ULType::FileDescriptor
                | ULType::GenericPictureDescriptor
                | ULType::GenericSoundDescriptor => {
                    Self::parse_file_descriptor(value_data, &mut metadata);
                }
                ULType::SourcePackageSet => {
                    Self::parse_source_package(value_data, &mut metadata);
                }
                _ => {}
            }
        }

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::MXF)
    }
}

/// Convenience function to parse MXF metadata from a reader.
pub fn parse_mxf_metadata(reader: &dyn FileReader) -> std::result::Result<MetadataMap, String> {
    let parser = MxfParser;
    parser.parse(reader).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestReader;

    #[test]
    fn test_mxf_signature_valid() {
        // Valid MXF header (Partition Pack Key)
        let data = [
            0x06, 0x0E, 0x2B, 0x34, 0x02, 0x05, 0x01, 0x01, 0x0D, 0x01, 0x02, 0x01, 0x01, 0x02,
            0x01, 0x00,
        ];
        assert!(MxfParser::verify_signature(&data));
    }

    #[test]
    fn test_mxf_signature_invalid() {
        let data = [0x00, 0x00, 0x00, 0x00];
        assert!(!MxfParser::verify_signature(&data));
    }

    #[test]
    fn test_ber_decode_short() {
        let data = [0x10];
        let (len, consumed) = MxfParser::decode_ber_length(&data).unwrap();
        assert_eq!(len, 16);
        assert_eq!(consumed, 1);
    }

    #[test]
    fn test_ber_decode_long() {
        let data = [0x82, 0x01, 0x00]; // 256 in 2 bytes
        let (len, consumed) = MxfParser::decode_ber_length(&data).unwrap();
        assert_eq!(len, 256);
        assert_eq!(consumed, 3);
    }

    #[test]
    fn test_mxf_minimal_file() {
        // Create minimal MXF with header partition pack
        let mut data = vec![0u8; 64];
        // Partition Pack Key
        data[0..16].copy_from_slice(&[
            0x06, 0x0E, 0x2B, 0x34, 0x02, 0x05, 0x01, 0x01, 0x0D, 0x01, 0x02, 0x01, 0x01, 0x02,
            0x01, 0x00,
        ]);
        // BER length (short form)
        data[16] = 24;
        // Header partition value (24 bytes)
        // Version: 1.2
        data[17] = 0x00;
        data[18] = 0x01;
        data[19] = 0x00;
        data[20] = 0x02;

        let reader = TestReader::from_slice(&data);
        let parser = MxfParser;
        let result = parser.parse(&reader);
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert_eq!(
            metadata.get("MXF:MXFVersion").unwrap().as_string(),
            Some("1.2")
        );
    }

    #[test]
    fn test_parse_utf16_string() {
        // "Test" in UTF-16 BE
        let data = [0x00, 0x54, 0x00, 0x65, 0x00, 0x73, 0x00, 0x74];
        let result = parse_utf16_string(&data);
        assert_eq!(result, Some("Test".to_string()));
    }

    #[test]
    fn test_parse_mxf_timestamp() {
        // 2010-12-20 00:14:40.228
        let data = [
            0x07, 0xDA, // 2010
            0x0C, // 12
            0x14, // 20
            0x00, // 00
            0x0E, // 14
            0x28, // 40
            0x39, // 228ms / 4 = 57
        ];
        let result = parse_mxf_timestamp(&data);
        assert!(result.is_some());
        let ts = result.unwrap();
        assert!(ts.starts_with("2010:12:20 00:14:40"));
    }
}
