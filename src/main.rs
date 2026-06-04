//! claude-hud — a lightweight, fixed-height statusline HUD for Claude Code.
//!
//! Reads the session JSON from stdin and prints the HUD to stdout.

mod git;
mod input;
mod render;
mod theme;

use std::io::Read;

fn main() {
    let mut buf = String::new();
    let _ = std::io::stdin().read_to_string(&mut buf);

    let data: input::StatusInput = serde_json::from_str(&buf).unwrap_or_default();

    // Claude Code v2.1.153+ exports the real terminal size; fall back to 80.
    let cols = env_usize("COLUMNS").filter(|c| *c > 0).unwrap_or(80);

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
