//! Disk cache for profiles and accounts
//!
//! Stores profile data in platform-specific cache directory:
//! - Linux/macOS: ~/.cache/awsom/
//! - Windows: %LOCALAPPDATA%\awsom\cache\

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::models::AccountRole;

const CACHE_FILENAME: &str = "profiles.json";

/// Cached profile entry (matches ProfileEntry in ui/app.rs but serializable)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedProfile {
    pub profile_name: String,
    pub account_role: AccountRole,
    pub session_name: String,
    pub is_default: bool,
}

/// Profile cache with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileCache {
    /// When the cache was last updated
    pub cached_at: DateTime<Utc>,
    /// Cached profiles
    pub profiles: Vec<CachedProfile>,
}

impl ProfileCache {
    /// Create a new cache with current timestamp
    pub fn new(profiles: Vec<CachedProfile>) -> Self {
        Self {
            cached_at: Utc::now(),
            profiles,
        }
    }

    /// Check if cache is stale (older than threshold)
    #[allow(dead_code)]
    pub fn is_stale(&self, max_age_seconds: i64) -> bool {
        let age = Utc::now() - self.cached_at;
        age.num_seconds() > max_age_seconds
    }

    /// Get age of cache in human-readable format
    pub fn age_display(&self) -> String {
        let age = Utc::now() - self.cached_at;
        let secs = age.num_seconds();

        if secs < 60 {
            format!("{}s ago", secs)
        } else if secs < 3600 {
            format!("{}m ago", secs / 60)
        } else if secs < 86400 {
            format!("{}h ago", secs / 3600)
        } else {
            format!("{}d ago", secs / 86400)
        }
    }
}

/// Get the cache directory path
pub fn cache_dir() -> Result<PathBuf> {
    let base =
        dirs::cache_dir().ok_or_else(|| anyhow::anyhow!("Could not find cache directory"))?;
    Ok(base.join("awsom"))
}

/// Get the full path to the profiles cache file
pub fn cache_file_path() -> Result<PathBuf> {
    Ok(cache_dir()?.join(CACHE_FILENAME))
}

/// Save profiles to disk cache
pub fn save_profiles(profiles: &[CachedProfile]) -> Result<()> {
    let cache = ProfileCache::new(profiles.to_vec());
    let cache_dir = cache_dir()?;

    // Create cache directory if it doesn't exist
    fs::create_dir_all(&cache_dir)?;

    let cache_file = cache_dir.join(CACHE_FILENAME);
    let json = serde_json::to_string_pretty(&cache)?;
    fs::write(&cache_file, json)?;

    tracing::debug!(
        "Saved {} profiles to cache: {:?}",
        profiles.len(),
        cache_file
    );
    Ok(())
}

/// Check if AWS config or credentials files have been modified since cache was created
fn is_cache_invalidated(cache: &ProfileCache) -> bool {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return true, // Can't determine home, invalidate to be safe
    };

    let config_path = home.join(".aws").join("config");
    let credentials_path = home.join(".aws").join("credentials");

    // Check if either file was modified after the cache was created
    for path in [config_path, credentials_path] {
        if path.exists() {
            if let Ok(metadata) = std::fs::metadata(&path) {
                if let Ok(modified) = metadata.modified() {
                    // Convert SystemTime to DateTime<Utc>
                    let modified_dt: DateTime<Utc> = modified.into();
                    if modified_dt > cache.cached_at {
                        tracing::debug!(
                            "Cache invalidated: {:?} modified at {:?}, cache from {:?}",
                            path,
                            modified_dt,
                            cache.cached_at
                        );
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Load profiles from disk cache
/// Returns None if cache doesn't exist or is invalidated (config/credentials modified)
pub fn load_profiles() -> Result<Option<ProfileCache>> {
    let cache_file = cache_file_path()?;

    if !cache_file.exists() {
        tracing::debug!("No profile cache found at {:?}", cache_file);
        return Ok(None);
    }

    let json = fs::read_to_string(&cache_file)?;
    let cache: ProfileCache = serde_json::from_str(&json)?;

    // Check if cache is invalidated by config/credentials changes
    if is_cache_invalidated(&cache) {
        tracing::debug!("Cache invalidated due to config/credentials file changes");
        return Ok(None);
    }

    tracing::debug!(
        "Loaded {} profiles from cache ({:?}), cached {}",
        cache.profiles.len(),
        cache_file,
        cache.age_display()
    );

    Ok(Some(cache))
}

/// Clear the profile cache
#[allow(dead_code)]
pub fn clear_cache() -> Result<()> {
    let cache_file = cache_file_path()?;

    if cache_file.exists() {
        fs::remove_file(&cache_file)?;
        tracing::debug!("Cleared profile cache: {:?}", cache_file);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_profile(name: &str) -> CachedProfile {
        CachedProfile {
            profile_name: name.to_string(),
            account_role: AccountRole {
                account_id: "123456789012".to_string(),
                account_name: "TestAccount".to_string(),
                role_name: "TestRole".to_string(),
            },
            session_name: "test-session".to_string(),
            is_default: false,
        }
    }

    #[test]
    fn test_profile_cache_new() {
        let profiles = vec![create_test_profile("test1"), create_test_profile("test2")];
        let cache = ProfileCache::new(profiles.clone());

        assert_eq!(cache.profiles.len(), 2);
        assert!(!cache.is_stale(60)); // Should not be stale immediately
    }

    #[test]
    fn test_profile_cache_staleness() {
        let profiles = vec![create_test_profile("test")];
        let mut cache = ProfileCache::new(profiles);

        // Set cached_at to 2 minutes ago
        cache.cached_at = Utc::now() - chrono::Duration::seconds(120);

        assert!(cache.is_stale(60)); // Stale after 1 minute
        assert!(!cache.is_stale(180)); // Not stale within 3 minutes
    }

    #[test]
    fn test_age_display() {
        let profiles = vec![create_test_profile("test")];
        let mut cache = ProfileCache::new(profiles);

        // Just created
        assert!(cache.age_display().ends_with("s ago"));

        // 5 minutes ago
        cache.cached_at = Utc::now() - chrono::Duration::minutes(5);
        assert!(cache.age_display().contains("m ago"));

        // 2 hours ago
        cache.cached_at = Utc::now() - chrono::Duration::hours(2);
        assert!(cache.age_display().contains("h ago"));

        // 3 days ago
        cache.cached_at = Utc::now() - chrono::Duration::days(3);
        assert!(cache.age_display().contains("d ago"));
    }
}
