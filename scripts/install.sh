#!/usr/bin/env bash
#
# install.sh — install the claude-code-plugin-hud statusline binary (claude-hud)
# on macOS and Linux, and wire it into Claude Code's settings.json.
#
# Designed to be run straight from the network:
#
#     curl -fsSL https://raw.githubusercontent.com/sj817/claude-code-plugin-hud/main/scripts/install.sh | bash
#
# Because it executes FROM STDIN, it never reads from stdin (no prompts) and
# does not rely on $0 being a real path. All options come from "$@" and env vars.
#
# Options / env vars:
#   --version <v>   | CLAUDE_HUD_VERSION   : "latest" (default) or a tag like v0.1.6 / 0.1.6
#   --dir <path>    | CLAUDE_HUD_DIR        : install dir (default: <config>/statusline)
#                     CLAUDE_CONFIG_DIR     : Claude config dir (default: <home>/.claude)
#   -h | --help

set -euo pipefail
# NOTE: we deliberately do NOT set a custom global IFS. A custom IFS (e.g.
# $'\n\t') combined with `set -u` makes the script brittle: a single future
# unquoted expansion would split only on newline/tab, and helpers like
# `printf '%s' "$*"` would join multi-arg calls on a newline. All expansions
# below are double-quoted, so the default IFS is both correct and safer.

REPO="sj817/claude-code-plugin-hud"
BIN_NAME="claude-hud"

# ---------------------------------------------------------------------------
# i18n — language selection + message catalog
# ---------------------------------------------------------------------------
# Pick the runtime language ONCE. English is always the universal fallback.
#   1) Explicit override: CLAUDE_HUD_LANG. A value starting with "zh" (any
#      case) -> zh; any other non-empty value -> en.
#   2) Otherwise auto-detect from the locale env vars; zh/ZH prefix -> zh.
#   3) Unknown / unset -> en.
detect_lang() {
  local override="${CLAUDE_HUD_LANG:-}"
  if [ -n "$override" ]; then
    case "$override" in
      zh*|ZH*|Zh*|zH*) echo "zh" ;;
      *)               echo "en" ;;
    esac
    return
  fi
  local loc="${LC_ALL:-${LC_MESSAGES:-${LANG:-}}}"
  case "$loc" in
    zh*|ZH*) echo "zh" ;;
    *)       echo "en" ;;
  esac
}
HUD_LANG="$(detect_lang)"

# t KEY [args...] — print one localized line.
#
# The catalog is two `case` blocks: a zh block (consulted only when
# HUD_LANG=zh) and an en block (the universal fallback). If the zh block has
# no entry for a key, `fmt` stays empty and we fall through to the en block,
# so a missing zh translation degrades to English automatically.
#
# The localized string is a printf FORMAT; interpolated values come in as
# "$@" and are NEVER translated (paths, URLs, codes, triples stay verbatim).
# shellcheck disable=SC2059
t() {
  local key="$1"; shift
  local fmt=""

  if [ "$HUD_LANG" = "zh" ]; then
    case "$key" in
      err_prefix)            fmt='错误：%s' ;;
      indent)                fmt='  %s' ;;

      usage)                 fmt='用法：install.sh [--version <版本>] [--dir <路径>]

  --version <版本>   要安装的版本："latest"（默认），或形如 v0.1.6 / 0.1.6
  --dir <路径>       安装目录（默认：<CLAUDE_CONFIG_DIR>/statusline）
  -h, --help         显示此帮助

环境变量覆盖：
  CLAUDE_HUD_VERSION   等同于 --version
  CLAUDE_HUD_DIR       等同于 --dir
  CLAUDE_CONFIG_DIR    Claude 配置目录（默认：$HOME/.claude）' ;;

      need_version_value)    fmt='--version 需要一个值' ;;
      need_dir_value)        fmt='--dir 需要一个值' ;;
      unknown_option)        fmt='未知选项：%s（请使用 --help）' ;;

      windows_use_ps1)       fmt='此脚本适用于 macOS 和 Linux。在 Windows 上请改用 install.ps1。' ;;
      unsupported_os)        fmt='不支持的操作系统 "%s"。在 Windows 上请使用 install.ps1；否则此平台不受支持。' ;;
      unsupported_arch)      fmt='不支持的架构 "%s"（需要 x86_64 或 arm64/aarch64）。在 Windows 上请使用 install.ps1。' ;;

      home_not_set)          fmt='未设置 $HOME；无法解析默认安装位置' ;;

      banner)                fmt='claude-code-plugin-hud 安装程序' ;;
      label_triple)          fmt='目标三元组    ：%s' ;;
      label_version)         fmt='版本          ：%s' ;;
      label_url)             fmt='下载地址      ：%s' ;;
      label_install)         fmt='安装路径      ：%s' ;;
      label_settings)        fmt='settings.json ：%s' ;;

      mktemp_failed)         fmt='创建临时目录失败' ;;

      download_hint)         fmt='  - 地址：%s
  - 如果你指定了 --version，请确认该版本存在：
      https://github.com/%s/releases
  - 请确认已为三元组 "%s" 发布了对应的资源文件。
  - 否则请检查你的网络 / 代理 / TLS 配置。' ;;

      downloading)           fmt='正在下载……' ;;
      dl_failed_404)         fmt='下载失败：未找到版本或资源文件（HTTP 404）。
%s' ;;
      dl_failed_noresp)      fmt='下载失败：无法连接到 GitHub（无 HTTP 响应）。
%s' ;;
      dl_failed_http)        fmt='下载失败（HTTP %s）。
%s' ;;
      dl_failed_busybox)     fmt='下载失败（busybox wget）：%s
  - BusyBox 的 wget 通常无法跟随 GitHub 指向
    objects.githubusercontent.com 的 HTTPS 重定向。请安装完整的 curl 后重试：
      apk add --no-cache curl
%s' ;;
      dl_failed_wget)        fmt='下载失败（wget）：%s
%s' ;;
      need_curl_or_wget)     fmt='需要 "curl" 或 "wget" 来下载，但两者都未安装。
  - 在 Alpine 上：apk add --no-cache curl
  - 在 Debian 上：apt-get install -y curl' ;;
      dl_empty)              fmt='下载的文件为空：%s（地址：%s）' ;;
      saved)                 fmt='已保存 %s' ;;

      extracting)            fmt='正在解压……' ;;
      unzip_failed)          fmt='解压失败：%s' ;;
      pyzip_failed)          fmt='python3 -m zipfile 解压失败：%s' ;;
      bsdtar_failed)         fmt='bsdtar 解压失败：%s' ;;
      no_extractor)          fmt='未找到解压工具。请安装以下任意一种：unzip、python3 或 bsdtar。' ;;
      bin_not_found)         fmt='在归档中找不到 "%s"（预期路径 %s/%s）。' ;;
      found_binary)          fmt='找到二进制文件 %s' ;;

      installing_binary)     fmt='正在安装二进制文件……' ;;
      mkdir_install_failed)  fmt='无法创建安装目录：%s' ;;
      copy_failed)           fmt='无法将二进制文件复制到 %s' ;;
      chmod_failed)          fmt='无法对 %s 执行 chmod +x' ;;
      move_failed)           fmt='无法将二进制文件移动到位：%s' ;;
      move_settings_failed)  fmt='无法将更新后的 settings.json 移动到位：%s' ;;
      installed_to)          fmt='已安装 -> %s' ;;

      mkdir_config_failed)   fmt='无法创建配置目录：%s' ;;
      backed_up)             fmt='已备份现有的 settings.json -> %s' ;;
      stage_new_failed)      fmt='无法暂存新的 settings.json' ;;
      write_settings_failed) fmt='无法写入 %s' ;;
      wrote_new_settings)    fmt='已写入包含 statusLine 的新 settings.json' ;;

      configuring)           fmt='正在配置 settings.json……' ;;
      py_update_failed)      fmt='通过 python3 更新 settings.json 失败' ;;
      py_invalid_json)       fmt='现有的 settings.json 不是有效的 JSON；拒绝覆盖（已创建 .bak 备份）。请修复后重试，或手动合并 statusLine 配置块。（%s）' ;;
      py_not_object)         fmt='现有的 settings.json 不是 JSON 对象；拒绝覆盖（已创建 .bak 备份）。' ;;
      updated_statusline)    fmt='已更新 %s 中的 statusLine' ;;
      jq_failed)             fmt='jq 更新 settings.json 失败。现有文件可能非空但不是有效的 JSON
  （UTF-8 BOM、未完成的写入或多余的文本都会导致此问题）。已创建 .bak 备份。
  请修复 "%s" 后重试，或手动合并 statusLine 配置块。' ;;

      warn_no_tool_1)        fmt='警告：未找到 "python3" 或 "jq"，因此 settings.json' ;;
      warn_no_tool_2)        fmt='         未被修改（你现有的文件保持不变；已创建 .bak 备份）。' ;;
      warn_no_tool_3)        fmt='         请自行将下面这段配置添加到 %s，' ;;
      warn_no_tool_4)        fmt='         并与你现有的 JSON 合并（保留所有其他键）：' ;;

      success)               fmt='安装成功！claude-hud 已安装，你的状态栏已配置完成。' ;;
      label_binary)          fmt='二进制文件 ：%s' ;;
      label_settings2)       fmt='设置文件   ：%s' ;;
      preview_header)        fmt='预览（模拟会话，COLUMNS=120）：' ;;
      preview_failed)        fmt='预览运行失败，但二进制文件已安装在 %s。' ;;
      restart_hint)          fmt='请重启 Claude Code（或新开一个会话）以查看状态栏。' ;;
      *)                     fmt='' ;;
    esac
  fi

  if [ -z "$fmt" ]; then
    case "$key" in
      err_prefix)            fmt='error: %s' ;;
      indent)                fmt='  %s' ;;

      usage)                 fmt='Usage: install.sh [--version <v>] [--dir <path>]

  --version <v>   Release to install: "latest" (default) or e.g. v0.1.6 / 0.1.6
  --dir <path>    Install directory (default: <CLAUDE_CONFIG_DIR>/statusline)
  -h, --help      Show this help

Environment overrides:
  CLAUDE_HUD_VERSION   same as --version
  CLAUDE_HUD_DIR       same as --dir
  CLAUDE_CONFIG_DIR    Claude config dir (default: $HOME/.claude)' ;;

      need_version_value)    fmt='--version requires a value' ;;
      need_dir_value)        fmt='--dir requires a value' ;;
      unknown_option)        fmt='unknown option: %s (try --help)' ;;

      windows_use_ps1)       fmt='this script is for macOS and Linux. On Windows, use install.ps1 instead.' ;;
      unsupported_os)        fmt="unsupported OS '%s'. On Windows use install.ps1; otherwise this platform is not supported." ;;
      unsupported_arch)      fmt="unsupported architecture '%s' (need x86_64 or arm64/aarch64). On Windows use install.ps1." ;;

      home_not_set)          fmt='$HOME is not set; cannot resolve default install location' ;;

      banner)                fmt='claude-code-plugin-hud installer' ;;
      label_triple)          fmt='target triple : %s' ;;
      label_version)         fmt='version       : %s' ;;
      label_url)             fmt='download URL  : %s' ;;
      label_install)         fmt='install path  : %s' ;;
      label_settings)        fmt='settings.json : %s' ;;

      mktemp_failed)         fmt='failed to create temp dir' ;;

      download_hint)         fmt='  - URL: %s
  - If you pinned --version, confirm that release exists:
      https://github.com/%s/releases
  - Confirm an asset is published for triple '"'"'%s'"'"'.
  - Otherwise check your network / proxy / TLS configuration.' ;;

      downloading)           fmt='Downloading...' ;;
      dl_failed_404)         fmt='download failed: release/asset not found (HTTP 404).
%s' ;;
      dl_failed_noresp)      fmt='download failed: could not reach GitHub (no HTTP response).
%s' ;;
      dl_failed_http)        fmt='download failed (HTTP %s).
%s' ;;
      dl_failed_busybox)     fmt='download failed (busybox wget): %s
  - BusyBox wget often can'"'"'t follow GitHub'"'"'s HTTPS redirect to
    objects.githubusercontent.com. Install full curl and re-run:
      apk add --no-cache curl
%s' ;;
      dl_failed_wget)        fmt='download failed (wget): %s
%s' ;;
      need_curl_or_wget)     fmt="need 'curl' or 'wget' to download, but neither is installed.
  - On Alpine:    apk add --no-cache curl
  - On Debian:    apt-get install -y curl" ;;
      dl_empty)              fmt='downloaded file is empty: %s (URL: %s)' ;;
      saved)                 fmt='saved %s' ;;

      extracting)            fmt='Extracting...' ;;
      unzip_failed)          fmt='unzip failed for %s' ;;
      pyzip_failed)          fmt='python3 -m zipfile extract failed for %s' ;;
      bsdtar_failed)         fmt='bsdtar failed for %s' ;;
      no_extractor)          fmt='no extractor found. Install one of: unzip, python3, or bsdtar.' ;;
      bin_not_found)         fmt="could not find '%s' inside the archive (expected %s/%s)." ;;
      found_binary)          fmt='found binary  %s' ;;

      installing_binary)     fmt='Installing binary...' ;;
      mkdir_install_failed)  fmt='could not create install dir: %s' ;;
      copy_failed)           fmt='could not copy binary into %s' ;;
      chmod_failed)          fmt='could not chmod +x %s' ;;
      move_failed)           fmt='could not move binary into place: %s' ;;
      move_settings_failed)  fmt='could not move updated settings into place: %s' ;;
      installed_to)          fmt='installed -> %s' ;;

      mkdir_config_failed)   fmt='could not create config dir: %s' ;;
      backed_up)             fmt='backed up existing settings.json -> %s' ;;
      stage_new_failed)      fmt='could not stage new settings.json' ;;
      write_settings_failed) fmt='could not write %s' ;;
      wrote_new_settings)    fmt='wrote new settings.json with statusLine' ;;

      configuring)           fmt='Configuring settings.json...' ;;
      py_update_failed)      fmt='failed to update settings.json via python3' ;;
      py_invalid_json)       fmt='existing settings.json is not valid JSON; refusing to overwrite (a .bak backup was made). Fix it and re-run, or merge the statusLine block manually. (%s)' ;;
      py_not_object)         fmt='existing settings.json is not a JSON object; refusing to overwrite (a .bak backup was made).' ;;
      updated_statusline)    fmt='updated statusLine in %s' ;;
      jq_failed)             fmt="jq failed to update settings.json. The existing file may be non-empty
  but not valid JSON (a UTF-8 BOM, a partial write, or stray text will do
  this). A .bak backup was made. Fix '%s' and re-run, or merge
  the statusLine block manually." ;;

      warn_no_tool_1)        fmt="WARNING: neither 'python3' nor 'jq' is available, so settings.json was" ;;
      warn_no_tool_2)        fmt='         NOT modified (your existing file is untouched; a .bak was made).' ;;
      warn_no_tool_3)        fmt='         Add this block to %s yourself, merging it with' ;;
      warn_no_tool_4)        fmt='         your existing JSON (keep all other keys):' ;;

      success)               fmt='Success! claude-hud is installed and your statusline is configured.' ;;
      label_binary)          fmt='binary   : %s' ;;
      label_settings2)       fmt='settings : %s' ;;
      preview_header)        fmt='Preview (mock session, COLUMNS=120):' ;;
      preview_failed)        fmt='preview run failed, but the binary is installed at %s.' ;;
      restart_hint)          fmt='Restart Claude Code (or start a new session) to see the statusline.' ;;
      *)                     fmt='%s' ;;
    esac
  fi

  printf "$fmt\n" "$@"
}

# ---------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------
log()  { printf '%s\n' "$*"; }
info() { printf '  %s\n' "$*"; }
err()  { t err_prefix "$*" >&2; }
die()  { err "$*"; exit 1; }

usage() {
  t usage
}

# A temp dir we clean up no matter how we exit. Settings writes also stage
# their temp files inside here so an interrupted write never orphans a
# *.tmp next to the real settings.json.
TMP_DIR=""
cleanup() {
  if [ -n "${TMP_DIR:-}" ] && [ -d "${TMP_DIR}" ]; then
    rm -rf "${TMP_DIR}" 2>/dev/null || true
  fi
}
trap cleanup EXIT

# ---------------------------------------------------------------------------
# parse args
# ---------------------------------------------------------------------------
VERSION="${CLAUDE_HUD_VERSION:-latest}"
DIR_OVERRIDE="${CLAUDE_HUD_DIR:-}"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --version)
      [ "$#" -ge 2 ] || die "$(t need_version_value)"
      VERSION="$2"; shift 2 ;;
    --version=*)
      VERSION="${1#*=}"; shift ;;
    --dir)
      [ "$#" -ge 2 ] || die "$(t need_dir_value)"
      DIR_OVERRIDE="$2"; shift 2 ;;
    --dir=*)
      DIR_OVERRIDE="${1#*=}"; shift ;;
    -h|--help)
      usage; exit 0 ;;
    *)
      die "$(t unknown_option "$1")" ;;
  esac
done

# ---------------------------------------------------------------------------
# detect OS / arch -> target triple
# ---------------------------------------------------------------------------
UNAME_S="$(uname -s 2>/dev/null || echo unknown)"
UNAME_M="$(uname -m 2>/dev/null || echo unknown)"

case "$UNAME_S" in
  Darwin) OS="darwin" ;;
  Linux)  OS="linux" ;;
  MINGW*|MSYS*|CYGWIN*|Windows_NT)
    die "$(t windows_use_ps1)" ;;
  *)
    die "$(t unsupported_os "$UNAME_S")" ;;
esac

case "$UNAME_M" in
  x86_64|amd64)   ARCH="x86_64" ;;
  arm64|aarch64)  ARCH="aarch64" ;;
  *)
    die "$(t unsupported_arch "$UNAME_M")" ;;
esac

# Detect the C library on Linux so we pick the right release asset. Getting
# this wrong is the most likely real-machine failure: shipping a glibc build
# to a musl box yields a runtime "No such file or directory" (loader absent)
# or a segfault. We therefore use several independent signals and treat ANY
# positive musl signal as decisive.
detect_libc() {
  # 1) The musl dynamic loader has a stable, glob-able name. Its presence is
  #    the most reliable positive signal and works even when `ldd` is busybox
  #    (Alpine) or entirely absent. glibc systems do not ship this file.
  local f
  for f in /lib/ld-musl-*.so.1 /usr/lib/ld-musl-*.so.1; do
    [ -e "$f" ] && { echo "musl"; return; }
  done

  # 2) GNU/musl `ldd --version` prints "musl" on real musl toolchains. (On
  #    busybox this prints a usage banner and won't match — that's why signal
  #    (1) exists.) The `if`-condition context means a non-zero/SIGPIPE exit
  #    from ldd does not trip `set -e`/`pipefail`.
  if command -v ldd >/dev/null 2>&1; then
    if ldd --version 2>&1 | grep -qi musl; then
      echo "musl"; return
    fi
  fi

  # 3) The glibc loader is present -> definitely glibc.
  for f in /lib/ld-linux-*.so.2 /lib64/ld-linux-*.so.2 \
           /lib/ld-linux.so.2 /lib/ld-linux-aarch64.so.1; do
    [ -e "$f" ] && { echo "gnu"; return; }
  done

  # 4) Distro hints for musl distros that have no glob-able loader path
  #    (Alpine, plus a generic os-release scan covering Void-musl, etc.).
  if [ -f /etc/alpine-release ]; then
    echo "musl"; return
  fi
  if [ -r /etc/os-release ] && grep -qiE 'musl|alpine' /etc/os-release 2>/dev/null; then
    echo "musl"; return
  fi

  # 5) Default: assume glibc (the common case; most Linux desktops/servers).
  echo "gnu"
}

if [ "$OS" = "darwin" ]; then
  TRIPLE="${ARCH}-apple-darwin"
else
  LIBC="$(detect_libc)"
  TRIPLE="${ARCH}-unknown-linux-${LIBC}"
fi

# ---------------------------------------------------------------------------
# normalize version -> download URL
# ---------------------------------------------------------------------------
ASSET="claude-code-plugin-hud-${TRIPLE}.zip"

if [ "$VERSION" = "latest" ] || [ -z "$VERSION" ]; then
  VERSION="latest"
  URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"
else
  # Accept "0.1.6" or "v0.1.6" (also "V0.1.6"); pinned URL needs a lowercase "v".
  TAG="$VERSION"
  case "$TAG" in
    v*) ;;
    V*) TAG="v${TAG#V}" ;;
    *)  TAG="v${TAG}" ;;
  esac
  VERSION="$TAG"
  URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET}"
fi

# ---------------------------------------------------------------------------
# resolve install dir + settings path
# ---------------------------------------------------------------------------
HOME_DIR="${HOME:-}"
[ -n "$HOME_DIR" ] || die "$(t home_not_set)"

CONFIG_DIR="${CLAUDE_CONFIG_DIR:-${HOME_DIR}/.claude}"

if [ -n "$DIR_OVERRIDE" ]; then
  INSTALL_DIR="$DIR_OVERRIDE"
else
  INSTALL_DIR="${CONFIG_DIR}/statusline"
fi

INSTALL_PATH="${INSTALL_DIR}/${BIN_NAME}"
SETTINGS_PATH="${CONFIG_DIR}/settings.json"

log "$(t banner)"
info "$(t label_triple "${TRIPLE}")"
info "$(t label_version "${VERSION}")"
info "$(t label_url "${URL}")"
info "$(t label_install "${INSTALL_PATH}")"
info "$(t label_settings "${SETTINGS_PATH}")"
log ""

# ---------------------------------------------------------------------------
# create temp workspace (portable fallback for hosts whose default mktemp -d
# fails, e.g. an unwritable TMPDIR — the GNU/BSD-portable template requires
# trailing X's).
# ---------------------------------------------------------------------------
TMP_DIR="$(mktemp -d 2>/dev/null || mktemp -d "${TMPDIR:-/tmp}/claude-hud.XXXXXX")"
[ -n "$TMP_DIR" ] && [ -d "$TMP_DIR" ] || die "$(t mktemp_failed)"

ZIP_PATH="${TMP_DIR}/${ASSET}"

# ---------------------------------------------------------------------------
# download
# ---------------------------------------------------------------------------
# Distinguish a 404 (e.g. a pinned --version that doesn't exist, or a triple
# with no published asset) from a transport/network failure, so a user who
# typo'd a version doesn't get a misleading "check your network".
download_hint() {
  t download_hint "${URL}" "${REPO}" "${TRIPLE}"
}

log "$(t downloading)"
if command -v curl >/dev/null 2>&1; then
  # Capture the HTTP status separately so we can name 404 explicitly. -f makes
  # curl exit non-zero on HTTP >= 400; -w prints the final status code.
  HTTP_CODE="$(curl -fsSL -o "$ZIP_PATH" -w '%{http_code}' "$URL" 2>/dev/null)" || HTTP_CODE="000"
  if [ ! -s "$ZIP_PATH" ]; then
    case "$HTTP_CODE" in
      404)
        die "$(t dl_failed_404 "$(download_hint)")" ;;
      000|"")
        die "$(t dl_failed_noresp "$(download_hint)")" ;;
      *)
        die "$(t dl_failed_http "${HTTP_CODE}" "$(download_hint)")" ;;
    esac
  fi
elif command -v wget >/dev/null 2>&1; then
  # BusyBox wget (common on Alpine, the very place curl is often missing) has
  # historically had spotty https+redirect support and may fail the TLS
  # handshake to objects.githubusercontent.com. Detect it so the error is
  # actionable rather than a bare "wget failed".
  WGET_IS_BUSYBOX="no"
  if wget --version 2>&1 | grep -qi busybox; then
    WGET_IS_BUSYBOX="yes"
  fi
  if ! wget -qO "$ZIP_PATH" "$URL"; then
    if [ "$WGET_IS_BUSYBOX" = "yes" ]; then
      die "$(t dl_failed_busybox "$URL" "$(download_hint)")"
    fi
    die "$(t dl_failed_wget "$URL" "$(download_hint)")"
  fi
else
  die "$(t need_curl_or_wget)"
fi

[ -s "$ZIP_PATH" ] || die "$(t dl_empty "$ZIP_PATH" "$URL")"
info "$(t saved "${ZIP_PATH}")"

# ---------------------------------------------------------------------------
# extract
# ---------------------------------------------------------------------------
EXTRACT_DIR="${TMP_DIR}/extract"
mkdir -p "$EXTRACT_DIR"

log "$(t extracting)"
if command -v unzip >/dev/null 2>&1; then
  unzip -q "$ZIP_PATH" -d "$EXTRACT_DIR" || die "$(t unzip_failed "$ZIP_PATH")"
elif command -v python3 >/dev/null 2>&1; then
  python3 -m zipfile -e "$ZIP_PATH" "$EXTRACT_DIR" || die "$(t pyzip_failed "$ZIP_PATH")"
elif command -v bsdtar >/dev/null 2>&1; then
  bsdtar -xf "$ZIP_PATH" -C "$EXTRACT_DIR" || die "$(t bsdtar_failed "$ZIP_PATH")"
else
  die "$(t no_extractor)"
fi

# Locate the binary at <triple>/claude-hud inside the extraction (this matches
# the published release layout: `zip -r ...-<triple>.zip <triple>`). Fall back
# to a constrained search ONLY under the expected <triple>/ subtree so an
# archive that ever bundled multiple triples can't hand us the wrong arch.
SRC_BIN="${EXTRACT_DIR}/${TRIPLE}/${BIN_NAME}"
if [ ! -f "$SRC_BIN" ]; then
  FOUND="$(find "${EXTRACT_DIR}/${TRIPLE}" -type f -name "$BIN_NAME" 2>/dev/null | head -n 1 || true)"
  if [ -z "$FOUND" ]; then
    # Last resort: search the whole extraction (single-binary archives only).
    FOUND="$(find "$EXTRACT_DIR" -type f -name "$BIN_NAME" 2>/dev/null | head -n 1 || true)"
  fi
  [ -n "$FOUND" ] || die "$(t bin_not_found "${BIN_NAME}" "${TRIPLE}" "${BIN_NAME}")"
  SRC_BIN="$FOUND"
fi
info "$(t found_binary "${SRC_BIN}")"

# ---------------------------------------------------------------------------
# install binary (idempotent: just overwrites)
# ---------------------------------------------------------------------------
log "$(t installing_binary)"
mkdir -p "$INSTALL_DIR" || die "$(t mkdir_install_failed "$INSTALL_DIR")"

# Copy then chmod into a temp name in the install dir, then mv for an
# atomic-ish replace (mv within the same filesystem is atomic; staging in the
# target dir keeps it on that filesystem).
TMP_INSTALL="${INSTALL_PATH}.new.$$"
cp "$SRC_BIN" "$TMP_INSTALL" || die "$(t copy_failed "$INSTALL_DIR")"
chmod +x "$TMP_INSTALL" || die "$(t chmod_failed "$TMP_INSTALL")"
mv -f "$TMP_INSTALL" "$INSTALL_PATH" || die "$(t move_failed "$INSTALL_PATH")"
info "$(t installed_to "${INSTALL_PATH}")"

# ---------------------------------------------------------------------------
# update settings.json (preserve all existing keys; back up first)
# ---------------------------------------------------------------------------
mkdir -p "$CONFIG_DIR" || die "$(t mkdir_config_failed "$CONFIG_DIR")"

# On these platforms the path is already forward-slashed.
CMD_PATH="$INSTALL_PATH"

PASTE_SNIPPET=$(cat <<EOF
{
  "statusLine": {
    "type": "command",
    "command": "${CMD_PATH}",
    "padding": 0,
    "refreshInterval": 30
  }
}
EOF
)

backup_settings() {
  if [ -f "$SETTINGS_PATH" ]; then
    cp "$SETTINGS_PATH" "${SETTINGS_PATH}.bak" \
      && info "$(t backed_up "${SETTINGS_PATH}.bak")"
  fi
}

# Atomically write the paste snippet as a fresh settings.json (used only when
# no settings.json exists and neither python3 nor jq is available). Staged in
# TMP_DIR so an interrupted write can never leave a partial settings.json.
write_minimal_settings() {
  local tmp="${TMP_DIR}/settings.new.json"
  printf '%s\n' "$PASTE_SNIPPET" > "$tmp" || die "$(t stage_new_failed)"
  mv -f "$tmp" "$SETTINGS_PATH" || die "$(t write_settings_failed "$SETTINGS_PATH")"
  info "$(t wrote_new_settings)"
}

log "$(t configuring)"

if command -v python3 >/dev/null 2>&1; then
  # python3: read-modify-write, preserving every existing key. The temp file
  # is staged in TMP_DIR (auto-cleaned by the EXIT trap) and atomically moved
  # into place, so a mid-write failure never orphans a *.tmp in CONFIG_DIR.
  #
  # The two user-facing error strings python3 may print are localized here and
  # passed in via env so the embedded program stays a pure read-modify-write.
  backup_settings
  MSG_INVALID_JSON="$(t py_invalid_json '%s')"
  MSG_NOT_OBJECT="$(t py_not_object)"
  SETTINGS_PATH="$SETTINGS_PATH" CMD_PATH="$CMD_PATH" TMP_DIR="$TMP_DIR" \
  MSG_INVALID_JSON="$MSG_INVALID_JSON" MSG_NOT_OBJECT="$MSG_NOT_OBJECT" \
    python3 - <<'PY' || die "$(t py_update_failed)"
import json, os, sys

path    = os.environ["SETTINGS_PATH"]
cmd     = os.environ["CMD_PATH"]
tmp_dir = os.environ["TMP_DIR"]

# Localized, runtime-selected error strings (English fallback applied by the
# shell before exec). MSG_INVALID_JSON carries a single %s for the parse error.
msg_invalid_json = os.environ["MSG_INVALID_JSON"]
msg_not_object   = os.environ["MSG_NOT_OBJECT"]

data = {}
if os.path.exists(path) and os.path.getsize(path) > 0:
    # utf-8-sig transparently strips a UTF-8 BOM (common on Windows-edited
    # files) and is a no-op when none is present.
    with open(path, "r", encoding="utf-8-sig") as f:
        raw = f.read()
    if raw.strip() == "":
        # Non-empty but whitespace-only (e.g. a partial prior write): treat as
        # absent and write a fresh, valid file. A .bak was already taken.
        data = {}
    else:
        try:
            data = json.loads(raw)
        except Exception as e:
            sys.stderr.write((msg_invalid_json % e) + "\n")
            sys.exit(1)
        if not isinstance(data, dict):
            sys.stderr.write(msg_not_object + "\n")
            sys.exit(1)

data["statusLine"] = {"type": "command", "command": cmd, "padding": 0, "refreshInterval": 30}

tmp = os.path.join(tmp_dir, "settings.merged.json")
try:
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
        f.write("\n")
    os.replace(tmp, path)
except Exception:
    try:
        os.remove(tmp)
    except OSError:
        pass
    raise
PY
  info "$(t updated_statusline "${SETTINGS_PATH}")"

elif command -v jq >/dev/null 2>&1; then
  # jq: read-modify-write, preserving every existing key. The merged output is
  # staged in TMP_DIR (auto-cleaned by the trap) and moved into place.
  backup_settings
  if [ -f "$SETTINGS_PATH" ] && [ -s "$SETTINGS_PATH" ]; then
    INPUT="$SETTINGS_PATH"
  else
    # Empty/absent -> feed an empty object so jq has a base to merge into.
    printf '{}' > "${TMP_DIR}/empty.json"
    INPUT="${TMP_DIR}/empty.json"
  fi
  MERGED="${TMP_DIR}/settings.merged.json"
  if jq --arg cmd "$CMD_PATH" \
       '.statusLine = {type:"command", command:$cmd, padding:0, refreshInterval:30}' \
       "$INPUT" > "$MERGED"; then
    mv -f "$MERGED" "$SETTINGS_PATH" \
      || die "$(t move_settings_failed "$SETTINGS_PATH")"
    info "$(t updated_statusline "${SETTINGS_PATH}")"
  else
    die "$(t jq_failed "${SETTINGS_PATH}")"
  fi

else
  # Neither python3 nor jq available.
  if [ -f "$SETTINGS_PATH" ]; then
    backup_settings
    log ""
    log "$(t warn_no_tool_1)"
    log "$(t warn_no_tool_2)"
    log "$(t warn_no_tool_3 "${SETTINGS_PATH}")"
    log "$(t warn_no_tool_4)"
    log ""
    printf '%s\n' "$PASTE_SNIPPET"
    log ""
  else
    write_minimal_settings
  fi
fi

# ---------------------------------------------------------------------------
# success + preview
# ---------------------------------------------------------------------------
log ""
log "$(t success)"
info "$(t label_binary "${INSTALL_PATH}")"
info "$(t label_settings2 "${SETTINGS_PATH}")"
log ""
log "$(t preview_header)"
log ""

# The CWD is interpolated raw into the mock JSON; a CWD containing a double
# quote or backslash (legal on Linux/macOS) would produce malformed JSON. The
# preview is cosmetic, but strip those characters so it always parses.
CWD_RAW="$(pwd 2>/dev/null || echo "$HOME_DIR")"
CWD="$(printf '%s' "$CWD_RAW" | tr -d '"\\')"
MOCK_JSON='{"model":{"display_name":"Opus 4.8 (1M context)"},"workspace":{"current_dir":"'"${CWD}"'"},"context_window":{"used_percentage":42,"total_input_tokens":84000,"context_window_size":200000},"cost":{"total_cost_usd":1.23,"total_duration_ms":185000,"total_lines_added":10,"total_lines_removed":2},"version":"2.1.161","effort":{"level":"high"},"session_id":"install-preview"}'

if printf '%s' "$MOCK_JSON" | COLUMNS=120 "$INSTALL_PATH"; then
  :
else
  log ""
  err "$(t preview_failed "${INSTALL_PATH}")"
fi

log ""
log "$(t restart_hint)"
