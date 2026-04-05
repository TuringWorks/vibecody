---
layout: page
title: "Linux"
permalink: /guides/linux/
parent: Deployment Guides
---

# Deploy VibeCody on Linux

Supports Ubuntu, Debian, Fedora, Arch, and most distributions. Optional CUDA/ROCm GPU acceleration.

**Setup time:** 2 minutes | **Cost:** Free | **GPU:** CUDA (NVIDIA) / ROCm (AMD)

## Quick Start

```bash
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh
vibecli
```

Or the full setup with always-on service:

```bash
cd vibecody/deploy/linux
./setup.sh --always-on
```

## Step-by-Step

### 1. Install Dependencies

```bash
# Ubuntu/Debian
sudo apt install curl pkg-config libssl-dev

# Fedora
sudo dnf install curl openssl-devel

# Arch
sudo pacman -S curl openssl
```

### 2. Install VibeCLI

```bash
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh
```

### 3. Install Ollama (Optional)

```bash
curl -fsSL https://ollama.com/install.sh | sh
ollama pull codellama:7b
```

### 4. Enable Always-On (Optional)

```bash
# Using our setup script:
./setup.sh --always-on

# Or manually:
mkdir -p ~/.config/systemd/user
cp deploy/linux/vibecody.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now vibecody.service
```

Check status: `systemctl --user status vibecody`

## GPU Setup

### NVIDIA (CUDA)

```bash
# Install NVIDIA drivers and container toolkit
sudo apt install nvidia-driver-535 nvidia-container-toolkit
ollama pull codellama:7b  # Ollama auto-detects CUDA
```

### AMD (ROCm)

```bash
sudo apt install rocm-dev
# Ollama supports ROCm on compatible GPUs
```

## Uninstall

```bash
./uninstall.sh
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| TLS errors building from source | Install `pkg-config` and `libssl-dev` |
| Service fails to start | Check: `journalctl --user -u vibecody -f` |
| Ollama not using GPU | Verify: `nvidia-smi` or check `/dev/kfd` for AMD |
| Binary not found | Add `~/.local/bin` to PATH |

## What's Next

- [Use Cases](/vibecody/use-cases/) | [Configuration](/vibecody/configuration/)
