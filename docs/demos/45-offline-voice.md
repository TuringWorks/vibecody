---
layout: page
title: "Demo 45: Offline Voice Coding"
permalink: /demos/45-offline-voice/
---


## Overview

VibeCody supports voice-driven coding through two modes: cloud-based transcription via Groq Whisper (fast, requires internet) and fully offline transcription via whisper.cpp (private, no network needed). This demo focuses on the offline mode -- ideal for air-gapped environments, sensitive codebases, or situations where you simply want to code by voice without any data leaving your machine.

**Time to complete:** ~10 minutes

## Prerequisites

- VibeCLI 0.5.1 installed and on your PATH
- A working microphone
- At least one AI provider configured (Ollama recommended for fully offline operation)
- For VibeUI: the desktop app running with the **VoiceLocalPanel** visible
- Disk space: ~150 MB for the small model, ~1.5 GB for the large model

## Whisper Model Sizes

| Model    | Size    | Speed     | Accuracy | Best for                     |
|----------|---------|-----------|----------|------------------------------|
| tiny     | 39 MB   | Very fast | ★★☆☆☆   | Quick commands, low-resource  |
| base     | 74 MB   | Fast      | ★★★☆☆   | General coding, short phrases |
| small    | 244 MB  | Medium    | ★★★★☆   | Recommended default           |
| medium   | 769 MB  | Slow      | ★★★★☆   | Longer dictation              |
| large-v3 | 1.5 GB  | Slowest   | ★★★★★   | Maximum accuracy              |

## Step-by-Step Walkthrough

### Step 1: Download a whisper.cpp model

Start VibeCLI and download the small model (recommended):

```bash
vibecli
```

```
> /voice model download small
```

```
[VoiceLocal] Downloading whisper.cpp model: small
[VoiceLocal] Source: huggingface.co/ggerganov/whisper.cpp (ggml-small.bin)
[VoiceLocal] Size: 244 MB

Downloading... ████████████████████████████████████████ 100% (244/244 MB)

[VoiceLocal] Model saved: ~/.vibecli/voice/models/ggml-small.bin
[VoiceLocal] SHA-256 verified ✓
[VoiceLocal] Ready for offline transcription.
```

List available models:

```
> /voice model list
```

```
Whisper Models — ~/.vibecli/voice/models/
═════════════════════════════════════════

Model    │ Status      │ Size    │ Path
─────────┼─────────────┼─────────┼──────────────────────────
tiny     │ not found   │ 39 MB   │ —
base     │ not found   │ 74 MB   │ —
small    │ ✓ installed │ 244 MB  │ ~/.vibecli/voice/models/ggml-small.bin
medium   │ not found   │ 769 MB  │ —
large-v3 │ not found   │ 1.5 GB  │ —

Active model: small
```

### Step 2: Start offline voice mode

Activate local voice transcription:

```
> /voice local
```

```
[VoiceLocal] Starting offline voice mode...
[VoiceLocal] Model: small (244 MB)
[VoiceLocal] Backend: whisper.cpp (CPU)
[VoiceLocal] Language: auto-detect
[VoiceLocal] Microphone: MacBook Pro Microphone (default)

🎤 Listening... (speak a command, or say "stop listening" to exit)
```

### Step 3: Issue voice commands

Speak naturally. VibeCody transcribes your speech locally and executes commands:

```
🎤 You said: "Create a new function called calculate total that takes a list of prices"

[VoiceLocal] Transcription time: 0.8s (local whisper.cpp)
[VoiceLocal] Confidence: 0.94

Generating code...

fn calculate_total(prices: &[f64]) -> f64 {
    prices.iter().sum()
}

Applied to src/lib.rs (appended at line 42)
```

Continue with follow-up commands:

```
🎤 You said: "Add a unit test for that function with three test cases"

[VoiceLocal] Transcription time: 0.6s
[VoiceLocal] Confidence: 0.97

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_total_basic() {
        assert_eq!(calculate_total(&[10.0, 20.0, 30.0]), 60.0);
    }

    #[test]
    fn test_calculate_total_empty() {
        assert_eq!(calculate_total(&[]), 0.0);
    }

    #[test]
    fn test_calculate_total_single() {
        assert_eq!(calculate_total(&[42.5]), 42.5);
    }
}

Applied to src/lib.rs (appended at line 46)
```

### Step 4: Use voice for REPL commands

You can speak slash commands too:

```
🎤 You said: "Slash help"

[VoiceLocal] Interpreted as: /help
[Executing /help...]

Available REPL Commands:
  /help           Show this help message
  /model          Switch model
  /clear          Clear conversation
  ...
```

### Step 5: Compare cloud vs local transcription

Check how offline whisper.cpp compares to cloud Groq Whisper:

```
> /voice compare
```

```
Voice Transcription Comparison
══════════════════════════════

                      │ Local (whisper.cpp)   │ Cloud (Groq Whisper)
──────────────────────┼───────────────────────┼──────────────────────
Model                 │ small (244 MB)        │ whisper-large-v3
Latency (avg)         │ 0.7s                  │ 0.3s
Accuracy (WER)        │ ~8.2%                 │ ~4.1%
Internet required     │ No                    │ Yes
Data leaves machine   │ No                    │ Yes (audio sent to Groq)
Cost per minute       │ Free                  │ ~$0.006
GPU acceleration      │ Optional (Metal/CUDA) │ N/A (server-side)
Max audio length      │ Unlimited             │ 25 MB / ~25 min

Recommendation:
  Security-sensitive work → Local (whisper.cpp)
  Maximum accuracy        → Cloud (Groq Whisper) or local large-v3
  Air-gapped environment  → Local (whisper.cpp) + Ollama
```

### Step 6: Configure voice settings

Tune transcription parameters:

```
> /voice config language en
> /voice config silence-threshold 500
> /voice config model large-v3
```

```
Voice configuration updated:
  Language:          en (English)
  Silence threshold: 500ms (pause before processing)
  Model:             large-v3 (not downloaded — run /voice model download large-v3)
```

### Step 7: Stop voice mode

Say "stop listening" or press Ctrl+C:

```
🎤 You said: "Stop listening"

[VoiceLocal] Voice mode deactivated.
[VoiceLocal] Session stats:
  Duration:        4m 23s
  Utterances:      6
  Avg latency:     0.7s
  Avg confidence:  0.95
  Total audio:     ~45s processed
  Cost:            $0.00 (fully local)
```

### Step 8: Fully air-gapped setup

For completely offline operation (no internet at all), pair whisper.cpp with Ollama:

```bash
# Pre-download everything while online
vibecli
> /voice model download small
> /quit

ollama pull codellama

# Now disconnect from the internet
# Everything works offline
vibecli
> /voice local
🎤 Listening... (fully offline: whisper.cpp + Ollama/codellama)
```

### Step 9: Use offline voice in VibeUI

In the VibeUI desktop app, open the **VoiceLocalPanel** from the AI sidebar. The panel provides:

- **Waveform Display** -- Real-time audio waveform visualization
- **Transcript Log** -- Scrollable history of all transcriptions with confidence scores
- **Model Manager** -- Download, delete, and switch between whisper.cpp models
- **Settings** -- Language, silence threshold, GPU acceleration toggle

## Demo Recording

```json
{
  "meta": {
    "title": "Offline Voice Coding",
    "description": "Code with voice commands using local whisper.cpp transcription.",
    "duration_seconds": 200,
    "version": "0.5.1"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/voice model download small", "delay_ms": 10000 },
        { "input": "/voice model list", "delay_ms": 2000 },
        { "input": "/voice local", "delay_ms": 3000 }
      ],
      "description": "Download model and start offline voice mode"
    },
    {
      "id": 2,
      "action": "voice_input",
      "utterances": [
        { "text": "Create a new function called calculate total that takes a list of prices", "delay_ms": 8000 },
        { "text": "Add a unit test for that function", "delay_ms": 8000 },
        { "text": "Stop listening", "delay_ms": 2000 }
      ],
      "description": "Simulated voice commands for code generation"
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/voice compare", "delay_ms": 3000 },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "Compare local vs cloud transcription"
    }
  ]
}
```

## What's Next

- [Demo 46: Code Replay](../46-code-replay/) -- Replay past agent sessions for debugging and auditing
- [Demo 44: Visual Verification](../44-visual-verify/) -- Screenshot-based design compliance checking
- [Demo 1: First Run & Setup](../01-first-run/) -- Initial VibeCody installation and configuration
