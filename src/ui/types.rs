// UI types - extracted from app.rs for better modularity

use crate::models::{AccountRole, SsoInstance, SsoToken};

/// Message type for background login tasks
pub enum LoginResult {
    Success {
        session_index: usize,
        token: Box<SsoToken>,
        instance: SsoInstance,
        session_name: String,
    },
    Error {
        message: String,
    },
    #[allow(dead_code)]
    Cancelled,
}

/// Wrapper for AccountRole with active status
#[derive(Debug, Clone)]
pub struct AccountRoleWithStatus {
    pub account_role: AccountRole,
    pub is_active: bool,
    pub expiration: Option<chrono::DateTime<chrono::Utc>>,
    pub is_default: bool,
    pub profile_name: Option<String>,
}

/// Profile entry that can be either SSO or static credentials
#[derive(Debug, Clone)]
pub enum ProfileEntry {
    Sso(AccountRoleWithStatus),
    Static {
        profile_name: String,
        is_default: bool,
        #[allow(dead_code)]
        credentials: crate::models::StaticCredentials,
    },
    /// Profile defined in config but without credentials
    Incomplete {
        profile_name: String,
        region: Option<String>,
        output: Option<String>,
    },
}

/// SSO Session with its status
#[derive(Debug, Clone)]
pub struct SsoSessionInfo {
    pub session_name: String,
    pub start_url: String,
    pub region: String,
    pub is_active: bool,
    pub token_expiration: Option<chrono::DateTime<chrono::Utc>>,
    pub instance: SsoInstance,
    pub token: Option<SsoToken>,
}

/// Active pane in two-pane layout
#[derive(Debug, Clone, PartialEq)]
pub enum ActivePane {
    Sessions,
    Accounts,
}

/// Action to perform when user confirms in confirmation dialog
#[derive(Debug, Clone)]
pub enum ConfirmAction {
    /// Make a profile the default profile
    MakeProfileDefault {
        from_profile: String,
        #[allow(dead_code)]
        account: AccountRole,
    },
    /// Rename/overwrite a profile
    RenameProfile {
        old_name: String,
        new_name: String,
        account: AccountRole,
    },
    /// Delete an SSO session
    DeleteSession {
        session_index: usize,
        session_name: String,
    },
    /// Delete a static credential profile
    DeleteProfile { profile_name: String },
}
