use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct AnalyzeParams {
    path: String,
}

pub async fn handle(arguments: Value) -> Result<String> {
    let params: AnalyzeParams =
        serde_json::from_value(arguments).context("Invalid arguments for analyze_metadata")?;

    // Validate path
    crate::utils::validate_path(&params.path)?;

    // Expand glob
    let files = crate::utils::expand_glob(&params.path)?;

    if files.is_empty() {
        return Ok(format!("No files matched pattern '{}'", params.path));
    }

    // Extract metadata from all files
    let mut all_metadata = Vec::new();
    for path in files {
        if let Ok(metadata) = extract_metadata(&path) {
            all_metadata.push(metadata);
        }
    }

    if all_metadata.is_empty() {
        return Ok("No metadata could be extracted from matched files.".to_string());
    }

    // Analyze the metadata
    let analysis = analyze_all_metadata(&all_metadata);

    Ok(analysis)
}

fn extract_metadata(path: &PathBuf) -> Result<HashMap<String, String>> {
    let metadata = std::fs::metadata(path)?;
    let mut result = HashMap::new();

    result.insert("FileSize".to_string(), metadata.len().to_string());
    result.insert(
        "FileType".to_string(),
        if metadata.is_file() {
            "File"
        } else {
            "Directory"
        }
        .to_string(),
    );

    // TODO: Use oxidex library for real metadata
    Ok(result)
}

fn analyze_all_metadata(all_metadata: &[HashMap<String, String>]) -> String {
    let mut output = format!("Analyzed {} files:\n\n", all_metadata.len());

    // Count occurrences of each tag value
    let mut tag_counts: HashMap<String, HashMap<String, usize>> = HashMap::new();

    for metadata in all_metadata {
        for (key, value) in metadata {
            tag_counts
                .entry(key.clone())
                .or_default()
                .entry(value.clone())
                .and_modify(|c| *c += 1)
                .or_insert(1);
        }
    }

    // Format the statistics
    for (tag, value_counts) in tag_counts {
        if tag == "FileSize" {
            // Special handling for file sizes
            let sizes: Vec<u64> = all_metadata
                .iter()
                .filter_map(|m| m.get(&tag)?.parse().ok())
                .collect();

            if !sizes.is_empty() {
                let total: u64 = sizes.iter().sum();
                let avg = total / sizes.len() as u64;
                output.push_str("File Sizes:\n");
                output.push_str(&format!("  Total: {} bytes\n", total));
                output.push_str(&format!("  Average: {} bytes\n", avg));
                output.push('\n');
            }
        } else {
            // Regular tag statistics
            output.push_str(&format!("{}:\n", tag));
            let mut sorted: Vec<_> = value_counts.iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(a.1));

            for (value, count) in sorted.iter().take(5) {
                output.push_str(&format!("  {}: {} files\n", value, count));
            }
            output.push('\n');
        }
    }

    output.trim_end().to_string()
}
