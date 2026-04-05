---
layout: page
title: "Windows"
permalink: /guides/windows/
parent: Deployment Guides
---

# Deploy VibeCody on Windows

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
