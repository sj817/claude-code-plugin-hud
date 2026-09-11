//! Git lookup (branch + dirty flag) with a short per-session cache.
//!
//! `git` calls are the slowest thing the HUD does and it runs on every
//! assistant message, so we cache keyed by `session_id` (stable per session,
//! unique across sessions — the doc's recommended key) with a few-seconds TTL.

use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
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

/// The origin's GitHub repository URL, independent of the current branch.
///
/// Only recognized GitHub remote forms become terminal hyperlinks. Cache both
/// successful lookups and missing/unsupported remotes, scoped to the directory
/// as well as the session so changing workspaces cannot reuse another URL.
pub fn repository_url(dir: &str, session_id: &str) -> Option<String> {
    let cache = repository_cache_path(dir, session_id);
    if let Some(url) = read_fresh(&cache) {
        // Validate cached data too: it is read from a shared temporary directory
        // and must never introduce terminal controls or an arbitrary URL.
        return url
            .strip_prefix("https://github.com/")
            .and_then(github_url_from_path);
    }
    let url = git(dir, &["remote", "get-url", "origin"])
        .and_then(|remote| github_repository_url(remote.trim_end_matches(['\r', '\n'])));
    let _ = fs::write(cache, url.as_deref().unwrap_or_default());
    url
}

fn github_repository_url(remote: &str) -> Option<String> {
    let path = remote
        .strip_prefix("https://github.com/")
        .or_else(|| remote.strip_prefix("git@github.com:"))
        .or_else(|| remote.strip_prefix("ssh://git@github.com/"))?;
    let path = path.strip_suffix('/').unwrap_or(path);
    let path = path.strip_suffix(".git").unwrap_or(path);
    github_url_from_path(path)
}

fn github_url_from_path(path: &str) -> Option<String> {
    let (owner, repository) = path.split_once('/')?;
    if owner.is_empty()
        || !owner
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        || repository.is_empty()
        || matches!(repository, "." | "..")
        || !repository
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return None;
    }
    Some(format!("https://github.com/{owner}/{repository}"))
}

fn repository_cache_path(dir: &str, session_id: &str) -> PathBuf {
    let directory = if dir.is_empty() {
        std::env::current_dir().unwrap_or_default()
    } else {
        PathBuf::from(dir)
    };
    let mut key = DefaultHasher::new();
    directory.hash(&mut key);
    session_id.hash(&mut key);
    std::env::temp_dir().join(format!("claude-hud-repository-{:016x}", key.finish()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_remote_forms_become_canonical_repository_urls() {
        for remote in [
            "https://github.com/sj817/claude-code-plugin-hud.git",
            "https://github.com/sj817/claude-code-plugin-hud",
            "https://github.com/sj817/claude-code-plugin-hud.git/",
            "git@github.com:sj817/claude-code-plugin-hud.git",
            "ssh://git@github.com/sj817/claude-code-plugin-hud.git",
        ] {
            assert_eq!(
                github_repository_url(remote).as_deref(),
                Some("https://github.com/sj817/claude-code-plugin-hud"),
                "{remote}"
            );
        }
        assert_eq!(
            github_repository_url("https://github.com/owner-name/repo_name.v2").as_deref(),
            Some("https://github.com/owner-name/repo_name.v2")
        );
        assert_eq!(
            github_repository_url("git@github.com:owner/repo.git.git").as_deref(),
            Some("https://github.com/owner/repo.git")
        );
        assert_eq!(
            github_url_from_path("owner/repo.git").as_deref(),
            Some("https://github.com/owner/repo.git")
        );
    }

    #[test]
    fn unsupported_or_unsafe_remotes_do_not_become_hyperlinks() {
        for remote in [
            "",
            "https://gitlab.com/owner/repo.git",
            "https://github.com.example.com/owner/repo.git",
            "https://token@github.com/owner/repo.git",
            "ssh://git@github.com.evil/owner/repo.git",
            "git@github.com:/owner/repo.git",
            "https://github.com/owner/repo/tree/main",
            "https://github.com/owner/repo?token=secret",
            "https://github.com/owner/repo#fragment",
            "https://github.com/owner/repo\u{1b}]8;;https://evil.example\u{7}",
            "https://github.com/owner/repo\n",
            "https://github.com/owner/re po",
            "https://github.com/owner/repo%0a",
            "https://github.com/owner/..",
            "https://github.com//repo",
            "https://github.com/owner/.git",
            "D:/Github/local-repo",
        ] {
            assert!(github_repository_url(remote).is_none(), "{remote:?}");
        }
    }

    #[test]
    fn repository_cache_is_scoped_to_directory_and_session() {
        let first = repository_cache_path("D:/Github/first", "session-a");
        assert_eq!(first, repository_cache_path("D:/Github/first", "session-a"));
        assert_ne!(
            first,
            repository_cache_path("D:/Github/second", "session-a")
        );
        assert_ne!(first, repository_cache_path("D:/Github/first", "session-b"));
        assert_ne!(first, cache_path("session-a"));
        assert_eq!(
            repository_cache_path("D:/Github/first", "../../nested/session").parent(),
            Some(std::env::temp_dir().as_path())
        );
    }
}
