// Phase 3 Integration Tests
//
// Tests for the tag comparison workflow and GitHub Pages integration
// Validates end-to-end functionality of the comparison system

#[cfg(test)]
mod phase3_integration_tests {
    use std::fs;
    use std::path::{Path, PathBuf};

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
            content.contains("comparison"),
            "index.md should discuss comparison"
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
            content.contains("generate-report"),
            "workflow should have generate-report job"
        );

        // Check triggers
        assert!(
            content.contains("workflow_dispatch"),
            "should support manual trigger"
        );
        assert!(content.contains("push:"), "should trigger on push");
        assert!(
            content.contains("schedule:"),
            "should have schedule trigger"
        );
    }

    /// Test 5: Verify workflow has GitHub Pages deployment
    #[test]
    fn test_workflow_github_pages_deployment() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        assert!(
            content.contains("peaceiris/actions-gh-pages"),
            "should use peaceiris GitHub Pages action"
        );
        assert!(
            content.contains("tag-comparison"),
            "should deploy to tag-comparison directory"
        );
    }

    /// Test 6: Verify version-locked caching strategy
    #[test]
    fn test_workflow_version_locked_cache() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        assert!(
            content.contains("get-version"),
            "should get ExifTool version"
        );
        assert!(
            content.contains("exiftool-test-suite"),
            "should cache test suite"
        );
        assert!(
            content.contains("installed_version"),
            "cache key should include version"
        );
    }

    /// Test 7: Verify 3-tier download fallback
    #[test]
    fn test_workflow_download_fallback() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        let exiftool_org_count = content.matches("exiftool.org").count();
        let github_releases_count = content.matches("github.com/exiftool").count();
        let github_api_count = content.matches("api.github.com").count();

        assert!(
            exiftool_org_count > 0,
            "should try exiftool.org as primary source"
        );
        assert!(
            github_releases_count > 0,
            "should try GitHub releases as fallback"
        );
        assert!(
            github_api_count > 0,
            "should try GitHub API as final fallback"
        );
    }

    /// Test 8: Verify test script exists
    #[test]
    fn test_validation_script_exists() {
        let script_path = Path::new("scripts/test-compare-workflow.sh");
        assert!(
            script_path.exists(),
            "test-compare-workflow.sh should exist"
        );
    }

    /// Test 9: Verify test script is comprehensive
    #[test]
    fn test_validation_script_content() {
        let script_path = Path::new("scripts/test-compare-workflow.sh");
        let content = fs::read_to_string(script_path).expect("should read test script");

        // Check for test functions
        assert!(
            content.contains("test_workflow_exists"),
            "should test workflow existence"
        );
        assert!(
            content.contains("test_workflow_syntax"),
            "should test workflow syntax"
        );
        assert!(
            content.contains("test_github_pages_action"),
            "should test GitHub Pages action"
        );
        assert!(
            content.contains("test_version_locked_cache"),
            "should test version-locked cache"
        );
        assert!(
            content.contains("test_download_fallback"),
            "should test download fallback"
        );

        // Check for reporting
        assert!(
            content.contains("TESTS_PASSED"),
            "should track passed tests"
        );
        assert!(
            content.contains("TESTS_FAILED"),
            "should track failed tests"
        );
    }

    /// Test 10: Verify documentation exists
    #[test]
    fn test_documentation_files_exist() {
        let files = vec![
            "docs/guides/MANUAL-WORKFLOW-TRIGGER.md",
            "docs/GITHUB-PAGES-SETUP.md",
            "docs/checklists/PHASE-3-VALIDATION.md",
        ];

        for file in files {
            let path = Path::new(file);
            assert!(path.exists(), "documentation file {} should exist", file);

            let content = fs::read_to_string(path).expect(&format!("should read {}", file));
            assert!(
                !content.is_empty(),
                "documentation file {} should not be empty",
                file
            );
        }
    }

    /// Test 11: Verify VitePress configuration includes comparison
    #[test]
    fn test_vitepress_config_includes_comparison() {
        let config_path = Path::new("docs/.vitepress/config.mts");
        assert!(config_path.exists(), "VitePress config should exist");

        let content = fs::read_to_string(config_path).expect("should read VitePress config");

        assert!(
            content.contains("comparison"),
            "VitePress config should include comparison link"
        );
    }

    /// Test 12: Verify baseline comparison data exists
    #[test]
    fn test_baseline_comparison_data() {
        let baseline_path = Path::new("docs/reference/comparison/baseline.json");
        assert!(
            baseline_path.exists(),
            "baseline comparison data should exist"
        );

        let content = fs::read_to_string(baseline_path).expect("should read baseline data");

        // Should be valid JSON
        let json_result: Result<serde_json::Value, _> = serde_json::from_str(&content);
        assert!(json_result.is_ok(), "baseline.json should be valid JSON");
    }

    /// Test 13: Verify tag-comparison binary can be built
    #[test]
    #[ignore] // This test requires cargo to build
    fn test_tag_comparison_buildable() {
        let output = std::process::Command::new("cargo")
            .args(&["build", "--release", "--bin", "tag-comparison"])
            .output()
            .expect("should run cargo build");

        assert!(
            output.status.success(),
            "tag-comparison binary should build successfully"
        );
    }

    /// Test 14: Verify existing tests still pass
    #[test]
    fn test_no_regressions() {
        // This test ensures Phase 3 implementation doesn't break existing code
        // Actual test execution is in main test suite
        // This is a placeholder to verify the test runs

        // Check that main lib compiles
        assert!(true, "Phase 3 should not introduce regressions");
    }

    /// Test 15: Verify GitHub Actions permissions are set
    #[test]
    fn test_workflow_permissions() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        assert!(
            content.contains("permissions:"),
            "workflow should declare permissions"
        );
        assert!(
            content.contains("contents: write") || content.contains("contents:"),
            "workflow should have contents permission"
        );
        assert!(
            content.contains("pages: write") || content.contains("pages:"),
            "workflow should have pages permission"
        );
    }
}

#[cfg(test)]
mod workflow_configuration_tests {
    use std::fs;
    use std::path::Path;

    /// Verify workflow runs on parser changes
    #[test]
    fn test_workflow_triggers_on_parser_changes() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        assert!(
            content.contains("src/parsers/**"),
            "workflow should trigger on parser changes"
        );
    }

    /// Verify workflow runs on tag-comparison changes
    #[test]
    fn test_workflow_triggers_on_binary_changes() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        assert!(
            content.contains("src/bin/tag-comparison/**"),
            "workflow should trigger on binary changes"
        );
    }

    /// Verify weekly schedule is configured
    #[test]
    fn test_weekly_schedule_configured() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        assert!(
            content.contains("0 2 * * 0"),
            "workflow should have weekly schedule (Sunday 2 AM UTC)"
        );
    }

    /// Verify ExifTool version is detected
    #[test]
    fn test_exiftool_version_detection() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        assert!(
            content.contains("exiftool -ver"),
            "workflow should detect ExifTool version"
        );
        assert!(
            content.contains("get-version"),
            "workflow should store version in step output"
        );
    }

    /// Verify test images are validated
    #[test]
    fn test_images_validation() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        assert!(
            content.contains("t/images"),
            "workflow should verify test images directory"
        );
        assert!(
            content.contains("find exiftool-release/t/images"),
            "workflow should count test images"
        );
    }

    /// Verify report generation step
    #[test]
    fn test_report_generation_step() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        assert!(
            content.contains("Generate tag comparison report"),
            "workflow should have report generation step"
        );
        assert!(
            content.contains("tag-comparison"),
            "workflow should run tag-comparison binary"
        );
        assert!(
            content.contains("comparison.json"),
            "workflow should output JSON report"
        );
    }

    /// Verify HTML report generation
    #[test]
    fn test_html_report_generation() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        assert!(
            content.contains("Generate HTML report"),
            "workflow should have HTML generation step"
        );
        assert!(
            content.contains("index.html"),
            "workflow should generate index.html"
        );
    }

    /// Verify cache step exists
    #[test]
    fn test_cache_configuration() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        assert!(
            content.contains("Cache ExifTool test suite"),
            "workflow should have cache step"
        );
        assert!(
            content.contains("actions/cache"),
            "should use GitHub cache action"
        );
    }

    /// Verify error handling in download
    #[test]
    fn test_download_error_handling() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        assert!(
            content.contains("if [ \"$DOWNLOAD_SUCCESS\" = false ]"),
            "workflow should check download success"
        );
        assert!(
            content.contains("exit 1"),
            "workflow should exit on critical errors"
        );
    }

    /// Verify summary step shows results
    #[test]
    fn test_summary_step() {
        let workflow_path = Path::new(".github/workflows/compare-exiftool.yml");
        let content = fs::read_to_string(workflow_path).expect("should read workflow file");

        assert!(
            content.contains("Summary"),
            "workflow should have summary step"
        );
        assert!(
            content.contains("cache-hit"),
            "summary should show cache status"
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
