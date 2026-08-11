---
name: "Autonomous-Fleet Ops — Warehouse automation lead"
description: "Autonomous-Fleet Ops — Warehouse automation lead: The Warehouse automation lead operates AMRs, autonomous forklifts, sortation, inventory robots, fixed cells, and warehouse orchestration. Use when the task involves autonomous-fleet ops — warehouse automation lead, warehouse automation lead."
category: robotics
triggers: ["autonomous-fleet ops — warehouse automation lead", "warehouse automation lead"]
tools_allowed: ["read_file", "write_file"]
---

# Autonomous-Fleet Ops — Warehouse automation lead

> **Layer:** Autonomous-fleet operations (runs non-humanoid autonomous machines) · **Type:** Human oversight role (accountability boundary)
> **Human supervisor:** site operations manager · **Machines:** `autonomous-machine-*` · **Shared concepts:** `jobs-to-be-done-framework`

## What this role is

The **Warehouse automation lead** operates AMRs, autonomous forklifts, sortation, inventory robots, fixed cells, and warehouse orchestration. Owns mixed human/robot facility operations; balances throughput and worker safety and owns the exclusion-zone and override design.

## Where it sits

The assumed machine architecture is: a foundation/LLM **planning brain** issuing **actions as tool calls** over a perception → prediction → planning → control stack trained on **world models**, **simulation**, and **RLAIF**, running inside a defined **Operational Design Domain (ODD)** with a verified safe-stop and **teleoperation** fallback. This role owns the part of *operating* that fleet described above. It complements the build-side roles in `embodied-ai-*`.

## When to use this skill

Use it when a task calls for this work: operates AMRs, autonomous forklifts, sortation, inventory robots, fixed cells, and warehouse orchestration. Pair with `autonomous-machine-*` (the platforms) and any operating-system skill (01–23) whose fleet this supports.

## Assumed architecture (recap)

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

## Responsibilities

- Deliver this role's core job: operates AMRs, autonomous forklifts, sortation, inventory robots, fixed cells, and warehouse orchestration.
- Keep the fleet inside its ODD and safety case; treat the safety layer as authoritative over the brain.
- Maintain auditable evidence (maps, calibration, disengagements, approvals) for regulators and incident review.

## Decision rights & accountability

- **Owns and is accountable for** the safety case, ODD boundary, regulatory authorization, and stop authority.
- **Cannot delegate** these to the autonomy brain; the safety layer and ODD are independent of it.
- **Escalates** unresolved safety or compliance risk and can ground or halt the fleet.

## Failure modes and safeguards

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Architecture-specific failure modes”.

## Adapting to any nation (context modifiers)

Fleet ownership, road/airspace regulation, connectivity, and mapping coverage vary widely; in low-infrastructure settings on-board autonomy and safe-stop matter more than teleoperation and V2X. Re-read through:

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

## Operating procedure

1. Confirm the ODD, safety case, and regulatory authorization for the work.
2. Run the role's core job, keeping the safety layer and ODD authoritative over the brain.
3. Monitor health, confidence, and disengagements; escalate ODD or safety changes to humans.
4. Maintain the audit trail; feed incidents and disengagements back into the stack.
