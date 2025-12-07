// Phase 3 Integration Tests
//
// Tests for the tag comparison workflow and GitHub Pages integration
// Validates end-to-end functionality of the comparison system

#[cfg(test)]
mod phase3_integration_tests {
    use std::fs;
    use std::path::Path;

    /// Test 1: Verify comparison directory exists
    #[test]
    fn test_comparison_directory_exists() {
        let comparison_dir = Path::new("docs/reference/comparison");
        assert!(
            comparison_dir.exists(),
            "comparison directory should exist at docs/reference/comparison"
        );
        assert!(
            comparison_dir.is_dir(),
            "comparison path should be a directory"
        );
    }

    /// Test 2: Verify comparison index.md exists and has content
    #[test]
    fn test_comparison_index_document() {
        let index_path = Path::new("docs/reference/comparison/index.md");
        assert!(
            index_path.exists(),
            "index.md should exist in comparison directory"
        );

        let content = fs::read_to_string(index_path).expect("should be able to read index.md");

        assert!(!content.is_empty(), "index.md should not be empty");
        assert!(
            content.contains("ExifTool"),
            "index.md should mention ExifTool"
        );
        assert!(
            content.contains("Coverage"),
            "index.md should show coverage information"
        );
    }

    /// Test 3: Verify workflow file exists
    #[test]
    fn test_workflow_file_exists() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        assert!(
            workflow_path.exists(),
            "compare-exiftool.yml workflow should exist"
        );
    }

    /// Test 4: Verify workflow has proper structure
    #[test]
    fn test_workflow_structure() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        // Check essential workflow components
        assert!(content.contains("name:"), "workflow should have a name");
        assert!(content.contains("jobs:"), "workflow should have jobs");
        assert!(
            content.contains("compare:"),
            "workflow should have compare job"
        );

        // Check triggers
        assert!(
            content.contains("workflow_dispatch"),
            "should support manual trigger"
        );
        assert!(content.contains("push:"), "should trigger on push");
    }

    /// Test 5: Verify workflow has permissions for commits
    #[test]
    fn test_workflow_permissions() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        assert!(
            content.contains("contents: write"),
            "should have write permissions for commits"
        );
    }

    /// Test 6: Verify ExifTool version detection
    #[test]
    fn test_exiftool_version_detection() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        assert!(
            content.contains("exiftool.org/ver.txt"),
            "workflow should detect ExifTool version from ver.txt"
        );
        assert!(
            content.contains("exiftool-version"),
            "workflow should store version in step output"
        );
    }

    /// Test 7: Verify cache configuration
    #[test]
    fn test_cache_configuration() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        assert!(
            content.contains("actions/cache"),
            "should use GitHub cache action"
        );
        assert!(
            content.contains("~/exiftool"),
            "should cache ExifTool directory"
        );
    }

    /// Test 8: Verify tag-comparison binary is built and run
    #[test]
    fn test_comparison_binary_usage() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        assert!(
            content.contains("--bin tag-comparison"),
            "workflow should build tag-comparison binary"
        );
        assert!(
            content.contains("./target/release/tag-comparison"),
            "workflow should run tag-comparison binary"
        );
    }

    /// Test 9: Verify report output configuration
    #[test]
    fn test_report_output() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        assert!(
            content.contains("comparison.json"),
            "workflow should output JSON report"
        );
        assert!(
            content.contains("--markdown-dir"),
            "workflow should generate markdown reports"
        );
    }

    /// Test 10: Verify commit step
    #[test]
    fn test_commit_step() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        assert!(
            content.contains("Commit reports"),
            "workflow should have commit step"
        );
        assert!(content.contains("git push"), "workflow should push changes");
    }
}

#[cfg(test)]
mod workflow_configuration_tests {
    use std::fs;
    use std::path::Path;

    /// Verify ExifTool is downloaded from GitHub
    #[test]
    fn test_exiftool_download() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        assert!(
            content.contains("github.com/exiftool/exiftool"),
            "should download ExifTool from GitHub"
        );
    }

    /// Verify test images path
    #[test]
    fn test_images_path() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        assert!(
            content.contains("t/images"),
            "workflow should use ExifTool test images"
        );
    }

    /// Verify baseline tracking
    #[test]
    fn test_baseline_tracking() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        assert!(
            content.contains("baseline.json"),
            "workflow should track baseline for regression detection"
        );
    }

    /// Verify version parameters are passed
    #[test]
    fn test_version_parameters() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        assert!(
            content.contains("--exiftool-version"),
            "workflow should pass ExifTool version"
        );
        assert!(
            content.contains("--oxidex-version"),
            "workflow should pass OxiDex version"
        );
    }
}

// Note: These are integration tests that verify the structure and configuration
// of the Phase 3 implementation. Full end-to-end testing requires:
// 1. GitHub Actions runner
// 2. ExifTool binary
// 3. Test images
// 4. Actual workflow execution
//
// These tests can be run locally with: cargo test --test phase3_integration_test
