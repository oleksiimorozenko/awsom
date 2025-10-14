// awsom - AWS Organization Manager

mod auth;
mod aws_config;
mod cli;
mod console;
mod credentials;
mod env;
mod error;
mod expiry;
mod models;
mod session;
mod sso_config;
mod ui;

use clap::Parser;
use error::Result;
use std::fs::OpenOptions;
use tracing_subscriber::fmt::writer::MakeWriterExt;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments first to get verbose flag
    let args = cli::Cli::parse();

    // Set headless mode override if --headless flag is set
    if args.headless {
        env::set_headless_override(true);
    }

    // Initialize tracing based on verbose flag
    let log_level = if args.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    // Check if running in TUI mode (no subcommand)
    let is_tui_mode = args.command.is_none();

    if is_tui_mode {
        // For TUI mode, write logs to a file to avoid breaking the UI
        let log_dir = dirs::cache_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join("awsom");

        // Create log directory if it doesn't exist
        let _ = std::fs::create_dir_all(&log_dir);

        let log_file = log_dir.join("awsom.log");

        // Open log file in append mode
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)
            .expect("Failed to open log file");

        // Initialize tracing to write to file
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env().add_directive(log_level.into()),
            )
            .with_writer(file.with_max_level(tracing::Level::TRACE))
            .with_ansi(false) // No color codes in file
            .init();
    } else {
        // For CLI commands, write logs to stderr as usual
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env().add_directive(log_level.into()),
            )
            .init();
    }

    // Execute the appropriate command
    cli::execute(args).await
}
