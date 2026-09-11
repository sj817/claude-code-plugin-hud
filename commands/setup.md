---
description: Configure claude-code-plugin-hud as your Claude Code statusline
allowed-tools: Bash, Read, Edit, Write, AskUserQuestion
---

You are configuring the **claude-code-plugin-hud** statusline. Goal: detect the user's
platform, pick the matching prebuilt binary bundled under this plugin's `dist/`,
and write a `statusLine` entry into their `settings.json` pointing at it.

The plugin's root directory is available as the environment variable
`${CLAUDE_PLUGIN_ROOT}`. The prebuilt binaries live at:

```
${CLAUDE_PLUGIN_ROOT}/dist/<rust-target-triple>/claude-hud[.exe]
```

## Step 1 — Detect platform and architecture

Use the harness environment context (`Platform:`) plus a probe. Map to a Rust
target triple and binary name:

| Platform (`uname`/env) | Arch         | Target triple                  | Binary       |
|------------------------|--------------|--------------------------------|--------------|
| macOS (`darwin`)       | arm64        | `aarch64-apple-darwin`         | `claude-hud` |
| macOS (`darwin`)       | x86_64       | `x86_64-apple-darwin`          | `claude-hud` |
| Linux (glibc)          | x86_64       | `x86_64-unknown-linux-gnu`     | `claude-hud` |
| Linux (glibc)          | aarch64      | `aarch64-unknown-linux-gnu`    | `claude-hud` |
| Linux (musl/Alpine)    | x86_64       | `x86_64-unknown-linux-musl`    | `claude-hud` |
| Linux (musl/Alpine)    | aarch64      | `aarch64-unknown-linux-musl`   | `claude-hud` |
| Windows (`win32`)      | x86_64       | `x86_64-pc-windows-msvc`       | `claude-hud.exe` |
| Windows (`win32`)      | arm64        | `aarch64-pc-windows-msvc`      | `claude-hud.exe` |

- **macOS/Linux**: run `uname -sm` to get OS and arch (`arm64`/`aarch64` vs `x86_64`).
- **Linux glibc vs musl**: prefer the `-gnu` build. Use `-musl` only when glibc is
  absent (e.g. Alpine) — a quick check is `ldd --version 2>&1 | grep -qi musl`.
- **Windows**: usually `x86_64`; pick `aarch64-pc-windows-msvc` on ARM devices
  (Surface Pro X, etc). Claude Code may route the command through Git Bash or
  PowerShell — either launches the `.exe` directly.

## Step 2 — Verify the binary exists

Check that `${CLAUDE_PLUGIN_ROOT}/dist/<triple>/<binary>` exists.

- If it exists, capture its **absolute path** (resolve `${CLAUDE_PLUGIN_ROOT}`).
  On macOS/Linux, also ensure it is executable: `chmod +x <path>`.
- If it is **missing**, the user is on a platform we haven't shipped a binary
  for, or the dist wasn't built. Offer to build it locally if Rust is installed:
  `cargo build --release` inside `${CLAUDE_PLUGIN_ROOT}`, then use
  `${CLAUDE_PLUGIN_ROOT}/target/release/claude-hud[.exe]` as the path instead.
  If Rust is not installed, tell the user and stop.

## Step 3 — Write the statusLine into settings.json

Target the user's settings file: `${CLAUDE_CONFIG_DIR:-~/.claude}/settings.json`
(ask whether they prefer project-level `.claude/settings.json` if relevant).

Read the existing JSON (create `{}` if absent), then set the `statusLine` key,
**preserving all other keys**:

```json
{
  "statusLine": {
    "type": "command",
    "command": "<ABSOLUTE_PATH_TO_BINARY>",
    "padding": 0,
    "refreshInterval": 30
  }
}
```

Notes:
- Keep `padding` at `0`. Claude Code already adds built-in horizontal spacing;
  this field is extra indentation on top of it.
- On Windows, write the path with **forward slashes** (e.g.
  `C:/Users/you/.claude/plugins/.../claude-hud.exe`). Git Bash treats unquoted
  backslashes as escapes and the command will silently fail.
- Set `refreshInterval` to `30` seconds so cache and quota countdowns keep
  updating while the conversation is idle. Event-driven updates still run.
  Updating the binary alone does not add this setting; re-run setup for
  existing installations too.
- The binary already emits exactly two fixed-height lines and adapts to
  `$COLUMNS`, so no extra wrapper or width flags are needed.

## Step 4 — Confirm

Tell the user the statusline is configured and that changes appear on their next
interaction with Claude Code. Show them a one-line preview by piping mock JSON:

```bash
echo '{"model":{"display_name":"Opus"},"workspace":{"current_dir":"'"$PWD"'"},"context_window":{"used_percentage":42},"cost":{"total_cost_usd":1.23,"total_duration_ms":185000},"session_id":"preview"}' | COLUMNS=120 "<ABSOLUTE_PATH_TO_BINARY>"
```
