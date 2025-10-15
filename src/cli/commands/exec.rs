use crate::auth::AuthManager;
use crate::aws_config;
use crate::credentials::CredentialManager;
use crate::error::{Result, SsoError};
use crate::models::SsoInstance;
use std::process::Command;

pub async fn execute(
    account_id: Option<String>,
    account_name: Option<String>,
    role_name: String,
    session_name: Option<String>,
    start_url: Option<String>,
    region: Option<String>,
    command: Vec<String>,
) -> Result<()> {
    if command.is_empty() {
        return Err(SsoError::InvalidConfig("No command specified".to_string()));
    }

    // Resolve SSO session using the new 4-level priority logic
    let (start_url, region) = aws_config::resolve_sso_session(
        session_name.as_deref(),
        start_url.as_deref(),
        region.as_deref(),
    )?;

    let instance = SsoInstance {
        start_url,
        region,
        session_name: None,
    };

    // Get SSO token
    let auth = AuthManager::new()?;
    let token = auth
        .get_cached_token(&instance)?
        .ok_or(SsoError::NoSessionFound)?;

    if token.is_expired() {
        return Err(SsoError::TokenExpired);
    }

    // Determine account ID
    let account_id = if let Some(id) = account_id {
        id
    } else if let Some(name) = account_name {
        // Look up account ID by name
        let cred_manager = CredentialManager::new()?;
        let accounts = cred_manager
            .list_accounts(&instance.region, &token.access_token)
            .await?;

        accounts
            .into_iter()
            .find(|(_, acc_name)| acc_name == &name)
            .map(|(id, _)| id)
            .ok_or_else(|| SsoError::InvalidConfig(format!("Account '{}' not found", name)))?
    } else {
        return Err(SsoError::InvalidConfig(
            "Either --account-id or --account-name is required".to_string(),
        ));
    };

    // Get credentials
    let cred_manager = CredentialManager::new()?;
    let creds = cred_manager
        .get_role_credentials(
            &instance.region,
            &token.access_token,
            &account_id,
            &role_name,
        )
        .await?;

    // Execute command with credentials in environment
    let status = Command::new(&command[0])
        .args(&command[1..])
        .env("AWS_ACCESS_KEY_ID", &creds.access_key_id)
        .env("AWS_SECRET_ACCESS_KEY", &creds.secret_access_key)
        .env("AWS_SESSION_TOKEN", &creds.session_token)
        .env("AWS_REGION", &instance.region)
        .env("AWS_DEFAULT_REGION", &instance.region)
        .status()
        .map_err(SsoError::Io)?;

    // Exit with same code as the command
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}
