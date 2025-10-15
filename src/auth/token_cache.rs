use crate::error::{Result, SsoError};
use crate::models::{SsoInstance, SsoToken};
use sha1::{Digest, Sha1};
use std::fs;
use std::path::PathBuf;

/// Token cache compatible with AWS CLI v2
/// Stores tokens in ~/.aws/sso/cache/
pub struct TokenCache {
    cache_dir: PathBuf,
}

impl TokenCache {
    pub fn new() -> Result<Self> {
        let cache_dir = dirs::home_dir()
            .ok_or_else(|| SsoError::CacheError("Could not determine home directory".to_string()))?
            .join(".aws")
            .join("sso")
            .join("cache");

        // Create cache directory if it doesn't exist
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)?;
        }

        Ok(Self { cache_dir })
    }

    /// Generate cache key (compatible with AWS CLI v2)
    /// Uses SHA1 of session_name when available (modern [sso-session] format),
    /// otherwise falls back to SHA1 of start_url (legacy SSO format)
    fn cache_key(&self, instance: &SsoInstance) -> String {
        let mut hasher = Sha1::new();

        // Use session_name if available (AWS CLI v2 with [sso-session]),
        // otherwise use start_url (legacy format)
        let key_material = instance
            .session_name
            .as_deref()
            .unwrap_or(&instance.start_url);

        hasher.update(key_material.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Get path to cache file for given instance
    fn cache_file_path(&self, instance: &SsoInstance) -> PathBuf {
        self.cache_dir
            .join(format!("{}.json", self.cache_key(instance)))
    }

    /// Get cached token for SSO instance
    pub fn get_token(&self, instance: &SsoInstance) -> Result<Option<SsoToken>> {
        let cache_file = self.cache_file_path(instance);

        if !cache_file.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(&cache_file)
            .map_err(|e| SsoError::CacheError(format!("Failed to read cache file: {}", e)))?;

        let token: SsoToken = serde_json::from_str(&contents)?;

        // Return None if token is expired
        if token.is_expired() {
            return Ok(None);
        }

        Ok(Some(token))
    }

    /// Save token to cache
    pub fn save_token(&self, instance: &SsoInstance, token: SsoToken) -> Result<()> {
        let cache_file = self.cache_file_path(instance);

        let json = serde_json::to_string_pretty(&token)?;

        fs::write(&cache_file, json)
            .map_err(|e| SsoError::CacheError(format!("Failed to write cache file: {}", e)))?;

        Ok(())
    }

    /// Remove token from cache (logout)
    pub fn remove_token(&self, instance: &SsoInstance) -> Result<()> {
        let cache_file = self.cache_file_path(instance);

        if cache_file.exists() {
            fs::remove_file(&cache_file)
                .map_err(|e| SsoError::CacheError(format!("Failed to remove cache file: {}", e)))?;
        }

        Ok(())
    }

    /// List all cached tokens
    pub fn list_tokens(&self) -> Result<Vec<(String, SsoToken)>> {
        let mut tokens = Vec::new();

        if !self.cache_dir.exists() {
            return Ok(tokens);
        }

        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(contents) = fs::read_to_string(&path) {
                    if let Ok(token) = serde_json::from_str::<SsoToken>(&contents) {
                        if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                            tokens.push((file_name.to_string(), token));
                        }
                    }
                }
            }
        }

        Ok(tokens)
    }
}
