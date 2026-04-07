---
triggers: ["business development", "outreach", "prospect", "lead pipeline", "referral partner", "CRM", "outreach tracker", "prospecting", "sales outreach", "lead status", "prospect pipeline"]
tools_allowed: ["read_file", "write_file", "bash", "web_search"]
category: productivity
---

# Business Development

Use this skill for outreach and prospect tracking work. Keep it separate from generic executive-assistant inbox clearing.

Prefer this skill over executive-assistant whenever the task touches the outreach tracker, lead status, prospect pipeline, or referral-partner outreach — even if scheduling is involved.

## Read these first at the start of every run

- `clawchief/priority-map.md`
- `clawchief/auto-resolver.md`
- `workspace/TOOLS.md`
- `skills/business-development/resources/partners.md`

## Core rules

1. The outreach sheet / tracker / CRM is the live source of truth; do not treat local prospect files as current state.
2. Do not silently broaden default prospecting beyond the configured target market / geography without explicit direction.
3. Verify a working website and a real public email before adding a new lead unless the user explicitly waives that requirement.
4. Ignore placeholder or junk addresses from site code.
5. Sweep sent mail so unanswered outreach does not disappear.
6. If the work touches lead status, pipeline state, or the outreach tracker, this skill owns it even when scheduling is part of the job.

## Source of truth

Google Sheet / tracker id: `{{GOOGLE_SHEET_ID}}`

Treat this as the live source of truth for outreach status. Do not rely on local `.md` or `.csv` prospect files as the current record.

## Current focus

Customize the business-development playbook in `workspace/TOOLS.md`. At minimum define:

- target geography
- target market or target segments
- default daily batch size
- verification requirements
- any follow-up cadence overrides

If no more specific override exists, default to:
- prospecting inside `{{TARGET_MARKET}}` in `{{TARGET_GEOGRAPHY}}`
- adding only verified leads
- using the default follow-up cadence in this skill

## When to update the tracker

Update the tracker every time outreach state changes — when you:
- send the initial outreach email
- get any meaningful reply
- ask for a meeting
- book, confirm, reschedule, or cancel a meeting
- record a decline / not-a-fit outcome
- learn a follow-up or next-step detail worth preserving

Do this before marking the thread handled.

## Inbound reply operating procedure

When partner / referral emails come in, process the inbox and the tracker as one workflow:

1. Use message-level Gmail search to find inbound replies.
2. Review each inbound thread and identify the current state.
3. Check whether the person already exists in the tracker.
4. Update the row with what changed.
5. Only after the tracker is current should the email be considered handled.
6. If a meeting is booked through a scheduler, update the row immediately after the booking succeeds.

## Follow-up cadence

- First follow-up: ~2 days after last unanswered outbound
- Second follow-up: ~5 days after previous follow-up
- Third follow-up: ~7 days after previous follow-up

After the third unanswered follow-up, stop and surface the lead if it still matters.

## Default outbound workflow

1. Verify the lead is not already in the tracker.
2. Verify the lead matches the configured target market / geography.
3. Verify a working website unless explicitly waived.
4. Inspect the website for a real public email address before leaving email blank.
5. Send the initial outreach email using `resources/partners.md`.
6. Update the tracker immediately after each action.
7. Sweep sent mail for unanswered outreach and follow up on cadence.
