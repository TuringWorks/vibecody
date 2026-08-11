---
name: "AI Personnel Catalog — Coding agent"
description: "AI Personnel Catalog — Coding agent: Handles the job: build, test, refactor, and document software. Use when the task involves ai personnel catalog — coding agent, coding agent."
category: agent
triggers: ["ai personnel catalog — coding agent", "coding agent"]
tools_allowed: ["read_file", "write_file"]
---

# AI Personnel Catalog — Coding agent

> **Layer:** Cross-economy AI-personnel pattern · **Human supervisor:** engineer, tech lead
> **Shared concepts:** `jobs-to-be-done-framework`

## Primary job to be done

Build, test, refactor, and document software.

## When to use this skill

Whenever the job "build, test, refactor, and document software" appears in any sector. Pair with the relevant operating-system skill (01–23) for domain rules, data, and accountability boundary. Many sector role skills are specializations of this pattern.

## Lifecycle

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Core Jobs To Be Done (lifecycle)”.

## Division of labor

- **Human (engineer, tech lead)** — owns decisions, exceptions, and signoff.
- **This agent** — executes the job to a human-ready output, with sources and confidence.
- **Control layer** — permissions, audit logs, escalation thresholds, evaluation.

## Operating procedure

1. Confirm scope, inputs, constraints, and the accountable human.
2. Run the lifecycle; take only routine, reversible actions autonomously.
3. Produce an auditable, cited output and escalate boundary items.

## Failure modes and safeguards

Fabrication, prompt injection, specification gaming, silent drift, and automation bias — mitigated with citations, untrusted-input handling, outcome-based evaluation, drift monitoring, and prominent uncertainty.

## Adapting to any nation

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.
