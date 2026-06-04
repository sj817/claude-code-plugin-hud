# claude-code-plugin-hud

A clean, two-line statusline for [Claude Code](https://code.claude.com/docs/en/statusline) — your model, context budget, cost, git, and rate limits, always visible and never in the way.

English · [简体中文](./README.zh-CN.md)

![demo](./demo.png)

---

## Quick start

```text
/plugin marketplace add sj817/claude-code-plugin-hud
/plugin install claude-code-plugin-hud
/claude-code-plugin-hud:setup
```

That's it. `:setup` picks the right prebuilt binary for your OS and writes the
statusline into your settings. Your next message shows the HUD.

> Plugin commands are namespaced, hence the `claude-code-plugin-hud:` prefix.

---

## What you're looking at

```
██░░░░░░ 397k/1M(40%) | +2318 -922 | 💰 $27.62 | ⏱ 3h4m | 🌿 main*    v2.1.161 ⚡high(Opus 4.8)
🎉Cache: 396k/397k(99%) | 5h(25% 57m) · 7d(38% 2d10h) | 📁 D:/Github/claude-code-plugin-hud
```

**Line 1 — right now**

| Segment            | Meaning                                              |
| ------------------ | ---------------------------------------------------- |
| `██░░ 397k/1M(40%)` | context window used / total (one bar, colored by load) |
| `+2318 -922`       | lines added / removed this session                   |
| `💰 $27.62`         | session cost                                         |
| `⏱ 3h4m`           | wall-clock time since the session started            |
| `🌿 main*`          | git branch (`*` = uncommitted changes)               |
| `v2.1.161 ⚡high(Opus 4.8)` | Claude Code version · effort · model (pinned right) |

**Line 2 — session & limits**

| Segment                  | Meaning                                          |
| ------------------------ | ------------------------------------------------ |
| `🎉Cache: 396k/397k(99%)` | prompt-cache hit (high is good, shown green)     |
| `5h(25% 57m)`            | 5-hour rate limit used, with reset countdown     |
| `7d(38% 2d10h)`          | 7-day rate limit used, with reset countdown      |
| `📁 …`                    | working directory (smart-trimmed when long)      |

Anything Claude Code doesn't provide yet (e.g. rate limits before your first
message) is simply omitted — the layout stays put.

---

## Why you might like it

- **Stays two lines, always.** It never balloons or jumps, so it won't push your
  prompt around.
- **Fits your terminal.** Reads `$COLUMNS` and trims the least important segments
  first instead of wrapping.
- **One progress bar.** Only the context window gets a bar; the rest is quick text.
- **Smart path.** Shows the full folder path when it fits, trims leading parts
  when it doesn't.
- **Calm colors.** Standard ANSI that follows your terminal theme — context goes
  green → yellow → red as it fills; cache hit-rate is green when healthy.
- **Tiny & fast.** A single ~310 KB Rust binary, no runtime dependencies.

---

## Configuration

Optional — it works out of the box.

| Env var               | Effect                                                |
| --------------------- | ----------------------------------------------------- |
| `CLAUDE_HUD_ONELINE`  | `1` renders a single line (frees the bottom mode row). |

`$COLUMNS` / `$LINES` are provided by Claude Code (v2.1.153+); when the terminal
is very short the HUD auto-collapses to one line.

---

## Manual setup

If you'd rather not use `:setup`, point your statusline at the binary directly:

```jsonc
// ~/.claude/settings.json
{
  "statusLine": {
    "type": "command",
    "command": "/absolute/path/to/claude-hud",   // forward slashes on Windows
    "padding": 2
  }
}
```

On Claude Code older than v2.1.153 (which doesn't export `$COLUMNS`), wrap it to
read the width from the tty:

```jsonc
"command": "cols=$(stty size </dev/tty 2>/dev/null | awk '{print $2}'); export COLUMNS=${cols:-120}; exec /absolute/path/to/claude-hud"
```

---

## Build from source

```bash
cargo build --release        # -> target/release/claude-hud
```

Try it with mock input:

```bash
echo '{"model":{"display_name":"Opus 4.8 (1M context)"},"context_window":{"used_percentage":25,"total_input_tokens":50000,"context_window_size":200000},"session_id":"x"}' \
  | COLUMNS=120 ./target/release/claude-hud
```

Cross-compile every shipped platform into `dist/<triple>/`:

```bash
scripts/build-all.sh
```

---

## Releases

Push a version tag and CI does the rest — builds Windows / macOS (x64 + arm64) /
Linux (x64 + arm64), commits the binaries into `dist/`, and publishes a GitHub
Release:

```bash
git tag v0.1.0 && git push origin v0.1.0
```

---

## License

[MIT](./LICENSE)
