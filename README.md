# claude-code-plugin-hud

A lightweight, **fixed-height** statusline HUD for [Claude Code](https://code.claude.com/docs/en/statusline), written in Rust.

Most HUDs sprawl onto many lines, wrap on narrow terminals, and get shoved
around when the input box grows. This one stays exactly two lines, adapts to
your terminal width, and uses calm colors that follow your terminal theme.

English | [简体中文](./README.zh-CN.md)

```
██░░░░░░ 311k/1M(31%) | +1820 -800 | 💰 $16.20 | ⏱  2h13m | 🌿 main*        v2.1.161 ⚡high(Opus 4.8)
🎉Cache: 310k/311k(99%) | 5h(17% 2h19m) · 7d(37% 2d11h) | 📁 Github/claude-code-plugin-hud
```

## What it shows

**Line 1** — live status:
`context bar used/total(%)` · `+added/-removed lines` · `💰 cost` · `⏱ duration` · `🌿 branch(* if dirty)`, with `version` · `⚡effort(model)` pinned to the right corner.

**Line 2** — session & limits:
`🎉Cache: read/total(hit%)` · `5h(used% reset) · 7d(used% reset)` · `📁 folder`.

## Design principles

The statusline renders inline above the prompt — it is **not** a pinned overlay,
and nothing can change that. So this HUD leans into the constraint:

1. **Constant height.** Always exactly two lines (one when collapsed). Missing
   data becomes a placeholder, never a dropped line — the HUD never jumps.
2. **Width-aware, never wraps.** Reads `$COLUMNS` and drops the lowest-priority
   segments until each line fits. A wrapped line silently doubles the height —
   the exact failure this avoids.
3. **One progress bar.** Only the context window gets a bar; everything else is
   compact text.
4. **Smart path.** The folder shows the full path when it fits ~40 chars,
   otherwise trims leading components (never below the final one).
5. **Calm colors.** Standard 16-color ANSI that follows your terminal theme,
   plus a couple of deliberate accents. Context uses green→yellow→red by usage;
   cache hit-rate is inverted (high = green = healthy).

## Install

This plugin ships as a Claude Code plugin via a marketplace:

```text
/plugin marketplace add sj817/claude-code-plugin-hud
/plugin install claude-code-plugin-hud
/claude-code-plugin-hud:setup
```

`/claude-code-plugin-hud:setup` (plugin commands are namespaced by the plugin
name) detects your OS/arch, picks the matching prebuilt binary from `dist/`, and
writes the `statusLine` entry into your `settings.json`. (Plugins cannot set the
main `statusLine` themselves — only a command can.)

### Manual

```jsonc
// ~/.claude/settings.json
{
  "statusLine": {
    "type": "command",
    "command": "/abs/path/to/claude-hud",   // forward slashes on Windows
    "padding": 2
  }
}
```

On older Claude Code (< v2.1.153) that doesn't export `$COLUMNS`, wrap it to
compute width from the tty:

```jsonc
"command": "cols=$(stty size </dev/tty 2>/dev/null | awk '{print $2}'); export COLUMNS=${cols:-120}; exec /abs/path/to/claude-hud"
```

## Configuration

| Env var               | Effect                                                        |
| --------------------- | ------------------------------------------------------------- |
| `CLAUDE_HUD_ONELINE`  | `1` forces a single line (keeps the built-in mode line free). |
| `COLUMNS` / `LINES`   | Terminal size; Claude Code v2.1.153+ exports these.           |

When `LINES` is very small the HUD auto-collapses to one line.

## Build

```bash
cargo build --release            # native -> target/release/claude-hud
scripts/build-all.sh             # all shipped targets -> dist/<triple>/
```

The release profile is tuned for size (`opt-level = "z"`, LTO, `panic = "abort"`,
stripped) — the binary is ~310 KB with no runtime dependencies.

Test with mock input:

```bash
echo '{"model":{"display_name":"Opus 4.8 (1M context)"},"context_window":{"used_percentage":25,"total_input_tokens":50000,"context_window_size":200000},"session_id":"x"}' \
  | COLUMNS=120 ./target/release/claude-hud
```

## Releasing

`git tag v0.1.0 && git push --tags` triggers the release workflow, which builds
all targets (Windows / macOS x64+arm64 / Linux x64+arm64) and attaches them to a
GitHub Release. To ship binaries with the plugin so `/plugin install` works
without a local build, place them under `dist/<triple>/` and commit.

## License

MIT
