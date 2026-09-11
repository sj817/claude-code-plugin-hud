#Requires -Version 5.1
<#
.SYNOPSIS
    Installs the claude-code-plugin-hud statusline binary (claude-hud) on Windows
    and wires it into Claude Code's settings.json.

.DESCRIPTION
    Designed to be run via:  irm <raw-url> | iex

    Because that pipeline cannot pass arguments, all options come from env vars:
        $env:CLAUDE_HUD_VERSION   - "latest" (default), or "v0.1.6" / "0.1.6"
        $env:CLAUDE_HUD_DIR       - install directory override
        $env:CLAUDE_CONFIG_DIR    - Claude config dir (default <home>\.claude)
        $env:CLAUDE_HUD_LANG      - "zh" forces Simplified Chinese; any other
                                    non-empty value forces English. Unset means
                                    auto-detect from the UI culture.

    Re-running is safe: it just refreshes the binary and re-points statusLine.

    NOTE on failure handling: when this script is piped into `iex`, calling
    `exit` would terminate the user's entire terminal session. So we NEVER call
    `exit` -- on failure we print a clear message, clean up, set
    $global:LASTEXITCODE = 1, and `return`. Callers using
    `powershell -File install.ps1` can still inspect $LASTEXITCODE.
#>

$ErrorActionPreference = 'Stop'

# --- Force UTF-8 console output so localized (e.g. Chinese) text renders -------
# Guarded: some hosts disallow changing the console encoding.
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch { }

# --- Localization -------------------------------------------------------------
# Language selection (English is the universal fallback):
#   * Explicit override first: $env:CLAUDE_HUD_LANG. A value starting with "zh"
#     (any case) -> zh; any other non-empty value -> en.
#   * Otherwise auto-detect from (Get-UICulture).TwoLetterISOLanguageName: 'zh'
#     -> zh, anything else -> en. Wrapped in try/catch, defaulting to en.
function Resolve-Lang {
    $override = $env:CLAUDE_HUD_LANG
    if ($override) {
        if ($override.ToLowerInvariant().StartsWith('zh')) { return 'zh' }
        return 'en'
    }
    try {
        $iso = (Get-UICulture).TwoLetterISOLanguageName
        if ($iso -eq 'zh') { return 'zh' }
        return 'en'
    } catch {
        return 'en'
    }
}
$lang = Resolve-Lang

# Message catalog. en is complete (the fallback); zh mirrors every key. At
# runtime a missing zh entry falls back to the en entry. Placeholders use the
# {0},{1},... [string]::Format convention; interpolated values (paths, URLs,
# codes, triples) are never translated.
$MSG = @{
    en = @{
        warn_tls          = 'Warning: could not force TLS 1.2; continuing anyway.'
        header_title      = 'claude-code-plugin-hud installer'
        header_rule       = '================================'
        unsupported_arch  = "Unsupported processor architecture '{0}'. claude-hud supports x86_64 (AMD64) and ARM64 on Windows."
        resolved_triple   = 'Resolved target triple: {0}'
        invalid_version   = "Invalid version '{0}'. Use 'latest' or a semver like '0.1.6' / 'v0.1.6'."
        version_line      = 'Version: {0}'
        download_url      = 'Download URL: {0}'
        no_home           = 'Could not determine home directory (HOME and USERPROFILE are both empty).'
        downloading       = 'Downloading...'
        download_failed   = "Download failed from {0}`n{1}"
        download_empty    = 'Downloaded file is missing or empty: {0}'
        verify_read_fail  = 'Could not read downloaded file to verify it: {0}'
        not_a_zip         = 'Downloaded file is not a zip (got an HTML or error page?). URL: {0}'
        extracting        = 'Extracting...'
        expand_fallback   = 'Expand-Archive unavailable or failed; falling back to .NET ZipFile.'
        extract_failed    = "Failed to extract archive '{0}'.`n{1}"
        binary_not_found  = 'Could not find {0} inside the extracted archive ({1}).'
        installing_to     = 'Installing to: {0}'
        install_failed    = 'Install failed: {0} was not created.'
        settings_parse    = "Existing settings.json could not be parsed as JSON/JSONC ({0}). Fix or remove it and re-run.`n{1}"
        backed_up         = 'Backed up existing settings to: {0}'
        backup_preserved  = 'Existing backup preserved: {0}'
        not_an_object     = 'settings.json did not parse to a JSON object; refusing to overwrite. Inspect: {0}'
        write_failed      = "Failed to write settings.json safely; the original file was left untouched.`n{0}"
        updated_statusln  = 'Updated statusLine in: {0}'
        success           = 'Success! claude-hud is installed and configured.'
        sum_triple        = '  Triple:        {0}'
        sum_url           = '  Download URL:  {0}'
        sum_install       = '  Install path:  {0}'
        sum_settings      = '  settings.json: {0}'
        restart_hint      = 'Restart Claude Code (or start a new session) to see the statusline.'
        preview_header    = 'Preview:'
        preview_failed    = 'Preview failed (the binary is still installed).'
        install_failed_hdr = 'Install failed: {0}'
    }
    zh = @{
        warn_tls          = '警告：无法强制启用 TLS 1.2，将继续尝试。'
        header_title      = 'claude-code-plugin-hud 安装程序'
        header_rule       = '================================'
        unsupported_arch  = "不支持的处理器架构 '{0}'。claude-hud 在 Windows 上仅支持 x86_64（AMD64）和 ARM64。"
        resolved_triple   = '已识别目标平台：{0}'
        invalid_version   = "无效的版本号 '{0}'。请使用 'latest' 或形如 '0.1.6' / 'v0.1.6' 的语义化版本号。"
        version_line      = '版本：{0}'
        download_url      = '下载地址：{0}'
        no_home           = '无法确定用户主目录（HOME 与 USERPROFILE 均为空）。'
        downloading       = '正在下载……'
        download_failed   = "从 {0} 下载失败`n{1}"
        download_empty    = '下载的文件缺失或为空：{0}'
        verify_read_fail  = '无法读取下载的文件以进行校验：{0}'
        not_a_zip         = '下载的文件不是 zip 压缩包（可能返回了 HTML 或错误页面？）。地址：{0}'
        extracting        = '正在解压……'
        expand_fallback   = 'Expand-Archive 不可用或执行失败，回退到 .NET ZipFile。'
        extract_failed    = "解压压缩包 '{0}' 失败。`n{1}"
        binary_not_found  = '在解压后的压缩包中未找到 {0}（{1}）。'
        installing_to     = '正在安装到：{0}'
        install_failed    = '安装失败：未能创建 {0}。'
        settings_parse    = "无法将现有的 settings.json 解析为 JSON/JSONC（{0}）。请修复或删除后重试。`n{1}"
        backed_up         = '已备份现有配置到：{0}'
        backup_preserved  = '已保留现有备份：{0}'
        not_an_object     = 'settings.json 未解析为 JSON 对象，已拒绝覆盖。请检查：{0}'
        write_failed      = "未能安全写入 settings.json，原文件保持不变。`n{0}"
        updated_statusln  = '已在以下文件中更新 statusLine：{0}'
        success           = '安装成功！claude-hud 已安装并完成配置。'
        sum_triple        = '  目标平台：    {0}'
        sum_url           = '  下载地址：    {0}'
        sum_install       = '  安装路径：    {0}'
        sum_settings      = '  settings.json：{0}'
        restart_hint      = '请重启 Claude Code（或新开一个会话）以查看状态栏。'
        preview_header    = '预览：'
        preview_failed    = '预览失败（二进制文件仍已安装）。'
        install_failed_hdr = '安装失败：{0}'
    }
}

# T 'key' [args...] -> localized string. Falls back to the en entry when the
# selected language lacks the key, then applies [string]::Format for any args.
function T {
    param([string]$Key)
    $fmt = $MSG[$lang][$Key]
    if ($null -eq $fmt) { $fmt = $MSG['en'][$Key] }
    if ($args.Count -gt 0) {
        return [string]::Format($fmt, $args)
    }
    return $fmt
}

# --- TLS 1.2 (required for downloads on Windows PowerShell 5.1) ----------------
try {
    [Net.ServicePointManager]::SecurityProtocol =
        [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
} catch {
    # Some locked-down hosts disallow setting this; the download may still work.
    Write-Host (T 'warn_tls') -ForegroundColor Yellow
}

# --- Small helpers ------------------------------------------------------------
function Write-Step { param([string]$Message) Write-Host $Message -ForegroundColor Cyan }
function Write-Info { param([string]$Message) Write-Host $Message -ForegroundColor Gray }
function Write-Ok   { param([string]$Message) Write-Host $Message -ForegroundColor Green }
function Fail       { param([string]$Message) throw $Message }

$REPO    = 'sj817/claude-code-plugin-hud'
$BINNAME = 'claude-hud.exe'

# Temp artifacts to clean up on exit.
$script:TempZip = $null
$script:TempDir = $null

function Remove-Temp {
    foreach ($p in @($script:TempZip, $script:TempDir)) {
        if ($p -and (Test-Path -LiteralPath $p)) {
            try { Remove-Item -LiteralPath $p -Recurse -Force -ErrorAction SilentlyContinue } catch { }
        }
    }
}

# Write text as UTF-8 WITHOUT a BOM, regardless of host edition.
# WinPS 5.1's `Set-Content -Encoding UTF8` emits a BOM (EF BB BF) which can
# break strict JSON parsers; the .NET call below is the portable BOM-less path.
function Write-Utf8NoBom {
    param([string]$Path, [string]$Text)
    $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, $Text, $utf8NoBom)
}

# Strip JSONC niceties (// and /* */ comments, trailing commas) so that
# WinPS 5.1's stricter ConvertFrom-Json can parse settings files that Claude
# Code itself tolerates. Conservative: skips comment-like sequences that appear
# inside string literals.
function ConvertFrom-Jsonc {
    param([string]$Raw)

    $sb       = New-Object System.Text.StringBuilder
    $inString = $false
    $escaped  = $false
    $i        = 0
    $len      = $Raw.Length

    while ($i -lt $len) {
        $c    = $Raw[$i]
        $next = if ($i + 1 -lt $len) { $Raw[$i + 1] } else { "`0" }

        if ($inString) {
            [void]$sb.Append($c)
            if ($escaped)        { $escaped = $false }
            elseif ($c -eq '\')  { $escaped = $true }
            elseif ($c -eq '"')  { $inString = $false }
            $i++
            continue
        }

        if ($c -eq '"') {
            $inString = $true
            [void]$sb.Append($c)
            $i++
            continue
        }

        # Line comment: // ... to end of line.
        if ($c -eq '/' -and $next -eq '/') {
            $i += 2
            while ($i -lt $len -and $Raw[$i] -ne "`n") { $i++ }
            continue
        }

        # Block comment: /* ... */
        if ($c -eq '/' -and $next -eq '*') {
            $i += 2
            while ($i -lt $len -and -not ($Raw[$i] -eq '*' -and ($i + 1 -lt $len) -and $Raw[$i + 1] -eq '/')) { $i++ }
            $i += 2
            continue
        }

        [void]$sb.Append($c)
        $i++
    }

    # Remove trailing commas before } or ] (handles intervening whitespace).
    $cleaned = [System.Text.RegularExpressions.Regex]::Replace($sb.ToString(), ',\s*([}\]])', '$1')
    return ($cleaned | ConvertFrom-Json -ErrorAction Stop)
}

try {
    Write-Host ""
    Write-Host (T 'header_title') -ForegroundColor White
    Write-Host (T 'header_rule') -ForegroundColor White

    # --- Resolve target triple from architecture -----------------------------
    # Prefer the OS architecture reported by .NET (reliable even when an x64
    # PowerShell runs under emulation on an ARM64 device). Fall back to env vars
    # on hosts where the API is unavailable. PROCESSOR_ARCHITEW6432 is set when a
    # 32-bit shell runs on a 64-bit OS and reflects the true OS architecture.
    $triple = $null
    try {
        $osArch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
        switch ($osArch) {
            'Arm64' { $triple = 'aarch64-pc-windows-msvc' }
            'X64'   { $triple = 'x86_64-pc-windows-msvc' }
        }
    } catch {
        # RuntimeInformation not available; fall through to env-var detection.
    }

    if (-not $triple) {
        $arch = $env:PROCESSOR_ARCHITEW6432
        if (-not $arch) { $arch = $env:PROCESSOR_ARCHITECTURE }
        if (-not $arch) { $arch = '' }
        $arch = $arch.ToUpperInvariant()

        switch ($arch) {
            'AMD64' { $triple = 'x86_64-pc-windows-msvc' }
            'ARM64' { $triple = 'aarch64-pc-windows-msvc' }
            default {
                Fail (T 'unsupported_arch' $arch)
            }
        }
    }
    Write-Step (T 'resolved_triple' $triple)

    # --- Resolve version + download URL --------------------------------------
    $version = $env:CLAUDE_HUD_VERSION
    if (-not $version) { $version = '' }
    $version = $version.Trim()

    $asset = "claude-code-plugin-hud-$triple.zip"

    if (-not $version -or $version -ieq 'latest') {
        $version = 'latest'
        $url = "https://github.com/$REPO/releases/latest/download/$asset"
    } else {
        # Normalize: pinned URL requires the leading 'v'. Accept "0.1.6" or "v0.1.6".
        $tag = $version
        if ($tag -match '^[vV]') { $tag = $tag.Substring(1) }
        $tag = "v$tag"

        # Validate the shape so a junk CLAUDE_HUD_VERSION fails clearly instead
        # of building a bogus pinned URL that 404s with a misleading message.
        if ($tag -notmatch '^v[0-9]+\.[0-9]+\.[0-9]+([-.+][0-9A-Za-z.-]+)?$') {
            Fail (T 'invalid_version' $env:CLAUDE_HUD_VERSION)
        }

        $version = $tag
        $url = "https://github.com/$REPO/releases/download/$tag/$asset"
    }
    Write-Step (T 'version_line' $version)
    Write-Info (T 'download_url' $url)

    # --- Resolve home + config dir + install dir -----------------------------
    $home_ = $HOME
    if (-not $home_) { $home_ = $env:USERPROFILE }
    if (-not $home_) { Fail (T 'no_home') }

    $configDir = $env:CLAUDE_CONFIG_DIR
    if (-not $configDir) { $configDir = Join-Path $home_ '.claude' }

    $installDir = $env:CLAUDE_HUD_DIR
    if (-not $installDir) { $installDir = Join-Path $configDir 'statusline' }

    $binPath      = Join-Path $installDir $BINNAME
    $settingsPath = Join-Path $configDir 'settings.json'

    # --- Download the release zip --------------------------------------------
    $tmpRoot = [System.IO.Path]::GetTempPath()
    $stamp   = [System.Guid]::NewGuid().ToString('N')
    $script:TempZip = Join-Path $tmpRoot ("claude-hud-$stamp.zip")
    $script:TempDir = Join-Path $tmpRoot ("claude-hud-$stamp")

    Write-Step (T 'downloading')
    try {
        Invoke-WebRequest -Uri $url -UseBasicParsing -OutFile $script:TempZip
    } catch {
        Fail (T 'download_failed' $url $_.Exception.Message)
    }
    if (-not (Test-Path -LiteralPath $script:TempZip) -or ((Get-Item -LiteralPath $script:TempZip).Length -le 0)) {
        Fail (T 'download_empty' $script:TempZip)
    }

    # Sniff the zip magic bytes (PK\x03\x04 -> 0x50 0x4B). Guards against an HTML
    # error/interstitial page being served with a 200 and "extracted" later.
    try {
        $fs    = [System.IO.File]::OpenRead($script:TempZip)
        $magic = New-Object byte[] 2
        [void]$fs.Read($magic, 0, 2)
        $fs.Close()
    } catch {
        Fail (T 'verify_read_fail' $_.Exception.Message)
    }
    if ($magic[0] -ne 0x50 -or $magic[1] -ne 0x4B) {
        Fail (T 'not_a_zip' $url)
    }

    # --- Extract --------------------------------------------------------------
    Write-Step (T 'extracting')
    $extracted = $false
    try {
        Expand-Archive -LiteralPath $script:TempZip -DestinationPath $script:TempDir -Force
        $extracted = $true
    } catch {
        # Expand-Archive may be absent (Server Core / trimmed images) or blocked.
        # Fall back to the always-available .NET ZipFile API.
        Write-Info (T 'expand_fallback')
    }

    if (-not $extracted) {
        try {
            Add-Type -AssemblyName System.IO.Compression.FileSystem -ErrorAction Stop
            if (Test-Path -LiteralPath $script:TempDir) {
                Remove-Item -LiteralPath $script:TempDir -Recurse -Force -ErrorAction SilentlyContinue
            }
            [System.IO.Compression.ZipFile]::ExtractToDirectory($script:TempZip, $script:TempDir)
        } catch {
            Fail (T 'extract_failed' $script:TempZip $_.Exception.Message)
        }
    }

    # Locate the binary. Prefer the documented <triple>/claude-hud.exe layout;
    # fall back to a recursive search to be robust to layout changes.
    $expected = Join-Path (Join-Path $script:TempDir $triple) $BINNAME
    if (Test-Path -LiteralPath $expected) {
        $found = Get-Item -LiteralPath $expected
    } else {
        $matches = @(Get-ChildItem -LiteralPath $script:TempDir -Recurse -File -Filter $BINNAME -ErrorAction SilentlyContinue)
        if ($matches.Count -eq 0) {
            Fail (T 'binary_not_found' $BINNAME $script:TempDir)
        }
        $found = $matches[0]
    }

    # --- Install --------------------------------------------------------------
    Write-Step (T 'installing_to' $binPath)
    if (-not (Test-Path -LiteralPath $installDir)) {
        New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    }
    Copy-Item -LiteralPath $found.FullName -Destination $binPath -Force
    if (-not (Test-Path -LiteralPath $binPath)) {
        Fail (T 'install_failed' $binPath)
    }

    # --- Configure settings.json ---------------------------------------------
    # The command path must use forward slashes so JSON/Claude treat it cleanly.
    $cmdPath = $binPath -replace '\\', '/'

    if (-not (Test-Path -LiteralPath $configDir)) {
        New-Item -ItemType Directory -Path $configDir -Force | Out-Null
    }

    $settings = $null
    if (Test-Path -LiteralPath $settingsPath) {
        $raw = $null
        try { $raw = Get-Content -LiteralPath $settingsPath -Raw -ErrorAction Stop } catch { $raw = $null }
        if ($raw -and $raw.Trim().Length -gt 0) {
            try {
                # Tolerant parse: Claude Code allows JSONC (comments / trailing
                # commas), and WinPS 5.1's ConvertFrom-Json rejects both.
                $settings = ConvertFrom-Jsonc -Raw $raw
            } catch {
                Fail (T 'settings_parse' $settingsPath $_.Exception.Message)
            }
        }

        # Back up the PRISTINE pre-install file once, and never overwrite it.
        # Re-running the installer must not clobber the last-known-good backup
        # with an already-modified settings.json.
        $backupPath = "$settingsPath.bak"
        if (-not (Test-Path -LiteralPath $backupPath)) {
            Copy-Item -LiteralPath $settingsPath -Destination $backupPath -Force
            Write-Info (T 'backed_up' $backupPath)
        } else {
            Write-Info (T 'backup_preserved' $backupPath)
        }
    }

    if (-not $settings) {
        # Empty/absent/whitespace -> start from an empty object.
        $settings = [PSCustomObject]@{}
    }

    # ConvertFrom-Json can yield non-object types if the file held e.g. an array.
    if ($settings -isnot [System.Management.Automation.PSCustomObject]) {
        Fail (T 'not_an_object' $settingsPath)
    }

    $statusLine = [PSCustomObject]@{
        type    = 'command'
        command = $cmdPath
        padding = 0
        refreshInterval = 30
    }

    # Set/replace the statusLine property, preserving every other key.
    if ($settings.PSObject.Properties.Name -contains 'statusLine') {
        $settings.statusLine = $statusLine
    } else {
        $settings | Add-Member -MemberType NoteProperty -Name 'statusLine' -Value $statusLine -Force
    }

    # CRITICAL: default ConvertTo-Json depth is 2 and silently truncates nested
    # settings to "System.Object[]" / "@{...}". Use a large depth.
    $json = $settings | ConvertTo-Json -Depth 100

    # Atomic-ish write: write to a temp file in the same directory, verify it
    # re-parses, then Move-Item over the live file. Prevents a partial write
    # (disk full / AV lock / permissions) from corrupting settings.json.
    $tmpSettings = "$settingsPath.tmp.$([System.Guid]::NewGuid().ToString('N'))"
    try {
        Write-Utf8NoBom -Path $tmpSettings -Text $json
        # Sanity check the bytes we just wrote actually parse.
        $verify = Get-Content -LiteralPath $tmpSettings -Raw -ErrorAction Stop
        [void]($verify | ConvertFrom-Json -ErrorAction Stop)
        Move-Item -LiteralPath $tmpSettings -Destination $settingsPath -Force
    } catch {
        if (Test-Path -LiteralPath $tmpSettings) {
            Remove-Item -LiteralPath $tmpSettings -Force -ErrorAction SilentlyContinue
        }
        Fail (T 'write_failed' $_.Exception.Message)
    }
    Write-Step (T 'updated_statusln' $settingsPath)

    # --- Summary --------------------------------------------------------------
    Write-Host ""
    Write-Ok   (T 'success')
    Write-Info (T 'sum_triple' $triple)
    Write-Info (T 'sum_url' $url)
    Write-Info (T 'sum_install' $binPath)
    Write-Info (T 'sum_settings' $settingsPath)
    Write-Host ""
    Write-Info (T 'restart_hint')

    # --- Preview --------------------------------------------------------------
    Write-Host ""
    Write-Step (T 'preview_header')
    $previewCwd = ($home_ -replace '\\', '/')
    $mock = '{"model":{"display_name":"Opus 4.8 (1M context)"},"workspace":{"current_dir":"' + $previewCwd + '"},"context_window":{"used_percentage":42,"total_input_tokens":84000,"context_window_size":200000},"cost":{"total_cost_usd":1.23,"total_duration_ms":185000,"total_lines_added":10,"total_lines_removed":2},"version":"2.1.161","effort":{"level":"high"},"session_id":"install-preview"}'

    # REQUIRED Windows preview fix: pipe via a BOM-less temp file + cmd
    # redirection. Piping a string straight to the native binary under Windows
    # PowerShell 5.1 prepends a UTF-8 BOM that breaks the JSON parser.
    #
    # Save the prior COLUMNS so the session is left as we found it. Note: $null
    # means the var was unset, so restore by REMOVING it (setting it to '' would
    # leave an empty var lingering in the session).
    $previewJson = Join-Path ([System.IO.Path]::GetTempPath()) ("claude-hud-preview-$([Guid]::NewGuid().ToString('N')).json")
    $hadColumns  = Test-Path Env:COLUMNS
    $prevColumns = if ($hadColumns) { $env:COLUMNS } else { $null }
    try {
        Write-Utf8NoBom -Path $previewJson -Text $mock
        $env:COLUMNS = '120'
        cmd /c "`"$binPath`" < `"$previewJson`""
    } catch {
        Write-Host (T 'preview_failed') -ForegroundColor Yellow
    } finally {
        if ($hadColumns) { $env:COLUMNS = $prevColumns } else { Remove-Item Env:COLUMNS -ErrorAction SilentlyContinue }
        Remove-Item -LiteralPath $previewJson -Force -ErrorAction SilentlyContinue
    }

    Write-Host ""
}
catch {
    Write-Host ""
    Write-Host (T 'install_failed_hdr' $_.Exception.Message) -ForegroundColor Red
    # IMPORTANT: do NOT call `exit` here. When this script is piped into `iex`,
    # `exit` terminates the user's entire terminal session before they can read
    # this message. Set a sentinel and return; `powershell -File` callers can
    # still inspect $LASTEXITCODE. Cleanup happens in finally.
    $global:LASTEXITCODE = 1
    return
}
finally {
    Remove-Temp
}
