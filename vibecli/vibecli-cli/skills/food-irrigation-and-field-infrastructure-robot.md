---
name: "Irrigation and field-infrastructure robot"
description: "Irrigation and field-infrastructure robot: The Irrigation and field-infrastructure robot is an embodied robot whose job is to install, inspect, and repair irrigation, fencing, and field sensors and take soil a. Use when the task involves irrigation and field-infrastructure robot, irrigation, field-infrastructure robot."
category: agriculture
triggers: ["irrigation and field-infrastructure robot", "irrigation", "field-infrastructure robot"]
tools_allowed: ["read_file", "write_file"]
---

# Irrigation and field-infrastructure robot

> **Operating system:** 05. Food, Agriculture, Fisheries, and Nutrition · **Personnel type:** LLM-brained embodied robot
> **Best environments:** fields, pastures, irrigation networks, remote plots
> **Sector skill:** `food-sector-operations` · **Stack:** `embodied-ai-*` · **Shared concepts:** `jobs-to-be-done-framework`

## What this role is

The **Irrigation and field-infrastructure robot** is an embodied robot whose job is to install, inspect, and repair irrigation, fencing, and field sensors and take soil and tissue samples. Maintains the physical farm: laying and fixing drip/sprinkler lines, mending fences, placing and servicing soil and weather sensors, and collecting georeferenced soil and tissue samples for the agronomy agents. Extends reach into remote acreage that is costly to service by hand.

## Operating-system context

This role serves the *Food* operating system, whose mission is to produce, inspect, distribute, and stabilize safe food. It takes physical field, barn, and crop work so human farmers and the sector's AI agents can focus on judgment, planning, and exceptions.

## When to use this skill

When a task needs the physical job "install, inspect, and repair irrigation, fencing, and field sensors and take soil and tissue samples" in environments such as fields, pastures, irrigation networks, remote plots. Pair with the sector skill (`food-sector-operations`) for domain rules and the human accountability boundary, the AI agents under `food-*` that plan and direct this work, and `embodied-ai-*` for the brain, policies, and safety layer that run it.

## Cognitive and control architecture (assumed)

These robot roles are assumed to be **LLM-brained embodied agents**, not hard-coded automatons. The stack:

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Cognitive and control architecture (assumed)”.

## Division of labor and safety

- **Human owner (farmer / ranch manager / vet)** — owns animal welfare, land stewardship, safety, and exceptions; holds override and stop authority.
- **LLM brain** — perceives the field/barn, plans the task, and issues motor-primitive tool calls (`navigate_to`, `grasp`, `pick`, `place`, `inspect`).
- **VLA policies** — execute dexterous, delicate manipulation (e.g., picking ripe fruit without bruising) under the engineered safety envelope.
- **AI agents** — the sector's planning/monitoring agents (crop planning, irrigation, livestock health, machinery dispatch) direct and schedule the robot's work.
- **Verified safety layer** — validates, refuses, or overrides unsafe tool calls independently of the brain (people, animals, and bystanders protected).

## Accountability boundary

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Accountability boundary”.

These remain human-owned. The robot executes within an engineered envelope and routes anything outside it — welfare concerns, chemical decisions, or unsafe conditions — to the accountable human.

## Operating and safety procedure

1. Confirm the field/barn is mapped, people and animals are protected, and the task is within the engineered envelope.
2. The brain plans and emits motor-primitive **tool calls**; the safety layer validates each before execution.
3. Execute within speed, force, reach, and low-stress-handling limits via VLA policies.
4. Report progress, yields, exceptions, and any safety or welfare event to the sector agents and human owner.
5. Stop and yield to humans for out-of-distribution conditions, animal-welfare risk, or anything outside the envelope.

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

In smallholder and informal-sector agriculture, this role may be shared equipment, cooperatively owned, or rented by the hour rather than owned per farm; affordability and repairability dominate. In high-income, labor-scarce settings it fills chronic field-labor shortages. Re-read through:

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.
