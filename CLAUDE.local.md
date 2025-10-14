# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**awsom** is a k9s-inspired Terminal User Interface (TUI) for managing AWS SSO sessions, written in Rust. It provides both an interactive TUI and comprehensive CLI commands for AWS SSO authentication and credential management.

## Development Commands

### Build & Run
```bash
# Check for errors (fast feedback)
cargo check

# Build debug version
cargo build

# Build optimized release version
cargo build --release

# Run the TUI (no subcommand)
cargo run

# Run CLI commands
cargo run -- login --start-url https://example.awsapps.com/start --region us-east-1
cargo run -- list
cargo run -- --verbose status

# Test the release binary
./target/release/awsom --version
./target/release/awsom
```

### Testing & Linting
```bash
# Run all tests
cargo test

# Run tests with verbose output
cargo test -- --nocapture

# Run specific test
cargo test test_sso_token_is_expired

# Check code formatting
cargo fmt -- --check

# Auto-format code
cargo fmt

# Run clippy (CI uses strict settings)
cargo clippy --all-targets --all-features -- -D warnings -A dead_code
```

## Architecture

### Core Design Principles

1. **AWS CLI v2 Compatibility**: awsom uses the same cache directories and formats as AWS CLI v2:
   - SSO tokens: `~/.aws/sso/cache/` (SHA-256 hashed filenames)
   - Role credentials: `~/.aws/cli/cache/`
   - Configuration: `~/.aws/config` (reads `[sso-session]` sections)

2. **Dual Interface**: The application operates in two modes:
   - **TUI Mode** (default, no subcommand): Interactive Ratatui-based interface with k9s-style navigation
   - **CLI Mode** (with subcommand): Non-interactive command execution for automation

3. **Logging Strategy**:
   - TUI mode: Logs written to `~/.cache/awsom/awsom.log` (or platform equivalent) to avoid breaking UI
   - CLI mode: Logs written to stderr
   - See `src/main.rs:32-68` for implementation

### Module Architecture

The codebase is organized into focused modules:

- **auth/** - SSO-OIDC authentication using AWS device authorization flow
  - `oidc.rs`: Client registration, device authorization, token polling
  - `token_cache.rs`: Token persistence in AWS CLI v2-compatible format

- **credentials/** - Credential fetching and caching
  - `fetcher.rs`: Fetches temporary credentials from AWS SSO API
  - `cache.rs`: Persists credentials in AWS CLI v2 format

- **aws_config.rs** - Manages `~/.aws/credentials` file (read/write/delete profiles)

- **sso_config.rs** - Reads SSO configuration from `~/.aws/config`

- **session/** - Session state management and profile tracking

- **console/** - AWS Console federated sign-in URL generation

- **ui/** - Ratatui TUI implementation
  - `app.rs`: Main TUI application (111KB, largest file in codebase)
  - Handles keyboard navigation, session management, profile CRUD operations

- **cli/** - Clap-based CLI interface
  - `commands/`: Individual command implementations (login, list, exec, export, console, status, logout, completions)
  - Each command is self-contained and async

- **models.rs** - Core data models with helper methods:
  - `SsoToken`, `RoleCredentials`, `AccountRole`, `ProfileSession`
  - All include expiration tracking and display formatting
  - Comprehensive unit tests included

### Key Data Flow

**Authentication Flow** (auth module):
1. Register client with AWS SSO-OIDC (`RegisterClient`)
2. Start device authorization flow (`StartDeviceAuthorization`)
3. User authorizes in browser
4. Poll for access token (`CreateToken`)
5. Cache token in `~/.aws/sso/cache/` with SHA-256 hashed filename

**Credential Flow** (credentials module):
1. List accounts via `ListAccounts` API
2. List roles per account via `ListAccountRoles` API
3. Fetch role credentials via `GetRoleCredentials` API
4. Cache credentials in `~/.aws/cli/cache/`
5. Optionally write to `~/.aws/credentials` as named profile

**TUI Flow** (ui module):
1. On startup, auto-load cached SSO sessions
2. Display all available account/role combinations
3. Visual indicators: 🟢 active / 🔴 inactive, ✓ default profile
4. Real-time countdown timers for token/credential expiration
5. Keyboard shortcuts for session management (see README.md:172-182)

### Important Conventions

1. **Error Handling**: Uses `anyhow::Result` with `thiserror` for custom error types (see `error.rs`)

2. **Async Runtime**: Tokio with `features = ["full"]` (see Cargo.toml:29)

3. **TUI Framework**: Ratatui 0.29 with Crossterm backend, Catppuccin color scheme

4. **Testing**: Uses `mockall` for mocking AWS SDK clients (see dev-dependencies)

5. **Profile Naming**: Profile names are auto-generated as `{account_name}/{role_name}` but can be renamed by user

6. **Session Status**: Four states defined in `models.rs:151-157`:
   - Active (>5 min remaining)
   - Expiring (<5 min remaining)
   - Expired (past expiration time)
   - Inactive (no credentials cached)

## Configuration Sources (Priority Order)

Settings are resolved in this order (later overrides earlier):
1. `~/.aws/config` `[sso-session]` sections
2. Environment variables (`AWS_SSO_START_URL`, `AWS_SSO_REGION`)
3. CLI flags (`--start-url`, `--region`)

## CI/CD

GitHub Actions workflows:
- **CI** (`.github/workflows/ci.yml`): Tests on Ubuntu/macOS/Windows, runs clippy with `-D warnings -A dead_code`
- **Release**: Builds static Linux binaries, cross-compiles for multiple platforms, generates Homebrew formula

## Testing Notes

- All core models have unit tests (see `models.rs:170-315`)
- Integration tests use temporary directories (`tempfile` crate)
- AWS SDK interactions can be mocked using `mockall` trait objects
- TUI mode cannot be easily tested; focus tests on business logic in other modules

## Key Files

- `src/main.rs` - Entry point, sets up logging based on TUI vs CLI mode
- `src/models.rs` - Core data structures with extensive helper methods
- `src/aws_config.rs` - Complex AWS credentials file parser/writer (36KB)
- `src/ui/app.rs` - Main TUI application logic (111KB, most complex file)
- `src/cli/mod.rs` - CLI command definitions and routing

## Release Process

Version is managed in `Cargo.toml:3`. See `RELEASE.md` and `RELEASE_SETUP.md` for release workflow details.

Homebrew formula is generated automatically in CI and stored in `Formula/` directory.

## AWS SDK Usage

The project uses AWS SDK for Rust with these crates:
- `aws-sdk-sso` - For fetching accounts/roles and credentials
- `aws-sdk-ssooidc` - For OIDC device authorization flow
- `aws-config` - For loading AWS configuration

All AWS API calls are made through async methods that return `Result<T, SdkError>`.
