use crate::error::{Result, SsoError};
use crate::models::RoleCredentials;
use aws_sdk_sso::Client as SsoClient;
use chrono::{TimeZone, Utc};

/// Fetches role credentials from AWS SSO
pub struct CredentialFetcher {
    client: SsoClient,
}

impl CredentialFetcher {
    pub async fn new(region: &str) -> Result<Self> {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        let client = SsoClient::new(&config);

        Ok(Self { client })
    }

    /// Fetch credentials for a specific account/role
    pub async fn fetch_credentials(
        &self,
        access_token: &str,
        account_id: &str,
        role_name: &str,
    ) -> Result<RoleCredentials> {
        let response = self
            .client
            .get_role_credentials()
            .access_token(access_token)
            .account_id(account_id)
            .role_name(role_name)
            .send()
            .await
            .map_err(|e| SsoError::AwsSdk(format!("Failed to get role credentials: {}", e)))?;

        let role_creds = response
            .role_credentials()
            .ok_or_else(|| SsoError::AwsSdk("No role_credentials in response".to_string()))?;

        let access_key_id = role_creds
            .access_key_id()
            .ok_or_else(|| SsoError::AwsSdk("No access_key_id in credentials".to_string()))?
            .to_string();

        let secret_access_key = role_creds
            .secret_access_key()
            .ok_or_else(|| SsoError::AwsSdk("No secret_access_key in credentials".to_string()))?
            .to_string();

        let session_token = role_creds
            .session_token()
            .ok_or_else(|| SsoError::AwsSdk("No session_token in credentials".to_string()))?
            .to_string();

        let expiration_ms = role_creds.expiration();
        let expiration = Utc
            .timestamp_millis_opt(expiration_ms)
            .single()
            .ok_or_else(|| SsoError::AwsSdk("Invalid expiration timestamp".to_string()))?;

        Ok(RoleCredentials {
            access_key_id,
            secret_access_key,
            session_token,
            expiration,
        })
    }

    /// List available accounts for the user
    pub async fn list_accounts(&self, access_token: &str) -> Result<Vec<(String, String)>> {
        let mut accounts = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self.client.list_accounts().access_token(access_token);

            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request
                .send()
                .await
                .map_err(|e| SsoError::AwsSdk(format!("Failed to list accounts: {}", e)))?;

            for account in response.account_list() {
                let account_id = account.account_id().unwrap_or("").to_string();
                let account_name = account.account_name().unwrap_or("").to_string();
                accounts.push((account_id, account_name));
            }

            next_token = response.next_token().map(|s| s.to_string());
            if next_token.is_none() {
                break;
            }
        }

        Ok(accounts)
    }

    /// List available roles for an account
    pub async fn list_account_roles(
        &self,
        access_token: &str,
        account_id: &str,
    ) -> Result<Vec<String>> {
        let mut roles = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self
                .client
                .list_account_roles()
                .access_token(access_token)
                .account_id(account_id);

            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request
                .send()
                .await
                .map_err(|e| SsoError::AwsSdk(format!("Failed to list account roles: {}", e)))?;

            for role in response.role_list() {
                if let Some(role_name) = role.role_name() {
                    roles.push(role_name.to_string());
                }
            }

            next_token = response.next_token().map(|s| s.to_string());
            if next_token.is_none() {
                break;
            }
        }

        Ok(roles)
    }
}
