#Requires -Version 5.1
<#
.SYNOPSIS
    VibeCody Windows installer.
.PARAMETER Tier
    Resource tier: lite, pro, max (default: lite)
.PARAMETER AlwaysOn
    Install as a Windows Scheduled Task for always-on operation
#>
param(
    [ValidateSet("lite","pro","max")][string]$Tier = "lite",
    [switch]$AlwaysOn,
    [switch]$Help
)

if ($Help) {
    Write-Host "VibeCody Windows Setup"
    Write-Host "  -Tier lite|pro|max  Resource tier (default: lite)"
    Write-Host "  -AlwaysOn           Install as background service"
    exit 0
}

$ErrorActionPreference = "Stop"
$InstallDir = "$env:LOCALAPPDATA\VibeCody"
$Repo = "TuringWorks/vibecody"

Write-Host "`n  VibeCody Windows Setup" -ForegroundColor Cyan
Write-Host "  =====================`n"

# Detect architecture
$Arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { Write-Error "64-bit Windows required"; exit 1 }
Write-Host "  [OK] Windows $Arch detected" -ForegroundColor Green

# Get latest release
Write-Host "  Fetching latest release..."
$Release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
$Version = $Release.tag_name
$Asset = $Release.assets | Where-Object { $_.name -like "*windows*.zip" } | Select-Object -First 1
if (-not $Asset) { Write-Error "No Windows binary found in release $Version"; exit 1 }

# Download
$TmpDir = Join-Path $env:TEMP "vibecody-install"
New-Item -ItemType Directory -Path $TmpDir -Force | Out-Null
$ZipPath = Join-Path $TmpDir $Asset.name
Write-Host "  Downloading $($Asset.name)..."
Invoke-WebRequest -Uri $Asset.browser_download_url -OutFile $ZipPath

# Extract and install
New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
Expand-Archive -Path $ZipPath -DestinationPath $InstallDir -Force
Remove-Item $TmpDir -Recurse -Force

Write-Host "  [OK] Installed to $InstallDir" -ForegroundColor Green

# Add to PATH
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$InstallDir;$UserPath", "User")
    Write-Host "  [OK] Added to PATH" -ForegroundColor Green
}

# Optionally install Ollama
$OllamaInstalled = Get-Command ollama -ErrorAction SilentlyContinue
if (-not $OllamaInstalled) {
    $InstallOllama = Read-Host "  Install Ollama for local AI models? [Y/n]"
    if ($InstallOllama -ne "n") {
        Write-Host "  Installing Ollama via winget..."
        winget install --id Ollama.Ollama --accept-package-agreements --accept-source-agreements
    }
}

# Always-on mode
if ($AlwaysOn) {
    Write-Host "`n  Setting up always-on service..."
    $Action = New-ScheduledTaskAction -Execute "$InstallDir\vibecli.exe" -Argument "serve --port 7878 --host 0.0.0.0"
    $Trigger = New-ScheduledTaskTrigger -AtStartup
    $Settings = New-ScheduledTaskSettingsSet -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1)
    Register-ScheduledTask -TaskName "VibeCody" -Action $Action -Trigger $Trigger -Settings $Settings -RunLevel Highest -Force
    Start-ScheduledTask -TaskName "VibeCody"
    Write-Host "  [OK] VibeCody service registered and started" -ForegroundColor Green
    Write-Host "  Access: http://localhost:7878"
}

Write-Host "`n  Setup complete!" -ForegroundColor Green
Write-Host "  Run: vibecli"
Write-Host "  Docs: https://vibecody.github.io/vibecody/guides/windows/`n"
