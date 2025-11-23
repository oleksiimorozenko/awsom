// Theme utilities - extracted from app.rs

use ratatui::style::Color;

/// Convert Catppuccin color to Ratatui Color
pub fn catppuccin_color(color: catppuccin::Color) -> Color {
    Color::Rgb(color.rgb.r, color.rgb.g, color.rgb.b)
}
