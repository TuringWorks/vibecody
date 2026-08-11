---
name: "Autonomous-Fleet Ops — Vehicle safety operator (in-field)"
description: "Autonomous-Fleet Ops — Vehicle safety operator (in-field): The Vehicle safety operator (in-field) rides in or shadows the machine during validation and takes manual control when needed. Use when the task involves autonomous-fleet ops — vehicle safety operator (in-field), vehicle safety operator (in-field)."
category: robotics
triggers: ["autonomous-fleet ops — vehicle safety operator (in-field)", "vehicle safety operator (in-field)"]
tools_allowed: ["read_file", "write_file"]
---

# Autonomous-Fleet Ops — Vehicle safety operator (in-field)

> **Layer:** Autonomous-fleet operations (runs non-humanoid autonomous machines) · **Type:** Human-in-the-loop operator
> **Human supervisor:** operations supervisor · **Machines:** `autonomous-machine-*` · **Shared concepts:** `jobs-to-be-done-framework`

## What this role is

The **Vehicle safety operator (in-field)** rides in or shadows the machine during validation and takes manual control when needed. The in-vehicle/in-field human fallback during testing and early deployment; every disengagement becomes data that improves the stack.

## Where it sits

The assumed machine architecture is: a foundation/LLM **planning brain** issuing **actions as tool calls** over a perception → prediction → planning → control stack trained on **world models**, **simulation**, and **RLAIF**, running inside a defined **Operational Design Domain (ODD)** with a verified safe-stop and **teleoperation** fallback. This role owns the part of *operating* that fleet described above. It complements the build-side roles in `embodied-ai-*`.

## When to use this skill

Use it when a task calls for this work: rides in or shadows the machine during validation and takes manual control when needed. Pair with `autonomous-machine-*` (the platforms) and any operating-system skill (01–23) whose fleet this supports.

## Assumed architecture (recap)

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

## Responsibilities

- Deliver this role's core job: rides in or shadows the machine during validation and takes manual control when needed.
- Keep the fleet inside its ODD and safety case; treat the safety layer as authoritative over the brain.
- Maintain auditable evidence (maps, calibration, disengagements, approvals) for regulators and incident review.

## Decision rights & accountability

- **Acts** when the machine escalates a low-confidence or out-of-ODD situation.
- **Provides** disengagement and demonstration data that improves the stack.
- **Escalates** recurring takeovers to safety and engineering.

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
