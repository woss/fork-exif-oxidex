//! Tests for camera raw format extension detection
//!
//! This test suite verifies that all 40+ camera raw file extensions
//! are properly recognized as supported formats by the batch processor.

use std::path::Path;

/// Test that all camera raw extensions are supported by the batch processor
///
/// This test verifies the SUPPORTED_EXTENSIONS constant in batch_processor.rs
/// includes all major camera raw formats from manufacturers like Canon,
/// Nikon, Sony, Fuji, Olympus, etc.
#[test]
fn test_raw_extensions_supported() {
    // List of all camera raw extensions that should be supported
    let raw_extensions = vec![
        // Canon
        "cr2", "cr3", "crw", // Nikon
        "nef", "nrw", // Sony
        "arw", "arq", "ari", "sr2", "srf", "srw", // Fujifilm
        "raf", // Olympus
        "orf", "ori", // Pentax
        "pef", // Panasonic
        "rw2", "rwl", // Hasselblad
        "3fr", "fff", // Phase One
        "iiq", // Mamiya
        "mef", // Leaf
        "mos", // Kodak
        "dcr", "kdc", // Minolta
        "mdc", "mrw", // Epson
        "erf", // Sigma
        "x3f", // GoPro
        "gpr", // Adobe DNG
        "dng", // HEIF
        "hif", // Light
        "lri", // Sinar
        "sti", // Generic
        "raw", "cam", "rev",
    ];

    // Test each extension with the batch processor's is_supported_file function
    for ext in raw_extensions {
        let path_str = format!("test.{}", ext);
        let path = Path::new(&path_str);

        // Use the actual batch_processor function to verify
        assert!(
            exiftool_rs::cli::batch_processor::is_supported_file(path),
            "Extension '{}' not supported by batch processor",
            ext
        );
    }
}

/// Test that existing (non-raw) formats are still supported
#[test]
fn test_existing_formats_still_supported() {
    let existing_formats = vec![
        "jpg", "jpeg", "jpe", "jfif", // JPEG
        "tif", "tiff", // TIFF
        "png",  // PNG
        "mp4", "m4v", "m4a", "m4b", "mov", // Video
        "pdf", // PDF
    ];

    for ext in existing_formats {
        let path_str = format!("test.{}", ext);
        let path = Path::new(&path_str);

        assert!(
            exiftool_rs::cli::batch_processor::is_supported_file(path),
            "Extension '{}' should still be supported",
            ext
        );
    }
}

/// Test that unsupported extensions are properly rejected
#[test]
fn test_unsupported_extensions_rejected() {
    let unsupported = vec!["txt", "doc", "exe", "zip", "mp3"];

    for ext in unsupported {
        let path_str = format!("test.{}", ext);
        let path = Path::new(&path_str);

        assert!(
            !exiftool_rs::cli::batch_processor::is_supported_file(path),
            "Extension '{}' should NOT be supported",
            ext
        );
    }
}

/// Test that extension matching is case-insensitive
#[test]
fn test_case_insensitive_matching() {
    let test_cases = vec![
        ("TEST.CR2", true),
        ("test.Cr2", true),
        ("test.NEF", true),
        ("test.Nef", true),
        ("test.JPG", true),
        ("test.TXT", false),
    ];

    for (filename, should_support) in test_cases {
        let path = Path::new(filename);
        assert_eq!(
            exiftool_rs::cli::batch_processor::is_supported_file(path),
            should_support,
            "File '{}' support status should be {}",
            filename,
            should_support
        );
    }
}
