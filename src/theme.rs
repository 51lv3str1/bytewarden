/// theme.rs — Theme system for bytewarden
///
/// Reads [theme] section from ~/.config/bytewarden/config.toml.
/// Falls back to built-in defaults for any key that is missing.
/// All keys are optional — override only what you want.
///
/// Config format:
///   [theme]
///   accent          = "#cba6f7"   # active borders, cursor, highlights
///   inactive        = "#6c7086"   # inactive panel borders
///   selected_bg     = "#313244"   # selected row background
///   success         = "#a6e3a1"   # success messages
///   error           = "#f38ba8"   # error messages
///   dim             = "#585b70"   # secondary text
///   item_login      = "#89b4fa"
///   item_card       = "#cba6f7"
///   item_identity   = "#f9e2af"
///   item_note       = "#a6e3a1"
///   item_ssh        = "#b4befe"
///   item_favorite   = "#f9e2af"

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

/// Loads the theme from the [theme] section of config.toml.
/// Returns Theme::default() if the file or section is missing.
/// Only keys present in the file override the default — all others
/// keep their default value, so partial configs are valid.
pub fn load(config_dir: &std::path::Path) -> Theme {
    let file = config_dir.join("config.toml");
    let Ok(text) = std::fs::read_to_string(&file) else {
        return Theme::default();
    };
    parse_theme_section(&text)
}

/// Parses individual color overrides from the [theme] section.
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
            let val = rest.trim();
            // Support both quoted ("#rrggbb") and unquoted values.
            // Inline comments after the value are ignored.
            let val = if val.starts_with('"') {
                val.trim_start_matches('"')
                   .splitn(2, '"').next().unwrap_or("").trim()
            } else {
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
