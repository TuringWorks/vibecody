---
name: "Operating System 07 — Energy, Utilities, and Grid Operations"
description: "Operating System 07 — Energy, Utilities, and Grid Operations: Produce, store, transmit, distribute, and balance energy safely and affordably. Use when the task involves energy, utilities, and grid operations, energy, utilities, grid operations."
category: energy
triggers: ["energy, utilities, and grid operations", "energy", "utilities", "grid operations"]
tools_allowed: ["read_file", "write_file"]
---

# Operating System 07 — Energy, Utilities, and Grid Operations

> **Layer:** National operating system (#7 of 23) · **Personnel model:** human-owned, AI- and robot-augmented
> **Cross-references:** `jobs-to-be-done-framework` (shared concepts, teaming pattern, accountability)

## Mission

Produce, store, transmit, distribute, and balance energy safely and affordably.

## When to use this skill

Load this skill when a task concerns energy, utilities, and grid operations. It gives an agent the sector map: the outcomes that must be produced, who owns them, what can be automated, and where human accountability is non-negotiable. From here, route to the specific role skills under `energy-*` for execution.

## Core Jobs To Be Done

These are the durable outcomes this operating system must reliably produce, written as trigger → response:

1. When demand changes second by second, balance supply and load.
2. When assets age or fail, maintain generation, storage, transmission, and distribution.
3. When fuel markets or weather shift, plan resilient supply.
4. When decarbonization is required, integrate renewables, storage, demand response, nuclear, hydro, geothermal, and efficiency.
5. When outages occur, restore service safely and communicate clearly.

## The universal lifecycle, applied

Every job in this sector moves through the same seven steps. Use it as a checklist when designing or executing work here:

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Core Jobs To Be Done (lifecycle)”.

## Human role families (who owns the work)

- Grid operator, power systems engineer, utility dispatcher.
- Electrician, lineworker, substation technician, relay technician.
- Renewable energy engineer, solar installer, wind turbine technician.
- Nuclear operator, plant engineer, safety analyst.
- Energy trader, load forecaster, demand response manager.
- Utility customer operations manager, field service technician.

These remain human-owned. AI personnel and robots augment them; they do not replace the accountable owner.

## Labor-market grounding (how these roles are advertised)

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding (how these roles are advertised)”.

- **Advertised titles & seniority ladder:** Apprentice lineworker/technician → journeyman → foreman; system-operator trainee → certified system operator → shift supervisor → control-center manager; EIT → PE → engineering manager; energy trader.
- **Skills, tools & tech employers list:** EMS/SCADA, OMS (outage management), ADMS/DMS, ISO/RTO market platforms, PI historian, PSS/E, GIS.
- **Qualifications, certifications & licenses:** NERC System Operator certification (RC/BA/TO), journeyman electrical license, PE, NRC reactor operator (nuclear), OSHA, CDL.
- **KPIs / metrics in postings:** SAIDI/SAIFI reliability, area control error/load balance, restoration time, OSHA recordables, market-settlement accuracy.
- **Where these roles are posted:** ZipRecruiter, Glassdoor, BuiltIn, LinkedIn, IBEW, utility career pages.

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Labor-market grounding”.

## AI personnel in this operating system (deployable role skills)

Each of the following has a dedicated, extensive skill under `energy-*`. Deploy them under the named human supervisor:

- **Load forecasting agent** — forecasts demand across horizons for balancing and trading. *(supervised by load forecaster; skill: `energy-load-forecasting-agent`)*
- **Grid anomaly detector** — detects faults and instability in telemetry. *(supervised by grid operator; skill: `energy-grid-anomaly-detector`)*
- **Outage restoration planner** — sequences crews and switching to restore service safely. *(supervised by distribution operations lead; skill: `energy-outage-restoration-planner`)*
- **Maintenance prediction agent** — predicts asset failures and schedules maintenance. *(supervised by reliability engineer; skill: `energy-maintenance-prediction-agent`)*
- **Energy market analyst** — analyzes prices and positions within market rules. *(supervised by energy trader; skill: `energy-energy-market-analyst`)*
- **Permitting documentation agent** — prepares siting and interconnection documentation. *(supervised by project engineer; skill: `energy-permitting-documentation-agent`)*
- **Customer outage communications agent** — drafts and targets outage and restoration updates. *(supervised by customer operations manager; skill: `energy-customer-outage-communications-agent`)*

## Humanoid robot roles

- Plant inspection rounds, warehouse logistics, solar-farm maintenance, substation visual inspection.
- Support for line crews with tools/materials, but energized work requires extreme controls.

> Shared pattern — see the `shared-embodied-autonomy-architecture` skill, section “Humanoid robot roles”.

## Non-humanoid autonomous machines

Self-driving vehicles, equipment, and drones for this sector (LLM-planned; physical actions as tool calls; ODD + teleoperation fallback):

- **Grid & renewable-asset inspection drone** — inspect powerlines, towers, substations, and solar/wind assets from the air. *(autonomous machine skill: `energy-grid-renewable-asset-inspection-drone`)*

> **How these machines work (assumed architecture):** each is a **non-humanoid autonomous machine** — a foundation/LLM planning brain issues **actions as tool calls** (`follow_route`, `dump_bucket`, `take_off`, `spray_zone`, …) over a perception → prediction → planning → control stack trained on world models, driving/field simulation, and **RLAIF**. Each runs inside a defined **Operational Design Domain (ODD)** with a verified safe-stop and **teleoperation** fallback. Full detail in `autonomous-machine-*` and `jobs-to-be-done-framework`.

## Human accountability boundary (must stay human-led)

Grid emergency authority, nuclear operations, safety switching, market-manipulation controls, and major infrastructure siting remain human-accountable.

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Human accountability boundary (must stay human-led)”.

## Division of labor (human / AI / robot)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Division of labor (human / AI / robot)”.

## Interfaces with other operating systems

This sector regularly depends on and feeds: Water & Sanitation, Materials & Manufacturing, Communications & Software, Resilience & Continuity. Coordinate handoffs explicitly; most systemic failures happen at the seams between operating systems.

## Strategic missions that draw on this sector

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Strategic missions that draw on this sector”.

- [Energy Abundance](../strategic-missions/energy-abundance/)
- [Frontier AI Production](../strategic-missions/frontier-ai-production/)
- [Digital Infrastructure](../strategic-missions/digital-infrastructure/)

## Sector success metrics (illustrative)

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Sector success metrics (illustrative)”.

## Failure modes to watch

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Failure modes to watch”.

## Deskilling watch & keep-warm regime

> Shared pattern — see the `shared-sector-operations-pattern` skill, section “Deskilling watch & keep-warm regime”.

- **Risk here:** System operators lose manual switching and restoration skill; black-start expertise becomes rare.
- **Countermeasures:** NERC recertification plus simulator training; black-start drills; manual-restoration practice.
- **Role/job simulators (keep-warm):** Control-room and black-start simulators; manual switching and restoration scenarios (already mature practice).

> Shared pattern — see the `shared-ai-personnel-pattern` skill, section “Deskilling watch & keep-warm”.

## Adapting to any nation (context modifiers)

The jobs above are universal; how they are staffed is not. Re-read this sector through:

> Shared pattern — see the `shared-national-context-modifiers` skill, section “Adapting to any nation (context modifiers)”.

## How to operate in this sector

1. Identify which Core JTBD the task serves.
2. Select the role skill(s) under `energy-*` that fit, and confirm the human supervisor.
3. Run the lifecycle: sense → interpret → decide → mobilize → execute → verify → govern.
4. Stop at the accountability boundary and route the decision to the accountable human.
5. Log actions to the control layer and surface anything that trips a failure mode.
