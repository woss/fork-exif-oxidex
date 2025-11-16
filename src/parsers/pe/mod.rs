//! PE (Portable Executable) format parser
//!
//! This module provides parsing for PE files (.exe, .dll, .sys),
//! extracting metadata from DOS headers, COFF headers, Optional headers,
//! and section information.

#![allow(dead_code)]

pub mod structures;
pub mod dos_parser;
pub mod coff_parser;
pub mod optional_parser;
pub mod metadata_extractor;

use crate::core::{FileReader, MetadataMap};
use crate::error::{ExifToolError, Result};

/// PE signature/magic bytes
const PE_SIGNATURE: &[u8] = b"PE\0\0";
const DOS_SIGNATURE: &[u8] = b"MZ";

/// Parses PE file and extracts all metadata.
pub fn parse_pe_metadata(reader: &dyn FileReader) -> Result<MetadataMap> {
    use dos_parser::parse_dos_header;
    use coff_parser::parse_coff_header;
    use optional_parser::{parse_optional_header_standard, parse_optional_header_nt};
    use metadata_extractor::{extract_dos_metadata, extract_coff_metadata, extract_optional_metadata};

    let mut metadata = MetadataMap::new();

    // Read DOS header (first 64 bytes)
    let dos_data = reader.read(0, 64)?;
    let (_, dos_header) = parse_dos_header(dos_data)
        .map_err(|e| ExifToolError::parse_error(format!("Failed to parse DOS header: {:?}", e)))?;

    // Verify DOS signature
    if dos_header.e_magic != 0x5A4D {
        return Err(ExifToolError::parse_error(
            "Invalid DOS signature (expected MZ)",
        ));
    }

    extract_dos_metadata(&dos_header, &mut metadata);

    // Read COFF header at e_lfanew offset
    let pe_offset = dos_header.e_lfanew as u64;
    let coff_data = reader.read(pe_offset, 24)?; // PE signature (4) + COFF header (20)
    let (remaining, coff_header) = parse_coff_header(coff_data)
        .map_err(|e| ExifToolError::parse_error(format!("Failed to parse COFF header: {:?}", e)))?;

    extract_coff_metadata(&coff_header, &mut metadata);

    // Parse Optional Header if present
    if coff_header.size_of_optional_header > 0 {
        let opt_size = coff_header.size_of_optional_header as usize;
        let opt_data = &remaining[..opt_size.min(remaining.len())];

        let (opt_remaining, std_header) = parse_optional_header_standard(opt_data)
            .map_err(|e| ExifToolError::parse_error(format!("Failed to parse Optional Header: {:?}", e)))?;

        let is_pe32_plus = std_header.magic == 0x020B;
        let (_, nt_header) = parse_optional_header_nt(opt_remaining, is_pe32_plus)
            .map_err(|e| ExifToolError::parse_error(format!("Failed to parse Optional Header NT fields: {:?}", e)))?;

        extract_optional_metadata(&std_header, &nt_header, &mut metadata);
    }

    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pe_metadata_placeholder() {
        // This test will be replaced with real tests
        let metadata = MetadataMap::new();
        assert_eq!(metadata.len(), 0);
    }
}
