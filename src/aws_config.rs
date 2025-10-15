// AWS credentials and config file writer
use crate::error::{Result, SsoError};
use crate::models::{AccountRole, RoleCredentials};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Check if awsom has been initialized (backups created)
fn is_initialized() -> Result<bool> {
    if let Some(home) = dirs::home_dir() {
        let marker_file = home.join(".aws").join(".awsom-initialized");
        Ok(marker_file.exists())
    } else {
        Err(SsoError::ConfigError(
            "Could not determine home directory".to_string(),
        ))
    }
}

/// Create backups of config and credentials files on first use
fn create_backups_if_needed() -> Result<()> {
    // Check if already initialized
    if is_initialized()? {
        return Ok(());
    }

    let home = dirs::home_dir()
        .ok_or_else(|| SsoError::ConfigError("Could not determine home directory".to_string()))?;

    let aws_dir = home.join(".aws");
    let config_path = aws_dir.join("config");
    let credentials_path = aws_dir.join("credentials");
    let marker_file = aws_dir.join(".awsom-initialized");

    // Create ~/.aws directory if it doesn't exist
    if !aws_dir.exists() {
        fs::create_dir_all(&aws_dir).map_err(|e| {
            SsoError::ConfigError(format!("Failed to create ~/.aws directory: {}", e))
        })?;
    }

    // Backup config file if it exists, then add header comment
    if config_path.exists() {
        let backup_path = aws_dir.join("config-before-awsom.bak");
        fs::copy(&config_path, &backup_path)
            .map_err(|e| SsoError::ConfigError(format!("Failed to backup config file: {}", e)))?;
        tracing::info!("Created backup: {:?}", backup_path);

        // Add header comment to config file
        add_header_comment(&config_path, "config-before-awsom.bak")?;
    }

    // Backup credentials file if it exists, then add header comment
    if credentials_path.exists() {
        let backup_path = aws_dir.join("credentials-before-awsom.bak");
        fs::copy(&credentials_path, &backup_path).map_err(|e| {
            SsoError::ConfigError(format!("Failed to backup credentials file: {}", e))
        })?;
        tracing::info!("Created backup: {:?}", backup_path);

        // Add header comment to credentials file
        add_header_comment(&credentials_path, "credentials-before-awsom.bak")?;
    }

    // Create marker file
    fs::write(&marker_file, "")
        .map_err(|e| SsoError::ConfigError(format!("Failed to create marker file: {}", e)))?;

    tracing::info!("awsom initialization complete - backups created");
    Ok(())
}

/// Add header comment to a file explaining it's managed by awsom
fn add_header_comment(file_path: &std::path::Path, backup_filename: &str) -> Result<()> {
    let content = fs::read_to_string(file_path)
        .map_err(|e| SsoError::ConfigError(format!("Failed to read file: {}", e)))?;

    // Check if header comment already exists
    if content.starts_with("# This file is managed by awsom") {
        return Ok(());
    }

    // Prepare header comment
    let header = format!(
        "# This file is managed by awsom (AWS Organization Manager)\n\
         # Original backup: {} (created on first run)\n\
         # For more information: https://github.com/oleksiimorozenko/awsom\n\
         \n",
        backup_filename
    );

    // Prepend header to existing content
    let new_content = format!("{}{}", header, content);

    // Write updated content
    fs::write(file_path, new_content)
        .map_err(|e| SsoError::ConfigError(format!("Failed to write file: {}", e)))?;

    tracing::info!("Added header comment to {:?}", file_path);
    Ok(())
}

// Section marker constants (public for import command)
pub const USER_MANAGED_MARKER: &str =
    "# ==================== User-managed sections ====================";
pub const USER_MANAGED_COMMENT: &str = "# (sections below this line are not modified by awsom)";
pub const AWSOM_MANAGED_MARKER: &str =
    "# ==================== Managed by awsom ====================";
pub const AWSOM_MANAGED_COMMENT: &str =
    "# (sections below this line are automatically managed by awsom)";

/// Check if a line is any marker (user-managed or awsom-managed)
fn is_marker_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed == AWSOM_MANAGED_MARKER
        || trimmed == AWSOM_MANAGED_COMMENT
        || trimmed == USER_MANAGED_MARKER
        || trimmed == USER_MANAGED_COMMENT
}

/// Check if a line is the awsom managed marker (starts the awsom section)
fn is_awsom_managed_marker(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed == AWSOM_MANAGED_MARKER || trimmed == AWSOM_MANAGED_COMMENT
}

/// Ensure config file has the management markers (public for import command)
/// Adds markers if they don't exist, placing existing content below the user-managed marker
pub fn ensure_markers(content: &str) -> String {
    // Check if marker already exists
    for line in content.lines() {
        if is_marker_line(line) {
            // Markers already exist
            return content.to_string();
        }
    }

    // No marker found - need to add markers
    // Separate header comments from actual content
    let mut header = String::new();
    let mut user_content = String::new();
    let mut in_header = true;

    for line in content.lines() {
        let trimmed = line.trim();

        // Header is any leading comment lines before first section
        if in_header {
            if trimmed.starts_with('#') || trimmed.is_empty() {
                header.push_str(line);
                header.push('\n');
            } else {
                // Found non-comment, non-empty content - header is done
                in_header = false;
                user_content.push_str(line);
                user_content.push('\n');
            }
        } else {
            user_content.push_str(line);
            user_content.push('\n');
        }
    }

    // Build result: header + user marker + user content + awsom marker
    let mut result = header;

    // Add blank line after header if there's content
    if !result.trim().is_empty() && !user_content.trim().is_empty() {
        result.push('\n');
    }

    // Add user-managed marker
    result.push_str(USER_MANAGED_MARKER);
    result.push('\n');
    result.push_str(USER_MANAGED_COMMENT);
    result.push('\n');

    // Add user content below the marker
    if !user_content.trim().is_empty() {
        result.push('\n');
        result.push_str(&user_content);
        // Ensure content ends with newline
        if !result.ends_with('\n') {
            result.push('\n');
        }
    }

    // Add blank line before awsom marker
    result.push('\n');

    // Add awsom-managed marker
    result.push_str(AWSOM_MANAGED_MARKER);
    result.push('\n');
    result.push_str(AWSOM_MANAGED_COMMENT);
    result.push('\n');

    result
}

/// Reconstruct config file with proper header, markers, and sections
fn reconstruct_config(header: &str, user_section: &str, awsom_section: &str) -> String {
    let mut result = String::new();

    // Add header if present
    if !header.trim().is_empty() {
        result.push_str(header);
        // Ensure blank line after header
        if !result.ends_with("\n\n") && !result.ends_with('\n') {
            result.push('\n');
        }
        if !result.ends_with("\n\n") {
            result.push('\n');
        }
    }

    // Add user-managed marker
    result.push_str(USER_MANAGED_MARKER);
    result.push('\n');
    result.push_str(USER_MANAGED_COMMENT);
    result.push('\n');

    // Add user section if present
    if !user_section.trim().is_empty() {
        result.push('\n');
        result.push_str(user_section);
        // Ensure it ends with newline
        if !result.ends_with('\n') {
            result.push('\n');
        }
    }

    // Add blank line before awsom marker
    result.push('\n');

    // Add awsom-managed marker
    result.push_str(AWSOM_MANAGED_MARKER);
    result.push('\n');
    result.push_str(AWSOM_MANAGED_COMMENT);
    result.push('\n');

    // Add awsom section if present
    if !awsom_section.trim().is_empty() {
        result.push('\n');
        result.push_str(awsom_section);
    }

    result
}

/// Split config content into header, user-managed and awsom-managed sections
/// Returns (header_section, user_section, awsom_section) tuple
/// Header is any leading comments before first section marker or profile
fn split_into_sections(content: &str) -> (String, String, String) {
    let mut header = String::new();
    let mut user_section = String::new();
    let mut awsom_section = String::new();
    let mut in_awsom_section = false;
    let mut in_header = true;
    let mut found_marker = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip all marker lines - they'll be re-added during reconstruction
        if is_marker_line(line) {
            if is_awsom_managed_marker(line) {
                in_awsom_section = true;
                found_marker = true;
            }
            in_header = false; // No longer in header after seeing a marker
            continue; // Skip all marker lines
        }

        // Collect header (leading comments before any section or marker)
        if in_header {
            if trimmed.starts_with('#') || trimmed.is_empty() {
                header.push_str(line);
                header.push('\n');
                continue;
            } else {
                // Found non-comment content, header is done
                in_header = false;
            }
        }

        // Collect content
        if in_awsom_section {
            awsom_section.push_str(line);
            awsom_section.push('\n');
        } else {
            user_section.push_str(line);
            user_section.push('\n');
        }
    }

    // If no marker was found, treat all non-header content as user-managed
    if !found_marker {
        (header, content.trim_start().to_string(), String::new())
    } else {
        (header, user_section, awsom_section)
    }
}

/// Split config content into user-managed and awsom-managed sections (public for import command)
/// Returns (user_section, awsom_section) tuple
/// This is a compatibility wrapper that combines header with user_section
pub fn split_by_marker(content: &str) -> (String, String) {
    let (header, user_section, awsom_section) = split_into_sections(content);

    // Combine header with user section for backward compatibility
    let mut combined_user = header;
    if !combined_user.is_empty() && !user_section.is_empty() {
        combined_user.push('\n');
    }
    combined_user.push_str(&user_section);

    (combined_user, awsom_section)
}

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

/// Resolve SSO session configuration from multiple sources
///
/// Priority order:
/// 1. Explicit flags (--start-url + --region) - highest priority, for scripting
/// 2. Session name (--session-name) - look up from config
/// 3. Active SSO token (if only one exists) - check cache (TODO: implement)
/// 4. Single configured session (if only one exists) - check config
///
/// Returns (start_url, region) tuple or error with helpful message
pub fn resolve_sso_session(
    session_name: Option<&str>,
    start_url: Option<&str>,
    region: Option<&str>,
) -> Result<(String, String)> {
    // Level 1: Explicit flags (both start_url and region must be provided)
    if let (Some(url), Some(reg)) = (start_url, region) {
        tracing::debug!(
            "Resolved SSO session from explicit flags: start_url={}, region={}",
            url,
            reg
        );
        return Ok((url.to_string(), reg.to_string()));
    }

    // If only one flag is provided, that's an error
    if start_url.is_some() || region.is_some() {
        return Err(SsoError::ConfigError(
            "Both --start-url and --region must be provided when using explicit flags".to_string(),
        ));
    }

    // Level 2: Session name - look up from config
    if let Some(name) = session_name {
        let sessions = read_all_sso_sessions()?;
        if let Some(session) = sessions.iter().find(|s| s.session_name == name) {
            tracing::debug!(
                "Resolved SSO session from session name '{}': start_url={}, region={}",
                name,
                session.sso_start_url,
                session.sso_region
            );
            return Ok((session.sso_start_url.clone(), session.sso_region.clone()));
        } else {
            return Err(SsoError::ConfigError(format!(
                "Session '{}' not found in ~/.aws/config",
                name
            )));
        }
    }

    // Level 3: Active SSO token (if only one exists)
    // TODO: Implement token cache checking
    // This would check ~/.aws/sso/cache/ for active tokens and if exactly one is found,
    // map it back to its session configuration
    // For now, skip to level 4

    // Level 4: Single configured session
    let sessions = read_all_sso_sessions()?;
    match sessions.len() {
        0 => Err(SsoError::ConfigError(
            "No SSO sessions configured. Add one with 'awsom session add' or provide --start-url and --region".to_string()
        )),
        1 => {
            let session = &sessions[0];
            tracing::debug!(
                "Resolved SSO session from single configured session '{}': start_url={}, region={}",
                session.session_name,
                session.sso_start_url,
                session.sso_region
            );
            Ok((session.sso_start_url.clone(), session.sso_region.clone()))
        }
        _ => {
            let session_list = sessions
                .iter()
                .map(|s| format!("  - {} ({})", s.session_name, s.sso_start_url))
                .collect::<Vec<_>>()
                .join("\n");
            Err(SsoError::ConfigError(format!(
                "Multiple SSO sessions configured. Specify one with --session-name:\n\n{}\n\nExample:\n  awsom exec --session-name {} --role-name <role> --account-name <account> -- <command>",
                session_list,
                sessions[0].session_name
            )))
        }
    }
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

/// Read awsom defaults from [profile awsom-defaults] section
pub fn read_awsom_defaults() -> Result<Option<DefaultConfig>> {
    let config_path = config_file_path()?;

    if !config_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| SsoError::ConfigError(format!("Failed to read config file: {}", e)))?;

    let mut in_awsom_defaults = false;
    let mut region: Option<String> = None;
    let mut output: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        // Check for section headers
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_awsom_defaults = trimmed == "[profile awsom-defaults]";
            continue;
        }

        if in_awsom_defaults && trimmed.contains('=') {
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

/// Write awsom defaults to [profile awsom-defaults] in awsom-managed section
pub fn write_awsom_defaults(config: &DefaultConfig) -> Result<()> {
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

    // Create backups on first write
    create_backups_if_needed()?;

    let existing_config = if config_path.exists() {
        fs::read_to_string(&config_path)
            .map_err(|e| SsoError::ConfigError(format!("Failed to read config file: {}", e)))?
    } else {
        String::new()
    };

    // Ensure markers exist in the config
    let config_with_markers = ensure_markers(&existing_config);

    // Split into header, user-managed and awsom-managed sections
    let (header, user_section, awsom_section) = split_into_sections(&config_with_markers);

    // Parse existing content from awsom section
    let sessions = parse_sso_sessions_from_content(&awsom_section);
    let (default_config_opt, mut profiles) = parse_profiles_from_content(&awsom_section);

    // Remove existing awsom-defaults profile if it exists
    profiles.retain(|(name, _)| name != "profile awsom-defaults");

    // Add awsom-defaults profile
    profiles.push((
        "profile awsom-defaults".to_string(),
        vec![
            ("region".to_string(), config.region.clone()),
            ("output".to_string(), config.output.clone()),
        ],
    ));

    // Sort profiles alphabetically by name
    profiles.sort_by(|a, b| a.0.cmp(&b.0));

    // Build the awsom-managed section
    let mut new_awsom_section = String::new();

    // Add [default] section if it exists
    if let Some(default_config) = default_config_opt {
        new_awsom_section.push_str("[default]\n");
        for (key, value) in default_config {
            new_awsom_section.push_str(&format!("{} = {}\n", key, value));
        }
        new_awsom_section.push('\n');
    }

    // Add sorted SSO sessions
    new_awsom_section.push_str(&rebuild_sso_sessions(&sessions));

    // Add sorted profiles (including awsom-defaults)
    for (profile_name, entries) in profiles {
        if profile_name != "default" {
            new_awsom_section.push_str(&format!("[{}]\n", profile_name));
            for (key, value) in entries {
                new_awsom_section.push_str(&format!("{} = {}\n", key, value));
            }
            new_awsom_section.push('\n');
        }
    }

    // Reconstruct the file using helper
    let result = reconstruct_config(&header, &user_section, &new_awsom_section);

    fs::write(&config_path, cleanup_empty_lines(&result))
        .map_err(|e| SsoError::ConfigError(format!("Failed to write config file: {}", e)))?;

    Ok(())
}

/// Check if a profile is in the awsom-managed section
pub fn is_profile_in_awsom_section(profile_name: &str) -> Result<bool> {
    let config_path = config_file_path()?;

    if !config_path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| SsoError::ConfigError(format!("Failed to read config file: {}", e)))?;

    // Ensure markers exist
    let content_with_markers = ensure_markers(&content);

    // Split into user-managed and awsom-managed sections
    let (_, awsom_section) = split_by_marker(&content_with_markers);

    // Check if profile exists in awsom section
    let profile_section = if profile_name == "default" {
        "[default]".to_string()
    } else {
        format!("[profile {}]", profile_name)
    };

    Ok(awsom_section.contains(&profile_section))
}

/// Get profile details for display (region, output, SSO info if available)
#[derive(Debug, Clone)]
pub struct ProfileDetails {
    pub region: Option<String>,
    pub output: Option<String>,
    pub sso_session: Option<String>,
    pub sso_account_id: Option<String>,
    pub sso_role_name: Option<String>,
}

pub fn get_profile_details(profile_name: &str) -> Result<Option<ProfileDetails>> {
    let config_path = config_file_path()?;

    if !config_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| SsoError::ConfigError(format!("Failed to read config file: {}", e)))?;

    let mut current_profile: Option<String> = None;
    let mut profile_data: HashMap<String, String> = HashMap::new();

    let target_section = if profile_name == "default" {
        "default".to_string()
    } else {
        format!("profile {}", profile_name)
    };

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Check if we found our target profile
            if let Some(found_profile) = current_profile.take() {
                if found_profile == target_section {
                    // Return the profile details
                    return Ok(Some(ProfileDetails {
                        region: profile_data.get("region").cloned(),
                        output: profile_data.get("output").cloned(),
                        sso_session: profile_data.get("sso_session").cloned(),
                        sso_account_id: profile_data.get("sso_account_id").cloned(),
                        sso_role_name: profile_data.get("sso_role_name").cloned(),
                    }));
                }
                profile_data.clear();
            }

            // Parse section header
            let section = &trimmed[1..trimmed.len() - 1];
            if section == "default" {
                current_profile = Some("default".to_string());
            } else if section.starts_with("profile ") {
                current_profile = Some(section.to_string());
            }
        } else if current_profile.is_some() && trimmed.contains('=') && !trimmed.starts_with('#') {
            let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
            if parts.len() == 2 {
                let key = parts[0].trim().to_string();
                let value = parts[1].trim().to_string();
                profile_data.insert(key, value);
            }
        }
    }

    // Check last profile
    if let Some(found_profile) = current_profile {
        if found_profile == target_section {
            return Ok(Some(ProfileDetails {
                region: profile_data.get("region").cloned(),
                output: profile_data.get("output").cloned(),
                sso_session: profile_data.get("sso_session").cloned(),
                sso_account_id: profile_data.get("sso_account_id").cloned(),
                sso_role_name: profile_data.get("sso_role_name").cloned(),
            }));
        }
    }

    Ok(None)
}

/// Write [default] section to ~/.aws/config with marker-based organization
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

    // Create backups on first write
    create_backups_if_needed()?;

    let existing_config = if config_path.exists() {
        fs::read_to_string(&config_path)
            .map_err(|e| SsoError::ConfigError(format!("Failed to read config file: {}", e)))?
    } else {
        String::new()
    };

    // Ensure markers exist in the config
    let config_with_markers = ensure_markers(&existing_config);

    // Split into user-managed and awsom-managed sections
    let (header, user_section, awsom_section) = split_into_sections(&config_with_markers);

    // Parse existing sessions and default config from awsom section
    let sessions = parse_sso_sessions_from_content(&awsom_section);

    // Build the awsom-managed section: [default] first, then sorted SSO sessions
    let mut new_awsom_section = String::new();

    // Add [default] section
    new_awsom_section.push_str("[default]\n");
    new_awsom_section.push_str(&format!("region = {}\n", config.region));
    new_awsom_section.push_str(&format!("output = {}\n", config.output));
    new_awsom_section.push('\n');

    // Add sorted SSO sessions
    new_awsom_section.push_str(&rebuild_sso_sessions(&sessions));

    // Reconstruct the file using helper
    let result = reconstruct_config(&header, &user_section, &new_awsom_section);

    fs::write(&config_path, cleanup_empty_lines(&result))
        .map_err(|e| SsoError::ConfigError(format!("Failed to write config file: {}", e)))?;

    Ok(())
}

/// Write SSO session to ~/.aws/config with marker-based organization
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

    // Create backups on first write
    create_backups_if_needed()?;

    let existing_config = if config_path.exists() {
        fs::read_to_string(&config_path)
            .map_err(|e| SsoError::ConfigError(format!("Failed to read config file: {}", e)))?
    } else {
        String::new()
    };

    // Ensure markers exist in the config
    let config_with_markers = ensure_markers(&existing_config);

    // Split into user-managed and awsom-managed sections
    let (header, user_section, awsom_section) = split_into_sections(&config_with_markers);

    // Parse existing SSO sessions from awsom section
    let mut sessions = parse_sso_sessions_from_content(&awsom_section);

    // Add or update the target session
    sessions.retain(|s| s.session_name != session.session_name);
    sessions.push(session.clone());

    // Sort sessions alphabetically by name
    sessions.sort_by(|a, b| a.session_name.cmp(&b.session_name));

    // Rebuild awsom section with sorted sessions
    let new_awsom_section = rebuild_sso_sessions(&sessions);

    // Reconstruct the file using helper
    let result = reconstruct_config(&header, &user_section, &new_awsom_section);

    fs::write(&config_path, cleanup_empty_lines(&result))
        .map_err(|e| SsoError::ConfigError(format!("Failed to write config file: {}", e)))?;

    Ok(())
}

/// Parse SSO sessions from INI content
fn parse_sso_sessions_from_content(content: &str) -> Vec<SsoSession> {
    let mut sessions = Vec::new();
    let mut current_session_name: Option<String> = None;
    let mut session_data: HashMap<String, String> = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Save previous session if complete
            if let Some(name) = current_session_name.take() {
                if let (Some(start_url), Some(region)) = (
                    session_data.get("sso_start_url"),
                    session_data.get("sso_region"),
                ) {
                    let scopes = session_data
                        .get("sso_registration_scopes")
                        .cloned()
                        .unwrap_or_else(|| "sso:account:access".to_string());

                    sessions.push(SsoSession {
                        session_name: name,
                        sso_start_url: start_url.clone(),
                        sso_region: region.clone(),
                        sso_registration_scopes: scopes,
                    });
                }
                session_data.clear();
            }

            // Check if this is an SSO session header
            if trimmed.starts_with("[sso-session ") {
                let name_part = &trimmed[13..trimmed.len() - 1];
                current_session_name = Some(name_part.trim().to_string());
            }
        } else if current_session_name.is_some()
            && trimmed.contains('=')
            && !trimmed.starts_with('#')
        {
            let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
            if parts.len() == 2 {
                let key = parts[0].trim().to_string();
                let value = parts[1].trim().to_string();
                session_data.insert(key, value);
            }
        }
    }

    // Handle last session
    if let Some(name) = current_session_name {
        if let (Some(start_url), Some(region)) = (
            session_data.get("sso_start_url"),
            session_data.get("sso_region"),
        ) {
            let scopes = session_data
                .get("sso_registration_scopes")
                .cloned()
                .unwrap_or_else(|| "sso:account:access".to_string());

            sessions.push(SsoSession {
                session_name: name,
                sso_start_url: start_url.clone(),
                sso_region: region.clone(),
                sso_registration_scopes: scopes,
            });
        }
    }

    sessions
}

/// Rebuild SSO sessions section from a sorted list
fn rebuild_sso_sessions(sessions: &[SsoSession]) -> String {
    let mut result = String::new();

    for session in sessions {
        result.push_str(&format!("[sso-session {}]\n", session.session_name));
        result.push_str(&format!("sso_start_url = {}\n", session.sso_start_url));
        result.push_str(&format!("sso_region = {}\n", session.sso_region));
        result.push_str(&format!(
            "sso_registration_scopes = {}\n",
            session.sso_registration_scopes
        ));
        result.push('\n');
    }

    result
}

/// Delete SSO session from ~/.aws/config with marker-based organization
pub fn delete_sso_session(session_name: &str) -> Result<()> {
    let config_path = config_file_path()?;

    if !config_path.exists() {
        return Ok(()); // Nothing to delete
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| SsoError::ConfigError(format!("Failed to read config file: {}", e)))?;

    // Ensure markers exist in the config
    let config_with_markers = ensure_markers(&content);

    // Split into user-managed and awsom-managed sections
    let (header, user_section, awsom_section) = split_into_sections(&config_with_markers);

    // Parse existing SSO sessions from awsom section
    let mut sessions = parse_sso_sessions_from_content(&awsom_section);

    // Remove the target session
    sessions.retain(|s| s.session_name != session_name);

    // Sort sessions alphabetically by name
    sessions.sort_by(|a, b| a.session_name.cmp(&b.session_name));

    // Rebuild awsom section with sorted sessions
    let new_awsom_section = rebuild_sso_sessions(&sessions);

    // Reconstruct the file using helper
    let result = reconstruct_config(&header, &user_section, &new_awsom_section);

    fs::write(&config_path, cleanup_empty_lines(&result))
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

    // Create backups on first write
    create_backups_if_needed()?;

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

    // Sort credentials profiles alphabetically
    let sorted_content = sort_credentials_profiles(&new_content);

    // Write updated credentials
    fs::write(&creds_path, sorted_content)
        .map_err(|e| SsoError::ConfigError(format!("Failed to write credentials file: {}", e)))?;

    // Check for profile name collision in user-managed section
    if profile_exists_in_user_section(profile_name)? {
        tracing::warn!(
            "Profile '{}' already exists in user-managed section of config file. \
            It will not be modified by awsom. Consider using 'awsom import' to move it \
            to awsom management, or choose a different profile name.",
            profile_name
        );
        // Return early - don't overwrite user-managed profiles
        return Err(SsoError::ConfigError(format!(
            "Profile '{}' exists in user-managed section. \
            Use a different name or run 'awsom import {}' to manage it with awsom.",
            profile_name, profile_name
        )));
    }

    // Also write to config file for region with marker-based organization
    let config_path = config_file_path()?;
    let existing_config = if config_path.exists() {
        fs::read_to_string(&config_path)
            .map_err(|e| SsoError::ConfigError(format!("Failed to read config file: {}", e)))?
    } else {
        String::new()
    };

    // Ensure markers exist in the config
    let config_with_markers = ensure_markers(&existing_config);

    // Split into user-managed and awsom-managed sections
    let (header, user_section, awsom_section) = split_into_sections(&config_with_markers);

    // Parse existing content from awsom section
    let sessions = parse_sso_sessions_from_content(&awsom_section);
    let (default_config_opt, mut profiles) = parse_profiles_from_content(&awsom_section);

    // Build profile config entries
    let profile_section = if profile_name == "default" {
        profile_name.to_string()
    } else {
        format!("profile {}", profile_name)
    };

    let mut config_entries_owned: Vec<(String, String)> = vec![];
    config_entries_owned.push(("region".to_string(), region.to_string()));

    if let Some(output) = output_format {
        config_entries_owned.push(("output".to_string(), output.to_string()));
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

    // Update or add profile
    profiles.retain(|(name, _)| name != &profile_section);
    profiles.push((profile_section, config_entries_owned));

    // Sort profiles alphabetically by name
    profiles.sort_by(|a, b| a.0.cmp(&b.0));

    // Build the awsom-managed section: [default] first (if exists), then sorted SSO sessions, then sorted profiles
    let mut new_awsom_section = String::new();

    // Add [default] section if it exists
    if let Some(default_config) = default_config_opt {
        new_awsom_section.push_str("[default]\n");
        for (key, value) in default_config {
            new_awsom_section.push_str(&format!("{} = {}\n", key, value));
        }
        new_awsom_section.push('\n');
    }

    // Add sorted SSO sessions
    new_awsom_section.push_str(&rebuild_sso_sessions(&sessions));

    // Add sorted profiles (skipping default as it was handled above)
    for (profile_name, entries) in profiles {
        if profile_name != "default" {
            new_awsom_section.push_str(&format!("[{}]\n", profile_name));
            for (key, value) in entries {
                new_awsom_section.push_str(&format!("{} = {}\n", key, value));
            }
            new_awsom_section.push('\n');
        }
    }

    // Reconstruct the file using helper
    let result = reconstruct_config(&header, &user_section, &new_awsom_section);

    fs::write(&config_path, cleanup_empty_lines(&result))
        .map_err(|e| SsoError::ConfigError(format!("Failed to write config file: {}", e)))?;

    Ok(())
}

/// Check if a profile exists in the user-managed section
/// Returns true if the profile name exists above the marker
fn profile_exists_in_user_section(profile_name: &str) -> Result<bool> {
    let config_path = config_file_path()?;

    if !config_path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| SsoError::ConfigError(format!("Failed to read config file: {}", e)))?;

    // Ensure markers exist
    let content_with_markers = ensure_markers(&content);

    // Split into user-managed and awsom-managed sections
    let (user_section, _) = split_by_marker(&content_with_markers);

    // Parse profiles from user section
    let (default_config, profiles) = parse_profiles_from_content(&user_section);

    // Check if profile_name matches
    let profile_section = if profile_name == "default" {
        "default".to_string()
    } else {
        format!("profile {}", profile_name)
    };

    // Check if it exists in default config
    if profile_section == "default" && default_config.is_some() {
        return Ok(true);
    }

    // Check if it exists in other profiles
    for (name, _) in profiles {
        if name == profile_section {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Type alias for profile parsing result
type ProfilesParseResult = (
    Option<Vec<(String, String)>>,
    Vec<(String, Vec<(String, String)>)>,
);

/// Sort profiles in credentials file alphabetically ([default] first, then sorted)
fn sort_credentials_profiles(content: &str) -> String {
    let mut profiles: Vec<(String, Vec<String>)> = Vec::new();
    let mut current_profile: Option<String> = None;
    let mut profile_lines: Vec<String> = Vec::new();
    let mut header_lines: Vec<String> = Vec::new();
    let mut in_header = true;

    for line in content.lines() {
        let trimmed = line.trim();

        // Collect header comments before first profile
        if in_header && !trimmed.starts_with('[') {
            header_lines.push(line.to_string());
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_header = false;

            // Save previous profile
            if let Some(name) = current_profile.take() {
                profiles.push((name, profile_lines.clone()));
                profile_lines.clear();
            }

            // Start new profile
            let profile_name = trimmed[1..trimmed.len() - 1].to_string();
            current_profile = Some(profile_name);
            profile_lines.push(line.to_string());
        } else if current_profile.is_some() {
            profile_lines.push(line.to_string());
        }
    }

    // Save last profile
    if let Some(name) = current_profile {
        profiles.push((name, profile_lines.clone()));
    }

    // Sort profiles: [default] first, then alphabetically
    profiles.sort_by(|a, b| match (a.0.as_str(), b.0.as_str()) {
        ("default", "default") => std::cmp::Ordering::Equal,
        ("default", _) => std::cmp::Ordering::Less,
        (_, "default") => std::cmp::Ordering::Greater,
        (x, y) => x.cmp(y),
    });

    // Rebuild file
    let mut result = String::new();

    // Add header
    for line in header_lines {
        result.push_str(&line);
        result.push('\n');
    }

    // Add sorted profiles
    for (_, lines) in profiles {
        for line in lines {
            result.push_str(&line);
            result.push('\n');
        }
        // Add blank line between profiles
        result.push('\n');
    }

    cleanup_empty_lines(&result)
}

/// Parse profiles from INI content
/// Returns (default_config_option, vec of (profile_name, vec of (key, value)))
fn parse_profiles_from_content(content: &str) -> ProfilesParseResult {
    let mut default_config: Option<Vec<(String, String)>> = None;
    let mut profiles = Vec::new();
    let mut current_profile_name: Option<String> = None;
    let mut profile_data: Vec<(String, String)> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Save previous profile if complete
            if let Some(name) = current_profile_name.take() {
                if name == "default" {
                    default_config = Some(profile_data.clone());
                } else {
                    profiles.push((name, profile_data.clone()));
                }
                profile_data.clear();
            }

            // Check if this is a profile section (not sso-session)
            let section = &trimmed[1..trimmed.len() - 1];
            if section == "default" {
                current_profile_name = Some("default".to_string());
            } else if section.starts_with("profile ") {
                current_profile_name = Some(section.to_string());
            } else if !section.starts_with("sso-session ") {
                // Some other section that's not sso-session
                current_profile_name = Some(section.to_string());
            }
        } else if current_profile_name.is_some()
            && trimmed.contains('=')
            && !trimmed.starts_with('#')
        {
            let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
            if parts.len() == 2 {
                let key = parts[0].trim().to_string();
                let value = parts[1].trim().to_string();
                profile_data.push((key, value));
            }
        }
    }

    // Handle last profile
    if let Some(name) = current_profile_name {
        if name == "default" {
            default_config = Some(profile_data);
        } else {
            profiles.push((name, profile_data));
        }
    }

    (default_config, profiles)
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

/// Profile configuration information
#[derive(Debug, Clone)]
pub struct ProfileInfo {
    pub name: String,
    pub region: String,
    pub output: String,
}

/// Get profile configuration by matching sso_session, sso_account_id, and sso_role_name
/// Searches both ~/.aws/config and ~/.aws/credentials for a matching profile
pub fn get_profile_by_role(
    sso_session_name: &str,
    account_id: &str,
    role_name: &str,
) -> Result<Option<ProfileInfo>> {
    // First, try to find in ~/.aws/config
    if let Some(profile) = get_profile_from_config(sso_session_name, account_id, role_name)? {
        return Ok(Some(profile));
    }

    // Fallback: try to find in ~/.aws/credentials (for orphaned credentials)
    if let Some(profile) = get_profile_from_credentials(account_id, role_name)? {
        return Ok(Some(profile));
    }

    Ok(None)
}

/// Search ~/.aws/config for profile with matching sso_session, account_id, and role_name
fn get_profile_from_config(
    sso_session_name: &str,
    account_id: &str,
    role_name: &str,
) -> Result<Option<ProfileInfo>> {
    let config_path = config_file_path()?;

    if !config_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| SsoError::ConfigError(format!("Failed to read config file: {}", e)))?;

    let mut current_profile: Option<String> = None;
    let mut profile_data: HashMap<String, String> = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("[profile ") && trimmed.ends_with(']') {
            // Check previous profile for match
            if let Some(profile_name) = current_profile.take() {
                if let Some(profile_info) = check_config_profile_match(
                    &profile_name,
                    &profile_data,
                    sso_session_name,
                    account_id,
                    role_name,
                )? {
                    return Ok(Some(profile_info));
                }
                profile_data.clear();
            }

            // Extract profile name (remove "[profile " prefix and "]" suffix)
            current_profile = Some(trimmed[9..trimmed.len() - 1].to_string());
        } else if trimmed.starts_with("[default]") {
            // Handle [default] section (no "profile" prefix)
            if let Some(profile_name) = current_profile.take() {
                if let Some(profile_info) = check_config_profile_match(
                    &profile_name,
                    &profile_data,
                    sso_session_name,
                    account_id,
                    role_name,
                )? {
                    return Ok(Some(profile_info));
                }
                profile_data.clear();
            }
            current_profile = Some("default".to_string());
        } else if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with('[') {
            // Parse key=value pairs
            if let Some(eq_pos) = trimmed.find('=') {
                let key = trimmed[..eq_pos].trim().to_string();
                let value = trimmed[eq_pos + 1..].trim().to_string();
                profile_data.insert(key, value);
            }
        }
    }

    // Check last profile
    if let Some(profile_name) = current_profile {
        if let Some(profile_info) = check_config_profile_match(
            &profile_name,
            &profile_data,
            sso_session_name,
            account_id,
            role_name,
        )? {
            return Ok(Some(profile_info));
        }
    }

    Ok(None)
}

/// Check if a config profile matches the criteria
fn check_config_profile_match(
    profile_name: &str,
    profile_data: &HashMap<String, String>,
    sso_session_name: &str,
    account_id: &str,
    role_name: &str,
) -> Result<Option<ProfileInfo>> {
    // Check for match on all three keys
    let matches_session = profile_data
        .get("sso_session")
        .map(|s| s == sso_session_name)
        .unwrap_or(false);
    let matches_account = profile_data
        .get("sso_account_id")
        .map(|s| s == account_id)
        .unwrap_or(false);
    let matches_role = profile_data
        .get("sso_role_name")
        .map(|s| s == role_name)
        .unwrap_or(false);

    if matches_session && matches_account && matches_role {
        // Found a match! Extract region and output
        let region = profile_data
            .get("region")
            .cloned()
            .unwrap_or_else(|| "us-east-1".to_string());
        let output = profile_data
            .get("output")
            .cloned()
            .unwrap_or_else(|| "json".to_string());

        return Ok(Some(ProfileInfo {
            name: profile_name.to_string(),
            region,
            output,
        }));
    }

    Ok(None)
}

/// Search ~/.aws/credentials for profile with matching account_id and role_name in metadata
fn get_profile_from_credentials(account_id: &str, role_name: &str) -> Result<Option<ProfileInfo>> {
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
            if let Some(profile) = &current_profile {
                if found_account_id && found_role_name {
                    // Found in credentials, but we don't have region/output info
                    // Return with defaults
                    return Ok(Some(ProfileInfo {
                        name: profile.clone(),
                        region: "us-east-1".to_string(),
                        output: "json".to_string(),
                    }));
                }
            }

            // Start new profile
            current_profile = Some(trimmed[1..trimmed.len() - 1].to_string());
            found_account_id = false;
            found_role_name = false;
        } else if current_profile.is_some() {
            // Check for metadata comments
            if trimmed.starts_with('#') {
                if trimmed.contains(&format!("Account: {}", account_id)) {
                    found_account_id = true;
                } else if trimmed.contains(&format!("Role: {}", role_name)) {
                    found_role_name = true;
                }
            }
        }
    }

    // Check last profile
    if let Some(profile) = current_profile {
        if found_account_id && found_role_name {
            return Ok(Some(ProfileInfo {
                name: profile,
                region: "us-east-1".to_string(),
                output: "json".to_string(),
            }));
        }
    }

    Ok(None)
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

/// Clean up empty lines in INI files (public for import command):
/// - Remove leading empty lines
/// - Ensure exactly one blank line between sections
/// - Remove trailing empty lines
pub fn cleanup_empty_lines(content: &str) -> String {
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
