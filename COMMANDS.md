# awsom Command Reference

Complete command structure for awsom CLI.

## Command Tree

```
awsom [GLOBAL OPTIONS]
├── session                    Session management commands
│   ├── add                   Add a new SSO session
│   │   --name <name>         Session name (required)
│   │   --start-url <url>     SSO start URL (required)
│   │   --region <region>     AWS region (required)
│   │
│   ├── list                  List all configured SSO sessions
│   │   [--format text|json]  Output format (default: text)
│   │
│   ├── delete <name>         Delete an SSO session
│   │   [--force]            Skip confirmation prompt
│   │
│   ├── edit <name>           Edit an existing SSO session
│   │   [--start-url <url>]  New SSO start URL (optional)
│   │   [--region <region>]  New AWS region (optional)
│   │
│   ├── switch <name>         Switch to a different SSO session
│   │
│   ├── login                 Authenticate with AWS SSO
│   │   [--session-name <name>]  Session to authenticate (optional)
│   │   [--force]               Force re-authentication
│   │
│   ├── logout                End SSO session
│   │   [--session-name <name>]  Session to logout (optional)
│   │
│   └── status                Check SSO session status
│       [--session-name <name>]  Session to check (optional)
│       [--json]                Output in JSON format
│
├── exec                      Execute command with AWS credentials
│   --role-name <role>        Role name (required)
│   --account-name <name>     Account name (required)
│   [--session-name <name>]   SSO session to use
│   [--start-url <url>]       SSO start URL (for scripting)
│   [--region <region>]       AWS region (for scripting)
│   -- <command>              Command to execute
│
├── export                    Export credentials as environment variables
│   --role-name <role>        Role name (required)
│   --account-name <name>     Account name (required)
│   [--session-name <name>]   SSO session to use
│   [--start-url <url>]       SSO start URL (for scripting)
│   [--region <region>]       AWS region (for scripting)
│   [--profile <name>]        Write to ~/.aws/credentials as profile
│
├── console                   Open AWS Console in browser
│   --role-name <role>        Role name (required)
│   --account-name <name>     Account name (required)
│   [--session-name <name>]   SSO session to use
│   [--start-url <url>]       SSO start URL (for scripting)
│   [--region <region>]       AWS region (for scripting)
│
├── list                      List available accounts and roles
│   [--session-name <name>]   SSO session to use
│   [--start-url <url>]       SSO start URL (for scripting)
│   [--region <region>]       AWS region (for scripting)
│   [--format text|json]      Output format (default: text)
│
├── import <name>             Import existing configs to awsom management
│   [--section-type profile|sso-session]  Type to import (default: profile)
│   [--force]                            Skip confirmation prompt
│
└── completions <shell>       Generate shell completion scripts
    [--show-install]          Show installation instructions

GLOBAL OPTIONS:
  --start-url <url>           SSO start URL (env: AWS_SSO_START_URL)
  --region <region>           SSO region (env: AWS_SSO_REGION)
  --headless                  Force headless mode - show URL in TUI instead of opening browser
  -v, --verbose               Enable debug logging
  -h, --help                  Print help
  -V, --version               Print version
```

## Session Resolution Logic

Commands that need SSO configuration (`exec`, `export`, `console`, `list`) resolve sessions in this priority order:

### 1. Explicit Flags (Highest Priority)
```bash
awsom exec --start-url https://... --region us-east-1 --role-name Admin --account-name Production -- aws s3 ls
```
- Uses provided `--start-url` and `--region`
- Good for scripting and CI/CD
- No session lookup needed

### 2. Session Name
```bash
awsom exec --session-name prod-sso --role-name Admin --account-name Production -- aws s3 ls
```
- Looks up session from `~/.aws/config` by name
- Error if session doesn't exist

### 3. Active SSO Token (If Only One Exists)
```bash
awsom exec --role-name Admin --account-name Production -- aws s3 ls
```
- Checks `~/.aws/sso/cache/` for active tokens
- If exactly one active token found, uses its associated session
- If multiple active tokens, requires explicit `--session-name`

### 4. Single Configured Session (If Only One Exists)
```bash
awsom exec --role-name Admin --account-name Production -- aws s3 ls
```
- Checks `~/.aws/config` for `[sso-session]` entries
- If exactly one session configured, uses it
- If multiple sessions, requires explicit `--session-name`

### 5. Error
```
Error: Multiple sessions configured. Specify --session-name or use --start-url + --region

Available sessions:
  - prod-sso (https://prod.awsapps.com/start)
  - staging-sso (https://staging.awsapps.com/start)

Example:
  awsom exec --session-name prod-sso --role-name Admin ...
```

## Headless Mode

When running in headless environments (SSH, Docker, CI/CD), awsom automatically detects and adapts:

### Auto-Detection

Headless mode is automatically enabled when:
- `DISPLAY` environment variable is not set (no X11)
- `SSH_TTY` or `SSH_CONNECTION` environment variables are set (SSH session)
- `TERM` is set to `dumb` or empty

### Explicit Headless Flag

Force headless mode:
```bash
awsom --headless session login
```

### Headless Behavior

In headless mode:
- ✅ Browser opening is **skipped**
- ✅ Single URL with code embedded shown (one copy-paste instead of two)
- ✅ Clear instructions for copy-pasting URL
- ✅ TUI shows popup dialog with auth info (remains responsive)
- ✅ Press 'q' or 'Esc' to cancel authentication at any time

### Example Output (Headless)

```
=== AWS SSO Login ===

Copy and paste this URL (code is already included):

https://seeking-alpha.awsapps.com/start/#/device?user_code=WMCQ-XCLX

Waiting for authorization...

Press 'q' or 'Esc' to cancel
```

## Common Usage Patterns

### Quick Start (Single Session)

```bash
# Add your SSO session
awsom session add --name my-org --start-url https://my-org.awsapps.com/start --region us-east-1

# Login (auto-detects single session)
awsom session login

# Launch TUI
awsom

# Or use CLI to list accounts
awsom list
```

### Multi-Session Environment

```bash
# Add multiple sessions
awsom session add --name prod --start-url https://prod.awsapps.com/start --region us-east-1
awsom session add --name staging --start-url https://staging.awsapps.com/start --region us-west-2

# Login to specific session
awsom session login --session-name prod

# Use specific session for commands
awsom exec --session-name prod --role-name Admin --account-name Production -- aws s3 ls
awsom console --session-name staging --role-name Developer --account-name Staging
```

### Scripting (No Session Configuration)

```bash
# Direct usage without session configuration
awsom exec \
  --start-url https://my-org.awsapps.com/start \
  --region us-east-1 \
  --role-name Admin \
  --account-name Production \
  -- aws s3 ls

# Export credentials
awsom export \
  --start-url https://my-org.awsapps.com/start \
  --region us-east-1 \
  --role-name Developer \
  --account-name Staging \
  --profile my-profile
```

### Headless/Docker Environment

```bash
# Explicit headless mode
awsom --headless session login --session-name prod

# Auto-detected in SSH
ssh user@server
awsom session login  # Automatically runs in headless mode
```

## Migration from v0.3.0

### Deprecated Commands (Will be removed in v0.4.0)

| Old Command | New Command | Status |
|------------|-------------|--------|
| `awsom login` | `awsom session login` | ⚠️ Deprecated |
| `awsom logout` | `awsom session logout` | ⚠️ Deprecated |
| `awsom status` | `awsom session status` | ⚠️ Deprecated |

### What's Unchanged

These commands still work exactly the same:
- `awsom exec --start-url <url> --region <region> ...` (scripting)
- `awsom export --start-url <url> --region <region> ...` (scripting)
- `awsom list --format json` (listing)
- `awsom` (TUI mode)
- All session management commands (`session add`, `session list`, etc.)

### New Features

- `--session-name` parameter added to: `exec`, `export`, `console`, `list`
- `--headless` global flag for SSH/Docker environments
- Auto-detection of headless environments
- Improved TUI auth dialog for headless systems

## Examples by Use Case

### Individual Developer

```bash
# Setup
awsom session add --name work --start-url https://company.awsapps.com/start --region us-east-1
awsom session login

# Daily usage - use TUI
awsom

# Or CLI
awsom list
awsom export --role-name Developer --account-name MyAccount --profile my-dev
```

### Team with Multiple Environments

```bash
# Setup all environments
awsom session add --name prod --start-url https://prod.awsapps.com/start --region us-east-1
awsom session add --name staging --start-url https://staging.awsapps.com/start --region us-west-2
awsom session add --name dev --start-url https://dev.awsapps.com/start --region eu-west-1

# Login to production
awsom session login --session-name prod

# Quick access
awsom exec --session-name prod --role-name Admin --account-name ProdAccount -- aws s3 ls
awsom console --session-name staging --role-name Developer --account-name StagingAccount
```

### CI/CD Pipeline

```bash
#!/bin/bash
# No session configuration needed - use direct credentials

awsom export \
  --start-url "$SSO_START_URL" \
  --region "$SSO_REGION" \
  --role-name "$ROLE_NAME" \
  --account-name "$ACCOUNT_NAME" \
  --profile ci-profile

# Use the profile
export AWS_PROFILE=ci-profile
aws s3 sync ./build s3://my-bucket/
```

### SSH/Headless Server

```bash
# SSH into server (headless auto-detected)
ssh admin@server

# Setup session
awsom session add --name prod --start-url https://prod.awsapps.com/start --region us-east-1

# Login (headless mode auto-detected, shows URL and code)
awsom session login

# Open the URL on your local machine, enter the code
# Credentials are now available on the server

# Use credentials
awsom exec --role-name Admin --account-name Production -- aws s3 ls
```

## Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `AWS_SSO_START_URL` | Default SSO start URL | `https://my-org.awsapps.com/start` |
| `AWS_SSO_REGION` | Default SSO region | `us-east-1` |
| `DISPLAY` | X11 display (headless detection) | `:0` |
| `SSH_TTY` | SSH terminal (headless detection) | `/dev/pts/0` |
| `SSH_CONNECTION` | SSH connection (headless detection) | `192.168.1.100 ...` |
| `TERM` | Terminal type (headless detection) | `xterm-256color` |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Session not found |
| 3 | Multiple sessions (explicit selection required) |
| 4 | Authentication failed |
| 5 | Token expired |

## See Also

- [README.md](README.md) - General documentation and getting started
- [CHANGELOG.md](CHANGELOG.md) - Version history and changes
- [GitHub Issues](https://github.com/oleksiimorozenko/awsom/issues) - Bug reports and feature requests
