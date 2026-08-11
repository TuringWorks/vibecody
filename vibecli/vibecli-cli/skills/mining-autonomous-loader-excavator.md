---
name: "Autonomous loader / excavator"
description: "Autonomous loader / excavator: The Autonomous loader / excavator is a non-humanoid autonomous machine whose job is to load trucks and dig and move material at the face. Use when the task involves autonomous loader / excavator, autonomous loader, excavator."
category: mining
triggers: ["autonomous loader / excavator", "autonomous loader", "excavator"]
tools_allowed: ["read_file", "write_file"]
---

# Autonomous loader / excavator

> **Operating system:** 08. Mining, Materials, Chemicals, and Industrial Inputs · **Personnel type:** Non-humanoid autonomous machine
> **Best environments:** mines, quarries, stockyards
> **Sector skill:** `mining-sector-operations` · **Operators:** `embodied-ai-*` · **Shared concepts:** `jobs-to-be-done-framework`

## What this machine is

The **Autonomous loader / excavator** is a non-humanoid autonomous machine whose job is to load trucks and dig and move material at the face. Autonomous loading unit working with the haul fleet under a site traffic-management system.

## Operating-system context

This platform serves the *Mining* operating system, whose mission is to extract and transform raw materials into safe, reliable inputs for the economy. It takes mobile and heavy-equipment work so people and the sector's AI agents can focus on planning, judgment, and exceptions.

## When to use this skill

When a task needs the physical job "load trucks and dig and move material at the face" in environments such as mines, quarries, stockyards. Pair with the sector skill (`mining-sector-operations`) for domain rules and the human accountability boundary, the AI agents under `mining-*` that plan and direct this work, and `embodied-ai-*` for the autonomy, fleet-ops, teleoperation, and safety roles that run it.

## Cognitive and control architecture (assumed)

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

## Division of labor and safety

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Division of labor and safety”.

## Accountability boundary

Mine safety, hazardous releases, environmental permits, community consent, and shutdown decisions remain human-led.

These remain human-owned. The machine operates within its ODD and engineered safety envelope and routes anything outside it to the accountable human.

## Architecture-specific failure modes

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Architecture-specific failure modes”.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Operator/technician → process/plant engineer → superintendent → plant manager; geologist and metallurgist tracks.
- **Skills, tools & tech employers list:** DCS process control, LIMS, mine-planning (Surpac, Vulcan), SCADA, EHS systems, simulation.
- **Qualifications, certifications & licenses:** PE, MSHA training, CSP (safety), HAZWOPER, Professional Geologist (PG), CIH (industrial hygiene).
- **KPIs / metrics in postings:** Throughput/recovery, yield and quality, safety (TRIR), environmental compliance, downtime.
- **Where these roles are posted:** Indeed, LinkedIn, ZipRecruiter, mining/chemical industry boards.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## Adapting to any nation (context modifiers)

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.
