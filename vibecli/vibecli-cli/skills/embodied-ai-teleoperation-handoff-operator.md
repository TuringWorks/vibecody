---
name: "Embodied-AI Stack — Teleoperation & handoff operator"
description: "Embodied-AI Stack — Teleoperation & handoff operator: The Teleoperation & handoff operator takes remote control for edge cases the autonomy cannot handle and provides demonstrations that feed back into training. Use when the task involves embodied-ai stack — teleoperation & handoff operator, teleoperation & handoff..."
category: robotics
triggers: ["embodied-ai stack — teleoperation & handoff operator", "teleoperation & handoff operator", "provides demonstrations that feed back into training"]
tools_allowed: ["read_file", "write_file"]
---

# Embodied-AI Stack — Teleoperation & handoff operator

> **Layer:** Embodied-AI control stack (builds & operates LLM-brained robots) · **Type:** Human-in-the-loop operator
> **Human supervisor:** fleet operations manager · **Shared concepts:** `jobs-to-be-done-framework` · **Robot roles:** `humanoid-*`

## What this role is

The **Teleoperation & handoff operator** takes remote control for edge cases the autonomy cannot handle and provides demonstrations that feed back into training. The human-in-the-loop fallback. Handles low-confidence or unsafe situations the brain escalates, and generates high-quality demonstration data for policy improvement.

## Where it sits in the stack

The assumed robot architecture is: **LLM brain** (plans, issues actions as tool calls) → **VLA policies** (execute motor primitives) → trained on **world models** and **robot gyms**, refined with **RLAIF** → wrapped by a **verified low-level safety layer** that can refuse or override any tool call independently of the brain. This role is responsible for the part of that stack described above.

## When to use this skill

Use this skill when a task calls for this work: takes remote control for edge cases the autonomy cannot handle and provides demonstrations that feed back into training. Pair with `humanoid-*` (the physical roles this stack powers) and any operating-system skill (01–23) whose robots this stack will run.

## Assumed architecture (recap)

These robot roles are assumed to be **LLM-brained embodied agents**, not hard-coded automatons. The stack:

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

## Responsibilities

- Deliver this role's core job: takes remote control for edge cases the autonomy cannot handle and provides demonstrations that feed back into training.
- Keep the brain, policies, simulation, feedback, or safety layer it owns measurable, auditable, and improvable.
- Respect the human-accountable safety boundary; the safety layer is never subordinate to the LLM brain.
- Feed the data and evaluation flywheel so the whole stack improves safely over time.

## Decision rights & accountability

- **Acts** when autonomy escalates a low-confidence or unsafe situation.
- **Provides** demonstrations and corrections that feed training.
- **Escalates** systemic issues (recurring takeovers) to engineering and safety.

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
