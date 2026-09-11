//! Typed view over the JSON that Claude Code streams to the statusline on stdin.
//!
//! Every field is optional: the docs are explicit that many fields are absent
//! (PR, worktree, rate_limits) or `null` early in a session (`current_usage`,
//! `used_percentage` before the first API response, and again right after
//! `/compact`). We never unwrap — missing data degrades to a placeholder so the
//! HUD height stays constant.

use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct StatusInput {
    #[serde(default)]
    pub model: Model,
    #[serde(default)]
    pub workspace: Workspace,
    #[serde(default)]
    pub context_window: ContextWindow,
    #[serde(default)]
    pub cost: Cost,
    /// Absent unless the user is on a Claude.ai Pro/Max plan and at least one
    /// API response has completed this session.
    pub rate_limits: Option<RateLimits>,
    /// Current reasoning effort. Absent when the model has no effort parameter.
    pub effort: Option<Effort>,
    /// Prompt-cache statistics for the main conversation. Requires Claude Code
    /// v2.1.251+, and absent until the conversation's first API response.
    pub prompt_cache: Option<PromptCache>,
    /// Whether fast mode is on for this session.
    #[serde(default)]
    pub fast_mode: bool,
    /// Claude Code version, e.g. `2.1.161`.
    #[serde(default)]
    pub version: String,
    /// Stable for the session's lifetime and unique per session — the doc's
    /// recommended cache key (process ids change on every invocation).
    #[serde(default)]
    pub session_id: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct Effort {
    #[serde(default)]
    pub level: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct Model {
    #[serde(default)]
    pub display_name: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct Workspace {
    /// Preferred over the top-level `cwd`; the two carry the same value.
    #[serde(default)]
    pub current_dir: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct ContextWindow {
    /// Pre-computed by Claude Code from input tokens only. May be `null` early
    /// in a session or right after `/compact`.
    pub used_percentage: Option<f64>,
    /// Max context window in tokens (200000, or 1000000 for extended context).
    pub context_window_size: Option<u64>,
    /// Tokens currently in the context window (input side, incl. cache). From
    /// v2.1.132+ this is the live in-window count, not a session total. `0`
    /// before the first API response.
    #[serde(default)]
    pub total_input_tokens: u64,
    /// Per-component token breakdown. `null` before the first API call and
    /// again right after `/compact`.
    pub current_usage: Option<CurrentUsage>,
}

#[derive(Debug, Default, Deserialize)]
pub struct CurrentUsage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
}

#[derive(Debug, Default, Deserialize)]
pub struct Cost {
    pub total_cost_usd: Option<f64>,
    pub total_duration_ms: Option<u64>,
    #[serde(default)]
    pub total_lines_added: u64,
    #[serde(default)]
    pub total_lines_removed: u64,
}

#[derive(Debug, Default, Deserialize)]
pub struct RateLimits {
    pub five_hour: Option<RateWindow>,
    pub seven_day: Option<RateWindow>,
    /// Spend limit applied behind a Claude apps gateway. Requires Claude Code
    /// v2.1.251+ and absent for everyone else. Unlike the other two windows its
    /// `used_percentage` can exceed 100 once the limit is passed.
    pub spend_limit: Option<RateWindow>,
}
#[derive(Debug, Default, Deserialize)]
pub struct RateWindow {
    pub used_percentage: Option<f64>,
    /// Unix epoch seconds when this window resets.
    pub resets_at: Option<i64>,
}

/// Session prompt-cache statistics. Only the fields the HUD renders are
/// modelled; the object carries several more (`ttl`, `requests`, `misses`,
/// token counts) that we ignore.
#[derive(Debug, Default, Deserialize)]
pub struct PromptCache {
    /// The cached prefix is still inside its TTL. `false` once the last
    /// response reported no cache tokens, even while `caching_observed` holds.
    #[serde(default)]
    pub warm: bool,
    /// Some response this session reported cache tokens. `false` means caching
    /// is off, or the provider/gateway does not report it — we then show
    /// nothing rather than a misleading 0%.
    #[serde(default)]
    pub caching_observed: bool,
    /// Cache reads over all input tokens this session, 0.0..1.0. `null` while
    /// every one of those counts is still zero.
    pub hit_ratio: Option<f64>,
    /// Epoch seconds at which the cached prefix goes cold. Compare with the
    /// current clock on every render, even if `warm` is from an older snapshot.
    pub expires_at: Option<i64>,
}
