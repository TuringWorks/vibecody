---
layout: page
title: "Demo 21: CRDT Collaboration"
permalink: /demos/crdt-collab/
nav_order: 21
parent: Demos
---


## Overview

VibeCody supports real-time collaborative editing powered by Conflict-free Replicated Data Types (CRDTs) via the `vibe-collab` crate. Multiple developers can edit the same file simultaneously without merge conflicts. The system provides multi-cursor support, presence indicators, and automatic conflict resolution. You can pair with colleagues over the local network using mDNS discovery, QR code pairing, or Tailscale tunnels for remote sessions.

**Time to complete:** ~12 minutes

## Prerequisites

- VibeCody installed on two or more machines (or two terminal sessions for local testing)
- Both machines on the same network (for mDNS discovery) or Tailscale installed (for remote pairing)
- (Optional) VibeUI installed for visual multi-cursor and presence indicators

## How CRDTs Work in VibeCody

Traditional collaborative editors rely on a central server to resolve conflicts. VibeCody uses CRDTs, which allow each participant to edit independently and merge changes automatically without a coordinator. Every character insertion and deletion is assigned a unique, causally ordered identifier. When edits from different users arrive, the CRDT algorithm guarantees all participants converge to the same document state regardless of message ordering or network delays.

## Step-by-Step Walkthrough

### Step 1: Start a collaboration session (Host)

On the host machine, open a REPL session and start sharing.

```bash
vibecli
```

```
/pair start --file src/main.rs
```

Expected output:

```
Collaboration session started
  Session ID: vibe-collab-a7f3b2c1
  File: src/main.rs
  Sharing via: mDNS (local network)
  QR Code: [displayed in terminal]
  Join command: /pair join vibe-collab-a7f3b2c1

Waiting for participants...
```

A QR code is displayed in the terminal that the other participant can scan with their phone or read with a camera to auto-fill the join command.

### Step 2: Discover sessions on the network

On the second machine, use mDNS discovery to find active sessions.

```
/discover
```

```
Discovered VibeCody sessions on local network:
  1. vibe-collab-a7f3b2c1  host: alice@macbook  file: src/main.rs  (2s ago)
  2. vibe-collab-d4e5f6g7  host: bob@desktop    file: lib.rs       (15s ago)
```

### Step 3: Join the session

```
/pair join vibe-collab-a7f3b2c1
```

```
Connected to session vibe-collab-a7f3b2c1
  Host: alice@macbook
  File: src/main.rs
  Participants: 2 (alice, bob)
  CRDT sync: active
```

Both participants now see each other's cursors and edits in real time.

### Step 4: Observe real-time editing

When Alice types on line 15, Bob's terminal (or VibeUI editor) immediately shows the insertion. The CRDT ensures that even if both users edit the same line simultaneously, the result is deterministic and consistent.

**Host (Alice) edits line 15:**

```rust
fn handle_request(req: Request) -> Response {
    // Alice adds: validate input
    let validated = validate(&req)?;
```

**Guest (Bob) sees the edit appear instantly and adds line 17:**

```rust
fn handle_request(req: Request) -> Response {
    let validated = validate(&req)?;
    // Bob adds: log the request
    log::info!("Handling request: {:?}", req);
```

Both edits merge without conflict.

### Step 5: Resolve concurrent edits on the same line

If both users edit the same character range simultaneously, the CRDT resolves it deterministically. Each user's edit is preserved, with the user who has the lower peer ID appearing first.

```
[CRDT] Concurrent edit detected at line 20, columns 5-12
[CRDT] Resolved: alice's edit placed before bob's edit
[CRDT] Both users notified of merge result
```

### Step 6: View presence indicators

In VibeUI, each participant's cursor is shown in a distinct color with their username label. The sidebar shows a presence list:

```
Collaborators:
  alice (host)  -- cursor at line 15, col 23  [blue]
  bob           -- cursor at line 17, col 42  [green]
```

In the TUI, presence is shown in the status bar:

```
[collab] 2 users | alice:L15 bob:L17 | CRDT synced
```

### Step 7: Use Tailscale for remote pairing

For collaborators not on the same local network, use Tailscale to create a secure tunnel.

```
/tailscale start
```

```
Tailscale funnel active
  URL: https://alice-vibecody.tail1234.ts.net
  Share this URL with your collaborator
```

The remote collaborator joins using the Tailscale URL:

```
/pair join --url https://alice-vibecody.tail1234.ts.net
```

```
Connected via Tailscale tunnel
  Latency: 23ms
  Encryption: WireGuard
  Session: vibe-collab-a7f3b2c1
```

### Step 8: End the session

The host can end the session at any time.

```
/pair stop
```

```
Collaboration session ended
  Duration: 8m 42s
  Participants: 2
  Edits merged: 47
  Conflicts resolved: 3 (all automatic)
  Final file saved: src/main.rs
```

### Step 9: Using CRDT Collaboration in VibeUI

In VibeUI, collaboration features are integrated directly into the editor.

1. **Start sharing** -- Right-click a file tab and select **Share for Collaboration**, or use the collaboration icon in the toolbar.
2. **Join** -- Click **File > Join Collaboration** and paste the session ID or scan the QR code.
3. **Multi-cursor view** -- Each collaborator's cursor and selection is highlighted in their assigned color.
4. **Presence panel** -- The right sidebar shows all connected users, their cursor positions, and connection quality.
5. **Conflict log** -- The bottom panel logs any concurrent edits and how they were resolved.

## CLI Command Reference

| Command | Description |
|---------|-------------|
| `/pair start --file <path>` | Start a collaboration session on a file |
| `/pair join <session-id>` | Join an existing session by ID |
| `/pair join --url <tailscale-url>` | Join via Tailscale tunnel URL |
| `/pair stop` | End the current collaboration session |
| `/pair status` | Show session info, participants, sync state |
| `/discover` | Discover VibeCody sessions on the local network via mDNS |
| `/tailscale start` | Start a Tailscale funnel for remote pairing |
| `/tailscale stop` | Stop the Tailscale funnel |

## Demo Recording

```json
{
  "demoRecording": {
    "version": "1.0",
    "title": "CRDT Collaboration Demo",
    "description": "Real-time collaborative editing with CRDT conflict resolution, multi-cursor, and remote pairing",
    "duration_seconds": 210,
    "steps": [
      {
        "timestamp": 0,
        "action": "repl_command",
        "command": "/pair start --file src/main.rs",
        "output": "Collaboration session started\n  Session ID: vibe-collab-a7f3b2c1\n  QR Code: [displayed]\n  Waiting for participants...",
        "narration": "Host starts a collaboration session and shares via mDNS"
      },
      {
        "timestamp": 20,
        "action": "repl_command",
        "command": "/discover",
        "output": "Discovered VibeCody sessions:\n  1. vibe-collab-a7f3b2c1  host: alice@macbook  file: src/main.rs",
        "narration": "Guest discovers the session on the local network"
      },
      {
        "timestamp": 35,
        "action": "repl_command",
        "command": "/pair join vibe-collab-a7f3b2c1",
        "output": "Connected to session vibe-collab-a7f3b2c1\n  Participants: 2 (alice, bob)\n  CRDT sync: active",
        "narration": "Guest joins the collaboration session"
      },
      {
        "timestamp": 55,
        "action": "collab_event",
        "user": "alice",
        "event": "edit",
        "location": "line 15",
        "content": "let validated = validate(&req)?;",
        "narration": "Alice edits line 15 -- Bob sees it instantly"
      },
      {
        "timestamp": 70,
        "action": "collab_event",
        "user": "bob",
        "event": "edit",
        "location": "line 17",
        "content": "log::info!(\"Handling request: {:?}\", req);",
        "narration": "Bob adds a log line -- Alice sees it instantly"
      },
      {
        "timestamp": 90,
        "action": "collab_event",
        "user": "both",
        "event": "concurrent_edit",
        "location": "line 20",
        "resolution": "CRDT auto-merge: alice's edit before bob's",
        "narration": "Both edit the same line simultaneously -- CRDT resolves it"
      },
      {
        "timestamp": 115,
        "action": "ui_interaction",
        "panel": "Editor",
        "action_detail": "presence_indicators",
        "details": "alice: blue cursor at L15, bob: green cursor at L17",
        "narration": "VibeUI shows colored cursors for each collaborator"
      },
      {
        "timestamp": 140,
        "action": "repl_command",
        "command": "/tailscale start",
        "output": "Tailscale funnel active\n  URL: https://alice-vibecody.tail1234.ts.net",
        "narration": "Enable Tailscale for a remote collaborator to join"
      },
      {
        "timestamp": 165,
        "action": "repl_command",
        "command": "/pair join --url https://alice-vibecody.tail1234.ts.net",
        "output": "Connected via Tailscale tunnel\n  Latency: 23ms\n  Participants: 3",
        "narration": "A remote collaborator joins through the Tailscale tunnel"
      },
      {
        "timestamp": 190,
        "action": "repl_command",
        "command": "/pair stop",
        "output": "Collaboration session ended\n  Duration: 8m 42s\n  Edits merged: 47\n  Conflicts resolved: 3",
        "narration": "End the session with a summary of all merged edits"
      }
    ]
  }
}
```

## What's Next

- [Demo 22: Gateway Messaging](../22-gateway/) -- Run AI agents across 18 messaging platforms
- [Demo 23: Test Runner & Coverage](../23-test-coverage/) -- AI-powered test generation with coverage tracking
- Combine CRDT collaboration with agent teams to have multiple agents and humans editing together
