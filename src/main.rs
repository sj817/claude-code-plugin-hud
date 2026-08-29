//! claude-hud — a lightweight, fixed-height statusline HUD for Claude Code.
//!
//! Reads the session JSON from stdin and prints the HUD to stdout.

mod git;
mod input;
mod render;
mod theme;

use std::io::Read;
use std::path::PathBuf;

fn main() {
    let mut buf = String::new();
    let _ = std::io::stdin().read_to_string(&mut buf);

    let data: input::StatusInput = serde_json::from_str(&buf).unwrap_or_default();

    // Claude Code v2.1.153+ exports the real terminal size; fall back to 80.
    let cols = env_usize("COLUMNS").filter(|c| *c > 0).unwrap_or(80);
    let cols = drawable_cols(cols);

    // Collapse to one line when height is tight, so the built-in mode indicator
    // below the prompt stays visible. `CLAUDE_HUD_ONELINE=1` forces it.
    let force_one = matches!(
        std::env::var("CLAUDE_HUD_ONELINE").as_deref(),
        Ok("1") | Ok("true")
    );
    let auto_one = env_usize("LINES").map(|l| l < 10).unwrap_or(false);

    let out = if force_one || auto_one {
        render::render_compact(&data, cols)
    } else {
        render::render(&data, cols)
    };
    println!("{out}");
}

fn env_usize(name: &str) -> Option<usize> {
    std::env::var(name)
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok())
}

/// Columns we may actually paint.
///
/// `COLUMNS` is the whole terminal, but Claude Code insets the statusline by
/// `statusLine.padding` columns on each side and cuts anything past that with
/// an `…`. Filling all of `COLUMNS` therefore loses the right corner. Reserve
/// the inset instead; `CLAUDE_HUD_MARGIN` overrides the reservation.
fn drawable_cols(cols: usize) -> usize {
    let margin = env_usize("CLAUDE_HUD_MARGIN").unwrap_or(2 * statusline_padding());
    cols.saturating_sub(margin).max(MIN_COLS)
}

/// Never shrink below this, however wide the padding claims to be.
const MIN_COLS: usize = 20;

/// `statusLine.padding` as Claude Code resolves it: user settings first, then
/// the project's, then its local overrides — last definition wins. Defaults to
/// Claude Code's own default of 1 when nobody sets it.
fn statusline_padding() -> usize {
    let mut paths: Vec<PathBuf> = Vec::new();
    if let Some(home) = home_dir() {
        paths.push(home.join(".claude").join("settings.json"));
    }
    paths.push(PathBuf::from(".claude/settings.json"));
    paths.push(PathBuf::from(".claude/settings.local.json"));

    let mut padding = 1;
    for p in paths {
        let Ok(text) = std::fs::read_to_string(&p) else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        if let Some(n) = v
            .get("statusLine")
            .and_then(|s| s.get("padding"))
            .and_then(|p| p.as_u64())
        {
            padding = n as usize;
        }
    }
    padding
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}
