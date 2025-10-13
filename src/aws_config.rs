// AWS credentials and config file writer
use crate::error::{Result, SsoError};
use crate::models::{AccountRole, RoleCredentials};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// SSO Session configuration
#[derive(Debug, Clone)]
pub struct SsoSession {
    pub session_name: String,
    pub sso_start_url: String,
    pub sso_region: String,
    pub sso_registration_scopes: String,
}

/// Get the AWS credentials file path
pub fn credentials_file_path() -> Result<PathBuf> {
    if let Some(home) = dirs::home_dir() {
        Ok(home.join(".aws").join("credentials"))
    } else {
        Err(SsoError::ConfigError(
            "Could not determine home directory".to_string(),
        ))
    }
}

/// Get the AWS config file path
pub fn config_file_path() -> Result<PathBuf> {
    if let Some(home) = dirs::home_dir() {
        Ok(home.join(".aws").join("config"))
    } else {
        Err(SsoError::ConfigError(
            "Could not determine home directory".to_string(),
        ))
    }
}

/// Read SSO session from ~/.aws/config
/// Returns the first sso-session found, or None if no session exists
pub fn read_sso_session() -> Result<Option<SsoSession>> {
    let config_path = config_file_path()?;

    if !config_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| SsoError::ConfigError(format!("Failed to read config file: {}", e)))?;

    let mut in_sso_session = false;
    let mut session_name: Option<String> = None;
    let mut session_data: HashMap<String, String> = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Check if previous section was an sso-session
            if in_sso_session {
                // Found a complete sso-session, return it
                if let Some(name) = session_name.take() {
                    if let (Some(start_url), Some(region)) = (
                        session_data.get("sso_start_url"),
                        session_data.get("sso_region"),
                    ) {
                        let scopes = session_data
                            .get("sso_registration_scopes")
                            .cloned()
                            .unwrap_or_else(|| "sso:account:access".to_string());

                        return Ok(Some(SsoSession {
                            session_name: name,
                            sso_start_url: start_url.clone(),
                            sso_region: region.clone(),
                            sso_registration_scopes: scopes,
                        }));
                    }
                }
            }

            // Parse new section header
            let section = &trimmed[1..trimmed.len() - 1];
            if let Some(name) = section.strip_prefix("sso-session ") {
                in_sso_session = true;
                session_name = Some(name.to_string());
                session_data.clear();
            } else {
                in_sso_session = false;
            }
        } else if in_sso_session && !trimmed.is_empty() && !trimmed.starts_with('#') {
            if let Some(eq_pos) = trimmed.find('=') {
                let key = trimmed[..eq_pos].trim().to_string();
                let value = trimmed[eq_pos + 1..].trim().to_string();
                session_data.insert(key, value);
            }
        }
    }

    // Check last section
    if in_sso_session {
        if let Some(name) = session_name {
            if let (Some(start_url), Some(region)) = (
                session_data.get("sso_start_url"),
                session_data.get("sso_region"),
            ) {
                let scopes = session_data
                    .get("sso_registration_scopes")
                    .cloned()
                    .unwrap_or_else(|| "sso:account:access".to_string());

                return Ok(Some(SsoSession {
                    session_name: name,
                    sso_start_url: start_url.clone(),
                    sso_region: region.clone(),
                    sso_registration_scopes: scopes,
                }));
            }
        }
    }

    Ok(None)
}

/// Read all SSO sessions from ~/.aws/config
/// Returns a vector of all sso-sessions found
pub fn read_all_sso_sessions() -> Result<Vec<SsoSession>> {
    let config_path = config_file_path()?;

    if !config_path.exists() {
        tracing::info!("Config file does not exist: {:?}", config_path);
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| SsoError::ConfigError(format!("Failed to read config file: {}", e)))?;

    tracing::info!("Reading config file: {:?}", config_path);
    let mut sessions = Vec::new();
    let mut in_sso_session = false;
    let mut session_name: Option<String> = None;
    let mut session_data: HashMap<String, String> = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Save previous sso-session if complete
            if in_sso_session {
                if let Some(name) = session_name.take() {
                    if let (Some(start_url), Some(region)) = (
                        session_data.get("sso_start_url"),
                        session_data.get("sso_region"),
                    ) {
                        let scopes = session_data
                            .get("sso_registration_scopes")
                            .cloned()
                            .unwrap_or_else(|| "sso:account:access".to_string());

                        let session = SsoSession {
                            session_name: name.clone(),
                            sso_start_url: start_url.clone(),
                            sso_region: region.clone(),
                            sso_registration_scopes: scopes,
                        };
                        tracing::info!(
                            "Adding session: {} ({}, {})",
                            session.session_name,
                            session.sso_start_url,
                            session.sso_region
                        );
                        sessions.push(session);
                    }
                }
                session_data.clear();
            }

            // Check if this is a new sso-session header
            if trimmed.starts_with("[sso-session ") {
                in_sso_session = true;
                let name_part = &trimmed[13..trimmed.len() - 1]; // Extract name between "[sso-session " and "]"
                session_name = Some(name_part.trim().to_string());
                tracing::info!("Found SSO session header: {}", name_part.trim());
            } else {
                in_sso_session = false;
                session_name = None;
            }
            continue;
        }

        if in_sso_session && trimmed.contains('=') {
            let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
            if parts.len() == 2 {
                let key = parts[0].trim().to_string();
                let value = parts[1].trim().to_string();
                session_data.insert(key, value);
            }
        }
    }

    // Handle last session if file ends without a new section
    if in_sso_session {
        if let Some(name) = session_name {
            if let (Some(start_url), Some(region)) = (
                session_data.get("sso_start_url"),
                session_data.get("sso_region"),
            ) {
                let scopes = session_data
                    .get("sso_registration_scopes")
                    .cloned()
                    .unwrap_or_else(|| "sso:account:access".to_string());

                let session = SsoSession {
                    session_name: name.clone(),
                    sso_start_url: start_url.clone(),
                    sso_region: region.clone(),
                    sso_registration_scopes: scopes,
                };
                tracing::info!(
                    "Adding last session: {} ({}, {})",
                    session.session_name,
                    session.sso_start_url,
                    session.sso_region
                );
                sessions.push(session);
            } else {
                tracing::info!(
                    "Incomplete session at end: name={:?}, data={:?}",
                    name,
                    session_data
                );
            }
        }
    }

    tracing::info!("Total sessions found: {}", sessions.len());
    Ok(sessions)
}

/// Default profile configuration
#[derive(Debug, Clone)]
pub struct DefaultConfig {
    pub region: String,
    pub output: String,
}

/// Read [default] section from ~/.aws/config
pub fn read_default_config() -> Result<Option<DefaultConfig>> {
    let config_path = config_file_path()?;

    if !config_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| SsoError::ConfigError(format!("Failed to read config file: {}", e)))?;

    let mut in_default_section = false;
    let mut region: Option<String> = None;
    let mut output: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        // Check for section headers
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_default_section = trimmed == "[default]";
            continue;
        }

        if in_default_section && trimmed.contains('=') {
            let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
            if parts.len() == 2 {
                let key = parts[0].trim();
                let value = parts[1].trim();

                match key {
                    "region" => region = Some(value.to_string()),
                    "output" => output = Some(value.to_string()),
                    _ => {}
                }
            }
        }
    }

    if region.is_some() || output.is_some() {
        Ok(Some(DefaultConfig {
            region: region.unwrap_or_else(|| "us-east-1".to_string()),
            output: output.unwrap_or_else(|| "json".to_string()),
        }))
    } else {
        Ok(None)
    }
}

/// Write [default] section to ~/.aws/config
pub fn write_default_config(config: &DefaultConfig) -> Result<()> {
    let config_path = config_file_path()?;
    let aws_dir = config_path
        .parent()
        .ok_or_else(|| SsoError::ConfigError("Invalid config path".to_string()))?;

    // Create ~/.aws directory if it doesn't exist
    if !aws_dir.exists() {
        fs::create_dir_all(aws_dir).map_err(|e| {
            SsoError::ConfigError(format!("Failed to create ~/.aws directory: {}", e))
        })?;
    }

    let existing_config = if config_path.exists() {
        fs::read_to_string(&config_path)
            .map_err(|e| SsoError::ConfigError(format!("Failed to read config file: {}", e)))?
    } else {
        String::new()
    };

    let new_config = update_ini_section(
        &existing_config,
        "default",
        &[("region", &config.region), ("output", &config.output)],
    );

    fs::write(&config_path, new_config)
        .map_err(|e| SsoError::ConfigError(format!("Failed to write config file: {}", e)))?;

    Ok(())
}

/// Write SSO session to ~/.aws/config
pub fn write_sso_session(session: &SsoSession) -> Result<()> {
    let config_path = config_file_path()?;
    let aws_dir = config_path
        .parent()
        .ok_or_else(|| SsoError::ConfigError("Invalid config path".to_string()))?;

    // Create ~/.aws directory if it doesn't exist
    if !aws_dir.exists() {
        fs::create_dir_all(aws_dir).map_err(|e| {
            SsoError::ConfigError(format!("Failed to create ~/.aws directory: {}", e))
        })?;
    }

    let existing_config = if config_path.exists() {
        fs::read_to_string(&config_path)
            .map_err(|e| SsoError::ConfigError(format!("Failed to read config file: {}", e)))?
    } else {
        String::new()
    };

    let section_name = format!("sso-session {}", session.session_name);
    let new_config = update_ini_section(
        &existing_config,
        &section_name,
        &[
            ("sso_start_url", &session.sso_start_url),
            ("sso_region", &session.sso_region),
            ("sso_registration_scopes", &session.sso_registration_scopes),
        ],
    );

    fs::write(&config_path, new_config)
        .map_err(|e| SsoError::ConfigError(format!("Failed to write config file: {}", e)))?;

    Ok(())
}

/// Write credentials to ~/.aws/credentials and config
pub fn write_credentials(
    profile_name: &str,
    creds: &RoleCredentials,
    region: &str,
    output_format: Option<&str>,
) -> Result<()> {
    write_credentials_with_metadata(profile_name, creds, region, output_format, None)
}

/// Write credentials with optional metadata for tracking account/role
pub fn write_credentials_with_metadata(
    profile_name: &str,
    creds: &RoleCredentials,
    region: &str,
    output_format: Option<&str>,
    account_role: Option<&AccountRole>,
) -> Result<()> {
    let creds_path = credentials_file_path()?;
    let aws_dir = creds_path
        .parent()
        .ok_or_else(|| SsoError::ConfigError("Invalid credentials path".to_string()))?;

    // Create ~/.aws directory if it doesn't exist
    if !aws_dir.exists() {
        fs::create_dir_all(aws_dir).map_err(|e| {
            SsoError::ConfigError(format!("Failed to create ~/.aws directory: {}", e))
        })?;
    }

    // Read existing credentials file
    let existing_content = if creds_path.exists() {
        fs::read_to_string(&creds_path)
            .map_err(|e| SsoError::ConfigError(format!("Failed to read credentials file: {}", e)))?
    } else {
        String::new()
    };

    // Build metadata comments if account_role is provided
    let metadata = if let Some(role) = account_role {
        vec![
            format!("# Account: {}", role.account_id),
            format!("# Role: {}", role.role_name),
            format!("# Valid: {}", creds.expiration.to_rfc3339()),
        ]
    } else {
        vec![]
    };

    let metadata = if !metadata.is_empty() {
        Some(metadata)
    } else {
        None
    };

    // Parse and update credentials
    let new_content = update_ini_section_with_comments(
        &existing_content,
        profile_name,
        &[
            ("aws_access_key_id", &creds.access_key_id),
            ("aws_secret_access_key", &creds.secret_access_key),
            ("aws_session_token", &creds.session_token),
        ],
        metadata.as_deref(),
    );

    // Write updated credentials
    fs::write(&creds_path, new_content)
        .map_err(|e| SsoError::ConfigError(format!("Failed to write credentials file: {}", e)))?;

    // Also write to config file for region
    let config_path = config_file_path()?;
    let existing_config = if config_path.exists() {
        fs::read_to_string(&config_path)
            .map_err(|e| SsoError::ConfigError(format!("Failed to read config file: {}", e)))?
    } else {
        String::new()
    };

    let profile_section = if profile_name == "default" {
        profile_name.to_string()
    } else {
        format!("profile {}", profile_name)
    };

    // Build config entries with owned strings for SSO data
    let mut config_entries_owned: Vec<(String, String)> = vec![];
    let mut config_entries: Vec<(&str, &str)> = vec![("region", region)];

    if let Some(output) = output_format {
        config_entries.push(("output", output));
    }

    // Add SSO session information if account_role is provided
    if let Some(role) = account_role {
        // Try to get the SSO session from config
        if let Ok(Some(session)) = read_sso_session() {
            config_entries_owned.push(("sso_session".to_string(), session.session_name));
            config_entries_owned.push(("sso_account_id".to_string(), role.account_id.clone()));
            config_entries_owned.push(("sso_role_name".to_string(), role.role_name.clone()));
        }
    }

    // Combine both vectors
    let config_entries_refs: Vec<(&str, &str)> = config_entries
        .iter()
        .map(|(k, v)| (*k, *v))
        .chain(
            config_entries_owned
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str())),
        )
        .collect();

    let new_config = update_ini_section(&existing_config, &profile_section, &config_entries_refs);

    fs::write(&config_path, new_config)
        .map_err(|e| SsoError::ConfigError(format!("Failed to write config file: {}", e)))?;

    Ok(())
}

/// Update or add a section in an INI-style file with optional comment metadata
fn update_ini_section_with_comments(
    content: &str,
    section_name: &str,
    key_values: &[(&str, &str)],
    comments: Option<&[String]>,
) -> String {
    let mut result = String::new();
    let mut in_target_section = false;
    let mut section_found = false;
    let mut updated_keys = std::collections::HashSet::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Check if this is a section header
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // If we were in the target section, add any missing keys
            if in_target_section {
                for (key, value) in key_values {
                    if !updated_keys.contains(*key) {
                        result.push_str(&format!("{} = {}\n", key, value));
                    }
                }
                updated_keys.clear();
            }

            let section = &trimmed[1..trimmed.len() - 1];
            in_target_section = section == section_name;
            if in_target_section {
                section_found = true;
                // Skip existing comments after section header (we'll replace them)
                result.push_str(line);
                result.push('\n');
                // Add metadata comments if provided
                if let Some(comment_lines) = comments {
                    for comment in comment_lines {
                        result.push_str(comment);
                        result.push('\n');
                    }
                }
                continue;
            }

            result.push_str(line);
            result.push('\n');
        } else if in_target_section {
            // Skip old comment lines in target section (they'll be replaced)
            if trimmed.starts_with('#') {
                continue;
            }
            // Process non-comment lines
            if !trimmed.is_empty() {
                if let Some(eq_pos) = trimmed.find('=') {
                    let key = trimmed[..eq_pos].trim();
                    if let Some((_, new_value)) = key_values.iter().find(|(k, _)| *k == key) {
                        // Update this key
                        result.push_str(&format!("{} = {}\n", key, new_value));
                        updated_keys.insert(key);
                        continue;
                    }
                }
            }
            result.push_str(line);
            result.push('\n');
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    // If we were in the target section at EOF, add any missing keys
    if in_target_section {
        for (key, value) in key_values {
            if !updated_keys.contains(*key) {
                result.push_str(&format!("{} = {}\n", key, value));
            }
        }
    }

    // If section wasn't found, add it at the end
    if !section_found {
        if !result.is_empty() && !result.ends_with('\n') {
            result.push('\n');
        }
        // Add blank line before new section for readability
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(&format!("[{}]\n", section_name));
        // Add metadata comments if provided
        if let Some(comment_lines) = comments {
            for comment in comment_lines {
                result.push_str(comment);
                result.push('\n');
            }
        }
        for (key, value) in key_values {
            result.push_str(&format!("{} = {}\n", key, value));
        }
    }

    cleanup_empty_lines(&result)
}

/// Update or add a section in an INI-style file
fn update_ini_section(content: &str, section_name: &str, key_values: &[(&str, &str)]) -> String {
    update_ini_section_with_comments(content, section_name, key_values, None)
}

/// Get all profile names from ~/.aws/credentials
pub fn list_profiles() -> Result<Vec<String>> {
    let creds_path = credentials_file_path()?;

    if !creds_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&creds_path)
        .map_err(|e| SsoError::ConfigError(format!("Failed to read credentials file: {}", e)))?;

    let mut profiles = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let profile = &trimmed[1..trimmed.len() - 1];
            profiles.push(profile.to_string());
        }
    }

    Ok(profiles)
}

/// Profile with credential status
#[derive(Debug, Clone)]
pub struct ProfileStatus {
    pub profile_name: String,
    pub account_id: Option<String>,
    pub role_name: Option<String>,
    pub has_credentials: bool,
    pub expiration: Option<DateTime<Utc>>,
}

/// Check if a role has active credentials in AWS config
pub fn get_profile_for_role(account: &AccountRole) -> Result<Option<ProfileStatus>> {
    let creds_path = credentials_file_path()?;

    if !creds_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&creds_path)
        .map_err(|e| SsoError::ConfigError(format!("Failed to read credentials file: {}", e)))?;

    let mut current_profile: Option<String> = None;
    let mut profile_data: HashMap<String, String> = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Check previous profile
            if let Some(profile) = current_profile.take() {
                if let Some(status) = check_profile_match(&profile, &profile_data, account)? {
                    return Ok(Some(status));
                }
                profile_data.clear();
            }

            current_profile = Some(trimmed[1..trimmed.len() - 1].to_string());
        } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
            if let Some(eq_pos) = trimmed.find('=') {
                let key = trimmed[..eq_pos].trim().to_string();
                let value = trimmed[eq_pos + 1..].trim().to_string();
                profile_data.insert(key, value);
            }
        }
    }

    // Check last profile
    if let Some(profile) = current_profile {
        if let Some(status) = check_profile_match(&profile, &profile_data, account)? {
            return Ok(Some(status));
        }
    }

    Ok(None)
}

fn check_profile_match(
    profile_name: &str,
    data: &HashMap<String, String>,
    _account: &AccountRole,
) -> Result<Option<ProfileStatus>> {
    // Check if this profile has credentials
    let has_key = data.contains_key("aws_access_key_id");
    let has_secret = data.contains_key("aws_secret_access_key");
    let has_session = data.contains_key("aws_session_token");

    if !has_key || !has_secret || !has_session {
        return Ok(None);
    }

    // For now, we can't definitively match without storing metadata
    // We'll consider any profile with credentials as potentially active
    // A better approach would be to add comments or use AWS SSO cache structure

    Ok(Some(ProfileStatus {
        profile_name: profile_name.to_string(),
        account_id: None,
        role_name: None,
        has_credentials: true,
        expiration: None,
    }))
}

/// Get the existing profile name for an account/role combination
/// Returns the profile name if found, based on matching account ID and role name in comments
pub fn get_existing_profile_name(account: &AccountRole) -> Result<Option<String>> {
    let creds_path = credentials_file_path()?;

    if !creds_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&creds_path)
        .map_err(|e| SsoError::ConfigError(format!("Failed to read credentials file: {}", e)))?;

    let mut current_profile: Option<String> = None;
    let mut found_account_id = false;
    let mut found_role_name = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Check if previous profile matched
            if current_profile.is_some() && found_account_id && found_role_name {
                return Ok(current_profile);
            }

            // Start new profile
            current_profile = Some(trimmed[1..trimmed.len() - 1].to_string());
            found_account_id = false;
            found_role_name = false;
        } else if current_profile.is_some() {
            // Check for metadata comments
            if trimmed.starts_with('#') {
                if trimmed.contains(&format!("Account: {}", account.account_id)) {
                    found_account_id = true;
                }
                if trimmed.contains(&format!("Role: {}", account.role_name)) {
                    found_role_name = true;
                }
            }
        }
    }

    // Check last profile
    if current_profile.is_some() && found_account_id && found_role_name {
        return Ok(current_profile);
    }

    Ok(None)
}

/// Rename a profile in AWS credentials and config files
pub fn rename_profile(old_name: &str, new_name: &str) -> Result<()> {
    // Rename in credentials file
    let creds_path = credentials_file_path()?;
    if creds_path.exists() {
        let content = fs::read_to_string(&creds_path).map_err(|e| {
            SsoError::ConfigError(format!("Failed to read credentials file: {}", e))
        })?;
        let new_content = rename_ini_section(&content, old_name, new_name);
        fs::write(&creds_path, new_content).map_err(|e| {
            SsoError::ConfigError(format!("Failed to write credentials file: {}", e))
        })?;
    }

    // Rename in config file
    let config_path = config_file_path()?;
    if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .map_err(|e| SsoError::ConfigError(format!("Failed to read config file: {}", e)))?;

        let old_section = if old_name == "default" {
            old_name.to_string()
        } else {
            format!("profile {}", old_name)
        };

        let new_section = if new_name == "default" {
            new_name.to_string()
        } else {
            format!("profile {}", new_name)
        };

        let new_content = rename_ini_section(&content, &old_section, &new_section);
        fs::write(&config_path, new_content)
            .map_err(|e| SsoError::ConfigError(format!("Failed to write config file: {}", e)))?;
    }

    Ok(())
}

/// Rename a section in an INI-style file
fn rename_ini_section(content: &str, old_name: &str, new_name: &str) -> String {
    let mut result = String::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section = &trimmed[1..trimmed.len() - 1];
            if section == old_name {
                result.push_str(&format!("[{}]\n", new_name));
                continue;
            }
        }

        result.push_str(line);
        result.push('\n');
    }

    cleanup_empty_lines(&result)
}

/// Invalidate a profile's credentials without deleting the profile structure
/// This preserves profile names and allows reactivation without losing configuration
pub fn invalidate_profile(profile_name: &str) -> Result<()> {
    let creds_path = credentials_file_path()?;

    if !creds_path.exists() {
        return Ok(()); // Nothing to invalidate
    }

    let content = fs::read_to_string(&creds_path)
        .map_err(|e| SsoError::ConfigError(format!("Failed to read credentials file: {}", e)))?;

    // Replace credentials with dummy values and mark as invalid
    let dummy_key = "INVALID_KEY";
    let dummy_secret = "INVALID_SECRET";
    let dummy_token = "INVALID_TOKEN";

    let metadata = Some(vec![
        format!("# Valid: false"),
        format!("# Invalidated: {}", Utc::now().to_rfc3339()),
    ]);

    let new_content = update_ini_section_with_comments(
        &content,
        profile_name,
        &[
            ("aws_access_key_id", dummy_key),
            ("aws_secret_access_key", dummy_secret),
            ("aws_session_token", dummy_token),
        ],
        metadata.as_deref(),
    );

    fs::write(&creds_path, new_content)
        .map_err(|e| SsoError::ConfigError(format!("Failed to write credentials file: {}", e)))?;

    Ok(())
}

/// Delete a profile from AWS credentials and config files
/// NOTE: Consider using invalidate_profile() instead to preserve profile names
pub fn delete_profile(profile_name: &str) -> Result<()> {
    // Delete from credentials file
    let creds_path = credentials_file_path()?;
    if creds_path.exists() {
        let content = fs::read_to_string(&creds_path).map_err(|e| {
            SsoError::ConfigError(format!("Failed to read credentials file: {}", e))
        })?;
        let new_content = delete_ini_section(&content, profile_name);
        fs::write(&creds_path, new_content).map_err(|e| {
            SsoError::ConfigError(format!("Failed to write credentials file: {}", e))
        })?;
    }

    // Delete from config file
    let config_path = config_file_path()?;
    if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .map_err(|e| SsoError::ConfigError(format!("Failed to read config file: {}", e)))?;

        let section_name = if profile_name == "default" {
            profile_name.to_string()
        } else {
            format!("profile {}", profile_name)
        };

        let new_content = delete_ini_section(&content, &section_name);
        fs::write(&config_path, new_content)
            .map_err(|e| SsoError::ConfigError(format!("Failed to write config file: {}", e)))?;
    }

    Ok(())
}

/// Delete a section from an INI-style file
fn delete_ini_section(content: &str, section_name: &str) -> String {
    let mut result = String::new();
    let mut in_target_section = false;
    let mut skip_blank_line = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section = &trimmed[1..trimmed.len() - 1];
            if section == section_name {
                in_target_section = true;
                skip_blank_line = true;
                continue;
            } else {
                in_target_section = false;
                skip_blank_line = false;
            }
        }

        if !in_target_section {
            // Skip one blank line after deleted section
            if skip_blank_line && trimmed.is_empty() {
                skip_blank_line = false;
                continue;
            }
            result.push_str(line);
            result.push('\n');
        }
    }

    cleanup_empty_lines(&result)
}

/// Clean up empty lines in INI files:
/// - Remove leading empty lines
/// - Ensure exactly one blank line between sections
/// - Remove trailing empty lines
fn cleanup_empty_lines(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = String::new();
    let mut previous_blank = false;
    let mut at_start = true;

    for line in lines.iter() {
        let trimmed = line.trim();
        let is_blank = trimmed.is_empty();

        // Skip leading blank lines
        if at_start && is_blank {
            continue;
        }

        // If we encounter non-blank content, we're no longer at start
        if !is_blank {
            at_start = false;
        }

        // Skip consecutive blank lines (keep only one)
        if is_blank && previous_blank {
            continue;
        }

        result.push_str(line);
        result.push('\n');
        previous_blank = is_blank;
    }

    // Remove trailing blank lines
    while result.ends_with("\n\n") {
        result.pop();
    }

    result
}

/// Get all profiles with their status
pub fn list_profile_statuses() -> Result<Vec<ProfileStatus>> {
    let creds_path = credentials_file_path()?;

    if !creds_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&creds_path)
        .map_err(|e| SsoError::ConfigError(format!("Failed to read credentials file: {}", e)))?;

    let mut profiles = Vec::new();
    let mut current_profile: Option<String> = None;
    let mut profile_data: HashMap<String, String> = HashMap::new();
    let mut account_id: Option<String> = None;
    let mut role_name: Option<String> = None;
    let mut expiration: Option<DateTime<Utc>> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Save previous profile
            if let Some(profile) = current_profile.take() {
                let has_creds = profile_data.contains_key("aws_access_key_id")
                    && profile_data.contains_key("aws_secret_access_key")
                    && profile_data.contains_key("aws_session_token");

                profiles.push(ProfileStatus {
                    profile_name: profile,
                    account_id: account_id.take(),
                    role_name: role_name.take(),
                    has_credentials: has_creds,
                    expiration: expiration.take(),
                });
                profile_data.clear();
            }

            current_profile = Some(trimmed[1..trimmed.len() - 1].to_string());
        } else if trimmed.starts_with('#') {
            // Parse metadata comments
            if let Some(rest) = trimmed.strip_prefix("# Account:") {
                account_id = Some(rest.trim().to_string());
            } else if let Some(rest) = trimmed.strip_prefix("# Role:") {
                role_name = Some(rest.trim().to_string());
            } else if let Some(rest) = trimmed.strip_prefix("# Valid:") {
                let value = rest.trim();
                if value == "false" {
                    // Profile is invalidated, no expiration
                    expiration = None;
                } else {
                    // Parse ISO 8601 timestamp (expiration date)
                    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
                        expiration = Some(dt.with_timezone(&Utc));
                    }
                }
            } else if let Some(rest) = trimmed.strip_prefix("# Expiration:") {
                // Backward compatibility: parse old format
                if let Ok(dt) = DateTime::parse_from_rfc3339(rest.trim()) {
                    expiration = Some(dt.with_timezone(&Utc));
                }
            }
        } else if !trimmed.is_empty() {
            if let Some(eq_pos) = trimmed.find('=') {
                let key = trimmed[..eq_pos].trim().to_string();
                let value = trimmed[eq_pos + 1..].trim().to_string();
                profile_data.insert(key, value);
            }
        }
    }

    // Save last profile
    if let Some(profile) = current_profile {
        let has_creds = profile_data.contains_key("aws_access_key_id")
            && profile_data.contains_key("aws_secret_access_key")
            && profile_data.contains_key("aws_session_token");

        profiles.push(ProfileStatus {
            profile_name: profile,
            account_id,
            role_name,
            has_credentials: has_creds,
            expiration,
        });
    }

    Ok(profiles)
}
