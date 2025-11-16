//! PE file structure definitions
//!
//! This module defines Rust structures matching PE file headers
//! as per Microsoft PE/COFF specification.

#![allow(dead_code)]

/// DOS Header (IMAGE_DOS_HEADER) - 64 bytes
#[derive(Debug, Clone, Copy)]
pub struct DosHeader {
    /// Magic number "MZ" (0x5A4D)
    pub e_magic: u16,
    /// Bytes on last page of file
    pub e_cblp: u16,
    /// Pages in file
    pub e_cp: u16,
    /// Relocations
    pub e_crlc: u16,
    /// Size of header in paragraphs
    pub e_cparhdr: u16,
    /// Minimum extra paragraphs needed
    pub e_minalloc: u16,
    /// Maximum extra paragraphs needed
    pub e_maxalloc: u16,
    /// Initial (relative) SS value
    pub e_ss: u16,
    /// Initial SP value
    pub e_sp: u16,
    /// Checksum
    pub e_csum: u16,
    /// Initial IP value
    pub e_ip: u16,
    /// Initial (relative) CS value
    pub e_cs: u16,
    /// File address of relocation table
    pub e_lfarlc: u16,
    /// Overlay number
    pub e_ovno: u16,
    /// Reserved words
    pub e_res: [u16; 4],
    /// OEM identifier
    pub e_oemid: u16,
    /// OEM information
    pub e_oeminfo: u16,
    /// Reserved words
    pub e_res2: [u16; 10],
    /// File address of new exe header
    pub e_lfanew: u32,
}

/// COFF File Header (IMAGE_FILE_HEADER) - 20 bytes
#[derive(Debug, Clone, Copy)]
pub struct CoffHeader {
    /// Target machine type
    pub machine: u16,
    /// Number of sections
    pub number_of_sections: u16,
    /// Time/date stamp
    pub time_date_stamp: u32,
    /// File offset of symbol table
    pub pointer_to_symbol_table: u32,
    /// Number of symbols
    pub number_of_symbols: u32,
    /// Size of optional header
    pub size_of_optional_header: u16,
    /// File characteristics flags
    pub characteristics: u16,
}

/// Optional Header Standard Fields
#[derive(Debug, Clone, Copy)]
pub struct OptionalHeaderStandard {
    /// Image format identifier (0x10B=PE32, 0x20B=PE32+)
    pub magic: u16,
    /// Linker major version
    pub major_linker_version: u8,
    /// Linker minor version
    pub minor_linker_version: u8,
    /// Size of code section
    pub size_of_code: u32,
    /// Size of initialized data
    pub size_of_initialized_data: u32,
    /// Size of uninitialized data
    pub size_of_uninitialized_data: u32,
    /// Entry point RVA
    pub address_of_entry_point: u32,
    /// Code section base RVA
    pub base_of_code: u32,
}

/// Optional Header NT-Specific Fields (PE32)
#[derive(Debug, Clone)]
pub struct OptionalHeaderNT {
    /// Preferred load address
    pub image_base: u64,
    /// Memory alignment
    pub section_alignment: u32,
    /// File alignment
    pub file_alignment: u32,
    /// OS major version
    pub major_operating_system_version: u16,
    /// OS minor version
    pub minor_operating_system_version: u16,
    /// Image major version
    pub major_image_version: u16,
    /// Image minor version
    pub minor_image_version: u16,
    /// Subsystem major version
    pub major_subsystem_version: u16,
    /// Subsystem minor version
    pub minor_subsystem_version: u16,
    /// Reserved
    pub win32_version_value: u32,
    /// Image size in memory
    pub size_of_image: u32,
    /// Header size
    pub size_of_headers: u32,
    /// Checksum
    pub checksum: u32,
    /// Subsystem type
    pub subsystem: u16,
    /// DLL characteristics
    pub dll_characteristics: u16,
    /// Stack reserve size
    pub size_of_stack_reserve: u64,
    /// Stack commit size
    pub size_of_stack_commit: u64,
    /// Heap reserve size
    pub size_of_heap_reserve: u64,
    /// Heap commit size
    pub size_of_heap_commit: u64,
    /// Loader flags (obsolete)
    pub loader_flags: u32,
    /// Data directory count
    pub number_of_rva_and_sizes: u32,
}

/// Machine type constants
#[allow(dead_code)]
pub mod machine_types {
    /// Unknown machine type
    pub const IMAGE_FILE_MACHINE_UNKNOWN: u16 = 0x0;
    /// Intel 386+
    pub const IMAGE_FILE_MACHINE_I386: u16 = 0x014C;
    /// MIPS R3000
    pub const IMAGE_FILE_MACHINE_R3000: u16 = 0x0162;
    /// MIPS R4000
    pub const IMAGE_FILE_MACHINE_R4000: u16 = 0x0166;
    /// MIPS R10000
    pub const IMAGE_FILE_MACHINE_R10000: u16 = 0x0168;
    /// Alpha AXP
    pub const IMAGE_FILE_MACHINE_ALPHA: u16 = 0x0184;
    /// PowerPC
    pub const IMAGE_FILE_MACHINE_POWERPC: u16 = 0x01F0;
    /// Intel Itanium
    pub const IMAGE_FILE_MACHINE_IA64: u16 = 0x0200;
    /// AMD64 (x64)
    pub const IMAGE_FILE_MACHINE_AMD64: u16 = 0x8664;
    /// ARM
    pub const IMAGE_FILE_MACHINE_ARM: u16 = 0x01C0;
    /// ARM64
    pub const IMAGE_FILE_MACHINE_ARM64: u16 = 0xAA64;
}

/// Subsystem type constants
#[allow(dead_code)]
pub mod subsystem_types {
    /// Unknown subsystem
    pub const IMAGE_SUBSYSTEM_UNKNOWN: u16 = 0;
    /// Native (driver)
    pub const IMAGE_SUBSYSTEM_NATIVE: u16 = 1;
    /// Windows GUI
    pub const IMAGE_SUBSYSTEM_WINDOWS_GUI: u16 = 2;
    /// Windows Console
    pub const IMAGE_SUBSYSTEM_WINDOWS_CUI: u16 = 3;
    /// OS/2 Console
    pub const IMAGE_SUBSYSTEM_OS2_CUI: u16 = 5;
    /// POSIX Console
    pub const IMAGE_SUBSYSTEM_POSIX_CUI: u16 = 7;
    /// EFI Application
    pub const IMAGE_SUBSYSTEM_EFI_APPLICATION: u16 = 10;
    /// EFI Boot Service
    pub const IMAGE_SUBSYSTEM_EFI_BOOT_SERVICE_DRIVER: u16 = 11;
    /// EFI Runtime Driver
    pub const IMAGE_SUBSYSTEM_EFI_RUNTIME_DRIVER: u16 = 12;
    /// Xbox
    pub const IMAGE_SUBSYSTEM_XBOX: u16 = 14;
}
