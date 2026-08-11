---
name: "Spraying & seeding drone"
description: "Spraying & seeding drone: The Spraying & seeding drone is a non-humanoid autonomous machine whose job is to apply crop inputs and seed precisely from the air on a prescription map. Use when the task involves spraying & seeding drone, food."
category: agriculture
triggers: ["spraying & seeding drone", "food"]
tools_allowed: ["read_file", "write_file"]
---

# Spraying & seeding drone

> **Operating system:** 05. Food, Agriculture, Fisheries, and Nutrition · **Personnel type:** Non-humanoid autonomous machine
> **Best environments:** fields, orchards, vineyards, paddies, and steep or wet ground machines can't reach
> **Sector skill:** `food-sector-operations` · **Operators:** `embodied-ai-*` · **Shared concepts:** `jobs-to-be-done-framework`

## What this machine is

The **Spraying & seeding drone** is a non-humanoid autonomous machine whose job is to apply crop inputs and seed precisely from the air on a prescription map. Variable-rate spray/seed UAV that covers terrain ground equipment cannot; pesticide decisions stay with the human.

## Operating-system context

This platform serves the *Food* operating system, whose mission is to produce, inspect, distribute, and stabilize safe food. It takes mobile and heavy-equipment work so people and the sector's AI agents can focus on planning, judgment, and exceptions.

## When to use this skill

When a task needs the physical job "apply crop inputs and seed precisely from the air on a prescription map" in environments such as fields, orchards, vineyards, paddies, and steep or wet ground machines can't reach. Pair with the sector skill (`food-sector-operations`) for domain rules and the human accountability boundary, the AI agents under `food-*` that plan and direct this work, and `embodied-ai-*` for the autonomy, fleet-ops, teleoperation, and safety roles that run it.

## Cognitive and control architecture (assumed)

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

## Division of labor and safety

- **Human owner (farmer / ranch manager)** — owns the safety case, the ODD, land/site/airspace rules, and stop authority; accountable for incidents.
- **Autonomy brain** — perceives, predicts, plans, and issues actuation as tool calls within the ODD.
- **Verified safety layer** — triggers a minimal-risk maneuver (safe-stop / return-to-base / hover) independently of the brain.
- **AI agents** — the sector's planning/monitoring agents direct and schedule the machine's missions.
- **Remote operator (teleop)** — supervises and takes over beyond the ODD.

## Accountability boundary

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Accountability boundary”.

These remain human-owned. The machine operates within its ODD and engineered safety envelope and routes anything outside it to the accountable human.

## Architecture-specific failure modes

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Architecture-specific failure modes”.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Farmworker/technician → crew lead/grower → farm/ranch manager → operations director; agronomy track; food safety: QA tech → QA manager → director of food safety.
- **Skills, tools & tech employers list:** Farm-management software (Climate FieldView, John Deere Operations Center, Granular), precision-ag/GIS, irrigation controllers, telematics, LIMS, HACCP/food-safety systems, ERP.
- **Qualifications, certifications & licenses:** CCA (Certified Crop Adviser), pesticide applicator license, PCQI (FSMA), ServSafe, DVM (veterinary), RD/RDN (dietitian), GlobalG.A.P., CDL for ag transport.
- **KPIs / metrics in postings:** Yield, input cost per acre/unit, loss/waste, food-safety audit scores, traceability completeness, on-time fulfillment.
- **Where these roles are posted:** AgCareers.com, Indeed, LinkedIn, GovernmentJobs (USDA/extension), Snagajob (seasonal/hourly), local co-ops.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## Adapting to any nation (context modifiers)

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.
