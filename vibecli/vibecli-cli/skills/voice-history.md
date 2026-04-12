# Voice Command History

Records, indexes, and replays voice commands with full-text search and confidence scoring. Matches Cody 6.0's voice command history feature.

## Key Types
- **VoiceHistory** — append-only log with capacity eviction
- **VoiceEntry** — raw text, normalized text, confidence, tags, executed flag
- **VoiceSearchResult** — scored search result with match positions
- **VoiceHistoryStats** — total / executed / avg_confidence / execution_rate

## Confidence Levels
| Score | Label |
|---|---|
| ≥ 0.9 | high |
| 0.7–0.9 | medium |
| < 0.7 | low |

## Commands
- `/voice history` — list recent voice commands
- `/voice search <query>` — search past commands
- `/voice replay <id>` — re-dispatch a past command
- `/voice stats` — show execution rate and average confidence

## Examples
```
/voice history
# [voice-3] "open file main.rs" (high, executed)
# [voice-2] "run tests" (high, executed)
# [voice-1] "build project" (medium)

/voice search "file"
# [voice-3] "open file main.rs" — score 0.86
```
