# Homebrew Formula for awsom
# This file should be placed in a homebrew-tap repository
# Repository: https://github.com/oleksiimorozenko/homebrew-tap
# Then users can install with: brew install oleksiimorozenko/tap/awsom

class Awsom < Formula
  desc "k9s-like TUI for AWS SSO sessions"
  homepage "https://github.com/oleksiimorozenko/awsom"
  version "0.1.0"
  license "Apache-2.0"

  if OS.mac? && Hardware::CPU.arm?
    url "https://github.com/oleksiimorozenko/awsom/releases/download/v#{version}/awsom-macos-arm64.tar.gz"
    sha256 "REPLACE_WITH_ACTUAL_SHA256_FOR_MACOS_ARM64"
  elsif OS.mac? && Hardware::CPU.intel?
    url "https://github.com/oleksiimorozenko/awsom/releases/download/v#{version}/awsom-macos-amd64.tar.gz"
    sha256 "REPLACE_WITH_ACTUAL_SHA256_FOR_MACOS_AMD64"
  elsif OS.linux? && Hardware::CPU.arm?
    url "https://github.com/oleksiimorozenko/awsom/releases/download/v#{version}/awsom-linux-arm64.tar.gz"
    sha256 "REPLACE_WITH_ACTUAL_SHA256_FOR_LINUX_ARM64"
  elsif OS.linux? && Hardware::CPU.intel?
    url "https://github.com/oleksiimorozenko/awsom/releases/download/v#{version}/awsom-linux-amd64.tar.gz"
    sha256 "REPLACE_WITH_ACTUAL_SHA256_FOR_LINUX_AMD64"
  end

  def install
    bin.install "awsom"

    # Generate shell completions
    generate_completions_from_executable(bin/"awsom", "completions")
  end

  test do
    assert_match "awsom", shell_output("#{bin}/awsom --version")
  end
end
