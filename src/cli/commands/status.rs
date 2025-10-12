use crate::auth::AuthManager;
use crate::error::Result;
use crate::models::SsoInstance;
use crate::sso_config;

pub async fn execute(json: bool) -> Result<()> {
    // Check if SSO config is available
    if !sso_config::has_sso_config(None, None) {
        if json {
            println!("{{\"active\":false,\"reason\":\"not_configured\"}}");
        } else {
            println!("SSO not configured");
        }
        std::process::exit(1);
    }

    // Get SSO config from env vars or ~/.aws/config
    let (start_url, region) = sso_config::get_sso_config(None, None)?;

    let instance = SsoInstance { start_url, region };

    // Check for cached token
    let auth = AuthManager::new()?;

    match auth.get_cached_token(&instance)? {
        Some(token) => {
            if token.is_expired() {
                if json {
                    println!("{{\"active\":false,\"reason\":\"expired\"}}");
                } else {
                    println!("SSO session expired");
                }
                std::process::exit(1);
            } else {
                let expires_in_minutes = token.expires_in_minutes();
                if json {
                    println!(
                        "{{\"active\":true,\"expires_in_minutes\":{}}}",
                        expires_in_minutes
                    );
                } else {
                    println!(
                        "SSO session active (expires in {} minutes)",
                        expires_in_minutes
                    );
                }
                std::process::exit(0);
            }
        }
        None => {
            if json {
                println!("{{\"active\":false,\"reason\":\"no_session\"}}");
            } else {
                println!("No SSO session found");
            }
            std::process::exit(1);
        }
    }
}
