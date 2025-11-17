//! Metadata extraction from PE headers

use std::collections::HashMap;

use crate::core::{MetadataMap, TagValue};
use crate::parsers::pe::structures::{
    machine_types, subsystem_types, CodeViewNB10, CodeViewRSDS, CoffHeader, DosHeader,
    OptionalHeaderNT, OptionalHeaderStandard, VsFixedFileInfo,
};

/// Extract metadata from DOS header
pub fn extract_dos_metadata(header: &DosHeader, metadata: &mut MetadataMap) {
    metadata.insert(
        "PE:DOSSignature".to_string(),
        TagValue::String(format!("{:#06X}", header.e_magic)),
    );
    metadata.insert(
        "PE:PEHeaderOffset".to_string(),
        TagValue::Integer(header.e_lfanew as i64),
    );
}

/// Extract metadata from COFF header
pub fn extract_coff_metadata(header: &CoffHeader, metadata: &mut MetadataMap) {
    // Machine type
    let machine_name = match header.machine {
        machine_types::IMAGE_FILE_MACHINE_I386 => "Intel 386",
        machine_types::IMAGE_FILE_MACHINE_AMD64 => "x64 (AMD64)",
        machine_types::IMAGE_FILE_MACHINE_ARM => "ARM",
        machine_types::IMAGE_FILE_MACHINE_ARM64 => "ARM64",
        machine_types::IMAGE_FILE_MACHINE_IA64 => "Intel Itanium",
        machine_types::IMAGE_FILE_MACHINE_POWERPC => "PowerPC",
        _ => "Unknown",
    };
    metadata.insert(
        "PE:MachineType".to_string(),
        TagValue::String(machine_name.to_string()),
    );
    metadata.insert(
        "PE:MachineTypeRaw".to_string(),
        TagValue::Integer(header.machine as i64),
    );

    // Number of sections
    metadata.insert(
        "PE:NumberOfSections".to_string(),
        TagValue::Integer(header.number_of_sections as i64),
    );

    // Timestamp (Unix epoch)
    if header.time_date_stamp > 0 {
        metadata.insert(
            "PE:TimeStamp".to_string(),
            TagValue::Integer(header.time_date_stamp as i64),
        );

        // Convert to human-readable date if possible
        use chrono::{TimeZone, Utc};
        if let Some(dt) = Utc.timestamp_opt(header.time_date_stamp as i64, 0).single() {
            metadata.insert(
                "PE:CompileTime".to_string(),
                TagValue::String(dt.format("%Y:%m:%d %H:%M:%S").to_string()),
            );
        }
    }

    // Characteristics
    metadata.insert(
        "PE:Characteristics".to_string(),
        TagValue::Integer(header.characteristics as i64),
    );

    // Decode characteristic bit flags into human-readable strings
    // Reference: Microsoft PE/COFF specification IMAGE_FILE_HEADER.Characteristics
    let mut flags = Vec::new();

    if (header.characteristics & 0x0001) != 0 {
        flags.push("No relocs");
    }
    if (header.characteristics & 0x0002) != 0 {
        flags.push("Executable");
    }
    if (header.characteristics & 0x0004) != 0 {
        flags.push("No line numbers");
    }
    if (header.characteristics & 0x0008) != 0 {
        flags.push("No symbols");
    }
    if (header.characteristics & 0x0020) != 0 {
        flags.push("Large address aware");
    }
    if (header.characteristics & 0x0100) != 0 {
        flags.push("32-bit");
    }
    if (header.characteristics & 0x0200) != 0 {
        flags.push("Bytes reversed lo");
    }
    if (header.characteristics & 0x1000) != 0 {
        flags.push("System file");
    }
    if (header.characteristics & 0x2000) != 0 {
        flags.push("DLL");
    }
    if (header.characteristics & 0x4000) != 0 {
        flags.push("Bytes reversed hi");
    }

    // Insert decoded characteristics as comma-separated string
    if !flags.is_empty() {
        metadata.insert(
            "PE:ImageFileCharacteristics".to_string(),
            TagValue::String(flags.join(", ")),
        );
    }

    // Decode common flags for FileType (kept for compatibility)
    let is_executable = (header.characteristics & 0x0002) != 0;
    let is_dll = (header.characteristics & 0x2000) != 0;
    let file_type = if is_dll {
        "DLL"
    } else if is_executable {
        "Executable"
    } else {
        "Object"
    };
    metadata.insert(
        "PE:FileType".to_string(),
        TagValue::String(file_type.to_string()),
    );
}

/// Extract metadata from Optional Header
pub fn extract_optional_metadata(
    std_header: &OptionalHeaderStandard,
    nt_header: &OptionalHeaderNT,
    metadata: &mut MetadataMap,
) {
    // Image format
    let image_format = match std_header.magic {
        0x010B => "PE32",
        0x020B => "PE32+",
        _ => "Unknown",
    };
    metadata.insert(
        "PE:ImageFormat".to_string(),
        TagValue::String(image_format.to_string()),
    );

    // PEType is an alias for ImageFormat (for ExifTool compatibility)
    metadata.insert(
        "PE:PEType".to_string(),
        TagValue::String(image_format.to_string()),
    );

    // Linker version
    metadata.insert(
        "PE:LinkerVersion".to_string(),
        TagValue::String(format!(
            "{}.{}",
            std_header.major_linker_version, std_header.minor_linker_version
        )),
    );

    // Code and data sizes
    metadata.insert(
        "PE:CodeSize".to_string(),
        TagValue::Integer(std_header.size_of_code as i64),
    );
    metadata.insert(
        "PE:InitializedDataSize".to_string(),
        TagValue::Integer(std_header.size_of_initialized_data as i64),
    );
    metadata.insert(
        "PE:UninitializedDataSize".to_string(),
        TagValue::Integer(std_header.size_of_uninitialized_data as i64),
    );

    // Entry point
    metadata.insert(
        "PE:EntryPoint".to_string(),
        TagValue::Integer(std_header.address_of_entry_point as i64),
    );

    // Image base
    metadata.insert(
        "PE:ImageBase".to_string(),
        TagValue::Integer(nt_header.image_base as i64),
    );

    // OS version
    metadata.insert(
        "PE:OSVersion".to_string(),
        TagValue::String(format!(
            "{}.{}",
            nt_header.major_operating_system_version, nt_header.minor_operating_system_version
        )),
    );

    // Image version
    metadata.insert(
        "PE:ImageVersion".to_string(),
        TagValue::String(format!(
            "{}.{}",
            nt_header.major_image_version, nt_header.minor_image_version
        )),
    );

    // Subsystem
    let subsystem_name = match nt_header.subsystem {
        subsystem_types::IMAGE_SUBSYSTEM_WINDOWS_GUI => "Windows GUI",
        subsystem_types::IMAGE_SUBSYSTEM_WINDOWS_CUI => "Windows Console",
        subsystem_types::IMAGE_SUBSYSTEM_NATIVE => "Native (Driver)",
        subsystem_types::IMAGE_SUBSYSTEM_EFI_APPLICATION => "EFI Application",
        _ => "Unknown",
    };
    metadata.insert(
        "PE:Subsystem".to_string(),
        TagValue::String(subsystem_name.to_string()),
    );
    metadata.insert(
        "PE:SubsystemRaw".to_string(),
        TagValue::Integer(nt_header.subsystem as i64),
    );

    // Subsystem version
    metadata.insert(
        "PE:SubsystemVersion".to_string(),
        TagValue::String(format!(
            "{}.{}",
            nt_header.major_subsystem_version, nt_header.minor_subsystem_version
        )),
    );

    // Checksum
    if nt_header.checksum != 0 {
        metadata.insert(
            "PE:Checksum".to_string(),
            TagValue::Integer(nt_header.checksum as i64),
        );
    }
}

/// Extract metadata from VERSION_INFO resource
pub fn extract_version_info_metadata(
    fixed_info: &VsFixedFileInfo,
    strings: &HashMap<String, String>,
    metadata: &mut MetadataMap,
) {
    // Fixed file info
    metadata.insert(
        "PE:FileVersionNumber".to_string(),
        TagValue::String(fixed_info.file_version()),
    );
    metadata.insert(
        "PE:ProductVersionNumber".to_string(),
        TagValue::String(fixed_info.product_version()),
    );
    metadata.insert(
        "PE:FileFlagsMask".to_string(),
        TagValue::String(format!("{:#06x}", fixed_info.file_flags_mask)),
    );

    let flags = fixed_info.file_flags_string();
    if !flags.is_empty() {
        metadata.insert(
            "PE:FileFlags".to_string(),
            TagValue::String(flags.join(", ")),
        );
    } else {
        metadata.insert(
            "PE:FileFlags".to_string(),
            TagValue::String("(none)".to_string()),
        );
    }

    metadata.insert(
        "PE:FileOS".to_string(),
        TagValue::String(fixed_info.file_os_string().to_string()),
    );
    metadata.insert(
        "PE:ObjectFileType".to_string(),
        TagValue::String(fixed_info.file_type_string().to_string()),
    );
    metadata.insert(
        "PE:FileSubtype".to_string(),
        TagValue::Integer(fixed_info.file_subtype as i64),
    );

    // String file info
    for (key, value) in strings {
        let tag_name = format!("PE:{}", key);
        metadata.insert(tag_name, TagValue::String(value.clone()));
    }
}

/// Extract metadata from CodeView RSDS debug info
pub fn extract_rsds_metadata(rsds: &CodeViewRSDS, metadata: &mut MetadataMap) {
    metadata.insert(
        "PE:PDBFileName".to_string(),
        TagValue::String(rsds.pdb_file_name.clone()),
    );
    metadata.insert("PE:PDBAge".to_string(), TagValue::Integer(rsds.age as i64));

    // Format GUID as string
    let guid_str = format!(
        "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        rsds.guid[3], rsds.guid[2], rsds.guid[1], rsds.guid[0],
        rsds.guid[5], rsds.guid[4],
        rsds.guid[7], rsds.guid[6],
        rsds.guid[8], rsds.guid[9],
        rsds.guid[10], rsds.guid[11], rsds.guid[12], rsds.guid[13], rsds.guid[14], rsds.guid[15]
    );
    metadata.insert("PE:PDBGUID".to_string(), TagValue::String(guid_str));
}

/// Extract metadata from CodeView NB10 debug info
pub fn extract_nb10_metadata(nb10: &CodeViewNB10, metadata: &mut MetadataMap) {
    metadata.insert(
        "PE:PDBFileName".to_string(),
        TagValue::String(nb10.pdb_file_name.clone()),
    );
    metadata.insert("PE:PDBAge".to_string(), TagValue::Integer(nb10.age as i64));

    // Convert timestamp to date
    use chrono::{TimeZone, Utc};
    if let Some(dt) = Utc.timestamp_opt(nb10.timestamp as i64, 0).single() {
        metadata.insert(
            "PE:PDBCreateDate".to_string(),
            TagValue::String(dt.format("%Y:%m:%d %H:%M:%S").to_string()),
        );
    }

    metadata.insert(
        "PE:PDBModifyDate".to_string(),
        TagValue::String("(same as create)".to_string()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_dos_metadata() {
        let header = DosHeader {
            e_magic: 0x5A4D,
            e_cblp: 0,
            e_cp: 0,
            e_crlc: 0,
            e_cparhdr: 0,
            e_minalloc: 0,
            e_maxalloc: 0,
            e_ss: 0,
            e_sp: 0,
            e_csum: 0,
            e_ip: 0,
            e_cs: 0,
            e_lfarlc: 0,
            e_ovno: 0,
            e_res: [0; 4],
            e_oemid: 0,
            e_oeminfo: 0,
            e_res2: [0; 10],
            e_lfanew: 0xF0,
        };

        let mut metadata = MetadataMap::new();
        extract_dos_metadata(&header, &mut metadata);

        assert!(metadata.contains_key("PE:DOSSignature"));
        assert!(metadata.contains_key("PE:PEHeaderOffset"));
    }

    #[test]
    fn test_extract_coff_metadata() {
        let header = CoffHeader {
            machine: machine_types::IMAGE_FILE_MACHINE_AMD64,
            number_of_sections: 5,
            time_date_stamp: 1609459200, // 2021-01-01 00:00:00 UTC
            pointer_to_symbol_table: 0,
            number_of_symbols: 0,
            size_of_optional_header: 0xF0,
            characteristics: 0x0022, // Executable, Large address aware
        };

        let mut metadata = MetadataMap::new();
        extract_coff_metadata(&header, &mut metadata);

        assert_eq!(
            metadata.get_string("PE:MachineType").unwrap(),
            "x64 (AMD64)"
        );
        assert_eq!(metadata.get_integer("PE:NumberOfSections").unwrap(), 5);
        assert!(metadata.contains_key("PE:CompileTime"));
        assert_eq!(metadata.get_string("PE:FileType").unwrap(), "Executable");
    }

    #[test]
    fn test_extract_rsds_metadata() {
        // Create a test RSDS structure
        let rsds = CodeViewRSDS {
            signature: *b"RSDS",
            guid: [
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
                0x0F, 0x10,
            ],
            age: 1,
            pdb_file_name: "test.pdb".to_string(),
        };

        let mut metadata = MetadataMap::new();
        extract_rsds_metadata(&rsds, &mut metadata);

        // Verify PDB file name
        assert_eq!(metadata.get_string("PE:PDBFileName").unwrap(), "test.pdb");

        // Verify age
        assert_eq!(metadata.get_integer("PE:PDBAge").unwrap(), 1);

        // Verify GUID format (note: byte order conversion in GUID formatting)
        assert!(metadata.contains_key("PE:PDBGUID"));
        let guid = metadata.get_string("PE:PDBGUID").unwrap();
        assert_eq!(guid.len(), 36); // GUID is 32 hex chars + 4 hyphens
        assert!(guid.contains('-'));
    }

    #[test]
    fn test_extract_nb10_metadata() {
        // Create a test NB10 structure
        let nb10 = CodeViewNB10 {
            signature: *b"NB10",
            offset: 0,
            timestamp: 1609459200, // 2021-01-01 00:00:00 UTC
            age: 1,
            pdb_file_name: "legacy.pdb".to_string(),
        };

        let mut metadata = MetadataMap::new();
        extract_nb10_metadata(&nb10, &mut metadata);

        // Verify PDB file name
        assert_eq!(metadata.get_string("PE:PDBFileName").unwrap(), "legacy.pdb");

        // Verify age
        assert_eq!(metadata.get_integer("PE:PDBAge").unwrap(), 1);

        // Verify create date was set
        assert!(metadata.contains_key("PE:PDBCreateDate"));
        let create_date = metadata.get_string("PE:PDBCreateDate").unwrap();
        assert!(create_date.starts_with("2021:01:01"));

        // Verify modify date placeholder
        assert_eq!(
            metadata.get_string("PE:PDBModifyDate").unwrap(),
            "(same as create)"
        );
    }
}
