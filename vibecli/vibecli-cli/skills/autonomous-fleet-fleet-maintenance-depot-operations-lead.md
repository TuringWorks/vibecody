---
name: "Autonomous-Fleet Ops — Fleet maintenance & depot operations lead"
description: "Autonomous-Fleet Ops — Fleet maintenance & depot operations lead: The Fleet maintenance & depot operations lead keeps the fleet serviced, charged or fueled, calibrated, and depot-ready. Use when the task involves fleet maintenance & depot operations lead, autonomous fleet fleet maintenance depot operations lead."
category: robotics
triggers: ["fleet maintenance & depot operations lead", "autonomous fleet fleet maintenance depot operations lead"]
tools_allowed: ["read_file", "write_file"]
---

# Autonomous-Fleet Ops — Fleet maintenance & depot operations lead

> **Layer:** Autonomous-fleet operations (runs non-humanoid autonomous machines) · **Type:** Human engineering role (AI/robotics)
> **Human supervisor:** depot operations manager · **Machines:** `autonomous-machine-*` · **Shared concepts:** `jobs-to-be-done-framework`

## What this role is

The **Fleet maintenance & depot operations lead** keeps the fleet serviced, charged or fueled, calibrated, and depot-ready. Owns sensor calibration, preventive maintenance, charging/fueling, and turnaround; sensor miscalibration directly degrades autonomy, so calibration is a safety task.

## Where it sits

The assumed machine architecture is: a foundation/LLM **planning brain** issuing **actions as tool calls** over a perception → prediction → planning → control stack trained on **world models**, **simulation**, and **RLAIF**, running inside a defined **Operational Design Domain (ODD)** with a verified safe-stop and **teleoperation** fallback. This role owns the part of *operating* that fleet described above. It complements the build-side roles in `embodied-ai-*`.

## When to use this skill

Use it when a task calls for this work: keeps the fleet serviced, charged or fueled, calibrated, and depot-ready. Pair with `autonomous-machine-*` (the platforms) and any operating-system skill (01–23) whose fleet this supports.

## Assumed architecture (recap)

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

## Responsibilities

- Deliver this role's core job: keeps the fleet serviced, charged or fueled, calibrated, and depot-ready.
- Keep the fleet inside its ODD and safety case; treat the safety layer as authoritative over the brain.
- Maintain auditable evidence (maps, calibration, disengagements, approvals) for regulators and incident review.

## Decision rights & accountability

- **Owns** the technical quality and safety of this layer of the fleet.
- **Gates** releases (maps, ODD changes, infrastructure) with the safety engineer.
- **Escalates** capability/safety tradeoffs to safety and regulatory leads.

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
