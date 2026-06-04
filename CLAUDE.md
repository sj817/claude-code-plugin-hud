# claude-code-plugin-hud — project notes

A Rust CLI that renders the Claude Code statusline. Packaged as a Claude Code
plugin: prebuilt per-platform binaries ship in `dist/`, and the namespaced
`/claude-code-plugin-hud:setup` command wires one into the user's
`settings.json` (plugins cannot set the main `statusLine` themselves — only a
command can).

Package name: `claude-code-plugin-hud`. Binary name: `claude-hud` (kept short).

## How the statusline contract works

- Claude Code streams session JSON to the binary on **stdin**; whatever it
  prints to **stdout** is shown. Each printed line = one rendered row.
- Runs after each assistant message, after `/compact`, on permission/vim
  changes (debounced 300ms). It does NOT re-run while you type.
- Terminal size is read from the `COLUMNS`/`LINES` env vars (v2.1.153+), not
  `tput`. Many JSON fields are optional/null (see `src/input.rs`) — never unwrap.
- Permission mode (auto/plan/…) is NOT in the statusline JSON, so the HUD does
  not show it. (Hooks expose `permission_mode`, but no hook fires on a bare
  shift+tab toggle, so it can't be shown reliably — intentionally omitted.)

## Architecture

- `src/main.rs`   — read stdin, parse, read `$COLUMNS`/`$LINES`, print.
- `src/input.rs`  — serde structs; every field optional/defaulted.
- `src/render.rs` — layout rules (constant height, width-aware drop by
  priority, single bar, smart path). Start here for display changes.
- `src/git.rs`    — branch + dirty flag, cached per `session_id` (5s TTL).
- `src/theme.rs`  — calm 16-color palette + two accents (pink cache, soft-gold
  folder) + usage/cache threshold colors.

## Layout

```
<bar> used/total(%) | +add -del | 💰 cost | ⏱ dur | 🌿 branch*    version ⚡effort(model)
🎉Cache: read/total(hit%) | 5h(% reset) · 7d(% reset) | 📁 smart-path
```

## Invariants (do not regress)

1. Output is exactly 2 lines (1 when `CLAUDE_HUD_ONELINE=1` or `LINES` tiny).
2. A line never exceeds `$COLUMNS` (drop segments, don't wrap).
3. At most one progress bar (the context window).
4. Build is warning-free; `cargo clippy -- -D warnings` is enforced in CI.

## Build & test

```bash
cargo build --release
cargo fmt --all && cargo clippy --release -- -D warnings
echo '{"model":{"display_name":"Opus 4.8 (1M context)"},"context_window":{"used_percentage":25,"total_input_tokens":50000,"context_window_size":200000},"session_id":"x"}' | COLUMNS=120 ./target/release/claude-hud
scripts/build-all.sh    # cross-compile all dist/ targets
```

CI (`.github/workflows/`): `ci.yml` runs fmt/clippy/build/smoke on push & PR;
`release.yml` builds all targets and attaches them to a GitHub Release on a
`v*` tag.
