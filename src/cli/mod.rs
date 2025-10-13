// CLI interface
pub mod commands;

use crate::error::Result;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "awsom")]
#[command(about = "AWS Organization Manager - TUI for managing AWS SSO sessions", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// SSO start URL
    #[arg(long, env = "AWS_SSO_START_URL")]
    pub start_url: Option<String>,

    /// SSO region
    #[arg(long, env = "AWS_SSO_REGION")]
    pub region: Option<String>,

    /// Enable verbose/debug logging
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Interactive login to AWS SSO
    Login {
        /// Force re-authentication
        #[arg(short, long)]
        force: bool,
    },

    /// List available accounts and roles
    List {
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Execute a command with AWS credentials
    Exec {
        /// Account ID
        #[arg(long)]
        account_id: Option<String>,

        /// Account name (alternative to account-id)
        #[arg(long)]
        account_name: Option<String>,

        /// Role name
        #[arg(long)]
        role_name: String,

        /// Command to execute
        command: Vec<String>,
    },

    /// Export credentials as environment variables or AWS profile
    Export {
        /// Account ID
        #[arg(long)]
        account_id: Option<String>,

        /// Account name (alternative to account-id)
        #[arg(long)]
        account_name: Option<String>,

        /// Role name
        #[arg(long)]
        role_name: String,

        /// Write to ~/.aws/credentials as this profile name (instead of exporting to env)
        #[arg(long)]
        profile: Option<String>,
    },

    /// Open AWS Console in browser for a role
    Console {
        /// Account ID
        #[arg(long)]
        account_id: Option<String>,

        /// Account name (alternative to account-id)
        #[arg(long)]
        account_name: Option<String>,

        /// Role name
        #[arg(long)]
        role_name: String,

        /// AWS region to open console in (defaults to profile default or SSO region)
        #[arg(long)]
        region: Option<String>,
    },

    /// Check SSO session status
    Status {
        /// Output in JSON format for scripting
        #[arg(long)]
        json: bool,
    },

    /// Logout from AWS SSO
    Logout,

    /// Manage SSO sessions
    Session {
        #[command(subcommand)]
        command: SessionCommands,
    },

    /// Import profiles or SSO sessions from user-managed section to awsom management
    ///
    /// Moves sections from above the "Managed by awsom" marker to below it,
    /// allowing awsom to manage them with automatic sorting and organization.
    Import {
        /// Profile or SSO session name to import
        name: String,

        /// Type of section to import (profile or sso-session)
        #[arg(short, long, default_value = "profile")]
        section_type: String,

        /// Force import without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Generate shell completion scripts
    ///
    /// INSTALLATION:
    ///
    /// Bash:
    ///   eval "$(awsom completions bash)"    # Add to ~/.bashrc
    ///
    /// Zsh:
    ///   eval "$(awsom completions zsh)"     # Add to ~/.zshrc
    ///
    /// Fish:
    ///   awsom completions fish > ~/.config/fish/completions/awsom.fish
    ///
    /// PowerShell:
    ///   awsom completions powershell | Out-String | Invoke-Expression
    ///
    /// Elvish:
    ///   eval (awsom completions elvish | slurp)
    Completions {
        /// Shell type to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Subcommand, Debug)]
pub enum SessionCommands {
    /// Add a new SSO session
    Add {
        /// Session name
        #[arg(long)]
        name: String,

        /// SSO start URL
        #[arg(long)]
        start_url: String,

        /// SSO region
        #[arg(long)]
        region: String,
    },

    /// List all SSO sessions
    List {
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Delete an SSO session
    Delete {
        /// Session name to delete
        name: String,

        /// Force deletion without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Edit an SSO session
    Edit {
        /// Session name to edit
        name: String,

        /// New SSO start URL (optional)
        #[arg(long)]
        start_url: Option<String>,

        /// New SSO region (optional)
        #[arg(long)]
        region: Option<String>,
    },

    /// Switch to a different SSO session (for multi-session support)
    Switch {
        /// Session name to switch to
        name: String,
    },
}

#[derive(Debug, Clone, ValueEnum)]
#[allow(clippy::enum_variant_names)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
}

pub async fn execute(args: Cli) -> Result<()> {
    match args.command {
        Some(Commands::Login { force }) => {
            commands::login::execute(args.start_url, args.region, force).await
        }
        Some(Commands::List { format }) => {
            commands::list::execute(args.start_url, args.region, format).await
        }
        Some(Commands::Exec {
            account_id,
            account_name,
            role_name,
            command,
        }) => commands::exec::execute(account_id, account_name, role_name, command).await,
        Some(Commands::Export {
            account_id,
            account_name,
            role_name,
            profile,
        }) => commands::export::execute(account_id, account_name, role_name, profile).await,
        Some(Commands::Console {
            account_id,
            account_name,
            role_name,
            region,
        }) => commands::console::execute(account_id, account_name, role_name, region).await,
        Some(Commands::Status { json }) => commands::status::execute(json).await,
        Some(Commands::Logout) => commands::logout::execute(args.start_url, args.region).await,
        Some(Commands::Session { command }) => commands::session::execute(command).await,
        Some(Commands::Import {
            name,
            section_type,
            force,
        }) => commands::import::execute(name, section_type, force).await,
        Some(Commands::Completions { shell }) => {
            commands::completions::execute(shell);
            Ok(())
        }
        None => {
            // No command specified, launch TUI
            use crate::ui::App;
            let mut app = App::new()?;
            app.run().await
        }
    }
}
