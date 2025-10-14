# awsom

[![CI](https://github.com/oleksiimorozenko/awsom/actions/workflows/ci.yml/badge.svg)](https://github.com/oleksiimorozenko/awsom/actions/workflows/ci.yml)
[![Release](https://github.com/oleksiimorozenko/awsom/actions/workflows/release.yml/badge.svg)](https://github.com/oleksiimorozenko/awsom/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/awsom.svg)](https://crates.io/crates/awsom)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)

**AWS Organization Manager** - The awesome way to manage AWS SSO sessions.

A modern, k9s-inspired Terminal User Interface (TUI) for managing AWS SSO sessions written in Rust.

## Features

- **Single SSO Login**: Authenticate once, access multiple AWS accounts and roles
- **Interactive TUI**: k9s-style interface for managing sessions with visual indicators
- **CLI Commands**: Full command-line interface for automation and scripting
- **Session Management**: Create, edit, delete, and switch SSO sessions via CLI or TUI
- **Multi-Session Support**: Track multiple SSO sessions across different organizations
- **Status Checking**: Programmatic session status for shell automation and scripting
- **Expiry Tracking**: Real-time countdown timers for token and credential expiration
- **Profile Management**: Create, rename, and manage AWS credential profiles
- **Console Access**: One-click federated sign-in to AWS Console in your browser
- **Default Profile**: Set and switch default AWS profile easily
- **Profile Export**: Export credentials as environment variables or to ~/.aws/credentials
- **AWS CLI Compatible**: Uses same cache directories and format as AWS CLI v2

## No AWS CLI Required! 🎉

**awsom** is a standalone tool that does NOT require the AWS CLI to be installed. It uses the official AWS SDK for Rust to communicate directly with AWS services and manages your `~/.aws/config` and `~/.aws/credentials` files as plain text.

This means:
- **Faster**: No Python runtime or AWS CLI overhead
- **Simpler**: One binary, zero dependencies (besides the AWS SDK)
- **Compatible**: Works alongside AWS CLI if you have it, but doesn't need it
- **Portable**: Easy to install on any system without package managers

If you have existing AWS CLI configurations, awsom will read and respect them. If you don't, awsom will create everything you need from scratch.

## Installation

### Using Cargo (Recommended)

Install from [crates.io](https://crates.io/crates/awsom):

```bash
cargo install awsom
```

### Using Homebrew (macOS/Linux)

**Linux Prerequisites:**
On Linux systems, you need to install `build-essential` before using Homebrew, even though awsom provides pre-built binaries. This is a Homebrew requirement because some of Homebrew's own dependencies may need to be compiled from source:

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y build-essential

# Then install awsom
brew install oleksiimorozenko/tap/awsom
```

**macOS:**
```bash
brew install oleksiimorozenko/tap/awsom
```

**Why build-essential on Linux?**
Homebrew on Linux requires compiler tools (`gcc`, `make`, etc.) to be available on the system. According to the [Homebrew on Linux documentation](https://docs.brew.sh/Homebrew-on-Linux), "Homebrew does not use any libraries provided by your host system, except glibc and gcc if they are new enough." While Homebrew provides pre-compiled binaries (bottles) for most packages, [some dependencies will need to be built directly on your machine](https://www.digitalocean.com/community/tutorials/how-to-install-and-use-homebrew-on-linux), which requires a working compiler environment.

The `build-essential` package provides all the necessary compilation tools including GCC, GNU Make, and other essential development utilities.

### Download Pre-built Binaries

Download the latest release for your platform from the [releases page](https://github.com/oleksiimorozenko/awsom/releases):

- **Linux (x86_64)**: `awsom-linux-amd64.tar.gz`
- **Linux (ARM64)**: `awsom-linux-arm64.tar.gz`
- **macOS (Intel)**: `awsom-macos-amd64.tar.gz`
- **macOS (Apple Silicon)**: `awsom-macos-arm64.tar.gz`
- **Windows (x86_64)**: `awsom-windows-amd64.zip`

After downloading, extract and verify the checksum:

```bash
# Example for Linux x86_64
tar xzf awsom-linux-amd64.tar.gz
sha256sum -c awsom-linux-amd64.tar.gz.sha256

# Move to PATH
sudo mv awsom /usr/local/bin/
```

### From Source

```bash
git clone https://github.com/oleksiimorozenko/awsom.git
cd awsom
cargo install --path .
```

### Prerequisites

- Rust 1.70+ (for building from source only)
- AWS SSO enabled for your organization (no AWS CLI required - awsom handles configuration)

### Shell Completion

Enable tab completion for your shell to make the CLI much easier to use:

#### Bash
```bash
# Add to ~/.bashrc or ~/.bash_profile:
eval "$(awsom completions bash)"

# Or save to completion directory:
awsom completions bash | sudo tee /usr/local/etc/bash_completion.d/awsom
```

#### Zsh (macOS default)
```bash
# Quick setup - add to ~/.zshrc:
eval "$(awsom completions zsh)"

# Or for better performance, save to completion directory:
mkdir -p ~/.zfunc
awsom completions zsh > ~/.zfunc/_awsom

# Then add to ~/.zshrc (if not already there):
fpath=(~/.zfunc $fpath)
autoload -Uz compinit && compinit
```

#### Fish
```bash
# Save to fish completion directory:
awsom completions fish > ~/.config/fish/completions/awsom.fish
```

#### PowerShell
```powershell
# Add to your PowerShell profile:
awsom completions powershell | Out-String | Invoke-Expression
```

After setting up completions, you can use Tab to autocomplete commands, options, and values!

## Quick Start

### 1. Login to AWS SSO

```bash
awsom login \
  --start-url https://your-org.awsapps.com/start \
  --region us-east-1
```

Or set environment variables:

```bash
export AWS_SSO_START_URL=https://your-org.awsapps.com/start
export AWS_SSO_REGION=us-east-1
awsom login
```

### 2. List Available Accounts and Roles

```bash
# Human-readable format
awsom list

# JSON format
awsom list --format json
```

### 3. Launch TUI

```bash
awsom
```

The TUI provides a k9s-style interactive interface for managing AWS SSO sessions.

**Keyboard Shortcuts:**
- `q` or `Esc` - Quit application
- `?` or `F1` - Show help screen
- `l` - Login/Logout (toggle)
- `r` - Refresh account/role list
- `↑`/`k` - Move selection up
- `↓`/`j` - Move selection down
- `Enter` - Start/stop session for selected role (activates or invalidates credentials)
- `p` - Edit profile name for selected role
- `d` - Set selected role's profile as default
- `c` - Open AWS Console in browser for selected role

**Features:**
- **Visual Indicators**: 🟢 Active sessions / 🔴 Inactive sessions
- **Default Profile Marker**: ✓ shows which profile is set as default
- **Expiration Countdown**: Real-time display of remaining session time
- **Automatic Session Loading**: Auto-loads cached SSO sessions on startup
- **Profile Management**: Create, rename, and delete AWS credential profiles
- **Console Access**: One-key access to AWS Console with federated sign-in

**Setup:**
1. Launch TUI: `awsom`
2. Press `l` to login
3. Follow the interactive prompts to configure your SSO (if not already configured)
4. Authenticate in your browser and start managing your AWS sessions!

## CLI Commands

> 📖 **For complete command reference with session resolution logic and examples, see [COMMANDS.md](COMMANDS.md)**

### Global Options

All commands support these global flags:
- `-v, --verbose`: Enable debug logging to see detailed operation information
- `--start-url <URL>`: SSO start URL (or set `AWS_SSO_START_URL`)
- `--region <REGION>`: AWS region for SSO (or set `AWS_SSO_REGION`)
- `--headless`: Headless mode - don't try to open browser (auto-detected in SSH/Docker)

### `login` - Authenticate with AWS SSO

```bash
awsom login [--start-url URL] [--region REGION] [--force] [--verbose]
```

Options:
- `--force`: Force re-authentication even if token is valid
- `--verbose`: Show debug information during authentication

Example with verbose output:
```bash
awsom -v login --start-url https://your-org.awsapps.com/start --region us-east-1
```

### `list` - List accounts and roles

```bash
awsom list [--format text|json]
```

### `logout` - End SSO session

```bash
awsom logout
```

### `exec` - Execute command with credentials

```bash
awsom exec --role-name Developer --account-name Production -- aws s3 ls
```

Options:
- `--account-id <ID>`: Account ID
- `--account-name <NAME>`: Account name (alternative to account-id)
- `--role-name <ROLE>`: Role name
- Command follows `--`

### `export` - Export credentials

```bash
# Export as environment variables
awsom export --role-name Developer --account-name Production
eval $(awsom export --role-name Developer --account-name Production)

# Or write to AWS credentials file
awsom export --role-name Developer --account-name Production --profile my-profile
```

Options:
- `--account-id <ID>`: Account ID
- `--account-name <NAME>`: Account name (alternative to account-id)
- `--role-name <ROLE>`: Role name
- `--profile <NAME>`: Write to ~/.aws/credentials as this profile

### `console` - Open AWS Console in browser

```bash
awsom console --role-name Developer --account-name Production
```

Opens the AWS Console in your default browser using federated sign-in with temporary credentials.

Options:
- `--account-id <ID>`: Account ID
- `--account-name <NAME>`: Account name (alternative to account-id)
- `--role-name <ROLE>`: Role name
- `--region <REGION>`: AWS region to open console in (defaults to profile default or SSO region)

### `status` - Check SSO session status

```bash
# Human-readable output
awsom status

# JSON output for scripting
awsom status --json
```

Check if your SSO session is active. Returns exit code 0 if active, 1 if not. Perfect for automation and shell scripts.

**Output Examples:**

Text format:
```
SSO session active (expires in 120 minutes)
SSO session expired
No SSO session found
SSO not configured
```

JSON format:
```json
{"active":true,"expires_in_minutes":120}
{"active":false,"reason":"expired"}
{"active":false,"reason":"no_session"}
{"active":false,"reason":"not_configured"}
```

**Shell Automation Example:**

Add this to your `~/.zshrc` or `~/.bashrc` to automatically manage your SSO sessions and common profiles:

```bash
# awsom - Automatic SSO session and profile management
awsom-auto() {
    # Check if SSO session is active
    if awsom status --json 2>/dev/null | grep -q '"active":true'; then
        echo "✓ SSO session active"

        # Export your commonly-used profiles in parallel
        # Adjust these to your actual account names and roles
        awsom export --account-name Production --role-name Developer --profile prod-dev &
        awsom export --account-name Staging --role-name Developer --profile stage-dev &
        awsom export --account-name Testing --role-name ReadOnly --profile test-ro &
        wait

        echo "✓ AWS profiles exported: prod-dev, stage-dev, test-ro"
    else
        echo "⚠ No active SSO session, logging in..."
        awsom login

        # After login, call this function again to export profiles
        if [ $? -eq 0 ]; then
            awsom-auto
        fi
    fi
}

# Optional: Run automatically on shell startup (comment out if too aggressive)
# awsom-auto
```

Then just run `awsom-auto` in your terminal to ensure your SSO session and profiles are ready!

### `session` - Manage SSO sessions

**Perfect for automation, CI/CD, and provisioning scripts!**

The `session` subcommand provides complete CLI management of SSO sessions without requiring the TUI.

#### `session add` - Add a new SSO session

```bash
awsom session add \
  --name my-org-sso \
  --start-url https://my-org.awsapps.com/start \
  --region us-east-1
```

Creates a new SSO session configuration and saves it to `~/.aws/config`. Great for:
- **Provisioning scripts**: Automate setup for new team members
- **CI/CD pipelines**: Configure AWS access in build environments
- **Infrastructure as Code**: Manage SSO configuration declaratively

#### `session list` - List all SSO sessions

```bash
# Human-readable format
awsom session list

# JSON format for scripting
awsom session list --format json
```

Example output (text):
```
SSO Sessions (2):

  production-sso
    Start URL: https://prod.awsapps.com/start
    Region: us-east-1

  staging-sso
    Start URL: https://stage.awsapps.com/start
    Region: us-west-2
```

Example output (JSON):
```json
[
  {
    "name": "production-sso",
    "start_url": "https://prod.awsapps.com/start",
    "region": "us-east-1",
    "registration_scopes": "sso:account:access"
  },
  {
    "name": "staging-sso",
    "start_url": "https://stage.awsapps.com/start",
    "region": "us-west-2",
    "registration_scopes": "sso:account:access"
  }
]
```

#### `session delete` - Delete an SSO session

```bash
# Interactive confirmation
awsom session delete my-org-sso

# Force deletion without confirmation (for scripts)
awsom session delete my-org-sso --force
```

Removes the session from `~/.aws/config`. Use `--force` in automation scripts to skip the confirmation prompt.

#### `session edit` - Edit an existing SSO session

```bash
# Update start URL
awsom session edit my-org-sso \
  --start-url https://new-url.awsapps.com/start

# Update region
awsom session edit my-org-sso \
  --region us-west-2

# Update both
awsom session edit my-org-sso \
  --start-url https://new-url.awsapps.com/start \
  --region us-west-2
```

Updates an existing session configuration. You'll need to re-authenticate after changing the start URL.

#### `session switch` - Switch active session

```bash
awsom session switch my-org-sso
```

Selects which SSO session to use (placeholder for future multi-session support). For now, use the TUI to switch between sessions interactively.

**Automation Example:**

```bash
#!/bin/bash
# setup-aws-sso.sh - Provision AWS SSO for new environment

# Add SSO sessions for different environments
awsom session add \
  --name prod-sso \
  --start-url https://prod.awsapps.com/start \
  --region us-east-1

awsom session add \
  --name stage-sso \
  --start-url https://stage.awsapps.com/start \
  --region us-west-2

# List configured sessions
awsom session list --format json | jq '.[] | .name'

# Authenticate with production
awsom login --start-url https://prod.awsapps.com/start --region us-east-1

# Export common profiles
awsom export --account-name Production --role-name Developer --profile prod-dev
```

### `import` - Import existing configurations to awsom management

**Migrate your existing AWS configurations to awsom's automatic organization!**

The `import` command allows you to move existing SSO sessions and profiles from the user-managed section to awsom's managed section, where they will be automatically organized and sorted.

#### Why use import?

When you first start using awsom with existing AWS configurations, awsom creates marker lines in your `~/.aws/config` file to separate:
- **User-managed sections** (above the marker) - Your existing configs that awsom won't touch
- **Awsom-managed sections** (below the marker) - Automatically organized with alphabetical sorting

The import command helps you migrate your existing configurations to awsom management, giving you:
- ✅ Automatic alphabetical sorting
- ✅ Consistent formatting
- ✅ Integration with awsom's TUI
- ✅ Collision detection to prevent overwrites

#### Import an SSO session

```bash
# Interactive import with preview
awsom import SA-SSO --section-type sso-session

# Force import without confirmation (for scripts)
awsom import SA-SSO --section-type sso-session --force
```

Example output:
```
Found sso-session to import:

[sso-session SA-SSO]
sso_start_url = https://seeking-alpha.awsapps.com/start
sso_region = us-west-2
sso_registration_scopes = sso:account:access

Move this sso-session to awsom management? (y/N): y
✓ Imported SSO session 'SA-SSO' to awsom management

The sso-session has been moved from user-managed to awsom-managed section.
It will now be automatically organized and sorted by awsom.
```

#### Import a profile

```bash
# Import a profile
awsom import my-profile --section-type profile

# Or just omit --section-type (defaults to profile)
awsom import my-profile
```

**Use Cases:**
- **Migrating to awsom**: Import your existing AWS configs when you start using awsom
- **Team standardization**: Import individual configs into awsom's managed format
- **Cleanup**: Let awsom organize and sort your existing configurations

**How it works:**
1. Finds the section in the user-managed area (above marker)
2. Shows you a preview and asks for confirmation (unless `--force`)
3. Removes it from user-managed area
4. Adds it to awsom-managed area with automatic sorting
5. Your configuration is now managed by awsom!

**Config File Structure:**

Before import:
```ini
# Your existing config
[sso-session SA-SSO]
...

[profile my-profile]
...

# ==================== Managed by awsom ====================
# (awsom's organized sections)
```

After import:
```ini
# Your other configs
...

# ==================== Managed by awsom ====================
[sso-session SA-SSO]  ← Now managed and sorted by awsom
...
```

### `completions` - Generate shell completions

```bash
awsom completions <SHELL>
```

Generate shell completion scripts for bash, zsh, fish, powershell, or elvish.
See [Shell Completion](#shell-completion) section for installation instructions.

## Configuration

awsom uses `~/.aws/config` as the single source of truth for SSO configuration, following AWS CLI v2 conventions. No separate configuration file is needed!

### Interactive Configuration

When you first run awsom and press 'l' to login, if no SSO configuration exists, you'll be guided through an interactive 3-step wizard that will:

1. Ask for your **SSO Start URL** (e.g., `https://your-org.awsapps.com/start`)
2. Ask for your **SSO Region** (e.g., `us-east-1`)
3. Ask for an optional **Session Name** (default: `default-sso`)

The configuration will be automatically saved to `~/.aws/config` as a `[sso-session]` section.

### Manual Configuration

You can also manually edit `~/.aws/config` to add or update SSO sessions:

```ini
[sso-session my-sso]
sso_start_url = https://your-org.awsapps.com/start
sso_region = us-east-1
sso_registration_scopes = sso:account:access
```

Or use the AWS CLI to configure SSO:

```bash
aws configure sso-session
```

### Environment Variables

You can override SSO configuration with environment variables:

- `AWS_SSO_START_URL`: SSO start URL
- `AWS_SSO_REGION`: SSO region

### Configuration Priority

Settings are loaded in this order (later sources override earlier ones):

1. `~/.aws/config` `[sso-session]` sections
2. Environment variables (`AWS_SSO_START_URL`, `AWS_SSO_REGION`)
3. CLI flags (`--start-url`, `--region`)

## Cache Locations

Compatible with AWS CLI v2:

- SSO tokens: `~/.aws/sso/cache/`
- Role credentials: `~/.aws/cli/cache/`

## Project Structure

```
awsom/
├── src/
│   ├── auth/           # SSO OIDC authentication & token caching
│   ├── credentials/    # Credential fetching and caching
│   ├── aws_config.rs   # AWS credentials file management
│   ├── console/        # AWS Console federated sign-in
│   ├── session/        # Session management
│   ├── ui/             # TUI components (Ratatui)
│   │   └── app.rs      # Main TUI application
│   ├── cli/            # CLI commands
│   │   └── commands/   # Individual command implementations
│   ├── config/         # Configuration management
│   ├── expiry/         # Expiry tracking utilities
│   ├── models.rs       # Core data models
│   ├── error.rs        # Error types
│   └── main.rs         # Application entry point
├── Cargo.toml
├── CHANGELOG.md
└── README.md
```

## Development Status

### ✅ Implemented & Tested
- Project structure and dependencies
- Error handling framework with proper AWS SDK error types
- Core data models (SsoToken, RoleCredentials, AccountRole, etc.)
- AWS SSO OIDC authentication (device flow) ✅ **Working**
- Token caching (AWS CLI v2 compatible) ✅ **Working**
- Credential fetching from AWS SSO ✅ **Working**
- AWS credentials file management (read/write/delete) ✅ **Working**
- CLI interface with clap
- `login` command ✅ **Working**
- `list` command ✅ **Working**
- `logout` command ✅ **Working**
- `exec` command for running commands with credentials ✅ **Working**
- `export` command for credential export ✅ **Working**
- `console` command for opening AWS Console in browser ✅ **Working**
- `status` command for session checking and automation ✅ **Working**
- `session` command for managing SSO sessions via CLI ✅ **Working**
  - `session add` for creating sessions programmatically
  - `session list` with text/JSON output
  - `session delete` with force flag for automation
  - `session edit` for updating session configuration
  - `session switch` for multi-session support (WIP)
- `completions` command for shell completion ✅ **Working**
- Verbose/debug logging with `--verbose` flag ✅ **Working**
- TUI interface with Ratatui ✅ **Working**
  - k9s-style keyboard navigation (j/k, arrows)
  - Account/role list display with status indicators
  - Visual indicators (🟢 active / 🔴 inactive)
  - Default profile marker (✓)
  - Real-time expiration countdown
  - Help screen
  - Status bar with token expiry
  - Profile creation and deletion (Enter key)
  - Profile renaming (p key)
  - Set default profile (d key)
  - Open AWS Console in browser (c key)
  - Login/logout in TUI (l key)
  - Auto-load cached SSO sessions on startup
- Configuration file support ✅ **Working**
  - XDG Base Directory compliance
  - `~/.config/awsom/config.toml`
  - Environment variable overrides
  - Profile defaults (region, output format)
  - `config init` and `config path` commands

### 📋 Planned
- Background session refresh
- Desktop notifications
- Multiple SSO instance support
- Profile favorites/bookmarks
- Interactive config editor in TUI

## Architecture

### Authentication Flow

1. **Register Client**: Register with AWS SSO-OIDC
2. **Device Authorization**: Start device authorization flow
3. **User Authorization**: User authorizes in browser
4. **Token Exchange**: Poll for access token
5. **Token Caching**: Cache token in `~/.aws/sso/cache/`

### Credential Flow

1. **List Accounts**: Fetch available AWS accounts
2. **List Roles**: Get roles for each account
3. **Get Credentials**: Fetch temporary credentials for selected role
4. **Cache Credentials**: Store in `~/.aws/cli/cache/`

## Building

```bash
# Check for errors
cargo check

# Build debug version
cargo build

# Build release version
cargo build --release

# Run
cargo run -- login --start-url https://your-org.awsapps.com/start --region us-east-1
```

## Testing

```bash
cargo test
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Inspiration

This project was inspired by the Python [aws-sso-util](https://github.com/benkehoe/aws-sso-util) by Ben Kehoe. We built **awsom** from scratch in Rust to provide a fast, modern TUI experience for AWS SSO management.

## License

Apache-2.0

## Troubleshooting

### Enable Verbose Logging

If you encounter any issues, run commands with the `--verbose` flag to see detailed debug information:

```bash
awsom --verbose login
awsom -v list
```

This will show:
- Client registration details
- Device authorization flow
- Token polling status
- API error messages
- Credential fetching progress

### Common Issues

**"No SSO session found"**
- Run `awsom login` first to authenticate

**"Token expired"**
- Your SSO token has expired. Run `awsom login --force` to re-authenticate

**"Service error"**
- Use `--verbose` to see the full error message
- Check your internet connection
- Verify your `--start-url` and `--region` are correct

## Roadmap

**Current Status: v0.1.0 - Feature Complete! 🎉**

All core features are now implemented and working:
- ✅ AWS SSO authentication with device flow
- ✅ Full TUI interface with profile management
- ✅ All CLI commands (`login`, `list`, `logout`, `exec`, `export`, `console`, `status`, `config`, `completions`)
- ✅ AWS credentials file integration
- ✅ Console federated sign-in
- ✅ Session status checking for automation
- ✅ Real-time expiration tracking
- ✅ Profile management (create, rename, delete, set default)
- ✅ Configuration file support

**Future Enhancements:**
- Background session refresh
- Desktop notifications for expiring sessions
- Multiple SSO instance support
- Profile favorites/bookmarks
- Session history and analytics
