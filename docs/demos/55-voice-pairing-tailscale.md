---
layout: page
title: "Demo 55: Voice, Pairing & Tailscale"
permalink: /demos/55-voice-pairing-tailscale/
nav_order: 55
parent: Demos
---


## Overview

VibeCody supports three collaboration and accessibility modes: voice input for hands-free coding via Groq Whisper transcription, QR-code pairing with the VibeMobile companion app for on-the-go access, and Tailscale Funnel for securely exposing your VibeCLI daemon to the internet without port forwarding or DNS configuration. Together these features let you interact with your AI coding assistant from anywhere -- by voice at your desk, from your phone on the go, or from a remote machine through a secure tunnel.

**Time to complete:** ~10 minutes

## Prerequisites

- VibeCLI v0.5.1 installed and on your PATH
- A working microphone (for voice mode)
- A GROQ_API_KEY environment variable set (for Whisper transcription)
- VibeMobile app installed on your phone (for pairing)
- Tailscale installed and authenticated (for Tailscale Funnel)

## Step-by-Step Walkthrough

### Step 1: Start Voice Mode

Launch VibeCLI with the `--voice` flag to enable real-time voice input. VibeCody records audio from your microphone, sends it to Groq Whisper for transcription, and feeds the transcribed text into the AI as if you had typed it.

```bash
vibecli --voice
```

Expected output:

```
VibeCLI v0.5.1 - Voice Mode
Provider: claude (claude-sonnet-4-6)
Transcription: Groq Whisper (whisper-large-v3)

Listening... (press Space to talk, Esc to exit)

[Recording] ████████████░░░░░░░░ 3.2s

Transcribed: "Add error handling to the parse config function"

Sending to AI...

I'll add error handling to the `parse_config` function. Let me look at
the current implementation first.

[tool_use: read_file src/config.rs]
...
```

Voice mode works in both the REPL and one-shot mode:

```bash
vibecli --voice "refactor the database module"
```

```
Listening... (press Space to talk, Esc to stop)

[Recording] ████████████████░░░░ 4.1s

Transcribed: "refactor the database module to use connection pooling"

Working on it...
```

### Step 2: Configure Voice Settings

Inside the REPL, use `/voice` to manage voice settings.

```bash
vibecli
```

```
/voice
```

```
Voice Configuration
  Backend:     Groq Whisper (whisper-large-v3)
  Sample rate: 16000 Hz
  Silence:     1.5s threshold
  Language:    auto-detect
  Status:      Ready

Subcommands:
  /voice on       Enable voice input
  /voice off      Disable voice input
  /voice lang en  Set language hint
  /voice test     Record and play back a test clip
```

Test your microphone setup:

```
/voice test
```

```
Recording test clip... speak now.

[Recording] ████████████████████ 5.0s

Transcribed: "This is a microphone test"
Confidence:  0.97
Latency:     280ms

Voice input is working correctly.
```

### Step 3: Pair with VibeMobile

Use the `/pair` command to generate a QR code that links your phone to the running VibeCLI session. The VibeMobile companion app (available for iOS and Android) connects over your local network or through Tailscale.

```
/pair
```

```
Pairing Code Generated

  ┌─────────────────────────────┐
  │  ██ ▀▀██ ██▀▀ ▀█▀▀██ ██   │
  │  ██ ████ ██ ████ ████ ██   │
  │  ██ ▀▀▀▀ █▀█▀█▀█ ▀▀▀▀ ██  │
  │  ███████ █ ▀ █ █ ███████   │
  │  ▀▀▀▀▀▀▀ █▀█▀▀▀█ ▀▀▀▀▀▀▀  │
  │  ██▀█▀██ ███▀▀▀██ █▀█▀██   │
  │  ▀▀▀▀▀▀▀ ▀▀▀▀▀▀▀▀ ▀▀▀▀▀▀  │
  └─────────────────────────────┘

  Session:  pair-a7f3c2e1
  Expires:  5 minutes
  Network:  192.168.1.42:7878

  1. Open VibeMobile on your phone
  2. Tap "Pair with Desktop"
  3. Scan this QR code

Waiting for connection...
```

Once the phone connects:

```
Device Connected
  Device:   iPhone 15 Pro (VibeMobile v1.2.0)
  User:     alice
  Network:  Local (192.168.1.42)
  Latency:  12ms

You can now send messages from VibeMobile.
Type /pair status to view connected devices.
```

### Step 4: Expose via Tailscale Funnel

Start the VibeCLI HTTP daemon with Tailscale Funnel to make it accessible from anywhere on the internet through a secure, auto-provisioned HTTPS URL.

```bash
vibecli --serve --tailscale
```

```
VibeCLI HTTP Daemon
  Local:      http://localhost:7878
  Tailscale:  https://alice-macbook.tail1234.ts.net:7878
  Funnel:     https://alice-macbook.tail1234.ts.net (public)
  Auth:       Tailscale identity (MagicDNS)

  Funnel is active. Anyone with the URL can access VibeCLI.
  Use --tailscale-auth to require Tailscale login.

Ready for connections.
```

You can also require authentication:

```bash
vibecli --serve --tailscale --tailscale-auth
```

```
VibeCLI HTTP Daemon
  Local:      http://localhost:7878
  Tailscale:  https://alice-macbook.tail1234.ts.net:7878
  Funnel:     https://alice-macbook.tail1234.ts.net (authenticated)
  Auth:       Tailscale identity required

  Only users on your Tailnet can access this instance.
```

### Step 5: Discover Peers on the Network

Use `/discover` in the REPL to find other VibeCLI instances on your local network via mDNS.

```
/discover
```

```
Discovering VibeCLI instances on the local network...

Found 3 instances:

  Host                    Port   Version  Provider   Status
  alice-macbook.local     7878   0.5.1    claude     idle
  bob-desktop.local       7878   0.5.1    openai     busy (agent running)
  ci-runner-01.local      7880   0.5.1    ollama     idle

Connect with: /pair connect <host>
```

### Step 6: View in VibeUI

The voice, pairing, and network features are also available in VibeUI:

- **Voice button** in the chat toolbar activates microphone input
- **Pair** menu item under Settings shows the QR code in a modal
- **Network** status bar indicator shows Tailscale Funnel status and connected devices

## Demo Recording JSON

```json
{
  "meta": {
    "title": "Voice, Pairing & Tailscale",
    "description": "Hands-free voice coding, mobile pairing, and secure remote access.",
    "duration_seconds": 240,
    "version": "0.5.1"
  },
  "steps": [
    {
      "id": 1,
      "action": "shell",
      "command": "vibecli --voice",
      "description": "Start voice mode with Groq Whisper transcription",
      "delay_ms": 5000
    },
    {
      "id": 2,
      "action": "repl",
      "commands": [
        { "input": "/voice test", "delay_ms": 6000 },
        { "input": "/pair", "delay_ms": 4000 },
        { "input": "/discover", "delay_ms": 5000 },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "Test voice, generate pairing QR code, discover network peers"
    },
    {
      "id": 3,
      "action": "shell",
      "command": "vibecli --serve --tailscale",
      "description": "Start HTTP daemon with Tailscale Funnel",
      "delay_ms": 5000
    }
  ]
}
```

## What's Next

- [Demo 56: Browser-Based Web Client](../56-web-client/) -- Access VibeCLI from any browser
- [Demo 22: Gateway Messaging](../22-gateway/) -- Use VibeCody on Slack, Discord, and 16 more platforms
- [Demo 01: First Run & Setup](../01-first-run/) -- Initial installation and provider configuration
