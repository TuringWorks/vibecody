<#
.SYNOPSIS
  Stage the Windows speech engine and model into a shell's Tauri resources.

.DESCRIPTION
  The Windows installers ship a working speech engine so that voice needs no
  terminal step. Neither piece can live in git — the engine is a third-party
  release and the model is 465 MB — so the packaging step fetches them and
  drops them where `tauri.windows.conf.json` expects:

      <shell>/src-tauri/resources/whisper/   whisper-server.exe + its DLLs
      <shell>/src-tauri/resources/models/    ggml-<model>.bin

  Both land beside the installed daemon, which is the first place
  `whisper_bin_roots()` and `whisper_model_roots()` look.

  Downloads are cached in scripts/.voice-cache so building all three shells
  fetches once rather than three times.

.PARAMETER Shell
  Which shell(s) to stage. Defaults to all three that support duplex voice.

.PARAMETER Model
  GGML model name. `small` is the quality floor for non-Latin scripts —
  `base` renders Devanagari in Arabic script. See docs/voice-duplex.md.

.PARAMETER Force
  Re-copy even when the destination already looks complete.

.EXAMPLE
  ./scripts/fetch-voice-assets.ps1
  ./scripts/fetch-voice-assets.ps1 -Shell vibecoder -Model base
#>
[CmdletBinding()]
param(
  [ValidateSet('vibecoder', 'vibedesk', 'vibeaichat')]
  [string[]] $Shell = @('vibecoder', 'vibedesk', 'vibeaichat'),

  [ValidateSet('tiny', 'base', 'small', 'medium')]
  [string] $Model = 'small',

  [switch] $Force
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# Pinned, not "latest": a release that changes under CI turns a reproducible
# build into a lottery, and the DLL set is what the daemon spawns.
$WhisperTag = 'b4938'
$WhisperAsset = 'whisper-bin-x64.zip'

$RepoRoot = Split-Path -Parent $PSScriptRoot
$Cache = Join-Path $PSScriptRoot '.voice-cache'
New-Item -ItemType Directory -Force -Path $Cache | Out-Null

# Only what the server needs to run. The full archive also carries SDL2, the
# llama and parakeet binaries and the test suite — 30 MB of things no installer
# should be carrying. The `ggml-cpu-*` set is not optional: ggml picks one at
# runtime by CPU feature, so dropping any of them makes the engine fail on
# exactly the machines that variant existed for.
$EngineFiles = @(
  'whisper-server.exe',
  'whisper.dll',
  'ggml.dll',
  'ggml-base.dll'
)
$EngineGlobs = @('ggml-cpu-*.dll')

function Get-Cached {
  param([string] $Url, [string] $Name)
  $path = Join-Path $Cache $Name
  if ((Test-Path $path) -and -not $Force) {
    Write-Host "  cached   $Name"
    return $path
  }
  Write-Host "  fetching $Name"
  $tmp = "$path.partial"
  $prev = $ProgressPreference
  $ProgressPreference = 'SilentlyContinue'   # progress rendering costs ~10x here
  try {
    Invoke-WebRequest -Uri $Url -OutFile $tmp -TimeoutSec 1800
    Move-Item $tmp $path -Force              # never leave a truncated file cached
  } finally {
    $ProgressPreference = $prev
    Remove-Item $tmp -ErrorAction SilentlyContinue
  }
  return $path
}

# ── engine ────────────────────────────────────────────────────────────────────
$zip = Get-Cached `
  -Url "https://github.com/ggml-org/whisper.cpp/releases/download/$WhisperTag/$WhisperAsset" `
  -Name "whisper-$WhisperTag-x64.zip"

$engine = Join-Path $Cache "whisper-$WhisperTag"
if ($Force -or -not (Test-Path (Join-Path $engine 'Release\whisper-server.exe'))) {
  Remove-Item $engine -Recurse -Force -ErrorAction SilentlyContinue
  Expand-Archive -Path $zip -DestinationPath $engine -Force
}
$release = Join-Path $engine 'Release'

# MIT, and redistributing the binaries means redistributing the terms.
$license = Get-Cached `
  -Url "https://raw.githubusercontent.com/ggml-org/whisper.cpp/$WhisperTag/LICENSE" `
  -Name 'whisper.cpp-LICENSE.txt'

# ── model ─────────────────────────────────────────────────────────────────────
$modelFile = "ggml-$Model.bin"
$modelPath = Get-Cached `
  -Url "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/$modelFile" `
  -Name $modelFile

# ── stage ─────────────────────────────────────────────────────────────────────
foreach ($s in $Shell) {
  $resources = Join-Path $RepoRoot "$s\src-tauri\resources"
  $binDir = Join-Path $resources 'whisper'
  $modelDir = Join-Path $resources 'models'
  New-Item -ItemType Directory -Force -Path $binDir, $modelDir | Out-Null

  foreach ($f in $EngineFiles) {
    $src = Join-Path $release $f
    if (-not (Test-Path $src)) { throw "$WhisperAsset ($WhisperTag) has no $f — the asset layout changed" }
    Copy-Item $src $binDir -Force
  }
  foreach ($g in $EngineGlobs) {
    $hits = @(Get-ChildItem (Join-Path $release $g) -ErrorAction SilentlyContinue)
    if ($hits.Count -eq 0) { throw "$WhisperAsset ($WhisperTag) matched no $g — ggml would have no CPU backend" }
    $hits | Copy-Item -Destination $binDir -Force
  }
  Copy-Item $license (Join-Path $binDir 'LICENSE-whisper.cpp.txt') -Force

  # Copy only when absent or stale: the model is 465 MB and CI stages it three
  # times, once per shell.
  $modelDst = Join-Path $modelDir $modelFile
  if ($Force -or -not (Test-Path $modelDst) -or (Get-Item $modelDst).Length -ne (Get-Item $modelPath).Length) {
    Copy-Item $modelPath $modelDst -Force
  }

  $mb = [math]::Round(((Get-ChildItem $resources -Recurse -File | Measure-Object Length -Sum).Sum / 1MB), 1)
  Write-Host "  staged   $s/src-tauri/resources  ($mb MB)"
}

Write-Host "voice assets staged: whisper.cpp $WhisperTag + $modelFile"
