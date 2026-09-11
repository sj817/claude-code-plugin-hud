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
| `src/ribbon.rs` | Model badge, plain context and secondary row, colored metrics, and width fitting. |
| `src/render.rs`| Layout rules: constant height, width-aware drop by priority, single bar, smart path. Start here for display changes. |
| `src/git.rs`   | Branch + dirty flag; GitHub origin URL with a separate directory/session cache (5s TTL). |
| `src/theme.rs` | Calm 16-color palette, three accents (pink cache, soft-gold folder, orange quota band), and the usage / four-band quota threshold colors. |

## Layout

The default is the refined Studio two-row layout in `src/ribbon.rs`:
model badge, plain context/meter, quota usage/reset timers, and cost on the first row;
cache, version/duration, branch, and full project path on the second.
Only the model has a background. Context has no background or trailing triangle;
use ` · ` after its meter. The second row is plain text without ribbon arrows.
Use ` │ ` between the other plain groups and ` · ` within groups.
Quota countdowns have no parentheses; cache is `Cache: hit%(time)` with no space
before the parentheses. Branch is its name plus an optional coral dirty `*`.
Keep the path last in normal left-to-right flow, preserve it whenever it fits,
and normalize all backslashes to `/`. Use the refined Studio palette:
teal quota percentages below 50%, yellow 50–74%, orange 75–89%, red 90%+;
cache hits reuse classic `theme::CACHE` pink and cost uses `theme::COST` gold.
Branch names use vivid blue-cyan (#00bfff), never dark or whitened blue.
Context labels, quota/cache countdowns, Cache label, version and parent paths
reuse classic DIM; quota window labels and session duration use classic WHITE.
Reset intensity and foreground between parts so DIM cannot leak into accents.
Context percentage and meter share five saturated bands: blue-cyan below 25%,
green 25–49%, yellow 50–69%, orange 70–89%, red 90%+.
Prefix the path with `› ` (`> ` in ASCII
mode), color parent directories classic neutral gray and the project name cyan. Include
the marker in width calculations. No model or branch icon.
Wrap the fitted path in a balanced OSC 8 hyperlink to a recognized GitHub origin;
keep links outside plain text width math. Missing remotes leave ordinary paths.
On narrow windows omit leading metadata before sacrificing the project name;
do not move session duration or branch back to the first row.
`CLAUDE_HUD_ASCII=1` uses plain joins, group separators, and meter.
Shrink compact variants before dropping segments; truncate only plain text and
always reset SGR at the end. Quota/cache take priority over secondary segments.
Quota timers must not disappear in compact variants. Recalculate reset times
and cache expiry from the same current clock, even when input is unchanged.

`CLAUDE_HUD_STYLE=classic` selects the previous layout below.

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
countdown to `expires_at`; classic mode renders the old
`read/total(hit%)` from `current_usage` for builds that send no `prompt_cache`.
Compare `expires_at` against the current clock too: an expired snapshot must
show `cold` even if its `warm` field still says `true`.
The 🚀 in the final line-1 segment marks `fast_mode`.

## Invariants (do not regress)

1. Both layouts output 2 lines (1 when
   `CLAUDE_HUD_ONELINE=1` or `LINES` is tiny).
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
