#!/usr/bin/env bash
# Cross-compile claude-hud for all shipped targets into dist/<triple>/.
# Requires the toolchains/targets installed (rustup target add ...), or use
# `cross` (https://github.com/cross-rs/cross) for the linux/foreign targets.
#
# Usage: scripts/build-all.sh            # cargo for native, prints hints for the rest
#        BUILDER=cross scripts/build-all.sh
set -euo pipefail

cd "$(dirname "$0")/.."

TARGETS=(
  "x86_64-pc-windows-msvc"
  "x86_64-apple-darwin"
  "aarch64-apple-darwin"
  "x86_64-unknown-linux-gnu"
  "aarch64-unknown-linux-gnu"
)

BUILDER="${BUILDER:-cargo}"

for triple in "${TARGETS[@]}"; do
  echo ">> building $triple"
  if "$BUILDER" build --release --target "$triple"; then
    bin="claude-hud"
    [[ "$triple" == *windows* ]] && bin="claude-hud.exe"
    mkdir -p "dist/$triple"
    cp "target/$triple/release/$bin" "dist/$triple/$bin"
    echo "   -> dist/$triple/$bin"
  else
    echo "   !! skipped $triple (toolchain/target not available)"
  fi
done

echo "Done. Bundled binaries under dist/."
