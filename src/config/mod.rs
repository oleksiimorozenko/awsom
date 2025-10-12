// Configuration management
use crate::error::{Result, SsoError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub sso: SsoConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub profile_defaults: ProfileDefaults,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SsoConfig {
    pub start_url: Option<String>,
    pub region: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval: u64,
}

fn default_refresh_interval() -> u64 {
    1
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            refresh_interval: default_refresh_interval(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileDefaults {
    pub region: Option<String>,
    pub output: Option<String>,
}

impl Config {
    /// Get the config directory path
    ///
    /// Priority:
    /// 1. XDG_CONFIG_HOME/awsom (if env var is set)
    /// 2. ~/.config/awsom (if ~/.config exists)
    /// 3. ~/.awsom (fallback on Unix, doesn't create ~/.config)
    /// 4. Platform default on Windows
    pub fn config_dir() -> Result<PathBuf> {
        // First, check XDG_CONFIG_HOME environment variable (explicit opt-in)
        if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            return Ok(PathBuf::from(xdg_config).join("awsom"));
        }

        // On Unix-like systems (Linux, macOS), detect existing structure
        #[cfg(unix)]
        {
            if let Some(home_dir) = dirs::home_dir() {
                let xdg_config = home_dir.join(".config");

                // If ~/.config exists, use it (user has adopted XDG)
                if xdg_config.exists() {
                    return Ok(xdg_config.join("awsom"));
                }

                // Otherwise, use ~/.awsom (don't create ~/.config for users)
                return Ok(home_dir.join(".awsom"));
            }
        }

        // Fall back to platform-specific default for Windows
        #[cfg(not(unix))]
        {
            if let Some(config_dir) = dirs::config_dir() {
                return Ok(config_dir.join("awsom"));
            }
        }

        Err(SsoError::ConfigError(
            "Could not determine config directory".to_string(),
        ))
    }

    /// Get the config file path
    pub fn config_file_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    /// Load configuration from file, environment variables, and defaults
    pub fn load() -> Result<Self> {
        let config_path = Self::config_file_path()?;

        let mut config = if config_path.exists() {
            tracing::debug!("Loading config from: {}", config_path.display());
            let contents = fs::read_to_string(&config_path)
                .map_err(|e| SsoError::ConfigError(format!("Failed to read config file: {}", e)))?;

            toml::from_str(&contents)
                .map_err(|e| SsoError::ConfigError(format!("Failed to parse config file: {}", e)))?
        } else {
            tracing::debug!(
                "Config file not found at {}, using defaults",
                config_path.display()
            );
            Config::default()
        };

        // Override with environment variables if set
        if let Ok(start_url) = std::env::var("AWS_SSO_START_URL") {
            tracing::debug!("Using AWS_SSO_START_URL from environment: {}", start_url);
            config.sso.start_url = Some(start_url);
        }

        if let Ok(region) = std::env::var("AWS_SSO_REGION") {
            tracing::debug!("Using AWS_SSO_REGION from environment: {}", region);
            config.sso.region = Some(region);
        }

        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_dir = Self::config_dir()?;
        let config_path = Self::config_file_path()?;

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).map_err(|e| {
                SsoError::ConfigError(format!("Failed to create config directory: {}", e))
            })?;
            tracing::info!("Created config directory: {}", config_dir.display());
        }

        // Serialize to TOML
        let toml_string = toml::to_string_pretty(self)
            .map_err(|e| SsoError::ConfigError(format!("Failed to serialize config: {}", e)))?;

        // Write to file
        fs::write(&config_path, toml_string)
            .map_err(|e| SsoError::ConfigError(format!("Failed to write config file: {}", e)))?;

        tracing::info!("Saved config to: {}", config_path.display());
        Ok(())
    }

    /// Create a sample config file with comments
    pub fn create_sample() -> Result<()> {
        let config_dir = Self::config_dir()?;
        let config_path = Self::config_file_path()?;

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).map_err(|e| {
                SsoError::ConfigError(format!("Failed to create config directory: {}", e))
            })?;
        }

        // Don't overwrite existing config
        if config_path.exists() {
            return Err(SsoError::ConfigError(format!(
                "Config file already exists at: {}",
                config_path.display()
            )));
        }

        let sample_config = r#"# AWS SSO TUI Configuration
# Location priority:
#   1. $XDG_CONFIG_HOME/awsom/config.toml (if XDG_CONFIG_HOME is set)
#   2. ~/.config/awsom/config.toml (if ~/.config exists)
#   3. ~/.awsom/config.toml (fallback)
#
# You can also set these values via environment variables:
#   AWS_SSO_START_URL
#   AWS_SSO_REGION

[sso]
# Your AWS SSO start URL (required)
# Example: start_url = "https://my-org.awsapps.com/start"
start_url = ""

# AWS region for SSO (required)
# Example: region = "us-east-1"
region = ""

[ui]
# Refresh interval for TUI in minutes (default: 1)
refresh_interval = 1

[profile_defaults]
# Default AWS region for profiles written to ~/.aws/config
# If not set, uses the SSO region
# Example: region = "us-west-2"
region = ""

# Default output format for AWS CLI
# Valid values: json, yaml, yaml-stream, text, table
# Example: output = "json"
output = ""
"#;

        fs::write(&config_path, sample_config)
            .map_err(|e| SsoError::ConfigError(format!("Failed to write sample config: {}", e)))?;

        println!("Created sample config file at: {}", config_path.display());
        println!("\nPlease edit the file and set your AWS SSO details:");
        println!("  start_url = \"https://your-org.awsapps.com/start\"");
        println!("  region = \"us-east-1\"");

        Ok(())
    }

    /// Check if config is complete (has required fields)
    pub fn is_complete(&self) -> bool {
        self.sso.start_url.is_some() && self.sso.region.is_some()
    }

    /// Get SSO configuration, returning an error if incomplete
    pub fn get_sso_config(&self) -> Result<(&str, &str)> {
        let start_url = self.sso.start_url.as_deref()
            .ok_or_else(|| SsoError::ConfigError(
                "SSO start_url not configured. Set it in config file or AWS_SSO_START_URL environment variable".to_string()
            ))?;

        let region = self.sso.region.as_deref()
            .ok_or_else(|| SsoError::ConfigError(
                "SSO region not configured. Set it in config file or AWS_SSO_REGION environment variable".to_string()
            ))?;

        Ok((start_url, region))
    }
}
