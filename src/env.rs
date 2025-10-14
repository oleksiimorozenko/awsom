// Environment detection utilities

/// Check if we're running in a headless environment
///
/// Headless mode is detected when:
/// - DISPLAY environment variable is not set (no X11)
/// - SSH_TTY or SSH_CONNECTION environment variables are set (SSH session)
/// - TERM is set to "dumb" or is empty
///
/// Returns true if running in headless mode
pub fn is_headless_environment() -> bool {
    // Check DISPLAY (X11)
    if std::env::var("DISPLAY").is_err() {
        tracing::debug!("Headless detected: DISPLAY not set");
        return true;
    }

    // Check SSH session
    if std::env::var("SSH_TTY").is_ok() {
        tracing::debug!("Headless detected: SSH_TTY set");
        return true;
    }

    if std::env::var("SSH_CONNECTION").is_ok() {
        tracing::debug!("Headless detected: SSH_CONNECTION set");
        return true;
    }

    // Check TERM
    if let Ok(term) = std::env::var("TERM") {
        if term == "dumb" || term.is_empty() {
            tracing::debug!("Headless detected: TERM is '{}'", term);
            return true;
        }
    }

    tracing::debug!("Not headless: detected graphical environment");
    false
}
