---
name: "Embodied-AI Stack — RLAIF pipeline engineer"
description: "Embodied-AI Stack — RLAIF pipeline engineer: The RLAIF pipeline engineer designs the reinforcement-learning-from-AI-feedback pipelines and AI critics that shape robot skills and judgment at scale. Use when the task involves embodied-ai stack — rlaif pipeline engineer, rlaif pipeline engineer, ai critics that shape r..."
category: robotics
triggers: ["embodied-ai stack — rlaif pipeline engineer", "rlaif pipeline engineer", "ai critics that shape robot skills", "judgment at scale"]
tools_allowed: ["read_file", "write_file"]
---

# Embodied-AI Stack — RLAIF pipeline engineer

> **Layer:** Embodied-AI control stack (builds & operates LLM-brained robots) · **Type:** Human engineering role (AI/robotics)
> **Human supervisor:** alignment / RL lead · **Shared concepts:** `jobs-to-be-done-framework` · **Robot roles:** `humanoid-*`

## What this role is

The **RLAIF pipeline engineer** designs the reinforcement-learning-from-AI-feedback pipelines and AI critics that shape robot skills and judgment at scale. Builds the AI-feedback reward and preference models that supplement scarce human feedback. Must guard against reward hacking and critic bias, and keep human oversight in the loop on safety-relevant behaviors.

## Where it sits in the stack

The assumed robot architecture is: **LLM brain** (plans, issues actions as tool calls) → **VLA policies** (execute motor primitives) → trained on **world models** and **robot gyms**, refined with **RLAIF** → wrapped by a **verified low-level safety layer** that can refuse or override any tool call independently of the brain. This role is responsible for the part of that stack described above.

## When to use this skill

Use this skill when a task calls for this work: designs the reinforcement-learning-from-AI-feedback pipelines and AI critics that shape robot skills and judgment at scale. Pair with `humanoid-*` (the physical roles this stack powers) and any operating-system skill (01–23) whose robots this stack will run.

## Assumed architecture (recap)

These robot roles are assumed to be **LLM-brained embodied agents**, not hard-coded automatons. The stack:

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

## Responsibilities

- Deliver this role's core job: designs the reinforcement-learning-from-AI-feedback pipelines and AI critics that shape robot skills and judgment at scale.
- Keep the brain, policies, simulation, feedback, or safety layer it owns measurable, auditable, and improvable.
- Respect the human-accountable safety boundary; the safety layer is never subordinate to the LLM brain.
- Feed the data and evaluation flywheel so the whole stack improves safely over time.

## Decision rights & accountability

- **Owns** the technical quality, robustness, and evaluation of this layer of the stack.
- **Gates** what is safe to ship to real hardware with the safety officer.
- **Escalates** capability/safety tradeoffs to research and safety leadership.

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
