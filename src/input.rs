//! Typed view over the JSON that Claude Code streams to the statusline on stdin.
//!
//! Every field is optional: the docs are explicit that many fields are absent
//! (PR, worktree, rate_limits) or `null` early in a session (`current_usage`,
//! `used_percentage` before the first API response, and again right after
//! `/compact`). We never unwrap — missing data degrades to a placeholder so the
//! HUD height stays constant.
//!
//! Some Claude Code-compatible gateways serialize numeric values as strings or
//! omit object-valued fields by sending `null`. The tolerant deserializers below
//! keep one provider-specific field from discarding the rest of the payload.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

#[derive(Debug, Default, Deserialize)]
pub struct StatusInput {
    #[serde(default, deserialize_with = "deserialize_model")]
    pub model: Model,
    #[serde(default, deserialize_with = "deserialize_default")]
    pub workspace: Workspace,
    /// Older Claude Code-compatible clients provide this top-level alias but
    /// omit `workspace.current_dir`.
    #[serde(default, deserialize_with = "deserialize_string")]
    pub cwd: String,
    #[serde(default, deserialize_with = "deserialize_default")]
    pub context_window: ContextWindow,
    #[serde(default, deserialize_with = "deserialize_default")]
    pub cost: Cost,
    /// Absent unless the user is on a Claude.ai Pro/Max plan and at least one
    /// API response has completed this session.
    #[serde(default, deserialize_with = "deserialize_option_tolerant")]
    pub rate_limits: Option<RateLimits>,
    /// Current reasoning effort. Absent when the model has no effort parameter.
    #[serde(default, deserialize_with = "deserialize_option_tolerant")]
    pub effort: Option<Effort>,
    /// Prompt-cache statistics for the main conversation. Requires Claude Code
    /// v2.1.251+, and absent until the conversation's first API response.
    #[serde(default, deserialize_with = "deserialize_option_tolerant")]
    pub prompt_cache: Option<PromptCache>,
    /// Whether fast mode is on for this session.
    #[serde(default)]
    pub fast_mode: bool,
    /// Claude Code version, e.g. `2.1.161`.
    #[serde(default, deserialize_with = "deserialize_string")]
    pub version: String,
    /// Stable for the session's lifetime and unique per session — the doc's
    /// recommended cache key (process ids change on every invocation).
    #[serde(default, deserialize_with = "deserialize_string")]
    pub session_id: String,
}

impl StatusInput {
    /// Return the working directory from either generation of the statusline
    /// payload. `workspace.current_dir` is preferred when both are present.
    pub fn current_dir(&self) -> &str {
        if self.workspace.current_dir.is_empty() {
            &self.cwd
        } else {
            &self.workspace.current_dir
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct Effort {
    #[serde(default, deserialize_with = "deserialize_string")]
    pub level: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct Model {
    #[serde(default, deserialize_with = "deserialize_string")]
    pub display_name: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct Workspace {
    /// Preferred over the top-level `cwd`; the two carry the same value.
    #[serde(default, deserialize_with = "deserialize_string")]
    pub current_dir: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct ContextWindow {
    /// Pre-computed by Claude Code from input tokens only. May be `null` early
    /// in a session or right after `/compact`.
    #[serde(default, deserialize_with = "deserialize_option_f64")]
    pub used_percentage: Option<f64>,
    /// Max context window in tokens (200000, or 1000000 for extended context).
    #[serde(default, deserialize_with = "deserialize_option_u64")]
    pub context_window_size: Option<u64>,
    /// Tokens currently in the context window (input side, incl. cache). From
    /// v2.1.132+ this is the live in-window count, not a session total. `0`
    /// before the first API response.
    #[serde(default, deserialize_with = "deserialize_u64")]
    pub total_input_tokens: u64,
    /// Per-component token breakdown. `null` before the first API call and
    /// again right after `/compact`.
    #[serde(default, deserialize_with = "deserialize_option_tolerant")]
    pub current_usage: Option<CurrentUsage>,
}

#[derive(Debug, Default, Deserialize)]
pub struct CurrentUsage {
    #[serde(default, deserialize_with = "deserialize_u64")]
    pub input_tokens: u64,
    #[serde(default, deserialize_with = "deserialize_u64")]
    pub cache_creation_input_tokens: u64,
    #[serde(default, deserialize_with = "deserialize_u64")]
    pub cache_read_input_tokens: u64,
}

#[derive(Debug, Default, Deserialize)]
pub struct Cost {
    #[serde(default, deserialize_with = "deserialize_option_f64")]
    pub total_cost_usd: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_option_u64")]
    pub total_duration_ms: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_u64")]
    pub total_lines_added: u64,
    #[serde(default, deserialize_with = "deserialize_u64")]
    pub total_lines_removed: u64,
}

#[derive(Debug, Default, Deserialize)]
pub struct RateLimits {
    #[serde(default, deserialize_with = "deserialize_option_tolerant")]
    pub five_hour: Option<RateWindow>,
    #[serde(default, deserialize_with = "deserialize_option_tolerant")]
    pub seven_day: Option<RateWindow>,
    /// Spend limit applied behind a Claude apps gateway. Requires Claude Code
    /// v2.1.251+ and absent for everyone else. Unlike the other two windows its
    /// `used_percentage` can exceed 100 once the limit is passed.
    #[serde(default, deserialize_with = "deserialize_option_tolerant")]
    pub spend_limit: Option<RateWindow>,
}

#[derive(Debug, Default, Deserialize)]
pub struct RateWindow {
    #[serde(default, deserialize_with = "deserialize_option_f64")]
    pub used_percentage: Option<f64>,
    /// Unix epoch seconds when this window resets.
    #[serde(default, deserialize_with = "deserialize_option_i64")]
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
    #[serde(default, deserialize_with = "deserialize_option_f64")]
    pub hit_ratio: Option<f64>,
    /// Epoch seconds at which the cached prefix goes cold. Compare with the
    /// current clock on every render, even if `warm` is from an older snapshot.
    #[serde(default, deserialize_with = "deserialize_option_i64")]
    pub expires_at: Option<i64>,
}

fn deserialize_value<'de, D>(deserializer: D) -> Result<Value, D::Error>
where
    D: Deserializer<'de>,
{
    Value::deserialize(deserializer)
}

/// Deserialize an object, treating `null`, a scalar, or an invalid nested
/// value as that object's default. This is intentionally field-local so valid
/// context/cost data survives an unrelated gateway extension field.
fn deserialize_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned + Default,
{
    let value = deserialize_value(deserializer)?;
    Ok(T::deserialize(value).unwrap_or_default())
}

fn deserialize_option_tolerant<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    let value = deserialize_value(deserializer)?;
    if !value.is_object() {
        Ok(None)
    } else {
        Ok(T::deserialize(value).ok())
    }
}

fn deserialize_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = deserialize_value(deserializer)?;
    Ok(match value {
        Value::String(value) => value,
        Value::Number(value) => value.to_string(),
        _ => String::new(),
    })
}

fn deserialize_model<'de, D>(deserializer: D) -> Result<Model, D::Error>
where
    D: Deserializer<'de>,
{
    let value = deserialize_value(deserializer)?;
    match value {
        Value::String(display_name) => Ok(Model { display_name }),
        value => Ok(Model::deserialize(value).unwrap_or_default()),
    }
}

fn deserialize_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(parse_u64(&deserialize_value(deserializer)?).unwrap_or_default())
}

fn deserialize_option_u64<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(parse_u64(&deserialize_value(deserializer)?))
}

fn deserialize_option_i64<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(parse_i64(&deserialize_value(deserializer)?))
}

fn deserialize_option_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(parse_f64(&deserialize_value(deserializer)?))
}

fn parse_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(value) => value.as_u64(),
        Value::String(value) => value.trim().parse().ok(),
        _ => None,
    }
}

fn parse_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Number(value) => value.as_i64(),
        Value::String(value) => value.trim().parse().ok(),
        _ => None,
    }
}

fn parse_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Number(value) => value.as_f64(),
        Value::String(value) => value.trim().parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_compatible_gateway_shapes_without_dropping_the_payload() {
        let input: StatusInput = serde_json::from_value(serde_json::json!({
            "model": "claude-opus-5",
            "cwd": "/tmp/provider-project",
            "context_window": {
                "used_percentage": "42",
                "total_input_tokens": "84000",
                "context_window_size": "200000"
            },
            "cost": {
                "total_cost_usd": "1.23",
                "total_duration_ms": "185000"
            },
            "rate_limits": null,
            "session_id": "provider-session"
        }))
        .unwrap();

        assert_eq!(input.model.display_name, "claude-opus-5");
        assert_eq!(input.current_dir(), "/tmp/provider-project");
        assert_eq!(input.context_window.used_percentage, Some(42.0));
        assert_eq!(input.context_window.total_input_tokens, 84_000);
        assert_eq!(input.cost.total_cost_usd, Some(1.23));
        assert_eq!(input.cost.total_duration_ms, Some(185_000));
    }

    #[test]
    fn prefers_workspace_directory_but_falls_back_to_cwd() {
        let input: StatusInput = serde_json::from_value(serde_json::json!({
            "cwd": "/original",
            "workspace": {"current_dir": "/current"}
        }))
        .unwrap();
        assert_eq!(input.current_dir(), "/current");

        let input: StatusInput = serde_json::from_value(serde_json::json!({
            "model": {"display_name": "Gateway model"},
            "cwd": "/original",
            "workspace": null,
            "context_window": null,
            "cost": null,
            "rate_limits": null
        }))
        .unwrap();
        assert_eq!(input.model.display_name, "Gateway model");
        assert_eq!(input.current_dir(), "/original");
    }

    #[test]
    fn ignores_malformed_optional_gateway_sections() {
        let input: StatusInput = serde_json::from_value(serde_json::json!({
            "model": {"display_name": "Opus"},
            "effort": "unsupported",
            "prompt_cache": [],
            "rate_limits": {"five_hour": "unsupported"}
        }))
        .unwrap();

        assert!(input.effort.is_none());
        assert!(input.prompt_cache.is_none());
        assert!(input.rate_limits.unwrap().five_hour.is_none());
    }
}
