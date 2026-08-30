---
layout: page
title: "Windows"
permalink: /guides/windows/
parent: Deployment Guides
---

PowerShell installer with optional always-on Scheduled Task.

**Setup time:** 3 minutes | **Cost:** Free | **GPU:** CUDA (NVIDIA)

## Quick Start

```powershell
irm https://raw.githubusercontent.com/TuringWorks/vibecody/main/deploy/windows/setup.ps1 | iex
```

Or with always-on mode:

```powershell
.\setup.ps1 -AlwaysOn -Tier pro
```

## Do I need this if I installed VibeCoder, VibeDesk or VibeAIChat?

No. The desktop installers (`.msi` / `.exe`) carry the daemon with them: each
shell declares `vibecli` as a Tauri `externalBin` sidecar in its
`tauri.windows.conf.json`, and the release build stages the freshly built
`vibecli.exe` into it (`scripts/stage-daemon-sidecar.ps1`). Windows lays that
sidecar down next to the app executable, which is one of the directories
`daemon_bootstrap::find_binary_in` probes, so the app autostarts its own daemon
on first launch with nothing else installed.

Use the PowerShell installer below when you want the daemon or the CLI on its
own — a headless machine, an always-on service via Scheduled Task, `vibecli`
in a terminal, or one daemon shared by several clients. A daemon already
listening on port 7878 is reused rather than duplicated, so the two installs
coexist: whichever is on `PATH` wins, and the bundled sidecar is the fallback.

Note that `setup.ps1` writes `PATH` to the user's registry environment. A
program that was already running — including Explorer, and therefore anything
launched from the Start menu since — keeps the environment block it started
with, so a freshly installed `vibecli.exe` is not on the `PATH` those programs
see until you sign out and back in. Autostart also probes
`%LOCALAPPDATA%\VibeCody` directly for exactly this reason.

## Step-by-Step

### 1. Run the Installer

```powershell
cd vibecody\deploy\windows
.\setup.ps1
```

This will:
- Download the latest `vibecli.exe` from GitHub Releases
- Verify SHA256 checksum
- Install to `%LOCALAPPDATA%\VibeCody\`
- Add to your PATH

### 2. Install Ollama (Optional)

```powershell
winget install Ollama.Ollama
ollama pull codellama:7b
```

### 3. Enable Always-On (Optional)

```powershell
.\setup.ps1 -AlwaysOn
```

This creates a Windows Scheduled Task that runs VibeCody at startup.

## Uninstall

```powershell
.\uninstall.ps1
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Execution policy error | Run: `Set-ExecutionPolicy RemoteSigned -Scope CurrentUser` |
| PATH not updated | Restart your terminal or run `refreshenv` |
| Service won't start | Check Task Scheduler for the "VibeCody" task |

## What's Next

- [Use Cases](/vibecody/use-cases/) | [Configuration](/vibecody/configuration/)
