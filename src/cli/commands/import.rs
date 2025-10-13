// Import command - moves sections from user-managed to awsom-managed area
use crate::aws_config;
use crate::error::{Result, SsoError};
use std::io::{self, Write};

pub async fn execute(name: String, section_type: String, force: bool) -> Result<()> {
    // Validate section type
    let section_type = section_type.to_lowercase();
    if section_type != "profile" && section_type != "sso-session" {
        return Err(SsoError::ConfigError(
            "Invalid section type. Must be 'profile' or 'sso-session'".to_string(),
        ));
    }

    // Read the config file
    let config_path = aws_config::config_file_path()?;
    if !config_path.exists() {
        return Err(SsoError::ConfigError(
            "Config file does not exist. Nothing to import.".to_string(),
        ));
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| SsoError::ConfigError(format!("Failed to read config file: {}", e)))?;

    // Check if the section exists in user-managed area
    let (user_section, _awsom_section) = find_section_in_user_area(&content, &name, &section_type)?;

    if user_section.is_none() {
        return Err(SsoError::ConfigError(format!(
            "{} '{}' not found in user-managed section. Nothing to import.",
            if section_type == "profile" {
                "Profile"
            } else {
                "SSO session"
            },
            name
        )));
    }

    let (section_name, section_content) = user_section.unwrap();

    // Confirm import unless --force is used
    if !force {
        println!("Found {} to import:", section_type);
        println!("\n[{}]", section_name);
        for line in section_content.lines() {
            if !line.trim().is_empty() {
                println!("{}", line);
            }
        }
        println!();
        print!("Move this {} to awsom management? (y/N): ", section_type);
        io::stdout().flush().map_err(SsoError::Io)?;

        let mut response = String::new();
        io::stdin().read_line(&mut response).map_err(SsoError::Io)?;

        if !response.trim().eq_ignore_ascii_case("y") {
            println!("Import cancelled.");
            return Ok(());
        }
    }

    // Perform the import based on section type
    if section_type == "sso-session" {
        import_sso_session(&name, &section_content)?;
        println!("✓ Imported SSO session '{}' to awsom management", name);
    } else {
        import_profile(&name, &section_name, &section_content)?;
        println!("✓ Imported profile '{}' to awsom management", name);
    }

    println!();
    println!(
        "The {} has been moved from user-managed to awsom-managed section.",
        section_type
    );
    println!("It will now be automatically organized and sorted by awsom.");

    Ok(())
}

/// Find a section in the user-managed area
/// Returns (Some((section_name, section_content)), awsom_section) if found, (None, awsom_section) if not found
fn find_section_in_user_area(
    content: &str,
    name: &str,
    section_type: &str,
) -> Result<(Option<(String, String)>, String)> {
    use crate::aws_config::{ensure_markers, split_by_marker};

    let content_with_markers = ensure_markers(content);
    let (user_section, awsom_section) = split_by_marker(&content_with_markers);

    // Determine the section header to look for
    let section_header = if section_type == "sso-session" {
        format!("[sso-session {}]", name)
    } else if name == "default" {
        "[default]".to_string()
    } else {
        format!("[profile {}]", name)
    };

    // Parse the user section to find the target
    let mut found_section: Option<(String, String)> = None;
    let mut in_target_section = false;
    let mut section_content = String::new();

    for line in user_section.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // If we were in the target section, save it
            if in_target_section {
                found_section = Some((section_header.clone(), section_content.clone()));
                break;
            }

            // Check if this is our target section
            if trimmed == section_header {
                in_target_section = true;
                section_content.clear();
            }
        } else if in_target_section && !trimmed.is_empty() {
            section_content.push_str(line);
            section_content.push('\n');
        }
    }

    // Handle case where target section is the last one
    if in_target_section && !section_content.is_empty() {
        found_section = Some((section_header, section_content));
    }

    Ok((found_section, awsom_section))
}

/// Import an SSO session by parsing its content and calling write_sso_session
fn import_sso_session(name: &str, content: &str) -> Result<()> {
    use std::collections::HashMap;

    // Parse the section content
    let mut properties: HashMap<String, String> = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.contains('=') && !trimmed.starts_with('#') {
            let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
            if parts.len() == 2 {
                let key = parts[0].trim().to_string();
                let value = parts[1].trim().to_string();
                properties.insert(key, value);
            }
        }
    }

    // Extract required fields
    let start_url = properties
        .get("sso_start_url")
        .ok_or_else(|| SsoError::ConfigError("SSO session missing sso_start_url".to_string()))?
        .clone();

    let region = properties
        .get("sso_region")
        .ok_or_else(|| SsoError::ConfigError("SSO session missing sso_region".to_string()))?
        .clone();

    let scopes = properties
        .get("sso_registration_scopes")
        .cloned()
        .unwrap_or_else(|| "sso:account:access".to_string());

    // Create SsoSession and write it (which will place it in awsom-managed section)
    let session = aws_config::SsoSession {
        session_name: name.to_string(),
        sso_start_url: start_url,
        sso_region: region,
        sso_registration_scopes: scopes,
    };

    // Remove from user-managed section first
    remove_section_from_user_area(name, "sso-session")?;

    // Write to awsom-managed section
    aws_config::write_sso_session(&session)?;

    Ok(())
}

/// Import a profile by parsing its content and calling write_credentials_with_metadata
fn import_profile(profile_name: &str, section_name: &str, content: &str) -> Result<()> {
    use std::collections::HashMap;

    // Parse the section content
    let mut properties: HashMap<String, String> = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.contains('=') && !trimmed.starts_with('#') {
            let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
            if parts.len() == 2 {
                let key = parts[0].trim().to_string();
                let value = parts[1].trim().to_string();
                properties.insert(key, value);
            }
        }
    }

    // Remove from user-managed section first
    remove_section_from_user_area(profile_name, "profile")?;

    // Re-write the profile to awsom-managed section
    // We'll use a simple INI update approach
    let config_path = aws_config::config_file_path()?;
    let existing_content = std::fs::read_to_string(&config_path)
        .map_err(|e| SsoError::ConfigError(format!("Failed to read config file: {}", e)))?;

    use crate::aws_config::{ensure_markers, split_by_marker};
    let content_with_markers = ensure_markers(&existing_content);
    let (user_section, awsom_section) = split_by_marker(&content_with_markers);

    // Add this profile to awsom section
    let mut new_awsom_section = awsom_section;
    new_awsom_section.push('\n');
    // Extract the section name without brackets if present
    let clean_section_name = if section_name.starts_with('[') && section_name.ends_with(']') {
        &section_name[1..section_name.len() - 1]
    } else {
        section_name
    };
    new_awsom_section.push_str(&format!("[{}]\n", clean_section_name));
    for (key, value) in properties {
        new_awsom_section.push_str(&format!("{} = {}\n", key, value));
    }

    // Reconstruct the file
    use crate::aws_config::{
        cleanup_empty_lines, AWSOM_MANAGED_COMMENT, AWSOM_MANAGED_MARKER, USER_MANAGED_COMMENT,
        USER_MANAGED_MARKER,
    };
    let mut result = user_section;
    result.push_str(USER_MANAGED_MARKER);
    result.push('\n');
    result.push_str(USER_MANAGED_COMMENT);
    result.push_str("\n\n");
    result.push_str(AWSOM_MANAGED_MARKER);
    result.push('\n');
    result.push_str(AWSOM_MANAGED_COMMENT);
    result.push('\n');
    if !new_awsom_section.trim().is_empty() {
        result.push('\n');
        result.push_str(&new_awsom_section);
    }

    std::fs::write(&config_path, cleanup_empty_lines(&result))
        .map_err(|e| SsoError::ConfigError(format!("Failed to write config file: {}", e)))?;

    Ok(())
}

/// Remove a section from the user-managed area
fn remove_section_from_user_area(name: &str, section_type: &str) -> Result<()> {
    use crate::aws_config::{ensure_markers, split_by_marker};

    let config_path = aws_config::config_file_path()?;
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| SsoError::ConfigError(format!("Failed to read config file: {}", e)))?;

    let content_with_markers = ensure_markers(&content);
    let (user_section, awsom_section) = split_by_marker(&content_with_markers);

    // Determine the section header to remove
    let section_header = if section_type == "sso-session" {
        format!("[sso-session {}]", name)
    } else if name == "default" {
        "[default]".to_string()
    } else {
        format!("[profile {}]", name)
    };

    // Remove the section from user_section
    let mut new_user_section = String::new();
    let mut in_target_section = false;

    for line in user_section.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Check if we're entering or leaving the target section
            if trimmed == section_header {
                in_target_section = true;
                continue; // Skip this line
            } else {
                in_target_section = false;
            }
        }

        if !in_target_section {
            new_user_section.push_str(line);
            new_user_section.push('\n');
        }
    }

    // Reconstruct the file without the removed section
    use crate::aws_config::{
        AWSOM_MANAGED_COMMENT, AWSOM_MANAGED_MARKER, USER_MANAGED_COMMENT, USER_MANAGED_MARKER,
    };
    let mut result = new_user_section;
    result.push_str(USER_MANAGED_MARKER);
    result.push('\n');
    result.push_str(USER_MANAGED_COMMENT);
    result.push_str("\n\n");
    result.push_str(AWSOM_MANAGED_MARKER);
    result.push('\n');
    result.push_str(AWSOM_MANAGED_COMMENT);
    result.push('\n');
    if !awsom_section.trim().is_empty() {
        result.push('\n');
        result.push_str(&awsom_section);
    }

    use crate::aws_config::cleanup_empty_lines;
    std::fs::write(&config_path, cleanup_empty_lines(&result))
        .map_err(|e| SsoError::ConfigError(format!("Failed to write config file: {}", e)))?;

    Ok(())
}
