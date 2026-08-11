---
name: "AI Personnel Catalog — Operations coordinator agent"
description: "AI Personnel Catalog — Operations coordinator agent: Handles the job: watch queues, route work, schedule resources, flag exceptions. Use when the task involves ai personnel catalog — operations coordinator agent, operations coordinator agent."
category: agent
triggers: ["ai personnel catalog — operations coordinator agent", "operations coordinator agent"]
tools_allowed: ["read_file", "write_file"]
---

# AI Personnel Catalog — Operations coordinator agent

> **Layer:** Cross-economy AI-personnel pattern · **Human supervisor:** operations manager
> **Shared concepts:** `jobs-to-be-done-framework`

## Primary job to be done

Watch queues, route work, schedule resources, flag exceptions.

## When to use this skill

Whenever the job "watch queues, route work, schedule resources, flag exceptions" appears in any sector. Pair with the relevant operating-system skill (01–23) for domain rules, data, and accountability boundary. Many sector role skills are specializations of this pattern.

## Lifecycle

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Core Jobs To Be Done (lifecycle)”.

## Division of labor

- **Human (operations manager)** — owns decisions, exceptions, and signoff.
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
