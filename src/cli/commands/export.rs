use crate::auth::AuthManager;
use crate::config::Config;
use crate::credentials::CredentialManager;
use crate::error::{Result, SsoError};
use crate::models::SsoInstance;

pub async fn execute(
    account_id: Option<String>,
    account_name: Option<String>,
    role_name: String,
    profile_name: Option<String>,
) -> Result<()> {
    // Load config
    let config = Config::load()?;
    let (start_url, region) = config.get_sso_config()?;

    let instance = SsoInstance {
        start_url: start_url.to_string(),
        region: region.to_string(),
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

    // If profile name specified, write to AWS credentials file
    if let Some(profile) = profile_name {
        // Use profile defaults for region and output format
        let profile_region = config
            .profile_defaults
            .region
            .as_deref()
            .unwrap_or(&instance.region);
        let output_format = config.profile_defaults.output.as_deref();

        crate::aws_config::write_credentials(&profile, &creds, profile_region, output_format)?;
        eprintln!("✓ Wrote credentials to ~/.aws/credentials");
        eprintln!("  Profile: {}", profile);
        eprintln!("  Region: {}", profile_region);
        if let Some(output) = output_format {
            eprintln!("  Output format: {}", output);
        }
        eprintln!("  Expires: {}", creds.expiration_display());
        eprintln!("\nUse with: aws s3 ls --profile {}", profile);
    } else {
        // Output as shell export commands
        println!("export AWS_ACCESS_KEY_ID=\"{}\"", creds.access_key_id);
        println!(
            "export AWS_SECRET_ACCESS_KEY=\"{}\"",
            creds.secret_access_key
        );
        println!("export AWS_SESSION_TOKEN=\"{}\"", creds.session_token);
        println!("export AWS_REGION=\"{}\"", instance.region);
        println!(
            "# Credentials expire at: {}",
            creds.expiration.format("%Y-%m-%d %H:%M:%S UTC")
        );
    }

    Ok(())
}
