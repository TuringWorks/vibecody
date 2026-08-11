---
name: "Autonomous Machine — Fixed industrial robotic cell"
description: "Autonomous Machine — Fixed industrial robotic cell: Handles the job: weld, assemble, pick-and-place, machine-tend, and inspect at a fixed station. Use when the task involves autonomous machine — fixed industrial robotic cell, fixed industrial robotic cell."
category: robotics
triggers: ["autonomous machine — fixed industrial robotic cell", "fixed industrial robotic cell"]
tools_allowed: ["read_file", "write_file"]
---

# Autonomous Machine — Fixed industrial robotic cell

> **Layer:** Non-humanoid autonomous machine (cross-economy) · **Best environments:** factories, labs, fabrication shops
> **Operated by:** `embodied-ai-*` roles (autonomy, fleet ops, teleoperation, safety) · **Shared concepts:** `jobs-to-be-done-framework`

## Primary job to be done

Weld, assemble, pick-and-place, machine-tend, and inspect at a fixed station.

## What it is

A stationary (non-mobile) robotic cell — the workhorse of automated production; not humanoid and not mobile, but an LLM-planned, tool-call-driven physical agent within a guarded, fixed envelope.

## When to use this skill

When a task needs the physical job "weld, assemble, pick-and-place, machine-tend, and inspect at a fixed station" in environments such as factories, labs, fabrication shops. Pair with the relevant operating-system skill (01–23) for domain rules and the human accountability boundary, and with `embodied-ai-*` for the roles that build, operate, and keep it safe.

## Cognitive and control architecture (assumed)

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

## Division of labor and safety

- **Human owner / fleet operator** — owns the safety case, the ODD, and stop authority; accountable for incidents.
- **Autonomy brain** — perceives, predicts, plans, and issues actuation as tool calls within the ODD.
- **Low-level controllers** — execute motion/actuation at high frequency.
- **Verified safety layer** — triggers a minimal-risk maneuver (safe-stop / return-to-base / hover) independently of the brain.
- **Remote operator (teleop)** — supervises and takes over beyond the ODD or below a confidence threshold.

## Architecture-specific failure modes

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Architecture-specific failure modes”.

## Adapting to any nation (context modifiers)

Ownership ranges from fleet-as-a-service to cooperatively shared or rented machines; regulation (road approval, airspace/BVLOS, mine/site rules) and infrastructure (maps, connectivity, GPS/RTK) gate where it can run. In low-connectivity settings, on-board autonomy and safe-stop matter more than teleop. Re-read through:

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.
