---
name: "Humanoid Robot Catalog — Lab assistant"
description: "Humanoid Robot Catalog — Lab assistant: Handles the job: move samples, load instruments, sanitize benches. Use when the task involves humanoid robot catalog — lab assistant, lab assistant."
category: robotics
triggers: ["humanoid robot catalog — lab assistant", "lab assistant"]
tools_allowed: ["read_file", "write_file"]
---

# Humanoid Robot Catalog — Lab assistant

> **Layer:** Cross-economy robot role · **Best environments:** labs, pharma, hospitals
> **Shared concepts:** `jobs-to-be-done-framework`

## Primary job to be done

Move samples, load instruments, sanitize benches.

## Why a humanoid/mobile form factor

The world is already designed around stairs, doors, handles, shelves, carts, tools, beds, counters, and vehicles built for human bodies. This role takes physical work in those human-built environments so people and AI personnel can focus on judgment and coordination.

## Cognitive and control architecture (assumed)

These robot roles are assumed to be **LLM-brained embodied agents**, not hard-coded automatons. The stack:

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

## When to use this skill

When a task needs the physical job "move samples, load instruments, sanitize benches" in environments such as labs, pharma, hospitals. Pair with the relevant operating-system skill (01–23) for domain safety rules and the human accountability boundary, and with `embodied-ai-*` for the roles that build and operate the brain, policies, and safety layer.

## Division of labor and safety

- **Human supervisor** — owns safety, exceptions, and any high-consequence physical action; holds override authority.
- **LLM brain** — perceives, plans, and issues actions as tool calls; interchangeable and upgradable.
- **VLA policies** — execute motor primitives at high frequency; trained in world models and robot gyms, refined with RLAIF.
- **Verified safety layer** — validates, refuses, or overrides tool calls independently of the brain.
- **AI personnel** — plan, schedule, monitor, and evaluate the robot's work.

## Operating and safety procedure

1. Confirm the environment is mapped and safe; verify people are protected.
2. The brain plans the task and emits motor-primitive **tool calls**; the safety layer validates each before execution.
3. Execute within speed, force, and zone limits via VLA policies.
4. Report status, exceptions, and any safety event immediately.
5. Stop and yield to humans for anything outside the engineered envelope or out-of-distribution for the policies.

## Architecture-specific failure modes

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Architecture-specific failure modes”.

## Adapting to any nation

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.
