---
layout: page
title: "Security"
permalink: /security/
---

VibeCody runs a local daemon that holds your provider keys, reads your source
tree, and executes commands a language model asked for. These pages describe
what that means and where the boundaries are.

`docs/index.md` has linked here since before this page existed, and the link
404'd — the directory held three documents and no index.

## The documents

| | |
|---|---|
| [Threat Model]({{ site.baseurl }}/security/threat-model/) | System-level STRIDE/DREAD decomposition — the trust boundaries, what an attacker gets by crossing each, and which countermeasures exist today |
| [Tainted Data Flow]({{ site.baseurl }}/security/tainted-data-flow/) | Prompt-injection containment: how content the model reads is marked, and what a tainted argument is not allowed to do |
| [Daemon Token Rotation]({{ site.baseurl }}/security/key-rotation/) | Invalidating a leaked or stale bearer token |

## The parts worth knowing without reading further

**Credentials are encrypted at rest, and not in your config file.** Provider
keys live in the ProfileStore (`~/.vibecli/profile_settings.db`), project
secrets in the WorkspaceStore (`<workspace>/.vibecli/workspace.db`). Nothing
writes a key to `config.toml`, and a key in an environment variable is a
fallback rather than the intended path. See
[Configuration]({{ site.baseurl }}/configuration/).

**The daemon's bearer token rotates on every start.** A fresh 128-bit token is
minted each time `vibecli --serve` runs and written to `~/.vibecli/daemon.token`.
Anything caching one across a restart gets a 401. That is also the reason not to
put port 7878 on a public address — see
[Deployment guides]({{ site.baseurl }}/guides/).

**Almost every route requires that token**, and "almost" covers three
mechanisms rather than one: a small set needs no credential, two WebSocket
routes validate `?token=` themselves because a handshake cannot set headers, and
`/webhook/github` checks `X-Hub-Signature-256`. The exact split is in
[api-reference]({{ site.baseurl }}/api-reference/#authentication).

**Memory "encryption" is not encryption.** The OpenMemory at-rest option is a
repeating-key XOR, not a cipher, and it is documented as such in
[Configuration]({{ site.baseurl }}/configuration/). Do not put secrets in
memories.

**Agent commands can be sandboxed, and are not by default.** Docker, Podman and
OpenSandbox are supported runtimes; approval tiers gate what runs without
asking. Both are opt-in choices you make per workspace.

## Reporting something

Open a GitHub issue for anything already public. For a suspected vulnerability,
prefer a private report through the repository's security advisory page rather
than a public issue.
