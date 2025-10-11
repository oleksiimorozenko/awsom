// Expiry tracking and notifications
use chrono::{DateTime, Utc};

pub fn format_time_remaining(expires_at: &DateTime<Utc>) -> String {
    let now = Utc::now();
    if *expires_at <= now {
        return "EXPIRED".to_string();
    }

    let duration = (*expires_at - now).num_seconds();
    let hours = duration / 3600;
    let minutes = (duration % 3600) / 60;
    let seconds = duration % 60;

    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

pub fn is_expiring_soon(expires_at: &DateTime<Utc>, threshold_minutes: i64) -> bool {
    let now = Utc::now();
    let duration = (*expires_at - now).num_minutes();
    duration > 0 && duration < threshold_minutes
}
