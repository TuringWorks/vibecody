---
name: "Autonomous Machine — Autonomous last-mile delivery vehicle"
description: "Autonomous Machine — Autonomous last-mile delivery vehicle: Handles the job: deliver parcels and groceries on local streets and sidewalks. Use when the task involves autonomous machine — autonomous last-mile delivery vehicle, autonomous last-mile delivery vehicle."
category: robotics
triggers: ["autonomous machine — autonomous last-mile delivery vehicle", "autonomous last-mile delivery vehicle"]
tools_allowed: ["read_file", "write_file"]
---

# Autonomous Machine — Autonomous last-mile delivery vehicle

> **Layer:** Non-humanoid autonomous machine (cross-economy) · **Best environments:** neighborhoods, campuses, sidewalks
> **Operated by:** `embodied-ai-*` roles (autonomy, fleet ops, teleoperation, safety) · **Shared concepts:** `jobs-to-be-done-framework`

## Primary job to be done

Deliver parcels and groceries on local streets and sidewalks.

## What it is

A low-speed sidewalk/road delivery robot; teleop-assisted at crossings and exceptions.

## When to use this skill

When a task needs the physical job "deliver parcels and groceries on local streets and sidewalks" in environments such as neighborhoods, campuses, sidewalks. Pair with the relevant operating-system skill (01–23) for domain rules and the human accountability boundary, and with `embodied-ai-*` for the roles that build, operate, and keep it safe.

## Cognitive and control architecture (assumed)

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
