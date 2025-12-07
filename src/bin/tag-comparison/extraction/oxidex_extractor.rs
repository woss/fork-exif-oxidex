//! OxiDex tag extractor - Extract tags by running OxiDex on test fixtures

use super::ExtractionResult;
use crate::models::TagInfo;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Extract tags from OxiDex by processing test fixtures
pub struct OxiDexExtractor {
    fixture_path: PathBuf,
    cache: HashMap<String, ExtractionResult>,
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
    /// ExtractionResult with tags and file count
    pub async fn extract_format_tags(
        &mut self,
        format: &str,
    ) -> Result<ExtractionResult, Box<dyn std::error::Error>> {
        // Check cache first
        if let Some(cached) = self.cache.get(format) {
            return Ok(cached.clone());
        }

        // Try format subdirectory first (e.g., samples/jpeg/)
        let format_path = self.fixture_path.join(format.to_lowercase());
        let files: Vec<PathBuf> = if format_path.exists() {
            WalkDir::new(&format_path)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| e.path().is_file())
                .map(|e| e.path().to_path_buf())
                .collect()
        } else {
            // Fall back to finding files by extension in the samples directory
            self.find_files_by_extension(format)?
        };

        let files_processed = files.len();

        if files.is_empty() {
            return Ok(ExtractionResult {
                tags: Vec::new(),
                files_processed: 0,
            });
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
                    eprintln!(
                        "Warning: Failed to extract tags from {}: {}",
                        file_path.display(),
                        e
                    );
                    // Continue processing other files
                }
            }
        }

        // Convert to final format
        let mut tags: Vec<TagInfo> = all_tags
            .into_values()
            .map(|(tag_info, _count)| tag_info)
            .collect();

        // Sort by key for consistency
        tags.sort_by_key(|a| a.key());

        let result = ExtractionResult {
            tags: tags.clone(),
            files_processed,
        };

        // Cache the result
        self.cache.insert(format.to_string(), result.clone());

        Ok(result)
    }

    /// Extract tags from a single file using OxiDex
    fn extract_tags_from_file(
        &self,
        file_path: &Path,
    ) -> Result<Vec<TagInfo>, Box<dyn std::error::Error>> {
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
            // Note: value is &TagValue, so we match on the reference
            let value_str = match value {
                oxidex::core::TagValue::String(s) => s.clone(),
                oxidex::core::TagValue::Integer(i) => i.to_string(),
                oxidex::core::TagValue::Float(f) => f.to_string(),
                oxidex::core::TagValue::Rational {
                    numerator,
                    denominator,
                } => {
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

    /// Find files by extension when format subdirectory doesn't exist
    fn find_files_by_extension(
        &self,
        format: &str,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let extensions = Self::format_to_extensions(format);
        if extensions.is_empty() {
            return Ok(Vec::new());
        }

        let files: Vec<PathBuf> = WalkDir::new(&self.fixture_path)
            .max_depth(2) // Don't go too deep
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| {
                if !e.path().is_file() {
                    return false;
                }
                if let Some(ext) = e.path().extension().and_then(|e| e.to_str()) {
                    extensions.contains(&ext.to_lowercase().as_str())
                } else {
                    false
                }
            })
            .map(|e| e.path().to_path_buf())
            .collect();

        Ok(files)
    }

    /// Map format name to file extensions
    fn format_to_extensions(format: &str) -> Vec<&'static str> {
        match format.to_uppercase().as_str() {
            "JPEG" => vec!["jpg", "jpeg"],
            "PNG" => vec!["png"],
            "TIFF" => vec!["tif", "tiff"],
            "GIF" => vec!["gif"],
            "WEBP" => vec!["webp"],
            "HEIC" => vec!["heic", "heif"],
            "MP4" => vec!["mp4", "m4v", "mov"],
            "AVI" => vec!["avi"],
            "MKV" => vec!["mkv"],
            "MP3" => vec!["mp3"],
            "WAV" => vec!["wav"],
            "PDF" => vec!["pdf"],
            "PSD" => vec!["psd"],
            "CR2" => vec!["cr2", "cr3"],
            "NEF" => vec!["nef"],
            "ARW" => vec!["arw"],
            "DNG" => vec!["dng"],
            "RAF" => vec!["raf"],
            "ORF" => vec!["orf"],
            "RW2" => vec!["rw2"],
            "XMP" => vec!["xmp"],
            _ => vec![],
        }
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
