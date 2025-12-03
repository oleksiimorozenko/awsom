# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.16.1] - 2024-12-02

### Fixed
- SSO profiles from `~/.aws/config` not loading at startup when SSO session is offline
- Static profile rename leaving unconfigured artifact in config file (now properly deletes old profile from both credentials and config files)
- Edit wizard not preserving region and output settings for static profiles (now pre-fills existing values from config file)
- JSON output contaminated with log messages when using `--json` flag (logs now redirect to file for clean JSON parsing)

## [0.16.0] - 2025-11-26

### Added
- **Environment variable `AWSOM_DEFAULT_REGION`** to pre-fill default AWS region in TUI profile creation wizards
- **Environment variable `AWSOM_DEFAULT_OUTPUT`** to pre-fill default output format in TUI profile creation wizards
- Centered text in confirmation dialogs for improved visual presentation
- Missing keyboard shortcuts to help screen: `a`, `D`, `s`, `x`, `f`, and full SSM Browser section

### Changed
- Profile creation wizards now check environment variables (`AWSOM_DEFAULT_REGION`, `AWSOM_DEFAULT_OUTPUT`) for default values
- Simplified config file management by removing internal section markers

### Fixed
- README keyboard shortcuts documentation - removed non-existent 'l' key, corrected 's'/'v' key descriptions
- README auto-refresh threshold documentation (now correctly states < 3 hours, not < 5 minutes)
- Help screen now includes all available keyboard shortcuts

### Removed
- **awsom-defaults internal profile** - Replaced with environment variables for setting default region/output
- **profiles.json cache file** - TUI now reads directly from `~/.aws/config` and `~/.aws/credentials`
- **.awsom-initialized marker file** - No longer tracks initialization state
- **Automatic backup creation** - Removed `config-before-awsom.bak` and `credentials-before-awsom.bak` creation
- **Section marker comments** - Removed "managed by awsom" comment blocks from config files
- **Non-existent features from README** - Removed references to 'l' key and config commands that don't exist

### Documentation
- Added `AWSOM_DEFAULT_REGION` and `AWSOM_DEFAULT_OUTPUT` to environment variables documentation
- Fixed keyboard shortcuts section with accurate key bindings
- Corrected SSM Browser access instructions (press 's', not 'v')
- Removed documentation for non-existent features (config.toml, config commands)
- Expanded TUI help screen with complete keyboard reference

## [0.15.0] - 2025-11-25

### Added
- **Platform-Agnostic Symbol Mode** - ASCII symbols for terminals without Unicode support
  - Set `AWSOM_ASCII_SYMBOLS=1` environment variable to enable ASCII mode
  - Centralized symbols module replaces 30+ hardcoded Unicode symbols
  - ASCII fallbacks: 🟢→[+], 🔴→[-], ✓→[x], ⚠→[!], spinner→|/-\
  - Supports Windows Terminal, cmd.exe, and all Linux/macOS terminals
- **SSM Browser State Sorting** - Press 's' to cycle through sort options
  - Sort order: unsorted → name → ID → state → IP → unsorted
  - State priority: running > pending > stopping > stopped > shutting-down > unknown
  - Groups connectable instances at top of list for easier access
- **SSM Browser Navigation in Search Mode** - Use Up/Down arrows while searching
  - Navigate filtered results without exiting search mode
  - Tab or Enter to exit search mode (keeps filter active)
  - Esc to clear filter (both in and out of search mode)

### Fixed
- SSO session token status now updates during auto-refresh cycle (not just credentials)
- Cursor visibility when connecting to SSM sessions (using spawn with inherited stdio)
- Blank screen after returning from SSM session (added terminal.clear())
- Clipboard copy ('y' key) now actually copies to clipboard using arboard library

### Changed
- Search prompt now shows in SSM browser header: "Search: <query>_"
- Main screen help text changed from 'ssm' to 'SSM browser'
- SSM browser help bar now includes 's: Sort' hint

### Documentation
- Added platform-agnostic ASCII mode instructions for Unix/Linux/macOS, PowerShell, and cmd.exe

## [0.14.0] - 2025-11-24

### Added
- **SSM Browser TUI** - Major feature for EC2 instance management
  - Press 's' from Profiles pane to open SSM browser
  - Lists all EC2 instances with SSM connection status (Online/Offline)
  - Shows instance name, ID, state, and private IP address
  - Navigate with ↑↓/jk keys, start session with Enter
  - Real-time search with '/' key (filters by name, ID, or IP)
  - View instance tags with 'v' key (sorted table format)
  - Toggle offline instances with 'o' key (shows X online / Y total)
  - Copy SSM command to clipboard with 'y' key
  - Suspended TUI integration: SSM sessions run in-place, TUI auto-resumes on exit
- **SSM SDK Integration** - aws-sdk-ssm and aws-sdk-ec2 dependencies
  - Cross-platform terminal session launching
  - EC2 instance listing with SSM status detection
  - Instance tags fetching and display
  - AWS CLI command generation for manual execution
- **Role Credentials Auto-Refresh** - Proactive credential renewal before expiration
  - Refreshes credentials 3 hours before they expire
  - Maintains continuous access without user intervention
  - Enables ~24h credential experience from 12h credential limits
  - Updates both ~/.aws/credentials and in-memory state

### Fixed
- Always exclude terminated instances from SSM browser list
- SSM commands now include AWS_PROFILE prefix for correct profile context
- Panic hook installed to restore terminal on crash during SSM sessions

### Changed
- Default SSM browser shows only online instances (toggle with 'o')
- SSM browser header displays online/total counts
- Help text capitalization: "Start session" / "Copy command"

### Internal
- Added tests for token and credentials refresh methods
- SSO token refresh infrastructure with client credentials storage

## [0.13.0] - 2025-11-23

### Added
- **Auto-Refresh for External Credential Changes** - TUI detects changes made by other tools
  - Light-weight refresh reads local credential files every 60 seconds
  - Updates profile statuses (active/expired) without AWS API calls
  - Works even when SSO session is logged out
  - Keeps awsom in sync when users authenticate via AWS CLI or other tools
- **SSO Token Refresh Infrastructure** - Foundation for silent token renewal
  - Added client_id, client_secret, and refresh_token storage in cached tokens
  - Request refresh_token grant type during client registration
  - Added OidcClient.refresh_token() method for silent token refresh
  - Helper methods: can_refresh(), needs_refresh(), should_auto_refresh()
  - Enables automatic SSO session refresh for supporting organizations

### Fixed
- Profiles now remain visible when SSO session is logged out
  - Previously, logging out cleared the profiles pane
  - Cached profile data and valid credentials are now preserved
  - Provides accurate state representation and better UX

### Changed
- Boxed SsoToken in LoginResult to avoid large enum variant warning

## [0.12.0] - 2025-11-23

### Fixed
- **Atomic Writes for Config Files** - Prevents file corruption on crashes
  - Uses temp file + fsync + atomic rename pattern
  - Protects ~/.aws/config and ~/.aws/credentials from corruption
  - Safe against process interruption (crash, Ctrl+C, power loss)
- **Eliminated Panic-Prone Code Paths** - Improved stability
  - Replaced unsafe .unwrap() with if-let pattern matching in aws_config.rs
  - Removed panic-prone Default impl for SessionManager
  - Added proper error handling for stdin/stdout operations
  - Added graceful stderr fallback when log file creation fails

### Changed
- **Device Authorization Refactoring** - Replaced Arc<Mutex<>> with tokio::sync::watch
  - Eliminates potential blocking in async context
  - Simplifies code with non-blocking borrow() instead of lock()
  - Cleaner async/await patterns

### Internal
- **UI Module Refactoring** - Extracted types, state, and theme from app.rs
  - Created types.rs with core domain types (LoginResult, AccountRoleWithStatus, ProfileEntry, etc.)
  - Created state.rs with AppState and input wizard step enums
  - Created theme.rs with catppuccin_color helper
  - Reduced app.rs from 4955 to 4824 lines of code
- Cleaned up dead code and added allow annotations (warnings reduced from 33 to 0)
- Removed unused session/ and expiry/ modules entirely

## [0.11.0] - 2025-11-23

### Fixed
- **Token Cache Key Consistency** - Fixed "No valid SSO token found" errors after login
  - Session login now uses session_name for token cache key (not start_url)
  - Consistent token lookup across login, status, and profile start commands
  - Previously caused immediate failures when using `--session-name` parameter

## [0.10.0] - 2025-11-23

### Fixed
- **SSO Session Renaming** - Properly handles session name changes in TUI
  - Deletes old session from ~/.aws/config when renamed
  - Updates all profiles referencing the old session name to use new name
  - Prevents duplicate sessions and broken profile references

## [0.9.0] - 2025-11-23

### Added
- **Profile-Specific Console Access** - New `--profile` parameter for console command
  - Run `awsom profile console --profile staging` to open console for specific profile
  - Reads account_id, role_name, and sso_session from profile configuration
  - Complements existing account/role selection workflow

### Fixed
- **CLI Session Resolution** - Fixed session context loss in all CLI commands
  - All commands (list, console, exec, export) now properly maintain session context
  - resolve_sso_session() returns (session_name, start_url, region) tuple
  - Commands pass session_name to SsoInstance for correct token lookup
  - Session status command now uses resolved session info for proper token finding

### Removed
- Deprecated 'import' command (section markers are no longer used)

## [0.8.1] - 2025-11-22

### Fixed
- Make-default dialog no longer shows "Profile already exists" incorrectly when no [default] exists
- Uses get_profile_details() instead of is_profile_in_awsom_section() for proper existence check

### Internal
- CI: Added test job as prerequisite for releases
- CI: Use Swatinem/rust-cache for more reliable caching (fixes macOS runner intermittent failures)

## [0.8.0] - 2025-11-22

### Added
- **Multi-Profile Support** - Support for static credentials and incomplete profiles
  - Static credentials profiles (non-SSO) now fully supported
  - Detection of incomplete profiles (config-only, no credentials)
  - Profile type indicators in TUI: SSO, STATIC, CONFIG
  - 'v' key to view detailed profile information
  - 'D' key to delete any profile type
- **Improved Disk Cache** - Automatic invalidation when config/credentials change
  - Detects external modifications to AWS configuration files
  - Profiles load even without active SSO sessions
  - Faster startup with intelligent cache invalidation

### Changed
- Renamed "Accounts & Roles" pane to "Profiles & Roles" throughout UI
- Updated keyboard shortcuts: Tab to switch panes, e/v/d/D for profile operations
- Added awsom-defaults comment explaining marker line purpose

### Deprecated
- Import command removed (section markers no longer used)

## [0.7.1] - 2025-11-22

### Added
- **Startup Status Messages** - Animated spinner during AWS API calls
  - Shows "Refreshing profile list..." during initial load
  - UI displays immediately on startup (no blank screen)
  - Skips API refresh if no SSO sessions are configured

### Changed
- Renamed "Accounts" to "Profiles" throughout UI and help screens
- Status messages now display in header bar with spinner animation

## [0.7.0] - 2025-11-22

### Added
- **Disk Cache for Profiles** - Improved multi-terminal support
  - Profiles cached on disk for faster startup
  - Cache shared across multiple terminal sessions
  - Automatic cache invalidation based on file modification times

## [0.6.0] - 2025-11-21

### Added
- **In-Memory Account/Role Caching** - Faster navigation within sessions
  - Per-session caching eliminates redundant AWS API calls
  - Press 'r' to bypass cache and fetch fresh data from AWS
- **Session Filtering** - Focus on specific SSO session accounts
  - Press 'f' on a session to show only its accounts
  - Visual indicators: [FILTERED] marker in Sessions and Accounts panes
  - Improves focus when managing multiple SSO sessions
- **Static Credentials Support Foundation** - Domain models for static credentials
  - CredentialType enum (Sso, Static)
  - StaticCredentials struct with validation
  - Comprehensive tests for static credential validation
- **Visual Improvements** - Active pane indicator
  - Asterisk (*) added to active pane title
  - Improves visual clarity beyond color-only indication

### Fixed
- Confirmation dialog Y/N buttons now display correctly (fixed text wrapping issue)
- Enter key on existing profile no longer shows incorrect "Overwrite" dialog

## [0.5.0] - 2025-10-15

### Breaking Changes
- **Complete CLI restructuring** - Removed all top-level profile/credential commands
  - `awsom list` → `awsom profile list`
  - `awsom exec` → `awsom profile exec`
  - `awsom export` → `awsom profile export`
  - `awsom console` → `awsom profile console`
  - **NO backward compatibility aliases** - Clean break for consistent command structure
  - All session management under `session` subcommand, all profile/credential operations under `profile` subcommand
  - This provides a clear, hierarchical command structure that scales better with future features

### Added
- **`profile` subcommand** - Unified namespace for all profile and credential operations
  - `profile list` - List available accounts and roles
  - `profile start <profile-name>` - **NEW**: Refresh credentials for an existing profile
  - `profile exec` - Execute commands with AWS credentials
  - `profile export` - Export credentials as environment variables or to ~/.aws/credentials
  - `profile console` - Open AWS Console in browser
- **Smart credential refresh** - `profile start` command for existing profiles
  - Reads profile configuration from `~/.aws/config`
  - Validates SSO profile requirements (sso_session, account_id, role_name)
  - Automatically resolves SSO session and fetches fresh credentials
  - Updates `~/.aws/credentials` with new temporary credentials
  - Shows expiration time and helpful error messages
  - Useful for long-running sessions and automation scripts

### Changed
- All profile/credential commands now require `profile` subcommand prefix
- Command structure now mirrors conceptual model: `session` for authentication, `profile` for credentials
- Help text and documentation updated throughout to reflect new structure

### Documentation
- Updated [COMMANDS.md](COMMANDS.md) with complete new command tree
- Updated [README.md](README.md) with new command examples in Quick Start and CLI Commands sections
- Removed migration section from COMMANDS.md (clean break, no compatibility mode)
- All examples updated to use new `profile` subcommand syntax

## [0.4.4] - 2025-10-14

### Breaking Changes
- **Removed top-level `login`, `logout`, and `status` commands** - Use `session` subcommands instead
  - `awsom login` → `awsom session login`
  - `awsom logout` → `awsom session logout`
  - `awsom status` → `awsom session status`
  - This change provides a cleaner, more consistent command structure
  - All session-related operations are now under the `session` namespace

### Added
- **--headless flag** - Force headless mode even on systems with browsers
  - Allows manual control of headless behavior
  - Useful when browser opening is undesired on graphical systems
  - Highest priority in headless detection (overrides auto-detection)
  - Works in both TUI and CLI modes
- **Cancel authentication** - Press 'q' or 'Esc' during login to cancel
  - Loading screen shows "Press 'q' or 'Esc' to cancel" instruction
  - TUI remains responsive during entire authentication process
  - Cancelled logins return to main screen with clear status message
- **Single-URL authentication** - One-click copy-paste for easier authentication
  - Authentication popup now shows complete URL with code embedded
  - No need to copy URL and code separately
  - Example: `https://example.awsapps.com/start/#/device?user_code=ABCD-EFGH`
  - Falls back to separate URL + code display if complete URL unavailable

### Fixed
- **Critical: Fixed TUI blocking during login** - TUI now remains responsive during SSO authentication
  - Login operations now run in background tasks using async channels
  - TUI event loop no longer blocks on `.await` during authentication
  - Authentication popup (URL + code) now displays properly in headless environments
  - Keyboard controls (q/Esc/Ctrl+C) work during login process
  - Device authorization info shared via Arc<Mutex<>> between background task and UI
- **Fixed headless detection on macOS** - Browser now opens correctly on macOS
  - macOS doesn't set `DISPLAY` variable (uses native windowing, not X11)
  - Previous version incorrectly detected macOS as headless environment
  - `DISPLAY` check now skipped on macOS using `#[cfg(not(target_os = "macos"))]`
  - Headless detection now properly works on macOS, Linux, Docker, and SSH
- **Fixed real-time status indicators** - Status indicators now update in real-time when credentials expire
  - Both Sessions and Accounts panes now calculate expiration on every render
  - Status indicators (🟢/🔴) now instantly turn red when credentials expire
  - No manual refresh needed to see expiration status changes
  - Fixes inconsistency where green indicator showed with "EXPIRED" text
- Improved headless detection priority order (--headless flag, SSH, CI, TERM, DISPLAY)
- Added CI environment detection (`CI` variable)

### Changed
- **TUI now defaults to Accounts pane when active session exists** - Improved startup UX
  - First active SSO session is automatically selected on startup
  - Accounts pane becomes active (instead of Sessions pane) when accounts are loaded
  - First account is automatically selected for immediate interaction
  - Only falls back to Sessions pane if no active sessions exist
- Login operations spawn background tasks with result channels
- Loading screen polls device authorization info from shared state
- Headless detection now platform-aware (different checks for macOS vs Linux)
- Improved debug logging for environment detection

## [0.4.3] - 2025-10-14

### Unreleased (skipped - contained incomplete fix)

## [0.4.2] - 2025-10-13

### Fixed
- TUI now properly detects headless environments (Docker, SSH) and skips browser launch
- Login screen shows appropriate instructions for headless vs normal environments
- Fixes "No valid browsers detected" error when running TUI in Docker/SSH sessions
- Auth URL and code are now displayed in TUI popup for manual authentication in headless mode

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
