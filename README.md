# claude-code-plugin-hud

A clean, two-line statusline for [Claude Code](https://code.claude.com/docs/en/statusline) — your model, context budget, cost, git, and rate limits, always visible and never in the way.

English · [简体中文](./README.zh-CN.md)

![demo](./demo.png)

## Install

```text
/plugin marketplace add sj817/claude-code-plugin-hud
/plugin install claude-code-plugin-hud
/reload-plugins
/claude-code-plugin-hud:setup
```

`/reload-plugins` activates the freshly installed command (or just restart Claude
Code). Then `/claude-code-plugin-hud:setup` picks the right prebuilt binary for
your OS and writes the statusline into your settings — your next message shows
the HUD.

> Plugin commands are namespaced by the plugin, hence the `claude-code-plugin-hud:` prefix.

## Reading the HUD

```
██░░░░░░ 397k/1M(40%) | +2318 -922 | 💰 $27.62 | ⏱ 3h4m | 🌿 main*    v2.1.161 ⚡high(Opus 4.8)
🎉Cache: 396k/397k(99%) | 5h(25% 57m) · 7d(38% 2d10h) | 📁 D:/Github/claude-code-plugin-hud
```

**Top line — what's happening right now**, left to right:

- `██░░ 397k/1M(40%)` — context window, used / total. The one progress bar, shifting green → yellow → red as it fills.
- `+2318 -922` — lines added and removed this session.
- `💰 $27.62` — session cost.
- `⏱ 3h4m` — wall-clock time since the session started.
- `🌿 main*` — git branch; the `*` means there are uncommitted changes.
- `v2.1.161 ⚡high(Opus 4.8)` — Claude Code version, effort level, and model, pinned to the right.

**Bottom line — session and limits:**

- `🎉Cache: 396k/397k(99%)` — prompt-cache hit rate. High is good, so it stays green.
- `5h(25% 57m)` and `7d(38% 2d10h)` — rate limits used, each with its reset countdown.
- `📁 …` — your working directory, trimmed from the front when the path is long.

Anything Claude Code hasn't reported yet (rate limits before your first message,
for example) is simply left out — the layout never shifts.

## What makes it pleasant

- **Always two lines.** It never balloons or jumps, so it won't push your prompt around.
- **Fits the terminal.** Reads `$COLUMNS` and drops the least important pieces before it would ever wrap.
- **One bar, on purpose.** Only the context window gets a progress bar; everything else is quick to read.
- **Calm, themed colors.** Standard ANSI that follows your terminal — no neon, no fighting your palette.
- **Tiny and fast.** A single ~310 KB Rust binary with no runtime dependencies.

## Configuration

It works out of the box; everything below is optional.

Set `CLAUDE_HUD_ONELINE=1` to render a single line — handy when you want to keep
the built-in mode row below the prompt visible. `$COLUMNS` and `$LINES` come from
Claude Code (v2.1.153+), and the HUD collapses to one line on a very short terminal.

## Manual setup

Prefer not to use `:setup`? Point your statusline straight at the binary:

```jsonc
// ~/.claude/settings.json
{
  "statusLine": {
    "type": "command",
    "command": "/absolute/path/to/claude-hud",
    "padding": 2
  }
}
```

On Windows, write the path with forward slashes. On Claude Code older than
v2.1.153 (no `$COLUMNS`), read the width from the tty instead:

```jsonc
"command": "cols=$(stty size </dev/tty 2>/dev/null | awk '{print $2}'); export COLUMNS=${cols:-120}; exec /absolute/path/to/claude-hud"
```

## Build from source

```bash
cargo build --release        # -> target/release/claude-hud
```

Give it a spin with mock input:

```bash
echo '{"model":{"display_name":"Opus 4.8 (1M context)"},"context_window":{"used_percentage":25,"total_input_tokens":50000,"context_window_size":200000},"session_id":"x"}' \
  | COLUMNS=120 ./target/release/claude-hud
```

`scripts/build-all.sh` cross-compiles every shipped platform into `dist/<triple>/`.

## Releases

Push a version tag and CI does the rest — it builds Windows, macOS (x64 + arm64),
and Linux (x64 + arm64), commits the binaries into `dist/`, and publishes a
GitHub Release:

```bash
git tag v0.1.0 && git push origin v0.1.0
```

## License

[MIT](./LICENSE)
