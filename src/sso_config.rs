// SSO Configuration - reads from ~/.aws/config instead of custom config file
use crate::aws_config::{read_sso_session, write_sso_session, SsoSession};
use crate::error::{Result, SsoError};

/// Get SSO configuration from ~/.aws/config or environment variables
/// Priority:
/// 1. Environment variables (AWS_SSO_START_URL, AWS_SSO_REGION)
/// 2. ~/.aws/config [sso-session] section
/// 3. CLI arguments (passed as parameters)
pub fn get_sso_config(
    start_url_arg: Option<String>,
    region_arg: Option<String>,
) -> Result<(String, String)> {
    // Priority 1: CLI arguments
    if let (Some(url), Some(region)) = (&start_url_arg, &region_arg) {
        return Ok((url.clone(), region.clone()));
    }

    // Priority 2: Environment variables
    let env_start_url = std::env::var("AWS_SSO_START_URL").ok();
    let env_region = std::env::var("AWS_SSO_REGION").ok();

    if let (Some(url), Some(region)) = (&env_start_url, &env_region) {
        return Ok((url.clone(), region.clone()));
    }

    // Priority 3: Read from ~/.aws/config
    if let Some(session) = read_sso_session()? {
        return Ok((session.sso_start_url, session.sso_region));
    }

    // Check if we have partial config from different sources
    let start_url = start_url_arg.or(env_start_url).ok_or_else(|| {
        SsoError::ConfigError(
            "SSO start URL not configured. Set AWS_SSO_START_URL environment variable, \
                 use --start-url flag, or configure [sso-session] in ~/.aws/config"
                .to_string(),
        )
    })?;

    let region = region_arg.or(env_region).ok_or_else(|| {
        SsoError::ConfigError(
            "SSO region not configured. Set AWS_SSO_REGION environment variable, \
             use --region flag, or configure [sso-session] in ~/.aws/config"
                .to_string(),
        )
    })?;

    Ok((start_url, region))
}

/// Check if SSO configuration is available from any source
pub fn has_sso_config(start_url_arg: Option<&String>, region_arg: Option<&String>) -> bool {
    // Check CLI args
    if start_url_arg.is_some() && region_arg.is_some() {
        return true;
    }

    // Check env vars
    if std::env::var("AWS_SSO_START_URL").is_ok() && std::env::var("AWS_SSO_REGION").is_ok() {
        return true;
    }

    // Check ~/.aws/config
    read_sso_session().ok().flatten().is_some()
}

/// Prompt user for SSO configuration and write to ~/.aws/config
/// Returns (start_url, region, session_name)
pub fn prompt_sso_config() -> Result<(String, String, String)> {
    use std::io::{self, Write};

    println!("\n=== AWS SSO Configuration ===");
    println!("No SSO session found in ~/.aws/config");
    println!("Please provide your AWS SSO details:\n");

    print!("SSO Start URL (e.g., https://my-org.awsapps.com/start): ");
    io::stdout().flush().unwrap();
    let mut start_url = String::new();
    io::stdin().read_line(&mut start_url).unwrap();
    let start_url = start_url.trim().to_string();

    if start_url.is_empty() {
        return Err(SsoError::ConfigError(
            "SSO start URL is required".to_string(),
        ));
    }

    print!("SSO Region (e.g., us-east-1): ");
    io::stdout().flush().unwrap();
    let mut region = String::new();
    io::stdin().read_line(&mut region).unwrap();
    let region = region.trim().to_string();

    if region.is_empty() {
        return Err(SsoError::ConfigError("SSO region is required".to_string()));
    }

    print!("SSO Session Name (default: default-sso): ");
    io::stdout().flush().unwrap();
    let mut session_name = String::new();
    io::stdin().read_line(&mut session_name).unwrap();
    let session_name = session_name.trim();
    let session_name = if session_name.is_empty() {
        "default-sso".to_string()
    } else {
        session_name.to_string()
    };

    // Write to ~/.aws/config
    let session = SsoSession {
        session_name: session_name.clone(),
        sso_start_url: start_url.clone(),
        sso_region: region.clone(),
        sso_registration_scopes: "sso:account:access".to_string(),
    };

    write_sso_session(&session)?;

    println!("\n✓ SSO configuration saved to ~/.aws/config");
    println!("  Session: [sso-session {}]", session_name);
    println!("  Start URL: {}", start_url);
    println!("  Region: {}\n", region);

    Ok((start_url, region, session_name))
}

/// Get default output format (can be made configurable later)
pub fn get_default_output_format() -> Option<&'static str> {
    // For now, return None to use AWS CLI's default
    // This can be enhanced later to read from environment or user preference
    None
}
