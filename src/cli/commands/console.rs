use crate::auth::AuthManager;
use crate::aws_config;
use crate::credentials::CredentialManager;
use crate::error::{Result, SsoError};
use crate::models::SsoInstance;

pub async fn execute(
    account_id: Option<String>,
    account_name: Option<String>,
    role_name: String,
    session_name: Option<String>,
    sso_start_url: Option<String>,
    sso_region: Option<String>,
    console_region: Option<String>,
) -> Result<()> {
    // Resolve SSO session using the new 4-level priority logic
    let (start_url, sso_region) = aws_config::resolve_sso_session(
        session_name.as_deref(),
        sso_start_url.as_deref(),
        sso_region.as_deref(),
    )?;

    let instance = SsoInstance {
        session_name: None,
        start_url,
        region: sso_region,
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

    // Determine which region to use for console (use SSO region as default)
    let console_region_resolved = console_region.as_deref().or(Some(instance.region.as_str()));

    eprintln!("Opening AWS Console in browser...");
    eprintln!("  Account: {}", account_id);
    eprintln!("  Role: {}", role_name);
    if let Some(r) = console_region_resolved {
        eprintln!("  Region: {}", r);
    }

    // Open console in browser
    crate::console::open_console(&creds, console_region_resolved)?;

    eprintln!("✓ Console opened successfully");

    Ok(())
}
