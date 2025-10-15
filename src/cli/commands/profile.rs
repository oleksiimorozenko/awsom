// Profile management CLI commands
use crate::cli::ProfileCommands;
use crate::error::Result;

pub async fn execute(
    command: ProfileCommands,
    start_url: Option<String>,
    region: Option<String>,
) -> Result<()> {
    match command {
        ProfileCommands::List {
            session_name,
            format,
        } => crate::cli::commands::list::execute(session_name, start_url, region, format).await,
        ProfileCommands::Start { profile_name } => profile_start(profile_name).await,
        ProfileCommands::Exec {
            account_id,
            account_name,
            role_name,
            session_name,
            command,
        } => {
            crate::cli::commands::exec::execute(
                account_id,
                account_name,
                role_name,
                session_name,
                start_url,
                region,
                command,
            )
            .await
        }
        ProfileCommands::Export {
            account_id,
            account_name,
            role_name,
            session_name,
            profile,
        } => {
            crate::cli::commands::export::execute(
                account_id,
                account_name,
                role_name,
                session_name,
                start_url,
                region,
                profile,
            )
            .await
        }
        ProfileCommands::Console {
            account_id,
            account_name,
            role_name,
            session_name,
            region: console_region,
        } => {
            crate::cli::commands::console::execute(
                account_id,
                account_name,
                role_name,
                session_name,
                start_url,
                region,
                console_region,
            )
            .await
        }
    }
}

async fn profile_start(profile_name: String) -> Result<()> {
    use crate::aws_config;
    use crate::credentials::CredentialManager;
    use crate::error::SsoError;
    use crate::models::AccountRole;

    println!("Refreshing credentials for profile '{}'...", profile_name);
    println!();

    // Step 1: Get profile details from config
    let profile_details = aws_config::get_profile_details(&profile_name)?.ok_or_else(|| {
        SsoError::ConfigError(format!(
            "Profile '{}' not found in ~/.aws/config.\n\n\
                 Use the TUI (run 'awsom') to create profiles interactively.",
            profile_name
        ))
    })?;

    // Step 2: Verify this profile has SSO configuration
    let sso_session = profile_details.sso_session.ok_or_else(|| {
        SsoError::ConfigError(format!(
            "Profile '{}' is not an SSO profile (no sso_session configured).\n\n\
             This command only works with SSO profiles managed by awsom.",
            profile_name
        ))
    })?;

    let account_id = profile_details.sso_account_id.ok_or_else(|| {
        SsoError::ConfigError(format!(
            "Profile '{}' is missing sso_account_id configuration.",
            profile_name
        ))
    })?;

    let role_name = profile_details.sso_role_name.ok_or_else(|| {
        SsoError::ConfigError(format!(
            "Profile '{}' is missing sso_role_name configuration.",
            profile_name
        ))
    })?;

    println!("  Profile: {}", profile_name);
    println!("  SSO Session: {}", sso_session);
    println!("  Account ID: {}", account_id);
    println!("  Role: {}", role_name);
    println!();

    // Step 3: Resolve SSO session to get start_url and region
    let (start_url, sso_region) = aws_config::resolve_sso_session(Some(&sso_session), None, None)?;

    // Step 4: Get SSO token
    let token_cache = crate::auth::TokenCache::new()?;

    // Create SsoInstance from session info
    let sso_instance = crate::models::SsoInstance {
        session_name: Some(sso_session.clone()),
        start_url: start_url.clone(),
        region: sso_region.clone(),
    };

    let token = token_cache.get_token(&sso_instance)?.ok_or_else(|| {
        SsoError::AuthenticationFailed(format!(
            "No valid SSO token found for session '{}'.\n\n\
                 Run 'awsom session login --session-name {}' to authenticate first.",
            sso_session, sso_session
        ))
    })?;

    // Check if token is expired
    if token.is_expired() {
        return Err(SsoError::AuthenticationFailed(format!(
            "SSO token for session '{}' has expired.\n\n\
             Run 'awsom session login --session-name {}' to re-authenticate.",
            sso_session, sso_session
        )));
    }

    println!("✓ Found valid SSO token");

    // Step 5: Fetch fresh credentials
    let credential_manager = CredentialManager::new()?;
    let credentials = credential_manager
        .get_role_credentials(&sso_region, &token.access_token, &account_id, &role_name)
        .await?;

    println!("✓ Fetched temporary credentials");

    // Step 6: Write credentials to file
    let account_role = AccountRole {
        account_id: account_id.clone(),
        account_name: account_id.clone(), // We don't have the friendly name here
        role_name: role_name.clone(),
    };

    aws_config::write_credentials_with_metadata(
        &profile_name,
        &credentials,
        profile_details.region.as_deref().unwrap_or(&sso_region),
        profile_details.output.as_deref(),
        Some(&account_role),
    )?;

    println!("✓ Updated credentials in ~/.aws/credentials");
    println!();
    println!("Profile '{}' is ready to use.", profile_name);
    println!("Credentials valid until: {}", credentials.expiration);

    Ok(())
}
