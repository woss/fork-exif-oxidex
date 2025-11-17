//! PE (Portable Executable) format parser
//!
//! This module provides parsing for PE files (.exe, .dll, .sys),
//! extracting metadata from DOS headers, COFF headers, Optional headers,
//! and section information.

#![allow(dead_code)]

pub mod coff_parser;
pub mod debug_parser;
pub mod dos_parser;
pub mod metadata_extractor;
pub mod optional_parser;
pub mod resource_parser;
pub mod section_parser;
pub mod structures;
pub mod version_info_parser;

use crate::core::{FileReader, MetadataMap};
use crate::error::{ExifToolError, Result};

/// PE signature/magic bytes
const PE_SIGNATURE: &[u8] = b"PE\0\0";
const DOS_SIGNATURE: &[u8] = b"MZ";

/// Parses PE file and extracts all metadata.
pub fn parse_pe_metadata(reader: &dyn FileReader) -> Result<MetadataMap> {
    use coff_parser::parse_coff_header;
    use debug_parser::parse_debug_directory_entry;
    use dos_parser::parse_dos_header;
    use metadata_extractor::{
        extract_coff_metadata, extract_dos_metadata, extract_nb10_metadata,
        extract_optional_metadata, extract_rsds_metadata, extract_version_info_metadata,
    };
    use optional_parser::{parse_optional_header_nt, parse_optional_header_standard};
    use resource_parser::find_resource_data;
    use section_parser::parse_section_table;
    use structures::{debug_types, resource_types};
    use version_info_parser::parse_version_info;

    let mut metadata = MetadataMap::new();

    // Step 1: Parse DOS header (first 64 bytes)
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

    // Step 2: Parse COFF header at e_lfanew offset
    // We need to read PE signature (4) + COFF header (20) + Optional header (variable)
    // Read up to 512 bytes to cover typical optional headers
    let pe_offset = dos_header.e_lfanew as u64;
    let pe_data = reader.read(pe_offset, 512)?;
    let (remaining, coff_header) = parse_coff_header(pe_data)
        .map_err(|e| ExifToolError::parse_error(format!("Failed to parse COFF header: {:?}", e)))?;

    extract_coff_metadata(&coff_header, &mut metadata);

    // Step 3: Parse Optional Header if present
    // Store NT header for later use (debug directory parsing)
    let resource_dir_rva: Option<u32>;
    let nt_header_opt: Option<structures::OptionalHeaderNT>;
    if coff_header.size_of_optional_header > 0 {
        let (opt_remaining, std_header) =
            parse_optional_header_standard(remaining).map_err(|e| {
                ExifToolError::parse_error(format!("Failed to parse Optional Header: {:?}", e))
            })?;

        let is_pe32_plus = std_header.magic == 0x020B;
        let (_, nt_header) =
            parse_optional_header_nt(opt_remaining, is_pe32_plus).map_err(|e| {
                ExifToolError::parse_error(format!(
                    "Failed to parse Optional Header NT fields: {:?}",
                    e
                ))
            })?;

        extract_optional_metadata(&std_header, &nt_header, &mut metadata);

        // Extract resource directory RVA from data directories (index 2)
        resource_dir_rva = nt_header.data_directories.get(2).map(|(rva, _size)| *rva);

        // Store nt_header for debug directory parsing
        nt_header_opt = Some(nt_header);
    } else {
        resource_dir_rva = None;
        nt_header_opt = None;
    }

    // Step 4: Calculate section table offset
    // Section table starts after PE signature + COFF header + Optional header
    // PE signature = 4 bytes, COFF header = 20 bytes
    let section_table_offset = pe_offset + 4 + 20 + coff_header.size_of_optional_header as u64;

    // Step 5: Parse section table
    let section_table_size = (coff_header.number_of_sections as u64) * 40; // Each section header is 40 bytes
    let section_data = reader.read(section_table_offset, section_table_size as usize)?;
    let (_, sections) =
        parse_section_table(section_data, coff_header.number_of_sections).map_err(|e| {
            ExifToolError::parse_error(format!("Failed to parse section table: {:?}", e))
        })?;

    // Step 6: Find .rsrc section
    if let Some(_resource_dir_rva) = resource_dir_rva {
        if let Some(rsrc_section) = sections.iter().find(|s| s.name_str() == ".rsrc") {
            // Step 7: Read resource section data
            let rsrc_file_offset = rsrc_section.pointer_to_raw_data as u64;
            let rsrc_size = rsrc_section.size_of_raw_data as usize;
            let rsrc_data = reader.read(rsrc_file_offset, rsrc_size)?;

            // Step 8: Find VERSION_INFO resource (type 16)
            if let Some((version_rva, version_size)) = find_resource_data(
                rsrc_data,
                rsrc_file_offset,
                resource_types::RT_VERSION,
                None,
            ) {
                // Convert RVA to file offset
                // RVA is relative to image base, need to convert using section info
                let version_offset_in_section = version_rva - rsrc_section.virtual_address as u64;
                let version_file_offset = rsrc_file_offset + version_offset_in_section;

                // Step 9: Read and parse VERSION_INFO data
                let version_data = reader.read(version_file_offset, version_size as usize)?;
                if let Some((fixed_info, strings)) = parse_version_info(version_data) {
                    // Step 10: Extract VERSION_INFO metadata
                    extract_version_info_metadata(&fixed_info, &strings, &mut metadata);
                }
            }
        }
    }

    // Step 11: Parse debug directory if present (data directory index 6)
    if let Some(ref nt_header) = nt_header_opt {
        if let Some(&(debug_rva, debug_size)) = nt_header.data_directories.get(6) {
            if debug_rva > 0 && debug_size > 0 {
                // Find section containing debug directory
                if let Some(debug_section) = sections.iter().find(|s| {
                    debug_rva >= s.virtual_address && debug_rva < s.virtual_address + s.virtual_size
                }) {
                    let debug_offset = debug_section.pointer_to_raw_data as u64
                        + (debug_rva - debug_section.virtual_address) as u64;

                    let debug_data = reader.read(debug_offset, debug_size as usize)?;

                    // Parse debug directory entries
                    let mut offset = 0;
                    while offset + 28 <= debug_size as usize {
                        if let Ok((_, entry)) = parse_debug_directory_entry(&debug_data[offset..]) {
                            // Check for CodeView debug info
                            if entry.debug_type == debug_types::IMAGE_DEBUG_TYPE_CODEVIEW
                                && entry.pointer_to_raw_data > 0
                                && entry.size_of_data > 0
                            {
                                let cv_data = reader.read(
                                    entry.pointer_to_raw_data as u64,
                                    entry.size_of_data as usize,
                                )?;

                                use debug_parser::{parse_codeview_nb10, parse_codeview_rsds};

                                // Try RSDS first (newer format)
                                if let Some(rsds) = parse_codeview_rsds(cv_data) {
                                    extract_rsds_metadata(&rsds, &mut metadata);
                                } else if let Some(nb10) = parse_codeview_nb10(cv_data) {
                                    extract_nb10_metadata(&nb10, &mut metadata);
                                }
                                break;
                            }
                            offset += 28;
                        } else {
                            break;
                        }
                    }
                }
            }
        }
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
