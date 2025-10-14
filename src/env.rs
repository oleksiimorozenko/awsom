// Environment detection utilities

/// Check if we're running in a headless environment
///
/// Headless mode is detected when:
/// - SSH_TTY or SSH_CONNECTION environment variables are set (SSH session)
/// - TERM is set to "dumb" or is empty
/// - On Linux: DISPLAY environment variable is not set (no X11)
/// - CI environment is detected
///
/// Note: macOS doesn't use DISPLAY, so we don't check it on Darwin
///
/// Returns true if running in headless mode
pub fn is_headless_environment() -> bool {
    // Check SSH session first (most reliable indicator)
    if std::env::var("SSH_TTY").is_ok() {
        tracing::debug!("Headless detected: SSH_TTY set");
        return true;
    }

    if std::env::var("SSH_CONNECTION").is_ok() {
        tracing::debug!("Headless detected: SSH_CONNECTION set");
        return true;
    }

    // Check for CI environment
    if std::env::var("CI").is_ok() {
        tracing::debug!("Headless detected: CI environment");
        return true;
    }

    // Check TERM
    if let Ok(term) = std::env::var("TERM") {
        if term == "dumb" || term.is_empty() {
            tracing::debug!("Headless detected: TERM is '{}'", term);
            return true;
        }
    }

    // On Linux (and other non-macOS Unix), check DISPLAY for X11
    // macOS doesn't use X11/DISPLAY, so skip this check on Darwin
    #[cfg(not(target_os = "macos"))]
    {
        if std::env::var("DISPLAY").is_err() {
            tracing::debug!("Headless detected: DISPLAY not set (non-macOS)");
            return true;
        }
    }

    tracing::debug!("Not headless: detected graphical environment");
    false
}
