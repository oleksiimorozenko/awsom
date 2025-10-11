use crate::auth::AuthManager;
use crate::config::Config;
use crate::credentials::CredentialManager;
use crate::error::{Result, SsoError};
use crate::models::SsoInstance;

pub async fn execute(
    account_id: Option<String>,
    account_name: Option<String>,
    role_name: String,
    region: Option<String>,
) -> Result<()> {
    // Load config
    let config = Config::load()?;
    let (start_url, sso_region) = config.get_sso_config()?;

    let instance = SsoInstance {
        start_url: start_url.to_string(),
        region: sso_region.to_string(),
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

    // Determine which region to use for console
    let console_region = region
        .as_deref()
        .or(config.profile_defaults.region.as_deref())
        .or(Some(instance.region.as_str()));

    eprintln!("Opening AWS Console in browser...");
    eprintln!("  Account: {}", account_id);
    eprintln!("  Role: {}", role_name);
    if let Some(r) = console_region {
        eprintln!("  Region: {}", r);
    }

    // Open console in browser
    crate::console::open_console(&creds, console_region)?;

    eprintln!("✓ Console opened successfully");

    Ok(())
}
