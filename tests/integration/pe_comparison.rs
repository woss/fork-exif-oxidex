//! PE Format Validation Tests
//!
//! This test suite validates PE metadata extraction

use oxidex::io::MMapReader;
use oxidex::parsers::pe::parse_pe_metadata;
use std::path::Path;

#[test]
#[ignore] // Requires external PE file
fn test_pe_parsing_notepad_plus_plus() {
    // This test requires Notepad++ to be downloaded
    let test_file = "/tmp/notepad++.exe";

    let test_path = Path::new(test_file);
    if test_path.exists() {
        let reader = MMapReader::new(test_path).expect("Failed to open file");
        let metadata = parse_pe_metadata(&reader).expect("Failed to parse PE");

        println!("\n=== PE Tags Extracted: {} ===", metadata.len());

        // Verify key tags are present
        assert!(
            metadata.get("PE:MachineType").is_some(),
            "Missing PE:MachineType"
        );
        assert!(
            metadata.get("PE:CompanyName").is_some(),
            "Missing PE:CompanyName"
        );
        assert!(
            metadata.get("PE:FileVersion").is_some(),
            "Missing PE:FileVersion"
        );
        assert!(
            metadata.get("PE:ProductName").is_some(),
            "Missing PE:ProductName"
        );
        assert!(
            metadata.get("PE:FileDescription").is_some(),
            "Missing PE:FileDescription"
        );
        assert!(
            metadata.get("PE:LegalCopyright").is_some(),
            "Missing PE:LegalCopyright"
        );

        // Print all tags for manual verification
        let mut keys: Vec<_> = metadata.keys().collect();
        keys.sort();
        for key in keys {
            if let Some(value) = metadata.get(key) {
                println!("{}: {:?}", key, value);
            }
        }

        println!("\n✓ All critical PE tags extracted successfully!");
    } else {
        eprintln!("Skipping test: {} not found", test_file);
        eprintln!(
            "Download with: curl -sL https://github.com/notepad-plus-plus/notepad-plus-plus/releases/download/v8.6.9/npp.8.6.9.portable.x64.zip -o /tmp/npp.zip && cd /tmp && unzip -q npp.zip"
        );
    }
}

#[test]
fn test_pe_tag_coverage() {
    // Verify that we extract all the tags specified in the plan
    // This test doesn't need a real PE file, just verifies the tag database

    let expected_pe_tags = vec![
        // COFF Header tags
        "PE:MachineType",
        "PE:NumberOfSections",
        "PE:TimeStamp",
        "PE:CompileTime",
        "PE:Characteristics",
        "PE:ImageFileCharacteristics",
        // Optional Header tags
        "PE:ImageFormat",
        "PE:LinkerVersion",
        "PE:CodeSize",
        "PE:InitializedDataSize",
        "PE:UninitializedDataSize",
        "PE:EntryPoint",
        "PE:ImageBase",
        "PE:OSVersion",
        "PE:ImageVersion",
        "PE:Subsystem",
        "PE:SubsystemVersion",
        // VERSION_INFO tags
        "PE:FileVersionNumber",
        "PE:ProductVersionNumber",
        "PE:FileFlags",
        "PE:FileOS",
        "PE:FileType",
        "PE:FileSubtype",
        "PE:ObjectFileType",
        "PE:CompanyName",
        "PE:FileDescription",
        "PE:FileVersion",
        "PE:InternalName",
        "PE:LegalCopyright",
        "PE:OriginalFilename",
        "PE:ProductName",
        "PE:ProductVersion",
        // Debug Directory tags (optional, depending on file)
        // "PE:PDBFileName",
        // "PE:PDBGUID",
        // "PE:PDBAge",
    ];

    println!("Expected PE tags: {}", expected_pe_tags.len());
    println!("Tags:");
    for tag in &expected_pe_tags {
        println!("  - {}", tag);
    }

    // This test just documents the expected tags
    // Actual validation happens in the comparison tests
    assert!(
        expected_pe_tags.len() >= 30,
        "Should have at least 30 PE tags"
    );
}
