use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents an AWS SSO instance configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SsoInstance {
    pub start_url: String,
    pub region: String,
}

/// Cached SSO-OIDC token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoToken {
    pub access_token: String,
    pub expires_at: DateTime<Utc>,
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
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
    pub fn display_name(&self) -> String {
        format!("{}/{}", self.account_name, self.role_name)
    }

    pub fn full_display(&self) -> String {
        format!(
            "{} ({}): {}",
            self.account_name, self.account_id, self.role_name
        )
    }
}

/// AWS temporary credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: String,
    pub expiration: DateTime<Utc>,
}

impl RoleCredentials {
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
#[derive(Debug, Clone)]
pub struct ProfileSession {
    pub profile_name: String,
    pub account_role: AccountRole,
    pub credentials: Option<RoleCredentials>,
    pub is_default: bool,
    pub sso_instance: SsoInstance,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Active,
    Expiring,
    Expired,
    Inactive,
}

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
