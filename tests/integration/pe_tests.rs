//! Integration tests for PE file parsing

use exiftool_rs::io::MMapReader;
use exiftool_rs::parsers::pe::parse_pe_metadata;
use std::path::Path;

#[test]
#[ignore] // Ignore until we have real PE test files
fn test_parse_real_pe_file() {
    // This test requires a real PE file
    // You can use any Windows executable or DLL
    let test_file = Path::new("tests/samples/pe/sample.exe");

    if !test_file.exists() {
        eprintln!("Skipping test: sample PE file not found");
        return;
    }

    let reader = MMapReader::new(test_file).expect("Failed to open PE file");
    let metadata = parse_pe_metadata(&reader).expect("Failed to parse PE metadata");

    // Verify basic metadata is present
    assert!(metadata.contains_key("PE:DOSSignature"));
    assert!(metadata.contains_key("PE:MachineType"));
    assert!(metadata.contains_key("PE:NumberOfSections"));
    assert!(metadata.contains_key("PE:CompileTime"));
    assert!(metadata.contains_key("PE:FileType"));
    assert!(metadata.contains_key("PE:Subsystem"));

    // Print all metadata for manual verification
    for (key, value) in metadata.iter() {
        println!("{}: {:?}", key, value);
    }
}

#[test]
fn test_parse_minimal_pe_structure() {
    // Create minimal valid PE structure in memory
    let mut data = Vec::new();

    // DOS Header (64 bytes)
    data.extend_from_slice(&0x5A4Du16.to_le_bytes()); // MZ signature
    data.resize(0x3C, 0x00); // Padding to e_lfanew
    data.extend_from_slice(&0x80u32.to_le_bytes()); // e_lfanew = 0x80

    // DOS Stub (from 0x40 to 0x80)
    data.resize(0x80, 0x00);

    // PE Signature and COFF Header
    data.extend_from_slice(b"PE\0\0"); // PE signature
    data.extend_from_slice(&0x014Cu16.to_le_bytes()); // Machine (i386)
    data.extend_from_slice(&3u16.to_le_bytes()); // Number of sections
    data.extend_from_slice(&1609459200u32.to_le_bytes()); // Timestamp
    data.extend_from_slice(&[0; 4]); // Symbol table ptr
    data.extend_from_slice(&[0; 4]); // Number of symbols
    data.extend_from_slice(&96u16.to_le_bytes()); // Optional header size (just fields we parse)
    data.extend_from_slice(&0x0102u16.to_le_bytes()); // Characteristics

    // Optional Header Standard Fields
    data.extend_from_slice(&0x010Bu16.to_le_bytes()); // Magic (PE32)
    data.push(14); // Major linker version
    data.push(0); // Minor linker version
    data.extend_from_slice(&0x1000u32.to_le_bytes()); // Size of code
    data.extend_from_slice(&0x2000u32.to_le_bytes()); // Initialized data
    data.extend_from_slice(&0x0u32.to_le_bytes()); // Uninitialized data
    data.extend_from_slice(&0x1000u32.to_le_bytes()); // Entry point
    data.extend_from_slice(&0x1000u32.to_le_bytes()); // Base of code

    // Optional Header NT Fields (PE32)
    data.extend_from_slice(&0x1000u32.to_le_bytes()); // Base of data
    data.extend_from_slice(&0x00400000u32.to_le_bytes()); // Image base
    data.extend_from_slice(&0x1000u32.to_le_bytes()); // Section alignment
    data.extend_from_slice(&0x200u32.to_le_bytes()); // File alignment
    data.extend_from_slice(&6u16.to_le_bytes()); // Major OS version
    data.extend_from_slice(&0u16.to_le_bytes()); // Minor OS version
    data.extend_from_slice(&1u16.to_le_bytes()); // Major image version
    data.extend_from_slice(&0u16.to_le_bytes()); // Minor image version
    data.extend_from_slice(&6u16.to_le_bytes()); // Major subsystem version
    data.extend_from_slice(&0u16.to_le_bytes()); // Minor subsystem version
    data.extend_from_slice(&0u32.to_le_bytes()); // Win32 version
    data.extend_from_slice(&0x10000u32.to_le_bytes()); // Size of image
    data.extend_from_slice(&0x400u32.to_le_bytes()); // Size of headers
    data.extend_from_slice(&0u32.to_le_bytes()); // Checksum
    data.extend_from_slice(&3u16.to_le_bytes()); // Subsystem (CUI)
    data.extend_from_slice(&0u16.to_le_bytes()); // DLL characteristics
    data.extend_from_slice(&0x100000u32.to_le_bytes()); // Stack reserve
    data.extend_from_slice(&0x1000u32.to_le_bytes()); // Stack commit
    data.extend_from_slice(&0x100000u32.to_le_bytes()); // Heap reserve
    data.extend_from_slice(&0x1000u32.to_le_bytes()); // Heap commit
    data.extend_from_slice(&0u32.to_le_bytes()); // Loader flags
    data.extend_from_slice(&16u32.to_le_bytes()); // Number of RVA/sizes

    // Pad the file to ensure we have enough data for reading
    // The parser reads 512 bytes from PE offset, so ensure file is at least that large
    data.resize(0x80 + 512, 0x00);

    // Write to temp file and parse
    use tempfile::NamedTempFile;
    use std::io::Write;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(&data).unwrap();
    temp_file.flush().unwrap();

    let reader = MMapReader::new(temp_file.path()).expect("Failed to open temp file");
    let metadata = parse_pe_metadata(&reader).expect("Failed to parse minimal PE");

    // Verify metadata
    assert_eq!(metadata.get_string("PE:MachineType").unwrap(), "Intel 386");
    assert_eq!(metadata.get_integer("PE:NumberOfSections").unwrap(), 3);
    assert_eq!(metadata.get_string("PE:FileType").unwrap(), "Executable");
    assert_eq!(metadata.get_string("PE:Subsystem").unwrap(), "Windows Console");
    assert_eq!(metadata.get_string("PE:ImageFormat").unwrap(), "PE32");
}
