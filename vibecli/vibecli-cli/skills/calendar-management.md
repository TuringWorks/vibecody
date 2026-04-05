---
triggers: ["calendar", "google calendar", "outlook calendar", "schedule", "meeting", "event", "free slots", "book time", "reschedule"]
tools_allowed: ["read_file", "write_file", "bash"]
category: productivity
---

# Calendar Management (Google Calendar & Outlook)

VibeCLI integrates with Google Calendar and Outlook Calendar via `/calendar` (alias `/cal`).

## Setup

**Google Calendar** — OAuth2 access token:
```toml
[calendar]
provider = "google"
access_token = "ya29.xxxx"
calendar_id = "primary"   # optional, defaults to primary
```
Or set `GOOGLE_CALENDAR_TOKEN` environment variable.

**Outlook Calendar** — Microsoft Graph token:
```toml
[calendar]
provider = "outlook"
access_token = "eyJ0..."
```
Or set `OUTLOOK_CALENDAR_TOKEN` environment variable.

## REPL Commands

| Command | Description |
|---------|-------------|
| `/cal today` | Events for today with times and locations |
| `/cal week` | Full week view |
| `/cal list [days]` | List next N days (default 7) |
| `/cal create <title> <start> <end> [desc]` | Create event (ISO8601 or natural language) |
| `/cal delete <event-id>` | Delete event |
| `/cal free [date]` | Find free slots on a day |
| `/cal move <event-id> <new-start>` | Reschedule event |
| `/cal next` | Show next upcoming event |
| `/cal remind <event-id> <minutes>` | Set reminder |

## Natural Language Time Parsing

The calendar module parses natural language time:
- `"tomorrow 2pm"` → next day 14:00 local time
- `"friday 10am for 1h"` → next Friday 10:00–11:00
- `"next week monday 9:30"` → Monday of next week
- `"in 3 days at noon"` → +3 days 12:00

## Effective Usage Patterns

1. **Day planning**: Start with `/cal today` to get a full picture before beginning work. Pair with `/email unread` for a complete morning briefing.
2. **Smart scheduling**: Use `/cal free` to find open slots before proposing meeting times — avoids double-booking and reduces back-and-forth.
3. **Event creation from conversation**: When discussing tasks with the AI, say "add that to my calendar for Thursday 3pm" and it issues `/cal create` automatically.
4. **Recurring event awareness**: The list/week views indicate recurring events with `[R]` so you can distinguish one-off vs standing meetings.
5. **Meeting prep**: Run `/cal next` then `/email search <meeting-topic>` to surface relevant email threads before a meeting.
6. **Time zone handling**: Set `timezone = "America/New_York"` in `[calendar]` config. All display times are converted to local; storage uses UTC.
7. **Buffer time**: After creating back-to-back events, use `/cal free` on the same day to verify there are gaps for travel or preparation.
8. **Reminder defaults**: Set `default_reminder_minutes = 15` in config to auto-add reminders to every created event.
9. **Read-only mode**: Set `calendar_readonly = true` in config to prevent accidental creates/deletes when using the AI in exploration mode.
10. **Conflict detection**: Creating an event that overlaps an existing one triggers a warning showing the conflicting event — confirm to override or pick a different slot.
