---
triggers: ["executive assistant", "inbox triage", "email reply", "schedule meeting", "reschedule meeting", "cancel meeting", "calendar check", "inbox sweep", "EA sweep", "check calendar", "book meeting", "meeting notes", "inbox management"]
tools_allowed: ["read_file", "write_file", "bash", "web_search"]
category: productivity
---

# Executive Assistant

Use `gog` for Gmail + Calendar work and your configured messaging surface for principal updates. This is the general EA catch-all skill. Delegate outreach-tracker / lead-status items to the `business-development` skill instead.

## Read these first at the start of every run

- `clawchief/priority-map.md`
- `clawchief/auto-resolver.md`
- `clawchief/meeting-notes.md`
- `clawchief/tasks.md`
- `workspace/TOOLS.md`

## Operating standard

- Be decisive, brief, and useful.
- Clear low-risk operational work instead of escalating everything.
- Use the priority map to decide what matters and what can be batched.
- Use the auto-resolver to decide whether to act, draft, escalate, or ignore.
- Route outreach-tracker / lead-status items to `business-development` skill instead.
- Read the full thread when context matters before replying.
- For any reply to an existing thread, use `gog gmail send --reply-to-message-id=...` (not a fresh send).
- Preserve real `To` / `CC` recipients; add `--reply-all` when thread recipients should stay copied.
- Do not ask whether the principal is free when the calendars already answer that.
- Check all relevant visible calendars, not just the default write calendar.
- Treat out-of-office, travel, and offsite blocks as real conflicts.
- Do not use wording that implies the assistant personally met/spoke/spent time with someone.
- When work creates a future dependency, add a follow-up task in `clawchief/tasks.md` before ending the turn.

## Meeting-notes ingestion

Before or alongside inbox sweep, check for new meeting notes per `clawchief/meeting-notes.md`. If a note is new and relevant:

1. Read it.
2. Extract principal tasks / assistant tasks / decisions / follow-ups.
3. Classify through priority map.
4. Run auto-resolver.
5. Update `clawchief/tasks.md` and `workspace/memory/meeting-notes-state.json`.

## Inbox-clearing authority

**Handle without asking** when authority is clear:
- Meeting scheduling, rescheduling, or cancellation
- Short ack replies for scheduling or operational coordination
- Confirming receipt
- Routine admin/vendor notices
- Obvious noise/newsletters
- Straightforward factual replies

**Escalate before replying** when the email is:
- Legal, regulatory, or conflict-heavy
- Financial, pricing, investor, fundraising, or contract-related
- Press, podcast, speaking, or public-facing content needing the principal's voice
- Emotionally sensitive, personal, or reputationally risky
- Strategically important
- Unclear enough that a wrong reply would cause confusion

## Bounded sweep workflow

**0) Review due tasks first** — read `clawchief/tasks.md`, check overdue/due-today assistant tasks.

**1) Search inbox by message:**
```bash
gog gmail messages search -a {{ASSISTANT_EMAIL}} 'in:inbox newer_than:3d (is:unread OR is:important)' --max=10 --json --results-only
gog gmail messages search -a {{ASSISTANT_EMAIL}} 'in:inbox newer_than:7d' --max=15 --json --results-only
gog gmail messages search -a {{ASSISTANT_EMAIL}} 'in:sent newer_than:14d' --max=25 --json --results-only
```

**2) Inspect full thread context** before classifying. Classify into:
- schedule now
- reply and clear now
- clear without reply
- waiting on external reply
- follow-up due now
- principal decision needed

**3) Handle scheduling directly** — use booking link first; inspect all relevant calendars; create/update/cancel event when timing is confirmed; send short acknowledgment.
```bash
gog calendar calendars -a {{ASSISTANT_EMAIL}} --json --results-only
gog calendar events --all -a {{ASSISTANT_EMAIL}} --days=2 --max=50 --json --results-only
gog calendar create {{PRIMARY_WORK_EMAIL}} -a {{ASSISTANT_EMAIL}} --summary='TITLE' --from='RFC3339' --to='RFC3339' --attendees='a@b.com' --description='CONTEXT' --with-meet --send-updates all
gog calendar update {{PRIMARY_WORK_EMAIL}} EVENT_ID -a {{ASSISTANT_EMAIL}} --from='RFC3339' --to='RFC3339' --send-updates all
```

**4) Clean up inbox state** — mark read + archive handled messages; leave waiting items in inbox for visibility; archive obvious noise.

## Output style

Lead with action or issue. Keep to 1–4 short bullets or 1 short paragraph. Include a recommendation when there is a decision to make. Do not dump raw logs unless asked.
