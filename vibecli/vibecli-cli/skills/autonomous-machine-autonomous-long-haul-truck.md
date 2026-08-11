---
name: "Autonomous Machine — Autonomous long-haul truck"
description: "Autonomous Machine — Autonomous long-haul truck: Handles the job: haul freight over highway corridors hub-to-hub without a driver in the cab. Use when the task involves autonomous machine — autonomous long-haul truck, autonomous long-haul truck."
category: robotics
triggers: ["autonomous machine — autonomous long-haul truck", "autonomous long-haul truck"]
tools_allowed: ["read_file", "write_file"]
---

# Autonomous Machine — Autonomous long-haul truck

> **Layer:** Non-humanoid autonomous machine (cross-economy) · **Best environments:** highways, freight corridors, transfer hubs
> **Operated by:** `embodied-ai-*` roles (autonomy, fleet ops, teleoperation, safety) · **Shared concepts:** `jobs-to-be-done-framework`

## Primary job to be done

Haul freight over highway corridors hub-to-hub without a driver in the cab.

## What it is

A Class 8 autonomous truck, often a hub-to-hub model with human drivers handling the first and last mile.

## When to use this skill

When a task needs the physical job "haul freight over highway corridors hub-to-hub without a driver in the cab" in environments such as highways, freight corridors, transfer hubs. Pair with the relevant operating-system skill (01–23) for domain rules and the human accountability boundary, and with `embodied-ai-*` for the roles that build, operate, and keep it safe.

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
