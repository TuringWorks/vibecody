---
name: "Harbor tug / survey vessel (USV)"
description: "Harbor tug / survey vessel (USV): The Harbor tug / survey vessel (USV) is a non-humanoid autonomous machine whose job is to assist berthing and survey harbors and channels without a crew. Use when the task involves harbor tug / survey vessel (usv), harbor tug, survey vessel (usv)."
category: logistics
triggers: ["harbor tug / survey vessel (usv)", "harbor tug", "survey vessel (usv)"]
tools_allowed: ["read_file", "write_file"]
---

# Harbor tug / survey vessel (USV)

> **Operating system:** 11. Transportation, Logistics, Postal, and Mobility · **Personnel type:** Non-humanoid autonomous machine
> **Best environments:** harbors, channels, coastal waters
> **Sector skill:** `transportation-sector-operations` · **Operators:** `embodied-ai-*` · **Shared concepts:** `jobs-to-be-done-framework`

## What this machine is

The **Harbor tug / survey vessel (USV)** is a non-humanoid autonomous machine whose job is to assist berthing and survey harbors and channels without a crew. Uncrewed/autonomous surface vessel for harbor assist, hydrographic survey, and patrol under VTS coordination.

## Operating-system context

This platform serves the *Transportation* operating system, whose mission is to move people and goods through networks safely, predictably, and economically. It takes mobile and heavy-equipment work so people and the sector's AI agents can focus on planning, judgment, and exceptions.

## When to use this skill

When a task needs the physical job "assist berthing and survey harbors and channels without a crew" in environments such as harbors, channels, coastal waters. Pair with the sector skill (`transportation-sector-operations`) for domain rules and the human accountability boundary, the AI agents under `transportation-*` that plan and direct this work, and `embodied-ai-*` for the autonomy, fleet-ops, teleoperation, and safety roles that run it.

## Cognitive and control architecture (assumed)

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

## Division of labor and safety

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Division of labor and safety”.

## Accountability boundary

Safety-critical vehicle operation, air-traffic-control authority, hazardous-goods approval, labor safety, and public-transport policy remain human-accountable.

These remain human-owned. The machine operates within its ODD and engineered safety envelope and routes anything outside it to the accountable human.

## Architecture-specific failure modes

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Architecture-specific failure modes”.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Driver/warehouse associate → lead/dispatcher → operations supervisor → terminal/DC manager → director of logistics; pilot and ATC tracks; mechanic apprentice → A&P/journeyman.
- **Skills, tools & tech employers list:** TMS, WMS, route optimization, ELD/telematics, dispatch systems, EDI, fleet-maintenance systems.
- **Qualifications, certifications & licenses:** CDL (A/B/C) + endorsements (HazMat, tanker) with ELDT/FMCSA medical, FAA A&P (mechanics), ATP/commercial pilot, FAA ATC, APICS CSCP/CLTD, TWIC (ports), OSHA/forklift.
- **KPIs / metrics in postings:** On-time delivery, cost per mile/shipment, fleet utilization, DOT safety compliance, dwell time, damage rate.
- **Where these roles are posted:** iHireTransportation, Indeed, ZipRecruiter, Snagajob (hourly), Dice (logistics tech), USAJOBS (FAA/USPS).

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## Adapting to any nation (context modifiers)

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.
