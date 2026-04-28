#Requires -Version 5.1
<#
.SYNOPSIS
    VibeCody Developer Setup Script (Windows-native parity for scripts/setup.sh).

.DESCRIPTION
    Installs / verifies all prerequisites for building VibeCody from source on Windows.
    Safe to re-run -- skips anything already installed.

    Run from cmd.exe:        powershell -NoProfile -ExecutionPolicy Bypass -File scripts\setup.ps1
    Run from PowerShell:     .\scripts\setup.ps1
#>

[CmdletBinding()]
param(
    [switch]$InstallMsvcBuildTools
)

$ErrorActionPreference = 'Stop'
$ProgressPreference    = 'SilentlyContinue'
$script:RepoRoot = Split-Path -Parent $PSScriptRoot

function Info($m) { Write-Host "[info]  $m" -ForegroundColor Cyan }
function Ok($m)   { Write-Host "[ok]    $m" -ForegroundColor Green }
function Warn($m) { Write-Host "[warn]  $m" -ForegroundColor Yellow }
function Fail($m) { Write-Host "[error] $m" -ForegroundColor Red; exit 1 }

$arch = if ([Environment]::Is64BitOperatingSystem) { 'x64' } else { 'x86' }
Info "Detected: windows ($arch)"

$script:Winget = $null
if (Get-Command winget -ErrorAction SilentlyContinue) {
    $script:Winget = (Get-Command winget).Source
} else {
    Warn 'winget not found. Manual install URLs will be printed for missing components.'
    Warn 'Install winget via: https://aka.ms/getwinget'
}

function Invoke-Winget {
    param([string]$Id, [string[]]$ExtraArgs = @())
    if (-not $script:Winget) { return $false }
    Info "winget install --id $Id"
    & winget install --id $Id --silent --accept-package-agreements --accept-source-agreements @ExtraArgs
    return ($LASTEXITCODE -eq 0)
}

# ── Rust ──────────────────────────────────────────────────────────────────────

if (Get-Command rustc -ErrorAction SilentlyContinue) {
    $rv = ((& rustc --version) -split ' ')[1]
    Ok "Rust $rv already installed"
} else {
    Info 'Installing Rust via rustup...'
    if ($script:Winget) {
        if (-not (Invoke-Winget 'Rustlang.Rustup')) {
            Fail 'Rustup install via winget failed. Download manually: https://rustup.rs'
        }
    } else {
        $rustupExe = Join-Path $env:TEMP 'rustup-init.exe'
        Info "Downloading rustup-init.exe -> $rustupExe"
        Invoke-WebRequest -Uri 'https://win.rustup.rs/x86_64' -OutFile $rustupExe -UseBasicParsing
        & $rustupExe -y --default-toolchain stable
        if ($LASTEXITCODE -ne 0) { Fail 'rustup-init.exe failed' }
    }
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
    Ok "Rust $(& rustc --version) installed"
}

if (Get-Command rustup -ErrorAction SilentlyContinue) {
    & rustup default stable-msvc 2>$null | Out-Null
}

# ── Node.js ───────────────────────────────────────────────────────────────────

$NODE_MIN = 18
if (Get-Command node -ErrorAction SilentlyContinue) {
    $nodeRaw = (& node -v).TrimStart('v')
    $nodeMajor = [int](($nodeRaw -split '\.')[0])
    if ($nodeMajor -ge $NODE_MIN) {
        Ok "Node.js v$nodeRaw already installed"
    } else {
        Warn "Node.js v$nodeRaw is below minimum v$NODE_MIN"
        Info '  upgrade:  winget upgrade --id OpenJS.NodeJS.LTS'
    }
} else {
    Info 'Installing Node.js LTS...'
    if ($script:Winget) {
        if (-not (Invoke-Winget 'OpenJS.NodeJS.LTS')) {
            Warn 'Node.js install via winget failed. Download from https://nodejs.org/'
        } else {
            Ok 'Node.js LTS installed (open a new shell to pick up PATH)'
        }
    } else {
        Warn 'winget unavailable. Install Node.js LTS manually: https://nodejs.org/'
    }
}

# ── Git ───────────────────────────────────────────────────────────────────────

if (Get-Command git -ErrorAction SilentlyContinue) {
    Ok "$((& git --version)) already installed"
} else {
    Info 'Installing Git...'
    if ($script:Winget) { Invoke-Winget 'Git.Git' | Out-Null }
    else { Warn 'Install Git manually: https://git-scm.com/download/win' }
}

# ── WebView2 Runtime (Tauri requirement on Windows) ───────────────────────────

function Test-WebView2Runtime {
    $keys = @(
        'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
        'HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
        'HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
    )
    foreach ($k in $keys) {
        $pv = (Get-ItemProperty -Path $k -Name pv -ErrorAction SilentlyContinue).pv
        if ($pv -and $pv -ne '0.0.0.0') { return $pv }
    }
    return $null
}

$wv2 = Test-WebView2Runtime
if ($wv2) {
    Ok "WebView2 Runtime $wv2 already installed"
} else {
    Info 'Installing WebView2 Runtime (Evergreen)...'
    if ($script:Winget) { Invoke-Winget 'Microsoft.EdgeWebView2Runtime' | Out-Null }
    else { Warn 'Install from https://developer.microsoft.com/microsoft-edge/webview2/' }
}

# ── MSVC C++ Build Tools (Tauri + cargo on Windows) ───────────────────────────

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$msvcVer = $null
if (Test-Path $vswhere) {
    $msvcVer = & $vswhere -latest -products * `
        -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
        -property installationVersion 2>$null
}

if ($msvcVer) {
    Ok "MSVC C++ Build Tools $msvcVer already installed"
} elseif ($InstallMsvcBuildTools) {
    if ($script:Winget) {
        Info 'Installing Visual Studio 2022 Build Tools (~3 GB; UAC prompt expected)...'
        & winget install --id Microsoft.VisualStudio.2022.BuildTools `
            --override "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --quiet --wait" `
            --accept-package-agreements --accept-source-agreements
        if ($LASTEXITCODE -ne 0) { Warn 'Build Tools install exited non-zero; verify manually.' }
    } else {
        Warn 'winget unavailable. Download Build Tools: https://aka.ms/vs/17/release/vs_BuildTools.exe'
    }
} else {
    Warn 'MSVC C++ Build Tools not detected. Required for cargo + Tauri on Windows.'
    Info 'To install (~3 GB), re-run with -InstallMsvcBuildTools, or run manually:'
    Info '  winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"'
    Info 'Or download: https://aka.ms/vs/17/release/vs_BuildTools.exe'
}

# ── npm install (vibeui + vibeapp) ────────────────────────────────────────────

foreach ($sub in @('vibeui', 'vibeapp')) {
    $pkg = Join-Path $script:RepoRoot "$sub\package.json"
    if (Test-Path $pkg) {
        Info "Installing $sub frontend dependencies..."
        Push-Location (Join-Path $script:RepoRoot $sub)
        try {
            & npm install --no-audit --no-fund
            if ($LASTEXITCODE -eq 0) { Ok "$sub npm dependencies installed" }
            else { Warn "$sub npm install exited with $LASTEXITCODE" }
        } finally { Pop-Location }
    }
}

# ── Summary ───────────────────────────────────────────────────────────────────

function Show-Version($label, $cmd) {
    if (Get-Command $cmd -ErrorAction SilentlyContinue) {
        $v = (& $cmd --version 2>&1 | Select-Object -First 1)
        Write-Host ("  {0,-9}{1}" -f "${label}:", $v)
    } else {
        Write-Host ("  {0,-9}not found" -f "${label}:")
    }
}

Write-Host ''
Write-Host '================================' -ForegroundColor Green
Write-Host '  VibeCody Setup Complete!'      -ForegroundColor Green
Write-Host '================================' -ForegroundColor Green
Write-Host ''
Show-Version 'Rust'    'rustc'
Show-Version 'Cargo'   'cargo'
Show-Version 'Node.js' 'node'
Show-Version 'npm'     'npm'
Write-Host ''
Write-Host 'Next steps:'
Write-Host ''
Write-Host '  # Verify environment'
Write-Host '  .\scripts\doctor.ps1'
Write-Host ''
Write-Host '  # Build VibeCLI'
Write-Host '  .\scripts\dev.ps1 cli'
Write-Host ''
Write-Host '  # Run VibeUI in dev mode'
Write-Host '  .\scripts\dev.ps1 ui'
Write-Host ''
Write-Host '  # Run all tests'
Write-Host '  .\scripts\dev.ps1 test'
Write-Host ''
Write-Host '  # See all available targets'
Write-Host '  .\scripts\dev.ps1 help'
Write-Host ''
