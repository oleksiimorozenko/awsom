#!/usr/bin/env bash
# Development environment setup script for awsom

set -e

echo "🚀 Setting up awsom development environment..."

# Check for Rust installation
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust is not installed. Please install it from: https://rustup.rs/"
    exit 1
fi

echo "✓ Rust found: $(rustc --version)"

# Install rustfmt and clippy if not already installed
echo "📦 Installing Rust components..."
rustup component add rustfmt clippy

# Check for pre-commit
if ! command -v pre-commit &> /dev/null; then
    echo "⚠️  pre-commit not found. Installing..."

    # Try to install with pip
    if command -v pip3 &> /dev/null; then
        pip3 install pre-commit
    elif command -v pip &> /dev/null; then
        pip install pre-commit
    elif command -v brew &> /dev/null; then
        brew install pre-commit
    else
        echo "❌ Could not install pre-commit. Please install manually:"
        echo "   pip install pre-commit"
        echo "   or: brew install pre-commit"
        exit 1
    fi
fi

echo "✓ pre-commit found: $(pre-commit --version)"

# Install pre-commit hooks
echo "🪝 Installing pre-commit hooks..."
pre-commit install
pre-commit install --hook-type commit-msg

# Run pre-commit on all files to ensure everything passes
echo "🔍 Running pre-commit checks..."
pre-commit run --all-files || {
    echo "⚠️  Some checks failed. This is normal on first run."
    echo "   Run 'cargo fmt' to fix formatting issues."
}

# Build the project
echo "🔨 Building project..."
cargo build

# Run tests
echo "🧪 Running tests..."
cargo test

echo ""
echo "✅ Development environment setup complete!"
echo ""
echo "Quick start commands:"
echo "  cargo build          # Build the project"
echo "  cargo test           # Run tests"
echo "  cargo fmt            # Format code"
echo "  cargo clippy         # Run linter"
echo "  cargo run            # Run awsom"
echo "  pre-commit run       # Run all pre-commit hooks"
echo ""
echo "Happy coding! 🎉"
