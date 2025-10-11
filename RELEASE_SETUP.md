# Release Automation Setup Guide

This guide explains how to set up automated releases with Homebrew tap updates using a GitHub App.

## Overview

The release workflow automatically:
1. Builds binaries for all supported platforms (Linux x86_64/ARM64, macOS x86_64/ARM64, Windows x86_64)
2. Creates a GitHub release with all binaries
3. Publishes to crates.io
4. **Automatically updates the Homebrew tap repository** with new formula

## Prerequisites

You need to set up the following secrets in your repository:
- `CARGO_REGISTRY_TOKEN` - For publishing to crates.io
- `TAP_UPDATE_APP_ID` - GitHub App ID
- `TAP_UPDATE_APP_PRIVATE_KEY` - GitHub App private key

## Setting Up the GitHub App

### Step 1: Create the GitHub App

1. Go to your GitHub account settings: https://github.com/settings/apps
2. Click **"New GitHub App"**
3. Fill in the required information:
   - **GitHub App name**: `awsom-tap-updater` (or any name you prefer)
   - **Homepage URL**: `https://github.com/oleksiimorozenko/awsom`
   - **Webhook**: Uncheck "Active" (we don't need webhooks)

### Step 2: Configure Permissions

Under **Repository permissions**, set:
- **Contents**: `Read and write` (required to clone, commit, and push to the tap repository)
- **Metadata**: `Read-only` (automatically selected, required for all apps)

All other permissions can remain at "No access".

### Step 3: Set Installation

Under **Where can this GitHub App be installed?**
- Select **"Only on this account"** (for personal use)

### Step 4: Create the App

1. Click **"Create GitHub App"**
2. You'll be redirected to the app's settings page

### Step 5: Generate Private Key

1. On the app settings page, scroll down to **"Private keys"**
2. Click **"Generate a private key"**
3. A `.pem` file will be downloaded - keep this secure!

### Step 6: Note the App ID

1. At the top of the app settings page, note the **App ID** (it's a number like `123456`)

### Step 7: Install the App

1. In the left sidebar, click **"Install App"**
2. Click **"Install"** next to your username
3. Select **"Only select repositories"**
4. Choose **`homebrew-tap`** repository
5. Click **"Install"**

## Adding Secrets to Your Repository

### Step 1: Add App ID

1. Go to your `awsom` repository: https://github.com/oleksiimorozenko/awsom
2. Click **Settings** → **Secrets and variables** → **Actions**
3. Click **"New repository secret"**
4. Name: `TAP_UPDATE_APP_ID`
5. Value: The App ID from Step 6 above (e.g., `123456`)
6. Click **"Add secret"**

### Step 2: Add Private Key

1. Open the `.pem` file you downloaded in a text editor
2. Copy the **entire contents** including the `-----BEGIN RSA PRIVATE KEY-----` and `-----END RSA PRIVATE KEY-----` lines
3. In the same secrets page, click **"New repository secret"**
4. Name: `TAP_UPDATE_APP_PRIVATE_KEY`
5. Value: Paste the entire private key
6. Click **"Add secret"**

### Step 3: Add Cargo Token (if not already done)

If you haven't already added your crates.io token:

1. Get your token from https://crates.io/me
2. Click **"New repository secret"**
3. Name: `CARGO_REGISTRY_TOKEN`
4. Value: Your crates.io token
5. Click **"Add secret"**

## Creating a Release

Once everything is set up, creating a release is simple:

```bash
# Make sure you're on main and up to date
git checkout main
git pull

# Create and push a version tag
git tag v0.1.0
git push origin v0.1.0
```

The workflow will automatically:
1. ✅ Build all platform binaries
2. ✅ Create a GitHub release
3. ✅ Upload all binaries to the release
4. ✅ Publish to crates.io
5. ✅ Update the Homebrew tap with the new version and SHA256 checksums

## Verification

After the release workflow completes:

1. Check the GitHub release: https://github.com/oleksiimorozenko/awsom/releases
2. Check the tap repository was updated: https://github.com/oleksiimorozenko/homebrew-tap
3. Test the Homebrew installation:
   ```bash
   brew update
   brew upgrade awsom
   # or for first install
   brew install oleksiimorozenko/tap/awsom
   ```

## Troubleshooting

### "Resource not accessible by integration" error

- Make sure the GitHub App is installed on the `homebrew-tap` repository
- Verify the app has "Contents: Read and write" permission
- Check that the App ID and private key are correct

### "Bad credentials" error

- Verify the private key was copied completely (including BEGIN/END lines)
- Make sure there are no extra spaces or newlines
- Regenerate the private key if needed

### Tap update fails but release succeeds

- The release and crates.io publish will succeed even if tap update fails
- Check the workflow logs for specific errors
- You can manually update the tap as a fallback

## Benefits of Using GitHub App vs PAT

✅ **Better security**: Scoped to specific repositories
✅ **Fine-grained permissions**: Only has access to what it needs
✅ **Better audit logs**: Actions are attributed to the app
✅ **No expiration**: Unlike PATs that expire and need rotation
✅ **Organization-friendly**: Can be shared across teams
