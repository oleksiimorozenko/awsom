use crate::auth::AuthManager;
use crate::config::Config;
use crate::error::{Result, SsoError};
use crate::models::SsoInstance;

pub async fn execute(start_url: Option<String>, region: Option<String>) -> Result<()> {
    // Load config file as fallback
    let config = Config::load()?;

    // Use CLI args, then env vars (already loaded in config), then config file
    let start_url = start_url
        .or(config.sso.start_url)
        .ok_or_else(|| {
            SsoError::InvalidConfig(
                "SSO start URL is required. Provide --start-url, set AWS_SSO_START_URL, or configure in config file".to_string(),
            )
        })?;

    let region = region
        .or(config.sso.region)
        .ok_or_else(|| {
            SsoError::InvalidConfig(
                "SSO region is required. Provide --region, set AWS_SSO_REGION, or configure in config file".to_string(),
            )
        })?;

    let instance = SsoInstance { start_url, region };

    let auth = AuthManager::new()?;
    auth.remove_token(&instance)?;

    println!("✓ Logged out successfully");

    Ok(())
}
