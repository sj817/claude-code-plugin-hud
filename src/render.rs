//! Builds the HUD lines.
//!
//! Layout (two lines):
//!   Line 1 — `███░ 252k/1M(25%) | +1507 -542 | 💰 $10.95 | ⏱ 1h36m | 🌿 main* ... v2.1.251 🚀 ⚡high(Opus 5)`
//!            context bar · lines changed · cost · duration · git on the left;
//!            version, fast-mode rocket, effort and model right-aligned in the corner.
//!   Line 2 — `Quota: 5h 10% 2h26m · 7d 35% 2d12h · $ 63% 20d3h | 🎉Cache: 91% 47m | 📁 Github/proj`
//!            quota usage (the focus), cache, folder (last; smart-trimmed to ~40 chars).
//!
//! Invariants: constant height; both lines are width-aware and drop their
//! lowest-priority segments before they would wrap; one progress bar only.
//! Segments join with a faint `|`; colors come from the coordinated palette in
//! `theme` so related fields read as related.

use crate::git;
use crate::input::{ContextWindow, Cost, StatusInput};
use crate::theme::{self, paint, DIM, RESET};
use std::time::{SystemTime, UNIX_EPOCH};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const BAR_WIDTH: usize = 8;
const PIPE_PLAIN: &str = " | ";

fn pipe() -> String {
    format!("{} | {RESET}", theme::SEP)
}

/// A line piece. `plain` drives width math; `rendered` is printed. Lower
/// `priority` is dropped first when the line is too wide.
struct Seg {
    plain: String,
    rendered: String,
    priority: u8,
}

impl Seg {
    fn new(plain: impl Into<String>, rendered: impl Into<String>, priority: u8) -> Self {
        Seg {
            plain: plain.into(),
            rendered: rendered.into(),
            priority,
        }
    }
    fn width(&self) -> usize {
        display_width(&self.plain)
    }
}

/// Two-line HUD.
pub fn render(data: &StatusInput, cols: usize) -> String {
    format!("{}\n{}", line1(data, cols), join_fit(line2(data), cols))
}

/// One-line HUD (line 1 only) — keeps the built-in mode indicator visible when
/// height is tight.
pub fn render_compact(data: &StatusInput, cols: usize) -> String {
    line1(data, cols)
}

// ---- line 1 ---------------------------------------------------------------

fn line1(data: &StatusInput, cols: usize) -> String {
    let cost = &data.cost;
    let mut segs: Vec<Seg> = Vec::new();

    segs.push(context_seg(data));
    if let Some(s) = lines_seg(cost) {
        segs.push(s);
    }
    if let Some(c) = cost.total_cost_usd {
        segs.push(Seg::new(
            format!("\u{1f4b0} ${c:.2}"),
            format!("\u{1f4b0} {}${c:.2}{RESET}", theme::COST),
            60,
        ));
    }
    if let Some(ms) = cost.total_duration_ms {
        let d = fmt_duration(ms);
        segs.push(Seg::new(
            format!("\u{23f1} {d}"),
            format!("\u{23f1} {}{d}{RESET}", theme::TIME),
            45,
        ));
    }
    if let Some(g) = git::status(&data.workspace.current_dir, &data.session_id) {
        let star = if g.dirty { "*" } else { "" };
        let star_r = if g.dirty {
            format!("{}*{RESET}", theme::DEL)
        } else {
            String::new()
        };
        segs.push(Seg::new(
            format!("\u{1f33f} {}{star}", g.branch),
            format!("\u{1f33f} {}{}{RESET}{star_r}", theme::BRANCH, g.branch),
            80,
        ));
    }

    // Right corner: version · effort(model).
    let right = right_corner(data);
    let rw = strip_width(&right);
    if right.is_empty() {
        return join_fit(segs, cols);
    }
    let budget = cols.saturating_sub(rw + 1);
    let left = join_fit(segs, budget);
    let lw = strip_width(&left);
    if lw + rw < cols {
        format!("{left}{}{right}", " ".repeat(cols - lw - rw))
    } else {
        format!("{left} {right}")
    }
}

/// `v2.1.251 🚀 ⚡high(Opus 5)` — version gray, a rocket while fast mode is on,
/// effort orange, model teal.
fn right_corner(data: &StatusInput) -> String {
    let model = compact_model(&data.model.display_name);
    let mut parts = Vec::new();
    if !data.version.is_empty() {
        parts.push(format!("{}v{}{RESET}", theme::VERSION, data.version));
    }
    if data.fast_mode {
        parts.push("\u{1f680}".to_string());
    }
    match data.effort.as_ref().filter(|e| !e.level.is_empty()) {
        Some(e) => parts.push(format!(
            "{}\u{26a1}{}{RESET}{m}({model}){RESET}",
            theme::EFFORT,
            e.level,
            m = theme::MODEL
        )),
        None => parts.push(paint(theme::MODEL, &model)),
    }
    parts.join(" ")
}

/// The single progress bar: `███░ 252k/1M(25%)`.
fn context_seg(data: &StatusInput) -> Seg {
    let cw = &data.context_window;
    let total = fmt_tokens(cw.context_window_size.unwrap_or(200_000));
    match cw.used_percentage {
        Some(pct) => {
            let pct = pct.clamp(0.0, 100.0);
            let filled = (((pct / 100.0) * BAR_WIDTH as f64).round() as usize).min(BAR_WIDTH);
            let bar: String = "\u{2588}".repeat(filled) + &"\u{2591}".repeat(BAR_WIDTH - filled);
            let c = theme::usage_color(pct);
            let used = fmt_tokens(cw.total_input_tokens);
            let p = pct.round() as u32;
            Seg::new(
                format!("{bar} {used}/{total}({p}%)"),
                format!(
                    "{c}{bar}{RESET} {m}{used}/{total}{RESET}({p}%)",
                    m = theme::MUTE
                ),
                100,
            )
        }
        None => {
            let bar = "\u{2591}".repeat(BAR_WIDTH);
            Seg::new(
                format!("{bar} -/{total}(-%)"),
                format!("{DIM}{bar} -/{total}(-%){RESET}"),
                100,
            )
        }
    }
}

/// `+1507 -542` — additions green, deletions red.
fn lines_seg(cost: &Cost) -> Option<Seg> {
    if cost.total_lines_added == 0 && cost.total_lines_removed == 0 {
        return None;
    }
    Some(Seg::new(
        format!("+{} -{}", cost.total_lines_added, cost.total_lines_removed),
        format!(
            "{}+{}{RESET} {}-{}{RESET}",
            theme::ADD,
            cost.total_lines_added,
            theme::DEL,
            cost.total_lines_removed
        ),
        70,
    ))
}

// ---- line 2 ---------------------------------------------------------------

fn line2(data: &StatusInput) -> Vec<Seg> {
    let mut segs = Vec::new();

    // Rate limits FIRST — this is the line's focal point (quota usage).
    if let Some(seg) = rate_seg(data) {
        segs.push(seg);
    }

    // 🎉 Cache: session hit ratio, and how long the prefix stays warm.
    if let Some(seg) = cache_seg(data) {
        segs.push(seg);
    }

    // 📁 folder LAST — smart path: show the full path if it fits the budget,
    // otherwise drop top-level components until it does (never below one).
    let dir = smart_path(&data.workspace.current_dir);
    if !dir.is_empty() {
        segs.push(Seg::new(
            format!("\u{1f4c1} {dir}"),
            format!("\u{1f4c1} {}{dir}{RESET}", theme::FOLDER),
            55,
        ));
    }

    segs
}

/// `Quota: 5h 56% 1h30m · 7d 52% 2d1h · $ 63% 20d3h` — the quota-usage focus. A
/// teal `Quota:` label leads it; the window label is bright, the % carries one
/// of four 25% bands (green/yellow/orange/red), and the reset countdown stays
/// dim. Highest priority on line 2 so it is the last thing dropped when space
/// runs out. `$` is the gateway spend limit, sent only to users who have one;
/// it is the single window whose % can read above 100.
fn rate_seg(data: &StatusInput) -> Option<Seg> {
    let rl = data.rate_limits.as_ref()?;
    let mut plain = Vec::new();
    let mut rendered = Vec::new();
    for (label, w) in [
        ("5h", rl.five_hour.as_ref()),
        ("7d", rl.seven_day.as_ref()),
        ("$", rl.spend_limit.as_ref()),
    ] {
        if let Some(w) = w {
            if let Some(p) = w.used_percentage {
                let cd = w
                    .resets_at
                    .map(|t| format!(" {}", fmt_countdown(t)))
                    .unwrap_or_default();
                plain.push(format!("{label} {:.0}%{cd}", p));
                rendered.push(format!(
                    "{wh}{label}{RESET} {lc}{:.0}%{RESET}{mu}{cd}{RESET}",
                    p,
                    wh = theme::WHITE,
                    lc = theme::limit_color(p),
                    mu = theme::MUTE
                ));
            }
        }
    }
    if plain.is_empty() {
        return None;
    }
    let dot = format!("{DIM} \u{b7} {RESET}");
    let plain_s = format!("Quota: {}", plain.join(" \u{b7} "));
    let rendered_s = format!("{q}Quota:{RESET} {}", rendered.join(&dot), q = theme::QUOTA);
    Some(Seg::new(plain_s, rendered_s, 60))
}

/// `🎉Cache: 91% 47m` — the session-wide hit ratio Claude Code computes, plus
/// the time left before the cached prefix goes cold (`cold` once it has). The
/// countdown is trustworthy: Claude Code re-runs the statusline at `expires_at`.
///
/// Claude Code older than v2.1.251 sends no `prompt_cache`, so we fall back to
/// the ratio derived from `current_usage` in the most recent response.
fn cache_seg(data: &StatusInput) -> Option<Seg> {
    let Some(pc) = data.prompt_cache.as_ref() else {
        return legacy_cache_seg(&data.context_window);
    };
    // Caching off, or a provider that does not report it: say nothing rather
    // than show a 0% that actually means "unknown".
    if !pc.caching_observed {
        return None;
    }
    let pct = (pc.hit_ratio? * 100.0).clamp(0.0, 100.0);
    let tail = if pc.warm {
        pc.expires_at
            .map(|t| format!(" {}", fmt_countdown(t)))
            .unwrap_or_default()
    } else {
        " cold".to_string()
    };
    Some(Seg::new(
        format!("\u{1f389}Cache: {pct:.0}%{tail}"),
        format!(
            "\u{1f389}{ca}Cache:{RESET} {pct:.0}%{m}{tail}{RESET}",
            ca = theme::CACHE,
            m = theme::MUTE
        ),
        40,
    ))
}

/// Pre-v2.1.251 shape: `🎉Cache: 528k/550k(99%)`, the cache share of the most
/// recent API response alone. `current_usage` is null before the first call and
/// again right after `/compact`.
fn legacy_cache_seg(cw: &ContextWindow) -> Option<Seg> {
    let cu = cw.current_usage.as_ref()?;
    let total_in = cu.input_tokens + cu.cache_creation_input_tokens + cu.cache_read_input_tokens;
    let rate = (cu.cache_read_input_tokens * 100).checked_div(total_in)?;
    let read = fmt_tokens(cu.cache_read_input_tokens);
    let tot = fmt_tokens(total_in);
    Some(Seg::new(
        format!("\u{1f389}Cache: {read}/{tot}({rate}%)"),
        format!(
            "\u{1f389}{ca}Cache:{RESET} {m}{read}/{tot}{RESET}({rate}%)",
            ca = theme::CACHE,
            m = theme::MUTE
        ),
        40,
    ))
}

// ---- shared ---------------------------------------------------------------

/// Join `segs` with the faint pipe, dropping lowest-priority segments until the
/// visible width fits `cols`.
fn join_fit(mut segs: Vec<Seg>, cols: usize) -> String {
    let sep_w = UnicodeWidthStr::width(PIPE_PLAIN);
    loop {
        let total: usize =
            segs.iter().map(|s| s.width()).sum::<usize>() + sep_w * segs.len().saturating_sub(1);
        if total <= cols || segs.len() <= 1 {
            break;
        }
        if let Some(i) = segs
            .iter()
            .enumerate()
            .min_by_key(|(_, s)| s.priority)
            .map(|(i, _)| i)
        {
            segs.remove(i);
        } else {
            break;
        }
    }
    let sep = pipe();
    segs.iter()
        .map(|s| s.rendered.as_str())
        .collect::<Vec<_>>()
        .join(&sep)
}

// ---- helpers --------------------------------------------------------------

/// `Opus 4.8 (1M context)` -> `Opus 4.8` (the 1M now shows in the bar's total).
fn compact_model(display_name: &str) -> String {
    let base = display_name
        .split('(')
        .next()
        .unwrap_or(display_name)
        .trim();
    if base.is_empty() {
        "Claude".to_string()
    } else {
        base.to_string()
    }
}

/// Max visible width of the folder path before we start trimming.
const MAX_PATH: usize = 40;

/// Smart path: show the full path when it fits `MAX_PATH`; otherwise drop
/// leading (top-level) components one at a time until it fits, never going
/// below the final component (which is shown even if it alone exceeds the cap).
fn smart_path(path: &str) -> String {
    let parts: Vec<&str> = path
        .trim_end_matches(['/', '\\'])
        .split(['/', '\\'])
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() {
        return String::new();
    }
    let mut start = 0;
    while start + 1 < parts.len() && parts[start..].join("/").chars().count() > MAX_PATH {
        start += 1;
    }
    parts[start..].join("/")
}

/// Cells a terminal actually paints for `c`.
///
/// `unicode-width` follows East_Asian_Width, which calls U+23F1 `⏱` neutral —
/// one cell — while terminals draw it as a two-cell emoji. Counting it as one
/// made line 1 a cell wider than we believed, and Claude Code cut the right
/// corner off with an `…`.
fn char_cells(c: char) -> usize {
    match c {
        '\u{23f0}'..='\u{23f3}' => 2,
        _ => UnicodeWidthChar::width(c).unwrap_or(0),
    }
}

/// Painted width of a plain (escape-free) string.
fn display_width(s: &str) -> usize {
    s.chars().map(char_cells).sum()
}

/// Visible width of a string that may contain ANSI SGR codes.
fn strip_width(s: &str) -> usize {
    let mut w = 0;
    let mut in_esc = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_esc = true;
        } else if in_esc {
            if c == 'm' {
                in_esc = false;
            }
        } else {
            w += char_cells(c);
        }
    }
    w
}

/// Whole minutes only, no seconds: `1h50m`, `45m`.
fn fmt_duration(ms: u64) -> String {
    let mins = ms / 60_000;
    let (h, m) = (mins / 60, mins % 60);
    if h > 0 {
        format!("{h}h{m}m")
    } else {
        format!("{m}m")
    }
}

/// `512`, `10.1k`, `252k`, `1M`, `1.2M`.
fn fmt_tokens(n: u64) -> String {
    if n < 1000 {
        n.to_string()
    } else if n < 1_000_000 {
        let k = n as f64 / 1000.0;
        if k < 100.0 {
            format!("{k:.1}k")
        } else {
            format!("{:.0}k", k)
        }
    } else {
        let m = n as f64 / 1_000_000.0;
        if (m - m.round()).abs() < 1e-9 {
            format!("{}M", m.round() as u64)
        } else {
            format!("{m:.1}M")
        }
    }
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// `resets_at` (epoch secs) -> `2d12h` / `2h26m` / `8m` / `now`.
fn fmt_countdown(resets_at: i64) -> String {
    let secs = resets_at - now_unix();
    if secs <= 0 {
        return "now".to_string();
    }
    let (d, h, m) = (secs / 86400, (secs % 86400) / 3600, (secs % 3600) / 60);
    if d > 0 {
        format!("{d}d{h}h")
    } else if h > 0 {
        format!("{h}h{m}m")
    } else {
        format!("{m}m")
    }
}
