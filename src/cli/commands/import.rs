// Import command - previously moved sections from user-managed to awsom-managed area
// This is now deprecated as section markers have been removed
use crate::error::{Result, SsoError};

pub async fn execute(_name: String, _section_type: String, _force: bool) -> Result<()> {
    Err(SsoError::ConfigError(
        "The 'import' command is no longer needed.\n\
         awsom no longer uses section markers - all profiles in your config are now \n\
         managed equally. Your existing profiles will work without any changes."
            .to_string(),
    ))
}
