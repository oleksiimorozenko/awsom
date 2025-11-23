use crate::auth::AuthManager;
use crate::error::Result;
use crate::models::SsoInstance;

pub async fn execute(
    session_name: Option<String>,
    start_url: String,
    region: String,
    json: bool,
) -> Result<()> {
    let instance = SsoInstance {
        session_name,
        start_url,
        region,
    };

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
