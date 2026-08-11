---
name: "Operating System 19 — Environment, Climate, Waste, and Resource Stewardship"
description: "Operating System 19 — Environment, Climate, Waste, and Resource Stewardship: Protect natural systems, manage waste, reduce pollution, and adapt to climate risk. Use when the task involves environment, climate, waste, and resource stewardship, environment, climate, waste, resource stewardship."
category: sustainability
triggers: ["environment, climate, waste, and resource stewardship", "environment", "climate", "waste", "resource stewardship"]
tools_allowed: ["read_file", "write_file"]
---

# Operating System 19 — Environment, Climate, Waste, and Resource Stewardship

> **Layer:** National operating system (#19 of 23) · **Personnel model:** human-owned, AI- and robot-augmented
> **Cross-references:** `jobs-to-be-done-framework` (shared concepts, teaming pattern, accountability)

## Mission

Protect natural systems, manage waste, reduce pollution, and adapt to climate risk.

## When to use this skill

Load this skill when a task concerns environment, climate, waste, and resource stewardship. It gives an agent the sector map: the outcomes that must be produced, who owns them, what can be automated, and where human accountability is non-negotiable. From here, route to the specific role skills under `environment-*` for execution.

## Core Jobs To Be Done

These are the durable outcomes this operating system must reliably produce, written as trigger → response:

1. When waste is produced, collect, sort, treat, recycle, compost, landfill, or neutralize it safely.
2. When pollution occurs, monitor, enforce, remediate, and prevent recurrence.
3. When ecosystems decline, conserve, restore, and manage land/water/wildlife.
4. When climate risks rise, forecast, adapt, insure, relocate, harden, and decarbonize.

## The universal lifecycle, applied

Every job in this sector moves through the same seven steps. Use it as a checklist when designing or executing work here:

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Core Jobs To Be Done (lifecycle)”.

## Human role families (who owns the work)

- Waste collection operator, recycling coordinator, landfill manager.
- Environmental scientist, conservation scientist, ecologist, hydrologist.
- Climate risk analyst, sustainability manager, carbon accounting specialist.
- Environmental compliance specialist, remediation project manager.
- Park ranger, natural resource manager, urban forester.

These remain human-owned. AI personnel and robots augment them; they do not replace the accountable owner.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Technician/operator → environmental scientist/analyst → project manager → program director; ranger → senior → manager; sustainability analyst → manager → director.
- **Skills, tools & tech employers list:** GIS, remote sensing, carbon/emissions-accounting platforms, environmental monitoring/LIMS, modeling, EHS systems.
- **Qualifications, certifications & licenses:** PE (environmental), PG, CHMM (hazmat), CSP, CDL (waste), Certified Energy Manager, ISO 14001 lead auditor, pesticide/remediation licenses.
- **KPIs / metrics in postings:** Emissions reduced, diversion/recycling rate, permit compliance, remediation milestones, habitat/biodiversity metrics.
- **Where these roles are posted:** GovernmentJobs (EPA/state), USAJOBS, Indeed, LinkedIn, conservation/environmental boards, Idealist.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## AI personnel in this operating system (deployable role skills)

Each of the following has a dedicated, extensive skill under `environment-*`. Deploy them under the named human supervisor:

- **Emissions accounting agent** — compiles and audits greenhouse-gas inventories. *(supervised by carbon accounting specialist; skill: `environment-emissions-accounting-agent`)*
- **Satellite monitoring analyst** — monitors land, water, and emissions from remote sensing. *(supervised by environmental scientist; skill: `environment-satellite-monitoring-analyst`)*
- **Climate risk modeler** — models physical and transition climate risk. *(supervised by climate risk analyst; skill: `environment-climate-risk-modeler`)*
- **Waste stream optimization agent** — optimizes collection, sorting, and recycling flows. *(supervised by recycling coordinator; skill: `environment-waste-stream-optimization-agent`)*
- **Permit compliance agent** — tracks environmental permit obligations. *(supervised by environmental compliance specialist; skill: `environment-permit-compliance-agent`)*
- **Environmental impact review assistant** — drafts and checks environmental impact assessments. *(supervised by remediation project manager; skill: `environment-environmental-impact-review-assistant`)*

## Humanoid robot roles

- Sorting facilities, hazardous cleanup support, field sampling, park maintenance, inspection.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Humanoid robot roles”.

## Non-humanoid autonomous machines

Self-driving vehicles, equipment, and drones for this sector (LLM-planned; physical actions as tool calls; ODD + teleoperation fallback):

- **Environmental survey & monitoring drone** — map habitats, measure emissions and effluent, and monitor land, water, and wildlife from the air. *(autonomous machine skill: `environment-environmental-survey-monitoring-drone`)*

> **How these machines work (assumed architecture):** each is a **non-humanoid autonomous machine** — a foundation/LLM planning brain issues **actions as tool calls** (`follow_route`, `dump_bucket`, `take_off`, `spray_zone`, …) over a perception → prediction → planning → control stack trained on world models, driving/field simulation, and **RLAIF**. Each runs inside a defined **Operational Design Domain (ODD)** with a verified safe-stop and **teleoperation** fallback. Full detail in `autonomous-machine-*` and `jobs-to-be-done-framework`.

## Human accountability boundary (must stay human-led)

Environmental justice, land-use tradeoffs, enforcement, relocation policy, protected-area governance, and remediation signoff remain human-led.

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Human accountability boundary (must stay human-led)”.

## Division of labor (human / AI / robot)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Division of labor (human / AI / robot)”.

## Interfaces with other operating systems

This sector regularly depends on and feeds: Water & Sanitation, Energy & Utilities, Food & Agriculture, Resilience & Continuity. Coordinate handoffs explicitly; most systemic failures happen at the seams between operating systems.

## Strategic missions that draw on this sector

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Strategic missions that draw on this sector”.

- [Bioeconomy](../strategic-missions/bioeconomy/)

## Sector success metrics (illustrative)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Sector success metrics (illustrative)”.

## Failure modes to watch

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Failure modes to watch”.

## Deskilling watch & keep-warm regime

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Deskilling watch & keep-warm regime”.

- **Risk here:** Field scientists lose taxonomic and naturalist skill as remote sensing and AI ID take over.
- **Countermeasures:** Maintain field competency; ground-truthing; train naturalists.
- **Role/job simulators (keep-warm):** Field-identification and survey simulators; ground-truthing exercises; specimen/identification drills.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Deskilling watch & keep-warm”.

## Adapting to any nation (context modifiers)

The jobs above are universal; how they are staffed is not. Re-read this sector through:

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

## How to operate in this sector

1. Identify which Core JTBD the task serves.
2. Select the role skill(s) under `environment-*` that fit, and confirm the human supervisor.
3. Run the lifecycle: sense → interpret → decide → mobilize → execute → verify → govern.
4. Stop at the accountability boundary and route the decision to the accountable human.
5. Log actions to the control layer and surface anything that trips a failure mode.
