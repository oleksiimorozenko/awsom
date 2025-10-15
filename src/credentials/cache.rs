use crate::error::{Result, SsoError};
use crate::models::{AccountRole, RoleCredentials, SsoInstance};
use sha1::{Digest, Sha1};
use std::fs;
use std::path::PathBuf;

/// Credential cache compatible with AWS CLI v2
/// Stores credentials in ~/.aws/cli/cache/
pub struct CredentialCache {
    cache_dir: PathBuf,
}

impl CredentialCache {
    pub fn new() -> Result<Self> {
        let cache_dir = dirs::home_dir()
            .ok_or_else(|| SsoError::CacheError("Could not determine home directory".to_string()))?
            .join(".aws")
            .join("cli")
            .join("cache");

        // Create cache directory if it doesn't exist
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)?;
        }

        Ok(Self { cache_dir })
    }

    /// Generate cache key for a role
    fn cache_key(&self, instance: &SsoInstance, role: &AccountRole) -> String {
        let key_str = format!(
            "{}:{}:{}",
            instance.start_url, role.account_id, role.role_name
        );
        let mut hasher = Sha1::new();
        hasher.update(key_str.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Get path to cache file
    fn cache_file_path(&self, instance: &SsoInstance, role: &AccountRole) -> PathBuf {
        self.cache_dir
            .join(format!("{}.json", self.cache_key(instance, role)))
    }

    /// Get cached credentials
    pub fn get_credentials(
        &self,
        instance: &SsoInstance,
        role: &AccountRole,
    ) -> Result<Option<RoleCredentials>> {
        let cache_file = self.cache_file_path(instance, role);

        if !cache_file.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(&cache_file)
            .map_err(|e| SsoError::CacheError(format!("Failed to read cache file: {}", e)))?;

        let creds: RoleCredentials = serde_json::from_str(&contents)?;

        // Return None if credentials are expired
        if creds.is_expired() {
            return Ok(None);
        }

        Ok(Some(creds))
    }

    /// Save credentials to cache
    pub fn save_credentials(
        &self,
        instance: &SsoInstance,
        role: &AccountRole,
        creds: &RoleCredentials,
    ) -> Result<()> {
        let cache_file = self.cache_file_path(instance, role);

        let json = serde_json::to_string_pretty(creds)?;

        fs::write(&cache_file, json)
            .map_err(|e| SsoError::CacheError(format!("Failed to write cache file: {}", e)))?;

        Ok(())
    }

    /// Remove credentials from cache
    pub fn remove_credentials(&self, instance: &SsoInstance, role: &AccountRole) -> Result<()> {
        let cache_file = self.cache_file_path(instance, role);

        if cache_file.exists() {
            fs::remove_file(&cache_file)
                .map_err(|e| SsoError::CacheError(format!("Failed to remove cache file: {}", e)))?;
        }

        Ok(())
    }

    /// Clear all cached credentials
    pub fn clear_all(&self) -> Result<()> {
        if self.cache_dir.exists() {
            for entry in fs::read_dir(&self.cache_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                    fs::remove_file(path)?;
                }
            }
        }
        Ok(())
    }
}
