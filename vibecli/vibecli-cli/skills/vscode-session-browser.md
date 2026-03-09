# VS Code Session Browser

Browse, search, and replay past VibeCLI agent sessions from VS Code.

## Triggers
- "session browser", "session history", "replay session"
- "VS Code sessions", "past sessions", "session list"

## Usage
```
/sessions list                    # List all sessions
/sessions search "login bug"      # Search by title or tag
/sessions filter active           # Filter by status
/sessions replay sess-1 step:3    # Replay to step 3
/sessions stats                   # Provider statistics
```

## Features
- Session lifecycle: Active, Completed, Failed, Paused
- File change tracking with type (Created, Modified, Deleted, Renamed)
- Snapshot-based replay with 5 action types: UserMessage, AssistantMessage, ToolCall, FileEdit, CommandRun
- Step-by-step replay with file diffs at each step
- Search by title and tags
- Filter by status or provider
- Provider usage statistics
- Message count tracking
