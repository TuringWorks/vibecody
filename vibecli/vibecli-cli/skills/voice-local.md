# Voice Local

Offline voice coding using local whisper.cpp for speech-to-text. No cloud API calls, no data leaves your machine. Supports voice commands, code dictation, and natural language instructions with configurable wake words.

## When to Use
- Coding hands-free when away from the keyboard
- Dictating code or comments using voice in air-gapped environments
- Using voice commands to navigate, edit, and run code
- Accessibility scenarios requiring speech input
- Reducing repetitive strain by alternating between typing and voice

## Commands
- `/voice start` — Start listening with local whisper.cpp
- `/voice stop` — Stop listening and disable voice input
- `/voice model <size>` — Select whisper model (tiny, base, small, medium, large)
- `/voice wake <word>` — Set the wake word (default: "hey vibe")
- `/voice language <lang>` — Set recognition language (default: en)
- `/voice devices` — List available audio input devices
- `/voice device <id>` — Select a specific audio input device
- `/voice test` — Run a quick recognition test to check quality

## Examples
```
/voice start
# Listening with whisper-small (local, offline). Wake word: "hey vibe"
# Say "hey vibe" followed by your command.

"hey vibe, add error handling to the parse function"
# Recognized: "add error handling to the parse function"
# Executing: Adding error handling to parse()...

/voice model medium
# Switched to whisper-medium (1.5GB, higher accuracy, ~2s latency)

/voice test
# Say something... "The quick brown fox"
# Recognized: "The quick brown fox" (confidence: 0.96, latency: 340ms)
```

## Best Practices
- Start with the small model for a good balance of speed and accuracy
- Use a quality microphone to improve recognition accuracy significantly
- Set a distinctive wake word to avoid accidental activations
- Speak commands in short clear phrases rather than long paragraphs
- Test recognition quality in your environment before relying on voice for edits
