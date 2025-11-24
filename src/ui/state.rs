// App state machine - extracted from app.rs

/// Application state machine
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    /// Main screen showing account/role list
    Main,
    /// Help screen
    Help,
    /// Loading state
    Loading,
    /// Error state
    Error(String),
    /// Profile name input
    #[allow(dead_code)]
    ProfileInput,
    /// SSO configuration input
    SsoConfigInput { step: SsoConfigStep },
    /// Default profile configuration input
    DefaultsConfigInput { step: DefaultsConfigStep },
    /// New profile configuration input (with region and output)
    NewProfileConfigInput { step: NewProfileConfigStep },
    /// Static credential input (for creating/editing static profiles)
    StaticCredentialInput { step: StaticCredentialStep },
    /// Confirmation dialog
    ConfirmationDialog { title: String, message: Vec<String> },
    /// View profile details
    ViewProfile { details: Vec<(String, String)> },
    /// SSM browser - browse EC2 instances for the current profile
    SsmBrowser,
}

/// Steps for static credential input wizard
#[derive(Debug, Clone, PartialEq)]
pub enum StaticCredentialStep {
    ProfileName,
    AccessKeyId,
    SecretAccessKey,
    SessionToken, // Optional
}

/// Steps for SSO configuration input wizard
#[derive(Debug, Clone, PartialEq)]
pub enum SsoConfigStep {
    StartUrl,
    Region,
    SessionName,
}

/// Steps for defaults configuration input wizard
#[derive(Debug, Clone, PartialEq)]
pub enum DefaultsConfigStep {
    Region,
    Output,
}

/// Steps for new profile configuration input wizard
#[derive(Debug, Clone, PartialEq)]
pub enum NewProfileConfigStep {
    ProfileName,
    Region,
    Output,
}
