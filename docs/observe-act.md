---
layout: page
title: Observe-Act — the visual grounding loop
permalink: /observe-act/
---

Observe-Act is VibeCody's computer-use loop: it takes a screenshot of your
desktop, asks a vision model what to do next, performs the actions on your real
mouse and keyboard, checks whether the screen changed the way the model said it
would, and repeats. It is the same pattern as Anthropic Computer Use and
OpenClaw, driven by whichever provider and model you select — not by a fixed
vendor.

**It drives your actual machine.** There is no sandbox here. Read
[Safety modes](#safety-modes) before running one autonomously, and start in
**Restricted**, which observes and records what it *would* do without touching
anything.

---

## Where it lives

VibeCoder → **System Monitor** → **Observe-Act**, with four tabs:

| Tab | What it does |
|---|---|
| **Setup** | Preflight status, task, vision model, safety mode, step and interval budgets, Start / Pause / Stop |
| **Monitor** | Live status, the latest screenshot, the model's current reasoning, and an event log |
| **History** | Every step: reasoning, actions run, actions *not* run, and the verification verdict |
| **Safety** | The rails — action budget, rate limit, forbidden key combos, forbidden screen regions |

Everything the panel does goes to the VibeCLI daemon over `/observe/*`. The
daemon owns the session, because there is one screen per machine: a registry
per shell would give three shells three different answers to "is a session
running".

---

## Prerequisites

The loop shells out to the platform's automation tools. **Setup** runs a
preflight and names anything missing, with a distinct message per cause — a
missing binary and a denied permission need different fixes.

| Platform | Tools | Also required |
|---|---|---|
| macOS | `osascript`, `screencapture`, `cliclick` (`brew install cliclick`) | Screen Recording **and** Accessibility permission for the process running the daemon |
| Linux (X11) | `xdotool`, `wmctrl`, `scrot`, `xdpyinfo` | — |
| Windows | `powershell` | — |

On macOS, `screencapture` exits **0** when Screen Recording is denied and
writes no file. The loop checks that the file exists rather than trusting the
exit code, and says which permission is missing — the one failure mode that
otherwise looks exactly like success.

---

## Safety modes

| Mode | What runs |
|---|---|
| **Restricted** | Nothing. The model observes and proposes; every proposal is recorded and shown struck through in History. This is the mode to watch a task in before letting it act. |
| **Cautious** (default) | Everything except destructive actions, which stop and ask. Unanswered for five minutes, the request is treated as a **refusal** — an operator who walked away has not approved anything. |
| **Autonomous** | Everything, without asking. Stop is the only gate. |

"Destructive" is classified in `observe_act::is_destructive`: key combos
containing delete/backspace, `ctrl+w`/`q`/`x`, `alt+f4`; typed text containing
`rm `, `del `, `sudo `, `shutdown`, `reboot`; and any drag.

### The rails

Configured on the **Safety** tab, enforced in the daemon before any action is
performed:

- **Max actions per step** — an over-long batch is refused *whole*, not
  truncated to the limit. Ceiling of 20 whatever the request asks for.
- **Rate limit** — minimum milliseconds between consecutive actions.
- **Max consecutive failures** — the session fails rather than burning its
  whole step budget.
- **Forbidden key combos** — never issued. `alt+f4` and `ctrl+alt+del` by
  default.
- **Forbidden screen regions** — rectangles no click, drag or mouse move may
  target, in the display's own units.
- **Hard caps that are not configurable**: a `wait` is clamped to 30 s, a
  `type` to 4096 characters, a `scroll` to 50 increments. Each of these would
  otherwise let one model turn park the session with nothing on screen saying
  why.

---

## Coordinate spaces

Three of them, and conflating any two puts the click somewhere the model never
looked:

| Space | What it is |
|---|---|
| **Image** | The pixels of the screenshot actually sent to the model, downscaled to fit the vision API's limits. The model answers in this space, because it is the only one it can see. |
| **Capture** | The screenshot file's own pixels. On a Retina display `screencapture` writes the backing store, so this is 2× the logical size. |
| **Logical** | The points `cliclick` and `xdotool` take. Where an action must land. |

The daemon measures image width and logical width and divides. The backing
scale factor is never guessed — the usual source for it (`system_profiler`
printing the word "Retina") is absent on a scaled resolution and wrong on a
second display, and a factor-of-two error there puts every click in the wrong
quadrant.

Coordinates are mapped **before** the safety rails run, so a forbidden region
is checked against the pixel the click will actually reach.

---

## Verification

When **Verify after action** is on, each step costs a second screenshot and a
second model call: the model states in advance what the screen *should* look
like, and is then shown the result and asked whether it matches.

A step has three verification states, not two:

| State | Meaning |
|---|---|
| **Verified** | Checked, and it matched. |
| **Failed** | Checked, and it did not. Counts toward the consecutive-failure limit. |
| **Unverified** | Not checked — verification is off, the model gave no expected change, the verification screenshot failed, or its verdict could not be parsed. |

**Unverified is not a failure.** It leaves the failure streak exactly where it
was, and it is excluded from the verified rate — a run with verification off
reads `n/a`, never `0%`.

---

## The model

The vision model is chosen in the panel, seeded from the toolbar's provider.
Both `provider` and `model` are **required** on the API; there is no fallback
to the daemon's boot provider, because a loop that silently used another vendor
would be sending screenshots of your desktop to a service you did not pick.

The daemon reports whether the provider *advertises* vision support but does
not enforce it: `AIProvider::supports_vision` defaults to `false` and several
providers that do accept images never override it, so refusing on it would
reject working configurations. If every step comes back blind, that warning is
why.

Screenshots are downscaled to the configured cap (1280×720 by default) and
re-encoded as JPEG at quality 80. A raw Retina PNG is roughly 12 MB
base64-encoded — past Anthropic's 5 MB per-image limit before the request is
even built.

---

## HTTP API

All authed (`Authorization: Bearer <token>`); the SSE stream also accepts
`?token=` because `EventSource` cannot set a header.

| Route | Purpose |
|---|---|
| `GET /observe/preflight` | Platform, missing tools, logical screen size, `ready` |
| `GET` / `PUT /observe/config` | The saved loop + safety configuration |
| `GET /observe/sessions` | Every session, newest first, without steps |
| `POST /observe/sessions` | Start one — `{ task, provider, model, config?, safety? }` |
| `GET /observe/sessions/{id}` | The full record, steps included |
| `POST /observe/sessions/{id}/pause` | Pause after the current step |
| `POST /observe/sessions/{id}/resume` | Resume |
| `POST /observe/sessions/{id}/abort` | Stop at the next action boundary |
| `POST /observe/sessions/{id}/approve` | `{ approval_id, approve }` |
| `GET /observe/sessions/{id}/events` | SSE — `snapshot`, then one `step` per loop event |
| `GET /observe/sessions/{id}/screenshot` | The most recent capture, as PNG |

Starting a session while one is still running returns **409**. Two loops moving
the same mouse would each verify against the other's half-finished work.

```bash
curl -sX POST localhost:7878/observe/sessions \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"task":"open the Downloads folder","provider":"claude","model":"claude-sonnet-4-6"}'
```

---

## Where the record lives

`~/.vibecli/observe_act/<session-id>/` — one PNG per step (plus a
`-verify.png` where verification ran) and a `session.json` written after every
step.

A session that was running when the daemon stopped reloads as **aborted**. The
process driving it is gone, so it is not running, and showing it as running
would leave a phantom in the history that no stop button could clear.

---

## Limits

- **One screen.** Multi-display setups are captured and driven as the primary
  display reports itself; a second monitor is outside the coordinate space.
- **No accessibility tree.** Everything is coordinate-based, from pixels. There
  is no element lookup, so a model that misreads a small label clicks the wrong
  thing — which is what Restricted mode and verification are for.
- **X11 only on Linux.** `xdotool` does not drive a Wayland compositor.
- **No CLI surface yet.** The loop is reachable from the panel and the HTTP
  API; `vibecli` has no `/observe` subcommand.
- **VibeCoder only, deliberately.** The loop drives the machine the daemon runs
  on, so a phone or a watch could only ever watch and stop one — worth having,
  not built. VibeDesk and VibeAIChat do not carry the panel either. The
  `/observe/*` routes are open to any authenticated client that wants to.
