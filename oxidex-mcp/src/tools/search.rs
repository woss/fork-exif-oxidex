use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct SearchParams {
    directory: String,
    filters: Vec<String>,
}

pub async fn handle(arguments: Value) -> Result<String> {
    let params: SearchParams =
        serde_json::from_value(arguments).context("Invalid arguments for search_metadata")?;

    // Validate directory
    crate::utils::validate_path(&params.directory)?;

    // Expand directory to all files
    let pattern = format!("{}/**/*", params.directory.trim_end_matches('/'));
    let files = match crate::utils::expand_glob(&pattern) {
        Ok(f) => f,
        Err(_) => {
            // Try without recursive glob
            let pattern = format!("{}/*", params.directory.trim_end_matches('/'));
            crate::utils::expand_glob(&pattern)?
        }
    };

    if files.is_empty() {
        return Ok(format!(
            "No files found in directory '{}'",
            params.directory
        ));
    }

    // Parse filters
    let filters = parse_filters(&params.filters)?;

    // Search files
    let mut matches = Vec::new();

    for path in files {
        if !path.is_file() {
            continue;
        }

        if let Ok(metadata) = extract_metadata(&path) {
            if matches_filters(&metadata, &filters) {
                let filename = path.to_string_lossy().to_string();
                matches.push((filename, metadata));
            }
        }
    }

    if matches.is_empty() {
        Ok(format!(
            "No files matched criteria: {}",
            params.filters.join(", ")
        ))
    } else {
        let summary = format!("Found {} file(s) matching criteria:\n\n", matches.len());
        Ok(summary + &crate::format::format_multiple_files(matches))
    }
}

#[derive(Debug)]
enum FilterOp {
    Equals(String, String),      // tag=value
    GreaterThan(String, String), // tag>value
    LessThan(String, String),    // tag<value
    Contains(String, String),    // tag~value
}

fn parse_filters(filter_strings: &[String]) -> Result<Vec<FilterOp>> {
    let mut filters = Vec::new();

    for s in filter_strings {
        if let Some((tag, value)) = s.split_once('=') {
            filters.push(FilterOp::Equals(tag.to_string(), value.to_string()));
        } else if let Some((tag, value)) = s.split_once('>') {
            filters.push(FilterOp::GreaterThan(tag.to_string(), value.to_string()));
        } else if let Some((tag, value)) = s.split_once('<') {
            filters.push(FilterOp::LessThan(tag.to_string(), value.to_string()));
        } else if let Some((tag, value)) = s.split_once('~') {
            filters.push(FilterOp::Contains(tag.to_string(), value.to_string()));
        } else {
            anyhow::bail!("Invalid filter syntax: '{}'. Use format: TagName=Value, TagName>Value, TagName<Value, or TagName~Value", s);
        }
    }

    Ok(filters)
}

fn matches_filters(metadata: &HashMap<String, String>, filters: &[FilterOp]) -> bool {
    for filter in filters {
        match filter {
            FilterOp::Equals(tag, value) => {
                if metadata.get(tag) != Some(value) {
                    return false;
                }
            }
            FilterOp::GreaterThan(tag, value) => {
                if let Some(actual) = metadata.get(tag) {
                    if actual <= value {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            FilterOp::LessThan(tag, value) => {
                if let Some(actual) = metadata.get(tag) {
                    if actual >= value {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            FilterOp::Contains(tag, value) => {
                if let Some(actual) = metadata.get(tag) {
                    if !actual.contains(value) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
    }

    true
}

fn extract_metadata(path: &PathBuf) -> Result<HashMap<String, String>> {
    // Reuse the extraction logic from extract tool
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

    Ok(result)
}
