// Credential fetching and caching
mod cache;
mod fetcher;

pub use cache::CredentialCache;
pub use fetcher::CredentialFetcher;

use crate::error::Result;
use crate::models::{AccountRole, RoleCredentials, SsoInstance, SsoToken};

/// High-level credential management
pub struct CredentialManager {
    cache: CredentialCache,
}

impl CredentialManager {
    pub fn new() -> Result<Self> {
        Ok(Self {
            cache: CredentialCache::new()?,
        })
    }

    /// Get credentials for a role, fetching if necessary
    pub async fn get_credentials(
        &self,
        instance: &SsoInstance,
        token: &SsoToken,
        role: &AccountRole,
    ) -> Result<RoleCredentials> {
        // Check cache first
        if let Some(creds) = self.cache.get_credentials(instance, role)? {
            if !creds.is_expired() {
                return Ok(creds);
            }
        }

        // Fetch fresh credentials
        let fetcher = CredentialFetcher::new(&instance.region).await?;
        let creds = fetcher
            .fetch_credentials(&token.access_token, &role.account_id, &role.role_name)
            .await?;

        // Cache for future use
        self.cache.save_credentials(instance, role, &creds)?;

        Ok(creds)
    }

    /// List all available accounts
    pub async fn list_accounts(
        &self,
        region: &str,
        access_token: &str,
    ) -> Result<Vec<(String, String)>> {
        let fetcher = CredentialFetcher::new(region).await?;
        fetcher.list_accounts(access_token).await
    }

    /// List roles for a specific account
    pub async fn list_account_roles(
        &self,
        region: &str,
        access_token: &str,
        account_id: &str,
    ) -> Result<Vec<String>> {
        let fetcher = CredentialFetcher::new(region).await?;
        fetcher.list_account_roles(access_token, account_id).await
    }

    /// Get role credentials directly (without instance/caching)
    pub async fn get_role_credentials(
        &self,
        region: &str,
        access_token: &str,
        account_id: &str,
        role_name: &str,
    ) -> Result<RoleCredentials> {
        let fetcher = CredentialFetcher::new(region).await?;
        fetcher
            .fetch_credentials(access_token, account_id, role_name)
            .await
    }

    /// Clear cached credentials for a role
    pub fn clear_credentials(&self, instance: &SsoInstance, role: &AccountRole) -> Result<()> {
        self.cache.remove_credentials(instance, role)
    }

    /// Clear all cached credentials
    pub fn clear_all(&self) -> Result<()> {
        self.cache.clear_all()
    }
}

impl Default for CredentialManager {
    fn default() -> Self {
        Self::new().expect("Failed to initialize CredentialManager")
    }
}
