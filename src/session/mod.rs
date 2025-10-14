// Session management module
use crate::auth::AuthManager;
use crate::credentials::CredentialManager;
use crate::error::Result;
use crate::models::{AccountRole, ProfileSession, SsoInstance, SsoToken};

pub struct SessionManager {
    auth: AuthManager,
    creds: CredentialManager,
}

impl SessionManager {
    pub fn new() -> Result<Self> {
        Ok(Self {
            auth: AuthManager::new()?,
            creds: CredentialManager::new()?,
        })
    }

    pub async fn login(
        &self,
        instance: &SsoInstance,
        force: bool,
        headless: bool,
    ) -> Result<SsoToken> {
        self.auth.login(instance, force, headless).await
    }

    pub async fn activate_session(
        &self,
        instance: &SsoInstance,
        token: &SsoToken,
        role: &AccountRole,
    ) -> Result<ProfileSession> {
        let credentials = self.creds.get_credentials(instance, token, role).await?;

        Ok(ProfileSession {
            profile_name: format!("{}_{}", role.account_name, role.role_name),
            account_role: role.clone(),
            credentials: Some(credentials),
            is_default: false,
            sso_instance: instance.clone(),
        })
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new().expect("Failed to initialize SessionManager")
    }
}
