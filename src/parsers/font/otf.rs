//! OpenType Font (OTF) format parser
//!
//! Implements comprehensive metadata extraction from OpenType font files.
//! OTF uses the same SFNT structure as TTF, with "OTTO" signature indicating CFF outlines.

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// OTF signature: "OTTO"
const OTF_SIGNATURE: &[u8] = b"OTTO";

/// SFNT table directory entry size
const TABLE_ENTRY_SIZE: usize = 16;

/// Seconds between 1904-01-01 and 1970-01-01
const EPOCH_DELTA: i64 = 2082844800;

#[derive(Debug)]
struct TableEntry {
    tag: [u8; 4],
    _checksum: u32,
    offset: u32,
    length: u32,
}

/// OTF parser for extracting metadata from OpenType fonts
pub struct OTFParser;

impl OTFParser {
    /// Verifies OTF signature
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        if reader.size() < 4 {
            return Ok(false);
        }

        let header = reader.read(0, 4)?;
        Ok(header == OTF_SIGNATURE)
    }

    /// Reads number of tables (offset 4, 2 bytes)
    fn read_num_tables(reader: &dyn FileReader) -> Result<u16> {
        if reader.size() < 6 {
            return Ok(0);
        }

        let num_tables_bytes = reader.read(4, 2)?;
        Ok(u16::from_be_bytes([
            num_tables_bytes[0],
            num_tables_bytes[1],
        ]))
    }

    /// Parses the SFNT table directory
    fn parse_table_directory(reader: &dyn FileReader, num_tables: u16) -> Result<Vec<TableEntry>> {
        let mut tables = Vec::new();
        let dir_offset = 12u64; // After SFNT header

        for i in 0..num_tables {
            let entry_offset = dir_offset + (i as u64 * TABLE_ENTRY_SIZE as u64);
            let entry_data = reader.read(entry_offset, TABLE_ENTRY_SIZE)?;

            let mut tag = [0u8; 4];
            tag.copy_from_slice(&entry_data[0..4]);

            let checksum = u32::from_be_bytes([
                entry_data[4],
                entry_data[5],
                entry_data[6],
                entry_data[7],
            ]);
            let offset = u32::from_be_bytes([
                entry_data[8],
                entry_data[9],
                entry_data[10],
                entry_data[11],
            ]);
            let length = u32::from_be_bytes([
                entry_data[12],
                entry_data[13],
                entry_data[14],
                entry_data[15],
            ]);

            tables.push(TableEntry {
                tag,
                _checksum: checksum,
                offset,
                length,
            });
        }

        Ok(tables)
    }

    /// Finds a table by tag
    fn find_table<'a>(tables: &'a [TableEntry], tag: &[u8; 4]) -> Option<&'a TableEntry> {
        tables.iter().find(|t| &t.tag == tag)
    }

    /// Parses the name table to extract font metadata
    fn parse_name_table(
        reader: &dyn FileReader,
        table: &TableEntry,
        metadata: &mut MetadataMap,
    ) -> Result<()> {
        let offset = table.offset as u64;
        let header = reader.read(offset, 6)?;

        let _format = u16::from_be_bytes([header[0], header[1]]);
        let count = u16::from_be_bytes([header[2], header[3]]);
        let string_offset = u16::from_be_bytes([header[4], header[5]]) as u64;

        // Parse name records
        for i in 0..count {
            let record_offset = offset + 6 + (i as u64 * 12);
            let record = reader.read(record_offset, 12)?;

            let platform_id = u16::from_be_bytes([record[0], record[1]]);
            let encoding_id = u16::from_be_bytes([record[2], record[3]]);
            let _language_id = u16::from_be_bytes([record[4], record[5]]);
            let name_id = u16::from_be_bytes([record[6], record[7]]);
            let length = u16::from_be_bytes([record[8], record[9]]) as usize;
            let name_offset = u16::from_be_bytes([record[10], record[11]]) as u64;

            // Prefer Windows Unicode (platform 3, encoding 1)
            if platform_id != 3 || encoding_id != 1 {
                continue;
            }

            let str_offset = offset + string_offset + name_offset;
            let str_data = reader.read(str_offset, length)?;

            // Decode UTF-16BE
            let decoded = Self::decode_utf16be(str_data);

            // Map nameID to metadata field
            let field_name = match name_id {
                0 => Some("Copyright"),
                1 => Some("FontFamily"),
                2 => Some("FontSubfamily"),
                4 => Some("FullFontName"),
                5 => Some("VersionString"),
                6 => Some("PostScriptName"),
                9 => Some("Designer"),
                11 => Some("VendorURL"),
                13 => Some("LicenseDescription"),
                _ => None,
            };

            if let Some(field) = field_name {
                if !decoded.is_empty() {
                    metadata.insert(field.to_string(), TagValue::String(decoded));
                }
            }
        }

        Ok(())
    }

    /// Parses the head table for font metadata
    fn parse_head_table(
        reader: &dyn FileReader,
        table: &TableEntry,
        metadata: &mut MetadataMap,
    ) -> Result<()> {
        let offset = table.offset as u64;

        // Ensure we have enough data
        if offset + 54 > reader.size() {
            return Ok(());
        }

        // UnitsPerEm at offset 18 (2 bytes)
        let units_data = reader.read(offset + 18, 2)?;
        let units_per_em = u16::from_be_bytes([units_data[0], units_data[1]]);
        metadata.insert(
            "UnitsPerEm".to_string(),
            TagValue::String(units_per_em.to_string()),
        );

        // Created timestamp at offset 20 (8 bytes, seconds since 1904-01-01)
        let created_data = reader.read(offset + 20, 8)?;
        let created = i64::from_be_bytes([
            created_data[0],
            created_data[1],
            created_data[2],
            created_data[3],
            created_data[4],
            created_data[5],
            created_data[6],
            created_data[7],
        ]);
        if created > 0 {
            let unix_timestamp = created - EPOCH_DELTA;
            if let Some(timestamp_str) = Self::format_timestamp(unix_timestamp) {
                metadata.insert("CreatedDate".to_string(), TagValue::String(timestamp_str));
            }
        }

        // Modified timestamp at offset 28 (8 bytes)
        let modified_data = reader.read(offset + 28, 8)?;
        let modified = i64::from_be_bytes([
            modified_data[0],
            modified_data[1],
            modified_data[2],
            modified_data[3],
            modified_data[4],
            modified_data[5],
            modified_data[6],
            modified_data[7],
        ]);
        if modified > 0 {
            let unix_timestamp = modified - EPOCH_DELTA;
            if let Some(timestamp_str) = Self::format_timestamp(unix_timestamp) {
                metadata.insert("ModifiedDate".to_string(), TagValue::String(timestamp_str));
            }
        }

        Ok(())
    }

    /// Decodes UTF-16BE string
    fn decode_utf16be(data: &[u8]) -> String {
        let utf16_chars: Vec<u16> = data
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect();

        String::from_utf16_lossy(&utf16_chars)
    }

    /// Formats Unix timestamp to ISO 8601 format
    fn format_timestamp(unix_timestamp: i64) -> Option<String> {
        if unix_timestamp < 0 {
            return None;
        }

        const SECS_PER_DAY: i64 = 86400;
        const SECS_PER_HOUR: i64 = 3600;
        const SECS_PER_MINUTE: i64 = 60;

        let days = unix_timestamp / SECS_PER_DAY;
        let remaining = unix_timestamp % SECS_PER_DAY;
        let hours = remaining / SECS_PER_HOUR;
        let remaining = remaining % SECS_PER_HOUR;
        let minutes = remaining / SECS_PER_MINUTE;
        let seconds = remaining % SECS_PER_MINUTE;

        // Calculate year and day of year
        let mut year = 1970i64;
        let mut remaining_days = days;

        loop {
            let days_in_year = if Self::is_leap_year(year) { 366 } else { 365 };
            if remaining_days < days_in_year {
                break;
            }
            remaining_days -= days_in_year;
            year += 1;
        }

        // Simplified month/day calculation (approximate)
        let month = (remaining_days / 30).min(11) + 1;
        let day = (remaining_days % 30).min(30) + 1;

        Some(format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            year, month, day, hours, minutes, seconds
        ))
    }

    /// Checks if a year is a leap year
    fn is_leap_year(year: i64) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }
}

impl FormatParser for OTFParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid OTF signature"));
        }

        let mut metadata = MetadataMap::new();

        metadata.insert("FileType".to_string(), TagValue::String("OTF".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );

        let num_tables = Self::read_num_tables(reader)?;
        metadata.insert(
            "NumTables".to_string(),
            TagValue::String(num_tables.to_string()),
        );

        // Parse table directory
        let tables = Self::parse_table_directory(reader, num_tables)?;

        // Check for CFF table (OTF-specific)
        if Self::find_table(&tables, b"CFF ").is_some() {
            metadata.insert(
                "OutlineFormat".to_string(),
                TagValue::String("CFF".to_string()),
            );
        }

        // Parse name table
        if let Some(name_table) = Self::find_table(&tables, b"name") {
            if let Err(e) = Self::parse_name_table(reader, name_table, &mut metadata) {
                // Log error but continue parsing
                eprintln!("Warning: Failed to parse name table: {}", e);
            }
        }

        // Parse head table
        if let Some(head_table) = Self::find_table(&tables, b"head") {
            if let Err(e) = Self::parse_head_table(reader, head_table, &mut metadata) {
                eprintln!("Warning: Failed to parse head table: {}", e);
            }
        }

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::OTF)
    }
}

/// Parses metadata from OTF files.
///
/// This is a convenience wrapper around OTFParser that provides a functional API.
pub fn parse_otf_metadata(reader: &dyn FileReader) -> std::result::Result<MetadataMap, String> {
    let parser = OTFParser;
    parser.parse(reader).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    struct TestReader {
        data: Vec<u8>,
    }

    impl TestReader {
        fn new(data: Vec<u8>) -> Self {
            Self { data }
        }
    }

    impl FileReader for TestReader {
        fn read(&self, offset: u64, length: usize) -> io::Result<&[u8]> {
            let start = offset as usize;
            let end = start.saturating_add(length).min(self.data.len());
            if start > self.data.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "offset beyond end",
                ));
            }
            Ok(&self.data[start..end])
        }

        fn size(&self) -> u64 {
            self.data.len() as u64
        }
    }

    #[test]
    fn test_otf_signature() {
        let mut data = b"OTTO".to_vec();
        data.extend_from_slice(&[0x00, 0x10]);
        let reader = TestReader::new(data);
        assert!(OTFParser::verify_signature(&reader).unwrap());
    }

    #[test]
    fn test_otf_basic_metadata() {
        // Create minimal OTF structure
        let mut data = Vec::new();

        // SFNT header
        data.extend_from_slice(b"OTTO"); // signature
        data.extend_from_slice(&[0x00, 0x02]); // numTables = 2
        data.extend_from_slice(&[0x00, 0x20]); // searchRange
        data.extend_from_slice(&[0x00, 0x01]); // entrySelector
        data.extend_from_slice(&[0x00, 0x00]); // rangeShift

        // Table directory
        // CFF table entry
        data.extend_from_slice(b"CFF "); // tag
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // checksum
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x64]); // offset = 100
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x10]); // length = 16

        // head table entry
        data.extend_from_slice(b"head"); // tag
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // checksum
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x80]); // offset = 128
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x36]); // length = 54

        let reader = TestReader::new(data);
        let parser = OTFParser;
        let metadata = parser.parse(&reader).unwrap();

        assert_eq!(metadata.get("FileType"), Some(&TagValue::String("OTF".to_string())));
        assert_eq!(metadata.get("NumTables"), Some(&TagValue::String("2".to_string())));
        assert_eq!(metadata.get("OutlineFormat"), Some(&TagValue::String("CFF".to_string())));
    }
}
