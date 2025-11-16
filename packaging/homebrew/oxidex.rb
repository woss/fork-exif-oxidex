# Homebrew Formula for oxidex
# To install: brew install --build-from-source /path/to/oxidex.rb
# To publish: Submit PR to homebrew/homebrew-core after stable release

class Oxidex < Formula
  desc "Modern, high-performance Rust implementation of ExifTool for metadata extraction"
  homepage "https://github.com/oxidex/oxidex"
  url "https://github.com/oxidex/oxidex/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "UPDATE_THIS_SHA256_AFTER_RELEASE" # Run: curl -sL <url> | shasum -a 256
  license "GPL-3.0"
  head "https://github.com/oxidex/oxidex.git", branch: "main"

  depends_on "rust" => :build

  def install
    # Build and install using cargo
    system "cargo", "install", *std_cargo_args

    # Generate shell completions (if implemented)
    # generate_completions_from_executable(bin/"oxidex", "completions")
  end

  test do
    # Test that the binary runs and outputs version
    assert_match version.to_s, shell_output("#{bin}/oxidex --version")

    # Test basic functionality with a test file
    (testpath/"test.txt").write("test file")
    system "#{bin}/oxidex", "--help"
  end
end

# Installation Instructions:
#
# For Local Testing (before release):
# 1. Build the project: cargo build --release
# 2. Install from local formula:
#    brew install --build-from-source ./packaging/homebrew/oxidex.rb
#
# After Official Release (v0.1.0+):
# 1. Update the sha256 hash:
#    curl -sL https://github.com/oxidex/oxidex/archive/refs/tags/v0.1.0.tar.gz | shasum -a 256
# 2. Replace "UPDATE_THIS_SHA256_AFTER_RELEASE" with the actual hash
# 3. Test installation:
#    brew install --build-from-source ./packaging/homebrew/oxidex.rb
# 4. For public distribution, submit to homebrew-core:
#    https://docs.brew.sh/How-To-Open-a-Homebrew-Pull-Request
#
# For Binary Bottles (future enhancement):
# Add bottle blocks for pre-compiled binaries:
#   bottle do
#     sha256 cellar: :any_skip_relocation, arm64_sonoma: "..."
#     sha256 cellar: :any_skip_relocation, arm64_ventura: "..."
#     sha256 cellar: :any_skip_relocation, sonoma: "..."
#     sha256 cellar: :any_skip_relocation, ventura: "..."
#     sha256 cellar: :any_skip_relocation, x86_64_linux: "..."
#   end
