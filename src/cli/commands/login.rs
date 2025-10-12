use crate::auth::AuthManager;
use crate::error::Result;
use crate::models::SsoInstance;
use crate::sso_config;

pub async fn execute(start_url: Option<String>, region: Option<String>, force: bool) -> Result<()> {
    // Get SSO config from CLI args, env vars, or ~/.aws/config
    let (start_url, region) = sso_config::get_sso_config(start_url, region)?;

    let instance = SsoInstance { start_url, region };

    let auth = AuthManager::new()?;
    let token = auth.login(&instance, force).await?;

    println!("✓ Login successful!");
    println!("  Token expires in: {}", token.expiration_display());

    Ok(())
}
