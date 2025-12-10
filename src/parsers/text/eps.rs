//! EPS (Encapsulated PostScript) parser
//!
//! Parses EPS files to extract metadata from:
//! - PostScript DSC (Document Structuring Convention) comments
//! - Embedded XMP data
//! - Embedded IPTC data (via Photoshop 8BIM blocks)

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};
use crate::parsers::jpeg::iptc_parser::{
    dataset_to_tag_name, decode_iptc_string, parse_all_iptc_records,
};
use crate::parsers::xmp::{parse_xmp, parse_xmp_history};

/// Maximum bytes to read from EPS file for parsing
const MAX_READ_SIZE: usize = 1024 * 1024; // 1MB

/// Parser for EPS (Encapsulated PostScript) files
///
/// Extracts metadata from EPS files including:
/// - PostScript DSC comments (BoundingBox, Creator, Title, etc.)
/// - Embedded XMP metadata
/// - Embedded IPTC metadata (via Photoshop 8BIM resource blocks)
pub struct EPSParser;

impl EPSParser {
    /// Verifies the EPS file by checking for the PostScript signature
    pub fn verify_signature(data: &[u8]) -> bool {
        // Check for ASCII EPS: %!PS-Adobe
        if data.starts_with(b"%!PS-Adobe") {
            return true;
        }

        // Check for binary EPS (DOS EPS): 0xC5D0D3C6 magic
        if data.len() >= 4
            && data[0] == 0xC5
            && data[1] == 0xD0
            && data[2] == 0xD3
            && data[3] == 0xC6
        {
            return true;
        }

        false
    }

    /// Extracts DSC (Document Structuring Convention) comments
    fn extract_dsc_comments(text: &str, metadata: &mut MetadataMap) {
        let mut version: Option<String> = None;
        let mut pages: Option<String> = None;

        for line in text.lines() {
            let line = line.trim();

            // DSC comments start with %%
            if !line.starts_with("%%") {
                continue;
            }

            // Parse specific DSC comments
            if let Some(value) = line.strip_prefix("%%BoundingBox:") {
                let value = value.trim();
                if value != "(atend)" {
                    metadata.insert(
                        "PostScript:BoundingBox".to_string(),
                        TagValue::String(value.to_string()),
                    );
                    // Also add EPS:BoundingBox for consistency with Worker 24 requirements
                    metadata.insert(
                        "EPS:BoundingBox".to_string(),
                        TagValue::new_string(value.to_string()),
                    );
                }
            } else if let Some(value) = line.strip_prefix("%%HiResBoundingBox:") {
                let value = value.trim();
                if value != "(atend)" {
                    metadata.insert(
                        "PostScript:HiResBoundingBox".to_string(),
                        TagValue::String(value.to_string()),
                    );
                }
            } else if let Some(value) = line.strip_prefix("%%Creator:") {
                let trimmed_value = value.trim().to_string();
                metadata.insert(
                    "PostScript:Creator".to_string(),
                    TagValue::String(trimmed_value.clone()),
                );
                // Add EPS:Creator as per Worker 24 requirements
                metadata.insert(
                    "EPS:Creator".to_string(),
                    TagValue::new_string(trimmed_value),
                );
            } else if let Some(value) = line.strip_prefix("%%CreationDate:") {
                let trimmed_value = value.trim().to_string();
                metadata.insert(
                    "PostScript:CreateDate".to_string(),
                    TagValue::String(trimmed_value.clone()),
                );
                // Add EPS:CreationDate as per Worker 24 requirements
                metadata.insert(
                    "EPS:CreationDate".to_string(),
                    TagValue::new_string(trimmed_value),
                );
            } else if let Some(value) = line.strip_prefix("%%Title:") {
                // Remove surrounding parentheses if present
                let value = value.trim();
                let value = if value.starts_with('(') && value.ends_with(')') {
                    &value[1..value.len() - 1]
                } else {
                    value
                };
                let value_str = value.to_string();
                metadata.insert(
                    "PostScript:Title".to_string(),
                    TagValue::String(value_str.clone()),
                );
                // Add EPS:Title as per Worker 24 requirements
                metadata.insert("EPS:Title".to_string(), TagValue::new_string(value_str));
            } else if let Some(value) = line.strip_prefix("%%For:") {
                let trimmed_value = value.trim().to_string();
                metadata.insert(
                    "PostScript:For".to_string(),
                    TagValue::String(trimmed_value.clone()),
                );
                // Add EPS:For as per Worker 24 requirements
                metadata.insert("EPS:For".to_string(), TagValue::new_string(trimmed_value));
            } else if let Some(value) = line.strip_prefix("%%DocumentData:") {
                metadata.insert(
                    "PostScript:DocumentData".to_string(),
                    TagValue::String(value.trim().to_string()),
                );
            } else if let Some(value) = line.strip_prefix("%%LanguageLevel:") {
                metadata.insert(
                    "PostScript:LanguageLevel".to_string(),
                    TagValue::String(value.trim().to_string()),
                );
            } else if let Some(value) = line.strip_prefix("%%Pages:") {
                let value = value.trim();
                if value != "(atend)" {
                    pages = Some(value.to_string());
                    metadata.insert(
                        "PostScript:Pages".to_string(),
                        TagValue::String(value.to_string()),
                    );
                }
            } else if let Some(value) = line.strip_prefix("%%ImageData:") {
                metadata.insert(
                    "PostScript:ImageData".to_string(),
                    TagValue::String(value.trim().to_string()),
                );
            }

            // Extract version from first line %!PS-Adobe-X.X EPSF-X.X
            if line.starts_with("%!PS-Adobe") && version.is_none() {
                if let Some(version_str) = extract_eps_version_from_header(line) {
                    version = Some(version_str);
                }
            }
        }

        // Add EPS:Version if extracted from header
        if let Some(v) = version {
            metadata.insert("EPS:Version".to_string(), TagValue::new_string(v));
        }

        // Add EPS:Pages as integer if available
        if let Some(pages_str) = pages {
            if let Ok(pages_int) = pages_str.parse::<i64>() {
                metadata.insert("EPS:Pages".to_string(), TagValue::new_integer(pages_int));
            }
        }

        // EPS:Orientation is typically not in DSC comments, but we can try to infer from BoundingBox
        // For now, we'll leave this for future enhancement
    }

    /// Extracts XMP metadata from EPS data
    fn extract_xmp(data: &[u8], metadata: &mut MetadataMap) {
        // Search for XMP packet markers
        const XMP_BEGIN: &[u8] = b"<?xpacket begin=";
        const XMP_END: &[u8] = b"<?xpacket end=";

        if let Some(begin_pos) = find_subsequence(data, XMP_BEGIN) {
            // Find the end of the xpacket processing instruction
            let after_begin = &data[begin_pos..];
            if let Some(xml_start_offset) = find_subsequence(after_begin, b"?>") {
                let xml_start_pos = begin_pos + xml_start_offset + 2; // +2 to skip ?>

                // Find XMP end marker
                if let Some(end_offset) = find_subsequence(&data[xml_start_pos..], XMP_END) {
                    let xmp_data = &data[xml_start_pos..xml_start_pos + end_offset];

                    // Parse the XMP data
                    if let Ok(xmp_tags) = parse_xmp(xmp_data) {
                        for (key, value) in xmp_tags {
                            metadata.insert(key, TagValue::new_string(value));
                        }
                    }

                    // Parse XMP history for forensic metadata
                    if let Ok(xml_str) = std::str::from_utf8(xmp_data) {
                        if let Ok(history_tags) = parse_xmp_history(xml_str) {
                            for (key, value) in history_tags {
                                metadata.insert(key, TagValue::new_string(value));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Extracts IPTC metadata from Photoshop 8BIM blocks in EPS data
    fn extract_iptc(data: &[u8], metadata: &mut MetadataMap) {
        // Search for Photoshop 8BIM signature
        const EIGHTBIM: &[u8] = b"8BIM";
        const IPTC_RESOURCE_ID: u16 = 0x0404;

        let mut pos = 0;
        while pos + 12 < data.len() {
            // Search for next 8BIM block
            if let Some(block_pos) = find_subsequence(&data[pos..], EIGHTBIM) {
                let abs_pos = pos + block_pos;

                // Verify we have enough data for the header
                if abs_pos + 12 > data.len() {
                    break;
                }

                // Parse resource ID (2 bytes after signature)
                let id = u16::from_be_bytes([data[abs_pos + 4], data[abs_pos + 5]]);

                // Parse Pascal string name length
                let name_len = data[abs_pos + 6] as usize;

                // Calculate padding for name (must be even total length)
                let total_name_len = 1 + name_len; // 1 for length byte
                let padding = if total_name_len % 2 == 1 { 1 } else { 0 };

                // Calculate data offset
                let data_offset = abs_pos + 7 + name_len + padding;

                // Verify we have enough data for size field
                if data_offset + 4 > data.len() {
                    pos = abs_pos + 1;
                    continue;
                }

                // Parse data size (4 bytes)
                let data_size = u32::from_be_bytes([
                    data[data_offset],
                    data[data_offset + 1],
                    data[data_offset + 2],
                    data[data_offset + 3],
                ]) as usize;

                let data_start = data_offset + 4;

                // Check if this is an IPTC block
                if id == IPTC_RESOURCE_ID && data_start + data_size <= data.len() && data_size > 0 {
                    let iptc_data = &data[data_start..data_start + data_size];

                    // Parse IPTC records
                    if let Ok(records) = parse_all_iptc_records(iptc_data) {
                        for record in records {
                            let tag_name =
                                dataset_to_tag_name(record.record_number, record.dataset_number);
                            let value = decode_iptc_string(&record.data);
                            metadata.insert(tag_name, TagValue::String(value));
                        }
                    }
                }

                // Move past this block
                pos = data_start + data_size;
            } else {
                // No more 8BIM blocks found
                break;
            }
        }

        // Also extract IPTCDigest from Photoshop resources
        // Resource ID 0x0425 contains IPTC digest
        const IPTC_DIGEST_RESOURCE_ID: u16 = 0x0425;
        pos = 0;
        while pos + 12 < data.len() {
            if let Some(block_pos) = find_subsequence(&data[pos..], EIGHTBIM) {
                let abs_pos = pos + block_pos;

                if abs_pos + 12 > data.len() {
                    break;
                }

                let id = u16::from_be_bytes([data[abs_pos + 4], data[abs_pos + 5]]);
                let name_len = data[abs_pos + 6] as usize;
                let total_name_len = 1 + name_len;
                let padding = if total_name_len % 2 == 1 { 1 } else { 0 };
                let data_offset = abs_pos + 7 + name_len + padding;

                if data_offset + 4 > data.len() {
                    pos = abs_pos + 1;
                    continue;
                }

                let data_size = u32::from_be_bytes([
                    data[data_offset],
                    data[data_offset + 1],
                    data[data_offset + 2],
                    data[data_offset + 3],
                ]) as usize;

                let data_start = data_offset + 4;

                if id == IPTC_DIGEST_RESOURCE_ID
                    && data_start + data_size <= data.len()
                    && data_size == 16
                {
                    // IPTC digest is a 16-byte MD5 hash
                    let digest_data = &data[data_start..data_start + data_size];
                    let digest_hex: String =
                        digest_data.iter().map(|b| format!("{:02x}", b)).collect();
                    metadata.insert(
                        "Photoshop:IPTCDigest".to_string(),
                        TagValue::String(digest_hex),
                    );
                }

                pos = data_start + data_size;
            } else {
                break;
            }
        }
    }
}

impl FormatParser for EPSParser {
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        // Read the file data
        let file_size = reader.size() as usize;
        let read_size = file_size.min(MAX_READ_SIZE);
        let data = reader.read(0, read_size)?;

        // Verify EPS signature
        if !Self::verify_signature(data) {
            return Err(ExifToolError::parse_error("Invalid EPS signature"));
        }

        let mut metadata = MetadataMap::new();

        // Set basic file info
        metadata.insert("FileType".to_string(), TagValue::String("EPS".to_string()));
        metadata.insert("FileSize".to_string(), TagValue::Integer(file_size as i64));
        metadata.insert(
            "MIMEType".to_string(),
            TagValue::String("application/postscript".to_string()),
        );

        // Handle binary EPS (DOS EPS) header
        let ps_data = if data.starts_with(&[0xC5, 0xD0, 0xD3, 0xC6]) && data.len() >= 30 {
            // Binary EPS header contains offsets to the PostScript section
            let ps_start = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
            let ps_length = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;

            if ps_start < data.len() && ps_start + ps_length <= data.len() {
                &data[ps_start..ps_start + ps_length]
            } else {
                data
            }
        } else {
            data
        };

        // Convert to text for DSC comment parsing
        if let Ok(text) = std::str::from_utf8(ps_data) {
            Self::extract_dsc_comments(text, &mut metadata);
        } else {
            // Try to find ASCII portions for DSC parsing
            // Some EPS files have mixed binary/text content
            let text: String = ps_data.iter().map(|&b| b as char).collect();
            Self::extract_dsc_comments(&text, &mut metadata);
        }

        // Extract XMP metadata
        Self::extract_xmp(data, &mut metadata);

        // Extract IPTC metadata
        Self::extract_iptc(data, &mut metadata);

        Ok(metadata)
    }

    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::EPS)
    }
}

/// Extracts EPS version from the header line
/// Example: "%!PS-Adobe-3.0 EPSF-3.0" returns "3.0"
fn extract_eps_version_from_header(header: &str) -> Option<String> {
    // Look for EPSF-X.X pattern
    if let Some(epsf_pos) = header.find("EPSF-") {
        let after_epsf = &header[epsf_pos + 5..];
        // Extract version number (typically X.X)
        let version: String = after_epsf
            .chars()
            .take_while(|c| c.is_numeric() || *c == '.')
            .collect();
        if !version.is_empty() {
            return Some(version);
        }
    }
    // Fallback: look for PS-Adobe-X.X pattern
    if let Some(ps_pos) = header.find("PS-Adobe-") {
        let after_ps = &header[ps_pos + 9..];
        let version: String = after_ps
            .chars()
            .take_while(|c| c.is_numeric() || *c == '.')
            .collect();
        if !version.is_empty() {
            return Some(version);
        }
    }
    None
}

/// Finds a subsequence in a byte slice
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Parses metadata from EPS files.
///
/// This is a convenience wrapper around EPSParser that provides a functional API.
pub fn parse_eps_metadata(reader: &dyn FileReader) -> std::result::Result<MetadataMap, String> {
    let parser = EPSParser;
    parser.parse(reader).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::BufferedReader;

    #[test]
    fn test_eps_ascii_signature() {
        let eps_data = b"%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 100 100\n";
        assert!(EPSParser::verify_signature(eps_data));
    }

    #[test]
    fn test_eps_binary_signature() {
        let mut eps_data = vec![0xC5, 0xD0, 0xD3, 0xC6]; // DOS EPS magic
        eps_data.extend_from_slice(&[0; 26]); // Padding
        assert!(EPSParser::verify_signature(&eps_data));
    }

    #[test]
    fn test_eps_dsc_parsing() {
        let eps_data = br#"%!PS-Adobe-3.0 EPSF-3.0
%%Creator: Test Creator
%%Title: (Test Title)
%%CreationDate: 2024/01/01
%%BoundingBox: 0 0 100 200
%%EndComments
"#;

        let reader = BufferedReader::from_bytes(eps_data);
        let parser = EPSParser;
        let metadata = parser.parse(&reader).unwrap();

        assert_eq!(metadata.get("FileType").unwrap().as_string(), Some("EPS"));
        assert_eq!(
            metadata.get("PostScript:Creator").unwrap().as_string(),
            Some("Test Creator")
        );
        assert_eq!(
            metadata.get("PostScript:Title").unwrap().as_string(),
            Some("Test Title")
        );
        assert_eq!(
            metadata.get("PostScript:CreateDate").unwrap().as_string(),
            Some("2024/01/01")
        );
        assert_eq!(
            metadata.get("PostScript:BoundingBox").unwrap().as_string(),
            Some("0 0 100 200")
        );
    }

    #[test]
    fn test_eps_invalid() {
        let invalid_data = b"Not an EPS file";
        let reader = BufferedReader::from_bytes(invalid_data);
        let parser = EPSParser;

        let result = parser.parse(&reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_subsequence() {
        let data = b"Hello World";
        assert_eq!(find_subsequence(data, b"World"), Some(6));
        assert_eq!(find_subsequence(data, b"Foo"), None);
    }
}
