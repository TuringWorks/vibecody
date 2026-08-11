---
name: "Grid & renewable-asset inspection drone"
description: "Grid & renewable-asset inspection drone: The Grid & renewable-asset inspection drone is a non-humanoid autonomous machine whose job is to inspect powerlines, towers, substations, and solar/wind assets from t. Use when the task involves grid & renewable-asset inspection drone, energy."
category: energy
triggers: ["grid & renewable-asset inspection drone", "energy"]
tools_allowed: ["read_file", "write_file"]
---

# Grid & renewable-asset inspection drone

> **Operating system:** 07. Energy, Utilities, and Grid Operations · **Personnel type:** Non-humanoid autonomous machine
> **Best environments:** transmission corridors, substations, solar and wind farms
> **Sector skill:** `energy-sector-operations` · **Operators:** `embodied-ai-*` · **Shared concepts:** `jobs-to-be-done-framework`

## What this machine is

The **Grid & renewable-asset inspection drone** is a non-humanoid autonomous machine whose job is to inspect powerlines, towers, substations, and solar/wind assets from the air. Autonomous UAV running thermal/RGB/LiDAR inspection missions; imagery feeds the maintenance-prediction agent and keeps crews off energized structures.

## Operating-system context

This platform serves the *Energy* operating system, whose mission is to produce, store, transmit, distribute, and balance energy safely and affordably. It takes mobile and heavy-equipment work so people and the sector's AI agents can focus on planning, judgment, and exceptions.

## When to use this skill

When a task needs the physical job "inspect powerlines, towers, substations, and solar/wind assets from the air" in environments such as transmission corridors, substations, solar and wind farms. Pair with the sector skill (`energy-sector-operations`) for domain rules and the human accountability boundary, the AI agents under `energy-*` that plan and direct this work, and `embodied-ai-*` for the autonomy, fleet-ops, teleoperation, and safety roles that run it.

## Cognitive and control architecture (assumed)

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

## Division of labor and safety

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Division of labor and safety”.

## Accountability boundary

Grid emergency authority, nuclear operations, safety switching, market-manipulation controls, and major infrastructure siting remain human-accountable.

These remain human-owned. The machine operates within its ODD and engineered safety envelope and routes anything outside it to the accountable human.

## Architecture-specific failure modes

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Architecture-specific failure modes”.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Apprentice lineworker/technician → journeyman → foreman; system-operator trainee → certified system operator → shift supervisor → control-center manager; EIT → PE → engineering manager; energy trader.
- **Skills, tools & tech employers list:** EMS/SCADA, OMS (outage management), ADMS/DMS, ISO/RTO market platforms, PI historian, PSS/E, GIS.
- **Qualifications, certifications & licenses:** NERC System Operator certification (RC/BA/TO), journeyman electrical license, PE, NRC reactor operator (nuclear), OSHA, CDL.
- **KPIs / metrics in postings:** SAIDI/SAIFI reliability, area control error/load balance, restoration time, OSHA recordables, market-settlement accuracy.
- **Where these roles are posted:** ZipRecruiter, Glassdoor, BuiltIn, LinkedIn, IBEW, utility career pages.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## Adapting to any nation (context modifiers)

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.
