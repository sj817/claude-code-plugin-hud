//! Classic palette: standard 16-color ANSI (so it follows the user's terminal
//! theme instead of fighting it), with one deliberate exception —
//!   • CACHE: a 256-color pink the user likes for the `🎉Cache:` label.
//!
//! Most text stays the terminal default; we color only the few accents that
//! carry meaning. No bold, no saturated shades.

pub const RESET: &str = "\x1b[0m";
pub const DIM: &str = "\x1b[2m";

// Standard accents (theme-adaptive)
pub const MODEL: &str = "\x1b[36m"; // cyan
pub const VERSION: &str = "\x1b[2m"; // dim
pub const EFFORT: &str = "\x1b[33m"; // yellow
pub const BRANCH: &str = "\x1b[34m"; // blue
pub const COST: &str = "\x1b[33m"; // yellow
pub const FOLDER: &str = "\x1b[38;5;179m"; // soft gold (muted, not bright)
pub const TIME: &str = ""; // default color
pub const MUTE: &str = "\x1b[2m"; // dim units / secondary
pub const SEP: &str = "\x1b[2m"; // dim separators
pub const WHITE: &str = "\x1b[97m"; // bright white
pub const ADD: &str = "\x1b[32m"; // green
pub const DEL: &str = "\x1b[31m"; // red

// Deliberate non-standard accents
pub const CACHE: &str = "\x1b[38;5;212m"; // pink (kept by request)
pub const ORANGE: &str = "\x1b[38;5;208m"; // 4th quota band, between yellow and red
pub const QUOTA: &str = "\x1b[38;5;80m"; // teal label for the quota segment (line 2 focus)

/// Wrap `text` in `color` + reset. No-op when `color` is empty.
pub fn paint(color: &str, text: &str) -> String {
    if color.is_empty() {
        text.to_string()
    } else {
        format!("{color}{text}{RESET}")
    }
}

/// Pressure color for a 0-100 value: green < 70, yellow < 90, else red.
/// (High = bad, e.g. context usage.)
pub fn usage_color(pct: f64) -> &'static str {
    if pct >= 90.0 {
        "\x1b[31m"
    } else if pct >= 70.0 {
        "\x1b[33m"
    } else {
        "\x1b[32m"
    }
}

/// Quota-usage color in four 25% bands (low = good, high = bad):
/// green < 25, yellow < 50, orange < 75, else red.
pub fn limit_color(pct: f64) -> &'static str {
    if pct >= 75.0 {
        "\x1b[31m" // red
    } else if pct >= 50.0 {
        ORANGE
    } else if pct >= 25.0 {
        "\x1b[33m" // yellow
    } else {
        "\x1b[32m" // green
    }
}

/// Model and saturated metric accents. Supporting text reuses classic ANSI styles.
pub mod ribbon {
    pub type Color = (u8, u8, u8);
    pub const MODEL_BG: Color = (160, 172, 245);
    pub const MODEL_TEXT: Color = (21, 28, 49);
    pub const TRACK: Color = (75, 89, 116);
    pub const BRANCH: Color = (0, 191, 255);
    pub const TEAL: Color = (94, 234, 212);
    pub const YELLOW: Color = (250, 204, 21);
    pub const ORANGE: Color = (251, 146, 60);
    pub const RED: Color = (248, 113, 113);
    pub const COLD: Color = (251, 113, 133);
    pub const PATH: Color = (102, 224, 203);

    pub fn foreground((r, g, b): Color) -> String {
        format!("\x1b[38;2;{r};{g};{b}m")
    }
    pub fn background((r, g, b): Color) -> String {
        format!("\x1b[48;2;{r};{g};{b}m")
    }
    pub fn context_color(percent: f64) -> Color {
        if percent >= 90.0 {
            (255, 66, 85)
        } else if percent >= 70.0 {
            (255, 145, 0)
        } else if percent >= 50.0 {
            (255, 212, 0)
        } else if percent >= 25.0 {
            (48, 222, 115)
        } else {
            (0, 195, 255)
        }
    }
    pub fn quota_color(percent: f64) -> Color {
        if percent >= 90.0 {
            RED
        } else if percent >= 75.0 {
            ORANGE
        } else if percent >= 50.0 {
            YELLOW
        } else {
            TEAL
        }
    }
}
