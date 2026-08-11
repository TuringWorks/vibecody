---
name: "Water-asset inspection drone"
description: "Water-asset inspection drone: The Water-asset inspection drone is a non-humanoid autonomous machine whose job is to inspect tanks, towers, pipelines, and treatment assets from the air. Use when the task involves water-asset inspection drone, water."
category: water
triggers: ["water-asset inspection drone", "water"]
tools_allowed: ["read_file", "write_file"]
---

# Water-asset inspection drone

> **Operating system:** 06. Water, Sanitation, and Public Hygiene · **Personnel type:** Non-humanoid autonomous machine
> **Best environments:** treatment plants, tank farms, pipeline corridors
> **Sector skill:** `water-sector-operations` · **Operators:** `embodied-ai-*` · **Shared concepts:** `jobs-to-be-done-framework`

## What this machine is

The **Water-asset inspection drone** is a non-humanoid autonomous machine whose job is to inspect tanks, towers, pipelines, and treatment assets from the air. Autonomous UAV running thermal/RGB/LiDAR inspection; imagery feeds the asset-maintenance-planner and leak-prediction agents.

## Operating-system context

This platform serves the *Water* operating system, whose mission is to provide safe water, remove waste, control flooding, and prevent waterborne disease. It takes mobile and heavy-equipment work so people and the sector's AI agents can focus on planning, judgment, and exceptions.

## When to use this skill

When a task needs the physical job "inspect tanks, towers, pipelines, and treatment assets from the air" in environments such as treatment plants, tank farms, pipeline corridors. Pair with the sector skill (`water-sector-operations`) for domain rules and the human accountability boundary, the AI agents under `water-*` that plan and direct this work, and `embodied-ai-*` for the autonomy, fleet-ops, teleoperation, and safety roles that run it.

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
