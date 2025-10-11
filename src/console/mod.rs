// AWS Console federation and URL generation
use crate::error::{Result, SsoError};
use crate::models::RoleCredentials;
use serde_json::json;
use std::collections::HashMap;

/// Generate an AWS Console sign-in URL using temporary credentials
///
/// This uses the AWS Federation endpoint to create a sign-in token
/// that allows accessing the AWS Console with temporary credentials.
pub fn generate_console_url(creds: &RoleCredentials, region: Option<&str>) -> Result<String> {
    // Create the session credentials JSON
    let session_json = json!({
        "sessionId": creds.access_key_id,
        "sessionKey": creds.secret_access_key,
        "sessionToken": creds.session_token,
    });

    // URL-encode the session JSON
    let session_string = session_json.to_string();
    let encoded_session = urlencoding::encode(&session_string);

    // AWS Federation endpoint
    let federation_url = "https://signin.aws.amazon.com/federation";

    // Step 1: Get the sign-in token
    let token_url = format!(
        "{}?Action=getSigninToken&SessionDuration={}&Session={}",
        federation_url,
        43200, // 12 hours (max for federated users)
        encoded_session
    );

    // Make HTTP request to get the token
    tracing::debug!("Requesting sign-in token from AWS federation endpoint");
    let response = reqwest::blocking::get(&token_url).map_err(|e| {
        SsoError::AuthenticationFailed(format!("Failed to get sign-in token: {}", e))
    })?;

    let token_response: HashMap<String, String> = response.json().map_err(|e| {
        SsoError::AuthenticationFailed(format!("Failed to parse token response: {}", e))
    })?;

    let signin_token = token_response
        .get("SigninToken")
        .ok_or_else(|| SsoError::AuthenticationFailed("No SigninToken in response".to_string()))?;

    // Step 2: Build the console URL
    let console_region = region.unwrap_or("us-east-1");
    let destination = format!("https://console.aws.amazon.com/?region={}", console_region);
    let encoded_destination = urlencoding::encode(&destination);

    let console_url = format!(
        "{}?Action=login&Issuer=awsom&Destination={}&SigninToken={}",
        federation_url, encoded_destination, signin_token
    );

    Ok(console_url)
}

/// Open the AWS Console in the default browser
pub fn open_console(creds: &RoleCredentials, region: Option<&str>) -> Result<()> {
    let url = generate_console_url(creds, region)?;

    tracing::info!("Opening AWS Console in browser");
    webbrowser::open(&url).map_err(|e| SsoError::BrowserLaunchFailed(format!("{}", e)))?;

    Ok(())
}
