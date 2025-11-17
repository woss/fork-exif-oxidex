//! Windows Shortcut (LNK) format parser
//!
//! Implements basic metadata extraction from Windows shortcut files (.lnk).
//! LNK files are binary files created by Windows to point to other files or directories.
//!
//! # Format Structure
//!
//! LNK files begin with a 76-byte header:
//! - Bytes 0-3: Magic number (0x4C 0x00 0x00 0x00) - "L" in little-endian
//! - Bytes 4-19: GUID for the Shell Link class
//! - Bytes 20-23: Flags indicating optional structures
//! - Bytes 24-27: File attributes of the target
//!
//! # References
//!
//! - Microsoft Shell Link (.LNK) Binary File Format Specification
//! - [MS-SHLLINK]: Shell Link (.LNK) Binary File Format

#![allow(dead_code)]

use crate::core::{FileFormat, FileReader, FormatParser, MetadataMap, TagValue};
use crate::error::{ExifToolError, Result};

/// LNK signature: 0x4C 0x00 0x00 0x00 (magic number)
/// This is the little-endian representation of 0x0000004C
const LNK_MAGIC: &[u8] = &[0x4C, 0x00, 0x00, 0x00];

/// Expected GUID for Shell Link class ID
/// {00021401-0000-0000-C000-000000000046}
const SHELL_LINK_GUID: &[u8] = &[
    0x01, 0x14, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
];

/// Minimum LNK header size (76 bytes)
const LNK_HEADER_SIZE: usize = 76;

/// Windows Shortcut (LNK) parser for extracting metadata from .lnk files
pub struct LNKParser;

impl LNKParser {
    /// Verifies LNK signature by checking magic number and GUID
    ///
    /// # Arguments
    ///
    /// * `reader` - File reader providing access to the LNK file
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Valid LNK signature detected
    /// * `Ok(false)` - Invalid or missing signature
    /// * `Err` - I/O error reading the file
    pub fn verify_signature(reader: &dyn FileReader) -> Result<bool> {
        // Check file is large enough for header
        if reader.size() < LNK_HEADER_SIZE as u64 {
            return Ok(false);
        }

        // Check magic number (bytes 0-3)
        let magic = reader.read(0, 4)?;
        if magic != LNK_MAGIC {
            return Ok(false);
        }

        // Check GUID (bytes 4-19) for Shell Link class ID
        let guid = reader.read(4, 16)?;
        Ok(guid == SHELL_LINK_GUID)
    }

    /// Reads link flags from the header
    ///
    /// Flags indicate which optional structures are present in the file.
    /// Located at offset 20, 4 bytes, little-endian.
    ///
    /// # Arguments
    ///
    /// * `reader` - File reader providing access to the LNK file
    ///
    /// # Returns
    ///
    /// * `Ok(flags)` - Link flags as u32
    /// * `Err` - I/O error or file too small
    fn read_link_flags(reader: &dyn FileReader) -> Result<u32> {
        if reader.size() < 24 {
            return Ok(0);
        }
        let flags_bytes = reader.read(20, 4)?;
        Ok(u32::from_le_bytes([
            flags_bytes[0],
            flags_bytes[1],
            flags_bytes[2],
            flags_bytes[3],
        ]))
    }

    /// Reads file attributes from the header
    ///
    /// File attributes of the link target.
    /// Located at offset 24, 4 bytes, little-endian.
    ///
    /// # Arguments
    ///
    /// * `reader` - File reader providing access to the LNK file
    ///
    /// # Returns
    ///
    /// * `Ok(attributes)` - File attributes as u32
    /// * `Err` - I/O error or file too small
    fn read_file_attributes(reader: &dyn FileReader) -> Result<u32> {
        if reader.size() < 28 {
            return Ok(0);
        }
        let attr_bytes = reader.read(24, 4)?;
        Ok(u32::from_le_bytes([
            attr_bytes[0],
            attr_bytes[1],
            attr_bytes[2],
            attr_bytes[3],
        ]))
    }

    /// Decodes file attributes into human-readable flags
    ///
    /// # Arguments
    ///
    /// * `attributes` - Raw file attributes bitmask
    ///
    /// # Returns
    ///
    /// Vector of attribute flag names
    fn decode_file_attributes(attributes: u32) -> Vec<&'static str> {
        let mut flags = Vec::new();

        if attributes & 0x0001 != 0 {
            flags.push("ReadOnly");
        }
        if attributes & 0x0002 != 0 {
            flags.push("Hidden");
        }
        if attributes & 0x0004 != 0 {
            flags.push("System");
        }
        if attributes & 0x0010 != 0 {
            flags.push("Directory");
        }
        if attributes & 0x0020 != 0 {
            flags.push("Archive");
        }
        if attributes & 0x0080 != 0 {
            flags.push("Normal");
        }
        if attributes & 0x0100 != 0 {
            flags.push("Temporary");
        }
        if attributes & 0x0800 != 0 {
            flags.push("Compressed");
        }
        if attributes & 0x1000 != 0 {
            flags.push("Offline");
        }
        if attributes & 0x2000 != 0 {
            flags.push("NotIndexed");
        }
        if attributes & 0x4000 != 0 {
            flags.push("Encrypted");
        }

        flags
    }
}

impl FormatParser for LNKParser {
    /// Parses metadata from a Windows shortcut (LNK) file
    ///
    /// # Arguments
    ///
    /// * `reader` - File reader providing access to the LNK file
    ///
    /// # Returns
    ///
    /// * `Ok(MetadataMap)` - Extracted metadata including file type, size, flags, and attributes
    /// * `Err(ExifToolError)` - Invalid signature or parse error
    fn parse(&self, reader: &dyn FileReader) -> Result<MetadataMap> {
        // Verify this is a valid LNK file
        if !Self::verify_signature(reader)? {
            return Err(ExifToolError::parse_error("Invalid LNK signature"));
        }

        let mut metadata = MetadataMap::new();

        // Basic file information
        metadata.insert("FileType".to_string(), TagValue::String("LNK".to_string()));
        metadata.insert(
            "FileSize".to_string(),
            TagValue::String(reader.size().to_string()),
        );

        // Read and store link flags
        let link_flags = Self::read_link_flags(reader)?;
        metadata.insert(
            "LinkFlags".to_string(),
            TagValue::String(format!("0x{:08X}", link_flags)),
        );

        // Read and decode file attributes
        let file_attributes = Self::read_file_attributes(reader)?;
        metadata.insert(
            "FileAttributes".to_string(),
            TagValue::String(format!("0x{:08X}", file_attributes)),
        );

        // Decode file attributes into readable flags
        let attr_flags = Self::decode_file_attributes(file_attributes);
        if !attr_flags.is_empty() {
            metadata.insert(
                "TargetFileAttributes".to_string(),
                TagValue::String(attr_flags.join(", ")),
            );
        }

        // Check for common link flag bits
        let mut link_flags_desc = Vec::new();
        if link_flags & 0x0001 != 0 {
            link_flags_desc.push("HasLinkTargetIDList");
        }
        if link_flags & 0x0002 != 0 {
            link_flags_desc.push("HasLinkInfo");
        }
        if link_flags & 0x0004 != 0 {
            link_flags_desc.push("HasName");
        }
        if link_flags & 0x0008 != 0 {
            link_flags_desc.push("HasRelativePath");
        }
        if link_flags & 0x0010 != 0 {
            link_flags_desc.push("HasWorkingDir");
        }
        if link_flags & 0x0020 != 0 {
            link_flags_desc.push("HasArguments");
        }
        if link_flags & 0x0040 != 0 {
            link_flags_desc.push("HasIconLocation");
        }

        if !link_flags_desc.is_empty() {
            metadata.insert(
                "LinkFlagsDescription".to_string(),
                TagValue::String(link_flags_desc.join(", ")),
            );
        }

        Ok(metadata)
    }

    /// Checks if this parser supports the given format
    ///
    /// # Arguments
    ///
    /// * `format` - File format to check
    ///
    /// # Returns
    ///
    /// * `true` - Parser supports LNK format
    /// * `false` - Parser does not support the format
    fn supports_format(&self, format: FileFormat) -> bool {
        matches!(format, FileFormat::LNK)
    }
}

/// Parses metadata from Windows shortcut (LNK) files.
///
/// This is the public API function for parsing LNK files.
///
/// # Arguments
///
/// * `reader` - File reader providing access to the LNK file
///
/// # Returns
///
/// * `Ok(MetadataMap)` - Successfully extracted metadata
/// * `Err(String)` - Parse error message
///
/// # Examples
///
/// ```no_run
/// use oxidex::parsers::specialized::lnk::parse_lnk_metadata;
/// use oxidex::io::MMapReader;
/// use std::path::Path;
///
/// # fn example() -> Result<(), String> {
/// let reader = MMapReader::new(Path::new("shortcut.lnk"))
///     .map_err(|e| e.to_string())?;
/// let metadata = parse_lnk_metadata(&reader)?;
/// println!("LNK metadata: {:?}", metadata);
/// # Ok(())
/// # }
/// ```
pub fn parse_lnk_metadata(reader: &dyn FileReader) -> std::result::Result<MetadataMap, String> {
    let parser = LNKParser;
    parser.parse(reader).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    /// Test implementation of FileReader for unit testing
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
                    "offset beyond end of data",
                ));
            }

            Ok(&self.data[start..end])
        }

        fn size(&self) -> u64 {
            self.data.len() as u64
        }
    }

    #[test]
    fn test_verify_signature_valid() {
        // Create minimal valid LNK header (76 bytes)
        let mut data = vec![0u8; 76];

        // Magic number (0x4C 0x00 0x00 0x00)
        data[0..4].copy_from_slice(&[0x4C, 0x00, 0x00, 0x00]);

        // Shell Link GUID
        data[4..20].copy_from_slice(&[
            0x01, 0x14, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
            0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
        ]);

        let reader = TestReader::new(data);
        assert!(LNKParser::verify_signature(&reader).unwrap());
    }

    #[test]
    fn test_verify_signature_invalid_magic() {
        let mut data = vec![0u8; 76];
        data[0..4].copy_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Wrong magic

        let reader = TestReader::new(data);
        assert!(!LNKParser::verify_signature(&reader).unwrap());
    }

    #[test]
    fn test_verify_signature_too_small() {
        let data = vec![0x4C, 0x00, 0x00, 0x00]; // Only magic, no GUID

        let reader = TestReader::new(data);
        assert!(!LNKParser::verify_signature(&reader).unwrap());
    }

    #[test]
    fn test_parse_valid_lnk() {
        let mut data = vec![0u8; 76];

        // Magic number
        data[0..4].copy_from_slice(&[0x4C, 0x00, 0x00, 0x00]);

        // Shell Link GUID
        data[4..20].copy_from_slice(&[
            0x01, 0x14, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
            0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
        ]);

        // Link flags (0x0001 - HasLinkTargetIDList)
        data[20..24].copy_from_slice(&[0x01, 0x00, 0x00, 0x00]);

        // File attributes (0x0020 - Archive)
        data[24..28].copy_from_slice(&[0x20, 0x00, 0x00, 0x00]);

        let reader = TestReader::new(data);
        let parser = LNKParser;
        let metadata = parser.parse(&reader).unwrap();

        assert_eq!(
            metadata.get("FileType"),
            Some(&TagValue::String("LNK".to_string()))
        );
        assert!(metadata.contains_key("LinkFlags"));
        assert!(metadata.contains_key("FileAttributes"));
    }

    #[test]
    fn test_decode_file_attributes() {
        // Test various attribute combinations
        let attrs = LNKParser::decode_file_attributes(0x0021); // ReadOnly + Archive
        assert!(attrs.contains(&"ReadOnly"));
        assert!(attrs.contains(&"Archive"));

        let attrs = LNKParser::decode_file_attributes(0x0010); // Directory
        assert!(attrs.contains(&"Directory"));
        assert_eq!(attrs.len(), 1);
    }
}
