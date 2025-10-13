// Session management CLI commands
use crate::aws_config::{self, SsoSession};
use crate::cli::SessionCommands;
use crate::error::{Result, SsoError};
use std::io::{self, Write};

pub async fn execute(command: SessionCommands) -> Result<()> {
    match command {
        SessionCommands::Add {
            name,
            start_url,
            region,
        } => add_session(name, start_url, region).await,
        SessionCommands::List { format } => list_sessions(format).await,
        SessionCommands::Delete { name, force } => delete_session(name, force).await,
        SessionCommands::Edit {
            name,
            start_url,
            region,
        } => edit_session(name, start_url, region).await,
        SessionCommands::Switch { name } => switch_session(name).await,
    }
}

async fn add_session(name: String, start_url: String, region: String) -> Result<()> {
    // Check if session already exists
    let existing_sessions = aws_config::read_all_sso_sessions()?;
    if existing_sessions.iter().any(|s| s.session_name == name) {
        return Err(SsoError::ConfigError(format!(
            "Session '{}' already exists. Use 'session edit' to modify it.",
            name
        )));
    }

    // Create new session
    let session = SsoSession {
        session_name: name.clone(),
        sso_start_url: start_url.clone(),
        sso_region: region.clone(),
        sso_registration_scopes: "sso:account:access".to_string(),
    };

    // Write to config
    aws_config::write_sso_session(&session)?;

    println!("✓ Added SSO session '{}' to ~/.aws/config", name);
    println!("  Start URL: {}", start_url);
    println!("  Region: {}", region);
    println!();
    println!("Run 'awsom login' or launch the TUI to authenticate with this session.");

    Ok(())
}

async fn list_sessions(format: String) -> Result<()> {
    let sessions = aws_config::read_all_sso_sessions()?;

    if sessions.is_empty() {
        if format == "json" {
            println!("[]");
        } else {
            println!("No SSO sessions configured.");
            println!();
            println!("Add a session with: awsom session add --name <name> --start-url <url> --region <region>");
        }
        return Ok(());
    }

    match format.as_str() {
        "json" => {
            let json_sessions: Vec<_> = sessions
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "name": s.session_name,
                        "start_url": s.sso_start_url,
                        "region": s.sso_region,
                        "registration_scopes": s.sso_registration_scopes,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&json_sessions)?);
        }
        _ => {
            println!("SSO Sessions ({}):", sessions.len());
            println!();
            for session in sessions {
                println!("  {}", session.session_name);
                println!("    Start URL: {}", session.sso_start_url);
                println!("    Region: {}", session.sso_region);
                println!();
            }
        }
    }

    Ok(())
}

async fn delete_session(name: String, force: bool) -> Result<()> {
    // Check if session exists
    let existing_sessions = aws_config::read_all_sso_sessions()?;
    let session = existing_sessions
        .iter()
        .find(|s| s.session_name == name)
        .ok_or_else(|| {
            SsoError::ConfigError(format!(
                "Session '{}' not found. Use 'awsom session list' to see available sessions.",
                name
            ))
        })?;

    // Confirm deletion unless --force is used
    if !force {
        print!(
            "Are you sure you want to delete session '{}'? (y/N): ",
            name
        );
        io::stdout().flush().map_err(SsoError::Io)?;

        let mut response = String::new();
        io::stdin().read_line(&mut response).map_err(SsoError::Io)?;

        if !response.trim().eq_ignore_ascii_case("y") {
            println!("Deletion cancelled.");
            return Ok(());
        }
    }

    // Delete the session
    aws_config::delete_sso_session(&name)?;

    println!("✓ Deleted SSO session '{}'", name);
    println!("  Start URL was: {}", session.sso_start_url);
    println!("  Region was: {}", session.sso_region);

    Ok(())
}

async fn edit_session(
    name: String,
    start_url: Option<String>,
    region: Option<String>,
) -> Result<()> {
    // Check if session exists
    let existing_sessions = aws_config::read_all_sso_sessions()?;
    let mut session = existing_sessions
        .iter()
        .find(|s| s.session_name == name)
        .cloned()
        .ok_or_else(|| {
            SsoError::ConfigError(format!(
                "Session '{}' not found. Use 'awsom session list' to see available sessions.",
                name
            ))
        })?;

    // Check if at least one field is being updated
    if start_url.is_none() && region.is_none() {
        return Err(SsoError::ConfigError(
            "No changes specified. Use --start-url and/or --region to update the session."
                .to_string(),
        ));
    }

    // Apply updates
    let mut changes = Vec::new();
    if let Some(new_start_url) = start_url {
        changes.push(format!(
            "Start URL: {} → {}",
            session.sso_start_url, new_start_url
        ));
        session.sso_start_url = new_start_url;
    }
    if let Some(new_region) = region {
        changes.push(format!("Region: {} → {}", session.sso_region, new_region));
        session.sso_region = new_region;
    }

    // Write updated session
    aws_config::write_sso_session(&session)?;

    println!("✓ Updated SSO session '{}'", name);
    for change in changes {
        println!("  {}", change);
    }
    println!();
    println!(
        "Note: You may need to re-authenticate with 'awsom login' for the changes to take effect."
    );

    Ok(())
}

async fn switch_session(name: String) -> Result<()> {
    // Check if session exists
    let existing_sessions = aws_config::read_all_sso_sessions()?;
    let _session = existing_sessions
        .iter()
        .find(|s| s.session_name == name)
        .ok_or_else(|| {
            SsoError::ConfigError(format!(
                "Session '{}' not found. Use 'awsom session list' to see available sessions.",
                name
            ))
        })?;

    // For now, this is a placeholder for future multi-session support
    println!("✓ Session '{}' selected", name);
    println!();
    println!("Note: Multi-session switching is not yet fully implemented.");
    println!("For now, use the TUI (just run 'awsom') to switch between sessions interactively.");

    Ok(())
}
