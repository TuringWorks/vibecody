#Requires -Version 5.1
<#
.SYNOPSIS
    Verifies the development environment is ready (Windows-native parity for `make doctor`).

.DESCRIPTION
    Run from cmd.exe:        powershell -NoProfile -ExecutionPolicy Bypass -File scripts\doctor.ps1
    Run from PowerShell:     pwsh scripts/doctor.ps1   (or)   ./scripts/doctor.ps1
#>

[CmdletBinding()]
param()

$ErrorActionPreference = 'Continue'
$script:RepoRoot = Split-Path -Parent $PSScriptRoot
$script:MissingRequired = 0

function Show-Check {
    param([string]$Label, [string]$Value)
    Write-Host ("  {0,-20}{1}" -f $Label, $Value)
}

function Get-CommandVersion {
    param(
        [Parameter(Mandatory)] [string]$Command,
        [string[]]$VersionArgs = @('--version'),
        [switch]$MergeStderr
    )
    if (-not (Get-Command $Command -ErrorAction SilentlyContinue)) { return $null }
    try {
        if ($MergeStderr) {
            $out = & $Command @VersionArgs 2>&1 | Select-Object -First 1
        } else {
            $out = & $Command @VersionArgs 2>$null | Select-Object -First 1
        }
        if ($null -eq $out) { return $null }
        return ($out | Out-String).Trim()
    } catch {
        return $null
    }
}

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

function Get-MsvcBuildToolsVersion {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path $vswhere)) { return $null }
    $ver = & $vswhere -latest -products * `
        -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
        -property installationVersion 2>$null
    if ($LASTEXITCODE -ne 0 -or -not $ver) { return $null }
    return ($ver | Select-Object -First 1).Trim()
}

function Get-JdkStatus {
    $pinFile = Join-Path $script:RepoRoot 'vibewatch\VibeCodyWear\.java-version'
    if (Test-Path $pinFile) {
        $pin = (Get-Content $pinFile -Raw).Trim()
        $pinMajor = ($pin -split '\.')[0]
        if ($pinMajor -eq '17' -or $pinMajor -eq '21') {
            return "pinned to $pin via .java-version (compatible with AGP 8.7.3)"
        }
        return "pinned to $pin -- INCOMPATIBLE with AGP 8.7.3 (install JDK 21)"
    }
    if (-not (Get-Command java -ErrorAction SilentlyContinue)) {
        return 'MISSING -- install: winget install EclipseAdoptium.Temurin.21.JDK'
    }
    $verLine = (& java -version 2>&1 | Select-Object -First 1) | Out-String
    $m = [regex]::Match($verLine, '"(\d+)')
    if (-not $m.Success) { return 'present (could not parse version)' }
    $major = $m.Groups[1].Value
    if ($major -eq '17' -or $major -eq '21') {
        return "no pin; current java is $major (compatible)"
    }
    return "no pin; current java is $major -- INCOMPATIBLE with AGP 8.7.3"
}

function Check-Required {
    param([string]$Label, [string]$Command, [string]$MissingHint)
    $v = Get-CommandVersion -Command $Command
    if ($v) {
        Show-Check $Label $v
    } else {
        Show-Check $Label "MISSING -- $MissingHint"
        $script:MissingRequired++
    }
}

function Check-Optional {
    param([string]$Label, [string]$Command, [string]$AbsentMessage = 'not installed (optional)')
    $v = Get-CommandVersion -Command $Command
    if ($v) { Show-Check $Label $v } else { Show-Check $Label $AbsentMessage }
}

Write-Host 'Checking development environment...'
Write-Host ''

Check-Required 'Rust:'    'rustc' "winget install Rustlang.Rustup (then 'rustup default stable-msvc')"
Check-Required 'Cargo:'   'cargo' 'comes with Rust toolchain'
Check-Required 'Node.js:' 'node'  'winget install OpenJS.NodeJS.LTS'
Check-Required 'npm:'     'npm'   'comes with Node.js'
Check-Required 'Git:'     'git'   'winget install Git.Git'

Check-Optional 'Ollama:' 'ollama'
Check-Optional 'Docker:' 'docker'

Show-Check 'JDK (watch-wear):' (Get-JdkStatus)

Check-Optional 'Flutter:' 'flutter' 'not installed (needed for mobile-*)'

Write-Host ''
Write-Host 'Windows -- checking Tauri system dependencies...'

$wv2 = Test-WebView2Runtime
if ($wv2) {
    Show-Check 'WebView2 Runtime:' $wv2
} else {
    Show-Check 'WebView2 Runtime:' 'MISSING -- download Evergreen installer from https://developer.microsoft.com/microsoft-edge/webview2/'
    $script:MissingRequired++
}

$msvc = Get-MsvcBuildToolsVersion
if ($msvc) {
    Show-Check 'MSVC Build Tools:' $msvc
} else {
    Show-Check 'MSVC Build Tools:' 'MISSING -- install Visual Studio 2022 Build Tools with "Desktop development with C++"'
    $script:MissingRequired++
}

Write-Host ''
Write-Host 'Required: Rust, Cargo, Node.js, npm, Git, WebView2 Runtime, MSVC Build Tools'
Write-Host 'Optional: Ollama (local AI), Docker (container sandbox), JDK 17/21 (watch-wear), Flutter (mobile-*)'

if ($script:MissingRequired -gt 0) {
    Write-Host ''
    Write-Host ("{0} required component(s) missing." -f $script:MissingRequired) -ForegroundColor Red
    exit 1
}
exit 0
