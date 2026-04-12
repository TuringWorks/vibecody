# Branch Lock

Distributed branch locking to prevent concurrent agent modifications. Locks expire automatically (TTL), can be renewed, and support force-release for admin recovery.

## When to Use
- Preventing two agents from editing the same branch simultaneously
- Coordinating multi-agent workflows on shared git branches
- Ensuring exclusive access during migrations or refactors
- Recovering from stale locks left by crashed sessions

## Lock Lifecycle
1. `acquire` — claim a lock (auto-expires stale locks)
2. `renew` — extend TTL while still working
3. `release` — free the lock when done
4. `force_release` — admin override for stuck locks
5. Expired locks are cleared automatically on the next `acquire`

## Commands
- `/lock acquire <branch>` — Acquire exclusive lock on a branch
- `/lock release <branch>` — Release your lock
- `/lock status <branch>` — Check lock status and holder
- `/lock renew <branch>` — Extend lock TTL
- `/lock list` — Show all active locks
- `/lock force-release <branch>` — Force-release a lock (admin)
- `/lock release-session <session-id>` — Release all locks for a session

## Examples
```
/lock acquire feat/my-feature
# Acquired: feat/my-feature (TTL: 300s, session: s-abc123)

/lock status feat/my-feature
# Locked by: s-abc123, expires in: 248s

/lock renew feat/my-feature
# TTL extended to: 300s

/lock force-release feat/my-feature
# Force-released (previous holder: s-dead001)
```
