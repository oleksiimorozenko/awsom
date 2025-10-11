# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
