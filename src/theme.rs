//! Calm palette: standard 16-color ANSI (so it follows the user's terminal
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
