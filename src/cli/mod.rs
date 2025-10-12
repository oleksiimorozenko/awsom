// CLI interface
pub mod commands;

use crate::error::Result;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "awsom")]
#[command(about = "A TUI for managing AWS SSO sessions", long_about = None)]
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
