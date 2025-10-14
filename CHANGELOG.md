# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.1] - 2025-10-13

### Fixed
- Completion hint messages no longer appear when eval'd in shell rc files (e.g., `.bashrc`, `.zshrc`)
- Hint messages now only show when running interactively (stdout is a terminal)
- Fixes issue where `eval "$(awsom completions bash)"` in `.bashrc` showed messages on every shell startup

## [0.4.0] - 2025-10-13

### Added
- **Multi-Session Auto-Resolution**:
  - 4-level priority session resolution logic:
    1. Explicit flags (`--start-url` + `--region`) - highest priority for scripting
    2. Session name (`--session-name`) - explicit session selection
    3. Active SSO token (if only one exists) - automatic detection
    4. Single configured session (if only one exists) - automatic fallback
  - Helpful error messages when multiple sessions exist with examples
- **Session Parameters**:
  - `--session-name` parameter added to: `exec`, `export`, `console`, `list` commands
  - Automatic session resolution for single-session environments
  - Clear error messages listing available sessions when resolution fails
- **Headless Mode Support**:
  - `--headless` global flag to disable browser opening
  - Auto-detection of headless environments (SSH, Docker, no DISPLAY)
  - Environment checks: `DISPLAY`, `SSH_TTY`, `SSH_CONNECTION`, `TERM`
  - Headless-specific authentication display with clear manual instructions
- **New Session Subcommands**:
  - `awsom session login [--session-name <name>]` - Authenticate with auto-resolution
  - `awsom session logout [--session-name <name>]` - Logout with auto-resolution
  - `awsom session status [--session-name <name>] [--json]` - Check status with auto-resolution
  - All session subcommands support `--session-name` parameter
- **Improved Completions**:
  - `--show-install` flag for showing installation instructions
  - Copy-paste ready installation commands for all shells (bash, zsh, fish, powershell, elvish)
  - Clean separation of script generation and installation help

### Changed
- Session login display now adapts to headless environments
- Browser opening is skipped in headless mode
- Authentication instructions formatted for easy copy-paste in headless mode
- Completion generation improved with cleaner output and helpful hints

### Deprecated
These top-level commands will be removed in v0.5.0 (use session subcommands instead):
- `awsom login` → use `awsom session login`
- `awsom logout` → use `awsom session logout`
- `awsom status` → use `awsom session status`

### Documentation
- Added [COMMANDS.md](COMMANDS.md) with complete command tree visualization
- Documented session resolution logic with priority order and examples
- Added headless mode documentation with auto-detection details
- Documented migration path from deprecated commands
- Added common usage patterns for different scenarios (single user, team, CI/CD, SSH)

## [0.3.0] - 2025-10-13

### Added
- **Config File Organization System**:
  - Marker-based separation of user-managed and awsom-managed sections
  - User-managed sections preserved above marker line
  - Awsom-managed sections automatically organized below marker line
  - Automatic alphabetical sorting within awsom-managed area
  - One-time backups on first run: `config-before-awsom.bak`, `credentials-before-awsom.bak`
  - Marker file (`~/.aws/.awsom-initialized`) to track initialization
  - Header comments in config/credentials files explaining backup location and management
- **Import Command** for migrating existing configurations:
  - `awsom import <name> --section-type <profile|sso-session>` - Import existing sections to awsom management
  - Interactive confirmation with preview (bypass with `--force`)
  - Moves sections from user-managed to awsom-managed area
  - Maintains proper formatting and alphabetical sorting after import
- **Profile Collision Detection**:
  - Prevents accidental overwrites of user-managed profiles
  - Clear error messages suggesting import command
  - Protects user configurations from unintended modifications
- **Session Management CLI Commands** for automation and scripting:
  - `awsom session add` - Add new SSO sessions via CLI
  - `awsom session list` - List all sessions (text/JSON formats)
  - `awsom session delete` - Delete sessions with optional `--force` flag
  - `awsom session edit` - Edit session start URL and/or region
  - `awsom session switch` - Switch between sessions (placeholder for multi-session support)
- **TUI Session Management Improvements**:
  - 'a' button: Add new SSO session dialog
  - 'e' button: Edit existing SSO session dialog
  - 'd' button: Delete session with double-press confirmation (2-second window)

### Fixed
- TUI 'a' button (add session) - now shows SSO configuration dialog
- TUI 'e' button (edit session) - pre-fills dialog with current values
- TUI 'd' button (delete session) - now actually deletes from ~/.aws/config file with confirmation

### Documentation
- Added "No AWS CLI Required!" section in README highlighting standalone nature
- Documented all session CLI commands with examples
- Documented import command with use cases
- Added automation/provisioning script examples
- Updated prerequisites to clarify AWS CLI is optional

## [0.2.2] - 2025-10-13

### Fixed
- Fixed status indicator showing green for expired credentials - now correctly displays red circle when credentials are expired

## [0.2.1] - 2024-10-13

### Fixed
- Static Linux binary builds
- Homebrew formula generation to match k9s pattern
- Linux prerequisites documentation for Homebrew installation

## [0.2.0] - 2024-10-11

### Added
- Multi-session support with two-pane layout
- Session management (add, edit, delete SSO sessions)
- Auto-refresh of account list every minute
- Improved keyboard navigation with Tab to switch panes
- Session-specific account loading
- Context-aware help text
- Visual pane highlighting

### Changed
- Redesigned UI with Sessions and Accounts panes
- Improved status indicators and expiration display
- Enhanced error handling and user feedback

## [0.1.0] - 2025-10-10

### Added - Feature Complete Release ✅

#### Core Features

- **AWS SSO OIDC Authentication**: Full device flow implementation
  - Client registration with AWS SSO-OIDC
  - Device authorization with automatic browser launch
  - Token polling with proper error handling
  - Automatic retry on `AuthorizationPendingException`
  - Support for `SlowDownException` handling
  - Token caching compatible with AWS CLI v2

- **Token Management**: AWS CLI v2 compatible caching
  - Tokens stored in `~/.aws/sso/cache/`
  - SHA-256 based cache key generation (compatible with AWS CLI)
  - Automatic expiration checking
  - Auto-load cached sessions on TUI startup

- **Credential Fetching**: Full AWS SSO integration
  - List available AWS accounts
  - List roles for each account
  - Fetch temporary credentials for account/role combinations
  - Real-time credential status tracking

- **AWS Credentials File Management**
  - Read, write, and delete profiles in `~/.aws/credentials`
  - Read and write profile configuration in `~/.aws/config`
  - Profile metadata tracking (account ID, role name)
  - Profile renaming support
  - Default profile management
  - Expiration time tracking

- **Console Access**
  - AWS Console federated sign-in
  - Generate federation sign-in tokens
  - Open console in default browser with temporary credentials
  - Configurable region for console access

- **CLI Interface**: Comprehensive command-line tool
  - `login`: Interactive SSO authentication with device flow
  - `list`: Display accounts and roles (text/JSON formats)
  - `logout`: End SSO session and clear cached tokens
  - `exec`: Execute commands with temporary AWS credentials
  - `export`: Export credentials as environment variables or to ~/.aws/credentials
  - `console`: Open AWS Console in browser with federated sign-in
  - `completions`: Generate shell completion scripts (bash, zsh, fish, powershell, elvish)
  - `config init`: Create sample configuration file
  - `config path`: Show configuration file path and status
  - Global `--verbose` flag for debug logging
  - Environment variable support (`AWS_SSO_START_URL`, `AWS_SSO_REGION`)

- **Terminal User Interface (TUI)**
  - k9s-inspired interactive interface using Ratatui
  - Real-time session status display
  - Visual indicators: 🟢 active sessions / 🔴 inactive sessions
  - Default profile marker (✓)
  - Real-time expiration countdown timers
  - Keyboard shortcuts:
    - `l`: Login/logout toggle
    - `r`: Refresh account/role list
    - `↑`/`↓` or `j`/`k`: Navigate selection
    - `Enter`: Start/stop session (create or delete profile)
    - `p`: Edit profile name
    - `d`: Set profile as default
    - `c`: Open AWS Console in browser
    - `?` or `F1`: Show help
    - `q` or `Esc`: Quit
  - Profile input dialog with cursor navigation
  - Auto-load cached SSO sessions on startup
  - In-TUI login flow with device code display
  - Ctrl+C double-press to force quit

- **Configuration File Support**
  - TOML configuration format
  - XDG Base Directory compliance (`~/.config/awsom/config.toml`)
  - SSO instance configuration (start URL, region)
  - Profile defaults (region, output format)
  - UI preferences
  - Environment variable overrides
  - Configuration priority: config file < env vars < CLI flags

- **Error Handling**: Proper AWS SDK error integration
  - Type-safe error handling with `thiserror`
  - Correct error code detection using `ProvideErrorMetadata` trait
  - User-friendly error messages
  - Graceful handling of expired tokens
  - Clear error messages for missing configuration

- **Logging**: Structured logging with `tracing`
  - Optional verbose mode with `--verbose` / `-v`
  - File-based logging for TUI mode (doesn't break UI)
  - Stderr logging for CLI commands
  - Debug information for all operations

#### Technical Implementation
- **Language**: Rust (Edition 2021)
- **Async Runtime**: Tokio 1.42 (full features)
- **CLI Framework**: Clap 4.5 with derive macros
- **TUI Framework**: Ratatui 0.29 with Crossterm backend
- **AWS SDK**: Official AWS SDK for Rust
  - `aws-sdk-sso` 1.56
  - `aws-sdk-ssooidc` 1.56
  - `aws-config` 1.5
  - `aws-types` 1.3
- **HTTP Client**: reqwest 0.12 (for console federation)
- **Serialization**: serde, serde_json, toml
- **Error Handling**: thiserror 2.0, anyhow 1.0
- **Logging**: tracing 0.1, tracing-subscriber 0.3
- **Other**: chrono (timestamps), webbrowser (console launch), urlencoding

#### Dependencies
All dependencies use stable, well-maintained versions:
- Core functionality: AWS SDK, Tokio, Clap, Ratatui
- No unstable features required
- Cross-platform support (macOS, Linux, Windows)

### Fixed
- Error matching in OIDC token polling using `ProvideErrorMetadata::code()`
- Proper handling of expired tokens with clear user messages
- File-based logging in TUI mode to prevent UI corruption
- Profile renaming edge cases (deleting old profile when name changes)

### Project Structure
```
awsom/
├── src/
│   ├── auth/               # SSO OIDC authentication
│   │   ├── mod.rs          # AuthManager
│   │   ├── oidc.rs         # Device flow implementation
│   │   └── token_cache.rs  # Token caching (AWS CLI compatible)
│   ├── credentials/        # Credential management
│   │   ├── mod.rs          # CredentialManager
│   │   ├── fetcher.rs      # AWS SSO API calls
│   │   └── cache.rs        # Credential caching
│   ├── aws_config.rs       # AWS credentials file I/O
│   ├── console/            # AWS Console access
│   │   └── mod.rs          # Federation sign-in URL generation
│   ├── cli/                # CLI interface
│   │   ├── mod.rs          # Argument parser
│   │   └── commands/       # Command implementations
│   ├── ui/                 # TUI interface
│   │   └── app.rs          # Main TUI application
│   ├── session/            # Session management (for future use)
│   ├── config/             # Configuration file management
│   │   └── mod.rs          # Config loading and XDG compliance
│   ├── expiry/             # Expiry tracking utilities
│   ├── models.rs           # Core data structures
│   ├── error.rs            # Error types
│   └── main.rs             # Entry point
├── Cargo.toml              # Dependencies and metadata
├── CHANGELOG.md            # This file
├── README.md               # Documentation
└── .gitignore              # Git ignore rules
```

### Known Limitations
- Background session refresh not yet implemented
- No desktop notifications for expiring sessions
- Single SSO instance support only (multi-instance planned)
- No session history or analytics

### Future Enhancements
- Background daemon for automatic session refresh
- Desktop notifications (libnotify/Windows toast)
- Multiple SSO instance management
- Profile favorites and bookmarks
- Session usage analytics
- Interactive configuration editor in TUI
