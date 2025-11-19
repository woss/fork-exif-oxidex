use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct ExtractParams {
    path: String,
}

pub async fn handle(arguments: Value) -> Result<String> {
    let params: ExtractParams =
        serde_json::from_value(arguments).context("Invalid arguments for extract_metadata")?;

    // Validate path
    crate::utils::validate_path(&params.path)?;

    // Check if it's a glob pattern
    let is_glob = params.path.contains('*') || params.path.contains('?');

    if is_glob {
        handle_glob_pattern(&params.path).await
    } else {
        handle_single_file(&params.path).await
    }
}

async fn handle_single_file(path: &str) -> Result<String> {
    let path_buf = PathBuf::from(path);

    if !path_buf.exists() {
        return Ok(format!("File not found: {}", path));
    }

    // Use OxiDex to extract metadata
    match extract_metadata_from_file(&path_buf) {
        Ok(metadata) => {
            let filename = path_buf
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path);
            Ok(crate::format::format_metadata_map(filename, &metadata))
        }
        Err(e) => Ok(crate::format::format_error(path, &e.to_string())),
    }
}

async fn handle_glob_pattern(pattern: &str) -> Result<String> {
    let files = crate::utils::expand_glob(pattern)?;

    if files.is_empty() {
        return Ok(format!(
            "No files matched pattern '{}' in current directory",
            pattern
        ));
    }

    // Process files in parallel using rayon
    let results: Vec<(String, Result<HashMap<String, String>>)> = files
        .iter()
        .map(|path| {
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            let result = extract_metadata_from_file(path);
            (filename, result)
        })
        .collect();

    // Separate successes and failures
    let mut successes = Vec::new();
    let mut failures = Vec::new();

    for (filename, result) in results {
        match result {
            Ok(metadata) => successes.push((filename, metadata)),
            Err(e) => failures.push((filename, e.to_string())),
        }
    }

    if successes.is_empty() && !failures.is_empty() {
        Ok(format!(
            "Could not extract metadata from any files:\n{}",
            failures
                .iter()
                .map(|(f, e)| format!("✗ {}: {}", f, e))
                .collect::<Vec<_>>()
                .join("\n")
        ))
    } else {
        Ok(crate::format::format_multiple_files(successes))
    }
}

fn extract_metadata_from_file(path: &PathBuf) -> Result<HashMap<String, String>> {
    // Use OxiDex to extract real metadata
    let metadata_map = oxidex::core::operations::read_metadata(path)?;

    // Convert MetadataMap to HashMap<String, String>
    let mut result = HashMap::new();

    // Extract all metadata tags and convert TagValue to String
    for (key, value) in metadata_map.iter() {
        let value_str = tag_value_to_string(value);
        result.insert(key.to_string(), value_str);
    }

    Ok(result)
}

/// Converts a TagValue to a human-readable string
fn tag_value_to_string(value: &oxidex::core::tag_value::TagValue) -> String {
    use oxidex::core::tag_value::TagValue;

    match value {
        TagValue::String(s) => s.clone(),
        TagValue::Integer(i) => i.to_string(),
        TagValue::Float(f) => f.to_string(),
        TagValue::Rational {
            numerator,
            denominator,
        } => {
            if *denominator == 1 {
                numerator.to_string()
            } else {
                format!("{}/{}", numerator, denominator)
            }
        }
        TagValue::Binary(data) => format!("(Binary, {} bytes)", data.len()),
        TagValue::DateTime(dt) => dt.to_rfc3339(),
        TagValue::Struct(_) => "(Structured data)".to_string(),
        TagValue::Array(values) => {
            let items: Vec<String> = values.iter().map(tag_value_to_string).collect();
            format!("[{}]", items.join(", "))
        }
    }
}
