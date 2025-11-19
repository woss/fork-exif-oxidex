use std::collections::HashMap;

/// Format metadata as human-readable text
pub fn format_metadata_map(filename: &str, metadata: &HashMap<String, String>) -> String {
    let mut output = format!("{}:\n", filename);

    let mut keys: Vec<&String> = metadata.keys().collect();
    keys.sort();

    for key in keys {
        if let Some(value) = metadata.get(key) {
            output.push_str(&format!("  {}: {}\n", key, value));
        }
    }

    output
}

/// Format multiple files' metadata
pub fn format_multiple_files(results: Vec<(String, HashMap<String, String>)>) -> String {
    if results.is_empty() {
        return "No files found.".to_string();
    }

    let mut output = format!("Found {} file(s):\n\n", results.len());

    for (filename, metadata) in results {
        output.push_str(&format_metadata_map(&filename, &metadata));
        output.push('\n');
    }

    output.trim_end().to_string()
}

/// Format error message
pub fn format_error(filename: &str, error: &str) -> String {
    format!("❌ {}: {}", filename, error)
}

/// Format batch results with successes and failures
pub fn format_batch_results(
    successes: Vec<(String, HashMap<String, String>)>,
    failures: Vec<(String, String)>,
) -> String {
    let total = successes.len() + failures.len();
    let mut output = format!(
        "Processed {}/{} files successfully:\n\n",
        successes.len(),
        total
    );

    if !successes.is_empty() {
        for (filename, _) in &successes {
            output.push_str(&format!("✓ {}\n", filename));
        }
    }

    if !failures.is_empty() {
        output.push_str("\nFailures:\n");
        for (filename, error) in &failures {
            output.push_str(&format!("✗ {}: {}\n", filename, error));
        }
    }

    output
}
