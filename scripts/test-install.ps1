#Requires -Version 5.1
# Run the installer against an isolated config and a local release archive.
# Exercise both UTF-8 text piped to iex and native -File decoding on each shell.
$ErrorActionPreference = 'Stop'
$hudTestRepo = Split-Path -Parent $PSScriptRoot
$hudTestInstaller = Join-Path $PSScriptRoot 'install.ps1'
$hudTestBytes = [IO.File]::ReadAllBytes($hudTestInstaller)
if ($hudTestBytes.Length -eq 0 -or $hudTestBytes[0] -ne 35) { throw 'Installer must begin with #, without a BOM.' }
if (@($hudTestBytes | Where-Object { $_ -gt 127 }).Count -ne 0) { throw 'Installer must remain ASCII for WinPS 5.1 file decoding.' }
$hudTestSource = [Text.Encoding]::UTF8.GetString($hudTestBytes)
$hudTestTokens = $null
$hudTestErrors = $null
[void][Management.Automation.Language.Parser]::ParseFile($hudTestInstaller, [ref]$hudTestTokens, [ref]$hudTestErrors)
if ($hudTestErrors.Count -ne 0) { throw ($hudTestErrors | Out-String) }

$hudTestRoot = Join-Path (Join-Path $hudTestRepo 'target/installer-tests') ([Guid]::NewGuid().ToString('N'))
$hudTestTemp = Join-Path $hudTestRoot 'temp'
$hudTestConfig = Join-Path $hudTestRoot 'config'
$hudTestInstall = Join-Path $hudTestRoot 'bin'
$hudTestTriple = 'x86_64-pc-windows-msvc'
$hudTestStage = Join-Path $hudTestRoot 'stage'
$hudTestStageTarget = Join-Path $hudTestStage $hudTestTriple
foreach ($hudTestDirectory in @($hudTestTemp, $hudTestConfig, $hudTestStageTarget)) {
    New-Item -ItemType Directory -Path $hudTestDirectory -Force | Out-Null
}
$hudTestBinary = Join-Path $hudTestRepo "dist/$hudTestTriple/claude-hud.exe"
Copy-Item -LiteralPath $hudTestBinary -Destination (Join-Path $hudTestStageTarget 'claude-hud.exe')
Add-Type -AssemblyName System.IO.Compression.FileSystem
$hudTestArchive = Join-Path $hudTestRoot 'release.zip'
[IO.Compression.ZipFile]::CreateFromDirectory($hudTestStage, $hudTestArchive)
$hudTestSettings = Join-Path $hudTestConfig 'settings.json'
$hudTestOriginal = '{"fixture":{"keep":[{"nested":[1,2]}]},"statusLine":{"type":"command","command":"old-command"}}'
[IO.File]::WriteAllText($hudTestSettings, $hudTestOriginal, [Text.UTF8Encoding]::new($false))

# Only the archive download is stubbed. Extraction, binary execution, config
# update, backup preservation, and cleanup run through the real installer.
function Invoke-WebRequest {
    param([string]$Uri, [switch]$UseBasicParsing, [string]$OutFile)
    $hudTestExpectedUrl = 'https://github.com/sj817/claude-code-plugin-hud/releases/download/v0.4.0/claude-code-plugin-hud-x86_64-pc-windows-msvc.zip'
    if ($Uri -ne $hudTestExpectedUrl) { throw "Unexpected download URL: $Uri" }
    if (-not [IO.Path]::GetFullPath($OutFile).StartsWith($hudTestTemp + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Download escaped the isolated test directory.'
    }
    Copy-Item -LiteralPath $hudTestArchive -Destination $OutFile
}

$hudTestEnvironment = @{}
foreach ($hudTestKey in @('CLAUDE_CONFIG_DIR','CLAUDE_HUD_DIR','CLAUDE_HUD_VERSION','CLAUDE_HUD_LANG','TEMP','TMP','COLUMNS','LINES','CLAUDE_HUD_ONELINE')) {
    $hudTestEnvironment[$hudTestKey] = [Environment]::GetEnvironmentVariable($hudTestKey, 'Process')
}
$hudTestPreviousExitCode = $global:LASTEXITCODE
try {
    $env:CLAUDE_CONFIG_DIR = $hudTestConfig
    $env:CLAUDE_HUD_DIR = $hudTestInstall
    $env:CLAUDE_HUD_VERSION = 'v0.4.0'
    $env:CLAUDE_HUD_LANG = 'zh'
    # Installer cleanup is confined to this verified directory, never real user files.
    $env:TEMP = $hudTestTemp
    $env:TMP = $hudTestTemp
    $env:COLUMNS = '91'
    $env:LINES = '24'
    $env:CLAUDE_HUD_ONELINE = '0'
    foreach ($hudTestMode in @('pipeline','file')) {
        $global:LASTEXITCODE = 0
        $hudTestOutput = if ($hudTestMode -eq 'pipeline') {
            & { $hudTestSource | Invoke-Expression } 6>&1 | Out-String
        } else {
            & $hudTestInstaller 6>&1 | Out-String
        }
        if ($global:LASTEXITCODE -ne 0) { throw "Installer failed in $hudTestMode mode: $hudTestOutput" }
        # 'Installation succeeded' in Chinese, represented as ASCII codepoints.
        $hudTestSuccess = -join ([char[]]@(0x5B89,0x88C5,0x6210,0x529F))
        if (-not $hudTestOutput.Contains($hudTestSuccess)) { throw "Chinese output was not preserved: $hudTestMode" }
        $hudTestInstalled = Join-Path $hudTestInstall 'claude-hud.exe'
        if ((Get-FileHash -LiteralPath $hudTestBinary).Hash -ne (Get-FileHash -LiteralPath $hudTestInstalled).Hash) { throw 'Installed binary differs from the archive.' }
        $hudTestParsed = [IO.File]::ReadAllText($hudTestSettings) | ConvertFrom-Json
        if ($hudTestParsed.statusLine.command -ne $hudTestInstalled.Replace('\','/') -or $hudTestParsed.statusLine.padding -ne 0 -or $hudTestParsed.statusLine.refreshInterval -ne 30) { throw 'Unexpected statusLine configuration.' }
        if (($hudTestParsed.fixture | ConvertTo-Json -Depth 10 -Compress) -ne '{"keep":[{"nested":[1,2]}]}') { throw 'Unrelated settings changed.' }
        if ([IO.File]::ReadAllText($hudTestSettings + '.bak') -ne $hudTestOriginal) { throw 'Original settings backup changed.' }
        if ($env:COLUMNS -ne '91') { throw 'Preview did not restore terminal width.' }
        # The preview binary legitimately keeps its short-lived Git caches.
        $hudTestLeftovers = @(Get-ChildItem -LiteralPath $hudTestTemp -Force | Where-Object { $_.Name -match '^claude-hud-([0-9a-f]{32}(\.zip)?|preview-[0-9a-f]{32}\.json)$' })
        if ($hudTestLeftovers.Count -ne 0) { throw 'Installer temporary files were not cleaned up.' }
    }
    Write-Host "Installer regression passed: PowerShell $($PSVersionTable.PSVersion), pipeline/file, Chinese, install, settings, backup and cleanup."
} finally {
    foreach ($hudTestKey in $hudTestEnvironment.Keys) {
        [Environment]::SetEnvironmentVariable($hudTestKey, $hudTestEnvironment[$hudTestKey], 'Process')
    }
    $global:LASTEXITCODE = $hudTestPreviousExitCode
}
