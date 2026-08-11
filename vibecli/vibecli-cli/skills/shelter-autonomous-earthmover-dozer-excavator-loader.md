---
name: "Autonomous earthmover (dozer/excavator/loader)"
description: "Autonomous earthmover (dozer/excavator/loader): The Autonomous earthmover (dozer/excavator/loader) is a non-humanoid autonomous machine whose job is to grade, excavate, load, and move material to a site model. Use when the task involves autonomous earthmover (dozer/excavator/loader), autonomous earthmover (dozer, ex..."
category: construction
triggers: ["autonomous earthmover (dozer/excavator/loader)", "autonomous earthmover (dozer", "excavator", "loader)"]
tools_allowed: ["read_file", "write_file"]
---

# Autonomous earthmover (dozer/excavator/loader)

> **Operating system:** 10. Shelter, Construction, Land, and the Built Environment · **Personnel type:** Non-humanoid autonomous machine
> **Best environments:** construction sites, road projects, earthworks
> **Sector skill:** `shelter-sector-operations` · **Operators:** `embodied-ai-*` · **Shared concepts:** `jobs-to-be-done-framework`

## What this machine is

The **Autonomous earthmover (dozer/excavator/loader)** is a non-humanoid autonomous machine whose job is to grade, excavate, load, and move material to a site model. Geofenced autonomous earthmoving equipment executing tasks against a 3D site/BIM model under a site safety system.

## Operating-system context

This platform serves the *Shelter* operating system, whose mission is to create and maintain places for living, working, mobility, commerce, and public life. It takes mobile and heavy-equipment work so people and the sector's AI agents can focus on planning, judgment, and exceptions.

## When to use this skill

When a task needs the physical job "grade, excavate, load, and move material to a site model" in environments such as construction sites, road projects, earthworks. Pair with the sector skill (`shelter-sector-operations`) for domain rules and the human accountability boundary, the AI agents under `shelter-*` that plan and direct this work, and `embodied-ai-*` for the autonomy, fleet-ops, teleoperation, and safety roles that run it.

## Cognitive and control architecture (assumed)

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

## Division of labor and safety

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Division of labor and safety”.

## Accountability boundary

Land-use decisions, structural signoff, occupancy approval, worker safety, eviction, and public consultation remain human-led.

These remain human-owned. The machine operates within its ODD and engineered safety envelope and routes anything outside it to the accountable human.

## Architecture-specific failure modes

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Architecture-specific failure modes”.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Laborer/apprentice → journeyman tradesperson → foreman/superintendent → project manager; design: intern architect/EIT → licensed architect/PE → principal; planner → senior planner → director.
- **Skills, tools & tech employers list:** BIM (Revit), AutoCAD, Procore/Bluebeam, estimating (PlanSwift), scheduling (Primavera P6, MS Project), GIS, permitting systems.
- **Qualifications, certifications & licenses:** PE, licensed architect (ARE/AIA), LEED, PMP, OSHA 30, ICC code certifications, trade journeyman/master licenses, PLS (surveyor).
- **KPIs / metrics in postings:** Schedule/cost variance (CPI/SPI), safety (TRIR/EMR), punch-list/defects, inspection pass rate, permit cycle time.
- **Where these roles are posted:** Indeed, LinkedIn, ZipRecruiter, construction boards, GovernmentJobs (inspectors/planners), trade unions.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## Adapting to any nation (context modifiers)

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.
