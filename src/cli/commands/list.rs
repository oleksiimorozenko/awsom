use crate::auth::AuthManager;
use crate::aws_config;
use crate::credentials::CredentialFetcher;
use crate::error::{Result, SsoError};
use crate::models::{AccountRole, SsoInstance};

pub async fn execute(
    session_name: Option<String>,
    start_url: Option<String>,
    region: Option<String>,
    format: String,
) -> Result<()> {
    // Resolve SSO session using the new 4-level priority logic
    let (start_url, region) = aws_config::resolve_sso_session(
        session_name.as_deref(),
        start_url.as_deref(),
        region.as_deref(),
    )?;

    let instance = SsoInstance {
        session_name: None,
        start_url: start_url.clone(),
        region: region.clone(),
    };

    // Get token
    let auth = AuthManager::new()?;
    let token = auth
        .get_cached_token(&instance)?
        .ok_or(SsoError::NoSessionFound)?;

    if token.is_expired() {
        return Err(SsoError::TokenExpired);
    }

    // List accounts and roles
    let fetcher = CredentialFetcher::new(&region).await?;
    let accounts = fetcher.list_accounts(&token.access_token).await?;

    let mut roles = Vec::new();
    for (account_id, account_name) in accounts {
        let account_roles = fetcher
            .list_account_roles(&token.access_token, &account_id)
            .await?;

        for role_name in account_roles {
            roles.push(AccountRole {
                account_id: account_id.clone(),
                account_name: account_name.clone(),
                role_name,
            });
        }
    }

    // Output
    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&roles)?);
    } else {
        println!("Available accounts and roles:\n");
        for role in roles {
            println!(
                "  {} ({}): {}",
                role.account_name, role.account_id, role.role_name
            );
        }
    }

    Ok(())
}
