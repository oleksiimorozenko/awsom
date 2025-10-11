use thiserror::Error;

#[derive(Error, Debug)]
pub enum SsoError {
    #[error("AWS SDK error: {0}")]
    AwsSdk(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Authorization pending - user must complete device flow")]
    AuthorizationPending,

    #[error("Authorization expired - user took too long to complete device flow")]
    AuthorizationExpired,

    #[error("Token expired or invalid")]
    TokenExpired,

    #[error("Invalid SSO configuration: {0}")]
    InvalidConfig(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML serialization error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("No SSO session found")]
    NoSessionFound,

    #[error("Account or role not found")]
    AccountRoleNotFound,

    #[error("Browser launch failed: {0}")]
    BrowserLaunchFailed(String),
}

pub type Result<T> = std::result::Result<T, SsoError>;
