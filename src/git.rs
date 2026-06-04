//! Git lookup (branch + dirty flag) with a short per-session cache.
//!
//! `git` calls are the slowest thing the HUD does and it runs on every
//! assistant message, so we cache keyed by `session_id` (stable per session,
//! unique across sessions — the doc's recommended key) with a few-seconds TTL.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

const CACHE_TTL: Duration = Duration::from_secs(5);

pub struct GitInfo {
    pub branch: String,
    /// Working tree has staged or unstaged changes.
    pub dirty: bool,
}

/// Current git info, or `None` when not in a repo.
pub fn status(dir: &str, session_id: &str) -> Option<GitInfo> {
    let cache = cache_path(session_id);
    if let Some(line) = read_fresh(&cache) {
        return parse(&line);
    }
    let info = run(dir);
    // Cache "branch\tdirty" (empty line for non-repo) so a non-repo dir doesn't
    // re-shell every render.
    let line = match &info {
        Some(g) => format!("{}\t{}", g.branch, if g.dirty { "1" } else { "0" }),
        None => String::new(),
    };
    let _ = fs::write(&cache, &line);
    info
}

fn parse(line: &str) -> Option<GitInfo> {
    let line = line.trim_end_matches('\n');
    if line.is_empty() {
        return None;
    }
    let mut it = line.splitn(2, '\t');
    let branch = it.next().unwrap_or("").to_string();
    let dirty = it.next() == Some("1");
    if branch.is_empty() {
        None
    } else {
        Some(GitInfo { branch, dirty })
    }
}

fn cache_path(session_id: &str) -> PathBuf {
    let key = if session_id.is_empty() {
        "default"
    } else {
        session_id
    };
    std::env::temp_dir().join(format!("claude-hud-git-{key}"))
}

fn read_fresh(path: &PathBuf) -> Option<String> {
    let meta = fs::metadata(path).ok()?;
    let age = meta.modified().ok()?.elapsed().unwrap_or(CACHE_TTL);
    if age <= CACHE_TTL {
        fs::read_to_string(path).ok()
    } else {
        None
    }
}

fn run(dir: &str) -> Option<GitInfo> {
    let branch = git(dir, &["branch", "--show-current"])?;
    let branch = branch.trim().to_string();
    if branch.is_empty() {
        return None;
    }
    let dirty = git(dir, &["status", "--porcelain"])
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    Some(GitInfo { branch, dirty })
}

fn git(dir: &str, args: &[&str]) -> Option<String> {
    let mut cmd = Command::new("git");
    if !dir.is_empty() {
        cmd.arg("-C").arg(dir);
    }
    let out = cmd.args(args).output().ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        None
    }
}
