<#
.SYNOPSIS
  Stage the vibecli daemon as a Tauri sidecar for the Windows installers.

.DESCRIPTION
  Until this ran, a Windows machine that installed only VibeCoder.msi had no
  daemon at all: `daemon_bootstrap::find_binary_in` probes for a sibling
  `vibecli.exe` beside the app — "a binary shipped alongside the app bundle's
  executable" — and nothing had ever put one there. The app could not autostart
  and every HTTP-backed panel was dead until the user found the separate CLI
  zip.

  Tauri resolves `externalBin` entries by target triple, so the binary is copied
  to `<shell>/src-tauri/binaries/vibecli-<triple>.exe` and installed next to the
  app executable as `vibecli.exe`.

.PARAMETER Shell
  Which shell(s) to stage. Defaults to all three.

.PARAMETER DaemonExe
  The built daemon. Defaults to the workspace release build.

.PARAMETER Triple
  Rust target triple. Defaults to the host's, read from rustc.

.EXAMPLE
  cargo build --release -p vibecli
  ./scripts/stage-daemon-sidecar.ps1
#>
[CmdletBinding()]
param(
  [ValidateSet('vibecoder', 'vibedesk', 'vibeaichat')]
  [string[]] $Shell = @('vibecoder', 'vibedesk', 'vibeaichat'),

  [string] $DaemonExe,
  [string] $Triple
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$RepoRoot = Split-Path -Parent $PSScriptRoot

if (-not $DaemonExe) { $DaemonExe = Join-Path $RepoRoot 'target\release\vibecli.exe' }
if (-not (Test-Path $DaemonExe)) {
  throw "No daemon at $DaemonExe — run: cargo build --release -p vibecli"
}

# `rustc -vV` is the only authority that agrees with what Tauri looks for; a
# hard-coded x86_64 triple silently produces an installer with no daemon on an
# arm64 runner.
if (-not $Triple) {
  $Triple = (& rustc -vV | Select-String '^host: ' | ForEach-Object { $_.Line.Substring(6).Trim() })
  if (-not $Triple) { throw "Could not read the host triple from rustc -vV" }
}

foreach ($s in $Shell) {
  $dst = Join-Path $RepoRoot "$s\src-tauri\binaries"
  New-Item -ItemType Directory -Force -Path $dst | Out-Null
  $target = Join-Path $dst "vibecli-$Triple.exe"
  Copy-Item $DaemonExe $target -Force
  $mb = [math]::Round((Get-Item $target).Length / 1MB, 1)
  Write-Host "  staged   $s/src-tauri/binaries/vibecli-$Triple.exe  ($mb MB)"
}

Write-Host "daemon sidecar staged for $Triple"
