# OxiDex Justfile
# Run `just` to see available commands
# Run `just <command>` to execute a command

# Default command when running `just` with no arguments
default:
    @just --list

# Run all tests (matches CI exactly)
test:
    @echo "Running all tests (matching CI)..."
    cargo test --release --verbose --all-features

# Run tests in debug mode
test-debug:
    @echo "Running tests in debug mode..."
    cargo test --workspace

# Run tests with output capture disabled
test-nocapture:
    @echo "Running all tests with output..."
    cargo test --release --verbose --all-features -- --nocapture --test-threads=1

# Run only unit tests
test-unit:
    @echo "Running unit tests..."
    cargo test --lib --workspace

# Run only integration tests (excludes comparison tests)
test-integration:
    @echo "Running integration tests..."
    cargo test --test integration --release

# Run ExifTool comparison tests (requires ExifTool installed)
test-comparison:
    @echo "Running ExifTool comparison tests..."
    @echo "Note: Requires 'exiftool' command to be available"
    cargo test --release --features exiftool-comparison -- --nocapture

# Run only doc tests
test-doc:
    @echo "Running doc tests..."
    cargo test --doc --workspace

# Run tests for specific package
test-package package:
    @echo "Running tests for {{package}}..."
    cargo test -p {{package}}

# Run tests for all tag crates
test-tags:
    @echo "Running tests for all tag crates..."
    cargo test -p oxidex-tags -p oxidex-tags-core -p oxidex-tags-camera -p oxidex-tags-media -p oxidex-tags-image -p oxidex-tags-document -p oxidex-tags-specialty

# Build the project in debug mode
build:
    @echo "Building project (debug)..."
    cargo build --workspace

# Build the project in release mode (matches CI configuration)
build-release:
    @echo "Building project (release, matching CI)..."
    cargo build --release --verbose --all-features

# Build just the binary
build-bin:
    @echo "Building binary..."
    cargo build --bin oxidex

# Build release binary
build-bin-release:
    @echo "Building release binary..."
    cargo build --bin oxidex --release

# Build MCP server
build-mcp:
    @echo "Building MCP server..."
    cargo build -p oxidex-mcp

# Build MCP server in release mode
build-mcp-release:
    @echo "Building MCP server (release)..."
    cargo build -p oxidex-mcp --release

# Check the project for errors without building
check:
    @echo "Checking project..."
    cargo check --workspace

# Check with all features
check-all:
    @echo "Checking project with all features..."
    cargo check --workspace --all-features

# Run clippy linter (matches CI configuration)
lint:
    @echo "Running clippy (matching CI)..."
    cargo clippy --all-features -- -D warnings

# Fix clippy warnings automatically
lint-fix:
    @echo "Running clippy with fixes..."
    cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged

# Format code with rustfmt
fmt:
    @echo "Formatting code..."
    cargo fmt --all

# Check if code is formatted
fmt-check:
    @echo "Checking code formatting..."
    cargo fmt --all -- --check

# Clean build artifacts
clean:
    @echo "Cleaning build artifacts..."
    cargo clean

# Clean and rebuild
rebuild: clean build

# Run the binary with arguments
run *args:
    @echo "Running oxidex..."
    cargo run --bin oxidex -- {{args}}

# Run the release binary with arguments
run-release *args:
    @echo "Running oxidex (release)..."
    cargo run --bin oxidex --release -- {{args}}

# Generate and open documentation
docs:
    @echo "Generating documentation..."
    cargo doc --workspace --no-deps --open

# Generate documentation without opening
docs-build:
    @echo "Generating documentation..."
    cargo doc --workspace --no-deps

# Run benchmarks
bench:
    @echo "Running benchmarks..."
    cargo bench --workspace

# Profiling
# ---------

# Simple text-based profiling (recommended, accessible)
profile-simple:
    @echo "Running text-based performance profiling..."
    @./profile_simple.sh

# Profile with flamegraph (requires sudo on macOS, generates SVG)
profile-flamegraph benchmark:
    @echo "Generating flamegraph for {{benchmark}}..."
    @echo "Note: Requires sudo on macOS. Use profile-simple for accessible alternative."
    cargo flamegraph --bench parse_benchmarks --root -o flamegraph-{{benchmark}}.svg -- --bench {{benchmark}}
    @echo "Flamegraph saved to: flamegraph-{{benchmark}}.svg"
    @echo "Convert to text: python3 parse_flamegraph.py flamegraph-{{benchmark}}.svg"

# Convert flamegraph SVG to accessible text
flamegraph-to-text svg:
    @echo "Converting flamegraph to accessible text..."
    python3 parse_flamegraph.py {{svg}}

# Profile a specific benchmark with samply
profile benchmark:
    @echo "Profiling {{benchmark}} benchmark..."
    samply record cargo bench --bench parse_benchmarks {{benchmark}}

# Profile integration benchmarks
profile-integration benchmark:
    @echo "Profiling integration benchmark: {{benchmark}}..."
    samply record cargo bench --bench integration_benchmarks {{benchmark}}

# Profile the CLI binary with arguments
profile-bin *args:
    @echo "Profiling binary with args: {{args}}..."
    cargo build --release
    samply record ./target/release/oxidex {{args}}

# Profile all parse benchmarks (warning: takes a while)
profile-all:
    @echo "Profiling all parse benchmarks..."
    samply record cargo bench --bench parse_benchmarks

# Update dependencies
update:
    @echo "Updating dependencies..."
    cargo update

# Check for outdated dependencies
outdated:
    @echo "Checking for outdated dependencies..."
    cargo outdated

# Run cargo audit for security vulnerabilities
audit:
    @echo "Auditing dependencies..."
    cargo audit

# Install the binary locally
install:
    @echo "Installing oxidex..."
    cargo install --path .

# Uninstall the binary
uninstall:
    @echo "Uninstalling oxidex..."
    cargo uninstall oxidex

# Install the MCP server locally
install-mcp:
    @echo "Installing oxidex-mcp..."
    cargo install --path oxidex-mcp

# Uninstall the MCP server
uninstall-mcp:
    @echo "Uninstalling oxidex-mcp..."
    cargo uninstall oxidex-mcp

# Build Debian package (requires cargo-deb)
deb:
    @echo "Building Debian package..."
    cargo deb

# Build RPM package (requires cargo-generate-rpm)
rpm:
    @echo "Building RPM package..."
    cargo build --release
    cargo generate-rpm

# Run CI checks (matches GitHub Actions CI workflow)
ci: build-release test lint fmt-check
    @echo "All CI checks passed!"
    @echo "✓ Build (release with all features)"
    @echo "✓ Tests (release with all features)"
    @echo "✓ Clippy (all features, deny warnings)"
    @echo "✓ Format check"

# Pre-commit hook: format, lint, test
pre-commit: fmt lint test
    @echo "Pre-commit checks passed!"

# Coverage report (requires cargo-tarpaulin)
coverage:
    @echo "Generating coverage report..."
    cargo tarpaulin --out Html --output-dir coverage --workspace

# Watch for changes and run tests (requires cargo-watch)
watch:
    @echo "Watching for changes..."
    cargo watch -x test

# Watch for changes and run specific command
watch-run cmd:
    @echo "Watching for changes to run: {{cmd}}..."
    cargo watch -x "{{cmd}}"

# Bloat analysis (requires cargo-bloat)
bloat:
    @echo "Analyzing binary bloat..."
    cargo bloat --release -n 20

# Show crate dependency tree
tree:
    @echo "Showing dependency tree..."
    cargo tree

# Show workspace information
workspace:
    @echo "Workspace members:"
    @cargo metadata --format-version 1 --no-deps | jq -r '.workspace_members[]'

# Git commands
# -------------

# Create a new git commit with formatted message
commit message:
    @echo "Creating commit: {{message}}"
    git add -A
    git commit -m "{{message}}"

# Commit and push to current branch
push message:
    @echo "Committing and pushing: {{message}}"
    git add -A
    git commit -m "{{message}}"
    git push origin $(git branch --show-current)

# Show git status
status:
    @git status

# Show recent commits
log:
    @git log --oneline -10

# Tag management
# -------------

# Create and push a new version tag
tag version:
    @echo "Creating tag: v{{version}}"
    git tag -a "v{{version}}" -m "Release v{{version}}"
    git push origin "v{{version}}"

# Delete a tag locally and remotely
untag version:
    @echo "Deleting tag: v{{version}}"
    git tag -d "v{{version}}"
    git push origin :refs/tags/v{{version}}

# macOS packaging
# ----------------

# Create macOS DMG installer
create-dmg version:
    @echo "Creating DMG for {{version}}..."
    mkdir -p dist/dmg-contents
    cp target/release/oxidex dist/dmg-contents/
    create-dmg \
      --volname "OxiDex {{version}}" \
      --no-internet-enable \
      --skip-jenkins \
      "dist/oxidex-{{version}}.dmg" \
      "dist/dmg-contents/"
    rm -rf dist/dmg-contents
    @echo "DMG created at dist/oxidex-{{version}}.dmg"

# Release workflow
# ----------------

# Prepare for release: run all checks
release-check: ci
    @echo "Release checks passed!"
    @echo "Ready for release."

# Full release workflow: check, build release, create tag
release version: release-check
    @echo "Creating release v{{version}}..."
    cargo build --release
    just tag {{version}}
    @echo "Release v{{version}} created and tagged!"
