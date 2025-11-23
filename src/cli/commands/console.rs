use crate::auth::AuthManager;
use crate::aws_config;
use crate::credentials::CredentialManager;
use crate::error::{Result, SsoError};
use crate::models::SsoInstance;

#[allow(clippy::too_many_arguments)]
pub async fn execute(
    profile: Option<String>,
    account_id: Option<String>,
    account_name: Option<String>,
    role_name: Option<String>,
    session_name: Option<String>,
    sso_start_url: Option<String>,
    sso_region: Option<String>,
    console_region: Option<String>,
) -> Result<()> {
    // If --profile is provided, look up details from config
    let (account_id, role_name, session_name, console_region) = if let Some(profile_name) = profile
    {
        // Read profile details from config
        let details = aws_config::get_profile_details(&profile_name)?.ok_or_else(|| {
            SsoError::InvalidConfig(format!("Profile '{}' not found", profile_name))
        })?;

        let account_id = details.sso_account_id.ok_or_else(|| {
            SsoError::InvalidConfig(format!(
                "Profile '{}' has no sso_account_id. This command only works with SSO profiles.",
                profile_name
            ))
        })?;

        let role_name = details.sso_role_name.ok_or_else(|| {
            SsoError::InvalidConfig(format!(
                "Profile '{}' has no sso_role_name. This command only works with SSO profiles.",
                profile_name
            ))
        })?;

        // Use profile's sso_session if not overridden
        let session = session_name.or(details.sso_session);
        // Use profile's region for console if not overridden
        let region = console_region.or(details.region);

        (Some(account_id), role_name, session, region)
    } else {
        // Require role_name when not using --profile
        let role_name = role_name.ok_or_else(|| {
            SsoError::InvalidConfig("Either --profile or --role-name is required".to_string())
        })?;
        (account_id, role_name, session_name, console_region)
    };

    // Resolve SSO session using the new 4-level priority logic
    let (resolved_session_name, start_url, sso_region) = aws_config::resolve_sso_session(
        session_name.as_deref(),
        sso_start_url.as_deref(),
        sso_region.as_deref(),
    )?;

    let instance = SsoInstance {
        session_name: resolved_session_name,
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
            "Either --account-id, --account-name, or --profile is required".to_string(),
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
