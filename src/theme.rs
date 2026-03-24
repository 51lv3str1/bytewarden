/// theme.rs — Theme system for bytewarden
///
/// Reads [theme] section from ~/.config/bytewarden/config.toml
/// Falls back to built-in defaults if section is missing.
///
/// Config format:
///   [theme]
///   accent          = "#00d4d4"
///   inactive        = "#8c8ca0"
///   selected_bg     = "#1e3c50"
///   success         = "#00c896"
///   error           = "#e05060"
///   dim             = "#888888"
///   item_login      = "#5b8fff"
///   item_card       = "#c060e0"
///   item_identity   = "#e0b840"
///   item_note       = "#00c896"
///   item_ssh        = "#a060e0"
///   item_favorite   = "#ffc800"

use ratatui::style::Color;

#[derive(Debug, Clone)]
pub struct Theme {
    pub accent:         Color,
    pub inactive:       Color,
    pub selected_bg:    Color,
    pub success:        Color,
    pub error:          Color,
    pub dim:            Color,
    pub item_login:     Color,
    pub item_card:      Color,
    pub item_identity:  Color,
    pub item_note:      Color,
    pub item_ssh:       Color,
    pub item_favorite:  Color,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            accent:        Color::Cyan,
            inactive:      Color::Rgb(140, 140, 160),
            selected_bg:   Color::Rgb(30, 60, 80),
            success:       Color::Green,
            error:         Color::Red,
            dim:           Color::DarkGray,
            item_login:    Color::Rgb(91, 143, 255),
            item_card:     Color::Rgb(192, 96, 224),
            item_identity: Color::Rgb(224, 184, 64),
            item_note:     Color::Rgb(0, 200, 150),
            item_ssh:      Color::Rgb(160, 96, 224),
            item_favorite: Color::Rgb(255, 200, 0),
        }
    }
}

/// Built-in Catppuccin Macchiato theme.
/// Colors from https://github.com/catppuccin/catppuccin
pub fn catppuccin() -> Theme {
    Theme {
        accent:        hex("#cba6f7"), // Mauve
        inactive:      hex("#a6adc8"), // Subtext0
        selected_bg:   hex("#313244"), // Surface0
        success:       hex("#a6e3a1"), // Green
        error:         hex("#f38ba8"), // Red
        dim:           hex("#585b70"), // Surface2
        item_login:    hex("#89b4fa"), // Blue
        item_card:     hex("#cba6f7"), // Mauve
        item_identity: hex("#f9e2af"), // Yellow
        item_note:     hex("#a6e3a1"), // Green
        item_ssh:      hex("#b4befe"), // Lavender
        item_favorite: hex("#f9e2af"), // Yellow
    }
}

/// Loads the theme from config.toml [theme] section.
/// Returns Default if file or section missing, or catppuccin if theme = "catppuccin".
pub fn load(config_dir: &std::path::Path) -> Theme {
    let file = config_dir.join("config.toml");
    let Ok(text) = std::fs::read_to_string(&file) else {
        return Theme::default();
    };

    // Check for named theme preset first
    for line in text.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("theme = ") {
            let name = val.trim().trim_matches('"').to_lowercase();
            return match name.as_str() {
                "catppuccin" | "catppuccin-macchiato" => catppuccin(),
                "default"                             => Theme::default(),
                _ => parse_theme_section(&text),
            };
        }
    }

    // No preset — try parsing [theme] section
    parse_theme_section(&text)
}

/// Parses individual color overrides from [theme] section.
/// Only overrides keys that are present; falls back to default for missing ones.
fn parse_theme_section(text: &str) -> Theme {
    let mut t = Theme::default();
    let mut in_theme = false;

    for line in text.lines() {
        let line = line.trim();
        if line == "[theme]" { in_theme = true; continue; }
        if line.starts_with('[') { in_theme = false; continue; }
        if !in_theme { continue; }
        if let Some((key, rest)) = line.split_once('=') {
            let key = key.trim();
            // Extract value between first pair of quotes: accent = "#00d4d4"  # comment
            // If quoted: extract content between quotes.
            // If unquoted: take the first word before any whitespace/comment.
            let val = rest.trim();
            let val = if val.starts_with('"') {
                // Quoted value — extract between quotes
                val.trim_start_matches('"')
                   .splitn(2, '"').next().unwrap_or("").trim()
            } else {
                // Unquoted — take up to first whitespace
                val.splitn(2, ' ').next().unwrap_or("").trim()
            };
            if val.len() != 7 || !val.starts_with('#') { continue; }
            let color = hex(val);
            match key {
                "accent"        => t.accent        = color,
                "inactive"      => t.inactive       = color,
                "selected_bg"   => t.selected_bg    = color,
                "success"       => t.success        = color,
                "error"         => t.error          = color,
                "dim"           => t.dim            = color,
                "item_login"    => t.item_login      = color,
                "item_card"     => t.item_card       = color,
                "item_identity" => t.item_identity   = color,
                "item_note"     => t.item_note       = color,
                "item_ssh"      => t.item_ssh        = color,
                "item_favorite" => t.item_favorite   = color,
                _ => {}
            }
        }
    }
    t
}

/// Parses a hex color string like "#cba6f7" into a ratatui Color::Rgb.
/// Returns Color::Reset on parse error.
fn hex(s: &str) -> Color {
    let s = s.trim_start_matches('#');
    if s.len() != 6 { return Color::Reset; }
    let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0);
    Color::Rgb(r, g, b)
}
