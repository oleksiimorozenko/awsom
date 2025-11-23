use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents an AWS SSO instance configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SsoInstance {
    pub start_url: String,
    pub region: String,
    /// Session name (for AWS CLI v2 [sso-session] compatibility)
    /// When present, token cache uses SHA1 of session_name instead of start_url
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_name: Option<String>,
}

/// Cached SSO-OIDC token (AWS CLI v2 compatible format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoToken {
    /// Access token (serialized as camelCase for AWS CLI v2 compatibility)
    #[serde(rename = "accessToken", alias = "access_token")]
    pub access_token: String,

    /// Expiration timestamp (serialized as camelCase for AWS CLI v2 compatibility)
    #[serde(rename = "expiresAt", alias = "expires_at")]
    pub expires_at: DateTime<Utc>,

    /// Refresh token (optional, serialized as camelCase for AWS CLI v2 compatibility)
    #[serde(
        rename = "refreshToken",
        alias = "refresh_token",
        skip_serializing_if = "Option::is_none"
    )]
    pub refresh_token: Option<String>,

    /// Region (optional, required for compatibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,

    /// Start URL (optional, for AWS CLI v2 compatibility)
    #[serde(
        rename = "startUrl",
        alias = "start_url",
        skip_serializing_if = "Option::is_none"
    )]
    pub start_url: Option<String>,
}

impl SsoToken {
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }

    pub fn expires_in_seconds(&self) -> i64 {
        (self.expires_at - Utc::now()).num_seconds().max(0)
    }

    pub fn expires_in_minutes(&self) -> i64 {
        self.expires_in_seconds() / 60
    }

    /// Format expiration time as human-readable string
    pub fn expiration_display(&self) -> String {
        let mins = self.expires_in_minutes();

        if mins >= 60 {
            let hours = mins / 60;
            let remaining_mins = mins % 60;
            if remaining_mins > 0 {
                format!("{}h {}m", hours, remaining_mins)
            } else {
                format!("{}h", hours)
            }
        } else if mins > 0 {
            format!("{} minutes", mins)
        } else {
            "EXPIRED".to_string()
        }
    }
}

/// Represents an AWS account available through SSO
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AwsAccount {
    pub account_id: String,
    pub account_name: String,
}

/// Represents a role within an AWS account
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AccountRole {
    pub account_id: String,
    pub account_name: String,
    pub role_name: String,
}

impl AccountRole {
    #[allow(dead_code)]
    pub fn display_name(&self) -> String {
        format!("{}/{}", self.account_name, self.role_name)
    }

    #[allow(dead_code)]
    pub fn full_display(&self) -> String {
        format!(
            "{} ({}): {}",
            self.account_name, self.account_id, self.role_name
        )
    }
}

/// Credential type - SSO or Static
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CredentialType {
    Sso,
    Static,
}

/// Static AWS credentials (long-term access keys)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    /// Optional session token for temporary static credentials
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_token: Option<String>,
}

impl StaticCredentials {
    /// Validate that credentials have required fields
    pub fn validate(&self) -> Result<(), String> {
        if self.access_key_id.trim().is_empty() {
            return Err("Access Key ID cannot be empty".to_string());
        }
        if self.secret_access_key.trim().is_empty() {
            return Err("Secret Access Key cannot be empty".to_string());
        }
        // Basic format validation for access key
        if !self.access_key_id.starts_with("AKIA") && !self.access_key_id.starts_with("ASIA") {
            return Err("Access Key ID should start with AKIA or ASIA".to_string());
        }
        Ok(())
    }

    /// Check if this includes a session token (temporary credentials)
    #[allow(dead_code)]
    pub fn is_temporary(&self) -> bool {
        self.session_token.is_some()
    }
}

/// AWS temporary credentials (from SSO role assumption)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: String,
    pub expiration: DateTime<Utc>,
}

impl RoleCredentials {
    #[allow(dead_code)]
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expiration
    }

    pub fn expires_in_seconds(&self) -> i64 {
        (self.expiration - Utc::now()).num_seconds().max(0)
    }

    pub fn expires_in_minutes(&self) -> i64 {
        self.expires_in_seconds() / 60
    }

    /// Format expiration time as human-readable string
    pub fn expiration_display(&self) -> String {
        let mins = self.expires_in_minutes();
        let secs = self.expires_in_seconds() % 60;

        if mins > 60 {
            let hours = mins / 60;
            let remaining_mins = mins % 60;
            format!("{}h {}m", hours, remaining_mins)
        } else if mins > 0 {
            format!("{}m {}s", mins, secs)
        } else if secs > 0 {
            format!("{}s", secs)
        } else {
            "EXPIRED".to_string()
        }
    }
}

/// Represents an active profile session
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ProfileSession {
    pub profile_name: String,
    pub account_role: AccountRole,
    pub credentials: Option<RoleCredentials>,
    pub is_default: bool,
    pub sso_instance: SsoInstance,
}

#[allow(dead_code)]
impl ProfileSession {
    pub fn is_active(&self) -> bool {
        self.credentials
            .as_ref()
            .map(|c| !c.is_expired())
            .unwrap_or(false)
    }

    pub fn status(&self) -> SessionStatus {
        match &self.credentials {
            None => SessionStatus::Inactive,
            Some(creds) if creds.is_expired() => SessionStatus::Expired,
            Some(creds) if creds.expires_in_minutes() < 5 => SessionStatus::Expiring,
            Some(_) => SessionStatus::Active,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Active,
    Expiring,
    Expired,
    Inactive,
}

#[allow(dead_code)]
impl SessionStatus {
    pub fn as_str(&self) -> &str {
        match self {
            SessionStatus::Active => "ACTIVE",
            SessionStatus::Expiring => "EXPIRING",
            SessionStatus::Expired => "EXPIRED",
            SessionStatus::Inactive => "INACTIVE",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_sso_instance_equality() {
        let instance1 = SsoInstance {
            start_url: "https://example.awsapps.com/start".to_string(),
            region: "us-east-1".to_string(),
            session_name: None,
        };
        let instance2 = SsoInstance {
            start_url: "https://example.awsapps.com/start".to_string(),
            region: "us-east-1".to_string(),
            session_name: None,
        };
        assert_eq!(instance1, instance2);
    }

    #[test]
    fn test_sso_token_is_expired() {
        let expired_token = SsoToken {
            access_token: "test".to_string(),
            expires_at: Utc::now() - Duration::hours(1),
            refresh_token: None,
            region: None,
            start_url: None,
        };
        assert!(expired_token.is_expired());

        let valid_token = SsoToken {
            access_token: "test".to_string(),
            expires_at: Utc::now() + Duration::hours(1),
            refresh_token: None,
            region: None,
            start_url: None,
        };
        assert!(!valid_token.is_expired());
    }

    #[test]
    fn test_sso_token_expiration_display() {
        let token = SsoToken {
            access_token: "test".to_string(),
            expires_at: Utc::now() + Duration::minutes(90),
            refresh_token: None,
            region: None,
            start_url: None,
        };
        let display = token.expiration_display();
        assert!(display.contains("1h"));

        let expired = SsoToken {
            access_token: "test".to_string(),
            expires_at: Utc::now() - Duration::minutes(10),
            refresh_token: None,
            region: None,
            start_url: None,
        };
        assert_eq!(expired.expiration_display(), "EXPIRED");
    }

    #[test]
    fn test_account_role_display() {
        let role = AccountRole {
            account_id: "123456789012".to_string(),
            account_name: "Production".to_string(),
            role_name: "Developer".to_string(),
        };
        assert_eq!(role.display_name(), "Production/Developer");
        assert_eq!(role.full_display(), "Production (123456789012): Developer");
    }

    #[test]
    fn test_role_credentials_expiration() {
        let creds = RoleCredentials {
            access_key_id: "AKIAIOSFODNN7EXAMPLE".to_string(),
            secret_access_key: "secret".to_string(),
            session_token: "token".to_string(),
            expiration: Utc::now() + Duration::minutes(30),
        };
        assert!(!creds.is_expired());
        assert!(creds.expires_in_minutes() > 0);
    }

    #[test]
    fn test_session_status() {
        assert_eq!(SessionStatus::Active.as_str(), "ACTIVE");
        assert_eq!(SessionStatus::Expiring.as_str(), "EXPIRING");
        assert_eq!(SessionStatus::Expired.as_str(), "EXPIRED");
        assert_eq!(SessionStatus::Inactive.as_str(), "INACTIVE");
    }

    #[test]
    fn test_profile_session_status() {
        let instance = SsoInstance {
            start_url: "https://example.awsapps.com/start".to_string(),
            region: "us-east-1".to_string(),
            session_name: None,
        };
        let role = AccountRole {
            account_id: "123456789012".to_string(),
            account_name: "Test".to_string(),
            role_name: "Admin".to_string(),
        };

        // Test inactive session (no credentials)
        let inactive_session = ProfileSession {
            profile_name: "test".to_string(),
            account_role: role.clone(),
            credentials: None,
            is_default: false,
            sso_instance: instance.clone(),
        };
        assert!(!inactive_session.is_active());
        assert_eq!(inactive_session.status(), SessionStatus::Inactive);

        // Test active session
        let active_creds = RoleCredentials {
            access_key_id: "key".to_string(),
            secret_access_key: "secret".to_string(),
            session_token: "token".to_string(),
            expiration: Utc::now() + Duration::hours(1),
        };
        let active_session = ProfileSession {
            profile_name: "test".to_string(),
            account_role: role.clone(),
            credentials: Some(active_creds),
            is_default: false,
            sso_instance: instance.clone(),
        };
        assert!(active_session.is_active());
        assert_eq!(active_session.status(), SessionStatus::Active);

        // Test expiring session
        let expiring_creds = RoleCredentials {
            access_key_id: "key".to_string(),
            secret_access_key: "secret".to_string(),
            session_token: "token".to_string(),
            expiration: Utc::now() + Duration::minutes(3),
        };
        let expiring_session = ProfileSession {
            profile_name: "test".to_string(),
            account_role: role,
            credentials: Some(expiring_creds),
            is_default: false,
            sso_instance: instance,
        };
        assert!(expiring_session.is_active());
        assert_eq!(expiring_session.status(), SessionStatus::Expiring);
    }

    #[test]
    fn test_static_credentials_validation() {
        // Valid credentials
        let valid_creds = StaticCredentials {
            access_key_id: "AKIAIOSFODNN7EXAMPLE".to_string(),
            secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
            session_token: None,
        };
        assert!(valid_creds.validate().is_ok());
        assert!(!valid_creds.is_temporary());

        // With session token (temporary)
        let temp_creds = StaticCredentials {
            access_key_id: "ASIAIOSFODNN7EXAMPLE".to_string(),
            secret_access_key: "secret".to_string(),
            session_token: Some("temp_token".to_string()),
        };
        assert!(temp_creds.validate().is_ok());
        assert!(temp_creds.is_temporary());

        // Invalid - empty access key
        let empty_key = StaticCredentials {
            access_key_id: "".to_string(),
            secret_access_key: "secret".to_string(),
            session_token: None,
        };
        assert!(empty_key.validate().is_err());

        // Invalid - empty secret
        let empty_secret = StaticCredentials {
            access_key_id: "AKIAIOSFODNN7EXAMPLE".to_string(),
            secret_access_key: "".to_string(),
            session_token: None,
        };
        assert!(empty_secret.validate().is_err());

        // Invalid - wrong prefix
        let wrong_prefix = StaticCredentials {
            access_key_id: "INVALIDKEY".to_string(),
            secret_access_key: "secret".to_string(),
            session_token: None,
        };
        assert!(wrong_prefix.validate().is_err());
    }

    #[test]
    fn test_credential_type() {
        assert_eq!(CredentialType::Sso, CredentialType::Sso);
        assert_eq!(CredentialType::Static, CredentialType::Static);
        assert_ne!(CredentialType::Sso, CredentialType::Static);
    }
}
