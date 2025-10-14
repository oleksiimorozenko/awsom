use crate::error::{Result, SsoError};
use crate::models::SsoToken;
use aws_sdk_ssooidc::Client as SsoOidcClient;
use chrono::{Duration, Utc};
use std::time::Duration as StdDuration;
use tokio::time::sleep;

const CLIENT_NAME: &str = "awsom";
const CLIENT_TYPE: &str = "public";
const POLL_INTERVAL_SECONDS: u64 = 5;

/// Device authorization information from StartDeviceAuthorization
#[derive(Debug, Clone)]
pub struct DeviceAuthorizationInfo {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    #[allow(dead_code)]
    pub expires_in: i32,
    pub interval: Option<i32>,
}

/// OIDC client for AWS SSO device flow authentication
pub struct OidcClient {
    client: SsoOidcClient,
    region: String,
}

impl OidcClient {
    pub async fn new(region: &str) -> Result<Self> {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        let client = SsoOidcClient::new(&config);

        Ok(Self {
            client,
            region: region.to_string(),
        })
    }

    /// Register this client with AWS SSO OIDC
    async fn register_client(&self) -> Result<(String, String)> {
        tracing::debug!("Registering client with SSO-OIDC");

        let response = self
            .client
            .register_client()
            .client_name(CLIENT_NAME)
            .client_type(CLIENT_TYPE)
            .send()
            .await
            .map_err(|e| SsoError::AwsSdk(format!("Failed to register client: {}", e)))?;

        let client_id = response
            .client_id()
            .ok_or_else(|| SsoError::AwsSdk("No client_id in response".to_string()))?
            .to_string();

        let client_secret = response
            .client_secret()
            .ok_or_else(|| SsoError::AwsSdk("No client_secret in response".to_string()))?
            .to_string();

        tracing::debug!("Client registered successfully");
        Ok((client_id, client_secret))
    }

    /// Start device authorization flow
    async fn start_device_authorization(
        &self,
        client_id: &str,
        client_secret: &str,
        start_url: &str,
    ) -> Result<DeviceAuthorizationInfo> {
        tracing::debug!("Starting device authorization for: {}", start_url);

        let response = self
            .client
            .start_device_authorization()
            .client_id(client_id)
            .client_secret(client_secret)
            .start_url(start_url)
            .send()
            .await
            .map_err(|e| {
                SsoError::AwsSdk(format!("Failed to start device authorization: {}", e))
            })?;

        Ok(DeviceAuthorizationInfo {
            device_code: response
                .device_code()
                .ok_or_else(|| SsoError::AwsSdk("No device_code in response".to_string()))?
                .to_string(),
            user_code: response
                .user_code()
                .ok_or_else(|| SsoError::AwsSdk("No user_code in response".to_string()))?
                .to_string(),
            verification_uri: response
                .verification_uri()
                .ok_or_else(|| SsoError::AwsSdk("No verification_uri in response".to_string()))?
                .to_string(),
            verification_uri_complete: response.verification_uri_complete().map(|s| s.to_string()),
            expires_in: response.expires_in(),
            interval: Some(response.interval()),
        })
    }

    /// Poll for token after user authorizes
    async fn poll_for_token(
        &self,
        client_id: &str,
        client_secret: &str,
        device_code: &str,
        poll_interval: u64,
    ) -> Result<SsoToken> {
        tracing::debug!("Polling for token with interval: {}s", poll_interval);

        loop {
            match self
                .client
                .create_token()
                .client_id(client_id)
                .client_secret(client_secret)
                .grant_type("urn:ietf:params:oauth:grant-type:device_code")
                .device_code(device_code)
                .send()
                .await
            {
                Ok(response) => {
                    tracing::debug!("Token received successfully");

                    let access_token = response
                        .access_token()
                        .ok_or_else(|| SsoError::AwsSdk("No access_token in response".to_string()))?
                        .to_string();

                    let expires_in = response.expires_in();
                    let expires_at = Utc::now() + Duration::seconds(expires_in as i64);

                    tracing::debug!("Token expires in {} seconds", expires_in);

                    return Ok(SsoToken {
                        access_token,
                        expires_at,
                        refresh_token: response.refresh_token().map(|s| s.to_string()),
                        region: Some(self.region.clone()),
                    });
                }
                Err(err) => {
                    // Check error metadata for the error code
                    use aws_sdk_ssooidc::error::ProvideErrorMetadata;

                    if let Some(code) = err.code() {
                        tracing::debug!(
                            "CreateToken error: {} - {}",
                            code,
                            err.message().unwrap_or("")
                        );

                        match code {
                            "AuthorizationPendingException" => {
                                // User hasn't authorized yet, continue polling
                                sleep(StdDuration::from_secs(poll_interval)).await;
                                continue;
                            }
                            "SlowDownException" => {
                                // We're polling too fast, slow down
                                tracing::debug!("SlowDown requested, increasing poll interval");
                                sleep(StdDuration::from_secs(poll_interval + 5)).await;
                                continue;
                            }
                            "ExpiredTokenException" => {
                                return Err(SsoError::AuthorizationExpired);
                            }
                            _ => {
                                return Err(SsoError::AwsSdk(format!(
                                    "Token creation failed with error code '{}': {}",
                                    code,
                                    err.message().unwrap_or("unknown error")
                                )));
                            }
                        }
                    } else {
                        // No error code, return generic error
                        return Err(SsoError::AwsSdk(format!("Token creation failed: {}", err)));
                    }
                }
            }
        }
    }

    /// Perform complete device flow authentication
    pub async fn perform_device_flow(&self, start_url: &str, headless: bool) -> Result<SsoToken> {
        // Step 1: Register client
        let (client_id, client_secret) = self.register_client().await?;

        // Step 2: Start device authorization
        let auth_info = self
            .start_device_authorization(&client_id, &client_secret, start_url)
            .await?;

        // Step 3: Display authorization info to user
        self.display_authorization_prompt(&auth_info, headless)?;

        // Step 4: Poll for token
        let poll_interval = auth_info
            .interval
            .map(|i| i as u64)
            .unwrap_or(POLL_INTERVAL_SECONDS);

        self.poll_for_token(
            &client_id,
            &client_secret,
            &auth_info.device_code,
            poll_interval,
        )
        .await
    }

    /// Perform device flow authentication with callback for displaying auth info
    /// This version allows the caller to control how the auth info is displayed
    pub async fn perform_device_flow_with_callback<F>(
        &self,
        start_url: &str,
        display_callback: F,
    ) -> Result<SsoToken>
    where
        F: FnOnce(&DeviceAuthorizationInfo) -> Result<()>,
    {
        // Step 1: Register client
        let (client_id, client_secret) = self.register_client().await?;

        // Step 2: Start device authorization
        let auth_info = self
            .start_device_authorization(&client_id, &client_secret, start_url)
            .await?;

        // Step 3: Call display callback (caller controls display)
        display_callback(&auth_info)?;

        // Step 4: Poll for token
        let poll_interval = auth_info
            .interval
            .map(|i| i as u64)
            .unwrap_or(POLL_INTERVAL_SECONDS);

        self.poll_for_token(
            &client_id,
            &client_secret,
            &auth_info.device_code,
            poll_interval,
        )
        .await
    }

    /// Display authorization prompt to user and optionally open browser
    fn display_authorization_prompt(
        &self,
        auth_info: &DeviceAuthorizationInfo,
        headless: bool,
    ) -> Result<()> {
        eprintln!("\n=== AWS SSO Login ===");

        if headless {
            // Headless mode - don't try to open browser, show clear instructions
            eprintln!("Running in headless mode - please open browser manually:");
            eprintln!();
            eprintln!("Visit: {}", auth_info.verification_uri);
            eprintln!("Enter code: {}", auth_info.user_code);
            eprintln!();
        } else {
            // Normal mode - try to open browser
            eprintln!("Opening browser to: {}", auth_info.verification_uri);
            eprintln!("\nIf browser doesn't open automatically, visit:");
            eprintln!("  {}", auth_info.verification_uri);
            eprintln!("\nAnd enter code: {}\n", auth_info.user_code);

            // Try to open browser with complete URL if available
            let url_to_open = auth_info
                .verification_uri_complete
                .as_ref()
                .unwrap_or(&auth_info.verification_uri);

            if let Err(e) = webbrowser::open(url_to_open) {
                eprintln!("Could not open browser automatically: {}", e);
                eprintln!("Please open the URL manually.\n");
            }
        }

        eprintln!("Waiting for authorization...");

        Ok(())
    }
}
