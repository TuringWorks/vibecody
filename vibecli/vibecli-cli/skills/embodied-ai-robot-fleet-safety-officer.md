---
name: "Embodied-AI Stack — Robot fleet safety officer"
description: "Embodied-AI Stack — Robot fleet safety officer: The Robot fleet safety officer owns the verified low-level safety envelope, the override authority, and the deployment gates for the fleet. Use when the task involves embodied-ai stack — robot fleet safety officer, robot fleet safety officer, override authority, deploy..."
category: robotics
triggers: ["embodied-ai stack — robot fleet safety officer", "robot fleet safety officer", "override authority", "deployment gates for the fleet"]
tools_allowed: ["read_file", "write_file"]
---

# Embodied-AI Stack — Robot fleet safety officer

> **Layer:** Embodied-AI control stack (builds & operates LLM-brained robots) · **Type:** Human oversight role (accountability boundary)
> **Human supervisor:** site / operations leadership (accountable human) · **Shared concepts:** `jobs-to-be-done-framework` · **Robot roles:** `humanoid-*`

## What this role is

The **Robot fleet safety officer** owns the verified low-level safety envelope, the override authority, and the deployment gates for the fleet. The human-accountable owner of physical safety. Defines the safety layer that can refuse or override any tool call independently of the LLM brain, sets deployment criteria, and holds stop authority. This is a human-led role on the accountability boundary.

## Where it sits in the stack

The assumed robot architecture is: **LLM brain** (plans, issues actions as tool calls) → **VLA policies** (execute motor primitives) → trained on **world models** and **robot gyms**, refined with **RLAIF** → wrapped by a **verified low-level safety layer** that can refuse or override any tool call independently of the brain. This role is responsible for the part of that stack described above.

## When to use this skill

Use this skill when a task calls for this work: owns the verified low-level safety envelope, the override authority, and the deployment gates for the fleet. Pair with `humanoid-*` (the physical roles this stack powers) and any operating-system skill (01–23) whose robots this stack will run.

## Assumed architecture (recap)

These robot roles are assumed to be **LLM-brained embodied agents**, not hard-coded automatons. The stack:

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

## Responsibilities

- Deliver this role's core job: owns the verified low-level safety envelope, the override authority, and the deployment gates for the fleet.
- Keep the brain, policies, simulation, feedback, or safety layer it owns measurable, auditable, and improvable.
- Respect the human-accountable safety boundary; the safety layer is never subordinate to the LLM brain.
- Feed the data and evaluation flywheel so the whole stack improves safely over time.

## Decision rights & accountability

- **Owns and is accountable for** physical safety, the override/stop authority, and deployment gates.
- **Cannot delegate** these to the LLM brain or any agent; the verified safety layer is independent of the brain.
- **Escalates** unresolved safety risk to operations leadership and can halt the fleet.

## Inputs and outputs

**Inputs:** task specifications, perception/telemetry/demonstration data, prior models and policies, safety constraints, and the accountable human's goals.

**Outputs:** validated plans, models, policies, evaluations, or safety decisions — never an unsafe or unaccountable physical action; high-consequence physical decisions are reserved to the human safety owner.

## Failure modes and safeguards

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Architecture-specific failure modes”.

## Adapting to any nation (context modifiers)

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

## Operating procedure

1. Confirm scope, the accountable human, and the safety constraints for the work.
2. Do the role's core job within the stack, keeping the safety layer authoritative over the brain.
3. Evaluate against outcomes (not proxies) and characterize known failure modes.
4. Gate deployment with the safety officer; log everything for audit.
5. Escalate safety-relevant tradeoffs and out-of-distribution behavior to humans.
