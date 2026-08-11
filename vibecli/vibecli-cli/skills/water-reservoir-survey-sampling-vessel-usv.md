---
name: "Reservoir survey & sampling vessel (USV)"
description: "Reservoir survey & sampling vessel (USV): The Reservoir survey & sampling vessel (USV) is a non-humanoid autonomous machine whose job is to survey reservoirs and waterways and collect water-quality samples au. Use when the task involves reservoir survey & sampling vessel (usv), water."
category: water
triggers: ["reservoir survey & sampling vessel (usv)", "water"]
tools_allowed: ["read_file", "write_file"]
---

# Reservoir survey & sampling vessel (USV)

> **Operating system:** 06. Water, Sanitation, and Public Hygiene · **Personnel type:** Non-humanoid autonomous machine
> **Best environments:** reservoirs, intakes, rivers, coastal outfalls
> **Sector skill:** `water-sector-operations` · **Operators:** `embodied-ai-*` · **Shared concepts:** `jobs-to-be-done-framework`

## What this machine is

The **Reservoir survey & sampling vessel (USV)** is a non-humanoid autonomous machine whose job is to survey reservoirs and waterways and collect water-quality samples autonomously. Uncrewed surface vessel mapping bathymetry and pulling samples for the water-quality-monitoring agent and the lab.

## Operating-system context

This platform serves the *Water* operating system, whose mission is to provide safe water, remove waste, control flooding, and prevent waterborne disease. It takes mobile and heavy-equipment work so people and the sector's AI agents can focus on planning, judgment, and exceptions.

## When to use this skill

When a task needs the physical job "survey reservoirs and waterways and collect water-quality samples autonomously" in environments such as reservoirs, intakes, rivers, coastal outfalls. Pair with the sector skill (`water-sector-operations`) for domain rules and the human accountability boundary, the AI agents under `water-*` that plan and direct this work, and `embodied-ai-*` for the autonomy, fleet-ops, teleoperation, and safety roles that run it.

## Cognitive and control architecture (assumed)

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

## Division of labor and safety

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Division of labor and safety”.

## Accountability boundary

Public health notices, water shutoffs, infrastructure investment, environmental-discharge approvals, and emergency allocation remain human-led.

These remain human-owned. The machine operates within its ODD and engineered safety envelope and routes anything outside it to the accountable human.

## Architecture-specific failure modes

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Architecture-specific failure modes”.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Operator trainee → certified operator (Grade I–IV) → chief operator/superintendent → utility director; engineering: EIT → PE.
- **Skills, tools & tech employers list:** SCADA, GIS, hydraulic modeling (EPANET, WaterGEMS), LIMS, CMMS (asset/maintenance), telemetry.
- **Qualifications, certifications & licenses:** State water/wastewater operator certification (Grades I–IV), PE (civil/environmental), backflow tester, confined-space, CDL (some).
- **KPIs / metrics in postings:** Water-quality compliance, non-revenue water/leakage, NPDES permit compliance, boil-water/outage events, asset condition.
- **Where these roles are posted:** GovernmentJobs, Careers.<state>.gov, AWWA/WEF job boards, Indeed, ZipRecruiter.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## Adapting to any nation (context modifiers)

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.
