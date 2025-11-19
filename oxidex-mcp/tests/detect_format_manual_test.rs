// Manual test demonstrating detect_format tool functionality
// Run with: cargo test --package oxidex-mcp --test detect_format_manual_test -- --nocapture

use serde_json::json;
use std::fs;
use std::io::Write;

#[tokio::test]
async fn test_detect_format_with_real_jpeg() {
    // Create a minimal JPEG file for testing
    let test_file = "tests/fixtures/test_jpeg.jpg";

    // JPEG magic bytes: FF D8 FF E0 00 10 4A 46 49 46 (JFIF header)
    let jpeg_data = vec![
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, // JFIF
        0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, // Version info
        0xFF, 0xD9, // EOI marker
    ];

    // Write test file
    let mut file = fs::File::create(test_file).expect("Failed to create test file");
    file.write_all(&jpeg_data).expect("Failed to write test data");
    drop(file);

    // Test format detection
    let args = json!({
        "path": test_file
    });

    let result = oxidex_mcp::tools::detect_format::handle(args).await;
    assert!(result.is_ok(), "detect_format should succeed: {:?}", result);

    let output = result.unwrap();
    println!("\n===== DETECT FORMAT OUTPUT =====");
    println!("{}", output);
    println!("================================\n");

    // Verify output
    assert!(output.contains("Format: JPEG"), "Should detect JPEG format");
    assert!(output.contains("MIME Type: image/jpeg"), "Should show JPEG MIME type");
    assert!(output.contains("EXIF"), "Should list EXIF support");
    assert!(output.contains("XMP"), "Should list XMP support");
    assert!(output.contains("Supported Operations:"), "Should show supported operations");
    assert!(output.contains("Extract metadata"), "Should support extraction");
    assert!(output.contains("Write metadata"), "Should support writing");

    // Clean up
    fs::remove_file(test_file).ok();
}

#[tokio::test]
async fn test_detect_format_with_real_png() {
    // Create a minimal PNG file for testing
    let test_file = "tests/fixtures/test_png.png";

    // PNG magic bytes and minimal structure
    let png_data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, // IHDR length
        0x49, 0x48, 0x44, 0x52, // "IHDR"
        0x00, 0x00, 0x00, 0x01, // Width: 1
        0x00, 0x00, 0x00, 0x01, // Height: 1
        0x08, 0x02, 0x00, 0x00, 0x00, // Bit depth, color type, compression, filter, interlace
        0x90, 0x77, 0x53, 0xDE, // CRC
        0x00, 0x00, 0x00, 0x00, // IEND length
        0x49, 0x45, 0x4E, 0x44, // "IEND"
        0xAE, 0x42, 0x60, 0x82, // CRC
    ];

    // Write test file
    let mut file = fs::File::create(test_file).expect("Failed to create test file");
    file.write_all(&png_data).expect("Failed to write test data");
    drop(file);

    // Test format detection
    let args = json!({
        "path": test_file
    });

    let result = oxidex_mcp::tools::detect_format::handle(args).await;
    assert!(result.is_ok(), "detect_format should succeed: {:?}", result);

    let output = result.unwrap();
    println!("\n===== DETECT FORMAT OUTPUT =====");
    println!("{}", output);
    println!("================================\n");

    // Verify output
    assert!(output.contains("Format: PNG"), "Should detect PNG format");
    assert!(output.contains("MIME Type: image/png"), "Should show PNG MIME type");
    assert!(output.contains("XMP"), "Should list XMP support");
    assert!(output.contains("read/write"), "Should show read/write capability");

    // Clean up
    fs::remove_file(test_file).ok();
}

#[tokio::test]
async fn test_detect_format_glob_multiple_files() {
    // Create multiple test files
    let jpeg_file = "tests/fixtures/test_img1.jpg";
    let png_file = "tests/fixtures/test_img2.png";

    // Create JPEG
    let jpeg_data = vec![
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46,
        0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00,
        0xFF, 0xD9,
    ];
    fs::File::create(jpeg_file).unwrap().write_all(&jpeg_data).unwrap();

    // Create PNG
    let png_data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE,
        0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    fs::File::create(png_file).unwrap().write_all(&png_data).unwrap();

    // Test glob pattern
    let args = json!({
        "path": "tests/fixtures/test_img*.jpg"
    });

    let result = oxidex_mcp::tools::detect_format::handle(args).await;
    assert!(result.is_ok(), "detect_format should succeed with glob");

    let output = result.unwrap();
    println!("\n===== GLOB PATTERN OUTPUT =====");
    println!("{}", output);
    println!("===============================\n");

    assert!(output.contains("Detected formats for"), "Should show file count");
    assert!(output.contains("Format: JPEG"), "Should detect JPEG format");

    // Clean up
    fs::remove_file(jpeg_file).ok();
    fs::remove_file(png_file).ok();
}
