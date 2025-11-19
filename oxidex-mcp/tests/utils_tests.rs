use std::collections::HashMap;
use std::fs::File;
use tempfile::TempDir;

#[test]
fn test_expand_glob_pattern() {
    let temp = TempDir::new().unwrap();
    let temp_path = temp.path();

    // Create test files
    File::create(temp_path.join("test1.jpg")).unwrap();
    File::create(temp_path.join("test2.jpg")).unwrap();
    File::create(temp_path.join("test.png")).unwrap();

    let pattern = format!("{}/*.jpg", temp_path.display());
    let files = oxidex_mcp::utils::expand_glob(&pattern).unwrap();

    assert_eq!(files.len(), 2);
    assert!(files.iter().all(|f| f.to_str().unwrap().ends_with(".jpg")));
}

#[test]
fn test_format_metadata_as_text() {
    let mut metadata = HashMap::new();
    metadata.insert("Make".to_string(), "Canon".to_string());
    metadata.insert("Model".to_string(), "EOS R5".to_string());

    let formatted = oxidex_mcp::format::format_metadata_map("test.jpg", &metadata);

    assert!(formatted.contains("test.jpg:"));
    assert!(formatted.contains("Make: Canon"));
    assert!(formatted.contains("Model: EOS R5"));
}
