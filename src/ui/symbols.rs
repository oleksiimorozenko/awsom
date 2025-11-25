// Platform-independent symbol abstraction
// Supports ASCII fallback via AWSOM_ASCII_SYMBOLS environment variable

/// Returns the active status indicator symbol
pub fn status_active() -> &'static str {
    if use_ascii_symbols() {
        "[+]"
    } else {
        "🟢"
    }
}

/// Returns the inactive status indicator symbol
pub fn status_inactive() -> &'static str {
    if use_ascii_symbols() {
        "[-]"
    } else {
        "🔴"
    }
}

/// Returns the warning symbol
pub fn warning() -> &'static str {
    if use_ascii_symbols() {
        "[!]"
    } else {
        "⚠"
    }
}

/// Returns the checkmark symbol (for default profile marker)
pub fn check_mark() -> &'static str {
    if use_ascii_symbols() {
        "[x]"
    } else {
        "✓"
    }
}

/// Returns the loading spinner animation frames
pub fn spinner_frames() -> &'static [char] {
    if use_ascii_symbols() {
        &['|', '/', '-', '\\']
    } else {
        &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏']
    }
}

/// Returns the up arrow symbol
pub fn arrow_up() -> &'static str {
    if use_ascii_symbols() {
        "^"
    } else {
        "↑"
    }
}

/// Returns the down arrow symbol
pub fn arrow_down() -> &'static str {
    if use_ascii_symbols() {
        "v"
    } else {
        "↓"
    }
}

/// Returns the left arrow symbol
pub fn arrow_left() -> &'static str {
    if use_ascii_symbols() {
        "<"
    } else {
        "←"
    }
}

/// Returns the right arrow symbol
pub fn arrow_right() -> &'static str {
    if use_ascii_symbols() {
        ">"
    } else {
        "→"
    }
}

/// Check if ASCII symbols should be used instead of Unicode
/// Returns true if AWSOM_ASCII_SYMBOLS environment variable is set to "true" or "1"
fn use_ascii_symbols() -> bool {
    std::env::var("AWSOM_ASCII_SYMBOLS")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}
