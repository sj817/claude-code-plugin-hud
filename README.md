# claude-code-plugin-hud

A two-line statusline for [Claude Code](https://code.claude.com/docs/en/statusline), written in Rust. Shows your model, context window, cost, git branch, and rate limits.

English · [简体中文](./README.zh-CN.md)

![demo](./demo.png)

## Install

### Plugin

```text
/plugin marketplace add sj817/claude-code-plugin-hud
/plugin install claude-code-plugin-hud
/reload-plugins
/claude-code-plugin-hud:setup
```

`/reload-plugins` activates the freshly installed command (or restart Claude Code). `/claude-code-plugin-hud:setup` then picks the prebuilt binary for your OS and writes the `statusLine` entry into your `settings.json`. The HUD appears on your next message.

Plugins cannot set `statusLine` directly, so a command does it. Plugin commands are namespaced, hence the `claude-code-plugin-hud:` prefix.

### Script

No plugin needed. This downloads only the binary for your platform and writes the `statusLine` for you (any existing `settings.json` is backed up first):

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/sj817/claude-code-plugin-hud/main/scripts/install.sh | bash
```

```powershell
# Windows
irm https://raw.githubusercontent.com/sj817/claude-code-plugin-hud/main/scripts/install.ps1 | iex
```

It installs to `~/.claude/statusline/`; re-run any time to update. The log follows your system language (Chinese or English). Optional overrides: `CLAUDE_HUD_VERSION` pins a release (e.g. `v0.1.6`), `CLAUDE_HUD_LANG` forces the language (`zh` / `en`).

## Anatomy

```text
███░░░░░ 397k/1M(40%) | +2318 -922 | 💰 $27.62 | ⏱ 3h4m | 🌿 main*    v2.1.251 🚀 ⚡high(Opus 5)
Quota: 5h 25% 57m · 7d 38% 2d10h | 🎉Cache: 99% 47m | 📁 D:/Github/claude-code-plugin-hud
```

### Line 1

| Segment | Example | Meaning |
| --- | --- | --- |
| Context bar | `███░░░░░ 397k/1M(40%)` | Context window used / total. The single progress bar. Green below 70%, yellow below 90%, red at 90%+. |
| Lines changed | `+2318 -922` | Lines added / removed this session. |
| Cost | `💰 $27.62` | Session cost in USD. |
| Duration | `⏱ 3h4m` | Wall-clock time since the session started. |
| Branch | `🌿 main*` | Git branch. A trailing `*` marks a dirty working tree. |
| Version · effort(model) | `v2.1.251 🚀 ⚡high(Opus 5)` | Final width-aware segment: Claude Code version, a 🚀 while fast mode is on, reasoning effort, model. |

### Line 2

| Segment | Example | Meaning |
| --- | --- | --- |
| Rate limits | `Quota: 5h 25% 57m · 7d 38% 2d10h` | Plan-quota usage per window, with reset countdown. The focal point of line 2: the `%` is colored in four 25% bands — green `<25`, yellow `<50`, orange `<75`, red `≥75`. A third window `$ 63% 20d3h` joins them when a gateway spend limit applies to you (Claude Code v2.1.251+); it is the one window whose `%` can read above 100. |
| Cache | `🎉Cache: 99% 47m` | Prompt-cache hit rate for the session, and how long the cached prefix stays warm (`cold` once it has expired). Older Claude Code builds send no cache statistics, so the HUD falls back to `396k/397k(99%)`, the cache share of the most recent response. |
| Folder | `📁 D:/Github/...` | Working directory, trimmed from the front when long (never below the final component). |

Notes:

- Height is always two lines. Segments never wrap; the lowest-priority ones drop when a line exceeds `$COLUMNS`.
- Missing data is omitted, not padded. Rate limits are absent before the first API response; the cache segment is absent until then, and while prompt caching is off.
- Colors stay on the standard 16-color ANSI palette, plus a pink `Cache:` label, a soft-gold folder, and an orange band in the quota scale. The `%` on the bar and cache are left plain.
- Permission mode (auto/plan) is not shown: it is not present in the statusline JSON.

## Configuration

Setup and the installers set `statusLine.refreshInterval` to `30` seconds so
cache and quota countdowns keep updating while the conversation is idle.
Existing users should re-run setup/the installer or add `"refreshInterval": 30`
to their `statusLine` settings; updating only the binary is not enough.

`CLAUDE_HUD_ONELINE` renders line 1 only, which keeps Claude Code's mode row visible below the prompt. Set it to `1` (or `true`):

```text
CLAUDE_HUD_ONELINE=1
```

The HUD also collapses to one line when `$LINES` is below 10. `$COLUMNS` and `$LINES` come from Claude Code v2.1.153+; width falls back to 80 when absent.

Claude Code already gives the status line built-in horizontal spacing, so the installer leaves the optional `statusLine.padding` at `0`. The HUD also reserves four columns inside `$COLUMNS` for the built-in gutters and the notification area that shares this row. `CLAUDE_HUD_MARGIN` overrides that safety margin if your terminal needs a different value:

```text
CLAUDE_HUD_MARGIN=4
```

## Manual setup

To skip `:setup`, point `statusLine` at the binary directly:

```jsonc
// ~/.claude/settings.json
{
  "statusLine": {
    "type": "command",
    "command": "/absolute/path/to/claude-hud",
    "padding": 0,
    "refreshInterval": 30
  }
}
```

On Windows, write the path with forward slashes. On Claude Code older than v2.1.153 (no `$COLUMNS`), read the width from the tty with an `stty` shim:

```jsonc
"command": "cols=$(stty size </dev/tty 2>/dev/null | awk '{print $2}'); export COLUMNS=${cols:-120}; exec /absolute/path/to/claude-hud"
```

## Build from source

```bash
cargo build --release        # -> target/release/claude-hud
```

Test with mock input:

```bash
echo '{"model":{"display_name":"Opus 4.8 (1M context)"},"context_window":{"used_percentage":25,"total_input_tokens":50000,"context_window_size":200000},"session_id":"x"}' \
  | COLUMNS=120 ./target/release/claude-hud
```

Cross-compile every shipped target into `dist/<triple>/`:

```bash
scripts/build-all.sh
```

The binary is small (roughly 290 KB to 545 KB depending on platform) and has no runtime dependencies. Shipped targets: Windows x64 and arm64, macOS x64 and arm64, Linux glibc x64 and arm64, Linux musl x64 and arm64.

## Releases

Push a `v*` tag. CI builds every target, commits the binaries into `dist/`, and publishes a GitHub Release:

```bash
git tag v0.1.0 && git push origin v0.1.0
```

## License

[MIT](./LICENSE)
