//! OxiDex tag extractor - Extract tags by running OxiDex on test fixtures

use crate::models::TagInfo;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use walkdir::WalkDir;

/// Extract tags from OxiDex by processing test fixtures
pub struct OxiDexExtractor {
    fixture_path: PathBuf,
    cache: HashMap<String, Vec<TagInfo>>,
}

impl OxiDexExtractor {
    /// Create a new OxiDex extractor
    pub fn new(fixture_path: PathBuf) -> Self {
        Self {
            fixture_path,
            cache: HashMap::new(),
        }
    }

    /// Extract tags from all fixtures of a specific format
    ///
    /// # Arguments
    /// * `format` - Format name (e.g., "JPEG", "PNG")
    ///
    /// # Returns
    /// Vector of TagInfo representing all unique tags found in fixtures
    pub async fn extract_format_tags(&mut self, format: &str) -> Result<Vec<TagInfo>, Box<dyn std::error::Error>> {
        // Check cache first
        if let Some(cached) = self.cache.get(format) {
            return Ok(cached.clone());
        }

        let format_path = self.fixture_path.join(format.to_lowercase());
        if !format_path.exists() {
            return Ok(Vec::new());
        }

        // Find all fixture files for this format
        let files: Vec<PathBuf> = WalkDir::new(&format_path)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.path().is_file())
            .map(|e| e.path().to_path_buf())
            .collect();

        if files.is_empty() {
            return Ok(Vec::new());
        }

        // Extract tags from each file
        let mut all_tags: HashMap<String, (TagInfo, usize)> = HashMap::new();

        for file_path in &files {
            match self.extract_tags_from_file(file_path) {
                Ok(file_tags) => {
                    for tag_info in file_tags {
                        all_tags
                            .entry(format!("{}:{}", tag_info.family, tag_info.name))
                            .and_modify(|(_info, count)| *count += 1)
                            .or_insert((tag_info.clone(), 1));
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to extract tags from {}: {}", file_path.display(), e);
                    // Continue processing other files
                }
            }
        }

        // Convert to final format
        let mut result: Vec<TagInfo> = all_tags
            .into_values()
            .map(|(tag_info, _count)| tag_info)
            .collect();

        // Sort by key for consistency
        result.sort_by(|a, b| a.key().cmp(&b.key()));

        // Cache the result
        self.cache.insert(format.to_string(), result.clone());

        Ok(result)
    }

    /// Extract tags from a single file using OxiDex
    fn extract_tags_from_file(&self, file_path: &Path) -> Result<Vec<TagInfo>, Box<dyn std::error::Error>> {
        // Use the oxidex API to read metadata
        let metadata = oxidex::core::operations::read_metadata(file_path)?;

        // Convert metadata to TagInfo
        let tags = self.flatten_metadata(&metadata);

        Ok(tags)
    }

    /// Flatten MetadataMap into TagInfo vector
    fn flatten_metadata(&self, metadata: &oxidex::core::MetadataMap) -> Vec<TagInfo> {
        let mut tags = Vec::new();

        // Iterate through all tags in metadata
        for (key, value) in metadata.iter() {
            // Parse tag key into family and name
            // ExifTool format: "EXIF:Make" or "XMP:Creator"
            let (family, name) = if let Some(colon_pos) = key.find(':') {
                let (fam, nam) = key.split_at(colon_pos);
                (fam.to_string(), nam[1..].to_string()) // Skip the ':'
            } else {
                ("UNKNOWN".to_string(), key.clone())
            };

            // Convert TagValue to string
            let value_str = match value {
                oxidex::core::TagValue::String(s) => s.clone(),
                oxidex::core::TagValue::Integer(i) => i.to_string(),
                oxidex::core::TagValue::Float(f) => f.to_string(),
                oxidex::core::TagValue::Rational { numerator, denominator } => {
                    format!("{}/{}", numerator, denominator)
                }
                oxidex::core::TagValue::Binary(_) => "[Binary data]".to_string(),
                oxidex::core::TagValue::DateTime(dt) => dt.to_rfc3339(),
                oxidex::core::TagValue::Struct(_) => "[Structured data]".to_string(),
                oxidex::core::TagValue::Array(arr) => format!("{:?}", arr),
            };

            let tag_info = TagInfo::new(name, family, value_str);
            tags.push(tag_info);
        }

        tags
    }

    /// Get frequency of a specific tag across files
    #[allow(dead_code)]
    fn calculate_frequency(&self, _tag_name: &str, _file_count: usize) -> usize {
        // Frequency is calculated in extract_format_tags
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oxidex_extractor_creation() {
        let extractor = OxiDexExtractor::new(PathBuf::from("tests/fixtures/jpeg"));
        assert_eq!(extractor.fixture_path, PathBuf::from("tests/fixtures/jpeg"));
    }

    #[test]
    fn test_oxidex_extractor_cache() {
        let extractor = OxiDexExtractor::new(PathBuf::from("tests/fixtures"));
        assert_eq!(extractor.cache.len(), 0);
    }

    #[test]
    fn test_flatten_metadata_empty() {
        let extractor = OxiDexExtractor::new(PathBuf::from("tests/fixtures"));
        let metadata = oxidex::core::MetadataMap::new();
        let tags = extractor.flatten_metadata(&metadata);
        assert_eq!(tags.len(), 0);
    }
}
