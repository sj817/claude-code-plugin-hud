# claude-code-plugin-hud — project notes

A Rust CLI that renders the Claude Code statusline, shipped as a Claude Code
plugin.

- Package name: `claude-code-plugin-hud`
- Binary name: `claude-hud` (kept short)
- Prebuilt per-platform binaries ship in `dist/`.
- The namespaced `/claude-code-plugin-hud:setup` command wires a binary into the
  user's `settings.json`. Plugins cannot set the main `statusLine` themselves;
  only a command can.

## How the statusline contract works

- Claude Code streams session JSON to the binary on **stdin**. Whatever the
  binary prints to **stdout** is shown, one rendered row per printed line.
- It runs after each assistant message, after `/compact`, and on permission/vim
  changes (debounced 300ms). Setup also sets `statusLine.refreshInterval: 30`
  (seconds) so cache/quota countdowns advance while idle. No background loop
  belongs in the binary: render once and exit on every invocation.
- Terminal size comes from the `COLUMNS`/`LINES` env vars (v2.1.153+), not
  `tput`. Keep `statusLine.padding` at 0 because it adds spacing on top of
  Claude Code's built-in gutter. Reserve a small safety margin inside `COLUMNS`.
- Width math must count emoji as terminals paint them. `unicode-width` follows
  East_Asian_Width, which calls `⏱` (U+23F1) one cell; `char_cells` in
  `render.rs` overrides that range to 2.
- Many JSON fields are optional or null (see `src/input.rs`). Never unwrap.
- Newer fields are also version-gated: `prompt_cache` and
  `rate_limits.spend_limit` need v2.1.251+, `pr.kind` needs v2.1.234+. Model
  them as `Option` and keep a fallback, so older builds still render fully.
- Permission mode (auto/plan/…) is not in the statusline JSON, so the HUD does
  not show it. Hooks expose `permission_mode`, but no hook fires on a bare
  shift+tab toggle, so it cannot be shown reliably. Omitted intentionally.

## Architecture

| File           | Responsibility                                                              |
| -------------- | -------------------------------------------------------------------------- |
| `src/main.rs`  | Read stdin, parse, read `$COLUMNS`/`$LINES`, reserve the UI safety margin, print. |
| `src/input.rs` | Serde structs; every field optional/defaulted.                             |
| `src/render.rs`| Layout rules: constant height, width-aware drop by priority, single bar, smart path. Start here for display changes. |
| `src/git.rs`   | Branch + dirty flag, cached per `session_id` (5s TTL).                      |
| `src/theme.rs` | Calm 16-color palette, three accents (pink cache, soft-gold folder, orange quota band), and the usage / four-band quota threshold colors. |

## Layout

```
<bar> used/total(%) | +add -del | 💰 cost | ⏱ dur | 🌿 branch*    version 🚀 ⚡effort(model)
Quota: 5h % reset · 7d % reset · $ % reset | 🎉Cache: hit% warm-left | 📁 smart-path
```

Line 2 leads with the quota segment (the focal point): a teal `Quota:` label,
bright window labels, the `%` in four 25% bands (green/yellow/orange/red), and a
dim reset countdown. The `$` window is the gateway spend limit and only appears
when Claude Code sends one; it is the single window whose `%` can exceed 100.
The `(%)` on the context bar and the cache are left uncolored.

The cache segment prefers the session-wide `prompt_cache.hit_ratio` plus the
countdown to `expires_at`; `legacy_cache_seg` renders the old
`read/total(hit%)` from `current_usage` for builds that send no `prompt_cache`.
Compare `expires_at` against the current clock too: an expired snapshot must
show `cold` even if its `warm` field still says `true`.
The 🚀 in the final line-1 segment marks `fast_mode`.

## Invariants (do not regress)

1. Output is exactly 2 lines (1 when `CLAUDE_HUD_ONELINE=1` or `LINES` is tiny).
2. A line never exceeds the drawable width. Drop segments, don't wrap.
   Drawable is `$COLUMNS` minus a four-column safety margin for Claude Code's
   built-in gutter and shared notification area; `CLAUDE_HUD_MARGIN` overrides it.
3. At most one progress bar (the context window).
4. Build is warning-free; CI enforces `cargo clippy -- -D warnings`.

## Build & test

```bash
cargo build --release
cargo fmt --all && cargo clippy --release -- -D warnings
echo '{"model":{"display_name":"Opus 4.8 (1M context)"},"context_window":{"used_percentage":25,"total_input_tokens":50000,"context_window_size":200000},"session_id":"x"}' | COLUMNS=120 ./target/release/claude-hud
scripts/build-all.sh    # cross-compile all dist/ targets
```

## CI

The workflows live in `.github/workflows/`:

- `ci.yml` runs fmt/clippy/build/smoke on push and PR.
- `release.yml` builds all targets and attaches them to a GitHub Release on a
  `v*` tag.
