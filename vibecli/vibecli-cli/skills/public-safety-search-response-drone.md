---
name: "Search & response drone"
description: "Search & response drone: The Search & response drone is a non-humanoid autonomous machine whose job is to search for people, map incidents, and deliver overhead situational awareness in emerg. Use when the task involves search & response drone, public safety."
category: public-safety
triggers: ["search & response drone", "public safety"]
tools_allowed: ["read_file", "write_file"]
---

# Search & response drone

> **Operating system:** 04. Public Safety, Justice Operations, and Emergency Response · **Personnel type:** Non-humanoid autonomous machine
> **Best environments:** disaster zones, wildland fires, search areas
> **Sector skill:** `public-safety-sector-operations` · **Operators:** `embodied-ai-*` · **Shared concepts:** `jobs-to-be-done-framework`

## What this machine is

The **Search & response drone** is a non-humanoid autonomous machine whose job is to search for people, map incidents, and deliver overhead situational awareness in emergencies. Autonomous UAV providing search and a live overhead picture for incident command; it does not make life-safety decisions.

## Operating-system context

This platform serves the *Public Safety* operating system, whose mission is to prevent harm, respond to emergencies, maintain order, and recover from acute incidents. It takes mobile and heavy-equipment work so people and the sector's AI agents can focus on planning, judgment, and exceptions.

## When to use this skill

When a task needs the physical job "search for people, map incidents, and deliver overhead situational awareness in emergencies" in environments such as disaster zones, wildland fires, search areas. Pair with the sector skill (`public-safety-sector-operations`) for domain rules and the human accountability boundary, the AI agents under `public-safety-*` that plan and direct this work, and `embodied-ai-*` for the autonomy, fleet-ops, teleoperation, and safety roles that run it.

## Cognitive and control architecture (assumed)

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

## Division of labor and safety

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Division of labor and safety”.

## Accountability boundary

Arrests, use of force, triage in scarce life-saving situations, sentencing, detention, and incident command remain human-led.

These remain human-owned. The machine operates within its ODD and engineered safety envelope and routes anything outside it to the accountable human.

## Architecture-specific failure modes

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Architecture-specific failure modes”.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Recruit/officer/EMT → detective/paramedic/senior → sergeant/lieutenant/captain → chief; dispatcher → comms supervisor; emergency-management coordinator → director.
- **Skills, tools & tech employers list:** CAD (computer-aided dispatch), RMS (records management), NIMS/ICS, body-cam/evidence systems, NCIC, GIS.
- **Qualifications, certifications & licenses:** POST certification (police), state EMT/Paramedic (NREMT), Firefighter I/II, EMD, FEMA ICS/NIMS, CEM (certified emergency manager).
- **KPIs / metrics in postings:** Response and call-answer times, case clearance rate, incident outcomes, mutual-aid readiness, safety.
- **Where these roles are posted:** GovernmentJobs, National Testing Network/PoliceApp, USAJOBS, local agency sites, Snagajob (some support roles).

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## Adapting to any nation (context modifiers)

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.
