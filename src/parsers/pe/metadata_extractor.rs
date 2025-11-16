//! Metadata extraction from PE headers

use crate::core::{MetadataMap, TagValue};
use crate::parsers::pe::structures::{
    machine_types, subsystem_types, CoffHeader, DosHeader, OptionalHeaderNT, OptionalHeaderStandard,
};

/// Extract metadata from DOS header
pub fn extract_dos_metadata(header: &DosHeader, metadata: &mut MetadataMap) {
    metadata.insert("PE:DOSSignature".to_string(), TagValue::String(format!("{:#06X}", header.e_magic)));
    metadata.insert("PE:PEHeaderOffset".to_string(), TagValue::Integer(header.e_lfanew as i64));
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
    metadata.insert("PE:MachineType".to_string(), TagValue::String(machine_name.to_string()));
    metadata.insert("PE:MachineTypeRaw".to_string(), TagValue::Integer(header.machine as i64));

    // Number of sections
    metadata.insert("PE:NumberOfSections".to_string(), TagValue::Integer(header.number_of_sections as i64));

    // Timestamp (Unix epoch)
    if header.time_date_stamp > 0 {
        metadata.insert("PE:TimeStamp".to_string(), TagValue::Integer(header.time_date_stamp as i64));

        // Convert to human-readable date if possible
        use chrono::{Utc, TimeZone};
        if let Some(dt) = Utc.timestamp_opt(header.time_date_stamp as i64, 0).single() {
            metadata.insert(
                "PE:CompileTime".to_string(),
                TagValue::String(dt.format("%Y:%m:%d %H:%M:%S").to_string()),
            );
        }
    }

    // Characteristics
    metadata.insert("PE:Characteristics".to_string(), TagValue::Integer(header.characteristics as i64));

    // Decode common flags
    let is_executable = (header.characteristics & 0x0002) != 0;
    let is_dll = (header.characteristics & 0x2000) != 0;
    let file_type = if is_dll {
        "DLL"
    } else if is_executable {
        "Executable"
    } else {
        "Object"
    };
    metadata.insert("PE:FileType".to_string(), TagValue::String(file_type.to_string()));
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
    metadata.insert("PE:ImageFormat".to_string(), TagValue::String(image_format.to_string()));

    // Linker version
    metadata.insert(
        "PE:LinkerVersion".to_string(),
        TagValue::String(format!(
            "{}.{}",
            std_header.major_linker_version, std_header.minor_linker_version
        )),
    );

    // Entry point
    metadata.insert("PE:EntryPoint".to_string(), TagValue::Integer(std_header.address_of_entry_point as i64));

    // Image base
    metadata.insert("PE:ImageBase".to_string(), TagValue::Integer(nt_header.image_base as i64));

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
    metadata.insert("PE:Subsystem".to_string(), TagValue::String(subsystem_name.to_string()));
    metadata.insert("PE:SubsystemRaw".to_string(), TagValue::Integer(nt_header.subsystem as i64));

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
        metadata.insert("PE:Checksum".to_string(), TagValue::Integer(nt_header.checksum as i64));
    }
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
}
