// AWS SSO OIDC authentication module
mod oidc;
mod token_cache;

pub use oidc::{DeviceAuthorizationInfo, OidcClient};
pub use token_cache::TokenCache;

use crate::error::Result;
use crate::models::{SsoInstance, SsoToken};

/// High-level authentication interface
pub struct AuthManager {
    token_cache: TokenCache,
}

impl AuthManager {
    pub fn new() -> Result<Self> {
        Ok(Self {
            token_cache: TokenCache::new()?,
        })
    }

    /// Get cached token if valid, None if expired or not found
    pub fn get_cached_token(&self, instance: &SsoInstance) -> Result<Option<SsoToken>> {
        self.token_cache.get_token(instance)
    }

    /// Save token to cache
    pub fn save_token(&self, instance: &SsoInstance, token: SsoToken) -> Result<()> {
        self.token_cache.save_token(instance, token)
    }

    /// Remove token from cache (logout)
    pub fn remove_token(&self, instance: &SsoInstance) -> Result<()> {
        self.token_cache.remove_token(instance)
    }

    /// Start interactive SSO login flow
    pub async fn login(
        &self,
        instance: &SsoInstance,
        force_refresh: bool,
        headless: bool,
    ) -> Result<SsoToken> {
        // Check cache first unless force_refresh
        if !force_refresh {
            if let Some(token) = self.get_cached_token(instance)? {
                if !token.is_expired() {
                    return Ok(token);
                }
            }
        }

        // Initiate OIDC device flow
        let oidc_client = OidcClient::new(&instance.region).await?;
        let token = oidc_client
            .perform_device_flow(&instance.start_url, headless)
            .await?;

        // Cache the token
        self.save_token(instance, token.clone())?;

        Ok(token)
    }

    /// Start interactive SSO login flow with custom display callback
    /// This allows the TUI to display the device code properly
    pub async fn login_with_callback<F>(
        &self,
        instance: &SsoInstance,
        force_refresh: bool,
        display_callback: F,
    ) -> Result<SsoToken>
    where
        F: FnOnce(&DeviceAuthorizationInfo) -> Result<()>,
    {
        // Check cache first unless force_refresh
        if !force_refresh {
            if let Some(token) = self.get_cached_token(instance)? {
                if !token.is_expired() {
                    return Ok(token);
                }
            }
        }

        // Initiate OIDC device flow with callback
        let oidc_client = OidcClient::new(&instance.region).await?;
        let token = oidc_client
            .perform_device_flow_with_callback(&instance.start_url, display_callback)
            .await?;

        // Cache the token
        self.save_token(instance, token.clone())?;

        Ok(token)
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new().expect("Failed to initialize AuthManager")
    }
}
