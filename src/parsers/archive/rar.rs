//! RAR archive format parser
//!
//! Implements comprehensive metadata extraction from RAR archive files.
//! Supports both RAR4 and RAR5 formats with detailed archive properties.

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};
use crate::io::EndianReader;

/// RAR signature: "Rar!" (0x52 0x61 0x72 0x21)
const RAR_SIGNATURE: &[u8] = b"Rar!";

/// RAR5 signature (additional marker at offset 7)
const RAR5_MARKER: u8 = 0x01;

/// RAR4 block types
const RAR4_BLOCK_ARCHIVE: u8 = 0x73;
const RAR4_BLOCK_FILE: u8 = 0x74;

/// RAR4 archive header flags (offset 2 in header)
const RAR4_FLAG_VOLUME: u16 = 0x0001; // Bit 0: Multi-part archive
const RAR4_FLAG_COMMENT: u16 = 0x0002; // Bit 1: Archive comment present
const RAR4_FLAG_LOCK: u16 = 0x0004; // Bit 2: Archive lock
const RAR4_FLAG_SOLID: u16 = 0x0008; // Bit 3: Solid archive
const RAR4_FLAG_NEWNUMBERING: u16 = 0x0010; // Bit 4: New naming scheme
const RAR4_FLAG_AUTH: u16 = 0x0020; // Bit 5: Authenticity info
const RAR4_FLAG_RECOVERY: u16 = 0x0040; // Bit 6: Recovery record
const RAR4_FLAG_ENCRYPTED: u16 = 0x0080; // Bit 7: Block headers encrypted

/// RAR5 header types
const RAR5_HEADER_MAIN: u64 = 1;
const RAR5_HEADER_FILE: u64 = 2;

/// RAR5 main archive flags
const RAR5_FLAG_VOLUME: u64 = 0x0001;
const RAR5_FLAG_RECOVERY: u64 = 0x0002;
const RAR5_FLAG_LOCKED: u64 = 0x0004;
const RAR5_FLAG_SOLID: u64 = 0x0008;

/// RAR5 encryption flags
const RAR5_FLAG_ENCRYPTED: u64 = 0x0001;

/// RAR parser for extracting metadata from RAR archives
pub struct RARParser;

impl RARParser {
    /// Verifies RAR signature
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 7 {
            return Ok(false);
        }

        let header = reader.read(0, 7)?;
        Ok(header.starts_with(RAR_SIGNATURE))
    }

    /// Detects RAR version (4.x or 5.0)
    pub fn detect_version(reader: &dyn FileReader) -> Result<&'static str> {
        if reader.size() < 8 {
            return Ok("Unknown");
        }

        let header = reader.read(0, 8)?;
        if header.len() >= 7 && &header[0..4] == RAR_SIGNATURE {
            // RAR5 has 0x01 at offset 7
            if header.len() >= 8 && header[7] == RAR5_MARKER {
                Ok("5.0")
            } else {
                Ok("4.x")
            }
        } else {
            Ok("Unknown")
        }
    }

    /// Parses RAR4 archive header and extracts metadata
    fn parse_rar4_metadata(reader: &dyn FileReader, metadata: &mut MetadataMap) -> Result<()> {
        // RAR4 structure: Signature (7 bytes) + Archive Header Block
        // Archive header starts at offset 7
        if reader.size() < 20 {
            return Ok(()); // Not enough data for full header
        }

        let header_data = reader.read(7, 13)?;
        if header_data.len() < 7 {
            return Ok(());
        }

        // Check if this is archive header (type 0x73)
        if header_data[2] != RAR4_BLOCK_ARCHIVE {
            return Ok(()); // Not an archive header
        }

        // Extract flags at offset 3-4 (little-endian u16)
        if header_data.len() >= 5 {
            let reader = EndianReader::little_endian(header_data);
            let flags = reader.u16_at(3).unwrap_or(0);

            metadata.insert(
                "IsVolume".to_string(),
                TagValue::String((flags & RAR4_FLAG_VOLUME != 0).to_string()),
            );
            metadata.insert(
                "IsSolid".to_string(),
                TagValue::String((flags & RAR4_FLAG_SOLID != 0).to_string()),
            );
            metadata.insert(
                "HasRecoveryRecord".to_string(),
                TagValue::String((flags & RAR4_FLAG_RECOVERY != 0).to_string()),
            );
            metadata.insert(
                "IsEncrypted".to_string(),
                TagValue::String((flags & RAR4_FLAG_ENCRYPTED != 0).to_string()),
            );
            metadata.insert(
                "HasComment".to_string(),
                TagValue::String((flags & RAR4_FLAG_COMMENT != 0).to_string()),
            );
            metadata.insert(
                "IsLocked".to_string(),
                TagValue::String((flags & RAR4_FLAG_LOCK != 0).to_string()),
            );
        }

        // Count file entries by scanning for file header blocks (0x74)
        if let Ok(count) = Self::count_rar4_files(reader) {
            metadata.insert("FileCount".to_string(), TagValue::String(count.to_string()));
        }

        Ok(())
    }

    /// Counts file entries in RAR4 archive
    fn count_rar4_files(reader: &dyn FileReader) -> Result<u32> {
        let mut offset = 7u64;
        let mut file_count = 0u32;
        let max_offset = reader.size().min(1024 * 1024); // Limit scan to 1MB

        while offset + 7 < max_offset {
            let block_header = reader.read(offset, 7)?;
            if block_header.len() < 7 {
                break;
            }

            let r = EndianReader::little_endian(block_header);
            let block_type = block_header[2];
            let _block_flags = r.u16_at(3).unwrap_or(0);
            let block_size = r.u16_at(5).unwrap_or(0);

            if block_type == RAR4_BLOCK_FILE {
                file_count += 1;
            }

            // Advance to next block
            if block_size == 0 {
                break;
            }
            offset += block_size as u64;

            // Safety limit: stop after 10000 files
            if file_count >= 10000 {
                break;
            }
        }

        Ok(file_count)
    }

    /// Parses RAR5 archive header and extracts metadata
    fn parse_rar5_metadata(reader: &dyn FileReader, metadata: &mut MetadataMap) -> Result<()> {
        // RAR5 structure: Signature (8 bytes) + Main Archive Header
        if reader.size() < 20 {
            return Ok(());
        }

        let header_data = reader.read(8, 32)?;
        if header_data.len() < 10 {
            return Ok(());
        }

        // Parse variable-length header
        let (_header_crc, pos) = Self::read_rar5_u32(header_data, 0)?;
        let (_header_size, pos) = Self::read_rar5_vint(header_data, pos)?;
        let (header_type, pos) = Self::read_rar5_vint(header_data, pos)?;
        let (header_flags, _) = Self::read_rar5_vint(header_data, pos)?;

        if header_type == RAR5_HEADER_MAIN {
            metadata.insert(
                "IsVolume".to_string(),
                TagValue::String((header_flags & RAR5_FLAG_VOLUME != 0).to_string()),
            );
            metadata.insert(
                "IsSolid".to_string(),
                TagValue::String((header_flags & RAR5_FLAG_SOLID != 0).to_string()),
            );
            metadata.insert(
                "HasRecoveryRecord".to_string(),
                TagValue::String((header_flags & RAR5_FLAG_RECOVERY != 0).to_string()),
            );
            metadata.insert(
                "IsLocked".to_string(),
                TagValue::String((header_flags & RAR5_FLAG_LOCKED != 0).to_string()),
            );

            // Check encryption flag
            let is_encrypted = (header_flags & RAR5_FLAG_ENCRYPTED) != 0;
            metadata.insert(
                "IsEncrypted".to_string(),
                TagValue::String(is_encrypted.to_string()),
            );
        }

        // Count file entries in RAR5
        if let Ok(count) = Self::count_rar5_files(reader) {
            metadata.insert("FileCount".to_string(), TagValue::String(count.to_string()));
        }

        Ok(())
    }

    /// Reads a 32-bit little-endian integer from RAR5 data
    fn read_rar5_u32(data: &[u8], offset: usize) -> Result<(u32, usize)> {
        let reader = EndianReader::little_endian(data);
        let value = reader
            .u32_at(offset)
            .ok_or_else(|| ExifToolError::parse_error("Unexpected end of RAR5 header"))?;
        Ok((value, offset + 4))
    }

    /// Reads a variable-length integer from RAR5 data
    fn read_rar5_vint(data: &[u8], offset: usize) -> Result<(u64, usize)> {
        if offset >= data.len() {
            return Err(ExifToolError::parse_error("Unexpected end of RAR5 vint"));
        }

        let mut result = 0u64;
        let mut shift = 0;
        let mut pos = offset;

        loop {
            if pos >= data.len() {
                return Err(ExifToolError::parse_error("Incomplete RAR5 vint"));
            }

            let byte = data[pos];
            pos += 1;

            result |= ((byte & 0x7F) as u64) << shift;
            shift += 7;

            if (byte & 0x80) == 0 {
                break;
            }

            if shift >= 64 {
                return Err(ExifToolError::parse_error("RAR5 vint overflow"));
            }
        }

        Ok((result, pos))
    }

    /// Counts file entries in RAR5 archive
    fn count_rar5_files(reader: &dyn FileReader) -> Result<u32> {
        let mut offset = 8u64;
        let mut file_count = 0u32;
        let max_offset = reader.size().min(1024 * 1024); // Limit scan to 1MB

        while offset + 10 < max_offset {
            let block_data = reader.read(offset, 20)?;
            if block_data.len() < 10 {
                break;
            }

            // Try to parse header
            if let Ok((_, pos1)) = Self::read_rar5_u32(block_data, 0) {
                if let Ok((header_size, pos2)) = Self::read_rar5_vint(block_data, pos1) {
                    if let Ok((header_type, _)) = Self::read_rar5_vint(block_data, pos2) {
                        if header_type == RAR5_HEADER_FILE {
                            file_count += 1;
                        }

                        // Advance to next block
                        if header_size == 0 || header_size > 1024 * 1024 {
                            break;
                        }
                        offset += header_size;

                        // Safety limit
                        if file_count >= 10000 {
                            break;
                        }
                        continue;
                    }
                }
            }
            break;
        }

        Ok(file_count)
    }
}

impl FormatParser for RARParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        // Verify signature
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid RAR signature"));
        }

        let mut metadata = MetadataMap::new();

        // Detect version
        let version = Self::detect_version(reader)?;
        metadata.insert("FileType".to_string(), TagValue::String("RAR".to_string()));
        metadata.insert(
            "RARVersion".to_string(),
            TagValue::String(version.to_string()),
        );
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );

        // Parse format-specific metadata
        match version {
            "5.0" => {
                Self::parse_rar5_metadata(reader, &mut metadata)?;
            }
            "4.x" => {
                Self::parse_rar4_metadata(reader, &mut metadata)?;
            }
            _ => {
                // Unknown version, skip detailed parsing
            }
        }

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::RAR)
    }
}

/// Standalone function for parsing RAR metadata
///
/// This function provides a convenient interface for parsing RAR archive metadata
/// by instantiating the RARParser and calling its parse method.
///
/// # Arguments
///
/// * `reader` - A FileReader providing access to the RAR file data
///
/// # Returns
///
/// * `Ok(MetadataMap)` - Successfully extracted metadata
/// * `Err(String)` - Parse error description
pub fn parse_rar_metadata(
    reader: &dyn crate::core::FileReader,
) -> std::result::Result<MetadataMap, String> {
    let parser = RARParser;
    parser
        .parse(reader)
        .map_err(|e| format!("RAR parse error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestReader;

    #[test]
    fn test_rar_signature() {
        let mut data = b"Rar!".to_vec();
        data.extend_from_slice(&[0x1A, 0x07, 0x00]);
        let reader = TestReader::new(data);
        assert!(RARParser::verify_signature(&reader).unwrap());
    }

    #[test]
    fn test_rar5_detection() {
        let mut data = b"Rar!".to_vec();
        data.extend_from_slice(&[0x1A, 0x07, 0x01, 0x01]);
        let reader = TestReader::new(data);
        assert_eq!(RARParser::detect_version(&reader).unwrap(), "5.0");
    }

    #[test]
    fn test_rar4_metadata_extraction() {
        // Create minimal RAR4 archive with header
        let mut data = b"Rar!".to_vec();
        data.extend_from_slice(&[0x1A, 0x07, 0x00]); // RAR4 signature
                                                     // Archive header block
        data.extend_from_slice(&[
            0x33, 0x92, // HEAD_CRC
            0x73, // HEAD_TYPE (archive)
            0x09, 0x00, // HEAD_FLAGS (solid + volume)
            0x0D, 0x00, // HEAD_SIZE
        ]);
        data.extend_from_slice(&[0x00; 6]); // Reserved

        let reader = TestReader::new(data);
        let parser = RARParser;
        let metadata = parser.parse(&reader).unwrap();

        assert_eq!(
            metadata.get("FileType").unwrap(),
            &TagValue::String("RAR".to_string())
        );
        assert_eq!(
            metadata.get("RARVersion").unwrap(),
            &TagValue::String("4.x".to_string())
        );
        assert_eq!(
            metadata.get("IsSolid").unwrap(),
            &TagValue::String("true".to_string())
        );
        assert_eq!(
            metadata.get("IsVolume").unwrap(),
            &TagValue::String("true".to_string())
        );
    }

    #[test]
    fn test_rar5_vint_parsing() {
        // Test variable-length integer parsing
        let data = vec![0x80, 0x01]; // 128 in vint format
        let (value, pos) = RARParser::read_rar5_vint(&data, 0).unwrap();
        assert_eq!(value, 128);
        assert_eq!(pos, 2);

        let data = vec![0x7F]; // 127 in vint format
        let (value, pos) = RARParser::read_rar5_vint(&data, 0).unwrap();
        assert_eq!(value, 127);
        assert_eq!(pos, 1);
    }

    #[test]
    fn test_rar4_file_counting() {
        // Create RAR4 archive with 2 file headers
        let mut data = b"Rar!".to_vec();
        data.extend_from_slice(&[0x1A, 0x07, 0x00]); // RAR4 signature

        // Archive header
        data.extend_from_slice(&[
            0x33, 0x92, // CRC
            0x73, // Type: archive
            0x00, 0x00, // Flags
            0x0D, 0x00, // Size: 13 bytes
        ]);
        data.extend_from_slice(&[0x00; 6]); // Reserved

        // File header 1
        data.extend_from_slice(&[
            0x33, 0x92, // CRC
            0x74, // Type: file
            0x00, 0x00, // Flags
            0x20, 0x00, // Size: 32 bytes
        ]);
        data.extend_from_slice(&[0x00; 25]); // Rest of file header

        // File header 2
        data.extend_from_slice(&[
            0x33, 0x92, // CRC
            0x74, // Type: file
            0x00, 0x00, // Flags
            0x20, 0x00, // Size: 32 bytes
        ]);
        data.extend_from_slice(&[0x00; 25]); // Rest of file header

        let reader = TestReader::new(data);
        let count = RARParser::count_rar4_files(&reader).unwrap();
        assert_eq!(count, 2);
    }
}
