#Requires -Version 5.1
<#
.SYNOPSIS
    Windows-native dispatcher for the day-to-day Make targets.

.DESCRIPTION
    Run from cmd.exe:        powershell -NoProfile -ExecutionPolicy Bypass -File scripts\dev.ps1 <target>
    Run from PowerShell:     .\scripts\dev.ps1 <target>

    Use `.\scripts\dev.ps1 help` to list targets.
#>

[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [ValidateSet(
        'cli', 'cli-run', 'ui', 'app',
        'check', 'lint', 'fmt', 'fmt-check',
        'test', 'test-fast', 'test-cli', 'test-ai', 'test-core',
        'build', 'build-ui', 'build-app',
        'clean', 'help'
    )]
    [string]$Target = 'help'
)

$ErrorActionPreference = 'Continue'
$script:RepoRoot = Split-Path -Parent $PSScriptRoot

function Invoke-Step {
    param([string]$Name, [scriptblock]$Block)
    & $Block
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[dev.ps1] step '$Name' exited with $LASTEXITCODE" -ForegroundColor Red
        exit $LASTEXITCODE
    }
}

function In-Subdir {
    param([string]$Dir, [scriptblock]$Block)
    Push-Location (Join-Path $script:RepoRoot $Dir)
    try { & $Block } finally { Pop-Location }
}

function Show-Help {
    @'
Usage: scripts\dev.ps1 <target>

Development:
  cli           Build VibeCLI release binary -> target\release\vibecli.exe
  cli-run       Build and run VibeCLI with TUI
  ui            Run VibeUI in dev mode (Vite + Tauri)
  app           Run VibeCLI App in dev mode

Quality:
  check         Fast type-check (cargo check + tsc --noEmit)
  lint          cargo clippy + tsc --noEmit
  fmt           cargo fmt --all
  fmt-check     cargo fmt --check (no modifications)

Testing:
  test          cargo test --workspace
  test-fast     cargo test --workspace --exclude vibe-collab
  test-cli      cargo test -p vibecli
  test-ai       cargo test -p vibe-ai
  test-core     cargo test -p vibe-core

Building:
  build         CLI + VibeUI + VibeCLI App (release)
  build-ui      VibeUI production bundle
  build-app     VibeCLI App production bundle

Cleanup:
  clean         cargo clean + remove dist + .vite caches

Environment:
  Run scripts\doctor.ps1 to verify prerequisites.
  Run scripts\setup.ps1 to install missing prerequisites.
'@ | Write-Host
}

Set-Location $script:RepoRoot

switch ($Target) {
    'help'      { Show-Help }

    'cli' {
        Invoke-Step 'cargo build vibecli' { cargo build --release -p vibecli }
        $exe = Join-Path $script:RepoRoot 'target\release\vibecli.exe'
        if (Test-Path $exe) {
            $sz = '{0:N1} MB' -f ((Get-Item $exe).Length / 1MB)
            Write-Host ''
            Write-Host "Binary: $exe ($sz)"
        }
    }

    'cli-run'   { Invoke-Step 'cargo run vibecli --tui' { cargo run --release -p vibecli -- --tui } }

    'ui'        { In-Subdir 'vibeui'  { Invoke-Step 'vibeui tauri:dev'  { npm run tauri:dev } } }
    'app'       { In-Subdir 'vibeapp' { Invoke-Step 'vibeapp tauri:dev' { npm run tauri:dev } } }

    'check' {
        Invoke-Step 'cargo check' { cargo check --workspace --exclude vibe-collab }
        In-Subdir 'vibeui' { Invoke-Step 'vibeui tsc' { npx tsc --noEmit } }
    }

    'lint' {
        Invoke-Step 'cargo clippy' { cargo clippy --workspace --exclude vibe-collab -- -D warnings }
        In-Subdir 'vibeui' { Invoke-Step 'vibeui tsc' { npx tsc --noEmit } }
    }

    'fmt'        { Invoke-Step 'cargo fmt'       { cargo fmt --all } }
    'fmt-check'  { Invoke-Step 'cargo fmt --check' { cargo fmt --all -- --check } }

    'test'       { Invoke-Step 'cargo test'      { cargo test --workspace } }
    'test-fast'  { Invoke-Step 'cargo test fast' { cargo test --workspace --exclude vibe-collab } }
    'test-cli'   { Invoke-Step 'cargo test -p vibecli'   { cargo test -p vibecli } }
    'test-ai'    { Invoke-Step 'cargo test -p vibe-ai'   { cargo test -p vibe-ai } }
    'test-core'  { Invoke-Step 'cargo test -p vibe-core' { cargo test -p vibe-core } }

    'build' {
        Invoke-Step 'cargo build vibecli' { cargo build --release -p vibecli }
        In-Subdir 'vibeui'  { Invoke-Step 'vibeui tauri:build'  { npm run tauri:build } }
        In-Subdir 'vibeapp' { Invoke-Step 'vibeapp tauri:build' { npm run tauri:build } }
    }
    'build-ui'   { In-Subdir 'vibeui'  { Invoke-Step 'vibeui tauri:build'  { npm run tauri:build } } }
    'build-app'  { In-Subdir 'vibeapp' { Invoke-Step 'vibeapp tauri:build' { npm run tauri:build } } }

    'clean' {
        Invoke-Step 'cargo clean' { cargo clean }
        $paths = @(
            'vibeui\dist',
            'vibeui\node_modules\.vite',
            'vibeapp\dist',
            'vibeapp\node_modules\.vite'
        )
        foreach ($p in $paths) {
            $full = Join-Path $script:RepoRoot $p
            if (Test-Path $full) {
                Remove-Item -Recurse -Force $full
                Write-Host "  removed $p"
            }
        }
    }
}
