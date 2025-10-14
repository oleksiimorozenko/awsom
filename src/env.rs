// Environment detection utilities

use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag to force headless mode (set by --headless CLI flag)
static FORCE_HEADLESS: AtomicBool = AtomicBool::new(false);

/// Set headless mode override (called from main with --headless flag)
pub fn set_headless_override(headless: bool) {
    FORCE_HEADLESS.store(headless, Ordering::Relaxed);
}

/// Check if we're running in a headless environment
///
/// Headless mode is detected when:
/// - --headless CLI flag is set (highest priority)
/// - SSH_TTY or SSH_CONNECTION environment variables are set (SSH session)
/// - TERM is set to "dumb" or is empty
/// - On Linux: DISPLAY environment variable is not set (no X11)
/// - CI environment is detected
///
/// Note: macOS doesn't use DISPLAY, so we don't check it on Darwin
///
/// Returns true if running in headless mode
pub fn is_headless_environment() -> bool {
    // Check --headless flag first (highest priority)
    if FORCE_HEADLESS.load(Ordering::Relaxed) {
        tracing::debug!("Headless mode: forced by --headless flag");
        return true;
    }

    // Check SSH session (most reliable auto-detection indicator)
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
