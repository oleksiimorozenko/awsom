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
    /// View EC2 instance tags
    ViewInstanceTags { tags: Vec<(String, String)> },
}

/// Steps for static credential input wizard
#[derive(Debug, Clone, PartialEq)]
pub enum StaticCredentialStep {
    ProfileName,
    AccessKeyId,
    SecretAccessKey,
    SessionToken, // Optional
    Region,       // NEW
    Output,       // NEW
}

/// Steps for SSO configuration input wizard
#[derive(Debug, Clone, PartialEq)]
pub enum SsoConfigStep {
    StartUrl,
    Region,
    SessionName,
}

/// Steps for new profile configuration input wizard
#[derive(Debug, Clone, PartialEq)]
pub enum NewProfileConfigStep {
    ProfileName,
    Region,
    Output,
}
