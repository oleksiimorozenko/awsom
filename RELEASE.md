# Release Process

This document describes the release process for awsom.

## Prerequisites

Before creating a release, ensure you have:

1. **GitHub Secrets** configured in your repository settings:
   - `CARGO_REGISTRY_TOKEN`: Token from crates.io for publishing
     - Get it from https://crates.io/me
     - Go to Settings > Secrets and variables > Actions
     - Create new secret named `CARGO_REGISTRY_TOKEN`

## Semantic Versioning

awsom follows [Semantic Versioning](https://semver.org/):

- **MAJOR** version (`1.0.0`): Incompatible API changes
- **MINOR** version (`0.1.0`): New features, backwards compatible
- **PATCH** version (`0.1.1`): Bug fixes, backwards compatible

### Version Guidelines

- `v0.1.x` - Initial development releases
- `v0.x.0` - Feature additions before 1.0
- `v1.0.0` - First stable release with public API guarantee
- `v1.x.0` - New features (backwards compatible)
- `v1.0.x` - Bug fixes only

## Creating a Release

### 1. Update Version Numbers

Update the version in `Cargo.toml`:

```toml
[package]
version = "0.2.0"  # Change this
```

### 2. Update CHANGELOG.md

Add release notes:

```markdown
## [0.2.0] - 2025-01-15

### Added
- New feature X
- New feature Y

### Changed
- Improved Z

### Fixed
- Bug fix A
```

### 3. Commit Changes

```bash
git add Cargo.toml CHANGELOG.md
git commit -m "chore: bump version to v0.2.0"
git push origin main
```

### 4. Create and Push Tag

```bash
# Create annotated tag
git tag -a v0.2.0 -m "Release v0.2.0"

# Push tag to trigger release workflow
git push origin v0.2.0
```

### 5. Monitor Release Process

The GitHub Actions workflow will automatically:

1. **Run Tests**: Execute full test suite on all platforms
2. **Build Binaries**: Compile for all target platforms
   - Linux (x86_64, ARM64)
   - macOS (Intel, Apple Silicon)
   - Windows (x86_64)
3. **Create GitHub Release**: Generate release with release notes
4. **Upload Artifacts**: Attach binaries and checksums
5. **Publish to crates.io**: Upload package to Rust registry

Check progress at: `https://github.com/oleksiimorozenko/awsom/actions`

### 6. Update Homebrew Formula

After the release is published, update the Homebrew formula:

1. Download the `.sha256` files from the release
2. Update `Formula/awsom.rb` with new version and SHA256 checksums
3. Commit and push to your homebrew-tap repository

## Troubleshooting

### Release Workflow Fails

**crates.io publish fails:**
- Ensure `CARGO_REGISTRY_TOKEN` secret is set correctly
- Verify you're an owner of the crate on crates.io
- Check that version number hasn't already been published

**Build fails:**
- Check CI workflow first (tests must pass)
- Review build logs in GitHub Actions
- Ensure Cargo.toml and Cargo.lock are up to date

### Version Conflicts

If you need to delete a tag:

```bash
# Delete local tag
git tag -d v0.2.0

# Delete remote tag
git push origin :refs/tags/v0.2.0
```

## Artifact Signing

Release artifacts include SHA256 checksums for verification:

```bash
# Verify downloaded binary (Linux example)
sha256sum -c awsom-linux-amd64.tar.gz.sha256

# Should output: awsom-linux-amd64.tar.gz: OK
```

## Publishing to Homebrew

### Initial Setup

Create a Homebrew tap repository:

```bash
# Create new repo: homebrew-tap
gh repo create oleksiimorozenko/homebrew-tap --public

# Clone and add formula
git clone https://github.com/oleksiimorozenko/homebrew-tap.git
mkdir -p homebrew-tap/Formula
cp Formula/awsom.rb homebrew-tap/Formula/
cd homebrew-tap
git add Formula/awsom.rb
git commit -m "Add awsom formula"
git push
```

### After Each Release

1. Download SHA256 checksums from GitHub release
2. Update `Formula/awsom.rb` with:
   - New version number
   - Updated SHA256 hashes for each platform
3. Test the formula:
   ```bash
   brew install --build-from-source oleksiimorozenko/tap/awsom
   brew test oleksiimorozenko/tap/awsom
   ```
4. Commit and push to homebrew-tap

## Publishing to crates.io

### First-Time Setup

1. Create account at https://crates.io
2. Get API token from https://crates.io/me
3. Add to GitHub Secrets as `CARGO_REGISTRY_TOKEN`

### Manual Publish (if needed)

```bash
# Login to crates.io
cargo login

# Publish (done automatically by release workflow)
cargo publish
```

## Release Checklist

- [ ] Update version in `Cargo.toml`
- [ ] Update `CHANGELOG.md` with release notes
- [ ] Commit changes
- [ ] Create and push tag `vX.Y.Z`
- [ ] Monitor GitHub Actions workflow
- [ ] Verify GitHub release is created
- [ ] Verify crates.io publish succeeded
- [ ] Update Homebrew formula
- [ ] Test installation from all sources
- [ ] Announce release (optional)

## Pre-release Versions

For testing before official release:

```bash
# Create pre-release tag
git tag -a v0.2.0-rc.1 -m "Release candidate 1"
git push origin v0.2.0-rc.1
```

The release workflow will mark it as a pre-release automatically.
