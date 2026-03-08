---
triggers: ["multiplayer", "netcode", "game server", "matchmaking", "lobby", "dedicated server", "client prediction", "rollback", "lag compensation", "game networking"]
tools_allowed: ["read_file", "write_file", "bash"]
category: gaming
---

# Multiplayer Game Networking

When working with multiplayer netcode, game servers, and online game systems:

1. Use an authoritative server architecture where the server owns all game state and validates every client action; clients send inputs (not state), the server simulates, and broadcasts authoritative state snapshots to prevent cheating and ensure consistency.

2. Implement client-side prediction by immediately applying local player inputs on the client while simultaneously sending them to the server; when the server's authoritative state arrives, reconcile by replaying unacknowledged inputs on top of the corrected state to hide latency without sacrificing correctness.

3. Apply entity interpolation for remote entities by buffering received states and rendering them with a fixed delay (typically 100ms); interpolate between the two most recent snapshots so remote players move smoothly even when packets arrive at uneven intervals.

4. Implement lag compensation on the server using a history buffer of world states; when processing a hit-scan or projectile from a client, rewind the world to the client's perceived time (accounting for their RTT) to validate the shot, then apply the result to the current authoritative state.

5. For fighting games or small-player-count genres requiring frame-perfect inputs, use rollback netcode (GGPO-style): predict remote inputs, simulate forward, and when actual inputs arrive, roll back, re-simulate with corrected inputs, and fast-forward to the present frame.

6. Design matchmaking with a proven rating system (Glicko-2 or TrueSkill) that accounts for rating uncertainty; match players within a confidence-bounded skill window, expand the window over queue time, and factor in latency, party size, and region for match quality.

7. Build lobby systems with clear state machines (waiting, ready-check, loading, in-game, post-game); implement host migration for P2P topologies, handle disconnects/reconnects gracefully, and use heartbeats to detect and remove stale players within seconds.

8. Scale dedicated game servers using containerized instances (one match per container) orchestrated by Kubernetes or Agones; auto-scale based on matchmaking queue depth, co-locate servers in multiple regions, and route players to the nearest healthy server.

9. Synchronize game state efficiently with delta compression (send only what changed since the client's last acknowledged snapshot), quantize floats to the minimum required precision, use bitpacking, and prioritize updates for entities closer to or visible to each client.

10. Implement anti-cheat in layers: server-side validation (speed checks, damage caps, cooldown enforcement) as the primary defense, encrypted and authenticated packets to prevent tampering, server-authoritative random number generation, and rate-limiting of client actions.

11. Use UDP as the transport layer for real-time game data with a custom reliability layer on top: unreliable for state snapshots (latest-wins), reliable-ordered for critical events (kills, pickups, chat), and implement congestion control to avoid flooding slow connections.

12. Build a replay system by recording all inputs (in a deterministic simulation) or all state snapshots with timestamps; replays serve debugging, spectating, anti-cheat review, and highlight generation with minimal additional infrastructure.

13. Handle player reconnection by maintaining the player's slot and game state on the server for a grace period after disconnect; upon reconnect, fast-forward the client to the current state and resume input processing without disrupting other players.

14. Implement a network simulation layer for development that can inject artificial latency, jitter, packet loss, and out-of-order delivery; test all netcode under adverse conditions (200ms+ RTT, 5% packet loss) to ensure the game remains playable on real-world connections.
